import type { ProfileSummary } from "../types";
import { Equalizer, ProfileIcon, brandColor } from "../icons";

interface Props {
  profiles: ProfileSummary[];
  activeId: string;
  contextEnabled: boolean;
  onSelect: (id: string) => void;
}

/** Left-hand sidebar listing all profiles with brand icons; the active one
 *  glows and shows a live equalizer. */
export function ProfileSwitcher({ profiles, activeId, contextEnabled, onSelect }: Props) {
  return (
    <aside className="switcher">
      <div className="switcher-head">Profile</div>
      <div className="switcher-list">
        {profiles.map((p) => {
          const active = p.id === activeId;
          const accent = brandColor(p.id) ?? "var(--accent)";
          return (
            <button
              key={p.id}
              className={`profile-item${active ? " active" : ""}`}
              onClick={() => onSelect(p.id)}
              style={active ? ({ "--brand": accent } as React.CSSProperties) : undefined}
            >
              <span className="profile-icon" style={{ color: accent }}>
                <ProfileIcon id={p.id} size={22} className="icon-anim" />
              </span>
              <span className="profile-name">{p.name}</span>
              {p.has_rules && <span className="auto-dot" title="Wechselt automatisch bei dieser App" />}
              {active && <Equalizer color={accent} />}
            </button>
          );
        })}
      </div>
      <div className={`switcher-foot${contextEnabled ? " on" : ""}`}>
        <span className="ctx-dot" />
        {contextEnabled ? "Auto-Wechsel aktiv" : "Auto-Wechsel aus"}
      </div>
    </aside>
  );
}
