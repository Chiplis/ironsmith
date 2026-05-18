export const MAX_SUPPORTED_ZIFFLE_DECK_COUNT = 100;
export const ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD = 2000;
export const MAX_ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS =
  MAX_SUPPORTED_ZIFFLE_DECK_COUNT * ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD;

export function isSupportedZiffleDeckCount(count) {
  const normalized = Number(count);
  return (
    Number.isSafeInteger(normalized)
    && normalized >= 2
    && normalized <= MAX_SUPPORTED_ZIFFLE_DECK_COUNT
  );
}

function ziffleDeckCountLimit(ceremonyOrDeckCount) {
  const deckCount = Number(
    ceremonyOrDeckCount && typeof ceremonyOrDeckCount === "object"
      ? ceremonyOrDeckCount.deckCount
      : ceremonyOrDeckCount
  );
  return isSupportedZiffleDeckCount(deckCount)
    ? deckCount
    : MAX_SUPPORTED_ZIFFLE_DECK_COUNT;
}

function isInvalidZifflePositionInput(rawPosition) {
  return (
    rawPosition == null
    || typeof rawPosition === "boolean"
    || (typeof rawPosition === "string" && rawPosition.trim() === "")
  );
}

export function normalizeZiffleCardPositions(cardPositions, ceremonyOrDeckCount = null, options = {}) {
  const label = String(options.label || "Ziffle reveal-token request");
  const rawPositions = Array.isArray(cardPositions)
    ? cardPositions
    : (cardPositions == null && options.allowEmpty ? [] : [cardPositions]);
  const deckCount = ziffleDeckCountLimit(ceremonyOrDeckCount);
  const positions = [];
  const seen = new Set();
  for (const rawPosition of rawPositions) {
    if (isInvalidZifflePositionInput(rawPosition)) {
      throw new Error(`${label} contains an invalid card position`);
    }
    const position = Number(rawPosition);
    if (!Number.isSafeInteger(position) || position < 0) {
      throw new Error(`${label} contains an invalid card position`);
    }
    if (position >= deckCount) {
      throw new Error(`${label} position ${position} is outside deck count ${deckCount}`);
    }
    if (!seen.has(position)) {
      seen.add(position);
      positions.push(position);
    }
  }
  if (positions.length === 0 && !options.allowEmpty) {
    throw new Error(`${label} is missing a card position`);
  }
  return positions;
}

export function ziffleRevealTokenTimeoutMs(cardCount = 1, ceremonyOrDeckCount = null) {
  const deckCount = ziffleDeckCountLimit(ceremonyOrDeckCount);
  const requestedCount = Math.ceil(Number(cardCount));
  const normalizedCount = Number.isFinite(requestedCount)
    ? Math.max(1, requestedCount)
    : 1;
  return Math.min(normalizedCount, deckCount) * ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD;
}

export function pendingActionIntentHardTimeoutMs(protocolResponseTimeoutMs) {
  const requestedProtocolTimeout = Number(protocolResponseTimeoutMs);
  const normalizedProtocolTimeout = Number.isFinite(requestedProtocolTimeout)
    ? Math.max(1, requestedProtocolTimeout)
    : 1;
  return MAX_ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS + normalizedProtocolTimeout;
}
