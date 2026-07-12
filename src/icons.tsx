// Open-source brand + generic icons. Brand logos come from `simple-icons`
// (CC0), rendered as inline SVG so they inherit colour and animate via CSS.

import {
  siSpotify,
  siSteam,
  siDiscord,
  siBattledotnet,
  siClaude,
  type SimpleIcon,
} from "simple-icons";

/** Maps a profile id to its brand logo, when one exists. */
export const PROFILE_BRAND: Record<string, SimpleIcon | undefined> = {
  spotify: siSpotify,
  steam: siSteam,
  discord: siDiscord,
  battlenet: siBattledotnet,
  claude: siClaude,
};

// Curated, always-visible accent colours. simple-icons' own hex is sometimes
// near-black (e.g. Steam), which disappears on a dark UI — so we override with
// a recognisable, readable brand tint used for both the icon and the glow.
const CURATED: Record<string, string> = {
  spotify: "#1db954",
  steam: "#66c0f4",
  discord: "#5865f2",
  battlenet: "#148eff",
  claude: "#cc785c",
};

/** Brand accent colour (hex incl. #) for a profile, if known. */
export function brandColor(id: string): string | undefined {
  if (CURATED[id]) return CURATED[id];
  const icon = PROFILE_BRAND[id];
  return icon ? `#${icon.hex}` : undefined;
}

export function BrandIcon({
  icon,
  size = 20,
  color,
  className,
}: {
  icon: SimpleIcon;
  size?: number;
  color?: string;
  className?: string;
}) {
  return (
    <svg
      role="img"
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill={color ?? `#${icon.hex}`}
      className={className}
      aria-label={icon.title}
    >
      <path d={icon.path} />
    </svg>
  );
}

/** A neutral gamepad glyph used when a profile has no brand logo. */
export function GamepadIcon({ size = 20, className }: { size?: number; className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden
    >
      <line x1="6" y1="11" x2="10" y2="11" />
      <line x1="8" y1="9" x2="8" y2="13" />
      <line x1="15" y1="12" x2="15.01" y2="12" />
      <line x1="18" y1="10" x2="18.01" y2="10" />
      <rect x="2" y="6" width="20" height="12" rx="6" />
    </svg>
  );
}

/** Infer a brand id from a free-text key label (e.g. "Open Spotify"). */
export function brandForLabel(label: string): string | undefined {
  const l = label.toLowerCase();
  if (l.includes("spotify")) return "spotify";
  if (l.includes("steam")) return "steam";
  if (l.includes("discord")) return "discord";
  if (l.includes("battle")) return "battlenet";
  if (l.includes("claude")) return "claude";
  return undefined;
}

/** The brand logo for a key label, if one applies. */
export function KeyBrand({ label, size = 26 }: { label: string; size?: number }) {
  const id = brandForLabel(label);
  const brand = id ? PROFILE_BRAND[id] : undefined;
  if (!brand) return null;
  return <BrandIcon icon={brand} size={size} color="#ffffffe6" />;
}

/** Render whatever icon best represents a profile (brand logo or gamepad). */
export function ProfileIcon({
  id,
  size = 20,
  className,
}: {
  id: string;
  size?: number;
  className?: string;
}) {
  const brand = PROFILE_BRAND[id];
  if (brand) return <BrandIcon icon={brand} size={size} color={brandColor(id)} className={className} />;
  return <GamepadIcon size={size} className={className} />;
}

/** A small animated equalizer, shown on the live/active profile. */
export function Equalizer({ color = "currentColor" }: { color?: string }) {
  return (
    <span className="equalizer" aria-hidden style={{ color }}>
      <i />
      <i />
      <i />
      <i />
    </span>
  );
}
