mod commands;

#[tauri::command]
fn greet(name: &str) -> String {
  format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_store::Builder::default().build())
    .plugin(tauri_plugin_sql::Builder::default().build())
    .invoke_handler(tauri::generate_handler![
    greet,
    commands::scoredata::import_scoredata,
    commands::scoredata::analyze_scoredata,
    commands::scoredata::inspect_scoredata,
    commands::scoredata::detect_scoredata_path,
    commands::scoredata::find_score_file_in_dir,
    commands::scoredata::diff_scoredata,
    commands::scoredata::snapshot_scoredata,
    commands::scoredata::diff_scoredata_summary,
    commands::scoredata::extract_inserted_block,
    commands::scoredata::parse_scoredata_header,
    commands::scoredata::list_scoredata_records,
    commands::scoredata::find_new_record,

    ])

    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
