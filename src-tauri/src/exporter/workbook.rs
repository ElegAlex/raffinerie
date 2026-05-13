use crate::aggregator::group_by_month;
use crate::exporter::monthly_sheet::write_monthly_sheets;
use crate::exporter::params_sheet::{write_params, ExportContext};
use crate::exporter::resolver::ResolverContext;
use crate::exporter::synthese::write_synthese;
use crate::filter::{FilterSet, FilteredRow};
use crate::parser::ParsedDump;
use rust_xlsxwriter::{Workbook, XlsxError};
use std::path::Path;

pub fn build_workbook(
    path: &Path,
    dump: &ParsedDump,
    rows: &[FilteredRow<'_>],
    columns: &[&str],
    filters: &FilterSet,
    export_ctx: &ExportContext,
) -> Result<(), XlsxError> {
    let mut wb = Workbook::new();
    let ctx = ResolverContext::build(&dump.uges, &dump.etapes, &dump.adresses);
    write_synthese(&mut wb, rows)?;
    let grouped = group_by_month(rows);
    write_monthly_sheets(&mut wb, &ctx, &grouped, columns)?;
    write_params(&mut wb, filters, columns, export_ctx)?;
    wb.save(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::evaluate;
    use crate::schema::Creance;
    use calamine::{open_workbook, Reader, Xlsx};
    use chrono::NaiveDate;
    use tempfile::NamedTempFile;

    fn mini_dump_with_one() -> ParsedDump {
        let mut d = ParsedDump::default();
        d.creances.push(Creance {
            id: 1,
            workflow: None,
            numero_creance: "C1".into(),
            date_der_ope: None,
            date_detect: NaiveDate::from_ymd_opt(2026, 1, 5),
            nature_compte: "IND".into(),
            statut_compte: "NO".into(),
            gest_num: "G".into(),
            numero_debiteur: "D1".into(),
            cat_debiteur: "C".into(),
            num_uge_gestion: "9501".into(),
            montant_initial: 100.0,
            solde: 100.0,
            part_mutuel: None,
            type_prest: None,
            arc_det: None,
            nature_der_ope: None,
            matricule_assure: None,
            date_mandatement: None,
            activite: None,
            num_compte: None,
            nom_assure: None,
            prenom_assure: None,
            num_uge_detect: "9501".into(),
            date_integration: NaiveDate::from_ymd_opt(2026, 1, 5),
            flux: None,
            commentaire_creance: None,
            iduge: None,
            creanceregroupeeid: None,
            num_technicien: None,
            date_prescription: None,
        });
        d
    }

    #[test]
    fn writes_workbook_with_expected_sheets() {
        let d = mini_dump_with_one();
        let fs = FilterSet::default();
        let rows = evaluate(&d, &fs);
        let tmp = NamedTempFile::new().unwrap().into_temp_path();
        let path = tmp.with_extension("xlsx");
        let cols = ["numero_creance", "montant_initial"];
        let ec = ExportContext {
            source_path: "test.dump".into(),
            source_sha256: "n/a".into(),
            source_size: 0,
            app_version: "0.0.0".into(),
            rows_read: 1,
            rows_after_filter: 1,
            corrupted_rows: 0,
        };
        build_workbook(&path, &d, &rows, &cols, &fs, &ec).unwrap();
        let wb: Xlsx<_> = open_workbook(&path).unwrap();
        let names = wb.sheet_names();
        assert!(
            names.iter().any(|n| n == "Synthèse"),
            "missing Synthèse, got: {names:?}"
        );
        assert!(
            names.iter().any(|n| n == "2026-01"),
            "missing 2026-01, got: {names:?}"
        );
        assert!(
            names.iter().any(|n| n == "Paramètres"),
            "missing Paramètres, got: {names:?}"
        );
    }
}
