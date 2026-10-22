import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";

/**
 * The local player's name opens the seat switcher in goldfish games.
 */
export default function PlayerPerspectiveMenu({ label, players, currentId, onSelect, isActivePlayer }) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef(null);
  const listRef = useRef(null);
  const listId = useId();
  const current = players.find((player) => Number(player.id) === Number(currentId)) || players[0];

  // A top-layer popover escapes every board/Stack stacking context while
  // retaining the menu's inherited styling and keyboard event handling.
  useLayoutEffect(() => {
    if (!open) return undefined;
    const menu = listRef.current;
    menu.showPopover();
    const position = () => {
      const anchor = rootRef.current.getBoundingClientRect();
      const width = Math.max(168, anchor.width);
      Object.assign(menu.style, {
        position: "fixed", margin: "0", right: "auto", bottom: "auto",
        minWidth: `${width}px`, maxWidth: "calc(100vw - 16px)",
        maxHeight: "calc(100vh - 16px)", overflowY: "auto",
        left: `${Math.max(8, Math.min(anchor.left, window.innerWidth - width - 8))}px`,
        top: `${Math.max(8, anchor.top - menu.getBoundingClientRect().height - 4)}px`,
      });
    };
    position();
    window.addEventListener("resize", position);
    window.addEventListener("scroll", position, true);
    return () => {
      window.removeEventListener("resize", position);
      window.removeEventListener("scroll", position, true);
      if (menu.matches(":popover-open")) menu.hidePopover();
    };
  }, [open]);

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
        className="player-header-perspective-trigger battlefield-name min-w-0 text-[16px] uppercase tracking-wider font-bold"
        data-player-target-name={currentId}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
        onClick={() => setOpen((value) => !value)}
      >
        <span className="player-header-perspective-label">{label}</span>
        <span className={isActivePlayer ? "battlefield-name-text--active" : undefined}>{current?.name}</span>
      </button>
      {open ? (
        <div popover="manual" ref={listRef} id={listId} className="player-header-perspective-menu" role="listbox" aria-label={label}>
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
                  onSelect(Number(player.id));
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
