//! End-to-end pipeline test against the real sucre_939.dump.
//! Exercises: parse → filter (CAMIEG-like) → export XLSX → re-read.
//! Skipped unless RAFFINERIE_REAL_DUMP env var points to a readable file.

use calamine::{open_workbook, Reader, Xlsx};
use chrono::NaiveDate;
use raffinerie::aggregator::group_by_month;
use raffinerie::catalog::profile_standard_camieg;
use raffinerie::exporter::params_sheet::ExportContext;
use raffinerie::exporter::workbook::build_workbook;
use raffinerie::filter::{evaluate, DatePivot, FilterSet, NotifCriterion};
use raffinerie::parser::parse;
use std::fs::File;
use std::time::Instant;
use tempfile::NamedTempFile;

#[test]
fn full_pipeline_with_camieg_filter() {
    let path = match std::env::var("RAFFINERIE_REAL_DUMP") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("RAFFINERIE_REAL_DUMP not set, skipping.");
            return;
        }
    };

    // 1. PARSE
    let file = File::open(&path).expect("open dump");
    let t0 = Instant::now();
    let dump = parse(file, |_b| {}).expect("parse");
    eprintln!("parse: {:.2}s", t0.elapsed().as_secs_f64());

    // 2. FILTER — CAMIEG profile (UGE 9531 = Pôle Camieg, IND, commentaire indu roc, motif notif rempli)
    let fs = FilterSet {
        uges: vec!["9531".into()],
        nature_compte: vec!["IND".into()],
        commentaire_contient: Some("indu roc".into()),
        commentaire_insensible: true,
        notif_criterion: NotifCriterion::MotifNotifNonVide,
        date_pivot: DatePivot::DateIntegration,
        date_min: NaiveDate::from_ymd_opt(2026, 1, 1),
        date_max: NaiveDate::from_ymd_opt(2026, 12, 31),
    };
    let t1 = Instant::now();
    let rows = evaluate(&dump, &fs);
    eprintln!(
        "filter: {:.3}s → {} rows after CAMIEG filter",
        t1.elapsed().as_secs_f64(),
        rows.len()
    );

    // 3. EXPORT
    let tmp = NamedTempFile::new().unwrap().into_temp_path();
    let xlsx = tmp.with_extension("xlsx");
    let cols = profile_standard_camieg();
    let ctx = ExportContext {
        source_path: path.clone(),
        source_sha256: "test-skip-sha".into(),
        source_size: std::fs::metadata(&path).unwrap().len(),
        app_version: env!("CARGO_PKG_VERSION").into(),
        rows_read: dump.creances.len(),
        rows_after_filter: rows.len(),
        corrupted_rows: dump.corrupted_rows,
    };
    let t2 = Instant::now();
    build_workbook(&xlsx, &dump, &rows, &cols, &fs, &ctx).expect("build_workbook");
    eprintln!(
        "export: {:.3}s → {} ({} bytes)",
        t2.elapsed().as_secs_f64(),
        xlsx.display(),
        std::fs::metadata(&xlsx).unwrap().len()
    );

    // 4. RE-READ
    let wb: Xlsx<_> = open_workbook(&xlsx).expect("open xlsx");
    let sheets = wb.sheet_names();
    eprintln!("sheets: {:?}", sheets);
    assert!(sheets.iter().any(|n| n == "Synthèse"), "missing Synthèse");
    assert!(
        sheets.iter().any(|n| n == "Paramètres"),
        "missing Paramètres"
    );

    // Verify at least one monthly sheet exists if there are rows
    if !rows.is_empty() {
        let monthly_sheets: Vec<&String> = sheets
            .iter()
            .filter(|n| n.len() == 7 && n.chars().nth(4) == Some('-'))
            .collect();
        assert!(
            !monthly_sheets.is_empty(),
            "expected at least one YYYY-MM sheet, got: {sheets:?}"
        );
        eprintln!("monthly sheets: {monthly_sheets:?}");

        // Sanity check the monthly count vs aggregator output
        let grouped = group_by_month(&rows);
        let n_with_date = grouped.iter().filter(|(k, _)| k.is_some()).count();
        assert_eq!(
            monthly_sheets.len(),
            n_with_date,
            "monthly sheet count mismatch with aggregator"
        );
    }
}
