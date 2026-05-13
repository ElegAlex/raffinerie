use crate::aggregator::{MonthKey, MonthlyBucket};
use crate::catalog::catalog;
use crate::exporter::resolver::{resolve, ResolverContext};
use crate::exporter::value::{date_format, header_format, money_format, write};
use rust_xlsxwriter::{Workbook, XlsxError};
use std::collections::BTreeMap;

pub fn write_monthly_sheets<'a>(
    wb: &mut Workbook,
    ctx: &ResolverContext<'a>,
    grouped: &BTreeMap<Option<MonthKey>, MonthlyBucket<'a>>,
    columns: &[&str],
) -> Result<(), XlsxError> {
    let cat = catalog();
    let header_fmt = header_format();
    let money_fmt = money_format();
    let date_fmt = date_format();

    for (month_opt, bucket) in grouped {
        let sheet_name = match month_opt {
            Some(m) => m.label(),
            None => "Sans date".into(),
        };
        let ws = wb.add_worksheet();
        ws.set_name(sheet_name.as_str())?;
        // Header row
        for (i, col_id) in columns.iter().enumerate() {
            let label = cat
                .iter()
                .find(|c| c.id == *col_id)
                .map(|c| c.label)
                .unwrap_or(*col_id);
            ws.write_string_with_format(0, i as u16, label, &header_fmt)?;
        }
        ws.set_freeze_panes(1, 0)?;
        // Data rows
        for (row_idx, fr) in bucket.rows.iter().enumerate() {
            for (col_idx, col_id) in columns.iter().enumerate() {
                let v = resolve(ctx, fr, col_id);
                write(
                    ws,
                    (row_idx + 1) as u32,
                    col_idx as u16,
                    &v,
                    &money_fmt,
                    &date_fmt,
                )?;
            }
        }
        // Autofilter on data range
        if !bucket.rows.is_empty() && !columns.is_empty() {
            ws.autofilter(0, 0, bucket.rows.len() as u32, (columns.len() - 1) as u16)?;
        }
        // Column widths
        for i in 0..columns.len() {
            ws.set_column_width(i as u16, 18.0)?;
        }
    }
    Ok(())
}
