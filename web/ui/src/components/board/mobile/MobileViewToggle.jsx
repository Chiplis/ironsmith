import useUiText from "@/i18n/useUiText";
import { ChevronRight, Hand } from "lucide-react";
import { cn } from "@/lib/utils";

export default function MobileViewToggle({ mode = "battlefield", onToggle, className }) {
  const ui = useUiText();
  const isHandView = mode === "hand";
  return (
    <button
      type="button"
      className={cn("mobile-mtga-view-toggle", className)}
      onClick={onToggle}
      aria-label={ui(isHandView ? "View battlefield" : "View hand")}
    >
      <Hand className="size-3.5" aria-hidden="true" />
      <span>{isHandView ? ui("View Battlefield") : ui("View Hand")}</span>
      <ChevronRight className="size-3" aria-hidden="true" />
    </button>
  );
}
