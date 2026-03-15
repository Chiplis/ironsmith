import { Checkbox } from "@/components/ui/checkbox";

const VIEWABLE_ZONES = [
  { id: "battlefield", label: "Battlefield" },
  { id: "hand", label: "Hand" },
  { id: "graveyard", label: "GY" },
  { id: "library", label: "Deck" },
  { id: "exile", label: "Exile" },
  { id: "command", label: "CZ" },
];

function normalizeZones(zones) {
  if (!Array.isArray(zones)) return ["battlefield"];
  const normalized = zones.filter((zone) => VIEWABLE_ZONES.some((entry) => entry.id === zone));
  return Array.from(new Set(["battlefield", ...normalized]));
}

export default function ZoneViewer({
  zoneViews = ["battlefield"],
  setZoneViews,
  embedded = false,
}) {
  const activeZones = normalizeZones(zoneViews);

  const toggleZone = (zoneId) => {
    if (typeof setZoneViews !== "function") return;
    if (zoneId === "battlefield") return;
    if (activeZones.includes(zoneId)) {
      if (activeZones.length === 1) return;
      setZoneViews(activeZones.filter((zone) => zone !== zoneId));
      return;
    }
    setZoneViews([...activeZones, zoneId]);
  };

  const zonesContent = (
    <div className="flex items-center gap-2 shrink-0">
      <span
        className="text-[12px] uppercase tracking-wide font-semibold text-[#8fb1d6] shrink-0 cursor-help"
        title="Toggles the visibility of zones for your local view only."
      >
        Local Zones
      </span>
      <div className="flex items-center gap-2 flex-wrap">
        {VIEWABLE_ZONES.map((zone) => {
          const checked = activeZones.includes(zone.id);
          return (
            <label
              key={zone.id}
              className={`inline-flex items-center gap-1 text-[13px] whitespace-nowrap cursor-pointer uppercase transition-colors ${
                checked ? "text-foreground" : "text-muted-foreground hover:text-foreground"
              } ${zone.id === "battlefield" ? "cursor-default opacity-85" : ""}`}
            >
              <Checkbox
                className="h-3.5 w-3.5"
                checked={checked}
                disabled={zone.id === "battlefield"}
                onCheckedChange={() => toggleZone(zone.id)}
              />
              {zone.label}
            </label>
          );
        })}
      </div>
    </div>
  );

  if (embedded) {
    return (
      <div className="zone-viewer flex items-center min-w-0">
        {zonesContent}
      </div>
    );
  }

  return (
    <section className="zone-viewer relative z-0 bg-[#0e141d] rounded px-2 py-1.5 min-h-[28px]">
      <div className="flex items-center gap-4 min-w-0">
        {zonesContent}
      </div>
    </section>
  );
}
