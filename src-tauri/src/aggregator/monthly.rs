use crate::filter::FilteredRow;
use chrono::{Datelike, NaiveDate};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MonthKey {
    pub year: i32,
    pub month: u32,
}

impl MonthKey {
    pub fn from_date(d: NaiveDate) -> Self {
        Self {
            year: d.year(),
            month: d.month(),
        }
    }
    pub fn label(&self) -> String {
        format!("{:04}-{:02}", self.year, self.month)
    }
}

#[derive(Debug, Default)]
pub struct MonthlyBucket<'a> {
    pub rows: Vec<&'a FilteredRow<'a>>,
}

#[derive(Debug, Default, Clone)]
pub struct SynthesisRow {
    pub month: Option<MonthKey>, // None = "Sans date pivot"
    pub uge: String,
    pub count: usize,
    pub somme_montant_initial: f64,
    pub somme_solde: f64,
}

pub fn group_by_month<'a>(
    rows: &'a [FilteredRow<'a>],
) -> BTreeMap<Option<MonthKey>, MonthlyBucket<'a>> {
    let mut out: BTreeMap<Option<MonthKey>, MonthlyBucket<'a>> = BTreeMap::new();
    for r in rows {
        let k = r.pivot_date.map(MonthKey::from_date);
        out.entry(k).or_default().rows.push(r);
    }
    out
}

pub fn synthesis(rows: &[FilteredRow<'_>]) -> Vec<SynthesisRow> {
    let mut map: BTreeMap<(Option<MonthKey>, String), SynthesisRow> = BTreeMap::new();
    for r in rows {
        let k = (
            r.pivot_date.map(MonthKey::from_date),
            r.creance.num_uge_gestion.clone(),
        );
        let entry = map.entry(k.clone()).or_insert_with(|| SynthesisRow {
            month: k.0,
            uge: k.1.clone(),
            ..Default::default()
        });
        entry.count += 1;
        entry.somme_montant_initial += r.creance.montant_initial;
        entry.somme_solde += r.creance.solde;
    }
    map.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::FilteredRow;
    use crate::schema::Creance;

    fn make_creance(id: i64, uge: &str, montant: f64) -> Creance {
        Creance {
            id,
            workflow: None,
            numero_creance: format!("C{id}"),
            date_der_ope: None,
            date_detect: None,
            nature_compte: "IND".into(),
            statut_compte: "NO".into(),
            gest_num: "G".into(),
            numero_debiteur: "D".into(),
            cat_debiteur: "C".into(),
            num_uge_gestion: uge.into(),
            montant_initial: montant,
            solde: montant,
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
            num_uge_detect: uge.into(),
            date_integration: None,
            flux: None,
            commentaire_creance: None,
            iduge: None,
            creanceregroupeeid: None,
            num_technicien: None,
            date_prescription: None,
        }
    }

    #[test]
    fn synthesis_aggregates_by_month_uge() {
        let c1 = make_creance(1, "9501", 100.0);
        let c2 = make_creance(2, "9501", 50.0);
        let c3 = make_creance(3, "9531", 200.0);
        let rows = vec![
            FilteredRow {
                creance: &c1,
                regroupee: None,
                pivot_date: NaiveDate::from_ymd_opt(2026, 1, 5),
            },
            FilteredRow {
                creance: &c2,
                regroupee: None,
                pivot_date: NaiveDate::from_ymd_opt(2026, 1, 20),
            },
            FilteredRow {
                creance: &c3,
                regroupee: None,
                pivot_date: NaiveDate::from_ymd_opt(2026, 2, 1),
            },
        ];
        let s = synthesis(&rows);
        assert_eq!(s.len(), 2);
        let r1 = s.iter().find(|r| r.uge == "9501").unwrap();
        assert_eq!(r1.count, 2);
        assert!((r1.somme_montant_initial - 150.0).abs() < 1e-6);
    }

    #[test]
    fn group_by_month_buckets() {
        let c1 = make_creance(1, "X", 10.0);
        let c2 = make_creance(2, "X", 20.0);
        let c3 = make_creance(3, "X", 30.0);
        let rows = vec![
            FilteredRow {
                creance: &c1,
                regroupee: None,
                pivot_date: NaiveDate::from_ymd_opt(2026, 1, 5),
            },
            FilteredRow {
                creance: &c2,
                regroupee: None,
                pivot_date: NaiveDate::from_ymd_opt(2026, 1, 20),
            },
            FilteredRow {
                creance: &c3,
                regroupee: None,
                pivot_date: NaiveDate::from_ymd_opt(2026, 2, 1),
            },
        ];
        let buckets = group_by_month(&rows);
        assert_eq!(buckets.len(), 2);
        let jan = MonthKey {
            year: 2026,
            month: 1,
        };
        assert_eq!(buckets[&Some(jan)].rows.len(), 2);
    }

    #[test]
    fn rows_without_pivot_grouped_as_none() {
        let c1 = make_creance(1, "X", 10.0);
        let rows = vec![FilteredRow {
            creance: &c1,
            regroupee: None,
            pivot_date: None,
        }];
        let buckets = group_by_month(&rows);
        assert!(buckets.contains_key(&None));
    }

    #[test]
    fn month_key_label() {
        let k = MonthKey {
            year: 2026,
            month: 3,
        };
        assert_eq!(k.label(), "2026-03");
    }
}
