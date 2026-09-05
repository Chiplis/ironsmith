import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useHoverActions } from "@/context/HoverContext";
import { SymbolText } from "@/lib/mana-symbols";
import { buildPriorityActionGroups } from "@/lib/priority-action-groups";

/** Strip "Activate CardName: " or "Cast CardName" prefix for compact display. */
function stripActionPrefix(label) {
  const activateMatch = label.match(/^Activate\s+.+?:\s*(.+)$/i);
  if (activateMatch) return activateMatch[1];
  return label;
}

function dispatchHandActionHover(objectId = null) {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent("ironsmith:hand-action-hover", {
    detail: { objectId: objectId != null ? String(objectId) : null },
  }));
}

export default function ActionPopover({
  anchorRect,
  actions,
  onAction,
  onClose,
  title = null,
  subtitle = null,
  variant = "light",
  collapseEquivalentActions = true,
}) {
  const ref = useRef(null);
  const openedAtRef = useRef(0);
  const { hoverCard, clearHover } = useHoverActions();
  const [phase, setPhase] = useState("entering");
  const [hoveredIdx, setHoveredIdx] = useState(-1);

  useEffect(() => {
    openedAtRef.current = Date.now();
    const raf = requestAnimationFrame(() => setPhase("open"));
    return () => {
      cancelAnimationFrame(raf);
      dispatchHandActionHover(null);
    };
  }, []);

  const handleClose = useCallback(() => {
    if (phase === "exiting") return;
    setHoveredIdx(-1);
    clearHover();
    dispatchHandActionHover(null);
    setPhase("exiting");
    setTimeout(onClose, 250);
  }, [clearHover, onClose, phase]);

  useEffect(() => {
    function handleClickOutside(e) {
      if (ref.current && !ref.current.contains(e.target)) {
        handleClose();
      }
    }
    function handleEscape(e) {
      if (e.key === "Escape") handleClose();
    }
    document.addEventListener("pointerdown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("pointerdown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [handleClose]);

  const palette = useMemo(
    () => (
      variant === "game"
        ? {
          panel: "var(--ui-surface-raised, #211f1c)",
          titleText: "var(--ui-text-primary, #f3ead8)",
          subtitleText: "var(--ui-text-secondary, #c4b79f)",
          rowText: "var(--ui-text-primary, #f3ead8)",
          rowDivider: "rgba(205, 180, 132, 0.14)",
          rowHoverBg: "#383021",
          rowHoverText: "#ffe3a3",
          tail: "var(--ui-surface-raised, #211f1c)",
          tailHover: "#383021",
        }
        : {
          panel: "#f0f0f0",
          titleText: "#1a1a1a",
          subtitleText: "#444",
          rowText: "#1a1a1a",
          rowDivider: "#d8d8d8",
          rowHoverBg: "#1a1a1a",
          rowHoverText: "#ffffff",
          tail: "#f0f0f0",
          tailHover: "#1a1a1a",
        }
    ),
    [variant]
  );
  const popoverWidth = variant === "game" ? 292 : 260;
  const rowHeight = 34;
  const headerHeight = (title || subtitle) ? (subtitle ? 58 : 40) : 0;
  const actionGroups = useMemo(
    () => (
      collapseEquivalentActions
        ? buildPriorityActionGroups(actions)
        : (actions || []).map((action, actionIndex) => ({
          key: `action-${action?.index ?? actionIndex}-${action?.kind || "unknown"}`,
          label: action?.label || "Action",
          firstAction: action,
          hoverObjectId: action?.object_id ?? null,
        }))
    ),
    [actions, collapseEquivalentActions]
  );
  const popoverHeight = (actionGroups.length * rowHeight) + headerHeight + 16;
  const anchorCenterX = anchorRect.left + anchorRect.width / 2;
  const maxLeft = window.innerWidth - 16;
  const left = Math.max(8, Math.min(anchorCenterX - popoverWidth / 2, maxLeft - popoverWidth));
  const viewportHeight = window.innerHeight;
  const spaceAbove = anchorRect.top - 8;
  const spaceBelow = viewportHeight - anchorRect.bottom - 8;
  const placeAbove = spaceAbove >= popoverHeight || spaceAbove > spaceBelow;
  const top = placeAbove
    ? Math.max(8, anchorRect.top - popoverHeight - 14)
    : Math.min(viewportHeight - popoverHeight - 8, anchorRect.bottom + 14);
  const tailLeft = Math.max(16, Math.min(anchorCenterX - left, popoverWidth - 16));

  const originX = tailLeft;
  const isOpen = phase === "open";

  // Tail color matches last row when hovered
  const lastIdx = actionGroups.length - 1;
  const tailColor = hoveredIdx === lastIdx ? palette.tailHover : palette.tail;
  const tailSize = 11;
  const yOrigin = placeAbove ? "100%" : "0%";

  return (
    <div
      ref={ref}
      className="fixed z-[152]"
      data-action-popover="true"
      data-action-count={actionGroups.length}
      style={{
        fontFamily: variant === "game" ? "var(--ironsmith-ui-font, 'Rajdhani'), system-ui, sans-serif" : undefined,
        left: `${left}px`,
        top: `${top}px`,
        filter: "drop-shadow(0 4px 12px rgba(0,0,0,0.4))",
        transformOrigin: `${originX}px ${yOrigin}`,
        transform: isOpen
          ? "scaleY(1) scaleX(1)"
          : `translateY(${placeAbove ? "6px" : "-6px"}) scaleY(0.2) scaleX(0.6)`,
        opacity: isOpen ? 1 : 0,
        transition: "transform 250ms cubic-bezier(0.34, 1.56, 0.64, 1), opacity 200ms ease",
      }}
    >
      <div
        className="min-w-[200px] rounded-none overflow-hidden"
        style={{
          width: `${popoverWidth}px`,
          background: palette.panel,
          border: variant === "game" ? "1px solid var(--ui-border-strong, rgba(218, 188, 126, 0.42))" : "none",
        }}
      >
        {(title || subtitle) && (
          <div
            className="px-3 py-2"
            style={{
              borderBottom: `1px solid ${palette.rowDivider}`,
              background: variant === "game" ? "var(--ui-surface-control, #2b2823)" : "rgba(255,255,255,0.82)",
            }}
          >
            {title && (
              <div className="text-[13px] font-bold leading-tight" style={{ color: palette.titleText }}>
                {title}
              </div>
            )}
            {subtitle && (
              <div className="mt-0.5 text-[12px] leading-snug" style={{ color: palette.subtitleText }}>
                {subtitle}
              </div>
            )}
          </div>
        )}
        {actionGroups.map((group, i) => {
          const action = group.firstAction;
          const objId = group.hoverObjectId != null
            ? String(group.hoverObjectId)
            : action?.object_id != null ? String(action.object_id) : null;
          const isFirst = i === 0;
          const showDivider = !isFirst || Boolean(title || subtitle);
          return (
            <div
              key={group.key}
              className="px-3 py-2 cursor-pointer select-none transition-colors duration-150"
              style={{
                fontSize: variant === "game" ? "12px" : "14px",
                fontWeight: variant === "game" ? 600 : 700,
                lineHeight: 1.4,
                color: palette.rowText,
                borderTop: showDivider ? `1px solid ${palette.rowDivider}` : undefined,
                background: hoveredIdx === i ? palette.rowHoverBg : "transparent",
              }}
              onClick={(e) => {
                if ((Date.now() - openedAtRef.current) < 160) return;
                e.preventDefault();
                e.stopPropagation();
                dispatchHandActionHover(null);
                onAction(action);
              }}
              onMouseEnter={() => {
                setHoveredIdx(i);
                if (objId) hoverCard(objId);
                dispatchHandActionHover(objId);
              }}
              onMouseLeave={() => {
                setHoveredIdx(-1);
                clearHover();
                dispatchHandActionHover(null);
              }}
              onFocus={() => {
                setHoveredIdx(i);
                if (objId) hoverCard(objId);
                dispatchHandActionHover(objId);
              }}
              onBlur={() => {
                setHoveredIdx(-1);
                clearHover();
                dispatchHandActionHover(null);
              }}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  if ((Date.now() - openedAtRef.current) < 160) return;
                  event.preventDefault();
                  dispatchHandActionHover(null);
                  onAction(action);
                }
              }}
              role="button"
              tabIndex={0}
            >
              <div
                style={{
                  color: hoveredIdx === i ? palette.rowHoverText : palette.rowText,
                  transition: "color 180ms ease",
                }}
              >
                <SymbolText text={stripActionPrefix(group.label || action.label)} />
              </div>
            </div>
          );
        })}
      </div>
      {/* Speech bubble tail */}
      <div
        className="absolute"
        style={{
          left: `${tailLeft}px`,
          [placeAbove ? "bottom" : "top"]: `-${tailSize + 1}px`,
          transform: "translateX(-50%)",
          width: 0,
          height: 0,
          borderLeft: `${tailSize}px solid transparent`,
          borderRight: `${tailSize}px solid transparent`,
          borderTop: placeAbove ? `${tailSize + 1}px solid ${tailColor}` : "none",
          borderBottom: placeAbove ? "none" : `${tailSize + 1}px solid ${tailColor}`,
          transition: "transform 200ms ease, opacity 200ms ease",
        }}
      />
    </div>
  );
}
