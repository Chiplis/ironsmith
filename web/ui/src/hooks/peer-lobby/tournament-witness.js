// Tournament matches: the relay-hosted witness certifies who sits in each
// seat and settles liveness disputes. Silence toward the witness loses the
// match; any answer is forwarded to the claimant, and an answer the
// claimant's engine contradicts becomes signed dispute evidence.
import { useRef } from "react";
import {
  canonicalJson,
  sha256Hex,
  signAuditPayload,
  verifyAuditPayload,
  importAuditPublicKey,
  matchGenesisPayload,
} from "../../lib/multiplayer-audit.js";
import {
  WITNESS_DISPUTE_TYPE,
  WITNESS_DOMAINS,
  WITNESS_FORFEIT_REASON,
  WITNESS_MAX_ANSWER_BYTES,
  WITNESS_MAX_REQUEST_BYTES,
  answerPayload,
  byteLength,
  claimPayload,
  genesisRequestPayload,
  verifyCertificate,
  verifyForfeitCertificate,
  verifyGenesisAttestation,
  verifySignedWitnessPayload,
} from "../../lib/tournament/witness-protocol.js";
import { isTrustedWitnessKey, storedTournamentCertificate } from "../../lib/tournament/credentials.js";
import {
  cloneMultiplayerPayload,
  playerNameForIndex,
  reindexPlayers,
  resolveLocalPlayerIndex,
  toErrorMessage,
} from "./shared.js";

const PROTOCOL_REQUEST_ANSWERERS = {
  crypto_material_request: "answerCryptoMaterialRequest",
  ziffle_reveal_token_request: "answerZiffleRevealTokenRequest",
  ziffle_shuffle_step_request: "answerZiffleShuffleStepRequest",
  rng_commit_request: "answerRngCommitRequest",
  rng_reveal_request: "answerRngRevealRequest",
};

export function usePeerLobbyTournamentWitness(base, servicesRef) {
  const {
    peerRef, multiplayerRef, matchStartPayloadRef, auditKeyPairRef, auditStateHashRef,
    stateRef, liveAuditTranscriptRef, setStatus,
  } = base;
  // Host only: certificates guests presented when they joined, by peer id.
  const guestCertificatesRef = useRef(new Map());
  // Challenges this browser opened, answered or saw decided, by challenge id.
  const challengesRef = useRef(new Map());

  const services = () => servicesRef.current;
  const matchTournament = () => matchStartPayloadRef.current?.tournament || null;
  const matchPlayers = () => reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);

  function sessionWitnessKey(session = multiplayerRef.current) {
    return String(session.tournament?.witnessPublicKey || "");
  }

  async function localTournamentCertificate(tournamentId) {
    const { publicKey } = await services().ensureAuditIdentity();
    const entry = storedTournamentCertificate(tournamentId, publicKey);
    if (!entry) throw new Error("Redeem your tournament invite in this browser before joining its matches");
    return entry;
  }

  // Host: a guest must present a certificate for this lobby's tournament whose
  // name is the name it joined with. Key binding is checked at match start,
  // once the guest has announced its audit key.
  async function acceptGuestCertificate(peerId, signed, joinName) {
    const session = multiplayerRef.current;
    const tournamentId = session.tournament?.tournamentId;
    if (!tournamentId) return;
    const cert = await verifyCertificate(signed, sessionWitnessKey(session), { tournamentId });
    if (cert.playerName !== joinName) throw new Error("Tournament certificate is for a different player name");
    for (const [otherPeer, other] of guestCertificatesRef.current) {
      if (otherPeer !== peerId && other.payload.inviteId === cert.inviteId) throw new Error("That tournament invite is already seated here");
    }
    guestCertificatesRef.current.set(peerId, cloneMultiplayerPayload(signed));
  }

  function forgetGuestCertificate(peerId) {
    guestCertificatesRef.current.delete(peerId);
  }

  // Host, after the final genesis signature: the witness attests which
  // certified keys occupy which seats of exactly this genesis.
  async function attachWitnessGenesis(payload) {
    const session = multiplayerRef.current;
    const tournament = session.tournament;
    if (!tournament?.tournamentId) return payload;
    const players = reindexPlayers(payload.players);
    const own = await localTournamentCertificate(tournament.tournamentId);
    const certificates = players.map((player) => {
      if (player.peerId === session.localPeerId) return own.certificate;
      const cert = guestCertificatesRef.current.get(player.peerId);
      if (!cert) throw new Error(`${player.name || "A player"} has not presented a tournament certificate`);
      if (cert.payload.auditPublicKey !== player.auditPublicKey) throw new Error(`${player.name || "A player"}'s certificate is for another browser key`);
      return cert;
    });
    const request = genesisRequestPayload({
      genesisHash: String(payload.genesis?.payloadHash || ""),
      matchId: payload.auditMatchId,
      lobbyId: payload.lobbyId,
      tournamentId: tournament.tournamentId,
      format: payload.format,
      securityMode: payload.securityMode,
      hostSeat: 0,
      players: players.map((player) => ({ seat: Number(player.index), peerId: player.peerId, name: player.name, auditPublicKey: player.auditPublicKey })),
    });
    const hostSignature = await signAuditPayload(auditKeyPairRef.current, request);
    const result = await peerRef.current.witness("genesis", { request, hostSignature, certificates });
    payload.tournament = {
      tournamentId: tournament.tournamentId,
      tournamentName: tournament.tournamentName || own.tournamentName,
      witnessPublicKey: String(result.witnessPublicKey || "").toLowerCase(),
    };
    payload.witnessGenesis = result.attestation;
    await verifyMatchWitness(payload, session);
    return payload;
  }

  // Every peer at match start. A lobby that was a tournament must stay one,
  // bound to the same tournament and a trusted witness key.
  async function verifyMatchWitness(payload, session = multiplayerRef.current) {
    const expected = session.tournament?.tournamentId;
    if (!expected) {
      if (payload.tournament) throw new Error("Host turned a casual lobby into a tournament match");
      return null;
    }
    const tournament = payload.tournament;
    if (tournament?.tournamentId !== expected) throw new Error("Match start is not bound to this lobby's tournament");
    const key = String(tournament.witnessPublicKey || "").toLowerCase();
    if (!isTrustedWitnessKey(key, { advertised: sessionWitnessKey(session) })) throw new Error("Match start uses an untrusted tournament witness");
    const genesisHash = await sha256Hex(canonicalJson(matchGenesisPayload(payload)));
    if (genesisHash !== String(payload.genesis?.payloadHash || "")) throw new Error("Match genesis hash does not match its payload");
    return verifyGenesisAttestation(payload.witnessGenesis, key, {
      genesisHash,
      matchId: payload.auditMatchId,
      lobbyId: payload.lobbyId,
      tournamentId: expected,
      format: payload.format,
      securityMode: "verified",
      players: reindexPlayers(payload.players).map((player) => ({
        seat: Number(player.index), peerId: player.peerId, name: player.name, auditPublicKey: player.auditPublicKey,
      })),
    });
  }

  function recordWitnessEvidence(entry) {
    if (!liveAuditTranscriptRef.current) return;
    const witness = liveAuditTranscriptRef.current.witnessEvidence || [];
    liveAuditTranscriptRef.current = {
      ...liveAuditTranscriptRef.current,
      witnessEvidence: [...witness, cloneMultiplayerPayload(entry)],
    };
  }

  // Replaces the peer-vote forfeit paths in tournament matches. Returns false
  // when this is not a tournament match so callers fall back to their own flow.
  async function openWitnessChallenge({ accusedSeat, reason, request = null }) {
    const tournament = matchTournament();
    if (!tournament) return false;
    const session = multiplayerRef.current;
    const claimantSeat = resolveLocalPlayerIndex(session);
    if (claimantSeat == null || Number(accusedSeat) === Number(claimantSeat)) return false;
    for (const record of challengesRef.current.values()) {
      if (record.role === "claimant" && record.status === "open" && record.challenge.accusedSeat === Number(accusedSeat)) return true;
    }
    const claim = claimPayload({
      matchId: matchStartPayloadRef.current.auditMatchId,
      tournamentId: tournament.tournamentId,
      claimantSeat,
      accusedSeat,
      reason,
      basisSequence: Number(session.lastAppliedSequence || 0),
      headStateHash: String(auditStateHashRef.current || ""),
      request: request && byteLength(request) <= WITNESS_MAX_REQUEST_BYTES ? cloneMultiplayerPayload(request) : null,
      claimedAt: Date.now(),
    });
    const claimSignature = await signAuditPayload(auditKeyPairRef.current, claim);
    let result;
    try {
      result = await peerRef.current.witness("challenge", { claim, claimSignature });
    } catch (err) {
      setStatus(`Could not reach the tournament witness: ${toErrorMessage(err)}`, true);
      return true;
    }
    const challenge = await verifySignedWitnessPayload(result.challenge, tournament.witnessPublicKey, WITNESS_DOMAINS.challenge, "Witness challenge");
    challengesRef.current.set(challenge.challengeId, { role: "claimant", status: "open", challenge, signed: result.challenge, claim });
    const name = playerNameForIndex(matchPlayers(), accusedSeat);
    const seconds = Math.max(1, Math.round((challenge.deadline - Date.now()) / 1000));
    setStatus(`${name} has ${seconds}s to answer the tournament witness, or forfeits.`, true);
    return true;
  }

  async function capturedProtocolResponses(request, claimantPeerId) {
    const answerer = PROTOCOL_REQUEST_ANSWERERS[String(request?.type || "")];
    if (!answerer || typeof services()[answerer] !== "function") return [];
    const responses = [];
    const conn = { peer: claimantPeerId, open: true, send: (payload) => { responses.push(cloneMultiplayerPayload(payload)); return { bytes: 0 }; } };
    try { await services()[answerer](conn, cloneMultiplayerPayload(request)); }
    catch { /* an unauthorized or stale request simply gets no response */ }
    return responses;
  }

  // Accused: honest clients always answer, so only a silent (offline or
  // malicious) seat can lose to a challenge.
  async function answerChallenge(signedChallenge, claim, claimSignature) {
    const tournament = matchTournament();
    const challenge = await verifySignedWitnessPayload(signedChallenge, tournament.witnessPublicKey, WITNESS_DOMAINS.challenge, "Witness challenge");
    const session = multiplayerRef.current;
    const localSeat = resolveLocalPlayerIndex(session);
    if (challenge.matchId !== matchStartPayloadRef.current.auditMatchId || challenge.accusedSeat !== localSeat) return;
    if (challengesRef.current.get(challenge.challengeId)?.status === "answered") return;
    const players = matchPlayers();
    const claimant = players.find((player) => Number(player.index) === challenge.claimantSeat);
    const claimValid = claim && claimant
      && await verifyAuditPayload(await importAuditPublicKey(claimant.auditPublicKey), claim, String(claimSignature || ""));
    if (!claimValid || (await sha256Hex(canonicalJson(claim))) !== challenge.claimHash) throw new Error("Witness challenge does not carry its signed claim");
    // Our own clock agrees the claim is fair: stay silent and let the witness
    // decide it, rather than stall a match we have already lost on time.
    if (challenge.reason === "match_clock_timeout" && ownClockExpired(localSeat)) return;
    let responses = claim.request ? await capturedProtocolResponses(claim.request, claimant.peerId) : [];
    const decision = stateRef.current?.decision;
    const build = (list) => answerPayload({
      challengeId: challenge.challengeId,
      matchId: challenge.matchId,
      accusedSeat: localSeat,
      headSequence: Number(session.lastAppliedSequence || 0),
      headStateHash: String(auditStateHashRef.current || ""),
      awaitingSeat: decision?.player ?? null,
      responses: list,
      actions: [],
      answeredAt: Date.now(),
    });
    let answer = build(responses);
    while (byteLength(answer) > WITNESS_MAX_ANSWER_BYTES && responses.length) {
      responses = responses.slice(0, -1);
      answer = build(responses);
    }
    const answerSignature = await signAuditPayload(auditKeyPairRef.current, answer);
    await peerRef.current.witness("answer", { answer, answerSignature });
    challengesRef.current.set(challenge.challengeId, { role: "accused", status: "answered", challenge, signed: signedChallenge, claim, answer });
    setStatus(`Answered a tournament witness challenge from ${claimant.name || "your opponent"}.`);
  }

  function ownClockExpired(seat) {
    const timer = services().updateMatchClockForState?.(stateRef.current);
    return Boolean(timer?.enabled && timer.expired && Number(timer.activePlayerIndex) === Number(seat));
  }

  // Claimant: the accused answered. Catch up if it is ahead; if it claims to
  // wait on us while our engine says it owes the decision, the signed answer
  // is dispute evidence and play cannot honestly continue.
  async function receiveAnswer(record, signedAnswer) {
    const tournament = matchTournament();
    const players = matchPlayers();
    const accused = players.find((player) => Number(player.index) === record.challenge.accusedSeat);
    const answer = await verifySignedWitnessPayload(signedAnswer, accused?.auditPublicKey, WITNESS_DOMAINS.answer, "Witness answer");
    if (answer.challengeId !== record.challenge.challengeId) return;
    record.status = "answered";
    record.answer = answer;
    record.answerSignature = signedAnswer.signature;
    const session = multiplayerRef.current;
    const localSeat = resolveLocalPlayerIndex(session);
    for (const response of answer.responses) {
      try { await services().handleWitnessForwardedResponse?.(response); } catch { /* evidence only */ }
    }
    if (answer.headSequence > Number(session.lastAppliedSequence || 0) && session.role === "client") {
      services().requestResync?.("Catching up after the witness answer...");
      return;
    }
    const localDecisionPlayer = stateRef.current?.decision?.player;
    const contradicted = answer.headSequence === Number(session.lastAppliedSequence || 0)
      && answer.awaitingSeat != null
      && Number(answer.awaitingSeat) === Number(localSeat)
      && localDecisionPlayer != null
      && Number(localDecisionPlayer) === record.challenge.accusedSeat;
    if (!contradicted) {
      setStatus(`${accused?.name || "Your opponent"} answered the tournament witness; play continues.`);
      return;
    }
    const dispute = {
      type: WITNESS_DISPUTE_TYPE,
      accusedPlayers: [record.challenge.accusedSeat],
      witnessPublicKey: tournament.witnessPublicKey,
      challenge: record.signed,
      claim: record.claim,
      answer: signedAnswer,
      sequence: answer.headSequence,
    };
    services().markMatchDisputed(
      `${accused?.name || "Your opponent"} told the tournament witness it is waiting on you, but the game is waiting on them. The signed answer is saved as evidence.`,
      { dispute },
    );
  }

  async function receiveForfeit(signedForfeit) {
    const tournament = matchTournament();
    const forfeit = await verifyForfeitCertificate(signedForfeit, tournament.witnessPublicKey, {
      matchId: matchStartPayloadRef.current.auditMatchId,
      tournamentId: tournament.tournamentId,
      claimantSeat: resolveLocalPlayerIndex(multiplayerRef.current),
    });
    const record = challengesRef.current.get(forfeit.challengeId);
    if (record?.status === "forfeit_submitted") return;
    challengesRef.current.set(forfeit.challengeId, { ...(record || {}), role: "claimant", status: "forfeit_submitted", forfeit: signedForfeit });
    recordWitnessEvidence({ type: "witness_forfeit", forfeit: signedForfeit });
    const name = playerNameForIndex(matchPlayers(), forfeit.accusedSeat);
    await services().submitMultiplayerCommand({
      type: "forfeit_player",
      player: forfeit.accusedSeat,
      reason: WITNESS_FORFEIT_REASON,
      witness_forfeit: cloneMultiplayerPayload(signedForfeit),
    }, `${name} forfeited: no answer to the tournament witness`);
  }

  async function handleWitnessEvent(message) {
    if (!matchTournament()) return;
    try {
      if (message?.event === "challenge") {
        await answerChallenge(message.challenge, message.claim, message.claimSignature);
      } else if (message?.event === "answer") {
        const record = challengesRef.current.get(message.challenge?.payload?.challengeId);
        if (record?.role === "claimant") await receiveAnswer(record, message.answer);
      } else if (message?.event === "forfeit") {
        await receiveForfeit(message.forfeit);
      }
    } catch (err) {
      setStatus(`Tournament witness: ${toErrorMessage(err)}`, true);
    }
  }

  // After (re)connecting to the relay: answer anything that arrived while we
  // were away and collect forfeits the witness decided for us.
  async function syncWitnessStatus() {
    const tournament = matchTournament();
    if (!tournament || typeof peerRef.current?.witness !== "function") return;
    let entries;
    try { entries = await peerRef.current.witness("status", { matchId: matchStartPayloadRef.current.auditMatchId }); }
    catch { return; }
    const localSeat = resolveLocalPlayerIndex(multiplayerRef.current);
    for (const entry of entries || []) {
      const challenge = entry?.challenge?.payload;
      if (!challenge) continue;
      if (entry.status === "open" && challenge.accusedSeat === localSeat) {
        await handleWitnessEvent({ event: "challenge", challenge: entry.challenge, claim: entry.claim, claimSignature: entry.claimSignature });
      } else if (entry.status === "forfeited" && entry.forfeit && challenge.claimantSeat === localSeat) {
        await handleWitnessEvent({ event: "forfeit", forfeit: entry.forfeit });
      }
    }
  }

  // Live gate for a received or locally built witness forfeit.
  async function validateWitnessForfeitCommand(command, { actorIndex }) {
    const tournament = matchTournament();
    if (!tournament) throw new Error("Witness forfeits exist only in tournament matches");
    await verifyForfeitCertificate(command.witness_forfeit, tournament.witnessPublicKey, {
      matchId: matchStartPayloadRef.current.auditMatchId,
      tournamentId: tournament.tournamentId,
      accusedSeat: Number(command.player),
      claimantSeat: Number(actorIndex),
    });
  }

  return {
    acceptGuestCertificate,
    attachWitnessGenesis,
    forgetGuestCertificate,
    handleWitnessEvent,
    localTournamentCertificate,
    openWitnessChallenge,
    syncWitnessStatus,
    validateWitnessForfeitCommand,
    verifyMatchWitness,
  };
}
