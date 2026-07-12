import type { Page } from "../types";
import { summarizeAction } from "../types";
import { KeyBrand } from "../icons";

export type Selection =
  | { kind: "key"; index: number }
  | { kind: "encoder"; index: number };

interface Props {
  page: Page;
  selection: Selection | null;
  onSelect: (sel: Selection) => void;
}

const ACCENT = ["#7ce0ff", "#ff6bd6", "#9d7cff", "#7cffa8"];

/** A visual mock of the Stream Deck +: 8 keys, the touch strip and 4 dials. */
export function DeckView({ page, selection, onSelect }: Props) {
  const isSel = (kind: Selection["kind"], index: number) =>
    selection?.kind === kind && selection.index === index;

  return (
    <div className="deck">
      <div className="deck-keys">
        {page.keys.slice(0, 8).map((key, i) => (
          <button
            key={i}
            className={`deck-key${isSel("key", i) ? " selected" : ""}`}
            style={{ background: key.color ?? "#1e1e2e" }}
            onClick={() => onSelect({ kind: "key", index: i })}
            title={summarizeAction(key.action)}
          >
            <span className="deck-key-brand">
              <KeyBrand label={key.label} />
            </span>
            <span className="deck-key-label">{key.label || `Taste ${i + 1}`}</span>
            <span className="deck-key-action">{summarizeAction(key.action)}</span>
          </button>
        ))}
      </div>

      <div className="deck-strip" aria-hidden>
        {page.encoders.slice(0, 4).map((enc, i) => {
          const bound =
            enc.on_press.type !== "none" ||
            enc.on_turn_cw.type !== "none" ||
            enc.on_turn_ccw.type !== "none" ||
            enc.on_touch.type !== "none";
          return (
            <div
              key={i}
              className="deck-strip-seg"
              style={{
                background: bound
                  ? `linear-gradient(180deg, ${ACCENT[i % ACCENT.length]}, ${ACCENT[i % ACCENT.length]}22)`
                  : "#181820",
              }}
            >
              {enc.label}
            </div>
          );
        })}
      </div>

      <div className="deck-dials">
        {page.encoders.slice(0, 4).map((enc, i) => (
          <button
            key={i}
            className={`deck-dial${isSel("encoder", i) ? " selected" : ""}`}
            onClick={() => onSelect({ kind: "encoder", index: i })}
            title={enc.label}
          >
            <span className="dial-knob" style={{ borderColor: ACCENT[i % ACCENT.length] }} />
            <span className="dial-label">{enc.label || `Dial ${i + 1}`}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
