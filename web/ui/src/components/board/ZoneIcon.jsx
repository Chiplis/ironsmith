// Solid zone glyphs in the spirit of MTG Arena's zone counters: a stacked
// deck, a headstone and an exile vortex. Each pairs with a count beside it.
const ZONE_GLYPHS = {
  battlefield: (
    <>
      <path d="M3 21V5h4v4h3V5h4v4h3V5h4v16z" />
      <path d="M10 21v-5a2 2 0 0 1 4 0v5z" fill="var(--zone-icon-ink, #13110e)" />
    </>
  ),
  hand: (
    <>
      <rect x="3" y="5" width="10" height="16" rx="1.5" transform="rotate(-18 8 13)" opacity=".5" />
      <rect x="8" y="3" width="10" height="17" rx="1.5" opacity=".75" />
      <rect x="12" y="5" width="9" height="16" rx="1.5" transform="rotate(18 16 13)" />
    </>
  ),
  stack: (<path d="m12 2 10 5-10 5L2 7zm-10 9 10 5 10-5v3l-10 5-10-5zm0 7 10 5 10-5v-3l-10 5-10-5z" />),
  ante: (<path d="M12 2 22 12 12 22 2 12zm0 5-5 5 5 5 5-5z" fillRule="evenodd" />),
  sideboard: (<path d="M3 4h12v16H3zm14 4h4v12h-4z" />),
  library: (
    <>
      <rect x="3" y="6.5" width="12" height="15" rx="1.6" opacity="0.45" />
      <rect x="6" y="4.5" width="12" height="15" rx="1.6" opacity="0.7" />
      <rect x="9" y="2.5" width="12" height="15" rx="1.6" />
      <path d="M15 6.3 17.6 10 15 13.7 12.4 10z" fill="var(--zone-icon-ink, #13110e)" />
    </>
  ),
  graveyard: (
    <>
      <path d="M5 21.5V10a7 7 0 0 1 14 0v11.5z" />
      <path d="M11.1 7.5h1.8v2.6h2.6v1.8h-2.6v5.6h-1.8v-5.6H8.5v-1.8h2.6z" fill="var(--zone-icon-ink, #13110e)" />
      <rect x="3" y="20.5" width="18" height="2" rx="1" />
    </>
  ),
  exile: (
    <path
      fillRule="evenodd"
      d="M12 2.2c5.6 0 9.8 4.1 9.8 9.4 0 4.6-3.6 8.2-8 8.2-3.8 0-6.7-2.9-6.7-6.4 0-3 2.4-5.3 5.3-5.3 2.4 0 4.2 1.8 4.2 4 0 1.8-1.4 3.2-3.1 3.2-1.4 0-2.4-1-2.4-2.3h1.8c0 .3.2.5.6.5.7 0 1.3-.6 1.3-1.4 0-1.2-1-2.2-2.4-2.2-1.9 0-3.5 1.6-3.5 3.5 0 2.5 2.1 4.6 4.9 4.6 3.4 0 6.2-2.8 6.2-6.4C20 7.4 16.5 4 12 4S4 7.6 4 12.2c0 3 1.5 5.6 3.9 7.2l-1 1.5C4 19 2.2 15.8 2.2 12.2 2.2 6.6 6.4 2.2 12 2.2z"
    />
  ),
  command: (
    <path d="M3 8.5 7.5 12 12 4.5l4.5 7.5L21 8.5 19 19H5zM5 20.5h14V22H5z" />
  ),
};

export default function ZoneIcon({ zone, className = "" }) {
  return (
    <svg
      className={`zone-icon ${className}`.trim()}
      data-zone-icon={zone}
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
      focusable="false"
    >
      {ZONE_GLYPHS[zone] || ZONE_GLYPHS.library}
    </svg>
  );
}

export function LookEyeGlyph({ className = "" }) {
  return (
    <svg className={`look-eye-glyph ${className}`.trim()} viewBox="0 0 96 54" aria-hidden="true" focusable="false">
      <path className="look-eye-glow" d="M8 27 C27 4 69 4 88 27 C69 50 27 50 8 27 Z" />
      <path className="look-eye-lid" d="M8 27 C27 4 69 4 88 27" />
      <ellipse className="look-eye-iris" cx="48" cy="27" rx="12" ry="15" />
      <circle className="look-eye-pupil" cx="48" cy="27" r="5" />
    </svg>
  );
}
