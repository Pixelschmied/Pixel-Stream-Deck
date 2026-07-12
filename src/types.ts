// Types mirroring the Rust `deck-core` model and the Tauri command payloads.

export type Action =
  | { type: "none" }
  | { type: "launch_app"; path: string; args: string[] }
  | { type: "run_command"; command: string }
  | { type: "open_url"; url: string }
  | { type: "send_hotkey"; keys: string[] }
  | { type: "switch_page"; page_id: string }
  | { type: "adjust_brightness"; delta: number };

export type ActionKind = Action["type"];

export interface KeyConfig {
  label: string;
  icon?: string | null;
  color?: string | null;
  action: Action;
}

export interface EncoderConfig {
  label: string;
  icon?: string | null;
  on_press: Action;
  on_turn_cw: Action;
  on_turn_ccw: Action;
  on_touch: Action;
}

export interface Page {
  id: string;
  name: string;
  keys: KeyConfig[];
  encoders: EncoderConfig[];
}

export interface AppMatch {
  process?: string | null;
  title_contains?: string | null;
}

export interface Profile {
  id: string;
  name: string;
  pages: Page[];
  activates_for: AppMatch[];
}

export type Theme = "system" | "dark" | "light";

export interface Settings {
  start_minimized: boolean;
  minimize_to_tray_on_close: boolean;
  launch_on_startup: boolean;
  brightness: number;
  active_profile_id: string;
  context_switching_enabled: boolean;
  theme: Theme;
}

export interface DeckInfo {
  model: string;
  serial: string;
  key_count: number;
  encoder_count: number;
  key_image_size: [number, number];
  touchstrip_size: [number, number];
}

export interface ProfileSummary {
  id: string;
  name: string;
  has_rules: boolean;
}

export type DeviceStatus =
  | { state: "searching" }
  | { state: "connected"; model: string; serial: string }
  | { state: "simulated" };

export interface Snapshot {
  productName: string;
  version: string;
  hardwareBuild: boolean;
  deckInfo: DeckInfo;
  deviceStatus: DeviceStatus;
  profiles: ProfileSummary[];
  activeProfileId: string;
  profile: Profile;
  settings: Settings;
}

/** Human-readable label for each action kind, used in the editor dropdown. */
export const ACTION_LABELS: Record<ActionKind, string> = {
  none: "Keine Aktion",
  launch_app: "Programm starten",
  run_command: "Befehl ausführen",
  open_url: "URL öffnen",
  send_hotkey: "Tastenkürzel senden",
  switch_page: "Seite wechseln",
  adjust_brightness: "Helligkeit ändern",
};

/** A fresh action of the given kind with sensible empty fields. */
export function emptyAction(kind: ActionKind): Action {
  switch (kind) {
    case "launch_app":
      return { type: "launch_app", path: "", args: [] };
    case "run_command":
      return { type: "run_command", command: "" };
    case "open_url":
      return { type: "open_url", url: "" };
    case "send_hotkey":
      return { type: "send_hotkey", keys: [] };
    case "switch_page":
      return { type: "switch_page", page_id: "" };
    case "adjust_brightness":
      return { type: "adjust_brightness", delta: 10 };
    default:
      return { type: "none" };
  }
}

/** Short one-line summary of an action for compact display. */
export function summarizeAction(action: Action): string {
  switch (action.type) {
    case "launch_app":
      return action.path ? `▶ ${action.path}` : "▶ (kein Pfad)";
    case "run_command":
      return action.command ? `$ ${action.command}` : "$ (leer)";
    case "open_url":
      return action.url ? `🔗 ${action.url}` : "🔗 (keine URL)";
    case "send_hotkey":
      return action.keys.length ? `⌨ ${action.keys.join("+")}` : "⌨ (keine Tasten)";
    case "switch_page":
      return `⇄ Seite: ${action.page_id || "?"}`;
    case "adjust_brightness":
      return `☀ ${action.delta > 0 ? "+" : ""}${action.delta}%`;
    default:
      return "—";
  }
}
