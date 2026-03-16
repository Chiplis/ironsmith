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
        className="zone-viewer-eye shrink-0 cursor-help"
        title="Toggles the visibility of zones for your local view only."
        aria-hidden="true"
      >
        <svg
          viewBox="0 0 24 24"
          className="zone-viewer-eye-icon h-3.5 w-3.5"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <path
            d="M2.6 12C4.5 8.9 7.9 7 12 7s7.5 1.9 9.4 5c-1.9 3.1-5.3 5-9.4 5s-7.5-1.9-9.4-5Z"
            fill="currentColor"
            fillOpacity="0.16"
            stroke="currentColor"
            strokeWidth="1.7"
            strokeLinejoin="round"
          />
          <circle cx="12" cy="12" r="3.15" fill="currentColor" fillOpacity="0.9" />
          <circle cx="12.9" cy="11.1" r="0.8" fill="#fff4d5" fillOpacity="0.72" />
        </svg>
      </span>
      <div className="flex items-center gap-2 flex-wrap">
        {VIEWABLE_ZONES.map((zone) => {
          const checked = activeZones.includes(zone.id);
          return (
            <label
              key={zone.id}
              className={`inline-flex items-center gap-1 text-[13px] whitespace-nowrap cursor-pointer uppercase transition-colors ${
                checked ? "text-[#eadbbb]" : "text-[#b8aa8c] hover:text-[#eadbbb]"
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
      <div className="zone-viewer zone-viewer--embedded flex items-center shrink-0">
        {zonesContent}
      </div>
    );
  }

  return (
    <section className="zone-viewer zone-viewer--panel relative z-0 rounded-none px-2 py-1.5 min-h-[28px]">
      <div className="flex items-center gap-4 min-w-0">
        {zonesContent}
      </div>
    </section>
  );
}
