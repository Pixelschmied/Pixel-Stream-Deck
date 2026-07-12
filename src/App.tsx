import { useEffect, useRef, useState } from "react";
import { getSnapshot, inTauri, saveProfile, saveSettings, setActiveProfile } from "./api";
import type { EncoderConfig, KeyConfig, Profile, ProfileSummary, Settings } from "./types";
import { DeckView, type Selection } from "./components/DeckView";
import { Inspector } from "./components/Inspector";
import { SettingsPanel } from "./components/SettingsPanel";
import { ProfileSwitcher } from "./components/ProfileSwitcher";
import { UpdateBanner } from "./components/UpdateBanner";
import { ProfileIcon, brandColor } from "./icons";
import { checkForUpdate, type AvailableUpdate } from "./updater";

type Tab = "deck" | "settings";

export function App() {
  const [profile, setProfile] = useState<Profile | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [activeId, setActiveId] = useState<string>("default");
  const [meta, setMeta] = useState({ productName: "Pixel Gaming Helper", version: "", hardwareBuild: false });
  const [tab, setTab] = useState<Tab>("deck");
  const [selection, setSelection] = useState<Selection | null>(null);
  const [pageIndex, setPageIndex] = useState(0);
  const [update, setUpdate] = useState<AvailableUpdate | null>(null);
  const [updateProgress, setUpdateProgress] = useState<number | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [updateStatus, setUpdateStatus] = useState<string>("");
  const firstLoad = useRef(true);

  // Initial load.
  useEffect(() => {
    getSnapshot().then((snap) => {
      setProfile(snap.profile);
      setSettings(snap.settings);
      setProfiles(snap.profiles);
      setActiveId(snap.activeProfileId);
      setMeta({ productName: snap.productName, version: snap.version, hardwareBuild: snap.hardwareBuild });
    });
  }, []);

  // React to auto profile switches coming from the context engine.
  useEffect(() => {
    if (!inTauri()) return;
    let unlisten: (() => void) | undefined;
    import("@tauri-apps/api/event").then(({ listen }) => {
      listen<Profile>("profile-activated", (e) => {
        setProfile(e.payload);
        setActiveId(e.payload.id);
        setSelection(null);
        setPageIndex(0);
      }).then((fn) => (unlisten = fn));
    });
    return () => unlisten?.();
  }, []);

  // Check for an update once on startup (silent if up to date).
  useEffect(() => {
    checkForUpdate()
      .then((u) => u && setUpdate(u))
      .catch((e) => console.error("update check failed", e));
  }, []);

  const installUpdate = () => {
    if (!update) return;
    setUpdateProgress(0);
    setUpdateError(null);
    update.install(setUpdateProgress).catch((e) => {
      setUpdateError(String(e));
      setUpdateProgress(null);
    });
  };

  const checkUpdatesManually = async () => {
    setUpdateStatus("Suche nach Updates …");
    try {
      const u = await checkForUpdate();
      if (u) {
        setUpdate(u);
        setUpdateStatus(`Update v${u.version} verfügbar.`);
      } else {
        setUpdateStatus("Du hast bereits die neueste Version.");
      }
    } catch (e) {
      setUpdateStatus(`Fehler bei der Update-Suche: ${e}`);
    }
  };

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

  useEffect(() => {
    if (profile && settings) firstLoad.current = false;
  }, [profile, settings]);

  if (!profile || !settings) {
    return <div className="loading">Lädt …</div>;
  }

  const page = profile.pages[pageIndex] ?? profile.pages[0];
  const pageIds = profile.pages.map((p) => p.id);
  const accent = brandColor(activeId);

  const selectProfile = async (id: string) => {
    setActiveId(id);
    setSelection(null);
    setPageIndex(0);
    const p = await setActiveProfile(id);
    if (p) setProfile(p);
  };

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
        <div className="active-pill" style={accent ? { borderColor: accent, color: accent } : undefined}>
          <ProfileIcon id={activeId} size={16} />
          {profile.name}
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

      {update && (
        <UpdateBanner
          update={update}
          progress={updateProgress}
          error={updateError}
          onInstall={installUpdate}
        />
      )}

      {tab === "deck" ? (
        <main className="workspace">
          <ProfileSwitcher
            profiles={profiles}
            activeId={activeId}
            contextEnabled={settings.context_switching_enabled}
            onSelect={selectProfile}
          />
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
          <SettingsPanel
            settings={settings}
            onChange={setSettings}
            onCheckUpdates={checkUpdatesManually}
            updateStatus={updateStatus}
            version={meta.version}
          />
        </main>
      )}
    </div>
  );
}
