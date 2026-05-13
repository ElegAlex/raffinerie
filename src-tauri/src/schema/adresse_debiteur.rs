use crate::parser::copy_block::Row;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AdresseDebiteur {
    pub numero_debiteur: String,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub adresse: Option<String>,
    pub code_postal: Option<String>,
    pub commune: Option<String>,
}

impl AdresseDebiteur {
    pub fn from_row(r: &Row) -> Result<Self, String> {
        Ok(Self {
            numero_debiteur: r
                .get("numero_debiteur")
                .ok_or("numero_debiteur missing")?
                .to_string(),
            nom: r.get("nom").map(String::from),
            prenom: r.get("prenom").map(String::from),
            adresse: r.get("adresse").map(String::from),
            code_postal: r.get("code_postal").map(String::from),
            commune: r.get("commune").map(String::from),
        })
    }
}
