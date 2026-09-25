// Generated pictures for the text cards (no official image): an emblem drawn
// from the card's name, and keyword icons. Nothing to download or publish.
import type { Keyword } from "./engine/types";

/** FNV-1a: the same name always gives the same emblem. */
function hash(name: string): number {
  let h = 0x811c9dc5;
  for (const c of name) h = Math.imul(h ^ c.charCodeAt(0), 0x01000193);
  return h >>> 0;
}

/** Abstract creature parts, drawn in a 100×100 box. */
const MOTIFS: ((ink: string) => React.ReactNode)[] = [
  // Scales
  (ink) => (
    <g fill="none" stroke={ink} strokeWidth="3">
      {[20, 38, 56, 74].flatMap((y, row) =>
        [0, 1, 2, 3, 4].map((i) => {
          const x = 10 + i * 20 + (row % 2) * 10;
          return <path key={`${row}-${i}`} d={`M${x - 10} ${y} a10 10 0 0 0 20 0`} />;
        }),
      )}
    </g>
  ),
  // Wing
  (ink) => (
    <g fill="none" stroke={ink} strokeWidth="3" strokeLinecap="round">
      {[0, 1, 2, 3, 4].map((i) => (
        <path key={i} d={`M50 85 Q${20 + i * 4} ${60 - i * 8} ${8 + i * 10} ${20 + i * 3}`} />
      ))}
      <path d="M50 85 Q55 40 30 12" strokeWidth="5" />
    </g>
  ),
  // Horns
  (ink) => (
    <g fill={ink}>
      <path d="M50 88 C20 80 8 50 18 14 C24 44 36 60 50 64 Z" />
      <path d="M50 88 C80 80 92 50 82 14 C76 44 64 60 50 64 Z" opacity="0.75" />
    </g>
  ),
  // Shell
  (ink) => (
    <g fill="none" stroke={ink} strokeWidth="3">
      <path d="M50 10 L84 30 L84 70 L50 90 L16 70 L16 30 Z" />
      <path d="M50 30 L67 40 L67 60 L50 70 L33 60 L33 40 Z" />
      <path d="M50 10 V30 M84 30 L67 40 M84 70 L67 60 M50 90 V70 M16 70 L33 60 M16 30 L33 40" />
    </g>
  ),
  // Tentacle
  (ink) => (
    <g fill="none" stroke={ink} strokeLinecap="round">
      <path d="M50 95 C50 70 80 65 78 45 C76 28 58 24 52 34 C47 42 56 50 62 44" strokeWidth="7" />
      <path d="M30 95 C30 78 14 70 18 52 C20 42 30 40 33 47" strokeWidth="4" />
    </g>
  ),
  // Eye
  (ink) => (
    <g fill="none" stroke={ink} strokeWidth="3">
      <path d="M8 50 Q50 10 92 50 Q50 90 8 50 Z" />
      <circle cx="50" cy="50" r="16" />
      <ellipse cx="50" cy="50" rx="4" ry="13" fill={ink} />
    </g>
  ),
];

/** Where the two halves meet: diagonal, vertical, or a curve. */
const CUTS = ["M0 0 H100 L0 100 Z", "M0 0 H50 V100 H0 Z", "M0 0 H100 V45 Q50 65 0 45 Z"];

/**
 * Two creature parts fused along a cut, like a Mindbug hybrid. The hues, the
 * parts, the cut and the mirroring all come from the name's hash.
 */
export function Emblem({ name }: { name: string }) {
  const h = hash(name);
  const hue1 = h % 360;
  const hue2 = (hue1 + 110 + ((h >>> 9) % 140)) % 360;
  const first = (h >>> 3) % MOTIFS.length;
  const drawn = (h >>> 7) % MOTIFS.length;
  // Never twice the same part.
  const second = drawn === first ? (drawn + 1) % MOTIFS.length : drawn;
  const cut = CUTS[(h >>> 12) % CUTS.length];
  const mirror = (h >>> 15) % 2 === 1 ? "matrix(-1 0 0 1 100 0)" : undefined;
  // Unique per name: several cards with the same name share the same defs.
  const id = `emblem-${h.toString(36)}`;
  return (
    <svg viewBox="0 0 100 100" preserveAspectRatio="xMidYMid slice" className="emblem" aria-hidden="true">
      <defs>
        <radialGradient id={`${id}-a`} cx="30%" cy="25%" r="90%">
          <stop offset="0" stopColor={`hsl(${hue1} 55% 58%)`} />
          <stop offset="1" stopColor={`hsl(${hue1} 50% 24%)`} />
        </radialGradient>
        <radialGradient id={`${id}-b`} cx="70%" cy="80%" r="90%">
          <stop offset="0" stopColor={`hsl(${hue2} 55% 55%)`} />
          <stop offset="1" stopColor={`hsl(${hue2} 50% 20%)`} />
        </radialGradient>
        <clipPath id={`${id}-cut`}>
          <path d={cut} />
        </clipPath>
      </defs>
      <rect width="100" height="100" fill={`url(#${id}-b)`} />
      <g transform={mirror}>{MOTIFS[second](`hsl(${hue2} 60% 82% / 0.8)`)}</g>
      <g clipPath={`url(#${id}-cut)`}>
        <rect width="100" height="100" fill={`url(#${id}-a)`} />
        <g transform={mirror}>{MOTIFS[first](`hsl(${hue1} 70% 86% / 0.85)`)}</g>
      </g>
      <path d={cut} fill="none" stroke="rgb(0 0 0 / 0.35)" strokeWidth="1.5" />
    </svg>
  );
}

const KEYWORD_ICON: Record<Keyword, React.ReactNode> = {
  Frenzy: <path d="M5 6l6 6-6 6M12 6l6 6-6 6" />,
  Hunter: (
    <>
      <circle cx="12" cy="12" r="7" />
      <path d="M12 2v5M12 17v5M2 12h5M17 12h5" />
      <circle cx="12" cy="12" r="1.2" fill="currentColor" />
    </>
  ),
  Poisonous: (
    <>
      <path d="M12 3c4 5 6 8 6 11a6 6 0 0 1-12 0c0-3 2-6 6-11z" />
      <path d="M9.5 14.5a2.5 2.5 0 0 0 2.5 2.5" />
    </>
  ),
  Sneaky: (
    <>
      <path d="M2.5 12S6 6 12 6s9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6z" />
      <circle cx="12" cy="12" r="2.5" />
      <path d="M4 20L20 4" />
    </>
  ),
  Tough: <path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z" />,
};

/** Decorative: the keyword's name is always written next to it. */
export function KeywordIcon({ keyword }: { keyword: Keyword }) {
  return (
    <svg
      viewBox="0 0 24 24"
      width="13"
      height="13"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {KEYWORD_ICON[keyword]}
    </svg>
  );
}
