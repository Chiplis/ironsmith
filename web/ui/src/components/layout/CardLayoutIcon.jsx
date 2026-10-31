const arenaLayouts = new Set(['adventure', 'omen', 'prepare', 'aftermath', 'meld']);

// Sources and attribution: public/card-layouts/sources.json.
export default function CardLayoutIcon({ layout }) {
  if (arenaLayouts.has(layout)) {
    return <img src={`${import.meta.env.BASE_URL}card-layouts/${layout}.png`} alt="" aria-hidden="true" className="size-6 shrink-0 object-contain" />;
  }
  if (layout === 'modal_dfc' || layout === 'transform') {
    const mask = `url("${import.meta.env.BASE_URL}card-layouts/${layout}.svg") center / contain no-repeat`;
    return <span aria-hidden="true" className="size-6 shrink-0 bg-[#d8b65e]" style={{ mask, WebkitMask: mask }} />;
  }
  // Layout diagrams, not claimed to be official mechanic symbols.
  return (
    <svg aria-hidden="true" className="size-6 shrink-0 text-[#d8b65e]" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      {layout === 'single' && <><rect x="5" y="2" width="14" height="20" rx="2" /><path d="M8 6h8v6H8zM8 16h8M8 19h5" /></>}
      {layout === 'flip' && <><rect x="5" y="2" width="14" height="20" rx="2" /><path d="M5 12h14M9 9l3-4 3 4zM9 15l3 4 3-4z" /></>}
      {layout === 'split' && <><rect x="2" y="5" width="20" height="14" rx="2" /><path d="M12 5v14M5 8h4v4H5zM15 8h4v4h-4zM5 15h4M15 15h4" /></>}
      {layout === 'room' && <><path d="M3 21V5l9-3 9 3v16H3zM12 2v19M5 20V8l5-2v14M14 20V6l5 2v12" /><path d="M8 13v2M16 13v2" /></>}
    </svg>
  );
}
