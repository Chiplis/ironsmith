import { useCallback, useEffect, useRef, useState } from "react";
import Peer from "peerjs";
import {
  MATCH_FORMAT_COMMANDER,
  MATCH_FORMAT_NORMAL,
  evaluateLobbyDeckSubmission,
  normalizeMatchFormat,
  parseCommanderList,
  parseDeckList,
} from "@/lib/decklists";

const PROTOCOL_VERSION = 1;
const DEFAULT_OPENING_HAND_SIZE = 7;

function createEmptyState() {
  return {
    role: null,
    mode: "idle",
    lobbyId: "",
    hostPeerId: "",
    localPeerId: "",
    localName: "",
    localPlayerIndex: null,
    desiredPlayers: 0,
    startingLife: 20,
    format: MATCH_FORMAT_NORMAL,
    localDeckText: "",
    localCommanderText: "",
    localDeckCount: 0,
    localCommanderCount: 0,
    players: [],
    matchStarted: false,
    lastAppliedSequence: 0,
    submittingAction: false,
  };
}

function sanitizePlayerName(raw, fallback = "Player") {
  const trimmed = String(raw || "").trim();
  return trimmed || fallback;
}

function createMatchSeed() {
  const seed = Math.floor(Math.random() * Number.MAX_SAFE_INTEGER);
  return seed > 0 ? seed : 1;
}

function safeSend(conn, payload) {
  if (!conn || conn.open === false) return;
  conn.send(payload);
}

function sanitizeCardList(cards) {
  if (!Array.isArray(cards)) return [];
  return cards
    .map((card) => String(card || "").trim())
    .filter(Boolean);
}

function parseDeckSubmission(format, deckText, commanderText = "") {
  const deck = sanitizeCardList(parseDeckList(deckText));
  const commanders = sanitizeCardList(parseCommanderList(commanderText));
  const status = evaluateLobbyDeckSubmission(format, deck, commanders);
  return {
    deck,
    commanders,
    deckCount: status.deckCount,
    commanderCount: status.commanderCount,
    ready: status.ready,
  };
}

function withDeckState(player, format, deck, commanders = []) {
  const normalizedDeck = sanitizeCardList(deck);
  const normalizedCommanders = sanitizeCardList(commanders);
  const status = evaluateLobbyDeckSubmission(
    format,
    normalizedDeck,
    normalizedCommanders
  );
  return {
    ...player,
    deck: normalizedDeck,
    commanders: normalizedCommanders,
    deckCount: status.deckCount,
    commanderCount: status.commanderCount,
    ready: status.ready,
  };
}

function reindexPlayers(players) {
  return players.map((player, index) => ({ ...player, index }));
}

function toPublicPlayer(player) {
  return {
    peerId: player.peerId,
    name: player.name,
    index: player.index,
    connected: player.connected !== false,
    ready: Boolean(player.ready),
    deckCount: Number(player.deckCount || 0),
    commanderCount: Number(player.commanderCount || 0),
  };
}

function toPublicPlayers(players) {
  return reindexPlayers(players).map(toPublicPlayer);
}

function canHostedMatchStart(session) {
  return (
    session.role === "host" &&
    !session.matchStarted &&
    session.mode !== "starting" &&
    session.players.length === session.desiredPlayers &&
    session.players.length > 0 &&
    session.players.every((player) => player.ready)
  );
}

export function usePeerLobby({ game, setState, setStatus, applySyncedCommand }) {
  const [multiplayer, setMultiplayer] = useState(() => createEmptyState());
  const peerRef = useRef(null);
  const hostConnectionRef = useRef(null);
  const clientConnectionsRef = useRef(new Map());
  const gameRef = useRef(game);
  const multiplayerRef = useRef(multiplayer);

  useEffect(() => {
    gameRef.current = game;
  }, [game]);

  useEffect(() => {
    multiplayerRef.current = multiplayer;
  }, [multiplayer]);

  const updateMultiplayer = useCallback((updater) => {
    const next =
      typeof updater === "function" ? updater(multiplayerRef.current) : updater;
    multiplayerRef.current = next;
    setMultiplayer(next);
    return next;
  }, []);

  const teardownPeer = useCallback(() => {
    const hostConn = hostConnectionRef.current;
    hostConnectionRef.current = null;
    if (hostConn) {
      try {
        hostConn.close();
      } catch (err) {
        void err;
      }
    }

    for (const conn of clientConnectionsRef.current.values()) {
      try {
        conn.close();
      } catch (err) {
        void err;
      }
    }
    clientConnectionsRef.current.clear();

    const peer = peerRef.current;
    peerRef.current = null;
    if (peer) {
      try {
        peer.destroy();
      } catch (err) {
        void err;
      }
    }
  }, []);

  const leaveLobby = useCallback(
    (message = "Left lobby") => {
      teardownPeer();
      updateMultiplayer(createEmptyState());
      if (message) {
        setStatus(message);
      }
    },
    [setStatus, teardownPeer, updateMultiplayer]
  );

  const broadcastToClients = useCallback((payload) => {
    for (const conn of clientConnectionsRef.current.values()) {
      safeSend(conn, payload);
    }
  }, []);

  const applyMatchStart = useCallback(
    async (payload) => {
      const currentGame = gameRef.current;
      if (!currentGame || typeof currentGame.startMatch !== "function") {
        throw new Error("Game engine is not ready for multiplayer");
      }

      const currentSession = multiplayerRef.current;
      const localEntry = payload.players.find(
        (player) => player.peerId === currentSession.localPeerId
      );

      if (!localEntry) {
        throw new Error("Local player is missing from the match payload");
      }

      await currentGame.startMatch({
        playerNames: payload.players.map((player) => player.name),
        startingLife: payload.startingLife,
        seed: payload.seed,
        format: payload.format,
        decks: payload.decks,
        commanders: payload.commanders,
        openingHandSize: payload.openingHandSize ?? DEFAULT_OPENING_HAND_SIZE,
      });
      await currentGame.setPerspective(localEntry.index);

      const nextState = await currentGame.uiState();
      setState(nextState);

      updateMultiplayer((prev) => ({
        ...prev,
        role: payload.hostPeerId === localEntry.peerId ? "host" : prev.role,
        mode: "in_match",
        lobbyId: payload.lobbyId || prev.lobbyId,
        hostPeerId: payload.hostPeerId || prev.hostPeerId,
        localPlayerIndex: localEntry.index,
        desiredPlayers: payload.players.length,
        startingLife: payload.startingLife,
        format: normalizeMatchFormat(payload.format),
        localDeckCount:
          payload.decks?.[localEntry.index]?.length ?? prev.localDeckCount,
        localCommanderCount:
          payload.commanders?.[localEntry.index]?.length ?? prev.localCommanderCount,
        players: payload.players,
        matchStarted: true,
        lastAppliedSequence: 0,
        submittingAction: false,
      }));

      setStatus(`Multiplayer match started as ${localEntry.name}`);
    },
    [setState, setStatus, updateMultiplayer]
  );

  const broadcastLobbyState = useCallback(() => {
    const session = multiplayerRef.current;
    if (session.role !== "host") return;
    broadcastToClients({
      type: "lobby_state",
      protocolVersion: PROTOCOL_VERSION,
      lobbyId: session.lobbyId,
      hostPeerId: session.localPeerId,
      desiredPlayers: session.desiredPlayers,
      startingLife: session.startingLife,
      format: session.format,
      players: toPublicPlayers(session.players),
      matchStarted: session.matchStarted,
    });
  }, [broadcastToClients]);

  const startHostedMatch = useCallback(async () => {
    const session = multiplayerRef.current;
    if (!canHostedMatchStart(session)) return;

    const players = reindexPlayers(session.players);

    const decks = players.map((player) => sanitizeCardList(player.deck));
    const format = normalizeMatchFormat(session.format);
    const commanders =
      format === MATCH_FORMAT_COMMANDER
        ? players.map((player) => sanitizeCardList(player.commanders))
        : null;

    const payload = {
      type: "match_start",
      protocolVersion: PROTOCOL_VERSION,
      lobbyId: session.lobbyId,
      hostPeerId: session.localPeerId,
      players: players.map(toPublicPlayer),
      format,
      decks,
      commanders: commanders || undefined,
      startingLife: session.startingLife,
      seed: createMatchSeed(),
      openingHandSize: DEFAULT_OPENING_HAND_SIZE,
    };

    updateMultiplayer((prev) => ({ ...prev, mode: "starting", players }));

    try {
      await applyMatchStart(payload);
      broadcastToClients(payload);
    } catch (err) {
      updateMultiplayer((prev) => ({ ...prev, mode: "lobby" }));
      setStatus(`Match start failed: ${err}`, true);
    }
  }, [applyMatchStart, broadcastToClients, setStatus, updateMultiplayer]);

  const handleHostDisconnect = useCallback(() => {
    updateMultiplayer((prev) => ({
      ...prev,
      mode: prev.matchStarted ? "in_match" : "idle",
    }));
    setStatus("Disconnected from lobby host", true);
  }, [setStatus, updateMultiplayer]);

  const handleHostMessage = useCallback(
    async (message) => {
      if (!message || typeof message !== "object") return;
      if (message.protocolVersion && message.protocolVersion !== PROTOCOL_VERSION) {
        setStatus("Lobby protocol version mismatch", true);
        return;
      }

      switch (message.type) {
        case "lobby_state": {
          updateMultiplayer((prev) => {
            const localEntry = (message.players || []).find(
              (player) => player.peerId === prev.localPeerId
            );
            return {
              ...prev,
              mode: message.matchStarted ? "starting" : "lobby",
              lobbyId: message.lobbyId || prev.lobbyId,
              hostPeerId: message.hostPeerId || prev.hostPeerId,
              desiredPlayers: Number(message.desiredPlayers || prev.desiredPlayers || 0),
              startingLife: Number(message.startingLife || prev.startingLife || 20),
              format: normalizeMatchFormat(message.format || prev.format),
              players: message.players || [],
              localPlayerIndex: localEntry ? localEntry.index : prev.localPlayerIndex,
              matchStarted: Boolean(message.matchStarted),
              submittingAction: false,
            };
          });
          return;
        }
        case "reject":
          leaveLobby(message.reason || "Lobby join rejected");
          return;
        case "action_error":
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus(message.reason || "Action rejected", true);
          return;
        case "match_start":
          await applyMatchStart(message);
          return;
        case "apply_action": {
          const nextSequence = Number(message.seq || 0);
          const session = multiplayerRef.current;
          if (nextSequence <= session.lastAppliedSequence) return;
          if (nextSequence !== session.lastAppliedSequence + 1) {
            setStatus("Multiplayer action order mismatch", true);
            return;
          }

          await applySyncedCommand(message.command, message.label || "", {
            actorIndex: message.actorIndex,
            sequence: nextSequence,
          });
          updateMultiplayer((prev) => ({
            ...prev,
            lastAppliedSequence: nextSequence,
            submittingAction: false,
          }));
          return;
        }
        default:
          return;
      }
    },
    [applyMatchStart, applySyncedCommand, leaveLobby, setStatus, updateMultiplayer]
  );

  const handleClientDisconnect = useCallback(
    (peerId) => {
      clientConnectionsRef.current.delete(peerId);
      const departed = multiplayerRef.current.players.find(
        (player) => player.peerId === peerId
      );
      updateMultiplayer((prev) => {
        if (prev.matchStarted) {
          return {
            ...prev,
            players: prev.players.map((player) =>
              player.peerId === peerId ? { ...player, connected: false } : player
            ),
          };
        }
        return {
          ...prev,
          players: reindexPlayers(
            prev.players.filter((player) => player.peerId !== peerId)
          ),
        };
      });
      if (departed) {
        setStatus(`${departed.name} disconnected`);
      }
      if (!multiplayerRef.current.matchStarted) {
        broadcastLobbyState();
      }
    },
    [broadcastLobbyState, setStatus, updateMultiplayer]
  );

  const sequenceHostedAction = useCallback(
    async ({ actorIndex, command, label, senderPeerId = null }) => {
      const session = multiplayerRef.current;
      const expectedActor = gameRef.current
        ? (await gameRef.current.uiState())?.decision?.player
        : null;
      if (
        expectedActor !== null
        && expectedActor !== undefined
        && Number(expectedActor) !== Number(actorIndex)
      ) {
        if (senderPeerId) {
          const conn = clientConnectionsRef.current.get(senderPeerId);
          safeSend(conn, {
            type: "action_error",
            protocolVersion: PROTOCOL_VERSION,
            reason: "It is not that player's turn to act",
          });
        }
        return;
      }

      const nextSequence = session.lastAppliedSequence + 1;
      await applySyncedCommand(command, label || "", {
        actorIndex,
        sequence: nextSequence,
      });
      updateMultiplayer((prev) => ({
        ...prev,
        lastAppliedSequence: nextSequence,
        submittingAction: false,
      }));
      broadcastToClients({
        type: "apply_action",
        protocolVersion: PROTOCOL_VERSION,
        seq: nextSequence,
        actorIndex,
        command,
        label: label || "",
      });
    },
    [applySyncedCommand, broadcastToClients, updateMultiplayer]
  );

  const handleClientMessage = useCallback(
    async (conn, message) => {
      if (!message || typeof message !== "object") return;
      if (message.protocolVersion && message.protocolVersion !== PROTOCOL_VERSION) {
        safeSend(conn, {
          type: "reject",
          protocolVersion: PROTOCOL_VERSION,
          reason: "Protocol version mismatch",
        });
        conn.close();
        return;
      }

      switch (message.type) {
        case "join_request": {
          const session = multiplayerRef.current;
          if (session.matchStarted) {
            safeSend(conn, {
              type: "reject",
              protocolVersion: PROTOCOL_VERSION,
              reason: "Match already started",
            });
            conn.close();
            return;
          }
          if (session.players.length >= session.desiredPlayers) {
            safeSend(conn, {
              type: "reject",
              protocolVersion: PROTOCOL_VERSION,
              reason: "Lobby is full",
            });
            conn.close();
            return;
          }

          clientConnectionsRef.current.set(conn.peer, conn);
          const name = sanitizePlayerName(
            message.name,
            `Player ${session.players.length + 1}`
          );
          updateMultiplayer((prev) => ({
            ...prev,
            mode: "lobby",
            players: reindexPlayers([
              ...prev.players.filter((player) => player.peerId !== conn.peer),
              withDeckState(
                {
                  peerId: conn.peer,
                  name,
                  connected: true,
                },
                prev.format,
                message.deck,
                message.commanders
              ),
            ]),
          }));
          setStatus(`${name} joined lobby`);
          broadcastLobbyState();
          return;
        }
        case "deck_update": {
          const session = multiplayerRef.current;
          if (session.matchStarted) return;
          updateMultiplayer((prev) => ({
            ...prev,
            players: reindexPlayers(
              prev.players.map((player) =>
                player.peerId === conn.peer
                  ? withDeckState(
                      player,
                      prev.format,
                      message.deck,
                      message.commanders
                    )
                  : player
              )
            ),
          }));
          broadcastLobbyState();
          return;
        }
        case "player_action":
          await sequenceHostedAction({
            actorIndex: Number(message.actorIndex),
            command: message.command,
            label: message.label || "",
            senderPeerId: conn.peer,
          });
          return;
        default:
          return;
      }
    },
    [
      broadcastLobbyState,
      sequenceHostedAction,
      setStatus,
      updateMultiplayer,
    ]
  );

  const configureHostConnection = useCallback(
    (conn) => {
      conn.on("data", (message) => {
        void handleClientMessage(conn, message).catch((err) => {
          safeSend(conn, {
            type: "action_error",
            protocolVersion: PROTOCOL_VERSION,
            reason: String(err),
          });
        });
      });
      conn.on("close", () => handleClientDisconnect(conn.peer));
      conn.on("error", () => handleClientDisconnect(conn.peer));
    },
    [handleClientDisconnect, handleClientMessage]
  );

  const createLobby = useCallback(
    ({
      name,
      desiredPlayers,
      startingLife,
      format = MATCH_FORMAT_NORMAL,
      deckText = "",
      commanderText = "",
    }) => {
      teardownPeer();
      const localName = sanitizePlayerName(name, "Host");
      const targetPlayers = Math.max(2, Math.min(4, Number(desiredPlayers) || 2));
      const lifeTotal = Math.max(1, Number(startingLife) || 20);
      const normalizedFormat = normalizeMatchFormat(format);
      const deckSubmission = parseDeckSubmission(
        normalizedFormat,
        deckText,
        commanderText
      );
      const peer = new Peer();
      peerRef.current = peer;

      updateMultiplayer({
        ...createEmptyState(),
        role: "host",
        mode: "hosting",
        localName,
        desiredPlayers: targetPlayers,
        startingLife: lifeTotal,
        format: normalizedFormat,
        localDeckText: String(deckText || ""),
        localCommanderText: String(commanderText || ""),
        localDeckCount: deckSubmission.deckCount,
        localCommanderCount: deckSubmission.commanderCount,
      });

      peer.on("open", (peerId) => {
        const session = multiplayerRef.current;
        const currentDeck = parseDeckSubmission(
          session.format,
          session.localDeckText,
          session.localCommanderText
        );
        updateMultiplayer((prev) => ({
          ...prev,
          mode: "lobby",
          lobbyId: peerId,
          hostPeerId: peerId,
          localPeerId: peerId,
          localPlayerIndex: 0,
          localDeckCount: currentDeck.deckCount,
          localCommanderCount: currentDeck.commanderCount,
          players: [
            withDeckState(
              {
                peerId,
                name: localName,
                index: 0,
                connected: true,
              },
              prev.format,
              currentDeck.deck,
              currentDeck.commanders
            ),
          ],
        }));
        setStatus(`Lobby created: ${peerId}`);
      });
      peer.on("connection", configureHostConnection);
      peer.on("error", (err) => {
        setStatus(`Lobby error: ${err}`, true);
        leaveLobby("");
      });
    },
    [configureHostConnection, leaveLobby, setStatus, teardownPeer, updateMultiplayer]
  );

  const joinLobby = useCallback(
    ({ name, lobbyId, deckText = "", commanderText = "" }) => {
      teardownPeer();
      const localName = sanitizePlayerName(name, "Guest");
      const targetLobby = String(lobbyId || "").trim();
      if (!targetLobby) {
        setStatus("Lobby code is required", true);
        return;
      }

      const deckSubmission = parseDeckSubmission(
        MATCH_FORMAT_NORMAL,
        deckText,
        commanderText
      );
      const peer = new Peer();
      peerRef.current = peer;

      updateMultiplayer({
        ...createEmptyState(),
        role: "client",
        mode: "joining",
        lobbyId: targetLobby,
        hostPeerId: targetLobby,
        localName,
        localDeckText: String(deckText || ""),
        localCommanderText: String(commanderText || ""),
        localDeckCount: deckSubmission.deckCount,
        localCommanderCount: deckSubmission.commanderCount,
      });

      peer.on("open", (peerId) => {
        updateMultiplayer((prev) => ({
          ...prev,
          localPeerId: peerId,
        }));

        const conn = peer.connect(targetLobby, { reliable: true });
        hostConnectionRef.current = conn;
        conn.on("open", () => {
          const session = multiplayerRef.current;
          const currentDeck = parseDeckSubmission(
            session.format,
            session.localDeckText,
            session.localCommanderText
          );
          safeSend(conn, {
            type: "join_request",
            protocolVersion: PROTOCOL_VERSION,
            name: localName,
            deck: currentDeck.deck,
            commanders: currentDeck.commanders,
          });
          setStatus(`Joined lobby ${targetLobby}`);
        });
        conn.on("data", (message) => {
          void handleHostMessage(message).catch((err) => {
            setStatus(`Lobby message failed: ${err}`, true);
          });
        });
        conn.on("close", handleHostDisconnect);
        conn.on("error", handleHostDisconnect);
      });
      peer.on("error", (err) => {
        setStatus(`Lobby error: ${err}`, true);
        leaveLobby("");
      });
    },
    [handleHostDisconnect, handleHostMessage, leaveLobby, setStatus, teardownPeer, updateMultiplayer]
  );

  const updateLobbyDeck = useCallback(
    (updates) => {
      const currentSession = multiplayerRef.current;
      if (currentSession.matchStarted || currentSession.mode === "starting") {
        return;
      }

      const nextDeckText =
        typeof updates === "string"
          ? String(updates)
          : Object.prototype.hasOwnProperty.call(updates || {}, "deckText")
            ? String(updates.deckText || "")
            : currentSession.localDeckText;
      const nextCommanderText =
        typeof updates === "string"
          ? currentSession.localCommanderText
          : Object.prototype.hasOwnProperty.call(updates || {}, "commanderText")
            ? String(updates.commanderText || "")
            : currentSession.localCommanderText;

      const deckSubmission = parseDeckSubmission(
        currentSession.format,
        nextDeckText,
        nextCommanderText
      );
      const nextSession = updateMultiplayer((prev) => ({
        ...prev,
        localDeckText: nextDeckText,
        localCommanderText: nextCommanderText,
        localDeckCount: deckSubmission.deckCount,
        localCommanderCount: deckSubmission.commanderCount,
        players:
          prev.role === "host" && prev.localPeerId
            ? reindexPlayers(
                prev.players.map((player) =>
                  player.peerId === prev.localPeerId
                    ? withDeckState(
                        player,
                        prev.format,
                        deckSubmission.deck,
                        deckSubmission.commanders
                      )
                    : player
                )
              )
            : prev.players,
      }));

      if (nextSession.role === "host") {
        broadcastLobbyState();
        return;
      }

      const conn = hostConnectionRef.current;
      if (nextSession.role === "client" && conn && conn.open !== false) {
        safeSend(conn, {
          type: "deck_update",
          protocolVersion: PROTOCOL_VERSION,
          deck: deckSubmission.deck,
          commanders: deckSubmission.commanders,
        });
      }
    },
    [broadcastLobbyState, updateMultiplayer]
  );

  const submitMultiplayerCommand = useCallback(
    async (command, label = "") => {
      const session = multiplayerRef.current;
      if (!session.matchStarted) {
        setStatus("Match has not started yet", true);
        return;
      }
      if (session.submittingAction) {
        setStatus("Waiting for the previous action to sync");
        return;
      }
      if (session.localPlayerIndex == null) {
        setStatus("Local player seat is not assigned", true);
        return;
      }

      if (session.role === "host") {
        updateMultiplayer((prev) => ({ ...prev, submittingAction: true }));
        try {
          await sequenceHostedAction({
            actorIndex: session.localPlayerIndex,
            command,
            label,
          });
        } catch (err) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          throw err;
        }
        return;
      }

      const conn = hostConnectionRef.current;
      if (!conn || conn.open === false) {
        setStatus("Host connection is not available", true);
        return;
      }

      updateMultiplayer((prev) => ({ ...prev, submittingAction: true }));
      safeSend(conn, {
        type: "player_action",
        protocolVersion: PROTOCOL_VERSION,
        actorIndex: session.localPlayerIndex,
        command,
        label,
      });
      setStatus("Waiting for host to sync action");
    },
    [sequenceHostedAction, setStatus, updateMultiplayer]
  );

  useEffect(
    () => () => {
      teardownPeer();
    },
    [teardownPeer]
  );

  return {
    multiplayer,
    canStartHostedMatch: canHostedMatchStart(multiplayer),
    createLobby,
    joinLobby,
    leaveLobby,
    startHostedMatch,
    updateLobbyDeck,
    submitMultiplayerCommand,
  };
}
