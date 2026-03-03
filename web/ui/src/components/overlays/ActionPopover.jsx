import { useEffect, useRef, useState } from "react";
import { useHover } from "@/context/HoverContext";
import { SymbolText } from "@/lib/mana-symbols";

/** Strip "Activate CardName: " or "Cast CardName" prefix for compact display. */
function stripActionPrefix(label) {
  const activateMatch = label.match(/^Activate\s+.+?:\s*(.+)$/i);
  if (activateMatch) return activateMatch[1];
  return label;
}

export default function ActionPopover({ anchorRect, actions, onAction, onClose }) {
  const ref = useRef(null);
  const { hoverCard, clearHover } = useHover();
  const [phase, setPhase] = useState("entering");
  const [hoveredIdx, setHoveredIdx] = useState(-1);

  useEffect(() => {
    const raf = requestAnimationFrame(() => setPhase("open"));
    return () => cancelAnimationFrame(raf);
  }, []);

  const handleClose = () => {
    if (phase === "exiting") return;
    setPhase("exiting");
    setTimeout(onClose, 250);
  };

  useEffect(() => {
    function handleClickOutside(e) {
      if (ref.current && !ref.current.contains(e.target)) {
        handleClose();
      }
    }
    function handleEscape(e) {
      if (e.key === "Escape") handleClose();
    }
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [phase]);

  const popoverWidth = 260;
  const anchorCenterX = anchorRect.left + anchorRect.width / 2;
  const maxLeft = window.innerWidth - 16;
  const left = Math.max(8, Math.min(anchorCenterX - popoverWidth / 2, maxLeft - popoverWidth));
  const popoverHeight = actions.length * 34 + 16;
  const top = Math.max(8, anchorRect.top - popoverHeight - 14);
  const tailLeft = Math.max(16, Math.min(anchorCenterX - left, popoverWidth - 16));

  const originX = tailLeft;
  const isOpen = phase === "open";

  // Tail color matches last row when hovered
  const lastIdx = actions.length - 1;
  const tailColor = hoveredIdx === lastIdx ? "#1a1a1a" : "#f0f0f0";

  return (
    <div
      ref={ref}
      className="fixed z-50"
      style={{
        left: `${left}px`,
        top: `${top}px`,
        filter: "drop-shadow(0 4px 12px rgba(0,0,0,0.4))",
        transformOrigin: `${originX}px 100%`,
        transform: isOpen ? "scaleY(1) scaleX(1)" : "scaleY(0.1) scaleX(0.6)",
        opacity: isOpen ? 1 : 0,
        transition: "transform 250ms cubic-bezier(0.34, 1.56, 0.64, 1), opacity 200ms ease",
      }}
    >
      <div
        className="min-w-[200px] max-w-[280px] rounded-xl overflow-hidden"
        style={{ background: "#f0f0f0" }}
      >
        {actions.map((action, i) => {
          const objId = action.object_id != null ? String(action.object_id) : null;
          const isFirst = i === 0;
          return (
            <div
              key={action.index}
              className="px-3 py-2 text-[14px] font-bold text-[#1a1a1a] cursor-pointer select-none transition-all duration-200 hover:bg-[#1a1a1a] hover:text-white"
              style={{
                borderTop: !isFirst ? "1px solid #d8d8d8" : undefined,
              }}
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                onAction(action);
              }}
              onMouseEnter={() => {
                setHoveredIdx(i);
                if (objId) hoverCard(objId);
              }}
              onMouseLeave={() => {
                setHoveredIdx(-1);
                clearHover();
              }}
            >
              <SymbolText text={stripActionPrefix(action.label)} />
            </div>
          );
        })}
      </div>
      {/* Speech bubble tail */}
      <div
        className="absolute"
        style={{
          left: `${tailLeft}px`,
          bottom: "-10px",
          transform: "translateX(-50%)",
          width: 0,
          height: 0,
          borderLeft: "10px solid transparent",
          borderRight: "10px solid transparent",
          borderTop: `12px solid ${tailColor}`,
          transition: "border-top-color 200ms ease",
        }}
      />
    </div>
  );
}
