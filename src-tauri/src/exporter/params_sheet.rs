use crate::exporter::value::header_format;
use crate::filter::FilterSet;
use chrono::Utc;
use rust_xlsxwriter::{Workbook, Worksheet, XlsxError};

pub struct ExportContext {
    pub source_path: String,
    pub source_sha256: String,
    pub source_size: u64,
    pub app_version: String,
    pub rows_read: usize,
    pub rows_after_filter: usize,
    pub corrupted_rows: usize,
}

fn put(
    ws: &mut Worksheet,
    r: &mut u32,
    k: &str,
    v: &str,
    h: &rust_xlsxwriter::Format,
) -> Result<(), XlsxError> {
    ws.write_string_with_format(*r, 0, k, h)?;
    ws.write_string(*r, 1, v)?;
    *r += 1;
    Ok(())
}

pub fn write_params(
    wb: &mut Workbook,
    filters: &FilterSet,
    columns: &[&str],
    ctx: &ExportContext,
) -> Result<(), XlsxError> {
    let ws = wb.add_worksheet();
    ws.set_name("Paramètres")?;
    let h = header_format();
    ws.set_column_width(0, 35.0)?;
    ws.set_column_width(1, 80.0)?;

    let mut row: u32 = 0;

    put(ws, &mut row, "Date d'export", &Utc::now().to_rfc3339(), &h)?;
    put(ws, &mut row, "Version raffinerie", &ctx.app_version, &h)?;
    put(ws, &mut row, "Dump source (chemin)", &ctx.source_path, &h)?;
    put(
        ws,
        &mut row,
        "Dump source (taille octets)",
        &ctx.source_size.to_string(),
        &h,
    )?;
    put(
        ws,
        &mut row,
        "Dump source (SHA-256)",
        &ctx.source_sha256,
        &h,
    )?;
    put(ws, &mut row, "Lignes lues", &ctx.rows_read.to_string(), &h)?;
    put(
        ws,
        &mut row,
        "Lignes après filtres",
        &ctx.rows_after_filter.to_string(),
        &h,
    )?;
    put(
        ws,
        &mut row,
        "Lignes corrompues skippées",
        &ctx.corrupted_rows.to_string(),
        &h,
    )?;
    row += 1;

    put(ws, &mut row, "Filtre — UGE", &filters.uges.join(", "), &h)?;
    put(
        ws,
        &mut row,
        "Filtre — Nature compte",
        &filters.nature_compte.join(", "),
        &h,
    )?;
    put(
        ws,
        &mut row,
        "Filtre — Commentaire contient",
        filters.commentaire_contient.as_deref().unwrap_or(""),
        &h,
    )?;
    put(
        ws,
        &mut row,
        "Filtre — Insensible casse/accents",
        &filters.commentaire_insensible.to_string(),
        &h,
    )?;
    put(
        ws,
        &mut row,
        "Filtre — Critère notification",
        &format!("{:?}", filters.notif_criterion),
        &h,
    )?;
    put(
        ws,
        &mut row,
        "Filtre — Date pivot",
        &format!("{:?}", filters.date_pivot),
        &h,
    )?;
    put(
        ws,
        &mut row,
        "Filtre — Date min",
        &filters.date_min.map(|d| d.to_string()).unwrap_or_default(),
        &h,
    )?;
    put(
        ws,
        &mut row,
        "Filtre — Date max",
        &filters.date_max.map(|d| d.to_string()).unwrap_or_default(),
        &h,
    )?;
    row += 1;

    put(ws, &mut row, "Colonnes exportées", &columns.join(", "), &h)?;
    Ok(())
}
