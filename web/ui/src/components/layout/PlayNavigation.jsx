import { BookOpen, Users } from "lucide-react";
import useUiText from "@/i18n/useUiText";
import CardCreationControls from "./CardCreationControls";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";

export default function PlayNavigation({ onOpenLobby, onAddCardNotice }) {
  const ui = useUiText();
  return <nav className="home-navigation" aria-label={ui("Game menu")}>
    <button type="button" className="home-action home-action-play" onClick={onOpenLobby} aria-label={ui("Lobby")} title={ui("Lobby")}>
      <Users size={16} aria-hidden="true" /><span className="home-action-label">{ui("Lobby")}</span>
    </button>
    <CardCreationControls navigation onAddCardNotice={onAddCardNotice} />
  </nav>;
}

export function LearnButton({ iconOnly = false }) {
  const ui = useUiText();
  const recordClick = () => {
    // The empty static resource is a no-op; count requests in hosting logs.
    // A unique URL also avoids CDN cache hits hiding repeat clicks.
    const clickId = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
    const url = `${import.meta.env.BASE_URL}events/learn-click.txt?click=${clickId}`;
    void fetch(url, { cache: "no-store", keepalive: true, credentials: "omit" }).catch(() => {});
  };
  return <Popover>
    <PopoverTrigger asChild>
      <button type="button" className="home-action home-action-learn" onClick={recordClick}
        aria-label={ui("Learn")} title={iconOnly ? ui("Learn") : undefined}>
        <BookOpen size={14} aria-hidden="true" />{iconOnly ? null : ui("Learn")}
      </button>
    </PopoverTrigger>
    <PopoverContent side="top" className="z-[200] w-auto px-4 py-3 text-sm">
      {ui("Coming soon!")}
    </PopoverContent>
  </Popover>;
}
