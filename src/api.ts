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

const NONE = { type: "none" } as Action;

/** Build a mock profile: 8 key labels sharing one colour + 4 encoder labels. */
function mockProfile(
  id: string,
  name: string,
  color: string,
  keyLabels: string[],
  encLabels: string[]
): Profile {
  const keys = Array.from({ length: 8 }, (_, i) => ({ label: keyLabels[i] ?? "", color, action: NONE }));
  const encoders = Array.from({ length: 4 }, (_, i) => ({
    label: encLabels[i] ?? "",
    on_press: NONE,
    on_turn_cw: NONE,
    on_turn_ccw: NONE,
    on_touch: NONE,
  }));
  return { id, name, pages: [{ id: "main", name: "Main", keys, encoders }], activates_for: [] };
}

const MOCK_PROFILES: Record<string, Profile> = {
  default: mockProfile("default", "Default", "#3a3d52",
    ["Discord", "Spotify", "Steam", "Claude", "Browser", "Screenshot", "Mute", "Battle.net"],
    ["Volume", "Bright", "Mic", "Scene"]),
  spotify: mockProfile("spotify", "Spotify", "#1db954",
    ["Play/Pause", "Next", "Prev", "Like", "Shuffle", "Repeat", "Search", "Open Spotify"],
    ["Volume", "Seek", "Bright", "Mute"]),
  steam: mockProfile("steam", "Steam", "#66c0f4",
    ["Library", "Big Picture", "Friends", "Store", "Downloads", "Screenshot", "Overlay", "Open Steam"],
    ["Volume", "Bright", "Mic", "Scene"]),
  discord: mockProfile("discord", "Discord", "#5865f2",
    ["Mute", "Deafen", "Video", "Screen", "Disconnect", "Overlay", "Emoji", "Open Discord"],
    ["Volume", "Mic", "Bright", "Scroll"]),
  battlenet: mockProfile("battlenet", "Battle.net", "#148eff",
    ["Launcher", "Friends", "Shop", "News", "Screenshot", "Mute", "Discord", "Record"],
    ["Volume", "Bright", "Mic", "Scene"]),
  claude: mockProfile("claude", "Claude", "#cc785c",
    ["New Chat", "Open Claude", "Projects", "Copy", "Paste", "Search", "Sidebar", "Send"],
    ["Volume", "Bright", "Scroll", "Zoom"]),
};

function mockSnapshot(): Snapshot {
  const profiles: ProfileSummary[] = Object.values(MOCK_PROFILES).map((p) => ({
    id: p.id,
    name: p.name,
    has_rules: p.id !== "default",
  }));
  return {
    productName: "Pixel Gaming Helper",
    version: "0.1.6",
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
    deviceStatus: { state: "simulated" },
    profile: MOCK_PROFILES.default,
    settings: {
      start_minimized: false,
      minimize_to_tray_on_close: true,
      launch_on_startup: false,
      brightness: 80,
      active_profile_id: "default",
      context_switching_enabled: false,
      theme: "system",
      spotify_client_id: "",
    },
  };
}

// ----- Public API ---------------------------------------------------------

export async function getSnapshot(): Promise<Snapshot> {
  if (!inTauri()) return mockSnapshot();
  return invoke<Snapshot>("get_snapshot");
}

export async function setActiveProfile(id: string): Promise<Profile | null> {
  if (!inTauri()) return MOCK_PROFILES[id] ?? null;
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

export async function spotifyConnect(): Promise<string> {
  if (!inTauri()) return "";
  return invoke<string>("spotify_connect");
}

export async function spotifyConnected(): Promise<boolean> {
  if (!inTauri()) return false;
  return invoke<boolean>("spotify_connected");
}

export async function runAction(action: Action): Promise<void> {
  if (!inTauri()) {
    // eslint-disable-next-line no-console
    console.info("[mock] runAction", action);
    return;
  }
  return invoke<void>("run_action", { action });
}
