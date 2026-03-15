import { useEffect, useCallback, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { parseNames } from "@/lib/constants";
import { UI_NOTICE_EVENT } from "@/lib/ui-notices";
import Topbar from "./Topbar";
import LobbyOverlay from "./LobbyOverlay";
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
    multiplayer,
    semanticThreshold,
    joinLobby,
  } = useGame();
  const [playerNames, setPlayerNames] = useState("Alice,Bob,Charlie,Diana");
  const [startingLife, setStartingLife] = useState(20);
  const [logOpen, setLogOpen] = useState(false);
  const [lobbyOpen, setLobbyOpen] = useState(false);
  const [zoneViews, setZoneViews] = useState(["battlefield"]);
  const [deckLoadingMode, setDeckLoadingMode] = useState(false);
  const [notices, setNotices] = useState([]);
  const nextNoticeIdRef = useRef(1);
  const autoJoinAttemptedLobbyRef = useRef("");
  const initialLobbyQueryRef = useRef(readLobbyQueryParams());

  const pushNotice = useCallback((notice) => {
    const id = nextNoticeIdRef.current++;
    setNotices((current) => [...current, { id, ...notice }].slice(-6));
    return id;
  }, []);

  const dismissNotice = useCallback((noticeId) => {
    setNotices((current) => current.filter((notice) => notice.id !== noticeId));
  }, []);

  useEffect(() => {
    const handleUiNotice = (event) => {
      const detail = event?.detail;
      if (!detail || typeof detail !== "object") return;
      pushNotice(detail);
    };

    window.addEventListener(UI_NOTICE_EVENT, handleUiNotice);
    return () => {
      window.removeEventListener(UI_NOTICE_EVENT, handleUiNotice);
    };
  }, [pushNotice]);

  useEffect(() => {
    if (multiplayer.matchStarted) {
      setLobbyOpen(false);
      setDeckLoadingMode(false);
    }
  }, [multiplayer.matchStarted]);

  useEffect(() => {
    if (loading || wasmError || !state || multiplayer.mode !== "idle") return;

    const queryLobby = initialLobbyQueryRef.current;
    const lobbyCode = queryLobby.lobbyId;
    if (!lobbyCode || autoJoinAttemptedLobbyRef.current === lobbyCode) return;

    autoJoinAttemptedLobbyRef.current = lobbyCode;
    setLobbyOpen(true);
    joinLobby({
      name: queryLobby.name || parseNames(playerNames)[0] || "Player",
      lobbyId: lobbyCode,
      deckText: queryLobby.deckText,
      commanderText: queryLobby.commanderText,
    });
  }, [joinLobby, loading, multiplayer.mode, playerNames, state, wasmError]);

  // Initialize game when WASM loads
  useEffect(() => {
    if (!game) return;
    async function init() {
      try {
        if (typeof game.setSemanticThreshold === "function") {
          await game.setSemanticThreshold(semanticThreshold);
        }
        await game.reset(parseNames(playerNames), startingLife);
        await addStartingBattlefieldPreset(game);
        await refresh("WASM loaded");
      } catch (err) {
        setStatus(`Init failed: ${err}`, true);
      }
    }

    init();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [game]);

  const handleReset = useCallback(async () => {
    if (!game) return;
    if (multiplayer.mode !== "idle") {
      setStatus("Reset is disabled while a lobby is active", true);
      return;
    }
    try {
      await game.reset(parseNames(playerNames), startingLife);
      await addStartingBattlefieldPreset(game);
      setDeckLoadingMode(false);
      await refresh("Game reset");
    } catch (err) {
      setStatus(`Reset failed: ${err}`, true);
    }
  }, [game, multiplayer.mode, playerNames, startingLife, refresh, setStatus]);

  const handleLoadCustomDecks = useCallback(async (decks) => {
    if (!game) return;
    if (multiplayer.mode !== "idle") {
      setStatus("Deck loading is disabled while a lobby is active", true);
      return;
    }
    try {
      const result = await game.loadDecks(decks);
      setDeckLoadingMode(false);
      const loaded = result?.loaded ?? 0;
      const failed = Array.isArray(result?.failed) ? result.failed : [];
      const failedBelowThreshold = Array.isArray(result?.failedBelowThreshold)
        ? result.failedBelowThreshold
        : [];
      const failedToParse = Array.isArray(result?.failedToParse)
        ? result.failedToParse
        : [];
      pushNotice({
        tone: "success",
        title: "Deck load complete",
        body: `Loaded ${loaded} card${loaded === 1 ? "" : "s"}.`,
      });
      if (failed.length > 0) {
        const copyActions = [
          {
            label: `Copy all (${failed.length})`,
            copyText: failed.join("\n"),
            copyStatusMessage: `Copied ${failed.length} failed deck card name${failed.length === 1 ? "" : "s"}`,
          },
        ];
        if (failedBelowThreshold.length > 0) {
          copyActions.push({
            label: `Copy threshold (${failedBelowThreshold.length})`,
            copyText: failedBelowThreshold.join("\n"),
            copyStatusMessage: `Copied ${failedBelowThreshold.length} low-fidelity deck card name${failedBelowThreshold.length === 1 ? "" : "s"}`,
          });
        }
        if (failedToParse.length > 0) {
          copyActions.push({
            label: `Copy parse (${failedToParse.length})`,
            copyText: failedToParse.join("\n"),
            copyStatusMessage: `Copied ${failedToParse.length} unparsed deck card name${failedToParse.length === 1 ? "" : "s"}`,
          });
        }
        const issueSummary = [
          failedBelowThreshold.length > 0
            ? `${failedBelowThreshold.length} below threshold`
            : null,
          failedToParse.length > 0 ? `${failedToParse.length} failed to parse` : null,
        ]
          .filter(Boolean)
          .join(". ");
        pushNotice({
          tone: "error",
          title: "Deck load issues",
          body: `${failed.length} card${failed.length === 1 ? "" : "s"} failed. ${issueSummary ? `${issueSummary}. ` : ""}Use the copy actions below.`,
          actions: copyActions,
        });
      }
      if (failed.length > 0) {
        const unique = [...new Set(failed)];
        const failedStr = unique.length <= 5
          ? unique.join(", ")
          : `${unique.slice(0, 5).join(", ")} (+${unique.length - 5} more)`;
        const issueSummary = [
          failedBelowThreshold.length > 0
            ? `${failedBelowThreshold.length} below threshold`
            : null,
          failedToParse.length > 0 ? `${failedToParse.length} failed to parse` : null,
        ]
          .filter(Boolean)
          .join(", ");
        await refresh(
          `Loaded ${loaded} cards. ${failed.length} failed${issueSummary ? ` (${issueSummary})` : ""}: ${failedStr}`
        );
      } else {
        await refresh(`Loaded ${loaded} cards`);
      }
    } catch (err) {
      setStatus(`Load decks failed: ${err}`, true);
    }
  }, [game, multiplayer.mode, pushNotice, refresh, setStatus]);

  const handleChangePerspective = useCallback(
    async (playerIndex) => {
      if (!game) return;
      if (multiplayer.matchStarted) {
        setStatus("Perspective is fixed during multiplayer matches", true);
        return;
      }
      try {
        await game.setPerspective(playerIndex);
        await refresh(`Viewing as player ${playerIndex}`);
      } catch (err) {
        setStatus(`Change player failed: ${err}`, true);
      }
    },
    [game, multiplayer.matchStarted, refresh, setStatus]
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
        onOpenLobby={() => setLobbyOpen(true)}
        deckLoadingMode={deckLoadingMode}
      />
      <AddCardBar
        zoneViews={zoneViews}
        setZoneViews={setZoneViews}
        onAddCardFailure={pushNotice}
      />
      <Workspace
        zoneViews={zoneViews}
        deckLoadingMode={deckLoadingMode}
        onLoadDecks={handleLoadCustomDecks}
        onCancelDeckLoading={() => setDeckLoadingMode(false)}
        notices={notices}
        onDismissNotice={dismissNotice}
      />
      <LogDrawer open={logOpen} onOpenChange={setLogOpen} />
      {lobbyOpen ? (
        <LobbyOverlay
          onClose={() => setLobbyOpen(false)}
          defaultName={parseNames(playerNames)[0] || "Player"}
          defaultStartingLife={startingLife}
          initialMode={initialLobbyQueryRef.current.lobbyId ? "join" : "create"}
          initialJoinCode={initialLobbyQueryRef.current.lobbyId}
          initialJoinName={initialLobbyQueryRef.current.name}
          initialJoinDeckText={initialLobbyQueryRef.current.deckText}
          initialJoinCommanderText={initialLobbyQueryRef.current.commanderText}
        />
      ) : null}
    </div>
  );
}

async function addStartingBattlefieldPreset(game) {
  await game.addCardToZone(0, "Omniscience", "battlefield", true);
}

function decodeBase64Utf8(raw) {
  if (!raw || typeof window === "undefined") return "";

  try {
    const normalized = String(raw)
      .trim()
      .replace(/-/g, "+")
      .replace(/_/g, "/");
    const padding = normalized.length % 4;
    const padded = padding === 0 ? normalized : `${normalized}${"=".repeat(4 - padding)}`;
    const binary = window.atob(padded);
    const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
    return new TextDecoder().decode(bytes);
  } catch {
    return "";
  }
}

function readLobbyQueryParams() {
  if (typeof window === "undefined") {
    return {
      lobbyId: "",
      name: "",
      deckText: "",
      commanderText: "",
    };
  }

  const params = new URLSearchParams(window.location.search);
  const lobbyId = String(params.get("lobby") || "").trim();
  if (!lobbyId) {
    return {
      lobbyId: "",
      name: "",
      deckText: "",
      commanderText: "",
    };
  }

  return {
    lobbyId,
    name: String(params.get("name") || "").trim(),
    deckText: decodeBase64Utf8(params.get("deck")),
    commanderText: decodeBase64Utf8(params.get("commander")),
  };
}
