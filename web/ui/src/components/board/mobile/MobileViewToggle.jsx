import { ChevronRight, Hand } from "lucide-react";
import { cn } from "@/lib/utils";

export default function MobileViewToggle({ mode = "battlefield", onToggle, className }) {
  const isHandView = mode === "hand";
  return (
    <button
      type="button"
      className={cn("mobile-mtga-view-toggle", className)}
      onClick={onToggle}
      aria-label={isHandView ? "View battlefield" : "View hand"}
    >
      <Hand className="size-3.5" aria-hidden="true" />
      <span>{isHandView ? "View Battlefield" : "View Hand"}</span>
      <ChevronRight className="size-3" aria-hidden="true" />
    </button>
  );
}
