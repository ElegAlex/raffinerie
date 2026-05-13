use crate::catalog::{catalog, profile_complet, profile_minimal, profile_standard_camieg};
use crate::exporter::params_sheet::ExportContext;
use crate::exporter::workbook::build_workbook;
use crate::filter::{evaluate, FilterSet};
use crate::parser::parse;
use crate::persistence::{self, Profiles, Session};
use crate::state::{AppState, LoadedDump};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::Instant;
use tauri::State;

#[derive(Serialize)]
pub struct ParseResult {
    pub creances: usize,
    pub regroupees: usize,
    pub uges: usize,
    pub etapes: usize,
    pub adresses: usize,
    pub corrupted: usize,
    pub elapsed_ms: u128,
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn parse_dump(path: String, state: State<'_, AppState>) -> Result<ParseResult, String> {
    let pb = PathBuf::from(&path);
    let size = std::fs::metadata(&pb)
        .map_err(|e| format!("metadata: {e}"))?
        .len();

    // SHA-256 in a separate pass to avoid holding two file handles.
    let sha = {
        let mut hasher = Sha256::new();
        let mut h_file = File::open(&pb).map_err(|e| format!("open: {e}"))?;
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = h_file.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        format!("{:x}", hasher.finalize())
    };

    let file = File::open(&pb).map_err(|e| format!("open: {e}"))?;
    let t0 = Instant::now();
    let dump = parse(file, |_b| {}).map_err(|e| format!("parse: {e}"))?;
    let elapsed_ms = t0.elapsed().as_millis();

    let result = ParseResult {
        creances: dump.creances.len(),
        regroupees: dump.creances_regroupees.len(),
        uges: dump.uges.len(),
        etapes: dump.etapes.len(),
        adresses: dump.adresses.len(),
        corrupted: dump.corrupted_rows,
        elapsed_ms,
    };
    *state.data.lock().unwrap() = Some(LoadedDump {
        dump,
        source_path: path,
        source_size: size,
        source_sha256: sha,
    });
    Ok(result)
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn list_uges(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    let mut s: BTreeSet<String> = d
        .dump
        .creances
        .iter()
        .map(|c| c.num_uge_gestion.clone())
        .collect();
    for u in &d.dump.uges {
        s.insert(u.num_uge.clone());
    }
    Ok(s.into_iter().collect())
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn list_natures(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    let s: BTreeSet<String> = d
        .dump
        .creances
        .iter()
        .map(|c| c.nature_compte.clone())
        .collect();
    Ok(s.into_iter().collect())
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn list_etapes(state: State<'_, AppState>) -> Result<Vec<(i32, String)>, String> {
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    let mut out: Vec<(i32, String)> = d
        .dump
        .etapes
        .iter()
        .map(|e| (e.id, e.libelle.clone()))
        .collect();
    out.sort_by_key(|(id, _)| *id);
    Ok(out)
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn list_statuts(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    let s: BTreeSet<String> = d
        .dump
        .creances
        .iter()
        .map(|c| c.statut_compte.clone())
        .collect();
    Ok(s.into_iter().collect())
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn count_filtered(
    filters: FilterSet,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    Ok(evaluate(&d.dump, &filters).len())
}

#[derive(Serialize)]
pub struct PreviewRow {
    pub cells: Vec<serde_json::Value>,
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn preview(
    filters: FilterSet,
    columns: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<PreviewRow>, String> {
    use crate::exporter::resolver::{resolve, ResolverContext};
    use crate::exporter::value::Value;
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    let rows = evaluate(&d.dump, &filters);
    let ctx = ResolverContext::build(&d.dump.uges, &d.dump.etapes, &d.dump.adresses);
    let out: Vec<PreviewRow> = rows
        .iter()
        .take(100)
        .map(|r| {
            let cells = columns
                .iter()
                .map(|c| match resolve(&ctx, r, c) {
                    Value::Empty => serde_json::Value::Null,
                    Value::Text(s) => serde_json::Value::String(s),
                    Value::Int(i) => serde_json::Value::Number(i.into()),
                    Value::Money(f) => serde_json::json!(f),
                    Value::Date(d) => serde_json::Value::String(d.format("%d/%m/%Y").to_string()),
                    Value::Bool(b) => {
                        serde_json::Value::String(if b { "Oui".into() } else { "Non".into() })
                    }
                })
                .collect();
            PreviewRow { cells }
        })
        .collect();
    Ok(out)
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn list_columns() -> Result<serde_json::Value, String> {
    let cat = catalog();
    let presets = serde_json::json!({
        "Standard CAMIEG": profile_standard_camieg(),
        "Complet": profile_complet(),
        "Minimal": profile_minimal(),
    });
    Ok(serde_json::json!({
        "columns": cat,
        "presets": presets,
    }))
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn load_profiles() -> Result<Profiles, String> {
    Ok(persistence::load_profiles())
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn save_profile(name: String, cols: Vec<String>) -> Result<(), String> {
    let mut p = persistence::load_profiles();
    p.profiles.insert(name, cols);
    persistence::save_profiles(&p)
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn delete_profile(name: String) -> Result<(), String> {
    let mut p = persistence::load_profiles();
    p.profiles.remove(&name);
    persistence::save_profiles(&p)
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn load_session() -> Result<Session, String> {
    Ok(persistence::load_session())
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn save_session(session: Session) -> Result<(), String> {
    persistence::save_session(&session)
}

#[allow(clippy::unused_async)]
#[tauri::command]
pub async fn export_xlsx(
    path: String,
    filters: FilterSet,
    columns: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let g = state.data.lock().unwrap();
    let d = g.as_ref().ok_or("dump not loaded")?;
    let rows = evaluate(&d.dump, &filters);
    let cols_refs: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();
    let ec = ExportContext {
        source_path: d.source_path.clone(),
        source_sha256: d.source_sha256.clone(),
        source_size: d.source_size,
        app_version: env!("CARGO_PKG_VERSION").into(),
        rows_read: d.dump.creances.len(),
        rows_after_filter: rows.len(),
        corrupted_rows: d.dump.corrupted_rows,
    };
    build_workbook(
        std::path::Path::new(&path),
        &d.dump,
        &rows,
        &cols_refs,
        &filters,
        &ec,
    )
    .map_err(|e| e.to_string())
}
