import { useEffect, useRef, useState } from "react";
import { getSnapshot, inTauri, saveProfile, saveSettings } from "./api";
import type { EncoderConfig, KeyConfig, Profile, Settings } from "./types";
import { DeckView, type Selection } from "./components/DeckView";
import { Inspector } from "./components/Inspector";
import { SettingsPanel } from "./components/SettingsPanel";

type Tab = "deck" | "settings";

export function App() {
  const [profile, setProfile] = useState<Profile | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [meta, setMeta] = useState({ productName: "Pixel Gaming Helper", version: "", hardwareBuild: false });
  const [tab, setTab] = useState<Tab>("deck");
  const [selection, setSelection] = useState<Selection | null>(null);
  const [pageIndex, setPageIndex] = useState(0);
  const firstLoad = useRef(true);

  // Initial load.
  useEffect(() => {
    getSnapshot().then((snap) => {
      setProfile(snap.profile);
      setSettings(snap.settings);
      setMeta({ productName: snap.productName, version: snap.version, hardwareBuild: snap.hardwareBuild });
    });
  }, []);

  // Apply theme.
  useEffect(() => {
    if (!settings) return;
    const root = document.documentElement;
    if (settings.theme === "system") root.removeAttribute("data-theme");
    else root.setAttribute("data-theme", settings.theme);
  }, [settings]);

  // Debounced autosave of the profile.
  useEffect(() => {
    if (!profile || firstLoad.current) return;
    const t = setTimeout(() => void saveProfile(profile), 400);
    return () => clearTimeout(t);
  }, [profile]);

  // Autosave settings immediately (they're small and few).
  useEffect(() => {
    if (!settings || firstLoad.current) return;
    void saveSettings(settings);
  }, [settings]);

  // Flip the first-load guard once both are present.
  useEffect(() => {
    if (profile && settings) firstLoad.current = false;
  }, [profile, settings]);

  if (!profile || !settings) {
    return <div className="loading">Lädt …</div>;
  }

  const page = profile.pages[pageIndex] ?? profile.pages[0];
  const pageIds = profile.pages.map((p) => p.id);

  const updateKey = (index: number, key: KeyConfig) => {
    setProfile((prev) => {
      if (!prev) return prev;
      const pages = prev.pages.map((p, pi) =>
        pi === pageIndex ? { ...p, keys: p.keys.map((k, ki) => (ki === index ? key : k)) } : p
      );
      return { ...prev, pages };
    });
  };

  const updateEncoder = (index: number, enc: EncoderConfig) => {
    setProfile((prev) => {
      if (!prev) return prev;
      const pages = prev.pages.map((p, pi) =>
        pi === pageIndex ? { ...p, encoders: p.encoders.map((e, ei) => (ei === index ? enc : e)) } : p
      );
      return { ...prev, pages };
    });
  };

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <img src="/src-tauri/icons/32x32.png" width={24} height={24} alt="" />
          <strong>{meta.productName}</strong>
          <span className="version">v{meta.version}</span>
          {!meta.hardwareBuild && <span className="badge">Simulation</span>}
          {!inTauri() && <span className="badge browser">Browser-Vorschau</span>}
        </div>
        <nav className="tabs">
          <button className={tab === "deck" ? "active" : ""} onClick={() => setTab("deck")}>
            Deck
          </button>
          <button className={tab === "settings" ? "active" : ""} onClick={() => setTab("settings")}>
            Einstellungen
          </button>
        </nav>
      </header>

      {tab === "deck" ? (
        <main className="workspace">
          <section className="stage">
            {profile.pages.length > 1 && (
              <div className="page-tabs">
                {profile.pages.map((p, i) => (
                  <button
                    key={p.id}
                    className={i === pageIndex ? "active" : ""}
                    onClick={() => {
                      setPageIndex(i);
                      setSelection(null);
                    }}
                  >
                    {p.name || p.id}
                  </button>
                ))}
              </div>
            )}
            <DeckView page={page} selection={selection} onSelect={setSelection} />
            <p className="hint">Tipp: „Aktion testen" führt eine Bindung sofort aus — auch ohne Gerät.</p>
          </section>
          <Inspector
            page={page}
            pageIds={pageIds}
            selection={selection}
            onKeyChange={updateKey}
            onEncoderChange={updateEncoder}
          />
        </main>
      ) : (
        <main className="workspace">
          <SettingsPanel settings={settings} onChange={setSettings} />
        </main>
      )}
    </div>
  );
}
