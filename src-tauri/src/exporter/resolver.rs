use crate::exporter::value::Value;
use crate::filter::FilteredRow;
use crate::schema::{AdresseDebiteur, EtapeWorkflow, Uge};
use std::collections::HashMap;

pub struct ResolverContext<'a> {
    pub uges_by_num: HashMap<&'a str, &'a Uge>,
    pub etapes_by_id: HashMap<i32, &'a EtapeWorkflow>,
    pub adresses_by_debiteur: HashMap<&'a str, &'a AdresseDebiteur>,
}

impl<'a> ResolverContext<'a> {
    pub fn build(
        uges: &'a [Uge],
        etapes: &'a [EtapeWorkflow],
        adresses: &'a [AdresseDebiteur],
    ) -> Self {
        Self {
            uges_by_num: uges.iter().map(|u| (u.num_uge.as_str(), u)).collect(),
            etapes_by_id: etapes.iter().map(|e| (e.id, e)).collect(),
            adresses_by_debiteur: adresses
                .iter()
                .map(|a| (a.numero_debiteur.as_str(), a))
                .collect(),
        }
    }
}

pub fn resolve(ctx: &ResolverContext, row: &FilteredRow, col_id: &str) -> Value {
    let c = row.creance;
    let r = row.regroupee;
    match col_id {
        // Créance
        "numero_creance" => Value::Text(c.numero_creance.clone()),
        "nature_compte" => Value::Text(c.nature_compte.clone()),
        "statut_compte" => Value::Text(c.statut_compte.clone()),
        "montant_initial" => Value::Money(c.montant_initial),
        "solde" => Value::Money(c.solde),
        "part_mutuel" => c.part_mutuel.map(Value::Money).unwrap_or(Value::Empty),
        "type_prest" => opt_text(&c.type_prest),
        "arc_det" => opt_text(&c.arc_det),
        "nature_der_ope" => opt_text(&c.nature_der_ope),
        "flux" => opt_text(&c.flux),
        "activite" => opt_text(&c.activite),
        "num_compte" => opt_text(&c.num_compte),
        "commentaire_creance" => opt_text(&c.commentaire_creance),
        "num_technicien" => opt_text(&c.num_technicien),
        // Dates
        "date_detect" => c.date_detect.map(Value::Date).unwrap_or(Value::Empty),
        "date_integration" => c.date_integration.map(Value::Date).unwrap_or(Value::Empty),
        "date_der_ope" => c.date_der_ope.map(Value::Date).unwrap_or(Value::Empty),
        "date_mandatement" => c.date_mandatement.map(Value::Date).unwrap_or(Value::Empty),
        "date_prescription" => c.date_prescription.map(Value::Date).unwrap_or(Value::Empty),
        // UGE
        "num_uge_gestion" => Value::Text(c.num_uge_gestion.clone()),
        "num_uge_detect" => Value::Text(c.num_uge_detect.clone()),
        "libelle_uge" => ctx
            .uges_by_num
            .get(c.num_uge_gestion.as_str())
            .and_then(|u| u.libelle.clone())
            .map(Value::Text)
            .unwrap_or(Value::Empty),
        // Débiteur
        "numero_debiteur" => Value::Text(c.numero_debiteur.clone()),
        "cat_debiteur" => Value::Text(c.cat_debiteur.clone()),
        "nom_assure" => opt_text(&c.nom_assure),
        "prenom_assure" => opt_text(&c.prenom_assure),
        "matricule_assure" => opt_text(&c.matricule_assure),
        "adresse_postale" => ctx
            .adresses_by_debiteur
            .get(c.numero_debiteur.as_str())
            .and_then(|a| a.adresse.clone())
            .map(Value::Text)
            .unwrap_or(Value::Empty),
        "code_postal" => ctx
            .adresses_by_debiteur
            .get(c.numero_debiteur.as_str())
            .and_then(|a| a.code_postal.clone())
            .map(Value::Text)
            .unwrap_or(Value::Empty),
        "commune" => ctx
            .adresses_by_debiteur
            .get(c.numero_debiteur.as_str())
            .and_then(|a| a.commune.clone())
            .map(Value::Text)
            .unwrap_or(Value::Empty),
        // Regroupée
        "numero_reference" => r
            .map(|r| Value::Text(r.numero_reference.clone()))
            .unwrap_or(Value::Empty),
        "commentaire_creance_regroupee" => r
            .and_then(|r| r.commentaire_creance.clone())
            .map(Value::Text)
            .unwrap_or(Value::Empty),
        "motif_notif" => r
            .and_then(|r| r.motif_notif.clone())
            .map(Value::Text)
            .unwrap_or(Value::Empty),
        "date_detection_regroupee" => r
            .and_then(|r| r.date_detection)
            .map(Value::Date)
            .unwrap_or(Value::Empty),
        "date_ar_notif_debiteur" => r
            .and_then(|r| r.date_ar_notif_debiteur)
            .map(Value::Date)
            .unwrap_or(Value::Empty),
        "date_ar_mdm_debiteur" => r
            .and_then(|r| r.date_ar_mdm_debiteur)
            .map(Value::Date)
            .unwrap_or(Value::Empty),
        "etapewf" => r
            .and_then(|r| r.etapewf)
            .map(|i| Value::Int(i as i64))
            .unwrap_or(Value::Empty),
        "libelle_etape" => r
            .and_then(|r| r.etapewf)
            .and_then(|id| ctx.etapes_by_id.get(&id).map(|e| e.libelle.clone()))
            .map(Value::Text)
            .unwrap_or(Value::Empty),
        "is_douteux" => r.map(|r| Value::Bool(r.is_douteux)).unwrap_or(Value::Empty),
        "numero_og3s" => r
            .and_then(|r| r.numero_og3s.clone())
            .map(Value::Text)
            .unwrap_or(Value::Empty),
        _ => Value::Empty,
    }
}

fn opt_text(s: &Option<String>) -> Value {
    s.clone().map(Value::Text).unwrap_or(Value::Empty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::FilteredRow;
    use crate::schema::Creance;
    use chrono::NaiveDate;

    #[test]
    fn resolves_basic_columns() {
        let c = Creance {
            id: 1,
            workflow: None,
            numero_creance: "X1".into(),
            date_der_ope: None,
            date_detect: NaiveDate::from_ymd_opt(2026, 1, 1),
            nature_compte: "IND".into(),
            statut_compte: "NO".into(),
            gest_num: "G".into(),
            numero_debiteur: "D".into(),
            cat_debiteur: "C".into(),
            num_uge_gestion: "9501".into(),
            montant_initial: 99.99,
            solde: 50.0,
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
            date_integration: None,
            flux: None,
            commentaire_creance: None,
            iduge: None,
            creanceregroupeeid: None,
            num_technicien: None,
            date_prescription: None,
        };
        let row = FilteredRow {
            creance: &c,
            regroupee: None,
            pivot_date: None,
        };
        let ctx = ResolverContext::build(&[], &[], &[]);
        match resolve(&ctx, &row, "numero_creance") {
            Value::Text(s) => assert_eq!(s, "X1"),
            _ => panic!("wrong variant"),
        }
        match resolve(&ctx, &row, "montant_initial") {
            Value::Money(f) => assert!((f - 99.99).abs() < 1e-6),
            _ => panic!("wrong variant"),
        }
        assert!(matches!(resolve(&ctx, &row, "libelle_uge"), Value::Empty));
        assert!(matches!(
            resolve(&ctx, &row, "unknown_column"),
            Value::Empty
        ));
    }
}
