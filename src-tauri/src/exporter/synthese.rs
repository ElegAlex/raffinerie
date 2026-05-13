use crate::aggregator::{synthesis, MonthKey, SynthesisRow};
use crate::exporter::value::{header_format, money_format};
use crate::filter::FilteredRow;
use rust_xlsxwriter::{Workbook, XlsxError};
use std::collections::BTreeSet;

pub fn write_synthese(wb: &mut Workbook, rows: &[FilteredRow<'_>]) -> Result<(), XlsxError> {
    let data = synthesis(rows);
    let ws = wb.add_worksheet();
    ws.set_name("Synthèse")?;

    let header_fmt = header_format();
    let money_fmt = money_format();

    let uges: BTreeSet<String> = data.iter().map(|r| r.uge.clone()).collect();
    let months: BTreeSet<Option<MonthKey>> = data.iter().map(|r| r.month).collect();

    // Headers
    ws.write_string_with_format(0, 0, "Mois", &header_fmt)?;
    let mut col_idx = 1u16;
    let mut uge_col_map: std::collections::HashMap<String, u16> = std::collections::HashMap::new();
    for uge in &uges {
        uge_col_map.insert(uge.clone(), col_idx);
        ws.write_string_with_format(0, col_idx, format!("{uge} — Nb"), &header_fmt)?;
        ws.write_string_with_format(0, col_idx + 1, format!("{uge} — Σ montant"), &header_fmt)?;
        ws.write_string_with_format(0, col_idx + 2, format!("{uge} — Σ solde"), &header_fmt)?;
        col_idx += 3;
    }
    ws.set_freeze_panes(1, 1)?;

    // Index by (month, uge)
    let mut by_key: std::collections::HashMap<(Option<MonthKey>, String), SynthesisRow> =
        std::collections::HashMap::new();
    for r in data {
        by_key.insert((r.month, r.uge.clone()), r);
    }

    // Body
    for (i, m) in months.iter().enumerate() {
        let row = (i + 1) as u32;
        let label = m
            .as_ref()
            .map(|m| m.label())
            .unwrap_or_else(|| "Sans date".into());
        ws.write_string(row, 0, &label)?;
        for uge in &uges {
            let base = *uge_col_map.get(uge).unwrap();
            if let Some(s) = by_key.get(&(*m, uge.clone())) {
                ws.write_number(row, base, s.count as f64)?;
                ws.write_number_with_format(row, base + 1, s.somme_montant_initial, &money_fmt)?;
                ws.write_number_with_format(row, base + 2, s.somme_solde, &money_fmt)?;
            }
        }
    }
    Ok(())
}
