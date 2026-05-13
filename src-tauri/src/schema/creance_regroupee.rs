use crate::parser::copy_block::Row;
use chrono::NaiveDate;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CreanceRegroupee {
    pub id: i64,
    pub numero_reference: String,
    pub numero_debiteur: String,
    pub date_detection: Option<NaiveDate>,
    pub motif_notif: Option<String>,
    pub date_ar_notif_debiteur: Option<NaiveDate>,
    pub date_ar_mdm_debiteur: Option<NaiveDate>,
    pub commentaire_creance: Option<String>,
    pub etapewf: Option<i32>,
    pub is_douteux: bool,
    pub numero_og3s: Option<String>,
}

impl CreanceRegroupee {
    pub fn from_row(r: &Row) -> Result<Self, String> {
        Ok(Self {
            id: r.as_i64("id").ok_or("creance_regroupee.id missing")?,
            numero_reference: r.get("numero_reference").unwrap_or("").to_string(),
            numero_debiteur: r.get("numero_debiteur").unwrap_or("").to_string(),
            date_detection: r.as_date("date_detection"),
            motif_notif: r.get("motif_notif").map(String::from),
            date_ar_notif_debiteur: r.as_date("date_ar_notif_debiteur"),
            date_ar_mdm_debiteur: r.as_date("date_ar_mdm_debiteur"),
            commentaire_creance: r.get("commentaire_creance").map(String::from),
            etapewf: r.as_i32("etapewf"),
            is_douteux: r.as_bool("is_douteux").unwrap_or(false),
            numero_og3s: r.get("numero_og3s").map(String::from),
        })
    }
}
