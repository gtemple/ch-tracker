use std::path::PathBuf;

#[tauri::command]
pub fn import_scoredata(path: String) -> Result<usize, String> {
  let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
  Ok(bytes.len())
}

#[tauri::command]
pub fn detect_scoredata_path() -> Option<String> {
  let userprofile = std::env::var("USERPROFILE").ok()?;
  let p = PathBuf::from(userprofile)
    .join("AppData")
    .join("LocalLow")
    .join("srylain Inc_")
    .join("Clone Hero")
    .join("scoredata.bin");

  if p.exists() {
    Some(p.to_string_lossy().to_string())
  } else {
    None
  }
}

#[tauri::command]
pub fn find_score_file_in_dir(dir: String) -> Result<String, String> {
  let base = PathBuf::from(dir);

  let candidates = [
    base.join("scoredata.bin"),
    base.join("scores.bin"),
  ];

  for p in candidates {
    if p.exists() {
      return Ok(p.to_string_lossy().to_string());
    }
  }

  Err("No scoredata.bin (or scores.bin) found in that folder.".to_string())
}
