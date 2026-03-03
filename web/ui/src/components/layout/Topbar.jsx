import { useGame } from "@/context/GameContext";
import { formatStep } from "@/lib/constants";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Checkbox } from "@/components/ui/checkbox";

const pill = "text-[13px] uppercase cursor-pointer hover:brightness-125 transition-all select-none";
const inputPill = "rounded-full bg-secondary text-secondary-foreground px-2.5 py-0.5 text-[13px] font-medium border-0 outline-none focus:ring-1 focus:ring-primary/50";
const selectPill = "rounded-full bg-secondary text-secondary-foreground px-2.5 py-0.5 text-[13px] font-medium border-0 outline-none cursor-pointer uppercase tracking-wide";

export default function Topbar({
  playerNames,
  setPlayerNames,
  startingLife,
  setStartingLife,
  onReset,
  onLoadDemoDecks,
  onAdvance,
  onChangePerspective,
  onRefresh,
  onToggleLog,
  onEnterDeckLoading,
  deckLoadingMode,
}) {
  const {
    state,
    wasmRegistryCount,
    wasmRegistryTotal,
    autoPassEnabled,
    setAutoPassEnabled,
    holdRule,
    setHoldRule,
  } = useGame();

  const players = state?.players || [];
  const perspective = state?.perspective;
  const me = players.find((p) => p.id === perspective) || players[0];
  const compiledLabel =
    Number.isFinite(wasmRegistryCount) && wasmRegistryCount >= 0 && wasmRegistryTotal > 0
      ? wasmRegistryTotal > 0
        ? `${wasmRegistryCount.toLocaleString()}/${wasmRegistryTotal.toLocaleString()}`
        : wasmRegistryCount.toLocaleString()
      : "-";

  return (
    <header className="panel-gradient flex items-center gap-1.5 rounded px-2.5 py-1 flex-wrap">
      <h1 className="m-0 text-[20px] uppercase tracking-wider whitespace-nowrap font-bold">
        Maishik
      </h1>

      <input
        className={`${inputPill} min-w-[60px] w-auto`}
        value={playerNames}
        onChange={(e) => setPlayerNames(e.target.value)}
      />
      <input
        className={`${inputPill} w-16 text-center [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none`}
        type="number"
        value={startingLife}
        min={1}
        onChange={(e) => setStartingLife(Number(e.target.value) || 20)}
      />

      <Badge variant="secondary" className={pill} onClick={onReset}>Reset</Badge>
      <Badge variant="secondary" className={pill} onClick={onLoadDemoDecks}>Decks</Badge>
      <Badge variant="secondary" className={pill} onClick={onEnterDeckLoading}>
        {deckLoadingMode ? "Cancel Load" : "Load Decks"}
      </Badge>

      <Separator orientation="vertical" className="h-4.5 mx-0.5" />

      <select
        className={selectPill}
        value={perspective ?? ""}
        onChange={(e) => onChangePerspective(Number(e.target.value))}
      >
        {players.map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
      </select>
      <Badge variant="secondary" className={pill} onClick={onAdvance}>Advance</Badge>
      <Badge variant="secondary" className={pill} onClick={onRefresh}>Refresh</Badge>

      <Separator orientation="vertical" className="h-4.5 mx-0.5" />

      <select
        className={selectPill}
        value={holdRule}
        onChange={(e) => setHoldRule(e.target.value)}
      >
        <option value="never">Never</option>
        <option value="if_actions">If actions</option>
        <option value="stack">Stack</option>
        <option value="main">Main</option>
        <option value="combat">Combat</option>
        <option value="ending">Ending</option>
        <option value="always">Always</option>
      </select>
      <label className="flex items-center gap-1 text-muted-foreground text-[13px] whitespace-nowrap cursor-pointer uppercase">
        <Checkbox
          checked={autoPassEnabled}
          onCheckedChange={(v) => setAutoPassEnabled(!!v)}
          className="h-3.5 w-3.5"
        />
        Auto-pass
      </label>

      <div className="flex-1" />

      <Badge variant="secondary" className="text-[13px] uppercase">
        Cards {compiledLabel}
      </Badge>
      <Badge variant="secondary" className="text-[13px] uppercase">
        View {me?.name || "-"}
      </Badge>
      <Badge variant="secondary" className="text-[13px] uppercase">
        Turn {state?.turn_number ?? "-"}
      </Badge>
      <Badge variant="secondary" className="text-[13px] uppercase">
        Phase {state?.phase ?? "-"}
      </Badge>
      <Badge variant="secondary" className="text-[13px] uppercase">
        Step {formatStep(state?.step)}
      </Badge>

      <Badge variant="secondary" className={pill} onClick={onToggleLog}>Log</Badge>
    </header>
  );
}
