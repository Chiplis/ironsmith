import { useEffect, useId, useRef, useState } from "react";

/**
 * "Playing as" seat switcher for goldfish games. The whole pill opens a small
 * list of players above it; picking one changes the perspective.
 */
export default function PlayerPerspectiveMenu({ label, players, currentId, onSelect }) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef(null);
  const listRef = useRef(null);
  const listId = useId();
  const current = players.find((player) => Number(player.id) === Number(currentId)) || players[0];

  useEffect(() => {
    if (!open) return undefined;
    const handlePointerDown = (event) => {
      if (!rootRef.current?.contains(event.target)) setOpen(false);
    };
    document.addEventListener("pointerdown", handlePointerDown, true);
    listRef.current?.querySelector('[aria-selected="true"], [role="option"]')?.focus();
    return () => document.removeEventListener("pointerdown", handlePointerDown, true);
  }, [open]);

  const moveFocus = (step) => {
    const options = [...(listRef.current?.querySelectorAll('[role="option"]') || [])];
    if (!options.length) return;
    const index = options.indexOf(document.activeElement);
    options[(index + step + options.length) % options.length].focus();
  };

  return (
    <div
      ref={rootRef}
      className="player-header-perspective"
      data-open={open ? "true" : "false"}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (!open) return;
        if (event.key === "Escape") {
          event.preventDefault();
          setOpen(false);
          rootRef.current?.querySelector(".player-header-perspective-trigger")?.focus();
        } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
          event.preventDefault();
          moveFocus(event.key === "ArrowDown" ? 1 : -1);
        }
      }}
    >
      <button
        type="button"
        className="player-header-perspective-trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
        onClick={() => setOpen((value) => !value)}
      >
        <span className="player-header-perspective-label">{label}</span>
        <span className="player-header-perspective-value">{current?.name}</span>
        <span className="player-header-perspective-caret" aria-hidden="true">▾</span>
      </button>
      {open ? (
        <div ref={listRef} id={listId} className="player-header-perspective-menu" role="listbox" aria-label={label}>
          {players.map((player) => {
            const selected = Number(player.id) === Number(currentId);
            return (
              <button
                key={player.id}
                type="button"
                role="option"
                aria-selected={selected}
                className="player-header-perspective-option"
                style={{ "--option-accent": player.accent }}
                onClick={() => {
                  setOpen(false);
                  if (!selected) onSelect(Number(player.id));
                }}
              >
                <span className="player-header-perspective-dot" aria-hidden="true" />
                <span className="player-header-perspective-name">{player.name}</span>
                {selected ? <span className="player-header-perspective-check" aria-hidden="true">✓</span> : null}
              </button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}
