// End-of-match disclosure (verified multiplayer).
//
// Some claims about hidden cards can only be checked once the card is opened:
// a face-down spell's public cast kind (morph, megamorph, disguise), a card an
// owner withheld from a forced "reveal every matching card", a hand choice the
// owner answered with fewer cards than the rules asked for. The engine keeps
// them in its obligation ledger (game_state/hidden_hand_choices.rs) and checks
// them whenever the card is opened during the game. Cards that stay hidden
// until the end are checked here: when the match ends each player publicly
// opens, in one signed message, every hidden card it still owns in its hand
// and every face-down spell or permanent it owns (never its library). Every
// peer verifies the openings against the deck commitments and the obligation
// ledger. A mismatch is reported as a detected cheat; a player that never
// sends its disclosure is reported as "disclosure missing". Neither blocks the
// other players, and nothing here touches the public checkpoint hash.

import {
  PROTOCOL_VERSION,
  cloneMultiplayerPayload,
  emitSyncFailureNotice,
  isVerifiedMultiplayerSecurityMode,
  mergeAuditOpenings,
  playerNameForIndex,
  recordPeerSyncPerf,
  sessionSecurityMode,
  signAuditPayload,
  toErrorMessage,
  useCallback,
  useEffect,
  useRef,
  verifyAuditPayload,
} from "./shared.js";

export const END_OF_MATCH_DISCLOSURE_DOMAIN = "ironsmith-end-of-match-disclosure-v1";
export const END_OF_MATCH_DISCLOSURE_TIMEOUT_MS = 120_000;

export const DISCLOSURE_STATUS_PENDING = "pending";
export const DISCLOSURE_STATUS_VERIFIED = "verified";
export const DISCLOSURE_STATUS_CHEAT = "cheat_detected";
export const DISCLOSURE_STATUS_MISSING = "missing";

function playerLeftGame(uiState, playerIndex) {
  const player = (uiState?.players || []).find((entry) =>
    Number(entry?.id ?? entry?.index) === Number(playerIndex)
  );
  return Boolean(
    player?.has_lost
    || player?.hasLost
    || player?.has_left_game
    || player?.hasLeftGame
  );
}

// Whether `playerIndex`'s part of the match is over on this engine: the game
// ended, or that player lost / left (its objects were removed and snapshotted
// for disclosure, CR 800.4a).
export function disclosureDueForPlayer(uiState, playerIndex) {
  return Boolean(uiState?.game_over) || playerLeftGame(uiState, playerIndex);
}

export function endOfMatchDisclosurePayload({ matchId, player, openings }) {
  return {
    domain: END_OF_MATCH_DISCLOSURE_DOMAIN,
    matchId: String(matchId || ""),
    player: Number(player),
    openings: cloneMultiplayerPayload(Array.isArray(openings) ? openings : []),
  };
}

export function usePeerLobbyEndOfMatchDisclosure(base, servicesRef) {
  const { gameRef, stateRef, multiplayerRef, setStatus } = base;
  const sentForMatchRef = useRef("");
  const pendingDisclosuresRef = useRef(new Map());
  const processedDisclosuresRef = useRef(new Set());
  const missingTimerRef = useRef(null);
  const missingTimerMatchRef = useRef("");

  const services = () => servicesRef.current || {};

  const setDisclosureStatus = useCallback((matchId, playerIndex, status, reason = "") => {
    const updateMultiplayer = services().updateMultiplayer;
    if (typeof updateMultiplayer !== "function") return;
    updateMultiplayer((prev) => {
      const current = prev.endOfMatchDisclosure?.matchId === matchId
        ? prev.endOfMatchDisclosure
        : { matchId, byPlayer: {} };
      const existing = current.byPlayer?.[playerIndex];
      // A final verdict is never downgraded (a late "missing" timer must not
      // hide a detected cheat, and a verified disclosure stays verified).
      if (
        existing
        && existing.status !== DISCLOSURE_STATUS_PENDING
        && status === DISCLOSURE_STATUS_MISSING
      ) {
        return prev;
      }
      return {
        ...prev,
        endOfMatchDisclosure: {
          ...current,
          byPlayer: {
            ...(current.byPlayer || {}),
            [playerIndex]: {
              status,
              reason: String(reason || ""),
              name: playerNameForIndex(prev.players, playerIndex),
            },
          },
        },
      };
    });
  // servicesRef is a stable ref; services() reads it at call time.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const verifiedMode = () => isVerifiedMultiplayerSecurityMode(sessionSecurityMode(multiplayerRef.current));

  // Build, sign and send the local disclosure once per match.
  const sendLocalDisclosure = useCallback(async () => {
    const session = multiplayerRef.current;
    const currentGame = gameRef.current;
    const svc = services();
    if (
      !verifiedMode()
      || !session.matchStarted
      || !currentGame
      || typeof currentGame.endOfMatchDisclosureRequirements !== "function"
    ) {
      return;
    }
    const matchId = String(svc.currentAuditMatchId?.() || "");
    if (!matchId || sentForMatchRef.current === matchId) return;
    sentForMatchRef.current = matchId;
    const localSeat = Number(svc.resolveLocalCryptoPlayerIndex?.());
    if (!Number.isInteger(localSeat) || localSeat < 0) return;
    try {
      const requirements = await currentGame.endOfMatchDisclosureRequirements(localSeat);
      const list = Array.isArray(requirements) ? requirements : [];
      const openingOptions = {
        requirements: list,
        timing: "post",
        forceZiffleOpeningProof: true,
      };
      const openings = list.length === 0
        ? []
        : mergeAuditOpenings(
          await svc.buildLocalOpeningsForCommand({}, list, openingOptions),
          await svc.buildLocalRequirementOpeningsForRequirements(list, openingOptions)
        );
      const payload = endOfMatchDisclosurePayload({ matchId, player: localSeat, openings });
      const { keyPair } = await svc.ensureAuditIdentity();
      const disclosure = {
        ...payload,
        signatureAlgorithm: "ecdsa-p256-sha256",
        signature: await signAuditPayload(keyPair, payload),
      };
      const message = {
        type: "end_of_match_disclosure",
        protocolVersion: PROTOCOL_VERSION,
        senderIndex: localSeat,
        disclosure,
      };
      for (const player of session.players || []) {
        if (Number(player?.index) === localSeat) continue;
        const peerId = svc.routePeerIdForPlayer?.(player);
        if (!peerId || peerId === session.localPeerId) continue;
        svc.sendDirectPeerMessage?.(peerId, message);
      }
      setDisclosureStatus(matchId, localSeat, DISCLOSURE_STATUS_VERIFIED, "sent");
      recordPeerSyncPerf("end_of_match_disclosure:sent", {
        player: localSeat,
        cards: list.length,
        openings: openings.length,
      });
    } catch (err) {
      sentForMatchRef.current = "";
      recordPeerSyncPerf("end_of_match_disclosure:send_failed", { error: toErrorMessage(err) });
      setStatus?.(`End-of-match disclosure failed: ${toErrorMessage(err)}`, true);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [setDisclosureStatus]);

  const verifyDisclosure = useCallback(async (disclosure) => {
    const svc = services();
    const currentGame = gameRef.current;
    const matchId = String(disclosure?.matchId || "");
    const player = Number(disclosure?.player);
    const payload = endOfMatchDisclosurePayload(disclosure || {});
    try {
      if (payload.domain !== END_OF_MATCH_DISCLOSURE_DOMAIN) {
        throw new Error("End-of-match disclosure has the wrong domain");
      }
      const publicKey = await svc.importCachedAuditPublicKey(svc.publicKeyForAuditSigner(player));
      const validSignature = await verifyAuditPayload(publicKey, payload, disclosure.signature || "");
      if (!validSignature) {
        throw new Error("End-of-match disclosure signature is invalid");
      }
      // Deck-manifest commitments and ziffle position proofs.
      await svc.verifyAuditOpeningsAgainstManifests(payload.openings, {});
      // Bind each opening to the hidden card it must open and check that
      // card's pending claims (face-down cast kinds, withheld cards).
      const result = await currentGame.verifyEndOfMatchDisclosure(player, payload.openings);
      const violations = Array.isArray(result?.violations) ? result.violations : [];
      const missing = Array.isArray(result?.missing) ? result.missing : [];
      if (violations.length > 0) {
        throw new Error(violations.join("; "));
      }
      if (missing.length > 0) {
        throw new Error(
          `End-of-match disclosure omits ${missing.length} hidden card`
          + `${missing.length === 1 ? "" : "s"} (objects ${missing.join(", ")})`
        );
      }
      setDisclosureStatus(matchId, player, DISCLOSURE_STATUS_VERIFIED);
      recordPeerSyncPerf("end_of_match_disclosure:verified", {
        player,
        openings: payload.openings.length,
      });
    } catch (err) {
      const reason = toErrorMessage(err);
      const name = playerNameForIndex(multiplayerRef.current.players, player);
      setDisclosureStatus(matchId, player, DISCLOSURE_STATUS_CHEAT, reason);
      recordPeerSyncPerf("end_of_match_disclosure:cheat_detected", { player, reason });
      emitSyncFailureNotice("Cheat detected", `End-of-match disclosure from ${name}: ${reason}`);
      setStatus?.(`Cheat detected from ${name}: ${reason}`, true);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [setDisclosureStatus]);

  // Verify every stashed disclosure whose sender's part of the match is over
  // on this engine (its final action has been applied here).
  const processPendingDisclosures = useCallback(async () => {
    const uiState = stateRef.current;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.verifyEndOfMatchDisclosure !== "function") return;
    for (const [key, disclosure] of [...pendingDisclosuresRef.current.entries()]) {
      if (!disclosureDueForPlayer(uiState, disclosure.player)) continue;
      pendingDisclosuresRef.current.delete(key);
      if (processedDisclosuresRef.current.has(key)) continue;
      processedDisclosuresRef.current.add(key);
      await verifyDisclosure(disclosure);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [verifyDisclosure]);

  const handleEndOfMatchDisclosureMessage = useCallback(async (message) => {
    const disclosure = message?.disclosure;
    if (!disclosure || typeof disclosure !== "object" || !verifiedMode()) return;
    const player = Number(disclosure.player);
    const localSeat = Number(services().resolveLocalCryptoPlayerIndex?.());
    if (!Number.isInteger(player) || player < 0 || player === localSeat) return;
    if (message.senderIndex != null && Number(message.senderIndex) !== player) return;
    const matchId = String(services().currentAuditMatchId?.() || "");
    if (!matchId || String(disclosure.matchId || "") !== matchId) return;
    const key = `${matchId}:${player}`;
    if (processedDisclosuresRef.current.has(key) || pendingDisclosuresRef.current.has(key)) return;
    pendingDisclosuresRef.current.set(key, cloneMultiplayerPayload(disclosure));
    setDisclosureStatus(matchId, player, DISCLOSURE_STATUS_PENDING, "received");
    await processPendingDisclosures();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [processPendingDisclosures, setDisclosureStatus]);

  // Once the match is over here: send the local disclosure, verify what has
  // arrived, and report every player still silent after the timeout as
  // "disclosure missing".
  const onRuntimeState = useCallback(async (uiState) => {
    const session = multiplayerRef.current;
    if (!verifiedMode() || !session.matchStarted) return;
    const localSeat = Number(services().resolveLocalCryptoPlayerIndex?.());
    if (Number.isInteger(localSeat) && disclosureDueForPlayer(uiState, localSeat)) {
      await sendLocalDisclosure();
    }
    await processPendingDisclosures();
    if (!uiState?.game_over) return;
    const matchId = String(services().currentAuditMatchId?.() || "");
    if (!matchId || missingTimerMatchRef.current === matchId) return;
    if (missingTimerRef.current) window.clearTimeout(missingTimerRef.current);
    missingTimerMatchRef.current = matchId;
    const players = (session.players || []).map((player) => Number(player?.index));
    for (const player of players) {
      if (player === localSeat) continue;
      const existing = session.endOfMatchDisclosure?.matchId === matchId
        ? session.endOfMatchDisclosure.byPlayer?.[player]
        : null;
      if (!existing) setDisclosureStatus(matchId, player, DISCLOSURE_STATUS_PENDING, "awaiting");
    }
    missingTimerRef.current = window.setTimeout(() => {
      missingTimerRef.current = null;
      const latest = multiplayerRef.current;
      if (String(services().currentAuditMatchId?.() || "") !== matchId) return;
      for (const player of players) {
        if (player === localSeat) continue;
        const key = `${matchId}:${player}`;
        if (processedDisclosuresRef.current.has(key)) continue;
        const entry = latest.endOfMatchDisclosure?.matchId === matchId
          ? latest.endOfMatchDisclosure.byPlayer?.[player]
          : null;
        if (entry && entry.status !== DISCLOSURE_STATUS_PENDING) continue;
        setDisclosureStatus(
          matchId,
          player,
          DISCLOSURE_STATUS_MISSING,
          pendingDisclosuresRef.current.has(key)
            ? "disclosure arrived before the match ended here"
            : "no end-of-match disclosure received"
        );
        recordPeerSyncPerf("end_of_match_disclosure:missing", { player });
      }
    }, END_OF_MATCH_DISCLOSURE_TIMEOUT_MS);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [processPendingDisclosures, sendLocalDisclosure, setDisclosureStatus]);

  useEffect(() => () => {
    if (missingTimerRef.current) {
      window.clearTimeout(missingTimerRef.current);
      missingTimerRef.current = null;
    }
  }, []);

  // A new match (rematch) starts with a clean slate.
  const resetEndOfMatchDisclosure = useCallback(() => {
    if (missingTimerRef.current) {
      window.clearTimeout(missingTimerRef.current);
      missingTimerRef.current = null;
    }
    pendingDisclosuresRef.current.clear();
  }, []);

  return {
    handleEndOfMatchDisclosureMessage,
    onEndOfMatchRuntimeState: onRuntimeState,
    resetEndOfMatchDisclosure,
  };
}
