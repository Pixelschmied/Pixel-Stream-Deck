import type { AvailableUpdate } from "../updater";

interface Props {
  update: AvailableUpdate;
  progress: number | null;
  error: string | null;
  onInstall: () => void;
}

/** A banner shown when a new version is available. */
export function UpdateBanner({ update, progress, error, onInstall }: Props) {
  const installing = progress !== null;
  return (
    <div className="update-banner">
      <span className="update-spark">⬇</span>
      <div className="update-text">
        <strong>Update verfügbar: v{update.version}</strong>
        {update.notes && <span className="update-notes">{update.notes}</span>}
        {error && <span className="update-error">{error}</span>}
      </div>
      {installing ? (
        <div className="update-progress">
          <div className="update-bar" style={{ width: `${progress}%` }} />
          <span>{progress}%</span>
        </div>
      ) : (
        <button className="update-btn" onClick={onInstall}>
          Installieren &amp; neu starten
        </button>
      )}
    </div>
  );
}
