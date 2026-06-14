export const MULTIPLAYER_SECURITY_TRUSTED = "trusted";
export const MULTIPLAYER_SECURITY_VERIFIED = "verified";

export function normalizeMultiplayerSecurityMode(value, fallback = MULTIPLAYER_SECURITY_TRUSTED) {
  const normalized = String(value || "").trim().toLowerCase();
  if (normalized === MULTIPLAYER_SECURITY_VERIFIED) return MULTIPLAYER_SECURITY_VERIFIED;
  if (normalized === MULTIPLAYER_SECURITY_TRUSTED) return MULTIPLAYER_SECURITY_TRUSTED;
  return fallback;
}

export function isVerifiedMultiplayerSecurityMode(value) {
  return normalizeMultiplayerSecurityMode(value) === MULTIPLAYER_SECURITY_VERIFIED;
}

export function isTrustedMultiplayerSecurityMode(value) {
  return normalizeMultiplayerSecurityMode(value) === MULTIPLAYER_SECURITY_TRUSTED;
}
