import { Checkbox } from "@/components/ui/checkbox";

const VIEWABLE_ZONES = [
  { id: "battlefield", label: "Battlefield" },
  { id: "hand", label: "Hand" },
  { id: "graveyard", label: "Graveyard" },
  { id: "exile", label: "Exile" },
];

function normalizeZones(zones) {
  if (!Array.isArray(zones)) return ["battlefield"];
  const normalized = zones.filter((zone) => VIEWABLE_ZONES.some((entry) => entry.id === zone));
  return normalized.length > 0 ? normalized : ["battlefield"];
}

export default function ZoneViewer({ zoneViews = ["battlefield"], setZoneViews }) {
  const activeZones = normalizeZones(zoneViews);

  const toggleZone = (zoneId) => {
    if (typeof setZoneViews !== "function") return;
    if (activeZones.includes(zoneId)) {
      if (activeZones.length === 1) return;
      setZoneViews(activeZones.filter((zone) => zone !== zoneId));
      return;
    }
    setZoneViews([...activeZones, zoneId]);
  };

  return (
    <section className="zone-viewer relative z-0 bg-[#0e141d] rounded px-2 py-1.5 min-h-[28px]">
      <div className="flex items-start gap-2">
        <span className="text-[12px] uppercase tracking-wide font-semibold text-[#8fb1d6] shrink-0">Zones</span>
        <div className="flex items-center gap-2 flex-wrap">
          {VIEWABLE_ZONES.map((zone) => {
            const checked = activeZones.includes(zone.id);
            return (
              <label
                key={zone.id}
                className={`inline-flex items-center gap-1 text-[13px] whitespace-nowrap cursor-pointer uppercase transition-colors ${
                  checked ? "text-foreground" : "text-muted-foreground hover:text-foreground"
                }`}
              >
                <Checkbox
                  className="h-3.5 w-3.5"
                  checked={checked}
                  onCheckedChange={() => toggleZone(zone.id)}
                />
                {zone.label}
              </label>
            );
          })}
        </div>
      </div>
    </section>
  );
}
