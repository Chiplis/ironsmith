import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { usePeerLobby } from "/src/hooks/usePeerLobby.js";

function createFakeGame() {
  let matchConfig = null;
  let perspective = 0;
  let actionSequence = 0;
  let battlefield = [];
  const syncEvents = [];

  const buildState = () => {
    const playerNames = matchConfig?.playerNames?.length
      ? matchConfig.playerNames
      : ["Host", "Guest"];
    const actor = actionSequence % Math.max(1, playerNames.length);
    return {
      snapshot_id: actionSequence,
      perspective,
      players: playerNames.map((name, index) => ({
        id: index,
        name,
        life: Number(matchConfig?.startingLife || 20),
        hand_cards: [],
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
        actions: [
          {
            index: 0,
            kind: "test_priority_action",
            action_ref: {
              kind: "test_priority_action",
              actor,
              sequence: actionSequence,
            },
          },
        ],
      },
    };
  };

  return {
    validateMatchConfig: async () => ({ valid: true, issues: [] }),
    startMatch: async (config) => {
      matchConfig = JSON.parse(JSON.stringify(config || {}));
      actionSequence = 0;
      battlefield = [];
      return buildState();
    },
    setPerspective: async (index) => {
      perspective = Number(index || 0);
      return buildState();
    },
    uiState: async () => buildState(),
    dispatch: async () => {
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
    cancelDecision: async () => buildState(),
    exportSyncCheckpoint: async () => ({
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
    setState,
    setStatus,
    applySyncedCommand,
  });

  useEffect(() => {
    window.__peerHarness = {
      ready: true,
      createLobby: lobby.createLobby,
      joinLobby: lobby.joinLobby,
      leaveLobby: lobby.leaveLobby,
      startHostedMatch: lobby.startHostedMatch,
      submitMultiplayerCommand: lobby.submitMultiplayerCommand,
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
