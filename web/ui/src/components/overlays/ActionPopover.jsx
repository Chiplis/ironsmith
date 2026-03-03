import { useEffect, useRef } from "react";
import { Button } from "@/components/ui/button";

export default function ActionPopover({ anchorRect, actions, onAction, onClose }) {
  const ref = useRef(null);

  useEffect(() => {
    function handleClickOutside(e) {
      if (ref.current && !ref.current.contains(e.target)) {
        onClose();
      }
    }
    function handleEscape(e) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [onClose]);

  // Position near the anchor element
  const maxLeft = window.innerWidth - 16;
  const left = Math.max(8, Math.min(anchorRect.left + 10, maxLeft - 260));
  const top = Math.max(8, Math.min(anchorRect.bottom + 8, window.innerHeight - 120));

  return (
    <div
      ref={ref}
      className="fixed z-50 grid gap-1 p-2 min-w-[200px] max-w-[260px] border border-[#6284a8] bg-[#0d1520] shadow-lg rounded-sm"
      style={{ left: `${left}px`, top: `${top}px` }}
    >
      <div className="text-[13px] uppercase tracking-wider text-muted-foreground font-bold pb-0.5 border-b border-game-line-2">
        Available actions
      </div>
      {actions.map((action) => (
        <Button
          key={action.index}
          variant="outline"
          size="sm"
          className="h-7 text-[14px] justify-start px-2 whitespace-normal text-left"
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onAction(action);
          }}
        >
          {action.label}
        </Button>
      ))}
    </div>
  );
}
