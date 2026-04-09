import { useEffect, useCallback, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { parseNames } from "@/lib/constants";
import { UI_NOTICE_EVENT } from "@/lib/ui-notices";
import { decodeBase64UrlUtf8, normalizePuzzlePayload, PUZZLE_ZONE_ORDER } from "@/lib/puzzles";
import { MATCH_FORMAT_COMMANDER, MATCH_FORMAT_NORMAL } from "@/lib/decklists";
import useViewportLayout from "@/hooks/useViewportLayout";
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
    runWasmInteraction,
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
  const [puzzleSetupMode, setPuzzleSetupMode] = useState(false);
  const [mobileOpponentIndex, setMobileOpponentIndex] = useState(0);
  const [notices, setNotices] = useState([]);
  const { landscapeMobileViewport, tabletCompactViewport, smallDesktopViewport } = useViewportLayout();
  const nextNoticeIdRef = useRef(1);
  const autoJoinAttemptedLobbyRef = useRef("");
  const autoLoadAttemptedPuzzleRef = useRef(false);
  const initialLobbyQueryRef = useRef(readLobbyQueryParams());
  const initialPuzzleQueryRef = useRef(readPuzzleQueryParams());
  const [lobbyOverlayInitial, setLobbyOverlayInitial] = useState(() => (
    buildLobbyOverlayInitialState(initialLobbyQueryRef.current)
  ));
  const borderlessPreview = (
    typeof window !== "undefined"
    && (
      new URLSearchParams(window.location.search).get("borderless") === "1"
      || window.localStorage.getItem("ironsmith-borderless-preview") === "1"
    )
  );

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
    const players = state?.players || [];
    const perspective = state?.perspective;
    const me = players.find((player) => player.id === perspective) || players[0];
    const meIndex = players.findIndex((player) => player.id === me?.id);
    const ordered = meIndex >= 0
      ? [...players.slice(meIndex), ...players.slice(0, meIndex)]
      : players;
    const opponentCount = ordered.filter((player) => player.id !== me?.id).length;
    setMobileOpponentIndex((currentIndex) => {
      if (opponentCount <= 1) return 0;
      return Math.min(currentIndex, opponentCount - 1);
    });
  }, [state?.players, state?.perspective]);

  useEffect(() => {
    const handleFocusPlayerTarget = (event) => {
      const targetPlayer = Number(event?.detail?.player);
      if (!Number.isFinite(targetPlayer)) return;

      const players = state?.players || [];
      const perspective = state?.perspective;
      const me = players.find((player) => player.id === perspective) || players[0];
      const meIndex = players.findIndex((player) => player.id === me?.id);
      const ordered = meIndex >= 0
        ? [...players.slice(meIndex), ...players.slice(0, meIndex)]
        : players;
      const opponents = ordered.filter((player) => player.id !== me?.id);
      const nextIndex = opponents.findIndex((player) => (
        Number(player.id) === targetPlayer || Number(player.index) === targetPlayer
      ));
      if (nextIndex < 0) return;
      setMobileOpponentIndex(nextIndex);
    };

    window.addEventListener("ironsmith:focus-player-target", handleFocusPlayerTarget);
    return () => {
      window.removeEventListener("ironsmith:focus-player-target", handleFocusPlayerTarget);
    };
  }, [state?.players, state?.perspective]);

  useEffect(() => {
    if (loading || wasmError || !state || multiplayer.mode !== "idle") return;
    if (initialPuzzleQueryRef.current) return;

    const queryLobby = initialLobbyQueryRef.current;
    const lobbyCode = queryLobby.lobbyId;
    if (!lobbyCode || autoJoinAttemptedLobbyRef.current === lobbyCode) return;

    autoJoinAttemptedLobbyRef.current = lobbyCode;
    setLobbyOverlayInitial(buildLobbyOverlayInitialState(queryLobby, "join"));
    setLobbyOpen(true);
    joinLobby({
      name: queryLobby.name || parseNames(playerNames)[0] || "Player",
      lobbyId: lobbyCode,
      deckText: queryLobby.deckText,
      commanderText: queryLobby.commanderText,
      onUnavailable: () => {
        const createPrefill = {
          ...queryLobby,
          lobbyId: "",
        };
        initialLobbyQueryRef.current = createPrefill;
        stripLobbyCodeFromUrl();
        setLobbyOverlayInitial(buildLobbyOverlayInitialState(createPrefill, "create"));
        setLobbyOpen(true);
      },
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
        if (!initialPuzzleQueryRef.current) {
          await addStartingBattlefieldPreset(game);
        }
        await refresh("WASM loaded");
      } catch (err) {
        setStatus(`Init failed: ${err}`, true);
      }
    }

    init();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [game]);

  const loadPuzzle = useCallback(
    async (payload, successMessage = "Puzzle loaded") => {
      if (!game) return false;
      if (multiplayer.mode !== "idle") {
        setStatus("Puzzles are disabled while a lobby is active", true);
        return false;
      }

      const normalized = normalizePuzzlePayload(payload);
      if (!normalized) {
        setStatus("Puzzle payload is invalid", true);
        return false;
      }

      const playerNamesList = normalized.players.map((player, index) => (
        String(player?.name || "").trim() || `Player ${index + 1}`
      ));
      const defaultStartingLife = Number(normalized.players[0]?.life) || 20;

      try {
        await game.reset(playerNamesList, defaultStartingLife);
        for (const [playerIndex, player] of normalized.players.entries()) {
          if (typeof game.setLife === "function") {
            await game.setLife(playerIndex, Number(player?.life) || 20);
          }
          for (const zone of PUZZLE_ZONE_ORDER) {
            for (const cardName of player.zones?.[zone] || []) {
              await game.addCardToZone(playerIndex, cardName, zone, true);
            }
          }
        }
        setPlayerNames(playerNamesList.join(","));
        setStartingLife(defaultStartingLife);
        setDeckLoadingMode(false);
        setPuzzleSetupMode(false);
        setLobbyOpen(false);
        await refresh(successMessage);
        return true;
      } catch (err) {
        setStatus(`Load puzzle failed: ${err}`, true);
        return false;
      }
    },
    [game, multiplayer.mode, refresh, setStatus]
  );

  useEffect(() => {
    if (loading || wasmError || !state || multiplayer.mode !== "idle") return;
    if (autoLoadAttemptedPuzzleRef.current) return;
    const initialPuzzle = initialPuzzleQueryRef.current;
    if (!initialPuzzle) return;

    autoLoadAttemptedPuzzleRef.current = true;
    void loadPuzzle(initialPuzzle, "Puzzle loaded from link");
  }, [loadPuzzle, loading, multiplayer.mode, state, wasmError]);

  const handleReset = useCallback(async () => {
    return runWasmInteraction(async () => {
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
    });
  }, [game, multiplayer.mode, playerNames, refresh, runWasmInteraction, setStatus, startingLife]);

  const handleLoadCustomDecks = useCallback(async (decks) => {
    return runWasmInteraction(async () => {
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
    });
  }, [game, multiplayer.mode, pushNotice, refresh, runWasmInteraction, setStatus]);

  const handleChangePerspective = useCallback(
    async (playerIndex) => {
      return runWasmInteraction(async () => {
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
      });
    },
    [game, multiplayer.matchStarted, refresh, runWasmInteraction, setStatus]
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
    <div
      className={
        landscapeMobileViewport
          ? "app-shell mobile-app-shell relative w-full h-[100dvh] overflow-hidden"
          : "app-shell w-full h-[100dvh] p-2 grid grid-rows-[auto_auto_minmax(0,1fr)] gap-2"
      }
      data-borderless-preview={borderlessPreview ? "true" : "false"}
      data-mobile-overlay-shell={landscapeMobileViewport ? "true" : "false"}
    >
      <Topbar
        playerNames={playerNames}
        setPlayerNames={setPlayerNames}
        startingLife={startingLife}
        setStartingLife={setStartingLife}
        onReset={handleReset}
        onChangePerspective={handleChangePerspective}
        onRefresh={() => void runWasmInteraction(() => refresh("Refreshed"))}
        onToggleLog={() => setLogOpen((o) => !o)}
        onEnterDeckLoading={() => {
          setPuzzleSetupMode(false);
          setDeckLoadingMode((mode) => !mode);
        }}
        onOpenPuzzleSetup={() => {
          setDeckLoadingMode(false);
          setPuzzleSetupMode((mode) => !mode);
        }}
        puzzleSetupMode={puzzleSetupMode}
        onOpenLobby={() => {
          setDeckLoadingMode(false);
          setPuzzleSetupMode(false);
          setLobbyOpen(true);
        }}
        deckLoadingMode={deckLoadingMode}
        onAddCardNotice={pushNotice}
        mobileOpponentIndex={mobileOpponentIndex}
        setMobileOpponentIndex={setMobileOpponentIndex}
        mobileOverlay={landscapeMobileViewport}
      />
      {!landscapeMobileViewport && !tabletCompactViewport ? (
        <AddCardBar
          compact={smallDesktopViewport}
          zoneViews={zoneViews}
          setZoneViews={setZoneViews}
          onAddCardNotice={pushNotice}
          onEnterDeckLoading={() => {
            setPuzzleSetupMode(false);
            setDeckLoadingMode((mode) => !mode);
          }}
          onOpenPuzzleSetup={() => {
            setDeckLoadingMode(false);
            setPuzzleSetupMode((mode) => !mode);
          }}
          onOpenLobby={() => {
            setDeckLoadingMode(false);
            setPuzzleSetupMode(false);
            setLobbyOpen(true);
          }}
          deckLoadingMode={deckLoadingMode}
          puzzleSetupMode={puzzleSetupMode}
        />
      ) : null}
      <Workspace
        zoneViews={zoneViews}
        deckLoadingMode={deckLoadingMode}
        puzzleSetupMode={puzzleSetupMode}
        onLoadDecks={handleLoadCustomDecks}
        onCancelDeckLoading={() => setDeckLoadingMode(false)}
        onLoadPuzzle={(payload, successMessage) => runWasmInteraction(
          () => loadPuzzle(payload, successMessage)
        )}
        onCancelPuzzleSetup={() => setPuzzleSetupMode(false)}
        notices={notices}
        onDismissNotice={dismissNotice}
        mobileOpponentIndex={mobileOpponentIndex}
        setMobileOpponentIndex={setMobileOpponentIndex}
      />
      <LogDrawer open={logOpen} onOpenChange={setLogOpen} />
      {lobbyOpen ? (
        <LobbyOverlay
          key={lobbyOverlayInitial.mode}
          onClose={() => setLobbyOpen(false)}
          defaultName={parseNames(playerNames)[0] || "Player"}
          defaultStartingLife={startingLife}
          initialMode={lobbyOverlayInitial.mode}
          initialCreateFormat={lobbyOverlayInitial.createFormat}
          initialCreateName={lobbyOverlayInitial.createName}
          initialCreateDeckText={lobbyOverlayInitial.createDeckText}
          initialCreateCommanderText={lobbyOverlayInitial.createCommanderText}
          initialJoinCode={lobbyOverlayInitial.joinCode}
          initialJoinName={lobbyOverlayInitial.joinName}
          initialJoinDeckText={lobbyOverlayInitial.joinDeckText}
          initialJoinCommanderText={lobbyOverlayInitial.joinCommanderText}
        />
      ) : null}
    </div>
  );
}

async function addStartingBattlefieldPreset(game) {
  const openingBattlefield = [
    "Omniscience",
    "Forest",
    "Plains",
    "Island",
    "Mountain",
    "Swamp",
    "Tropical Island",
    "Volcanic Island",
    "Yawgmoth, Thran Physician",
    "Ornithopter",
    "Myr Moonvessel",
  ];

  for (const playerIndex of [0, 1]) {
    for (const cardName of openingBattlefield) {
      try {
        await game.addCardToZone(playerIndex, cardName, "battlefield", true);
      } catch (err) {
        console.warn(`Skipping startup battlefield card "${cardName}":`, err);
      }
    }
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
  return {
    lobbyId,
    name: String(params.get("name") || "").trim(),
    deckText: decodeBase64UrlUtf8(params.get("deck")),
    commanderText: decodeBase64UrlUtf8(params.get("commander")),
  };
}

function stripLobbyCodeFromUrl() {
  if (typeof window === "undefined") return;
  const url = new URL(window.location.href);
  url.searchParams.delete("lobby");
  window.history.replaceState({}, "", url.toString());
}

function inferCreateFormatFromLobbyQuery(query) {
  return String(query?.commanderText || "").trim()
    ? MATCH_FORMAT_COMMANDER
    : MATCH_FORMAT_NORMAL;
}

function buildLobbyOverlayInitialState(query, mode = null) {
  const nextMode = mode || (query?.lobbyId ? "join" : "create");
  return {
    mode: nextMode === "join" ? "join" : "create",
    createFormat: inferCreateFormatFromLobbyQuery(query),
    createName: String(query?.name || "").trim(),
    createDeckText: String(query?.deckText || ""),
    createCommanderText: String(query?.commanderText || ""),
    joinCode: String(query?.lobbyId || "").trim(),
    joinName: String(query?.name || "").trim(),
    joinDeckText: String(query?.deckText || ""),
    joinCommanderText: String(query?.commanderText || ""),
  };
}

function readPuzzleQueryParams() {
  if (typeof window === "undefined") return null;

  const params = new URLSearchParams(window.location.search);
  const rawPuzzle = String(params.get("puzzle") || "").trim();
  if (!rawPuzzle) return null;

  const decoded = decodeBase64UrlUtf8(rawPuzzle);
  if (!decoded) return null;

  try {
    return normalizePuzzlePayload(JSON.parse(decoded));
  } catch {
    return null;
  }
}
