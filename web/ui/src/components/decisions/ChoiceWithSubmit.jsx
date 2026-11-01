import { Button } from "@/components/ui/button";
import useUiText from "@/i18n/useUiText";
import { usePointerClickGuard } from "@/lib/usePointerClickGuard";
import { cn } from "@/lib/utils";

// Keep Submit a sibling of the choice button, so it never toggles the choice.
export default function ChoiceWithSubmit({ children, onSubmit, label, highlighted = false, className, ...props }) {
  const ui = useUiText();
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();
  return <div {...props} className={cn("decision-target-option decision-choice-with-submit", onSubmit && "has-quick-submit", highlighted && "is-hovered", className)}>
    {children}
    {onSubmit && <Button variant="ghost" size="sm" className="decision-neon-button decision-target-quick-submit"
      aria-label={ui("Select {0} and submit", { 0: label })}
      onPointerDown={(event) => {
        if (!registerPointerDown(event)) return;
        event.preventDefault();
        event.stopPropagation();
        onSubmit();
      }}
      onClick={(event) => {
        event.stopPropagation();
        if (shouldHandleClick(event)) onSubmit();
      }}>{ui("Submit")}</Button>}
  </div>;
}
