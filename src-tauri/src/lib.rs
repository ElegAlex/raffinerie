pub mod aggregator;
pub mod catalog;
pub mod exporter;
pub mod filter;
pub mod ipc;
pub mod parser;
pub mod persistence;
pub mod schema;
pub mod state;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(state::AppState)
        .invoke_handler(tauri::generate_handler![
            ipc::parse_dump,
            ipc::list_uges,
            ipc::list_natures,
            ipc::list_etapes,
            ipc::list_statuts,
            ipc::count_filtered,
            ipc::preview,
            ipc::list_columns,
            ipc::load_profiles,
            ipc::save_profile,
            ipc::delete_profile,
            ipc::load_session,
            ipc::save_session,
            ipc::export_xlsx,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
