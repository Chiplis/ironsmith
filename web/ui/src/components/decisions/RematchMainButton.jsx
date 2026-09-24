import { Button } from "@/components/ui/button";
import useRematchMainAction from "@/hooks/useRematchMainAction";
import { decisionButtonAccentVars } from "@/lib/decision-button-style";
import { cn } from "@/lib/utils";

// The main decision button after a multiplayer game: Play again, then Ready,
// then (host) Start game. It is always the local player's own action.
export default function RematchMainButton({ className = "", variant = "strip", subtitle = "" }) {
  const action = useRematchMainAction();
  if (!action.available) return null;
  const mobile = variant === "mobile";
  return (
    <Button
      type="button"
      variant="ghost"
      size="sm"
      className={cn(
        mobile
          ? "mobile-decision-primary-button decision-main-button"
          : "decision-neon-button decision-main-button decision-submit-button rounded-none px-3 text-[14px] font-bold uppercase",
        className
      )}
      style={decisionButtonAccentVars()}
      data-local-action="true"
      data-rematch-action={action.phase || "offer"}
      disabled={action.disabled}
      aria-disabled={action.disabled}
      onClick={() => { void action.press(); }}
    >
      {mobile ? (
        <>
          <span className="mobile-decision-primary-label">{action.label}</span>
          {subtitle ? <span className="mobile-decision-primary-subtitle">{subtitle}</span> : null}
        </>
      ) : action.label}
    </Button>
  );
}
