import { LazyStore } from "@tauri-apps/plugin-store";

const store = new LazyStore("settings.json");

export async function getSavedScorePath(): Promise<string | null> {
  return (await store.get<string>("scorePath")) ?? null;
}

export async function setSavedScorePath(path: string) {
  await store.set("scorePath", path);
  await store.save();
}