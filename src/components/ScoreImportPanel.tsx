import { useEffect, useState } from "react";
import { open, message } from "@tauri-apps/plugin-dialog";
import {
  detectScoredataPath,
  findScoreFileInDir,
  analyzeScoredata,
  snapshotScoredata,
  diffScoredata,
  diffScoredataSummary,
  extractInsertedBlock,
  listScoredataRecords,
  type ByteDiff,
  type DiffSummary,
  type ScoreRecordParsed,
} from "../tauri/scoredata";
import { initDb } from "../tauri/db";

type ImportRun = {
  imported_at: string;
  path: string;
  bytes: number;
  snapshot_path: string | null;
  sha256: string | null;
  mtime_ms: number | null;
};

function toHex(n: number) {
  return "0x" + n.toString(16).padStart(4, "0");
}

export function ScoreImportPanel() {
  const [detectedPath, setDetectedPath] = useState<string | null>(null);
  const [history, setHistory] = useState<ImportRun[]>([]);
  const [isImporting, setIsImporting] = useState(false);
  const [diffSummary, setDiffSummary] = useState<string | null>(null);

  async function refreshHistory() {
    const db = await initDb();
    const rows = await db.select<ImportRun[]>(
      "SELECT imported_at, path, bytes, snapshot_path, sha256, mtime_ms FROM import_runs ORDER BY id DESC LIMIT 10"
    );
    setHistory(rows);
  }

  useEffect(() => {
    (async () => {
      const p = await detectScoredataPath();
      setDetectedPath(p);
      await refreshHistory();
    })();
  }, []);

async function importPath(path: string) {
  setIsImporting(true);
  try {
    const meta = await analyzeScoredata(path); // { bytes, mtime_ms, sha256 }
    const snapshotPath = await snapshotScoredata(path, meta.sha256);

    const db = await initDb();
    await db.execute(
      "INSERT INTO import_runs (imported_at, path, bytes, mtime_ms, sha256, snapshot_path) VALUES (?, ?, ?, ?, ?, ?)",
      [
        new Date().toISOString(),
        path,
        meta.bytes,
        meta.mtime_ms,
        meta.sha256,
        snapshotPath,
      ]
    );

    await refreshHistory();

    await message(`Imported ${meta.bytes} bytes.\n\nSnapshot:\n${snapshotPath}`, {
      title: "Import complete",
    });
  } finally {
    setIsImporting(false);
  }
}


async function compareLastTwoImports() {
  if (history.length < 2) return;

  const newest = history[0];
  const prev = history[1];

  setDiffSummary("Comparing…");

  try {
    const newestSnap = newest.snapshot_path;
    const prevSnap = prev.snapshot_path;

    if (!newestSnap || !prevSnap) {
      setDiffSummary(
        "Missing snapshots for one or both imports. Re-import twice after enabling snapshots."
      );
      return;
    }

    // NEW: get structural summary first
    const summary: DiffSummary = await diffScoredataSummary(prevSnap, newestSnap);
    const inserted = await extractInsertedBlock(prevSnap, newestSnap);

    // Existing: get first N byte diffs
    const diffs: ByteDiff[] = await diffScoredata(prevSnap, newestSnap, 500);

    const lines = [
      `A len: ${summary.a_len} bytes`,
      `B len: ${summary.b_len} bytes`,
      `Delta: ${summary.delta >= 0 ? "+" : ""}${summary.delta} bytes`,
      ``,
      `Common prefix: ${summary.prefix_len} (${toHex(summary.prefix_len)})`,
      `Common suffix: ${summary.suffix_len} (${toHex(summary.suffix_len)})`,
      ``,
      `A changed range: ${toHex(summary.a_changed_start)}..${toHex(summary.a_changed_end)} (${summary.a_changed_end - summary.a_changed_start} bytes)`,
      `B changed range: ${toHex(summary.b_changed_start)}..${toHex(summary.b_changed_end)} (${summary.b_changed_end - summary.b_changed_start} bytes)`,
      ``,
      `Prev snapshot: ${prevSnap}`,
      `New snapshot:  ${newestSnap}`,
      ``,
      `Diffs found (capped): ${diffs.length}`,
    ];

    if (diffs.length === 0) {
      lines.push("", "No differing bytes found within cap.");
      setDiffSummary(lines.join("\n"));
      return;
    }

    const first = diffs[0];
    const last = diffs[diffs.length - 1];

    lines.push(
      `First diff @ ${toHex(first.offset)}: ${first.a} -> ${first.b}`,
      `Last diff  @ ${toHex(last.offset)}: ${last.a} -> ${last.b}`,
      ``,
      `Sample (first 20):`,
      ...diffs.slice(0, 20).map((d) => `${toHex(d.offset)}: ${d.a} -> ${d.b}`)
    );

    lines.push(
  "",
  `Inserted block: ${inserted.inserted_len} bytes (delta=${inserted.delta})`,
  `Insert offset in new file: ${toHex(inserted.insert_at_in_file)} (${inserted.insert_at_in_file})`,
  `Inserted hex:`,
  inserted.inserted_hex
);

    const HEADER_LEN = 26;
    const RECORD_LEN = 36;

    const recIndex = Math.floor((inserted.insert_at_in_file - HEADER_LEN) / RECORD_LEN);
    const start = Math.max(recIndex - 2, 0);

    const prevRecs: ScoreRecordParsed[] = await listScoredataRecords(prevSnap, start, 6);
    const newRecs: ScoreRecordParsed[] = await listScoredataRecords(newestSnap, start, 6);

    const fmt = (r: ScoreRecordParsed) =>
      `#${r.index} @ ${toHex(r.start_offset)} id=${r.id16_hex} ` +
      `f1be=${r.f1_u16_be} f2be=${r.f2_u16_be} f3be=${r.f3_u16_be} f4be=${r.f4_u16_be} ` +
      `f5be=${r.f5_u32_be} f6be=${r.f6_u16_be} flags_be=${r.flags_u16_be}`;

    lines.push(
      "",
      `Inserted record index guess: ${recIndex} (showing ${start}..${start + 5})`,
      "",
      "Prev records:",
      ...prevRecs.map(fmt),
      "",
      "New records:",
      ...newRecs.map(fmt)
    );

    setDiffSummary(lines.join("\n"));
  } catch (err) {
    setDiffSummary(`Compare failed:\n${String(err)}`);
  }
}



  async function pickAndImportFile() {
    const path = await open({
      multiple: false,
      directory: false,
      title: "Select Clone Hero scoredata.bin",
      filters: [{ name: "Clone Hero", extensions: ["bin"] }],
      defaultPath: detectedPath ?? undefined,
    });

    if (!path || Array.isArray(path)) return;
    await importPath(path);
  }

  async function pickCloneHeroFolder() {
    const dir = await open({
      directory: true,
      multiple: false,
      title: "Select your Clone Hero folder",
    });

    if (!dir || Array.isArray(dir)) return;

    const scorePath = await findScoreFileInDir(dir);
    await importPath(scorePath);
  }

  return (
    <div style={{ display: "grid", gap: 12 }}>
      <div style={{ display: "grid", gap: 8 }}>
        {detectedPath ? (
          <>
            <div>
              Detected: <code>{detectedPath}</code>
            </div>
            <button
              type="button"
              disabled={isImporting}
              onClick={() => importPath(detectedPath)}
            >
              {isImporting ? "Importing…" : "Import detected scoredata.bin"}
            </button>
          </>
        ) : (
          <div>No default score file detected.</div>
        )}

        <button type="button" disabled={isImporting} onClick={pickCloneHeroFolder}>
          Select Clone Hero folder…
        </button>

        <button type="button" disabled={isImporting} onClick={pickAndImportFile}>
          Pick a .bin file…
        </button>

        <button
          type="button"
          disabled={history.length < 2}
          onClick={compareLastTwoImports}
        >
          Compare last two imports
        </button>
      </div>

      {diffSummary && (
        <pre style={{ whiteSpace: "pre-wrap", margin: 0 }}>
          {diffSummary}
        </pre>
      )}

      <div style={{ display: "grid", gap: 6 }}>
        <h3 style={{ margin: 0 }}>Recent imports</h3>

        {history.length === 0 ? (
          <div style={{ opacity: 0.8 }}>No imports yet.</div>
        ) : (
          <ul style={{ margin: 0, paddingLeft: 18 }}>
            {history.map((r) => (
              <li key={`${r.imported_at}-${r.path}`}>
                <div>
                  <strong>{new Date(r.imported_at).toLocaleString()}</strong>{" "}
                  — {r.bytes.toLocaleString()} bytes
                </div>
                <div style={{ opacity: 0.8 }}>
                  <code>{r.path}</code>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
