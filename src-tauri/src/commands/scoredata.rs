use flate2::read::{GzDecoder, ZlibDecoder};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{io::Read, path::PathBuf, time::SystemTime};
use tauri::{AppHandle, Manager};

use std::collections::HashSet;

#[derive(serde::Serialize)]
pub struct NewRecord {
  pub id16_hex: String,
  pub index: usize,
  pub start_offset: usize,
  pub raw_hex: String,
}

#[derive(serde::Serialize)]
pub struct NewRecordResult {
  pub a_count: usize,
  pub b_count: usize,
  pub new_in_b: Vec<NewRecord>, // should usually be length 1
}

#[tauri::command]
pub fn find_new_record(a_path: String, b_path: String) -> Result<NewRecordResult, String> {
  let a = std::fs::read(&a_path).map_err(|e| e.to_string())?;
  let b = std::fs::read(&b_path).map_err(|e| e.to_string())?;

  // counts from header (u32 LE at offset 4)
  let a_count = u32_le(&a[4..8]) as usize;
  let b_count = u32_le(&b[4..8]) as usize;

  let mut a_ids = HashSet::<String>::new();

  // collect IDs from A
  for i in 0..a_count {
    let s = HEADER_LEN + i * RECORD_LEN;
    let e = s + RECORD_LEN;
    if e > a.len().saturating_sub(FOOTER_LEN) { break; }

    let rec = &a[s..e];
    let id16 = &rec[18..34];
    a_ids.insert(hex_bytes_local(id16));
  }

  // find IDs in B that are not in A
  let mut new_in_b = Vec::new();
  for i in 0..b_count {
    let s = HEADER_LEN + i * RECORD_LEN;
    let e = s + RECORD_LEN;
    if e > b.len().saturating_sub(FOOTER_LEN) { break; }

    let rec = &b[s..e];
    let id16 = &rec[18..34];
    let id_hex = hex_bytes_local(id16);

    if !a_ids.contains(&id_hex) {
      new_in_b.push(NewRecord {
        id16_hex: id_hex,
        index: i,
        start_offset: s,
        raw_hex: hex_bytes_local(rec),
      });
    }
  }

  Ok(NewRecordResult { a_count, b_count, new_in_b })
}

#[derive(Serialize)]
pub struct ScoredataMeta {
  pub bytes: u64,
  pub mtime_ms: Option<i64>,
  pub sha256: String,
}

#[derive(Serialize)]
pub struct ScoredataInspect {
  pub meta: ScoredataMeta,
  pub head_hex: String,
  pub is_gzip: bool,
  pub is_zlib: bool,
  pub is_zip: bool,
  pub ascii_samples: Vec<String>,
  pub decompressed_bytes: Option<u64>,
  pub ascii_samples_decompressed: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct ByteDiff {
  pub offset: usize,
  pub a: u8,
  pub b: u8,
}

#[derive(Serialize)]
pub struct DiffSummary {
  pub a_len: usize,
  pub b_len: usize,
  pub delta: isize,
  pub prefix_len: usize,
  pub suffix_len: usize,
  pub a_changed_start: usize,
  pub a_changed_end: usize,
  pub b_changed_start: usize,
  pub b_changed_end: usize,
}

#[derive(Serialize)]
pub struct InsertedBlock {
  pub delta: isize,
  pub prefix_len: usize,
  pub suffix_len: usize,
  pub insert_at_in_changed: usize,
  pub insert_at_in_file: usize,
  pub inserted_len: usize,
  pub inserted_hex: String,
}

const HEADER_LEN: usize = 26;
const RECORD_LEN: usize = 36;
const FOOTER_LEN: usize = 46;

fn u16_le(b: &[u8]) -> u16 { u16::from_le_bytes([b[0], b[1]]) }
fn u16_be(b: &[u8]) -> u16 { u16::from_be_bytes([b[0], b[1]]) }
fn u32_le(b: &[u8]) -> u32 { u32::from_le_bytes([b[0], b[1], b[2], b[3]]) }
fn u32_be(b: &[u8]) -> u32 { u32::from_be_bytes([b[0], b[1], b[2], b[3]]) }

fn hex_bytes(data: &[u8]) -> String {
  data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")
}

#[derive(serde::Serialize)]
pub struct ScoredataHeaderParsed {
  pub version_u32_le: u32,
  pub count_u32_le: u32,
  pub expected_len: usize,
  pub actual_len: usize,
}

#[derive(serde::Serialize)]
pub struct ScoreRecordParsed {
  pub index: usize,
  pub start_offset: usize,

  pub f0_u32_le: u32,

  // These are the next fields as BOTH endian interpretations (until we confirm which is correct)
  pub f1_u16_le: u16, pub f1_u16_be: u16,
  pub f2_u16_le: u16, pub f2_u16_be: u16,
  pub f3_u16_le: u16, pub f3_u16_be: u16,
  pub f4_u16_le: u16, pub f4_u16_be: u16,

  pub f5_u32_le: u32, pub f5_u32_be: u32,

  pub f6_u16_le: u16, pub f6_u16_be: u16,

  pub id16_hex: String,

  pub flags_u16_le: u16,
  pub flags_u16_be: u16,

  pub raw_hex: String,
}

#[tauri::command]
pub fn parse_scoredata_header(path: String) -> Result<ScoredataHeaderParsed, String> {
  let data = std::fs::read(&path).map_err(|e| e.to_string())?;
  if data.len() < HEADER_LEN + FOOTER_LEN {
    return Err("File too small to be scoredata.bin".to_string());
  }

  let version_u32_le = u32_le(&data[0..4]);
  let count_u32_le = u32_le(&data[4..8]);

  let expected_len = HEADER_LEN + (count_u32_le as usize) * RECORD_LEN + FOOTER_LEN;

  Ok(ScoredataHeaderParsed {
    version_u32_le,
    count_u32_le,
    expected_len,
    actual_len: data.len(),
  })
}

#[tauri::command]
pub fn list_scoredata_records(
  path: String,
  offset: Option<usize>,
  limit: Option<usize>,
) -> Result<Vec<ScoreRecordParsed>, String> {
  let data = std::fs::read(&path).map_err(|e| e.to_string())?;
  let header = parse_scoredata_header(path.clone())?;

  // Don’t hard-fail if it doesn’t match; just parse what we can safely
  let count = header.count_u32_le as usize;

  let start = offset.unwrap_or(0).min(count);
  let lim = limit.unwrap_or(25).min(200);
  let end = (start + lim).min(count);

  let mut out = Vec::new();

  for i in start..end {
    let rec_start = HEADER_LEN + i * RECORD_LEN;
    let rec_end = rec_start + RECORD_LEN;

    if rec_end > data.len().saturating_sub(FOOTER_LEN) {
      break;
    }

    let rec = &data[rec_start..rec_end];

    let f0 = u32_le(&rec[0..4]);

    let f1_le = u16_le(&rec[4..6]);  let f1_be = u16_be(&rec[4..6]);
    let f2_le = u16_le(&rec[6..8]);  let f2_be = u16_be(&rec[6..8]);
    let f3_le = u16_le(&rec[8..10]); let f3_be = u16_be(&rec[8..10]);
    let f4_le = u16_le(&rec[10..12]);let f4_be = u16_be(&rec[10..12]);

    let f5_le = u32_le(&rec[12..16]); let f5_be = u32_be(&rec[12..16]);

    let f6_le = u16_le(&rec[16..18]); let f6_be = u16_be(&rec[16..18]);

    let id16 = &rec[18..34];

    let flags_le = u16_le(&rec[34..36]);
    let flags_be = u16_be(&rec[34..36]);

    out.push(ScoreRecordParsed {
      index: i,
      start_offset: rec_start,

      f0_u32_le: f0,

      f1_u16_le: f1_le, f1_u16_be: f1_be,
      f2_u16_le: f2_le, f2_u16_be: f2_be,
      f3_u16_le: f3_le, f3_u16_be: f3_be,
      f4_u16_le: f4_le, f4_u16_be: f4_be,

      f5_u32_le: f5_le, f5_u32_be: f5_be,

      f6_u16_le: f6_le, f6_u16_be: f6_be,

      id16_hex: hex_bytes(id16),

      flags_u16_le: flags_le,
      flags_u16_be: flags_be,

      raw_hex: hex_bytes(rec),
    });
  }

  Ok(out)
}


// -------------------------
// Shared helpers
// -------------------------

fn common_prefix(a: &[u8], b: &[u8]) -> usize {
  let n = a.len().min(b.len());
  let mut i = 0;
  while i < n && a[i] == b[i] {
    i += 1;
  }
  i
}

fn common_suffix(a: &[u8], b: &[u8], prefix_len: usize) -> usize {
  let mut i = 0;
  let mut ai = a.len();
  let mut bi = b.len();

  while ai > prefix_len && bi > prefix_len {
    if a[ai - 1] != b[bi - 1] {
      break;
    }
    ai -= 1;
    bi -= 1;
    i += 1;
  }
  i
}

fn analyze_scoredata_inner(path: &str) -> Result<(ScoredataMeta, Vec<u8>), String> {
  let data = std::fs::read(path).map_err(|e| e.to_string())?;
  let bytes = data.len() as u64;

  let digest = Sha256::digest(&data);
  let sha256 = digest.iter().map(|b| format!("{:02x}", b)).collect();

  let mtime_ms = std::fs::metadata(path)
    .and_then(|m| m.modified())
    .ok()
    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
    .map(|d| d.as_millis() as i64);

  Ok((ScoredataMeta { bytes, mtime_ms, sha256 }, data))
}

fn head_hex(data: &[u8], n: usize) -> String {
  hex_bytes(&data[..data.len().min(n)])
}

fn extract_ascii_runs(data: &[u8], min_len: usize, max_items: usize) -> Vec<String> {
  let mut out = Vec::new();
  let mut buf: Vec<u8> = Vec::new();

  let flush = |buf: &mut Vec<u8>, out: &mut Vec<String>| {
    if buf.len() >= min_len {
      out.push(String::from_utf8_lossy(buf).to_string());
    }
    buf.clear();
  };

  for &b in data {
    let printable =
      b == b'\n' || b == b'\r' || b == b'\t' || (b >= 0x20 && b <= 0x7E);

    if printable {
      buf.push(b);
      if buf.len() > 2000 {
        flush(&mut buf, &mut out);
      }
    } else {
      flush(&mut buf, &mut out);
      if out.len() >= max_items {
        break;
      }
    }
  }

  if out.len() < max_items {
    flush(&mut buf, &mut out);
  }

  out.truncate(max_items);
  out
}

fn looks_like_gzip(data: &[u8]) -> bool {
  data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b
}

fn looks_like_zip(data: &[u8]) -> bool {
  data.len() >= 4
    && data[0] == b'P'
    && data[1] == b'K'
    && (data[2] == 0x03 || data[2] == 0x05 || data[2] == 0x07)
}

fn looks_like_zlib(data: &[u8]) -> bool {
  data.len() >= 2 && data[0] == 0x78 && matches!(data[1], 0x01 | 0x9c | 0xda)
}

fn try_decompress(data: &[u8]) -> Result<Option<Vec<u8>>, String> {
  const MAX_OUT: u64 = 25 * 1024 * 1024;

  if looks_like_gzip(data) {
    let dec = GzDecoder::new(data);
    let mut out = Vec::new();
    dec.take(MAX_OUT)
      .read_to_end(&mut out)
      .map_err(|e| e.to_string())?;
    return Ok(Some(out));
  }

  if looks_like_zlib(data) {
    let dec = ZlibDecoder::new(data);
    let mut out = Vec::new();
    dec.take(MAX_OUT)
      .read_to_end(&mut out)
      .map_err(|e| e.to_string())?;
    return Ok(Some(out));
  }

  Ok(None)
}

// -------------------------
// Commands
// -------------------------

#[tauri::command]
pub fn import_scoredata(path: String) -> Result<usize, String> {
  let (meta, _) = analyze_scoredata_inner(&path)?;
  Ok(meta.bytes as usize)
}

#[tauri::command]
pub fn analyze_scoredata(path: String) -> Result<ScoredataMeta, String> {
  let (meta, _) = analyze_scoredata_inner(&path)?;
  Ok(meta)
}

#[tauri::command]
pub fn inspect_scoredata(path: String) -> Result<ScoredataInspect, String> {
  let (meta, data) = analyze_scoredata_inner(&path)?;

  let is_gzip = looks_like_gzip(&data);
  let is_zlib = looks_like_zlib(&data);
  let is_zip = looks_like_zip(&data);

  let ascii_samples = extract_ascii_runs(&data, 6, 25);

  let decompressed = try_decompress(&data)?;
  let (decompressed_bytes, ascii_samples_decompressed) = match decompressed {
    Some(ref out) => {
      let samples = extract_ascii_runs(out, 6, 25);
      (Some(out.len() as u64), Some(samples))
    }
    None => (None, None),
  };

  Ok(ScoredataInspect {
    meta,
    head_hex: head_hex(&data, 64),
    is_gzip,
    is_zlib,
    is_zip,
    ascii_samples,
    decompressed_bytes,
    ascii_samples_decompressed,
  })
}

#[tauri::command]
pub fn snapshot_scoredata(app: AppHandle, path: String, sha256: String) -> Result<String, String> {
  let data = std::fs::read(&path).map_err(|e| e.to_string())?;

  let mut dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
  dir.push("snapshots");
  std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

  let out_path = dir.join(format!("scoredata-{}.bin", sha256));
  if !out_path.exists() {
    std::fs::write(&out_path, &data).map_err(|e| e.to_string())?;
  }

  Ok(out_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn diff_scoredata(
  a_path: String,
  b_path: String,
  limit: Option<usize>,
) -> Result<Vec<ByteDiff>, String> {
  let a = std::fs::read(&a_path).map_err(|e| e.to_string())?;
  let b = std::fs::read(&b_path).map_err(|e| e.to_string())?;

  let n = a.len().min(b.len());
  let cap = limit.unwrap_or(2000);

  let mut diffs = Vec::new();
  for i in 0..n {
    if a[i] != b[i] {
      diffs.push(ByteDiff {
        offset: i,
        a: a[i],
        b: b[i],
      });
      if diffs.len() >= cap {
        break;
      }
    }
  }

  Ok(diffs)
}

#[tauri::command]
pub fn diff_scoredata_summary(a_path: String, b_path: String) -> Result<DiffSummary, String> {
  let a = std::fs::read(&a_path).map_err(|e| e.to_string())?;
  let b = std::fs::read(&b_path).map_err(|e| e.to_string())?;

  let prefix_len = common_prefix(&a, &b);
  let suffix_len = common_suffix(&a, &b, prefix_len);

  let a_changed_start = prefix_len;
  let b_changed_start = prefix_len;

  let a_changed_end = a.len().saturating_sub(suffix_len);
  let b_changed_end = b.len().saturating_sub(suffix_len);

  Ok(DiffSummary {
    a_len: a.len(),
    b_len: b.len(),
    delta: b.len() as isize - a.len() as isize,
    prefix_len,
    suffix_len,
    a_changed_start,
    a_changed_end,
    b_changed_start,
    b_changed_end,
  })
}

#[tauri::command]
pub fn extract_inserted_block(a_path: String, b_path: String) -> Result<InsertedBlock, String> {
  let a = std::fs::read(&a_path).map_err(|e| e.to_string())?;
  let b = std::fs::read(&b_path).map_err(|e| e.to_string())?;

  let prefix_len = common_prefix(&a, &b);
  let suffix_len = common_suffix(&a, &b, prefix_len);

  let a_end = a.len().saturating_sub(suffix_len);
  let b_end = b.len().saturating_sub(suffix_len);

  let a_changed = &a[prefix_len..a_end];
  let b_changed = &b[prefix_len..b_end];

  let delta = b_changed.len() as isize - a_changed.len() as isize;
  if delta <= 0 {
    return Err(format!("Expected an insertion (delta > 0), got delta={}", delta));
  }
  let delta_u = delta as usize;

  let mut best_i = 0usize;
  let mut best_match = 0usize;

  for i in 0..=a_changed.len() {
    if i + delta_u > b_changed.len() {
      break;
    }

    let mut m = 0usize;
    while i + m < a_changed.len()
      && i + delta_u + m < b_changed.len()
      && a_changed[i + m] == b_changed[i + delta_u + m]
    {
      m += 1;
    }

    if m > best_match {
      best_match = m;
      best_i = i;
    }
  }

  let inserted = &b_changed[best_i..best_i + delta_u];

  Ok(InsertedBlock {
    delta,
    prefix_len,
    suffix_len,
    insert_at_in_changed: best_i,
    insert_at_in_file: prefix_len + best_i,
    inserted_len: inserted.len(),
    inserted_hex: hex_bytes(inserted),
  })
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
  let candidates = [base.join("scoredata.bin"), base.join("scores.bin")];

  for p in candidates {
    if p.exists() {
      return Ok(p.to_string_lossy().to_string());
    }
  }

  Err("No scoredata.bin (or scores.bin) found in that folder.".to_string())
}
