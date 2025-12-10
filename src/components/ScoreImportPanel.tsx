import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  detectScoredataPath,
  findScoreFileInDir,
  importScoredata,
} from "../tauri/scoredata";

export function ScoreImportPanel() {
  const [detectedPath, setDetectedPath] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      const p = await detectScoredataPath();
      setDetectedPath(p);
    })();
  }, []);

  async function importPath(path: string) {
    const size = await importScoredata(path);
    alert(`Read ${size} bytes from:\n${path}`);
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
    <div style={{ display: "grid", gap: 8 }}>
      {detectedPath ? (
        <>
          <div>
            Detected: <code>{detectedPath}</code>
          </div>

          <button
            type="button"
            onClick={async () => {
              try {
                console.log("Import detected clicked:", detectedPath);
                const size = await importScoredata(detectedPath!);
                alert(`Read ${size} bytes from:\n${detectedPath}`);
              } catch (err) {
                console.error(err);
                alert(`Import detected failed:\n${String(err)}`);
              }
            }}
          >
            Import detected scoredata.bin
          </button>
        </>
      ) : (
        <div>No default score file detected.</div>
      )}

      <button type="button" onClick={pickCloneHeroFolder}>
        Select Clone Hero folder…
      </button>

      <button type="button" onClick={pickAndImportFile}>
        Pick a .bin file…
      </button>
    </div>
  );
}
