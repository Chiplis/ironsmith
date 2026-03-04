import { useEffect, useCallback, useState } from "react";
import { useGame } from "@/context/GameContext";
import { parseNames } from "@/lib/constants";
import Topbar from "./Topbar";
import AddCardBar from "./AddCardBar";
import Workspace from "./Workspace";
import LogDrawer from "@/components/overlays/LogDrawer";

export default function Shell() {
  const {
    game,
    state,
    loading,
    wasmError,
    wasmProgress,
    wasmPhase,
    wasmRegistryCount,
    refresh,
    setStatus,
    setState,
  } = useGame();
  const [playerNames, setPlayerNames] = useState("Alice,Bob,Charlie,Diana");
  const [startingLife, setStartingLife] = useState(20);
  const [logOpen, setLogOpen] = useState(false);
  const [zoneView, setZoneView] = useState("battlefield");
  const [deckLoadingMode, setDeckLoadingMode] = useState(false);

  // Initialize game when WASM loads
  useEffect(() => {
    if (!game) return;
    async function init() {
      try {
        await game.reset(parseNames(playerNames), startingLife);
        await refresh("WASM loaded");
        // Run demo setup: advance to main phase + 3 Black Lotuses
        await setupDemoBoard();
      } catch (err) {
        setStatus(`Init failed: ${err}`, true);
      }
    }

    // Settle all opponent priority/combat decisions so game advances
    async function settle(g, st) {
      for (let i = 0; i < 80 && st?.decision && st.decision.player !== st.perspective; i++) {
        if (st.decision.kind === "priority") {
          const pass = (st.decision.actions || []).find((a) => a.kind === "pass_priority");
          if (!pass) break;
          st = await g.dispatch({ type: "priority_action", action_index: pass.index });
        } else if (st.decision.kind === "attackers") {
          st = await g.dispatch({ type: "declare_attackers", declarations: [] });
        } else if (st.decision.kind === "blockers") {
          st = await g.dispatch({ type: "declare_blockers", declarations: [] });
        } else {
          break;
        }
      }
      return st;
    }

    async function setupDemoBoard() {
      try {
        let st = await game.uiState();
        const playerIndex = Number(st.active_player);

        // Pass priority twice (with opponent settle) to advance to main phase
        for (let i = 0; i < 2; i++) {
          if (!st || !st.decision) break;
          const passAction = (st.decision.actions || []).find((a) => a.kind === "pass_priority");
          if (!passAction) break;
          st = await game.dispatch({ type: "priority_action", action_index: passAction.index });
          st = await settle(game, st);
        }

        // Add 3 Black Lotuses and cast each one
        for (let i = 0; i < 3; i++) {
          const objectId = Number(
            await game.addCardToZone(playerIndex, "Black Lotus", "hand", true)
          );
          st = await game.uiState();
          st = await settle(game, st);
          if (st.decision && st.decision.actions) {
            const castAction = st.decision.actions.find(
              (a) => a.kind === "cast_spell" && Number(a.object_id) === objectId
            );
            if (castAction) {
              st = await game.dispatch({ type: "priority_action", action_index: castAction.index });
              st = await settle(game, st);
              if (st && st.decision) {
                const passAction = (st.decision.actions || []).find((a) => a.kind === "pass_priority");
                if (passAction) {
                  st = await game.dispatch({ type: "priority_action", action_index: passAction.index });
                  st = await settle(game, st);
                }
              }
            }
          }
        }
        // Add Anger to graveyard and Mountain to battlefield
        await game.addCardToZone(playerIndex, "Anger", "graveyard", true);
        await game.addCardToZone(playerIndex, "Mountain", "battlefield", true);

        await refresh("Demo: 3 Black Lotuses + Mountain on battlefield, Anger in graveyard");
      } catch (err) {
        console.error("setupDemoBoard failed:", err);
      }
    }

    init();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [game]);

  const handleReset = useCallback(async () => {
    if (!game) return;
    try {
      await game.reset(parseNames(playerNames), startingLife);
      setDeckLoadingMode(false);
      await refresh("Game reset");
    } catch (err) {
      setStatus(`Reset failed: ${err}`, true);
    }
  }, [game, playerNames, startingLife, refresh, setStatus]);

  const handleLoadDemoDecks = useCallback(async () => {
    if (!game) return;
    try {
      await game.loadDemoDecks();
      setDeckLoadingMode(false);
      await refresh("Demo decks loaded");
    } catch (err) {
      setStatus(`Load decks failed: ${err}`, true);
    }
  }, [game, refresh, setStatus]);

  const handleLoadCustomDecks = useCallback(async (decks) => {
    if (!game) return;
    try {
      const result = await game.loadDecks(decks);
      setDeckLoadingMode(false);
      const loaded = result?.loaded ?? 0;
      const failed = result?.failed || [];
      if (failed.length > 0) {
        const unique = [...new Set(failed)];
        const failedStr = unique.length <= 5
          ? unique.join(", ")
          : `${unique.slice(0, 5).join(", ")} (+${unique.length - 5} more)`;
        await refresh(`Loaded ${loaded} cards. ${failed.length} failed: ${failedStr}`);
      } else {
        await refresh(`Loaded ${loaded} cards`);
      }
    } catch (err) {
      setStatus(`Load decks failed: ${err}`, true);
    }
  }, [game, refresh, setStatus]);

  const handleChangePerspective = useCallback(
    async (playerIndex) => {
      if (!game) return;
      try {
        await game.setPerspective(playerIndex);
        await refresh(`Viewing as player ${playerIndex}`);
      } catch (err) {
        setStatus(`Change player failed: ${err}`, true);
      }
    },
    [game, refresh, setStatus]
  );

  if (loading) {
    const widthPct = Math.max(0, Math.min(100, wasmProgress * 100));
    // Avoid showing 100% until progress has actually completed.
    const pct = wasmProgress >= 1 ? 100 : Math.floor(widthPct);
    const phaseLabel =
      wasmPhase === "module" ? "Loading module..." :
      wasmPhase === "download" ? "Downloading WASM..." :
      wasmPhase === "registry" ? `Compiled ${Number(wasmRegistryCount || 0).toLocaleString()} cards...` :
      "Initializing";
    return (
      <div className="flex flex-col items-center justify-center h-screen gap-3 text-muted-foreground">
        {wasmPhase === "init" ? (
          <span className="text-[18px] font-bold uppercase tracking-wider">
            {phaseLabel}
            <span className="loading-dots" aria-hidden="true">
              <span className="loading-dot loading-dot-1">.</span>
              <span className="loading-dot loading-dot-2">.</span>
              <span className="loading-dot loading-dot-3">.</span>
            </span>
          </span>
        ) : (
          <span className="text-[18px] font-bold uppercase tracking-wider">{phaseLabel}</span>
        )}
        <div className="w-64 h-2 bg-[#1a2433] border border-game-line rounded-sm overflow-hidden">
          <div
            className="h-full bg-primary"
            style={{ width: `${widthPct}%` }}
          />
        </div>
        <span className="text-[16px]">{pct}%</span>
      </div>
    );
  }

  if (wasmError) {
    return (
      <div className="flex items-center justify-center h-screen text-destructive">
        WASM failed: {wasmError.message}
      </div>
    );
  }

  // Worker can be ready before initial reset/demo-setup has produced first UI state.
  if (!state) {
    return (
      <div className="flex flex-col items-center justify-center h-screen gap-3 text-muted-foreground">
        <span className="text-[18px] font-bold uppercase tracking-wider">
          Preparing Game
          <span className="loading-dots" aria-hidden="true">
            <span className="loading-dot loading-dot-1">.</span>
            <span className="loading-dot loading-dot-2">.</span>
            <span className="loading-dot loading-dot-3">.</span>
          </span>
        </span>
      </div>
    );
  }

  return (
    <div className="w-full h-screen p-2 grid grid-rows-[auto_auto_minmax(0,1fr)] gap-2">
      <Topbar
        playerNames={playerNames}
        setPlayerNames={setPlayerNames}
        startingLife={startingLife}
        setStartingLife={setStartingLife}
        onReset={handleReset}
        onChangePerspective={handleChangePerspective}
        onRefresh={() => refresh("Refreshed")}
        onToggleLog={() => setLogOpen((o) => !o)}
        onEnterDeckLoading={() => setDeckLoadingMode((m) => !m)}
        deckLoadingMode={deckLoadingMode}
      />
      <AddCardBar zoneView={zoneView} setZoneView={setZoneView} />
      <Workspace
        zoneView={zoneView}
        deckLoadingMode={deckLoadingMode}
        onLoadDecks={handleLoadCustomDecks}
        onCancelDeckLoading={() => setDeckLoadingMode(false)}
      />
      <LogDrawer open={logOpen} onOpenChange={setLogOpen} />
    </div>
  );
}
