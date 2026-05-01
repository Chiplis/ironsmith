import { useGame } from "@/context/GameContext";
import { Checkbox } from "@/components/ui/checkbox";
import { Slider } from "@/components/ui/slider";
import ZoneViewer from "@/components/board/ZoneViewer";

const selectPill = "stone-select rounded-none px-2.5 py-0.5 text-[13px] font-medium border-0 outline-none cursor-pointer uppercase tracking-wide";

export default function AddCardBar({
  compact = false,
  zoneViews = ["battlefield"],
  setZoneViews,
  onChangePerspective,
}) {
  const {
    state,
    semanticThreshold,
    setSemanticThreshold,
    cardsMeetingThreshold,
    multiplayer,
    autoPassEnabled,
    setAutoPassEnabled,
    holdRule,
    setHoldRule,
  } = useGame();

  const players = state?.players || [];
  const perspective = state?.perspective ?? 0;
  const matchLocked = multiplayer.matchStarted;

  return (
    <div className={`add-card-toolbar table-toolbar table-toolbar--secondary rounded-none px-3 py-2${compact ? " add-card-toolbar--compact" : ""}`}>
      <div className="add-card-toolbar-left">
        {!compact ? (
          <>
            <span className="add-card-toolbar-separator" aria-hidden="true" />
            <span
              className="add-card-toolbar-meta text-[13px] uppercase whitespace-nowrap cursor-help"
              title="Controls the threshold for semantic similarity when parsing custom cards. Higher means stricter text matching."
            >
              Fidelity
            </span>
            <Slider
              className="w-20"
              min={0}
              max={100}
              step={1}
              value={[Math.round(semanticThreshold)]}
              onValueChange={([value]) => setSemanticThreshold(value)}
            />
            <span className="add-card-toolbar-meta-value text-[13px] tabular-nums whitespace-nowrap">
              {semanticThreshold > 0 ? `${Math.round(semanticThreshold)}%` : "Off"}
              {" "}({cardsMeetingThreshold})
            </span>
          </>
        ) : null}
        <span className="add-card-toolbar-separator" aria-hidden="true" />
        <select
          className={selectPill}
          value={holdRule}
          onChange={(event) => setHoldRule(event.target.value)}
          aria-label="Auto-pass hold rule"
        >
          <option value="never">Never</option>
          <option value="if_actions">If actions</option>
          <option value="stack">Stack</option>
          <option value="main">Main</option>
          <option value="combat">Combat</option>
          <option value="ending">Ending</option>
          <option value="always">Always</option>
        </select>
        <label className="toolbar-checkbox add-card-toolbar-toggle flex items-center gap-1.5 whitespace-nowrap cursor-pointer uppercase">
          <Checkbox
            checked={autoPassEnabled}
            onCheckedChange={(value) => setAutoPassEnabled(!!value)}
            className="h-3.5 w-3.5"
          />
          Auto-pass
        </label>
        <label className="add-card-toolbar-perspective add-card-toolbar-toggle flex items-center gap-1.5 whitespace-nowrap uppercase">
          <span>Playing as</span>
          <select
            className={`${selectPill} add-card-toolbar-perspective-select`}
            value={perspective}
            disabled={matchLocked}
            onChange={(event) => onChangePerspective?.(Number(event.target.value))}
            aria-label="Playing as"
          >
            {players.map((player) => (
              <option key={player.id} value={player.id}>
                {player.name}
              </option>
            ))}
          </select>
        </label>
      </div>
      <div className="add-card-toolbar-right">
        <ZoneViewer zoneViews={zoneViews} setZoneViews={setZoneViews} embedded />
      </div>
    </div>
  );
}
