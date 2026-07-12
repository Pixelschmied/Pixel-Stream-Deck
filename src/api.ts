// Thin wrapper around Tauri commands. When running outside the Tauri shell
// (e.g. `vite` in a plain browser for UI development), it falls back to an
// in-memory mock so the whole UI is still explorable.

import type { Action, Profile, ProfileSummary, Settings, Snapshot } from "./types";

/** True when the page is running inside the Tauri webview. */
export const inTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// ----- Browser mock -------------------------------------------------------

function mockSnapshot(): Snapshot {
  const none = { type: "none" } as Action;
  const keys = Array.from({ length: 8 }, (_, i) => ({
    label: ["Discord", "Spotify", "Steam", "Claude", "Browser", "Screenshot", "Mute", "Battle.net"][i] ?? "",
    color: ["#5865f2", "#1db954", "#66c0f4", "#cc785c", "#7ce0ff", "#9d7cff", "#ff6b6b", "#148eff"][i],
    action: none,
  }));
  const encoders = ["Volume", "Mic", "Bright", "Scene"].map((label) => ({
    label,
    on_press: none,
    on_turn_cw: none,
    on_turn_ccw: none,
    on_touch: none,
  }));
  const profiles: ProfileSummary[] = [
    { id: "default", name: "Default", has_rules: false },
    { id: "spotify", name: "Spotify", has_rules: true },
    { id: "steam", name: "Steam", has_rules: true },
    { id: "discord", name: "Discord", has_rules: true },
    { id: "battlenet", name: "Battle.net", has_rules: true },
    { id: "claude", name: "Claude", has_rules: true },
  ];
  return {
    productName: "Pixel Gaming Helper",
    version: "0.1.0",
    hardwareBuild: false,
    deckInfo: {
      model: "Stream Deck +",
      serial: "MOCK",
      key_count: 8,
      encoder_count: 4,
      key_image_size: [120, 120],
      touchstrip_size: [800, 100],
    },
    profiles,
    activeProfileId: "default",
    profile: { id: "default", name: "Default", pages: [{ id: "main", name: "Main", keys, encoders }], activates_for: [] },
    settings: {
      start_minimized: false,
      minimize_to_tray_on_close: true,
      launch_on_startup: false,
      brightness: 80,
      active_profile_id: "default",
      context_switching_enabled: false,
      theme: "system",
    },
  };
}

// ----- Public API ---------------------------------------------------------

export async function getSnapshot(): Promise<Snapshot> {
  if (!inTauri()) return mockSnapshot();
  return invoke<Snapshot>("get_snapshot");
}

export async function setActiveProfile(id: string): Promise<Profile | null> {
  if (!inTauri()) return null;
  return invoke<Profile>("set_active_profile", { id });
}

export async function saveProfile(profile: Profile): Promise<void> {
  if (!inTauri()) return;
  return invoke<void>("save_profile", { profile });
}

export async function saveSettings(settings: Settings): Promise<void> {
  if (!inTauri()) return;
  return invoke<void>("save_settings", { settings });
}

export async function runAction(action: Action): Promise<void> {
  if (!inTauri()) {
    // eslint-disable-next-line no-console
    console.info("[mock] runAction", action);
    return;
  }
  return invoke<void>("run_action", { action });
}
