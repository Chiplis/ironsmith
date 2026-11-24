// Dependency-free payload helpers shared by the Verified/Trusted session
// modules. They used to live in hooks/peer-lobby/shared.js, which also pulls in
// React, PeerJS and the relay socket; shared.js now re-exports these.

import { canonicalJson } from "../multiplayer-audit.js";
import {
  MULTIPLAYER_SECURITY_TRUSTED,
  normalizeMultiplayerSecurityMode,
} from "../multiplayer-security.js";

export function cloneMultiplayerPayload(value) {
  if (value == null) return value;
  return JSON.parse(JSON.stringify(value));
}

export function canonicalMultiplayerPayload(value) {
  return canonicalJson(cloneMultiplayerPayload(value));
}

export function sessionSecurityMode(session, fallback = MULTIPLAYER_SECURITY_TRUSTED) {
  return normalizeMultiplayerSecurityMode(session?.securityMode, fallback);
}

export function toErrorMessage(err, fallback = "Action rejected") {
  const message = String(err?.message || err || "").trim();
  return message || fallback;
}

export function playerNameForIndex(players, playerIndex) {
  const target = (players || []).find((player) => Number(player?.index) === Number(playerIndex));
  return String(target?.name || `Player ${Number(playerIndex) + 1}`);
}

// Runs `task` after every earlier task on the queue held in `queueRef.current`
// (a promise), whether or not those succeeded.
export function enqueueAsync(queueRef, task) {
  const next = queueRef.current.catch(() => undefined).then(task);
  queueRef.current = next.catch(() => undefined);
  return next;
}
