import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { usePeerLobby } from "/src/hooks/usePeerLobby.js";

function createFakeGame() {
  let matchConfig = null;
  let perspective = 0;
  let actionSequence = 0;
  let nextObjectId = 1000;
  let battlefield = [];
  let addedHands = new Map();
  const syncEvents = [];

  const fakeHex = (label) => Array.from(String(label || "fixture"))
    .map((char) => char.charCodeAt(0).toString(16).padStart(2, "0"))
    .join("")
    .padEnd(64, "0")
    .slice(0, 64);

  const buildState = () => {
    const playerNames = matchConfig?.playerNames?.length
      ? matchConfig.playerNames
      : ["Host", "Guest"];
    const actor = actionSequence % Math.max(1, playerNames.length);
    const handCardsByPlayer = playerNames.map((_, index) =>
      (addedHands.get(index) || []).map((card) => ({ ...card }))
    );
    const actions = [
      {
        index: 0,
        kind: "test_priority_action",
        action_ref: {
          kind: "test_priority_action",
          actor,
          sequence: actionSequence,
        },
      },
    ];
    for (const card of handCardsByPlayer[actor] || []) {
      actions.push({
        index: actions.length,
        kind: "cast_spell",
        object_id: card.id,
        action_ref: {
          kind: "cast_spell",
          spell_id: card.id,
          from_zone: "hand",
          casting_method: { kind: "normal" },
        },
      });
    }
    return {
      snapshot_id: actionSequence,
      perspective,
      players: playerNames.map((name, index) => ({
        id: index,
        name,
        life: Number(matchConfig?.startingLife || 20),
        hand_cards: handCardsByPlayer[index] || [],
        graveyard_cards: [],
        exile_cards: [],
        command_cards: [],
        sideboard_cards: [],
        battlefield: index === 0
          ? battlefield.map((card) => ({ ...card }))
          : [],
      })),
      decision: {
        kind: "priority",
        player: actor,
        actions,
      },
    };
  };

  const stableStringify = (value) => {
    if (value === null || typeof value !== "object") return JSON.stringify(value);
    if (Array.isArray(value)) return `[${value.map(stableStringify).join(",")}]`;
    const keys = Object.keys(value).sort();
    return `{${keys.map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(",")}}`;
  };

  const actionRefAvailable = (command) => {
    if (command?.type !== "priority_action" || !command.action_ref) return true;
    const state = buildState();
    return (state.decision?.actions || []).some((action) =>
      stableStringify(action.action_ref) === stableStringify(command.action_ref)
    );
  };

  return {
    validateMatchConfig: async () => ({ valid: true, issues: [] }),
    ziffleKeygen: async ({ context = "fixture", deckCount = 60 }) => ({
      publicKeyHex: fakeHex(`public:${context}:${deckCount}`),
      secretKeyHex: fakeHex(`secret:${context}:${deckCount}`),
      ownershipProofHex: fakeHex(`proof:${context}:${deckCount}`),
    }),
    ziffleBuildShuffleStep: async ({ context = "fixture", deckCount = 60, shuffler = 0 }) => ({
      shuffler: Number(shuffler || 0),
      deckHex: fakeHex(`deck:${context}:${deckCount}:${shuffler}`),
      proofHex: fakeHex(`shuffle:${context}:${deckCount}:${shuffler}`),
    }),
    ziffleVerifyShuffle: async ({ context = "fixture", deckCount = 60, steps = [] }) => ({
      deckHash: fakeHex(`verified:${context}:${deckCount}:${steps.length}`),
    }),
    startMatch: async (config) => {
      matchConfig = JSON.parse(JSON.stringify(config || {}));
      actionSequence = 0;
      nextObjectId = 1000;
      battlefield = [];
      addedHands = new Map();
      return buildState();
    },
    setPerspective: async (index) => {
      perspective = Number(index || 0);
      return buildState();
    },
    uiState: async () => buildState(),
    dispatch: async (command) => {
      if (!actionRefAvailable(command)) {
        throw new Error(`invalid priority action ref: ${JSON.stringify(command?.action_ref || null)}`);
      }
      actionSequence += 1;
      battlefield = [
        ...battlefield,
        {
          id: actionSequence,
          stable_id: actionSequence,
          name: `Synced Permanent ${actionSequence}`,
          tapped: false,
          count: 1,
          member_ids: [actionSequence],
          member_stable_ids: [actionSequence],
          lane: "creatures",
          oracle_text: "Fixture permanent created by a synced action.",
          counters: [],
        },
      ];
      return buildState();
    },
    addCardToZone: async (playerIndex, cardName, zone = "hand") => {
      if (String(zone || "hand") !== "hand") {
        throw new Error("fixture only supports adding to hand");
      }
      const owner = Number(playerIndex || 0);
      const card = {
        id: nextObjectId,
        stable_id: nextObjectId,
        name: String(cardName || "Fixture Card"),
        tapped: false,
        count: 1,
        member_ids: [nextObjectId],
        member_stable_ids: [nextObjectId],
      };
      nextObjectId += 1;
      addedHands.set(owner, [...(addedHands.get(owner) || []), card]);
      return card.id;
    },
    cancelDecision: async () => buildState(),
    exportSyncCheckpoint: async () => ({
      matchConfig: JSON.parse(JSON.stringify(matchConfig || {})),
      perspective,
      actionSequence,
      battlefield: JSON.parse(JSON.stringify(battlefield)),
    }),
    exportRedactedSyncCheckpoint: async () => ({
      matchConfig: JSON.parse(JSON.stringify(matchConfig || {})),
      perspective,
      actionSequence,
      battlefield: JSON.parse(JSON.stringify(battlefield)),
    }),
    importSyncCheckpoint: async (checkpoint, perspectiveIndex = 0) => {
      matchConfig = JSON.parse(JSON.stringify(checkpoint?.matchConfig || {}));
      perspective = Number(perspectiveIndex ?? checkpoint?.perspective ?? 0);
      actionSequence = Number(checkpoint?.actionSequence || 0);
      battlefield = JSON.parse(JSON.stringify(checkpoint?.battlefield || []));
      addedHands = new Map();
      const nextState = buildState();
      syncEvents.push({
        type: "sync_checkpoint_import",
        snapshotId: nextState?.snapshot_id ?? null,
        perspective: nextState?.perspective ?? null,
        battlefieldCount: nextState?.players?.[0]?.battlefield?.length ?? 0,
      });
      return nextState;
    },
    syncEvents: () => [...syncEvents],
  };
}

function Harness() {
  const [visibleState, setVisibleState] = useState(null);
  const statusEventsRef = useRef([]);
  const syncEventsRef = useRef([]);
  const autoPassEnabledRef = useRef(false);
  const autoPassAttemptRef = useRef("");
  const game = useMemo(() => createFakeGame(), []);

  const setState = useCallback((nextState) => {
    setVisibleState(nextState);
  }, []);

  const setStatus = useCallback((message, isError = false) => {
    statusEventsRef.current.push({
      message: String(message || ""),
      isError: Boolean(isError),
    });
  }, []);

  const applySyncedCommand = useCallback(async (command, label = "", syncContext = null) => {
    const nextState = command?.type === "cancel_decision"
      ? await game.cancelDecision()
      : await game.dispatch(command);
    syncEventsRef.current.push({
      type: "synced_command",
      command,
      label,
      syncContext,
      snapshotId: nextState?.snapshot_id ?? null,
      perspective: nextState?.perspective ?? null,
    });
    setVisibleState(nextState);
    return nextState;
  }, [game]);

  const lobby = usePeerLobby({
    game,
    state: visibleState,
    setState,
    setStatus,
    applySyncedCommand,
  });

  useEffect(() => {
    if (!autoPassEnabledRef.current) return;
    if (!lobby.multiplayer.matchStarted || lobby.multiplayer.submittingAction) return;
    if (!visibleState?.decision || Number(visibleState.decision.player) !== Number(lobby.multiplayer.localPlayerIndex)) return;
    const action = (visibleState.decision.actions || [])[0];
    if (!action?.action_ref) return;
    const key = [
      lobby.multiplayer.lastAppliedSequence,
      visibleState.snapshot_id,
      visibleState.decision.player,
      action.index,
    ].join("|");
    if (autoPassAttemptRef.current === key) return;
    autoPassAttemptRef.current = key;
    void lobby.submitMultiplayerCommand({
      type: "priority_action",
      action_ref: action.action_ref,
    }, "harness auto-pass");
  }, [
    lobby,
    lobby.multiplayer.lastAppliedSequence,
    lobby.multiplayer.localPlayerIndex,
    lobby.multiplayer.matchStarted,
    lobby.multiplayer.submittingAction,
    visibleState,
  ]);

  useEffect(() => {
    window.__peerHarness = {
      ready: true,
      createLobby: lobby.createLobby,
      joinLobby: lobby.joinLobby,
      leaveLobby: lobby.leaveLobby,
      startHostedMatch: lobby.startHostedMatch,
      submitMultiplayerCommand: lobby.submitMultiplayerCommand,
      submitMultiplayerAddCardCheat: lobby.submitMultiplayerAddCardCheat,
      setAutoPass: (enabled) => {
        autoPassEnabledRef.current = Boolean(enabled);
        autoPassAttemptRef.current = "";
      },
      silentlyAddCard: async ({ playerIndex, cardName, zone = "hand" } = {}) => {
        await game.addCardToZone(
          Number(playerIndex ?? lobby.multiplayer.localPlayerIndex ?? 0),
          String(cardName || "Black Lotus"),
          zone
        );
        const nextState = await game.uiState();
        setVisibleState(nextState);
        return nextState;
      },
      snapshot: () => ({
        multiplayer: lobby.multiplayer,
        canStartHostedMatch: lobby.canStartHostedMatch,
        visibleState,
        statusEvents: [...statusEventsRef.current],
        syncEvents: [...syncEventsRef.current, ...game.syncEvents()],
      }),
    };
  });

  return React.createElement("pre", { id: "state" }, JSON.stringify({
    multiplayer: lobby.multiplayer,
    visibleState,
  }));
}

createRoot(document.getElementById("root")).render(React.createElement(Harness));
