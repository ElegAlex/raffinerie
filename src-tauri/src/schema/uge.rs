use crate::parser::copy_block::Row;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Uge {
    pub id: i32,
    pub num_uge: String,
    pub libelle: Option<String>,
}

impl Uge {
    pub fn from_row(r: &Row) -> Result<Self, String> {
        Ok(Self {
            id: r.as_i32("id").ok_or("uge.id missing")?,
            num_uge: r.get("num_uge").ok_or("num_uge missing")?.to_string(),
            libelle: r.get("libelle").map(String::from),
        })
    }
}
