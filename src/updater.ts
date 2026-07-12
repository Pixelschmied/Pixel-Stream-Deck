// Auto-update via the Tauri updater plugin. Checks the GitHub release endpoint
// configured in tauri.conf.json, downloads the signed update and relaunches.

import { inTauri } from "./api";

export interface AvailableUpdate {
  version: string;
  notes?: string;
  /** Download + install the update, then relaunch. Reports 0..100 progress. */
  install: (onProgress?: (percent: number) => void) => Promise<void>;
}

/** Returns an available update, or null if up to date / not in the desktop app. */
export async function checkForUpdate(): Promise<AvailableUpdate | null> {
  if (!inTauri()) return null;
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) return null;

  return {
    version: update.version,
    notes: update.body ?? undefined,
    install: async (onProgress) => {
      let total = 0;
      let received = 0;
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            total = event.data.contentLength ?? 0;
            onProgress?.(0);
            break;
          case "Progress":
            received += event.data.chunkLength;
            if (total > 0) onProgress?.(Math.round((received / total) * 100));
            break;
          case "Finished":
            onProgress?.(100);
            break;
        }
      });
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    },
  };
}
