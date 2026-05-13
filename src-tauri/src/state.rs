use crate::parser::ParsedDump;
use std::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    pub data: Mutex<Option<LoadedDump>>,
}

pub struct LoadedDump {
    pub dump: ParsedDump,
    pub source_path: String,
    pub source_size: u64,
    pub source_sha256: String,
}
