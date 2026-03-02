import { useGame } from "@/context/GameContext";
import { formatStep } from "@/lib/constants";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Checkbox } from "@/components/ui/checkbox";

export default function Topbar({
  playerNames,
  setPlayerNames,
  startingLife,
  setStartingLife,
  onReset,
  onLoadDecks,
  onDraw,
  onAdvance,
  onChangePerspective,
  onRefresh,
  onToggleLog,
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
      <h1 className="m-0 text-[15px] uppercase tracking-wider whitespace-nowrap font-bold">
        Maishik
      </h1>

      <Input
        className="h-6 text-[11px] w-auto min-w-[60px] bg-transparent"
        value={playerNames}
        onChange={(e) => setPlayerNames(e.target.value)}
      />
      <Input
        className="h-6 text-[11px] w-12 bg-transparent"
        type="number"
        value={startingLife}
        min={1}
        onChange={(e) => setStartingLife(Number(e.target.value) || 20)}
      />
      <Button variant="outline" size="sm" className="h-6 text-[11px] px-2" onClick={onReset}>
        Reset
      </Button>
      <Button variant="outline" size="sm" className="h-6 text-[11px] px-2" onClick={onLoadDecks}>
        Decks
      </Button>
      <Button variant="outline" size="sm" className="h-6 text-[11px] px-2" onClick={onDraw}>
        Draw 7
      </Button>

      <Separator orientation="vertical" className="h-4.5 mx-0.5" />

      <span className="text-muted-foreground text-[11px] whitespace-nowrap">Player</span>
      <select
        className="h-6 text-[11px] px-1.5 bg-[#0b1118] border border-[#344a61] text-foreground rounded-sm uppercase tracking-wide cursor-pointer"
        value={perspective ?? ""}
        onChange={(e) => onChangePerspective(Number(e.target.value))}
      >
        {players.map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
      </select>
      <Button variant="outline" size="sm" className="h-6 text-[11px] px-2" onClick={onAdvance}>
        Advance
      </Button>
      <Button variant="outline" size="sm" className="h-6 text-[11px] px-2" onClick={onRefresh}>
        Refresh
      </Button>

      <Separator orientation="vertical" className="h-4.5 mx-0.5" />

      <span className="text-muted-foreground text-[11px] whitespace-nowrap">Hold</span>
      <select
        className="h-6 text-[11px] px-1.5 bg-[#0b1118] border border-[#344a61] text-foreground rounded-sm cursor-pointer"
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
      <label className="flex items-center gap-1 text-muted-foreground text-[11px] whitespace-nowrap cursor-pointer">
        <Checkbox
          checked={autoPassEnabled}
          onCheckedChange={(v) => setAutoPassEnabled(!!v)}
          className="h-3.5 w-3.5"
        />
        Auto-pass
      </label>

      <div className="flex-1" />

      <Badge variant="secondary" className="text-[10px] uppercase">
        Cards {compiledLabel}
      </Badge>
      <Badge variant="secondary" className="text-[10px] uppercase">
        View {me?.name || "-"}
      </Badge>
      <Badge variant="secondary" className="text-[10px] uppercase">
        Turn {state?.turn_number ?? "-"}
      </Badge>
      <Badge variant="secondary" className="text-[10px] uppercase">
        Phase {state?.phase ?? "-"}
      </Badge>
      <Badge variant="secondary" className="text-[10px] uppercase">
        Step {formatStep(state?.step)}
      </Badge>

      <Button variant="outline" size="sm" className="h-6 text-[11px] px-2" onClick={onToggleLog}>
        Log
      </Button>
    </header>
  );
}
