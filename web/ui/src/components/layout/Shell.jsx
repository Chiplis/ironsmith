import { recordPlayedDeck } from "@/lib/recent-decks";
import useUiText from "@/i18n/useUiText";
import { useEffect, useCallback, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { parseNames } from "@/lib/constants";
import { installMainDecisionShortcut } from "@/lib/main-decision-shortcut";
import { installHandToggleShortcut } from "@/lib/hand-open";
import { readTableSetup, storeTableSetup, tableSeatNames } from "@/lib/table-setup";
import { UI_NOTICE_EVENT } from "@/lib/ui-notices";
import { decodeBase64UrlUtf8, normalizePuzzlePayload, PUZZLE_ZONE_ORDER } from "@/lib/puzzles";
import { withSupportedCards } from "@/lib/unsupported-card-substitution";
import { describeColorNoteIssues, submissionColorNoteIssues } from "@/lib/deck-color-notes";
import { PUBLIC_FORMATS, usesCommanders } from "@/lib/relay/formats";
import {
  MATCH_FORMAT_COMMANDER,
  normalizeMatchFormat,
  readDefaultLobbyDeck,
} from "@/lib/decklists";
import {
  MULTIPLAYER_SECURITY_TRUSTED,
  normalizeMultiplayerSecurityMode,
} from "@/lib/multiplayer-security";
import useViewportLayout from "@/hooks/useViewportLayout";
import useTabAttention from "@/hooks/useTabAttention";
import Topbar from "./Topbar";
import TopbarUtilityControls from "./TopbarUtilityControls";
import LobbyOverlay from "./LobbyOverlay";
import { readStoredPlayerName } from "@/lib/player-name";
import AddCardBar from "./AddCardBar";
import DiagnosticsSheet from "./DiagnosticsSheet";
import TableActionControls from "./TableActionControls";
import Workspace from "./Workspace";
import LoadingSmith from "./LoadingSmith";
import TableErrorBoundary from "./TableErrorBoundary";
import MobileLandscapeGate from "./MobileLandscapeGate";
import LogDrawer from "@/components/overlays/LogDrawer";
import PerspectiveHandoffBanner from "@/components/overlays/PerspectiveHandoffBanner";
import KeyboardShortcutsOverlay from "@/components/overlays/KeyboardShortcutsOverlay";
import { copyTextToClipboard } from "@/lib/clipboard";
import {
  addFixedStartingBoardPreset,
  loadRandomStartingBoard,
  prefetchRandomStartingBoard,
  startingBoardKey,
} from "@/lib/starting-board";
import { markStartupFoils } from "@/lib/foil-cards";
import { installDockLabelFit } from "@/lib/dock-label-fit";
import FullCardsManaRail from "@/components/board/FullCardsManaRail";
import { syncFullCardsFlag } from "@/lib/full-cards-flag";
import useBattlefieldAppearance from "@/hooks/useBattlefieldAppearance";

export default function Shell() {
  const ui = useUiText();
  useEffect(() => installMainDecisionShortcut(document), []);
  useEffect(() => installHandToggleShortcut(document), []);
  useEffect(() => installDockLabelFit(), []);
  const [battlefieldAppearance] = useBattlefieldAppearance();
  useEffect(() => syncFullCardsFlag(battlefieldAppearance), [battlefieldAppearance]);
  const {
    game,
    state,
    loading,
    wasmError,
    wasmProgress,
    wasmPhase,
    wasmRegistryCount,
    refresh,
    changeGoldfishPlayer,
    runWasmInteraction,
    setStatus,
    multiplayer,
    semanticThreshold,
    fixedStartingBoard,
    joinLobby,
  } = useGame();
  useTabAttention();
  const [commanderStartup] = useState(() => (
    typeof window !== "undefined"
    && new URLSearchParams(window.location.search).get("commander") === "true"
  ));
  const initialPuzzleQueryRef = useRef(commanderStartup ? null : readPuzzleQueryParams());
  const syncedLobbyUrlRef = useRef("");
  const [savedTableSetup] = useState(readTableSetup);
  const [playerNames, setPlayerNames] = useState(
    () => commanderStartup ? "Alice,Bob,Charlie,Diana"
      : initialPuzzlePlayerNames(initialPuzzleQueryRef.current) || savedTableSetup.playerNames
  );

  useEffect(() => {
    if (typeof window === "undefined") return;
    const lobbyId = String(multiplayer?.lobbyId || multiplayer?.hostPeerId || "").trim();
    const active = multiplayer?.mode && multiplayer.mode !== "idle";
    const currentUrl = new URL(window.location.href);
    if (active && lobbyId) {
      if (currentUrl.searchParams.get("lobby") !== lobbyId) {
        currentUrl.searchParams.set("lobby", lobbyId);
        window.history.replaceState({}, "", currentUrl.toString());
      }
      syncedLobbyUrlRef.current = lobbyId;
      return;
    }
    if (syncedLobbyUrlRef.current && currentUrl.searchParams.get("lobby") === syncedLobbyUrlRef.current) {
      currentUrl.searchParams.delete("lobby");
      window.history.replaceState({}, "", currentUrl.toString());
    }
    syncedLobbyUrlRef.current = "";
  }, [multiplayer?.hostPeerId, multiplayer?.lobbyId, multiplayer?.mode]);
  const [startingLife, setStartingLife] = useState(
    () => commanderStartup ? 40
      : initialPuzzleStartingLife(initialPuzzleQueryRef.current) ?? savedTableSetup.startingLife
  );
  // Menu edits are the player's table setup, so they are saved; loads that
  // bring their own names (puzzles, goldfish) use the raw setters instead.
  const editPlayerNames = useCallback((value) => {
    setPlayerNames(value);
    storeTableSetup({ playerNames: value, startingLife });
  }, [startingLife]);
  const editStartingLife = useCallback((value) => {
    setStartingLife(value);
    storeTableSetup({ playerNames, startingLife: value });
  }, [playerNames]);
  const [logOpen, setLogOpen] = useState(false);
  const [lobbyOpen, setLobbyOpen] = useState(false);
  const [zoneViews, setZoneViews] = useState(["battlefield"]);
  const [deckLoadingMode, setDeckLoadingMode] = useState(false);
  const [puzzleSetupMode, setPuzzleSetupMode] = useState(false);
  const [initializationError, setInitializationError] = useState(null);
  const [mobileOpponentIndex, setMobileOpponentIndex] = useState(0);
  const [mobileViewMode, setMobileViewMode] = useState("battlefield");
  const [notices, setNotices] = useState([]);
  const { landscapeMobileViewport, nonDesktopViewport, tabletCompactViewport, smallDesktopViewport } = useViewportLayout();
  const nextNoticeIdRef = useRef(1);
  const autoJoinAttemptedLobbyRef = useRef("");
  const autoLoadAttemptedPuzzleRef = useRef(false);
  const initialLobbyQueryRef = useRef(
    (commanderStartup || initialPuzzleQueryRef.current) ? emptyLobbyQueryParams() : readLobbyQueryParams()
  );
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

  const handleNoticeCopy = useCallback(async (copyTarget) => {
    if (!copyTarget?.copyText) return;
    const copied = await copyTextToClipboard(copyTarget.copyText);
    setStatus(
      copied
        ? (copyTarget.copyStatusMessage || "Copied to clipboard")
        : "Could not copy to clipboard",
      !copied
    );
  }, [setStatus]);

  const topLevelNotices = notices.length > 0 ? (
    <div className="pointer-events-none fixed inset-x-2 top-2 z-[10000] flex justify-end">
      <div className="pointer-events-auto flex max-w-[min(460px,calc(100vw-1rem))] flex-col gap-2">
        {notices.map((notice) => {
          const toneClasses = notice.tone === "success"
            ? "workspace-notice workspace-notice--success"
            : notice.tone === "error"
              ? "workspace-notice workspace-notice--error"
              : notice.tone === "warning"
                ? "workspace-notice workspace-notice--warning"
                : "workspace-notice workspace-notice--info";
          const actions = Array.isArray(notice.actions)
            ? notice.actions.filter((action) => action?.copyText)
            : [];
          return (
            <div key={notice.id} className={`relative overflow-hidden border shadow-[0_10px_26px_rgba(0,0,0,0.45)] ${toneClasses}`}>
              <div className="workspace-notice-body px-3 py-2 pr-9 text-left">
                <div className="workspace-notice-title text-[13px] font-bold uppercase tracking-wide">{ui(notice.title)}</div>
                {notice.body ? <div className="workspace-notice-text mt-1 max-h-[40vh] overflow-y-auto whitespace-pre-line text-[13px] font-semibold leading-tight">{ui(notice.body)}</div> : null}
              </div>
              {actions.length > 0 ? (
                <div className="flex gap-2 overflow-x-auto px-3 pb-3 pr-9">
                  {actions.map((action, index) => (
                    <button key={`${notice.id}:${action.label}:${index}`} type="button" className="workspace-notice-action shrink-0 border px-2.5 py-1 text-[11px] font-bold uppercase tracking-wide transition-colors" onClick={() => handleNoticeCopy(action)}>
                      {ui(action.label)}
                    </button>
                  ))}
                </div>
              ) : null}
              <button type="button" className="workspace-notice-dismiss absolute right-1.5 top-1.5 px-1 text-[12px] font-bold text-current opacity-80 transition-opacity hover:opacity-100" onClick={() => dismissNotice(notice.id)} aria-label={ui("Dismiss {0}", { 0: notice.title })}>x</button>
            </div>
          );
        })}
      </div>
    </div>
  ) : null;

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
      name: queryLobby.name || readStoredPlayerName() || parseNames(playerNames)[0] || "Player",
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

  // The startup board needs no engine, so build it while the WASM downloads.
  const startupBoardRef = useRef(null);
  useEffect(() => {
    if (initialPuzzleQueryRef.current || (!commanderStartup && fixedStartingBoard)) return;
    const lobbyQuery = initialLobbyQueryRef.current;
    if (lobbyQuery.lobbyId || lobbyQuery.deckText || lobbyQuery.commanderText) return;
    startupBoardRef.current = prefetchRandomStartingBoard(
      tableSeatNames(playerNames), startingLife, commanderStartup ? 0 : semanticThreshold
    );
    // Mount-time settings are the ones the first init reads.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Initialize game when WASM loads
  useEffect(() => {
    if (!game) return;
    async function init() {
      setInitializationError(null);
      try {
        if (typeof game.setSemanticThreshold === "function") {
          await game.setSemanticThreshold(semanticThreshold);
        }
        const initialPuzzle = initialPuzzleQueryRef.current;
        if (commanderStartup) {
          // This URL is an explicit preset, independent of saved table/board
          // settings and deck fidelity filters. Four seats choose EDH or PDH.
          const payload = await loadRandomStartingBoard(tableSeatNames(playerNames), 40, 0, {
            initialPayload: await startupBoardRef.current?.payload.catch(() => null),
            apply: (built) => applyStartingBoard(game, built),
          });
          startupBoardRef.current = null;
          setStartingLife(payload.match.startingLife);
          await refresh("WASM loaded", { throwOnError: true });
        } else if (initialPuzzle) {
          const loaded = await applyPuzzleToGame(game, initialPuzzle);
          setPlayerNames(loaded.playerNamesList.join(","));
          setStartingLife(loaded.defaultStartingLife);
          setDeckLoadingMode(false);
          setPuzzleSetupMode(false);
          setLobbyOpen(false);
          autoLoadAttemptedPuzzleRef.current = true;
          const skippedSuffix = loaded.skippedCardNames.length > 0
            ? `; skipped unsupported cards: ${loaded.skippedCardNames.join(", ")}`
            : "";
          await refresh(`Puzzle loaded from link${skippedSuffix}`, { throwOnError: true });
        } else if (!fixedStartingBoard && (
          initialLobbyQueryRef.current.lobbyId
          || initialLobbyQueryRef.current.deckText
          || initialLobbyQueryRef.current.commanderText
        )) {
          // A multiplayer link supplies its own decks. Initialize the empty
          // table needed by the lobby without loading an unrelated random game.
          await game.resetEmpty(parseNames(playerNames), startingLife);
          await refresh("WASM loaded", { throwOnError: true });
        } else {
          const names = tableSeatNames(playerNames);
          const prefetched = startupBoardRef.current;
          startupBoardRef.current = null;
          const prefetchedPayload = prefetched?.key === startingBoardKey(names, startingLife, semanticThreshold)
            ? await prefetched.payload.catch(() => null)
            : null;
          if (prefetchedPayload && !fixedStartingBoard) {
            await loadRandomStartingBoard(names, startingLife, semanticThreshold, {
              initialPayload: prefetchedPayload,
              apply: (built) => applyStartingBoard(game, built),
            });
            await markStartupFoils(game);
          } else {
            await resetStartingBoard(game, names, startingLife, fixedStartingBoard, semanticThreshold);
          }
          await refresh("WASM loaded", { throwOnError: true });
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setInitializationError(message);
        setStatus(`Init failed: ${message}`, true);
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

      try {
        const loaded = await applyPuzzleToGame(game, normalized);
        setPlayerNames(loaded.playerNamesList.join(","));
        setStartingLife(loaded.defaultStartingLife);
        setDeckLoadingMode(false);
        setPuzzleSetupMode(false);
        setLobbyOpen(false);
        const skippedSuffix = loaded.skippedCardNames.length > 0
          ? `; skipped unsupported cards: ${loaded.skippedCardNames.join(", ")}`
          : "";
        await refresh(`${successMessage}${skippedSuffix}`);
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
        const names = tableSeatNames(playerNames);
        await resetStartingBoard(game, names, startingLife, fixedStartingBoard, semanticThreshold);
        setDeckLoadingMode(false);
        await refresh("Game reset");
      } catch (err) {
        setStatus(`Reset failed: ${err}`, true);
      }
    });
  }, [game, multiplayer.mode, playerNames, refresh, runWasmInteraction, setStatus, startingLife, fixedStartingBoard, semanticThreshold]);

  const handleLoadCustomDecks = useCallback(async (payload) => {
    return runWasmInteraction(async () => {
      if (!game) return false;
      if (multiplayer.mode !== "idle") {
        setStatus("Deck loading is disabled while a lobby is active", true);
        return false;
      }
      try {
        setStatus("Preparando partida de prueba con bots...");
        const requestedPlayerCount = Number(payload?.playerCount);
        const playerCount = payload?.preserveMissingDecks && Number.isFinite(requestedPlayerCount)
          ? Math.max(2, Math.min(4, Math.floor(requestedPlayerCount)))
          : null;
        const names = parseNames(playerNames);
        const testNames = playerCount
          ? Array.from({ length: playerCount }, (_, index) => (
            names[index] || (index === 0 ? "Player 1" : `Bot ${index + 1}`)
          ))
          : names;
        if (playerCount && testNames.join(",") !== names.join(",")) {
          await game.reset(testNames, startingLife);
          setPlayerNames(testNames.join(","));
        }
        const loadPayload = playerCount
          ? {
              ...payload,
              decks: Array.isArray(payload?.decks) ? payload.decks.slice(0, playerCount) : [],
              sideboards: Array.isArray(payload?.sideboards) ? payload.sideboards.slice(0, playerCount) : [],
              stickerSheets: Array.isArray(payload?.stickerSheets) ? payload.stickerSheets.slice(0, playerCount) : [],
            }
          : payload;
        const result = await game.loadDecks(loadPayload);
        if (playerCount && Number.isFinite(Number(payload?.perspectivePlayerIndex))) {
          await game.setPerspective(
            Math.max(0, Math.min(playerCount - 1, Number(payload.perspectivePlayerIndex)))
          );
        }
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
            tone: payload?.allowPartialDecks ? "warning" : "error",
            title: payload?.allowPartialDecks ? "Deck test warnings" : "Deck load issues",
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
            `Partida de prueba lista: ${loaded} cartas en bibliotecas y manos. ${failed.length} fallaron${issueSummary ? ` (${issueSummary})` : ""}: ${failedStr}`
          );
        } else {
          await refresh(`Partida de prueba lista: ${loaded} cartas en bibliotecas y manos.`);
        }
        return true;
      } catch (err) {
        setDeckLoadingMode(false);
        setStatus(`Load decks failed: ${err}`, true);
        return false;
      }
    });
  }, [game, multiplayer.mode, playerNames, pushNotice, refresh, runWasmInteraction, setStatus, startingLife]);

  // Goldfish games are local. Normal ones reuse the deck-test path (bots keep
  // the demo deck in empty seats); Commander needs a real commander setup, so
  // it starts a full match where empty seats mirror a loaded deck.
  useEffect(() => {
    if (multiplayer.matchStarted) recordPlayedDeck(readDefaultLobbyDeck());
  }, [multiplayer.matchStarted]);

  const handleStartGoldfish = useCallback(async (payload) => {
    let started = false;
    // Same rule as hosted lobbies, in every format: an unloadable card becomes
    // a basic land instead of being dropped or rejecting the whole deck.
    const substitutions = [];
    const withSupportedSeats = async (decks, sideboards) => {
      for (let index = 0; index < decks.length; index += 1) {
        const supported = await withSupportedCards(
          { deck: decks[index] || [], sideboard: sideboards[index] || [] },
          { game, onSubstitute: (entries) => substitutions.push(...entries) },
        );
        decks[index] = supported.deck;
        sideboards[index] = supported.sideboard;
      }
      // A card that circles colors as its owner builds the deck needs them
      // written after its name ("Cryptic Spires ({W}{B})").
      for (let index = 0; index < decks.length; index += 1) {
        const issues = await submissionColorNoteIssues(game, {
          deck: decks[index], sideboard: sideboards[index], commanders: payload?.commanders?.[index],
        });
        if (issues.length) throw new Error(describeColorNoteIssues(issues));
      }
    };
    // Pauper EDH goldfish runs as its own engine format (30 life, 16 commander damage).
    const goldfishFormat = normalizeMatchFormat(payload?.format);
    if (!usesCommanders(goldfishFormat)) {
      const decks = [...(payload?.decks || [])];
      const sideboards = decks.map((_, index) => payload?.sideboards?.[index] || []);
      try {
        await withSupportedSeats(decks, sideboards);
      } catch (err) {
        setStatus(`Goldfish failed: ${err}`, true);
        return false;
      }
      started = await handleLoadCustomDecks({ ...payload, decks, sideboards });
    } else {
      started = await runWasmInteraction(async () => {
        if (!game || typeof game.startMatch !== "function") return false;
        if (multiplayer.mode !== "idle") {
          setStatus("Deck loading is disabled while a lobby is active", true);
          return false;
        }
        const playerCount = Math.max(2, Math.min(4, Math.floor(Number(payload?.playerCount) || 2)));
        const decks = Array.from({ length: playerCount }, (_, index) => payload?.decks?.[index] || []);
        const commanders = Array.from({ length: playerCount }, (_, index) => payload?.commanders?.[index] || []);
        const sideboards = Array.from({ length: playerCount }, (_, index) => payload?.sideboards?.[index] || []);
        const stickerSheets = Array.from({ length: playerCount }, (_, index) => payload?.stickerSheets?.[index] || []);
        const sourceIndex = decks.findIndex((deck, index) => deck.length > 0 && commanders[index].length > 0);
        if (sourceIndex < 0) {
          setStatus("Commander goldfish needs at least one deck with a Commander section", true);
          return false;
        }
        for (let index = 0; index < playerCount; index += 1) {
          if (decks[index].length > 0 && commanders[index].length > 0) continue;
          decks[index] = decks[sourceIndex];
          commanders[index] = commanders[sourceIndex];
          sideboards[index] = sideboards[sourceIndex];
          stickerSheets[index] = stickerSheets[sourceIndex];
        }
        const names = parseNames(playerNames);
        const goldfishNames = Array.from({ length: playerCount }, (_, index) => (
          names[index] || (index === 0 ? "Player 1" : `Bot ${index + 1}`)
        ));
        try {
          setStatus("Starting Commander goldfish...");
          await withSupportedSeats(decks, sideboards);
          await game.startMatch({
            playerNames: goldfishNames,
            startingLife: PUBLIC_FORMATS[goldfishFormat]?.startingLife || 40,
            seed: Math.floor(Math.random() * Number.MAX_SAFE_INTEGER),
            format: PUBLIC_FORMATS[goldfishFormat]?.engineFormat || MATCH_FORMAT_COMMANDER,
            decks,
            sideboards,
            commanders,
            stickerSheets,
          });
          setPlayerNames(goldfishNames.join(","));
          const perspective = Math.max(0, Math.min(playerCount - 1, Number(payload?.perspectivePlayerIndex) || 0));
          await game.setPerspective(perspective);
          setDeckLoadingMode(false);
          setPuzzleSetupMode(false);
          await refresh("Commander goldfish ready");
          return true;
        } catch (err) {
          setStatus(`Goldfish failed: ${err}`, true);
          return false;
        }
      });
    }
    if (started) {
      const seat = payload?.perspectivePlayerIndex || 0;
      recordPlayedDeck({
        deckText: (payload?.decks?.[seat] || []).map(name => `1 ${name}`).join("\n") + "\n\nSideboard\n" + (payload?.sideboards?.[seat] || []).map(name => `1 ${name}`).join("\n"),
        commanderText: (payload?.commanders?.[seat] || []).map(name => `1 ${name}`).join("\n"),
      });
      setLobbyOpen(false);
      // Mirrored seats repeat the same swaps; list each card once.
      const replaced = [...new Map(substitutions.map((entry) => [entry.from, entry])).values()];
      if (replaced.length > 0) {
        const lines = replaced.map(({ from, to }) => `${from} → ${to}`);
        pushNotice({
          tone: "warning",
          title: replaced.length === 1 ? "1 card replaced by a land" : `${replaced.length} cards replaced by lands`,
          body: lines.join("\n"),
          actions: [{
            label: "Copy list",
            copyText: lines.join("\n"),
            copyStatusMessage: "Copied replaced card list",
          }],
        });
      }
    }
    return started;
  }, [game, handleLoadCustomDecks, multiplayer.mode, playerNames, pushNotice, refresh, runWasmInteraction, setPlayerNames, setStatus]);

  const handleChangePerspective = useCallback(
    async (playerIndex) => {
      if (multiplayer.matchStarted) return;
      return changeGoldfishPlayer(playerIndex);
    },
    [multiplayer.matchStarted, changeGoldfishPlayer]
  );

  const [smithComplete, setSmithComplete] = useState(false);

  if (loading || (!smithComplete && !wasmError && !initializationError)) {
    const widthPct = Math.max(0, Math.min(100, wasmProgress * 100));
    // Avoid showing 100% until progress has actually completed.
    const pct = wasmProgress >= 1 ? 100 : Math.floor(widthPct);
    const phaseLabel =
      wasmPhase === "module" ? "Loading module..." :
      wasmPhase === "download" ? "Downloading WASM..." :
      wasmPhase === "registry" ? `Compiled ${Number(wasmRegistryCount || 0).toLocaleString()} cards...` :
      "Initializing";
    return (
      <div className="game-loading-screen flex h-screen flex-col items-center justify-center gap-4 text-muted-foreground">
        <LoadingSmith progress={wasmProgress} ready={!loading && !!state} onComplete={() => setSmithComplete(true)} />
        {wasmPhase === "init" ? (
          <span className="game-loading-status text-[16px] font-semibold">
            {ui(phaseLabel)}
            <span className="loading-dots" aria-hidden="true">
              <span className="loading-dot loading-dot-1">.</span>
              <span className="loading-dot loading-dot-2">.</span>
              <span className="loading-dot loading-dot-3">.</span>
            </span>
          </span>
        ) : (
          <span className="game-loading-status text-[16px] font-semibold">{ui(phaseLabel)}</span>
        )}
        <div className="game-loading-track h-2 w-64 overflow-hidden border">
          <div
            className="game-loading-progress h-full"
            style={{ width: `${widthPct}%` }}
          />
        </div>
        <span className="game-loading-percent text-[14px] tabular-nums">{pct}%</span>
      </div>
    );
  }

  if (wasmError) {
    return (
      <div className="flex items-center justify-center h-screen text-destructive">{ui("WASM failed:") + " "}{ui(wasmError.message)}
      </div>
    );
  }

  if (initializationError) {
    return (
      <div className="flex flex-col items-center justify-center h-screen gap-3 px-6 text-center text-destructive">
        <span className="text-[18px] font-bold uppercase tracking-wider">{ui("Game initialization failed")}</span>
        <span className="max-w-2xl text-sm text-muted-foreground">
          {ui(initializationError)}
        </span>
        <span className="text-sm text-muted-foreground">
          {ui("Refresh the tab to try again.")}
        </span>
      </div>
    );
  }

  // Worker can be ready before initial reset/demo-setup has produced first UI state.
  if (!state) {
    return (
      <div className="game-loading-screen flex h-screen flex-col items-center justify-center gap-4 text-muted-foreground">
        <LoadingSmith progress={wasmProgress} />
        <span className="game-loading-status text-[16px] font-semibold">{ui("Preparing Game")}<span className="loading-dots" aria-hidden="true">
            <span className="loading-dot loading-dot-1">.</span>
            <span className="loading-dot loading-dot-2">.</span>
            <span className="loading-dot loading-dot-3">.</span>
          </span>
        </span>
      </div>
    );
  }

  const dockToolbarsInTable = !nonDesktopViewport && !tabletCompactViewport;
  const renderTopLevelAddCardBar = !landscapeMobileViewport && !tabletCompactViewport && !dockToolbarsInTable;
  const utilityControlsElement = (
    <TopbarUtilityControls
      playerNames={playerNames}
      setPlayerNames={editPlayerNames}
      startingLife={startingLife}
      setStartingLife={editStartingLife}
      onReset={handleReset}
      onRefresh={() => void runWasmInteraction(() => refresh("Refreshed"))}
      onToggleLog={() => setLogOpen((o) => !o)}
      onOpenPuzzleSetup={() => {
        setDeckLoadingMode(false);
        setPuzzleSetupMode((mode) => !mode);
      }}
      onGenerateRandomGame={(payload, successMessage) => {
        setDeckLoadingMode(false);
        setPuzzleSetupMode(false);
        return runWasmInteraction(() => loadPuzzle(payload, successMessage));
      }}
      puzzleSetupMode={puzzleSetupMode}
      onOpenLobby={() => {
        setDeckLoadingMode(false);
        setPuzzleSetupMode(false);
        setLobbyOpen(true);
      }}
      onAddCardNotice={pushNotice}
    />
  );
  const topbarElement = (
    <Topbar
      utilityControls={utilityControlsElement}
      playerNames={playerNames}
      setPlayerNames={editPlayerNames}
      startingLife={startingLife}
      setStartingLife={editStartingLife}
      onReset={handleReset}
      onRefresh={() => void runWasmInteraction(() => refresh("Refreshed"))}
      onToggleLog={() => setLogOpen((o) => !o)}
      onOpenPuzzleSetup={() => {
        setDeckLoadingMode(false);
        setPuzzleSetupMode((mode) => !mode);
      }}
      onGenerateRandomGame={(payload, successMessage) => {
        setDeckLoadingMode(false);
        setPuzzleSetupMode(false);
        return runWasmInteraction(() => loadPuzzle(payload, successMessage));
      }}
      puzzleSetupMode={puzzleSetupMode}
      onOpenLobby={() => {
        setDeckLoadingMode(false);
        setPuzzleSetupMode(false);
        setLobbyOpen(true);
      }}
      onAddCardNotice={pushNotice}
      mobileOpponentIndex={mobileOpponentIndex}
      setMobileOpponentIndex={setMobileOpponentIndex}
      mobileOverlay={landscapeMobileViewport}
      middleDocked={dockToolbarsInTable}
      onChangePerspective={handleChangePerspective}
    />
  );
  const addCardBarElement = (
    <AddCardBar
      compact={smallDesktopViewport}
      utilityControls={utilityControlsElement}
    />
  );
  const zoneActionControlsElement = (
    <TableActionControls
      compact={false}
      onAddCardNotice={pushNotice}
      onOpenPuzzleSetup={() => {
        setDeckLoadingMode(false);
        setPuzzleSetupMode((mode) => !mode);
      }}
      onGenerateRandomGame={(payload, successMessage) => {
        setDeckLoadingMode(false);
        setPuzzleSetupMode(false);
        return runWasmInteraction(() => loadPuzzle(payload, successMessage));
      }}
      onOpenLobby={() => {
        setDeckLoadingMode(false);
        setPuzzleSetupMode(false);
        setLobbyOpen(true);
      }}
      puzzleSetupMode={puzzleSetupMode}
    />
  );

  return (
    <div
      className={
        landscapeMobileViewport
          ? "app-shell mobile-app-shell relative w-full h-[100dvh] overflow-hidden"
          : dockToolbarsInTable
            ? "app-shell w-full h-[100dvh] p-2 grid grid-rows-[minmax(0,1fr)] gap-2"
            : renderTopLevelAddCardBar
              ? "app-shell w-full h-[100dvh] p-2 grid grid-rows-[auto_auto_minmax(0,1fr)] gap-2"
              : "app-shell w-full h-[100dvh] p-2 grid grid-rows-[auto_minmax(0,1fr)] gap-2"
      }
      data-borderless-preview={borderlessPreview ? "true" : "false"}
      data-mobile-overlay-shell={landscapeMobileViewport ? "true" : "false"}
    >
      {topLevelNotices}
      {!deckLoadingMode && !puzzleSetupMode && multiplayer?.rematch?.phase !== "sideboarding" && <MobileLandscapeGate />}
      {(!landscapeMobileViewport && (!dockToolbarsInTable || deckLoadingMode || puzzleSetupMode)) ? (
        <div className="table-persistent-diagnostics-fallback"><DiagnosticsSheet /></div>
      ) : null}
      {!dockToolbarsInTable ? topbarElement : null}
      {renderTopLevelAddCardBar ? addCardBarElement : null}
      <TableErrorBoundary resetKey={state}>
        <Workspace
          onChangePerspective={handleChangePerspective}
          zoneViews={zoneViews}
          setZoneViews={setZoneViews}
          deckLoadingMode={deckLoadingMode}
          puzzleSetupMode={puzzleSetupMode}
          onLoadDecks={handleLoadCustomDecks}
          onTestDecks={handleLoadCustomDecks}
          onCancelDeckLoading={() => setDeckLoadingMode(false)}
          onLoadPuzzle={(payload, successMessage) => runWasmInteraction(
            () => loadPuzzle(payload, successMessage)
          )}
          onCancelPuzzleSetup={() => setPuzzleSetupMode(false)}
          notices={[]}
          onDismissNotice={dismissNotice}
          mobileOpponentIndex={mobileOpponentIndex}
          setMobileOpponentIndex={setMobileOpponentIndex}
          mobileViewMode={mobileViewMode}
          setMobileViewMode={setMobileViewMode}
          middleUtilityControls={dockToolbarsInTable ? utilityControlsElement : null}
          middleTopbar={dockToolbarsInTable ? topbarElement : null}
          middleAddCardBar={null}
          zoneActionControls={zoneActionControlsElement}
        />
      </TableErrorBoundary>
      <LogDrawer open={logOpen} onOpenChange={setLogOpen} />
      {!multiplayer.matchStarted ? <PerspectiveHandoffBanner /> : null}
      <KeyboardShortcutsOverlay />
      <FullCardsManaRail />
      {lobbyOpen ? (
        <LobbyOverlay
          key={lobbyOverlayInitial.mode}
          onClose={() => setLobbyOpen(false)}
          onStartGoldfish={handleStartGoldfish}
          defaultName={parseNames(playerNames)[0] || "Player"}
          defaultStartingLife={startingLife}
          initialMode={lobbyOverlayInitial.mode}
          initialCreateFormat={lobbyOverlayInitial.createFormat}
          initialCreateName={lobbyOverlayInitial.createName}
          initialCreateDeckText={lobbyOverlayInitial.createDeckText}
          initialCreateDeckOptions={lobbyOverlayInitial.createDeckOptions}
          initialCreateCommanderText={lobbyOverlayInitial.createCommanderText}
          initialCreateSecurityMode={lobbyOverlayInitial.createSecurityMode}
          initialDesiredPlayers={lobbyOverlayInitial.desiredPlayers}
          initialJoinCode={lobbyOverlayInitial.joinCode}
          initialJoinName={lobbyOverlayInitial.joinName}
          initialJoinDeckText={lobbyOverlayInitial.joinDeckText}
          initialJoinCommanderText={lobbyOverlayInitial.joinCommanderText}
        />
      ) : null}
    </div>
  );
}

function emptyLobbyQueryParams() {
  return {
    lobbyId: "",
    name: "",
    deckText: "",
    commanderText: "",
  };
}

function puzzlePlayerNames(payload) {
  const normalized = normalizePuzzlePayload(payload);
  if (!normalized) return [];
  return normalized.players.map((player, index) => (
    String(player?.name || "").trim() || `Player ${index + 1}`
  ));
}

function initialPuzzlePlayerNames(payload) {
  const names = puzzlePlayerNames(payload);
  return names.length > 0 ? names.join(",") : "";
}

function initialPuzzleStartingLife(payload) {
  const normalized = normalizePuzzlePayload(payload);
  if (!normalized) return null;
  return Number(normalized.players[0]?.life) || 20;
}

async function resetStartingBoard(game, names, startingLife, fixed, semanticThreshold) {
  if (fixed) {
    await game.reset(names, startingLife);
    await addFixedStartingBoardPreset(game, names.length);
    return;
  }
  await loadRandomStartingBoard(names, startingLife, semanticThreshold, {
    apply: (built) => applyStartingBoard(game, built),
  });
  await markStartupFoils(game);
}

// Commander tables start as real matches; other formats load a seeded board.
async function applyStartingBoard(game, payload) {
  if (!payload?.match) return applyPuzzleToGame(game, payload);
  if (typeof game.startMatch !== "function") throw new Error("This engine cannot start a Commander match");
  await game.startMatch({
    ...payload.match,
    seed: Math.floor(Math.random() * Number.MAX_SAFE_INTEGER),
  });
  if (typeof game.setPerspective === "function") {
    const state = await game.uiState();
    await game.setPerspective(state.active_player);
  }
}

async function applyPuzzleToGame(game, payload) {
  const normalized = normalizePuzzlePayload(payload);
  if (!normalized) throw new Error("Puzzle payload is invalid");

  const playerNamesList = puzzlePlayerNames(normalized);
  const defaultStartingLife = Number(normalized.players[0]?.life) || 20;
  if (typeof game.resetEmpty === "function") {
    await game.resetEmpty(playerNamesList, defaultStartingLife);
  } else {
    await game.reset(playerNamesList, defaultStartingLife);
  }
  const cardsToAdd = [];
  for (const [playerIndex, player] of normalized.players.entries()) {
    if (typeof game.setLife === "function") {
      await game.setLife(playerIndex, Number(player?.life) || 20);
    }
    for (const zone of PUZZLE_ZONE_ORDER) {
      for (const cardName of player.zones?.[zone] || []) {
        cardsToAdd.push({
          playerIndex,
          cardName,
          zoneName: zone,
          skipTriggers: true,
        });
      }
    }
  }
  let supportedCards = cardsToAdd;
  let skippedCardNames = [];
  if (cardsToAdd.length > 0 && typeof game.filterKnownCardNames === "function") {
    const uniqueCardNames = [...new Set(cardsToAdd.map((entry) => entry.cardName))];
    const knownCardNames = await game.filterKnownCardNames(uniqueCardNames);
    const knownKeys = new Set(
      (Array.isArray(knownCardNames) ? knownCardNames : [])
        .map((name) => String(name || "").trim().toLocaleLowerCase("en-US"))
    );
    supportedCards = cardsToAdd.filter((entry) => (
      knownKeys.has(entry.cardName.toLocaleLowerCase("en-US"))
    ));
    skippedCardNames = uniqueCardNames.filter((name) => (
      !knownKeys.has(name.toLocaleLowerCase("en-US"))
    ));
  }
  if (supportedCards.length > 0) {
    if (typeof game.addCardsToZones === "function") {
      await game.addCardsToZones(supportedCards);
    } else {
      for (const entry of supportedCards) {
        await game.addCardToZone(
          entry.playerIndex,
          entry.cardName,
          entry.zoneName,
          entry.skipTriggers
        );
      }
    }
  }
  if (typeof game.finishPuzzleSetup === "function") {
    await game.finishPuzzleSetup();
  }

  return { playerNamesList, defaultStartingLife, skippedCardNames };
}

function readLobbyQueryParams() {
  if (typeof window === "undefined") {
    return emptyLobbyQueryParams();
  }

  const params = new URLSearchParams(window.location.search);
  const lobbyId = String(params.get("lobby") || "").trim();
  const hasDeckParam = params.has("deck");
  const hasCommanderParam = params.has("commander");
  const savedLobbyDeck =
    lobbyId && !hasDeckParam && !hasCommanderParam ? readDefaultLobbyDeck() : null;
  return {
    lobbyId,
    name: String(params.get("name") || "").trim(),
    deckText: hasDeckParam
      ? decodeBase64UrlUtf8(params.get("deck"))
      : String(savedLobbyDeck?.deckText || ""),
    commanderText: hasCommanderParam
      ? decodeBase64UrlUtf8(params.get("commander"))
      : String(savedLobbyDeck?.commanderText || ""),
    securityMode: normalizeMultiplayerSecurityMode(
      params.get("securityMode") || params.get("security"),
      MULTIPLAYER_SECURITY_TRUSTED
    ),
  };
}

function stripLobbyCodeFromUrl() {
  if (typeof window === "undefined") return;
  const url = new URL(window.location.href);
  url.searchParams.delete("lobby");
  window.history.replaceState({}, "", url.toString());
}

// A commander list implies Commander; otherwise the host picks the format.
function inferCreateFormatFromLobbyQuery(query) {
  return String(query?.commanderText || "").trim()
    ? MATCH_FORMAT_COMMANDER
    : "";
}

function buildLobbyOverlayInitialState(query, mode = null) {
  const nextMode = mode || (query?.lobbyId ? "join" : "create");
  return {
    mode: nextMode === "join" ? "join" : "create",
    createFormat: inferCreateFormatFromLobbyQuery(query),
    createName: String(query?.name || "").trim(),
    createDeckText: String(query?.deckText || ""),
    createDeckOptions: [],
    createCommanderText: String(query?.commanderText || ""),
    createSecurityMode: normalizeMultiplayerSecurityMode(
      query?.securityMode,
      MULTIPLAYER_SECURITY_TRUSTED
    ),
    desiredPlayers: Math.max(2, Math.min(4, Number(query?.desiredPlayers) || 2)),
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
