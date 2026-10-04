const ZONE_PATHS = {
  battlefield: (
    <>
      <path d="M3.5 7h7v10h-7zM13.5 5h7v10h-7z" />
      <path d="M5.5 9.5h3M15.5 7.5h3M3 19h18" />
    </>
  ),
  hand: (
    <>
      <path d="m3.5 8.5 5.2-3 4.2 10.8-5.2 2z" />
      <path d="M9 4h6v14H9z" />
      <path d="m14.5 5.5 5.7 3-4.3 10.3-5.1-2.2z" />
    </>
  ),
  graveyard: (
    <>
      <path d="M5 20V9a7 7 0 0 1 14 0v11z" />
      <path d="m12 8 1.4 1.4-1.4 1.4-1.4-1.4zM8 17h8" />
    </>
  ),
  library: (
    <>
      <path d="M7 3.5h12v14H7z" />
      <path d="M5 5.5v14h12M3 7.5v12h12" />
    </>
  ),
  exile: (
    <>
      <path d="M7 4.5 3.5 12 7 19.5M17 4.5l3.5 7.5-3.5 7.5" />
      <path d="m9 8 5-1 2 9-5 1zM15 12h3m-2-2 2 2-2 2" />
    </>
  ),
  command: (
    <>
      <path d="M4.5 9h10v12h-10zM7 12h5" />
      <path d="m17 3 1.2 2.8L21 7l-2.8 1.2L17 11l-1.2-2.8L13 7l2.8-1.2z" />
    </>
  ),
  ante: (
    <>
      <path d="M3.5 4.5h11v14h-11z" />
      <circle cx="16.5" cy="16.5" r="4" />
      <path d="m16.5 14.7 1.2 1.8-1.2 1.8-1.2-1.8z" />
    </>
  ),
};

export default function ZoneCountIcon({ zone, className = "" }) {
  return (
    <svg
      className={className}
      data-zone-icon={zone}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {ZONE_PATHS[zone] || ZONE_PATHS.library}
    </svg>
  );
}
