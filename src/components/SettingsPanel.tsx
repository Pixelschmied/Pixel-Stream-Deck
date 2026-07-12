import type { Settings, Theme } from "../types";

interface Props {
  settings: Settings;
  onChange: (settings: Settings) => void;
  onCheckUpdates: () => void;
  updateStatus: string;
  version: string;
}

/** The Settings tab: user-configurable options for the app and the deck. */
export function SettingsPanel({ settings, onChange, onCheckUpdates, updateStatus, version }: Props) {
  const set = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    onChange({ ...settings, [key]: value });

  return (
    <div className="settings">
      <h2>Einstellungen</h2>

      <section>
        <h3>Fenster</h3>
        <Toggle
          label="Minimiert im Tray starten"
          checked={settings.start_minimized}
          onChange={(v) => set("start_minimized", v)}
        />
        <Toggle
          label="Beim Schließen in den Tray minimieren"
          checked={settings.minimize_to_tray_on_close}
          onChange={(v) => set("minimize_to_tray_on_close", v)}
        />
        <Toggle
          label="Beim Systemstart automatisch starten"
          checked={settings.launch_on_startup}
          onChange={(v) => set("launch_on_startup", v)}
        />
      </section>

      <section>
        <h3>Gerät</h3>
        <label className="slider">
          Helligkeit: {settings.brightness}%
          <input
            type="range"
            min={0}
            max={100}
            value={settings.brightness}
            onChange={(e) => set("brightness", Number(e.target.value))}
          />
        </label>
        <Toggle
          label="Profil automatisch nach aktiver App wechseln"
          checked={settings.context_switching_enabled}
          onChange={(v) => set("context_switching_enabled", v)}
        />
      </section>

      <section>
        <h3>Darstellung</h3>
        <label className="inline">
          Theme
          <select
            value={settings.theme}
            onChange={(e) => set("theme", e.target.value as Theme)}
          >
            <option value="system">System</option>
            <option value="dark">Dunkel</option>
            <option value="light">Hell</option>
          </select>
        </label>
      </section>

      <section>
        <h3>Updates</h3>
        <p className="update-version">Installierte Version: v{version}</p>
        <button className="test-btn" onClick={onCheckUpdates}>
          Jetzt nach Updates suchen
        </button>
        {updateStatus && <p className="update-status">{updateStatus}</p>}
        <p className="update-hint">
          Beim Start wird automatisch geprüft, ob im Repository eine neue Version
          veröffentlicht wurde.
        </p>
      </section>
    </div>
  );
}

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="toggle">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <span>{label}</span>
    </label>
  );
}
