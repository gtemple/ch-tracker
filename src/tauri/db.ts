import Database from "@tauri-apps/plugin-sql";

const DB_URL = "sqlite:clonehero.db";

async function safeExecute(db: any, sql: string) {
  try {
    await db.execute(sql);
  } catch (e) {
    const msg = String(e);
    // Ignore "already exists" / "duplicate column" style errors
    if (
      msg.includes("duplicate column name") ||
      msg.includes("already exists")
    ) return;
    throw e;
  }
}

export async function initDb() {
  const db = await Database.load(DB_URL);

  // base table (works for first run)
  await db.execute(`
    CREATE TABLE IF NOT EXISTS import_runs (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      imported_at TEXT NOT NULL,
      path TEXT NOT NULL,
      bytes INTEGER NOT NULL
    )
  `);

  // migrations for older installs
  await safeExecute(db, `ALTER TABLE import_runs ADD COLUMN mtime_ms INTEGER`);
  await safeExecute(db, `ALTER TABLE import_runs ADD COLUMN sha256 TEXT`);

  // index after column exists
  await safeExecute(
    db,
    `CREATE INDEX IF NOT EXISTS idx_import_runs_sha ON import_runs(sha256)`
  );
  await safeExecute(db, `ALTER TABLE import_runs ADD COLUMN snapshot_path TEXT`);
  return db;
}
