import type { Action, ActionKind } from "../types";
import { ACTION_LABELS, emptyAction } from "../types";

interface Props {
  action: Action;
  pageIds: string[];
  onChange: (action: Action) => void;
}

/** Edits a single Action: a kind selector plus kind-specific fields. */
export function ActionEditor({ action, pageIds, onChange }: Props) {
  const kinds = Object.keys(ACTION_LABELS) as ActionKind[];

  return (
    <div className="action-editor">
      <select
        value={action.type}
        onChange={(e) => onChange(emptyAction(e.target.value as ActionKind))}
      >
        {kinds.map((k) => (
          <option key={k} value={k}>
            {ACTION_LABELS[k]}
          </option>
        ))}
      </select>

      {action.type === "launch_app" && (
        <>
          <input
            placeholder="Pfad / Programm (z. B. steam)"
            value={action.path}
            onChange={(e) => onChange({ ...action, path: e.target.value })}
          />
          <input
            placeholder="Argumente (Leerzeichen-getrennt)"
            value={action.args.join(" ")}
            onChange={(e) =>
              onChange({ ...action, args: e.target.value.split(/\s+/).filter(Boolean) })
            }
          />
        </>
      )}

      {action.type === "run_command" && (
        <input
          placeholder="Shell-Befehl"
          value={action.command}
          onChange={(e) => onChange({ ...action, command: e.target.value })}
        />
      )}

      {action.type === "open_url" && (
        <input
          placeholder="https://…"
          value={action.url}
          onChange={(e) => onChange({ ...action, url: e.target.value })}
        />
      )}

      {action.type === "send_hotkey" && (
        <input
          placeholder="Tasten mit + (z. B. ctrl+shift+m)"
          value={action.keys.join("+")}
          onChange={(e) =>
            onChange({
              ...action,
              keys: e.target.value.split("+").map((k) => k.trim().toLowerCase()).filter(Boolean),
            })
          }
        />
      )}

      {action.type === "switch_page" && (
        <select
          value={action.page_id}
          onChange={(e) => onChange({ ...action, page_id: e.target.value })}
        >
          <option value="">— Seite wählen —</option>
          {pageIds.map((id) => (
            <option key={id} value={id}>
              {id}
            </option>
          ))}
        </select>
      )}

      {action.type === "adjust_brightness" && (
        <label className="inline">
          Δ
          <input
            type="number"
            min={-100}
            max={100}
            value={action.delta}
            onChange={(e) => onChange({ ...action, delta: Number(e.target.value) })}
          />
          %
        </label>
      )}

      {action.type === "spotify" && (
        <select value={action.op} onChange={(e) => onChange({ ...action, op: e.target.value })}>
          <option value="play_pause">Play / Pause</option>
          <option value="next">Nächster Titel</option>
          <option value="prev">Vorheriger Titel</option>
        </select>
      )}
    </div>
  );
}
