import { useMemo, useState } from "react";
import { useGame } from "@/context/GameContext";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { ExternalLink, Github, RefreshCw, Settings2 } from "lucide-react";

const inputClass =
  "w-full rounded-md border border-[#344a61] bg-[#0b1118] px-3 py-2 text-[14px] text-foreground outline-none focus:border-primary/60 disabled:cursor-not-allowed disabled:opacity-50";
const labelClass =
  "grid gap-1 text-[11px] uppercase tracking-[0.2em] text-muted-foreground";
const sectionClass =
  "grid gap-3 rounded-lg border border-game-line bg-[linear-gradient(180deg,rgba(18,27,39,0.96)_0%,rgba(10,17,24,0.96)_100%)] p-3";

function MenuSection({ eyebrow, title, description, children }) {
  return (
    <section className={sectionClass}>
      <div className="grid gap-1">
        <span className="text-[10px] uppercase tracking-[0.24em] text-[#7d97b4]">
          {eyebrow}
        </span>
        <div className="text-[16px] font-bold uppercase tracking-[0.16em] text-foreground">
          {title}
        </div>
        {description ? (
          <p className="m-0 text-[13px] leading-5 text-muted-foreground">{description}</p>
        ) : null}
      </div>
      {children}
    </section>
  );
}

export default function TopbarMenuSheet({
  playerNames,
  setPlayerNames,
  startingLife,
  setStartingLife,
  onReset,
  onChangePerspective,
  onRefresh,
  onEnterDeckLoading,
  onOpenLobby,
  deckLoadingMode,
}) {
  const [open, setOpen] = useState(false);
  const {
    state,
    wasmRegistryCount,
    wasmRegistryTotal,
    multiplayer,
  } = useGame();

  const players = state?.players || [];
  const perspective = state?.perspective;
  const me = players.find((player) => player.id === perspective) || players[0];
  const lobbyBusy = multiplayer.mode !== "idle";
  const matchLocked = multiplayer.matchStarted;
  const compiledLabel = useMemo(() => {
    if (!Number.isFinite(wasmRegistryCount) || wasmRegistryCount < 0) return "-";
    if (wasmRegistryTotal > 0) {
      return `${wasmRegistryCount.toLocaleString()}/${wasmRegistryTotal.toLocaleString()}`;
    }
    return wasmRegistryCount.toLocaleString();
  }, [wasmRegistryCount, wasmRegistryTotal]);
  const lobbyLabel = lobbyBusy
    ? `Lobby ${multiplayer.players.length}/${multiplayer.desiredPlayers || 0}`
    : "No lobby";

  const handleOpenLobby = () => {
    setOpen(false);
    onOpenLobby();
  };

  const handleToggleDeckLoading = () => {
    setOpen(false);
    onEnterDeckLoading();
  };

  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetTrigger asChild>
        <Button
          variant="secondary"
          size="icon-xs"
          className="rounded-none text-muted-foreground hover:text-foreground"
          aria-label="Open game menu"
        >
          <Settings2 className="size-3.5" />
        </Button>
      </SheetTrigger>
      <SheetContent
        side="right"
        className="w-[min(92vw,420px)] overflow-y-auto border-game-line bg-[#0b1017] p-0"
      >
        <SheetHeader className="border-b border-game-line bg-[linear-gradient(180deg,rgba(18,28,41,0.98)_0%,rgba(11,16,23,0.98)_100%)] pr-12">
          <div className="text-[11px] uppercase tracking-[0.24em] text-[#7d97b4]">Menu</div>
          <SheetTitle className="text-[22px] uppercase tracking-[0.18em] text-foreground">
            Table Settings
          </SheetTitle>
          <SheetDescription className="max-w-[32ch] text-[13px] leading-5">
            Match setup, diagnostics, and secondary info live here so the top bar can stay
            focused on gameplay.
          </SheetDescription>
        </SheetHeader>

        <div className="grid gap-4 p-4">
          <MenuSection
            eyebrow="Match"
            title="Setup"
            description="Pre-game configuration and table management. These controls stay out of the way once the match is underway."
          >
            <div className="grid gap-3 sm:grid-cols-2">
              <label className={labelClass}>
                Player Names
                <input
                  className={inputClass}
                  value={playerNames}
                  disabled={lobbyBusy}
                  onChange={(event) => setPlayerNames(event.target.value)}
                />
              </label>
              <label className={labelClass}>
                Starting Life
                <input
                  className={inputClass}
                  type="number"
                  min={1}
                  value={startingLife}
                  disabled={lobbyBusy}
                  onChange={(event) => setStartingLife(Number(event.target.value) || 20)}
                />
              </label>
            </div>
            <div className="grid gap-2 sm:grid-cols-2">
              <Button
                variant="secondary"
                size="sm"
                disabled={lobbyBusy}
                onClick={onReset}
              >
                Reset Match
              </Button>
              <Button
                variant="secondary"
                size="sm"
                disabled={lobbyBusy}
                onClick={handleToggleDeckLoading}
              >
                {deckLoadingMode ? "Cancel Deck Load" : "Load Decks"}
              </Button>
              <Button variant="secondary" size="sm" onClick={handleOpenLobby}>
                {lobbyBusy ? "Open Lobby" : "Create Lobby"}
              </Button>
              <Button variant="secondary" size="sm" onClick={onRefresh}>
                <RefreshCw className="size-3.5" />
                Refresh View
              </Button>
            </div>
          </MenuSection>

          <MenuSection
            eyebrow="View"
            title="Perspective"
            description="UI-side controls that affect what you are inspecting, not the game rules themselves."
          >
            <label className={labelClass}>
              Playing As
              <select
                className={inputClass}
                value={perspective ?? ""}
                disabled={matchLocked}
                onChange={(event) => onChangePerspective(Number(event.target.value))}
              >
                {players.map((player) => (
                  <option key={player.id} value={player.id}>
                    {player.name}
                  </option>
                ))}
              </select>
            </label>
          </MenuSection>

          <MenuSection
            eyebrow="Info"
            title="Session"
            description="Reference info that does not need to sit in the gameplay lane."
          >
            <div className="grid gap-2 text-[13px] text-foreground">
              <div className="flex items-center justify-between gap-3 rounded-md border border-game-line-2 bg-[#0a1118] px-3 py-2">
                <span className="uppercase tracking-[0.16em] text-muted-foreground">View</span>
                <Badge variant="secondary" className="text-[12px] uppercase">
                  {me?.name || "-"}
                </Badge>
              </div>
              <div className="flex items-center justify-between gap-3 rounded-md border border-game-line-2 bg-[#0a1118] px-3 py-2">
                <span className="uppercase tracking-[0.16em] text-muted-foreground">
                  Cards Compiled
                </span>
                <Badge variant="secondary" className="text-[12px] uppercase">
                  {compiledLabel}
                </Badge>
              </div>
              <div className="flex items-center justify-between gap-3 rounded-md border border-game-line-2 bg-[#0a1118] px-3 py-2">
                <span className="uppercase tracking-[0.16em] text-muted-foreground">Lobby</span>
                <Badge variant="secondary" className="text-[12px] uppercase">
                  {lobbyLabel}
                </Badge>
              </div>
            </div>
            <Button variant="secondary" size="sm" asChild>
              <a
                href="https://github.com/Chiplis/ironsmith"
                target="_blank"
                rel="noopener noreferrer"
              >
                <Github className="size-3.5" />
                Repository
                <ExternalLink className="size-3" />
              </a>
            </Button>
          </MenuSection>
        </div>
      </SheetContent>
    </Sheet>
  );
}
