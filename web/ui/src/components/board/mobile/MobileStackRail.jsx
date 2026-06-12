import { useCallback, useMemo, useState } from "react";
import { X } from "lucide-react";
import { useGame } from "@/context/GameContext";
import StackCard from "@/components/cards/StackCard";
import useNewCards from "@/hooks/useNewCards";
import useStackStartAlert from "@/hooks/useStackStartAlert";
import useMobileLongPress from "@/hooks/useMobileLongPress";
import { cn } from "@/lib/utils";

const RAIL_VISIBLE_LIMIT = 5;

function MobileStackRailEntry({
  entry,
  isNew,
  showStackAlert,
  isFocused,
  onFocus,
  onLongPressInspect,
}) {
  const handleLongPress = useCallback(() => {
    onLongPressInspect?.(entry?.inspect_object_id ?? entry?.id, {
      source: "stack",
      stackEntry: entry,
    });
  }, [entry, onLongPressInspect]);
  const longPress = useMobileLongPress({ onLongPress: handleLongPress });

  const handleClick = useCallback(() => {
    if (longPress.consumeTrigger()) return;
    onFocus?.(entry);
  }, [entry, longPress, onFocus]);

  return (
    <div
      className={cn(
        "mobile-mtga-stack-rail-entry",
        isFocused && "mobile-mtga-stack-rail-entry--focused"
      )}
      data-arrow-anchor="stack"
      data-object-id={entry?.id}
      data-card-name={entry?.name || `Object#${entry?.id}`}
      onPointerDown={longPress.onPointerDown}
      onPointerMove={longPress.onPointerMove}
      onPointerUp={longPress.onPointerUp}
      onPointerCancel={longPress.onPointerCancel}
      onPointerLeave={longPress.onPointerLeave}
      onClick={handleClick}
    >
      <StackCard
        entry={entry}
        isNew={isNew}
        isActive={isFocused}
        showStackAlert={showStackAlert}
        className="mobile-mtga-stack-rail-card"
        entryMotion="mobile-stack"
      />
    </div>
  );
}

function MobileStackBrowser({ entries, focusedId, onFocus, onClose, onInspect }) {
  return (
    <section
      className="mobile-mtga-stack-browser"
      role="dialog"
      aria-modal="true"
      aria-label="Full stack"
    >
      <header className="mobile-mtga-stack-browser-header">
        <span className="mobile-mtga-stack-browser-title">Stack</span>
        <span className="mobile-mtga-stack-browser-count">{entries.length}</span>
        <button
          type="button"
          className="mobile-mtga-stack-browser-close"
          aria-label="Close stack browser"
          onClick={onClose}
        >
          <X className="size-4" aria-hidden="true" />
        </button>
      </header>
      <div className="mobile-mtga-stack-browser-list">
        {entries.map((entry) => (
          <button
            key={entry.id}
            type="button"
            className={cn(
              "mobile-mtga-stack-browser-row",
              focusedId != null && String(focusedId) === String(entry.id)
                && "mobile-mtga-stack-browser-row--focused"
            )}
            onClick={() => {
              onFocus?.(entry);
              onClose?.();
            }}
            onContextMenu={(event) => {
              event.preventDefault();
              onInspect?.(entry?.inspect_object_id ?? entry?.id, {
                source: "stack",
                stackEntry: entry,
              });
            }}
          >
            <span className="mobile-mtga-stack-browser-name">
              {entry?.name || `Object #${entry?.id}`}
            </span>
            <span className="mobile-mtga-stack-browser-kind">
              {entry?.ability_kind ? `${entry.ability_kind} ability` : "Spell"}
            </span>
          </button>
        ))}
      </div>
    </section>
  );
}

export default function MobileStackRail({
  objects = [],
  focusedStackObjectId = null,
  onFocusStackObject,
  onInspect,
  className,
}) {
  const { state } = useGame();
  const stackIds = useMemo(
    () => objects.map((entry) => (entry?.id != null ? String(entry.id) : null)).filter(Boolean),
    [objects]
  );
  const { newIds } = useNewCards(stackIds);
  const { alertEntryId, dismissAlert } = useStackStartAlert(objects, state?.perspective);
  const [browserOpen, setBrowserOpen] = useState(false);

  const handleFocus = useCallback((entry) => {
    dismissAlert();
    onFocusStackObject?.(entry);
  }, [dismissAlert, onFocusStackObject]);

  const handleLongPressInspect = useCallback((objectId, meta) => {
    dismissAlert();
    onInspect?.(objectId, meta);
  }, [dismissAlert, onInspect]);

  if (!objects.length) return null;

  // getVisibleStackObjects is already top-first (index 0 is the top / resolving
  // object — see getVisibleTopStackObject), matching the desktop panels.
  const topFirst = objects;
  const visible = topFirst.slice(0, RAIL_VISIBLE_LIMIT);
  const overflow = topFirst.length - visible.length;

  return (
    <>
      <aside
        className={cn("mobile-mtga-stack-rail", className)}
        aria-label={`Stack (${objects.length} item${objects.length === 1 ? "" : "s"})`}
      >
        {visible.map((entry) => (
          <MobileStackRailEntry
            key={entry.id}
            entry={entry}
            isNew={newIds.has(String(entry.id))}
            showStackAlert={alertEntryId != null && String(entry.id) === String(alertEntryId)}
            isFocused={focusedStackObjectId != null && String(focusedStackObjectId) === String(entry.id)}
            onFocus={handleFocus}
            onLongPressInspect={handleLongPressInspect}
          />
        ))}
        {overflow > 0 ? (
          <button
            type="button"
            className="mobile-mtga-stack-rail-overflow"
            aria-label={`Show ${overflow} more stack item${overflow === 1 ? "" : "s"}`}
            onClick={() => setBrowserOpen(true)}
          >
            +{overflow}
          </button>
        ) : null}
      </aside>

      {browserOpen ? (
        <MobileStackBrowser
          entries={topFirst}
          focusedId={focusedStackObjectId}
          onFocus={handleFocus}
          onClose={() => setBrowserOpen(false)}
          onInspect={handleLongPressInspect}
        />
      ) : null}
    </>
  );
}
