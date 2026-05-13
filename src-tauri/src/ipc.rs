// Stub IPC handlers — will be implemented in Wave 4.
// Each returns a placeholder error so the app compiles.

use serde::Serialize;

#[derive(Serialize)]
pub struct ParseResult { pub creances: usize, pub elapsed_ms: u128 }

#[tauri::command] pub async fn parse_dump(_path: String) -> Result<ParseResult, String> { Err("not implemented".into()) }
#[tauri::command] pub async fn list_uges() -> Result<Vec<String>, String> { Err("ni".into()) }
#[tauri::command] pub async fn list_natures() -> Result<Vec<String>, String> { Err("ni".into()) }
#[tauri::command] pub async fn list_etapes() -> Result<Vec<(i32, String)>, String> { Err("ni".into()) }
#[tauri::command] pub async fn list_statuts() -> Result<Vec<String>, String> { Err("ni".into()) }
#[tauri::command] pub async fn count_filtered(_filters: serde_json::Value) -> Result<usize, String> { Err("ni".into()) }
#[tauri::command] pub async fn preview(_filters: serde_json::Value, _columns: Vec<String>) -> Result<serde_json::Value, String> { Err("ni".into()) }
#[tauri::command] pub async fn list_columns() -> Result<serde_json::Value, String> { Err("ni".into()) }
#[tauri::command] pub async fn load_profiles() -> Result<serde_json::Value, String> { Err("ni".into()) }
#[tauri::command] pub async fn save_profile(_name: String, _cols: Vec<String>) -> Result<(), String> { Err("ni".into()) }
#[tauri::command] pub async fn delete_profile(_name: String) -> Result<(), String> { Err("ni".into()) }
#[tauri::command] pub async fn load_session() -> Result<serde_json::Value, String> { Err("ni".into()) }
#[tauri::command] pub async fn save_session(_session: serde_json::Value) -> Result<(), String> { Err("ni".into()) }
#[tauri::command] pub async fn export_xlsx(_path: String, _filters: serde_json::Value, _columns: Vec<String>) -> Result<(), String> { Err("ni".into()) }
