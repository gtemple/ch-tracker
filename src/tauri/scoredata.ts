import { invoke } from "@tauri-apps/api/core";

export async function detectScoredataPath(): Promise<string | null> {
  return invoke<string | null>("detect_scoredata_path");
}

export async function importScoredata(path: string): Promise<number> {
  return invoke<number>("import_scoredata", { path });
}

export async function findScoreFileInDir(dir: string): Promise<string> {
  return invoke<string>("find_score_file_in_dir", { dir });
}
