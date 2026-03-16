import { useGame } from "@/context/GameContext";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Checkbox } from "@/components/ui/checkbox";
import PhaseTrack from "@/components/board/PhaseTrack";
import TopbarMenuSheet from "./TopbarMenuSheet";

const pill = "stone-pill text-[13px] uppercase cursor-pointer hover:brightness-110 transition-all select-none";
const selectPill = "stone-select rounded-none px-2.5 py-0.5 text-[13px] font-medium border-0 outline-none cursor-pointer uppercase tracking-wide";

export default function Topbar({
  playerNames,
  setPlayerNames,
  startingLife,
  setStartingLife,
  onReset,
  onChangePerspective,
  onRefresh,
  onToggleLog,
  onEnterDeckLoading,
  onOpenLobby,
  deckLoadingMode,
}) {
  const {
    autoPassEnabled,
    setAutoPassEnabled,
    holdRule,
    setHoldRule,
    inspectorDebug,
    setInspectorDebug,
    multiplayer,
  } = useGame();

  const matchLocked = multiplayer.matchStarted;

  return (
    <header className="table-toolbar table-toolbar--primary flex items-center gap-2 rounded px-2.5 py-1 overflow-x-auto overflow-y-hidden">
      <h1 className="toolbar-brand m-0 text-[20px] uppercase tracking-wider whitespace-nowrap font-bold">
        Ironsmith
      </h1>

      <Separator orientation="vertical" className="h-4.5 mx-0.5" />

      <select
        className={selectPill}
        value={holdRule}
        disabled={matchLocked}
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
      <label className="toolbar-checkbox flex items-center gap-1 text-[13px] whitespace-nowrap cursor-pointer uppercase">
        <Checkbox
          checked={autoPassEnabled}
          disabled={matchLocked}
          onCheckedChange={(v) => setAutoPassEnabled(!!v)}
          className="h-3.5 w-3.5"
        />
        Auto-pass
      </label>

      <div className="mx-1 min-w-[200px] flex-1">
        <PhaseTrack />
      </div>

      <div className="flex items-center gap-1 shrink-0">
        <label className="toolbar-checkbox toolbar-debug-toggle flex items-center gap-1 text-[13px] whitespace-nowrap cursor-pointer uppercase">
          <Checkbox
            checked={inspectorDebug}
            onCheckedChange={(value) => setInspectorDebug(!!value)}
            className="h-3.5 w-3.5"
          />
          Debug
        </label>
        <Badge variant="secondary" className={pill} onClick={onToggleLog}>Log</Badge>
        <TopbarMenuSheet
          playerNames={playerNames}
          setPlayerNames={setPlayerNames}
          startingLife={startingLife}
          setStartingLife={setStartingLife}
          onReset={onReset}
          onChangePerspective={onChangePerspective}
          onRefresh={onRefresh}
          onEnterDeckLoading={onEnterDeckLoading}
          onOpenLobby={onOpenLobby}
          deckLoadingMode={deckLoadingMode}
        />
      </div>
    </header>
  );
}
