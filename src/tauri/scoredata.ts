import { invoke } from "@tauri-apps/api/core";

export type ScoredataInspect = {
  meta: { bytes: number; mtime_ms: number | null; sha256: string };
  head_hex: string;
  is_gzip: boolean;
  is_zlib: boolean;
  is_zip: boolean;
  ascii_samples: string[];
  decompressed_bytes: number | null;
  ascii_samples_decompressed: string[] | null;
};

export type DiffSummary = {
  a_len: number;
  b_len: number;
  delta: number;
  prefix_len: number;
  suffix_len: number;
  a_changed_start: number;
  a_changed_end: number;
  b_changed_start: number;
  b_changed_end: number;
};

export function diffScoredataSummary(aPath: string, bPath: string) {
  return invoke<DiffSummary>("diff_scoredata_summary", { aPath, bPath });
}


export type InsertedBlock = {
  delta: number;
  prefix_len: number;
  suffix_len: number;
  insert_at_in_changed: number;
  insert_at_in_file: number;
  inserted_len: number;
  inserted_hex: string;
};

export type ScoredataHeaderParsed = {
  version_u32_le: number;
  count_u32_le: number;
  expected_len: number;
  actual_len: number;
};

export type ScoreRecordParsed = {
  index: number;
  start_offset: number;

  f0_u32_le: number;

  f1_u16_le: number; f1_u16_be: number;
  f2_u16_le: number; f2_u16_be: number;
  f3_u16_le: number; f3_u16_be: number;
  f4_u16_le: number; f4_u16_be: number;

  f5_u32_le: number; f5_u32_be: number;

  f6_u16_le: number; f6_u16_be: number;

  id16_hex: string;

  flags_u16_le: number;
  flags_u16_be: number;

  raw_hex: string;
};

export function parseScoredataHeader(path: string) {
  return invoke<ScoredataHeaderParsed>("parse_scoredata_header", { path });
}

export function listScoredataRecords(path: string, offset?: number, limit?: number) {
  return invoke<ScoreRecordParsed[]>("list_scoredata_records", { path, offset, limit });
}

export function extractInsertedBlock(aPath: string, bPath: string) {
  return invoke<InsertedBlock>("extract_inserted_block", { aPath, bPath });
}



export type ScoredataMeta = {
  bytes: number;
  mtime_ms: number | null;
  sha256: string;
};

export function analyzeScoredata(path: string) {
  return invoke<ScoredataMeta>("analyze_scoredata", { path });
}

export function snapshotScoredata(path: string, sha256: string) {
  return invoke<string>("snapshot_scoredata", { path, sha256 });
}

export type ByteDiff = { offset: number; a: number; b: number };

export function diffScoredata(aPath: string, bPath: string, limit = 2000) {
  return invoke<ByteDiff[]>("diff_scoredata", { aPath, bPath, limit });
}

export async function detectScoredataPath(): Promise<string | null> {
  return invoke<string | null>("detect_scoredata_path");
}

export async function importScoredata(path: string): Promise<number> {
  return invoke<number>("import_scoredata", { path });
}

export async function findScoreFileInDir(dir: string): Promise<string> {
  return invoke<string>("find_score_file_in_dir", { dir });
}

export const inspectScoredata = (path: string) =>
  invoke<ScoredataInspect>("inspect_scoredata", { path });

