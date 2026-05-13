use crate::parser::copy_block::Row;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EtapeWorkflow {
    pub id: i32,
    pub libelle: String,
}

impl EtapeWorkflow {
    pub fn from_row(r: &Row) -> Result<Self, String> {
        Ok(Self {
            id: r.as_i32("id").ok_or("etapeworkflow.id missing")?,
            libelle: r.get("libelle").unwrap_or("").to_string(),
        })
    }
}
