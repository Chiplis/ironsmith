import { Library, Skull, Sparkles, Crown } from "lucide-react";
import { cn } from "@/lib/utils";

const ZONES = [
  { key: "graveyard", label: "Graveyard", short: "Gy", Icon: Skull },
  { key: "exile", label: "Exile", short: "Ex", Icon: Sparkles },
  { key: "library", label: "Library", short: "Lib", Icon: Library },
  { key: "command", label: "Command", short: "Cmd", Icon: Crown },
  { key: "ante", label: "Ante", short: "Ante", Icon: Crown },
];

function zoneCount(player, zone) {
  switch (zone) {
    case "graveyard":
      return Number(player?.graveyard_size ?? 0);
    case "exile":
      return Array.isArray(player?.exile_cards)
        ? player.exile_cards.length
        : Number(player?.exile_size ?? 0);
    case "library":
      return Number(player?.library_size ?? 0);
    case "command":
      return Array.isArray(player?.command_zone)
        ? player.command_zone.length
        : Number(player?.command_size ?? 0);
    case "ante":
      return Array.isArray(player?.ante_cards)
        ? player.ante_cards.length
        : Number(player?.ante_size ?? 0);
    default:
      return 0;
  }
}

export default function MobileZoneTray({ player, onOpenZone, className }) {
  if (!player) return null;
  const items = ZONES.map((zone) => {
    const { key, label, short } = zone;
    const ZoneIcon = zone.Icon;
    const count = zoneCount(player, key);
    if ((key === "command" || key === "ante") && count <= 0) return null;
    return (
      <button
        key={key}
        type="button"
        className="mobile-mtga-zone-pill"
        data-zone={key}
        aria-label={`${label} (${count})`}
        onClick={() => onOpenZone?.(key)}
      >
        <ZoneIcon className="size-3" aria-hidden="true" />
        <span className="mobile-mtga-zone-pill-short">{short}</span>
        <span className="mobile-mtga-zone-pill-count">{count}</span>
      </button>
    );
  }).filter(Boolean);

  return (
    <div
      className={cn("mobile-mtga-zone-tray", className)}
      role="group"
      aria-label="Your zones"
    >
      {items}
    </div>
  );
}
