use crate::parser::copy_block::Row;
use chrono::NaiveDate;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Creance {
    pub id: i64,
    pub workflow: Option<i32>,
    pub numero_creance: String,
    pub date_der_ope: Option<NaiveDate>,
    pub date_detect: Option<NaiveDate>,
    pub nature_compte: String,
    pub statut_compte: String,
    pub gest_num: String,
    pub numero_debiteur: String,
    pub cat_debiteur: String,
    pub num_uge_gestion: String,
    pub montant_initial: f64,
    pub solde: f64,
    pub part_mutuel: Option<f64>,
    pub type_prest: Option<String>,
    pub arc_det: Option<String>,
    pub nature_der_ope: Option<String>,
    pub matricule_assure: Option<String>,
    pub date_mandatement: Option<NaiveDate>,
    pub activite: Option<String>,
    pub num_compte: Option<String>,
    pub nom_assure: Option<String>,
    pub prenom_assure: Option<String>,
    pub num_uge_detect: String,
    pub date_integration: Option<NaiveDate>,
    pub flux: Option<String>,
    pub commentaire_creance: Option<String>,
    pub iduge: Option<i32>,
    pub creanceregroupeeid: Option<i64>,
    pub num_technicien: Option<String>,
    pub date_prescription: Option<NaiveDate>,
}

impl Creance {
    pub fn from_row(r: &Row) -> Result<Self, String> {
        Ok(Self {
            id: r.as_i64("id").ok_or("creance.id missing")?,
            workflow: r.as_i32("workflow"),
            numero_creance: r
                .get("numero_creance")
                .ok_or("numero_creance missing")?
                .to_string(),
            date_der_ope: r.as_date("date_der_ope"),
            date_detect: r.as_date("date_detect"),
            nature_compte: r.get("nature_compte").unwrap_or("").to_string(),
            statut_compte: r.get("statut_compte").unwrap_or("").to_string(),
            gest_num: r.get("gest_num").unwrap_or("").to_string(),
            numero_debiteur: r.get("numero_debiteur").unwrap_or("").to_string(),
            cat_debiteur: r.get("cat_debiteur").unwrap_or("").to_string(),
            num_uge_gestion: r.get("num_uge_gestion").unwrap_or("").to_string(),
            montant_initial: r.as_f64("montant_initial").unwrap_or(0.0),
            solde: r.as_f64("solde").unwrap_or(0.0),
            part_mutuel: r.as_f64("part_mutuel"),
            type_prest: r.get("type_prest").map(String::from),
            arc_det: r.get("arc_det").map(String::from),
            nature_der_ope: r.get("nature_der_ope").map(String::from),
            matricule_assure: r.get("matricule_assure").map(String::from),
            date_mandatement: r.as_date("date_mandatement"),
            activite: r.get("activite").map(String::from),
            num_compte: r.get("num_compte").map(String::from),
            nom_assure: r.get("nom_assure").map(String::from),
            prenom_assure: r.get("prenom_assure").map(String::from),
            num_uge_detect: r.get("num_uge_detect").unwrap_or("").to_string(),
            date_integration: r.as_date("date_integration"),
            flux: r.get("flux").map(String::from),
            commentaire_creance: r.get("commentaire_creance").map(String::from),
            iduge: r.as_i32("iduge"),
            creanceregroupeeid: r.as_i64("creanceregroupeeid"),
            num_technicien: r.get("num_technicien").map(String::from),
            date_prescription: r.as_date("date_prescription"),
        })
    }
}
