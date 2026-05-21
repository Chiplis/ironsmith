import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { usePeerLobby } from "/src/hooks/usePeerLobby.js";
import { UI_NOTICE_EVENT } from "/src/lib/ui-notices.js";

const ZIFFLE_PUBLIC_OPEN_OBJECT_ID = 4242;
const ZIFFLE_PUBLIC_OPEN_POSITION = 53;
const ZIFFLE_PUBLIC_OPEN_ORIGINAL_SLOT = 7;
const ZIFFLE_PUBLIC_OPEN_CARD = "Mystical Tutor";
const ZIFFLE_OPENED_LAND_OBJECT_ID = 4343;
const ZIFFLE_OPENED_LAND_POSITION = 54;
const ZIFFLE_OPENED_LAND_ORIGINAL_SLOT = 7;
const ZIFFLE_OPENED_LAND_WRONG_REVEAL_SLOT = 13;
const ZIFFLE_OPENED_LAND_CARD = "Island";
const POST_PUBLIC_OPEN_OBJECT_ID = 4444;
const POST_PUBLIC_OPEN_ORIGINAL_SLOT = 13;
const POST_PUBLIC_OPEN_CARD = "Mystical Tutor";
const LATE_PUBLIC_OPEN_OBJECT_ID = 4545;
const LATE_PUBLIC_OPEN_ORIGINAL_SLOT = 17;
const LATE_PUBLIC_OPEN_CARD = "Mountain";

function createFakeGame() {
  let matchConfig = null;
  let perspective = 0;
  let actionSequence = 0;
  let nextObjectId = 1000;
  let battlefield = [];
  let addedHands = new Map();
  let zifflePublicOpenRevealed = false;
  let ziffleOpenedLandRevealed = false;
  let postPublicOpenDispatched = false;
  let latePublicOpenDispatched = false;
  let omitOwnerOpenedLandPosition = false;
  let failOpenedLandExport = false;
  let includeOpenedLandInCheckpointHand = false;
  const syncEvents = [];
  const instrumentation = {
    exportPublicAuditCheckpoint: 0,
    exportSyncCheckpoint: 0,
    revealHiddenSlot: 0,
    postPublicOpenRevealSlot: 0,
    latePublicOpenRevealSlot: 0,
  };

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
      {
        index: 1,
        kind: "ziffle_shuffle_action",
        action_ref: {
          kind: "ziffle_shuffle_action",
          actor,
          sequence: actionSequence,
        },
      },
      {
        index: 2,
        kind: "ziffle_public_open_action",
        action_ref: {
          kind: "ziffle_public_open_action",
          actor,
          sequence: actionSequence,
        },
      },
      {
        index: 3,
        kind: "play_land",
        object_id: ZIFFLE_OPENED_LAND_OBJECT_ID,
        action_ref: {
          kind: "play_land",
          land_id: ZIFFLE_OPENED_LAND_OBJECT_ID,
          actor,
          sequence: actionSequence,
        },
      },
      {
        index: 4,
        kind: "post_public_open_action",
        action_ref: {
          kind: "post_public_open_action",
          actor,
          sequence: actionSequence,
        },
      },
      {
        index: 5,
        kind: "late_public_open_action",
        action_ref: {
          kind: "late_public_open_action",
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
    const cryptoRequirements = latePublicOpenDispatched
      ? [
          {
            id: "fixture-late-public-open",
            type: "public_open",
            owner: 0,
            zone: "hidden_zone",
            slot: LATE_PUBLIC_OPEN_ORIGINAL_SLOT,
            objectId: LATE_PUBLIC_OPEN_OBJECT_ID,
            card: LATE_PUBLIC_OPEN_CARD,
          },
        ]
      : [];
    return {
      snapshot_id: actionSequence,
      perspective,
      crypto_requirements: cryptoRequirements,
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

  const zifflePublicOpenCommitment = () => String(
    matchConfig?.hiddenDeckManifests?.[0]?.slotCommitments?.find((entry) =>
      Number(entry?.slot) === ZIFFLE_PUBLIC_OPEN_POSITION
    )?.commitment || ""
  );
  const ziffleCommitmentForPosition = (position) => String(
    matchConfig?.hiddenDeckManifests?.[0]?.slotCommitments?.find((entry) =>
      Number(entry?.slot) === Number(position)
    )?.commitment || ""
  );
  const privateManifestForOwner = (owner) => {
    try {
      return Object.keys(window.localStorage || {})
        .filter((key) => key.startsWith("ironsmith.auditDeckManifest.v1"))
        .map((key) => JSON.parse(window.localStorage.getItem(key)))
        .find((manifest) => Number(manifest?.owner) === Number(owner)) || null;
    } catch {
      return null;
    }
  };
  const privateCommitmentForSlot = (owner, slot) => String(
    privateManifestForOwner(owner)?.slotSecrets?.find((entry) =>
      Number(entry?.slot) === Number(slot)
    )?.commitment || ""
  );

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
    ziffleBuildRevealToken: async ({ context = "fixture", cardPosition = 0, player = 0 }) => ({
      player: Number(player || 0),
      cardPosition: Number(cardPosition || 0),
      tokenHex: fakeHex(`reveal:${context}:${cardPosition}:${player}`),
      proofHex: fakeHex(`reveal-proof:${context}:${cardPosition}:${player}`),
    }),
    ziffleRevealCard: async ({ cardPosition = 0 }) => {
      if (Number(cardPosition) !== ZIFFLE_PUBLIC_OPEN_POSITION) {
        if (Number(cardPosition) === ZIFFLE_OPENED_LAND_POSITION) {
          return {
            originalSlot: ZIFFLE_OPENED_LAND_ORIGINAL_SLOT,
            cardPosition: ZIFFLE_OPENED_LAND_POSITION,
          };
        }
        throw new Error(`unexpected ziffle reveal position ${cardPosition}`);
      }
      return {
        originalSlot: ZIFFLE_PUBLIC_OPEN_ORIGINAL_SLOT,
        cardPosition: ZIFFLE_PUBLIC_OPEN_POSITION,
      };
    },
    previewCryptoRequirements: async (command) => {
      if (command?.action_ref?.kind === "ziffle_public_open_action") {
        return [
          {
            id: "fixture-ziffle-public-open",
            type: "public_open",
            owner: 0,
            zone: "library",
            slot: ZIFFLE_PUBLIC_OPEN_POSITION,
            objectId: ZIFFLE_PUBLIC_OPEN_OBJECT_ID,
            card: ZIFFLE_PUBLIC_OPEN_CARD,
          },
        ];
      }
      if (command?.action_ref?.kind === "post_public_open_action") {
        return [
          {
            id: "fixture-post-public-open",
            type: "public_open",
            owner: 0,
            zone: "hidden_zone",
            slot: POST_PUBLIC_OPEN_ORIGINAL_SLOT,
            objectId: POST_PUBLIC_OPEN_OBJECT_ID,
            card: POST_PUBLIC_OPEN_CARD,
          },
        ];
      }
      if (
        command?.action_ref?.kind === "play_land"
        && Number(command?.action_ref?.land_id) === ZIFFLE_OPENED_LAND_OBJECT_ID
      ) {
        return [
          {
            id: "fixture-opened-land-public-open",
            type: "public_open",
            owner: 0,
            zone: "battlefield",
            slot: ZIFFLE_OPENED_LAND_ORIGINAL_SLOT,
            objectId: ZIFFLE_OPENED_LAND_OBJECT_ID,
            card: ZIFFLE_OPENED_LAND_CARD,
          },
        ];
      }
      if (command?.action_ref?.kind !== "ziffle_shuffle_action") return [];
      return [
        {
          id: "fixture-live-library-shuffle",
          type: "verifiable_shuffle",
          owner: 0,
          zone: "library",
          beforeOrder: [0, 1, 2],
          afterOrder: [2, 1, 0],
        },
      ];
    },
    applyVerifiedHiddenLibraryShuffle: async () => buildState(),
    revealHiddenSlot: async ({ owner, slot, commitment }) => {
      instrumentation.revealHiddenSlot += 1;
      if (
        Number(owner) === 0
        && Number(slot) === ZIFFLE_PUBLIC_OPEN_POSITION
        && String(commitment || "") === zifflePublicOpenCommitment()
      ) {
        return buildState();
      }
      if (
        Number(owner) === 0
        && Number(slot) === POST_PUBLIC_OPEN_ORIGINAL_SLOT
        && postPublicOpenDispatched
      ) {
        instrumentation.postPublicOpenRevealSlot += 1;
        return buildState();
      }
      if (
        Number(owner) === 0
        && Number(slot) === LATE_PUBLIC_OPEN_ORIGINAL_SLOT
        && latePublicOpenDispatched
      ) {
        instrumentation.latePublicOpenRevealSlot += 1;
        return buildState();
      }
      throw new Error("hidden card commitment does not match reveal");
    },
    revealHiddenPosition: async ({ owner, position, originalSlot, positionCommitment }) => {
      if (
        Number(owner) === 0
        && Number(position) === ZIFFLE_PUBLIC_OPEN_POSITION
        && Number(originalSlot) === ZIFFLE_PUBLIC_OPEN_ORIGINAL_SLOT
        && String(positionCommitment || "") === zifflePublicOpenCommitment()
      ) {
        zifflePublicOpenRevealed = true;
        return buildState();
      }
      if (
        Number(owner) === 0
        && Number(position) === ZIFFLE_OPENED_LAND_POSITION
        && Number(originalSlot) === ZIFFLE_OPENED_LAND_ORIGINAL_SLOT
        && String(positionCommitment || "") === ziffleCommitmentForPosition(ZIFFLE_OPENED_LAND_POSITION)
      ) {
        ziffleOpenedLandRevealed = true;
        return buildState();
      }
      throw new Error("hidden ziffle position commitment does not match reveal");
    },
    exportHiddenCardOpening: async (objectId) => {
      if (Number(objectId) !== ZIFFLE_PUBLIC_OPEN_OBJECT_ID) {
        if (Number(objectId) === POST_PUBLIC_OPEN_OBJECT_ID) {
          return {
            owner: 0,
            slot: POST_PUBLIC_OPEN_ORIGINAL_SLOT,
            objectId: POST_PUBLIC_OPEN_OBJECT_ID,
            object_id: POST_PUBLIC_OPEN_OBJECT_ID,
            commitment: privateCommitmentForSlot(0, POST_PUBLIC_OPEN_ORIGINAL_SLOT),
            card: POST_PUBLIC_OPEN_CARD,
          };
        }
        if (Number(objectId) === LATE_PUBLIC_OPEN_OBJECT_ID) {
          return {
            owner: 0,
            slot: LATE_PUBLIC_OPEN_ORIGINAL_SLOT,
            objectId: LATE_PUBLIC_OPEN_OBJECT_ID,
            object_id: LATE_PUBLIC_OPEN_OBJECT_ID,
            commitment: privateCommitmentForSlot(0, LATE_PUBLIC_OPEN_ORIGINAL_SLOT),
            card: LATE_PUBLIC_OPEN_CARD,
          };
        }
        if (Number(objectId) !== ZIFFLE_OPENED_LAND_OBJECT_ID) {
          throw new Error(`unknown hidden object ${objectId}`);
        }
        if (failOpenedLandExport) {
          throw new Error("object is not tracked as a hidden card");
        }
        return {
          owner: 0,
          slot: ZIFFLE_OPENED_LAND_ORIGINAL_SLOT,
          objectId: ZIFFLE_OPENED_LAND_OBJECT_ID,
          object_id: ZIFFLE_OPENED_LAND_OBJECT_ID,
          commitment: privateCommitmentForSlot(0, ZIFFLE_OPENED_LAND_ORIGINAL_SLOT),
          card: ZIFFLE_OPENED_LAND_CARD,
        };
      }
      return {
        owner: 0,
        slot: ZIFFLE_PUBLIC_OPEN_POSITION,
        objectId: ZIFFLE_PUBLIC_OPEN_OBJECT_ID,
        object_id: ZIFFLE_PUBLIC_OPEN_OBJECT_ID,
        commitment: zifflePublicOpenCommitment(),
        card: ZIFFLE_PUBLIC_OPEN_CARD,
      };
    },
    startMatch: async (config) => {
      matchConfig = JSON.parse(JSON.stringify(config || {}));
      actionSequence = 0;
      nextObjectId = 1000;
      battlefield = [];
      addedHands = new Map();
      zifflePublicOpenRevealed = false;
      ziffleOpenedLandRevealed = false;
      postPublicOpenDispatched = false;
      latePublicOpenDispatched = false;
      failOpenedLandExport = false;
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
      if (
        command?.type === "priority_action"
        && command?.action_ref?.kind === "play_land"
        && Number(command?.action_ref?.land_id) === ZIFFLE_OPENED_LAND_OBJECT_ID
        && Number(perspective) !== 0
        && !ziffleOpenedLandRevealed
      ) {
        throw new Error("hidden card commitment does not match reveal");
      }
      if (
        command?.type === "priority_action"
        && command?.action_ref?.kind === "post_public_open_action"
      ) {
        postPublicOpenDispatched = true;
      }
      if (
        command?.type === "priority_action"
        && command?.action_ref?.kind === "late_public_open_action"
      ) {
        latePublicOpenDispatched = true;
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
    exportSyncCheckpoint: async () => {
      instrumentation.exportSyncCheckpoint += 1;
      const openedLandVisibleToLocal = Number(perspective) === 0 || ziffleOpenedLandRevealed;
      const openedLandHiddenCard = openedLandVisibleToLocal
        ? {
            owner: 0,
            slot: ZIFFLE_OPENED_LAND_ORIGINAL_SLOT,
            commitment: privateCommitmentForSlot(0, ZIFFLE_OPENED_LAND_ORIGINAL_SLOT),
            ...(!omitOwnerOpenedLandPosition
              ? {
                  publicSlot: ZIFFLE_OPENED_LAND_POSITION,
                  publicCommitment: ziffleCommitmentForPosition(ZIFFLE_OPENED_LAND_POSITION),
                }
              : {}),
          }
        : {
            owner: 0,
            slot: ZIFFLE_OPENED_LAND_POSITION,
            commitment: ziffleCommitmentForPosition(ZIFFLE_OPENED_LAND_POSITION),
          };
      return {
        matchConfig: JSON.parse(JSON.stringify(matchConfig || {})),
        perspective,
        actionSequence,
        players: (matchConfig?.playerNames || ["Host", "Guest"]).map((_, index) => ({
          id: index,
          hand: includeOpenedLandInCheckpointHand && index === 0
            ? [ZIFFLE_OPENED_LAND_OBJECT_ID]
            : [],
          library: index === 0 ? [ZIFFLE_PUBLIC_OPEN_OBJECT_ID, ZIFFLE_OPENED_LAND_OBJECT_ID] : [],
        })),
        objects: [
          {
            id: ZIFFLE_PUBLIC_OPEN_OBJECT_ID,
            owner: 0,
            zone: "library",
            hiddenCard: {
              owner: 0,
              slot: ZIFFLE_PUBLIC_OPEN_POSITION,
              commitment: zifflePublicOpenCommitment(),
            },
          },
          {
            id: ZIFFLE_OPENED_LAND_OBJECT_ID,
            owner: 0,
            zone: "hand",
            hiddenCard: openedLandHiddenCard,
          },
        ],
        battlefield: JSON.parse(JSON.stringify(battlefield)),
      };
    },
    exportRedactedSyncCheckpoint: async () => ({
      matchConfig: JSON.parse(JSON.stringify(matchConfig || {})),
      perspective,
      actionSequence,
      battlefield: JSON.parse(JSON.stringify(battlefield)),
    }),
    exportPublicAuditCheckpoint: async () => {
      instrumentation.exportPublicAuditCheckpoint += 1;
      return {
        players: JSON.parse(JSON.stringify(matchConfig?.playerNames || [])),
        startingLife: Number(matchConfig?.startingLife || 20),
        format: String(matchConfig?.format || "normal"),
        actionSequence,
        battlefield: JSON.parse(JSON.stringify(battlefield)),
      };
    },
    importSyncCheckpoint: async (checkpoint, perspectiveIndex = 0) => {
      matchConfig = JSON.parse(JSON.stringify(checkpoint?.matchConfig || {}));
      perspective = Number(perspectiveIndex ?? checkpoint?.perspective ?? 0);
      actionSequence = Number(checkpoint?.actionSequence || 0);
      battlefield = JSON.parse(JSON.stringify(checkpoint?.battlefield || []));
      addedHands = new Map();
      zifflePublicOpenRevealed = false;
      ziffleOpenedLandRevealed = false;
      postPublicOpenDispatched = false;
      latePublicOpenDispatched = false;
      failOpenedLandExport = false;
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
    instrumentation: () => ({ ...instrumentation }),
    resetInstrumentation: () => {
      instrumentation.exportPublicAuditCheckpoint = 0;
      instrumentation.exportSyncCheckpoint = 0;
      instrumentation.revealHiddenSlot = 0;
      instrumentation.postPublicOpenRevealSlot = 0;
      instrumentation.latePublicOpenRevealSlot = 0;
    },
    setOmitOwnerOpenedLandPosition: (enabled) => {
      omitOwnerOpenedLandPosition = Boolean(enabled);
    },
    setFailOpenedLandExport: (enabled) => {
      failOpenedLandExport = Boolean(enabled);
    },
    setIncludeOpenedLandInCheckpointHand: (enabled) => {
      includeOpenedLandInCheckpointHand = Boolean(enabled);
    },
  };
}

function Harness() {
  const [visibleState, setVisibleState] = useState(null);
  const statusEventsRef = useRef([]);
  const noticeEventsRef = useRef([]);
  const syncEventsRef = useRef([]);
  const autoPassEnabledRef = useRef(false);
  const autoPassAttemptRef = useRef("");
  const applyDelayMsRef = useRef(0);
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

  useEffect(() => {
    const listener = (event) => {
      noticeEventsRef.current.push(event.detail || null);
    };
    window.addEventListener(UI_NOTICE_EVENT, listener);
    return () => window.removeEventListener(UI_NOTICE_EVENT, listener);
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
    if (applyDelayMsRef.current > 0) {
      await new Promise((resolve) => {
        window.setTimeout(resolve, applyDelayMsRef.current);
      });
    }
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
      setApplyDelay: (delayMs = 0) => {
        applyDelayMsRef.current = Math.max(0, Number(delayMs) || 0);
      },
      setOmitOwnerOpenedLandPosition: (enabled) => {
        game.setOmitOwnerOpenedLandPosition(enabled);
      },
      setFailOpenedLandExport: (enabled) => {
        game.setFailOpenedLandExport(enabled);
      },
      setIncludeOpenedLandInCheckpointHand: (enabled) => {
        game.setIncludeOpenedLandInCheckpointHand(enabled);
      },
      resetInstrumentation: game.resetInstrumentation,
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
      snapshot: async () => {
        const auditTranscript =
          typeof lobby.exportAuditTranscript === "function"
            ? await lobby.exportAuditTranscript({ includeLiveCheckpoint: false })
            : null;

        return {
          multiplayer: {
            ...lobby.multiplayer,
            players: (lobby.multiplayer.players || []).map((player) => ({
              ...player,
              routePeerId: typeof lobby.routePeerIdForPlayer === "function"
                ? lobby.routePeerIdForPlayer(player)
                : (player.currentPeerId || player.peerId || ""),
            })),
          },
          canStartHostedMatch: lobby.canStartHostedMatch,
          visibleState,
          statusEvents: [...statusEventsRef.current],
          noticeEvents: [...noticeEventsRef.current],
          syncEvents: [...syncEventsRef.current, ...game.syncEvents()],
          instrumentation: game.instrumentation(),
          perfEvents: Array.isArray(window.__ironsmithPerfEvents)
            ? window.__ironsmithPerfEvents.slice(-100)
            : [],
          auditTranscript,
        };
      },
    };
  });

  return React.createElement("pre", { id: "state" }, JSON.stringify({
    multiplayer: lobby.multiplayer,
    visibleState,
  }));
}

createRoot(document.getElementById("root")).render(React.createElement(Harness));
