import type { Action, KeyConfig, EncoderConfig, Page } from "../types";
import { runAction } from "../api";
import { ActionEditor } from "./ActionEditor";
import type { Selection } from "./DeckView";

interface Props {
  page: Page;
  pageIds: string[];
  selection: Selection | null;
  onKeyChange: (index: number, key: KeyConfig) => void;
  onEncoderChange: (index: number, enc: EncoderConfig) => void;
}

/** Right-hand panel that edits the currently selected key or encoder. */
export function Inspector({ page, pageIds, selection, onKeyChange, onEncoderChange }: Props) {
  if (!selection) {
    return (
      <aside className="inspector empty">
        <p>Wähle links eine Taste oder einen Dial, um sie zu konfigurieren.</p>
      </aside>
    );
  }

  if (selection.kind === "key") {
    const key = page.keys[selection.index];
    return (
      <aside className="inspector">
        <h2>Taste {selection.index + 1}</h2>
        <label>
          Beschriftung
          <input
            value={key.label}
            onChange={(e) => onKeyChange(selection.index, { ...key, label: e.target.value })}
          />
        </label>
        <label>
          Farbe
          <input
            type="color"
            value={key.color ?? "#1e1e2e"}
            onChange={(e) => onKeyChange(selection.index, { ...key, color: e.target.value })}
          />
        </label>
        <label>Aktion beim Druck</label>
        <ActionEditor
          action={key.action}
          pageIds={pageIds}
          onChange={(action) => onKeyChange(selection.index, { ...key, action })}
        />
        <TestButton action={key.action} />
      </aside>
    );
  }

  const enc = page.encoders[selection.index];
  const gestures: Array<[keyof EncoderConfig, string]> = [
    ["on_press", "Druck"],
    ["on_turn_cw", "Drehen →"],
    ["on_turn_ccw", "Drehen ←"],
    ["on_touch", "Touch-Tap"],
  ];
  return (
    <aside className="inspector">
      <h2>Dial {selection.index + 1}</h2>
      <label>
        Beschriftung
        <input
          value={enc.label}
          onChange={(e) => onEncoderChange(selection.index, { ...enc, label: e.target.value })}
        />
      </label>
      {gestures.map(([field, title]) => (
        <div key={field} className="gesture">
          <label>{title}</label>
          <ActionEditor
            action={enc[field] as Action}
            pageIds={pageIds}
            onChange={(action) => onEncoderChange(selection.index, { ...enc, [field]: action })}
          />
          <TestButton action={enc[field] as Action} />
        </div>
      ))}
    </aside>
  );
}

function TestButton({ action }: { action: Action }) {
  if (action.type === "none") return null;
  return (
    <button className="test-btn" onClick={() => void runAction(action)}>
      ▶ Aktion testen
    </button>
  );
}
