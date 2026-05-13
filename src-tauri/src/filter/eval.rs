use crate::filter::set::{DatePivot, FilterSet, NotifCriterion};
use crate::parser::ParsedDump;
use crate::schema::{Creance, CreanceRegroupee};
use chrono::NaiveDate;
use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug)]
pub struct FilteredRow<'a> {
    pub creance: &'a Creance,
    pub regroupee: Option<&'a CreanceRegroupee>,
    pub pivot_date: Option<NaiveDate>,
}

pub fn evaluate<'a>(dump: &'a ParsedDump, fs: &FilterSet) -> Vec<FilteredRow<'a>> {
    let regroupee_idx: HashMap<i64, &CreanceRegroupee> =
        dump.creances_regroupees.iter().map(|r| (r.id, r)).collect();

    let needle = fs.commentaire_contient.as_ref().map(|s| {
        if fs.commentaire_insensible {
            normalize(s)
        } else {
            s.clone()
        }
    });

    dump.creances
        .iter()
        .filter_map(|c| {
            // Match UGE on either gestion or detection. Reason: dans SUCRE, les indus
            // ROC ont num_uge_gestion="0" mais num_uge_detect porte la vraie UGE métier
            // (ex. 9531 = Pôle Camieg). Filtrer uniquement sur gestion masquerait ces lignes.
            if !fs.uges.is_empty()
                && !fs.uges.contains(&c.num_uge_gestion)
                && !fs.uges.contains(&c.num_uge_detect)
            {
                return None;
            }
            if !fs.nature_compte.is_empty() && !fs.nature_compte.contains(&c.nature_compte) {
                return None;
            }

            let regr = c
                .creanceregroupeeid
                .and_then(|id| regroupee_idx.get(&id).copied());

            if let Some(n) = &needle {
                let hay_raw = regr
                    .and_then(|r| r.commentaire_creance.as_deref())
                    .unwrap_or("");
                let hay = if fs.commentaire_insensible {
                    normalize(hay_raw)
                } else {
                    hay_raw.to_string()
                };
                if !hay.contains(n.as_str()) {
                    return None;
                }
            }

            match &fs.notif_criterion {
                NotifCriterion::Aucun => {}
                NotifCriterion::MotifNotifNonVide => {
                    regr.and_then(|r| r.motif_notif.as_deref())
                        .filter(|s| !s.is_empty())?;
                }
                NotifCriterion::DateArNotifNonVide => {
                    regr.and_then(|r| r.date_ar_notif_debiteur)?;
                }
                NotifCriterion::EtapeWfDans { ids } => {
                    let etape = regr.and_then(|r| r.etapewf);
                    if !etape.map(|e| ids.contains(&e)).unwrap_or(false) {
                        return None;
                    }
                }
                NotifCriterion::StatutCompteDans { values } => {
                    if !values.contains(&c.statut_compte) {
                        return None;
                    }
                }
            }

            let pivot = pivot_date(c, regr, fs.date_pivot);

            if let Some(min) = fs.date_min {
                if pivot.map(|d| d < min).unwrap_or(true) {
                    return None;
                }
            }
            if let Some(max) = fs.date_max {
                if pivot.map(|d| d > max).unwrap_or(true) {
                    return None;
                }
            }

            Some(FilteredRow {
                creance: c,
                regroupee: regr,
                pivot_date: pivot,
            })
        })
        .collect()
}

fn pivot_date(c: &Creance, r: Option<&CreanceRegroupee>, pivot: DatePivot) -> Option<NaiveDate> {
    match pivot {
        DatePivot::DateDetect => c.date_detect,
        DatePivot::DateIntegration => c.date_integration,
        DatePivot::DateDerOpe => c.date_der_ope,
        DatePivot::DateMandatement => c.date_mandatement,
        DatePivot::DateArNotifDebiteur => r.and_then(|r| r.date_ar_notif_debiteur),
        DatePivot::DateDetectionRegroupee => r.and_then(|r| r.date_detection),
    }
}

fn normalize(s: &str) -> String {
    s.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::*;

    fn dummy_dump() -> ParsedDump {
        let mut d = ParsedDump::default();
        d.creances_regroupees.push(CreanceRegroupee {
            id: 1,
            numero_reference: "REF1".into(),
            numero_debiteur: "D1".into(),
            date_detection: None,
            motif_notif: Some("Notification envoyée".into()),
            date_ar_notif_debiteur: None,
            date_ar_mdm_debiteur: None,
            commentaire_creance: Some("Indu ROC".into()),
            etapewf: Some(79),
            is_douteux: false,
            numero_og3s: None,
        });
        d.creances_regroupees.push(CreanceRegroupee {
            id: 2,
            numero_reference: "REF2".into(),
            numero_debiteur: "D2".into(),
            date_detection: None,
            motif_notif: None,
            date_ar_notif_debiteur: None,
            date_ar_mdm_debiteur: None,
            commentaire_creance: Some("Autre".into()),
            etapewf: Some(1),
            is_douteux: false,
            numero_og3s: None,
        });
        for (i, regr_id) in [(101_i64, 1_i64), (102, 2)].iter() {
            d.creances.push(Creance {
                id: *i,
                workflow: None,
                numero_creance: format!("C{}", i),
                date_der_ope: None,
                date_detect: None,
                nature_compte: "IND".into(),
                statut_compte: "NO".into(),
                gest_num: "G".into(),
                numero_debiteur: "D".into(),
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
                date_integration: NaiveDate::from_ymd_opt(2026, 3, 15),
                flux: None,
                commentaire_creance: None,
                iduge: None,
                creanceregroupeeid: Some(*regr_id),
                num_technicien: None,
                date_prescription: None,
            });
        }
        d
    }

    #[test]
    fn no_filter_returns_all() {
        let d = dummy_dump();
        let r = evaluate(&d, &FilterSet::default());
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn uge_filter() {
        let d = dummy_dump();
        let fs = FilterSet {
            uges: vec!["9501".into()],
            ..Default::default()
        };
        assert_eq!(evaluate(&d, &fs).len(), 2);
        let fs = FilterSet {
            uges: vec!["9999".into()],
            ..Default::default()
        };
        assert_eq!(evaluate(&d, &fs).len(), 0);
    }

    #[test]
    fn uge_filter_matches_on_detect_field_too() {
        // Reproduit le cas CAMIEG : num_uge_gestion="0", num_uge_detect="9531"
        let mut d = ParsedDump::default();
        d.creances.push(Creance {
            id: 1,
            workflow: None,
            numero_creance: "C1".into(),
            date_der_ope: None,
            date_detect: None,
            nature_compte: "IND".into(),
            statut_compte: "N".into(),
            gest_num: "G".into(),
            numero_debiteur: "D".into(),
            cat_debiteur: "C".into(),
            num_uge_gestion: "0".into(),
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
            num_uge_detect: "9531".into(),
            date_integration: None,
            flux: None,
            commentaire_creance: None,
            iduge: None,
            creanceregroupeeid: None,
            num_technicien: None,
            date_prescription: None,
        });
        let fs = FilterSet {
            uges: vec!["9531".into()],
            ..Default::default()
        };
        assert_eq!(
            evaluate(&d, &fs).len(),
            1,
            "filter UGE 9531 should match num_uge_detect even when num_uge_gestion='0'"
        );
    }

    #[test]
    fn commentaire_filter_case_accent_insensitive() {
        let d = dummy_dump();
        let fs = FilterSet {
            commentaire_contient: Some("INDU rOC".into()),
            commentaire_insensible: true,
            ..Default::default()
        };
        let r = evaluate(&d, &fs);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].creance.id, 101);
    }

    #[test]
    fn motif_notif_filter() {
        let d = dummy_dump();
        let fs = FilterSet {
            notif_criterion: NotifCriterion::MotifNotifNonVide,
            ..Default::default()
        };
        let r = evaluate(&d, &fs);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].creance.id, 101);
    }

    #[test]
    fn date_range_with_pivot_integration() {
        let d = dummy_dump();
        let fs = FilterSet {
            date_pivot: DatePivot::DateIntegration,
            date_min: NaiveDate::from_ymd_opt(2026, 1, 1),
            date_max: NaiveDate::from_ymd_opt(2026, 12, 31),
            ..Default::default()
        };
        assert_eq!(evaluate(&d, &fs).len(), 2);
        let fs2 = FilterSet {
            date_pivot: DatePivot::DateIntegration,
            date_min: NaiveDate::from_ymd_opt(2027, 1, 1),
            ..Default::default()
        };
        assert_eq!(evaluate(&d, &fs2).len(), 0);
    }
}
