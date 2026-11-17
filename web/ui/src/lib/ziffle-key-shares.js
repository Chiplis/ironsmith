// Ziffle seat-key shares.
//
// Every hidden-card reveal needs a reveal token from every roster key, so a
// seat that is gone for good would stop the remaining seats from drawing. In a
// match of three or more seats each seat therefore deals, at match start, a
// verifiable sharing of its Ziffle secret key to the other seats. The deals
// are part of the signed match genesis. All of the other seats together can
// rebuild a key; nothing less can.
//
// A share is sealed to its holder's Ziffle public key, so a deal is public:
// the holder opens its share with its own key, and whoever rebuilds a
// forfeited seat's key can also open the shares that seat was holding. That is
// what keeps a four-seat match going through a second forfeit.
//
// Once a seat is forfeited in the accepted transcript, the remaining seats
// exchange their opened shares of its key (each is checked against the
// dealer's public commitments), rebuild the key and produce its reveal tokens
// themselves. Two seats lost before either's shares were exchanged cannot be
// recovered: the remaining seats alone must never be able to read a hand.
import {
  canonicalJson,
  importAuditPublicKey,
  signAuditPayload,
  verifyAuditPayload,
} from "./multiplayer-audit.js";

export const ZIFFLE_KEY_SHARE_DEAL_DOMAIN = "ironsmith-ziffle-key-share-deal-v1";
export const ZIFFLE_KEY_SHARE_MIN_PLAYERS = 3;

// A two-seat match ends when a seat leaves, so it deals nothing.
export function ziffleKeySharesRequired(playerCount) {
  return Number(playerCount) >= ZIFFLE_KEY_SHARE_MIN_PLAYERS;
}

// Every other seat must take part in rebuilding a key.
export function ziffleKeyShareThreshold(playerCount) {
  return ziffleKeySharesRequired(playerCount) ? Number(playerCount) - 1 : 0;
}

function sortedKeys(keys) {
  return (Array.isArray(keys) ? keys : [])
    .map((key) => ({
      player: Number(key?.player),
      publicKeyHex: String(key?.publicKeyHex || ""),
    }))
    .sort((left, right) => left.player - right.player);
}

export function normalizeZiffleKeyShareDeal(deal) {
  return {
    dealer: Number(deal?.dealer),
    threshold: Number(deal?.threshold),
    commitmentsHex: (Array.isArray(deal?.commitmentsHex) ? deal.commitmentsHex : []).map(String),
    shares: (Array.isArray(deal?.shares) ? deal.shares : [])
      .map((share) => ({
        holder: Number(share?.holder),
        ephemeralHex: String(share?.ephemeralHex || ""),
        cipherHex: String(share?.cipherHex || ""),
      }))
      .sort((left, right) => left.holder - right.holder),
    signature: String(deal?.signature || ""),
  };
}

export function normalizeZiffleKeyShareDeals(deals) {
  return (Array.isArray(deals) ? deals : [])
    .map(normalizeZiffleKeyShareDeal)
    .sort((left, right) => left.dealer - right.dealer);
}

function dealStatement(context, deal) {
  const { signature: _signature, ...body } = normalizeZiffleKeyShareDeal(deal);
  return { domain: ZIFFLE_KEY_SHARE_DEAL_DOMAIN, context: String(context || ""), ...body };
}

// Input for the verifier's `ziffleDealKeyShares`.
export function ziffleKeyShareDealRequest({ context, keys, dealer, keyPair, entropyHex }) {
  const roster = sortedKeys(keys);
  return {
    context: String(context || ""),
    dealer: Number(dealer),
    publicKeyHex: String(keyPair?.publicKeyHex || ""),
    secretKeyHex: String(keyPair?.secretKeyHex || ""),
    holders: roster.filter((key) => key.player !== Number(dealer)),
    threshold: ziffleKeyShareThreshold(roster.length),
    entropyHex: String(entropyHex || ""),
  };
}

export async function signZiffleKeyShareDeal(keyPair, context, deal, cryptoImpl = globalThis.crypto) {
  const normalized = normalizeZiffleKeyShareDeal(deal);
  return {
    ...normalized,
    signature: await signAuditPayload(keyPair, dealStatement(context, normalized), cryptoImpl),
  };
}

// Checks everything about the deals that needs no secret: one deal per seat,
// each bound to its dealer's public key, sealed to exactly the other seats,
// with the all-others threshold and the dealer's signature. Whether a sealed
// share is honest is checked by its holder (`openZiffleKeyShare`).
export async function verifyZiffleKeyShareDeals({
  deals,
  keys,
  context,
  auditPublicKeyForSeat,
}, cryptoImpl = globalThis.crypto) {
  const roster = sortedKeys(keys);
  const normalized = normalizeZiffleKeyShareDeals(deals);
  if (!ziffleKeySharesRequired(roster.length)) {
    if (normalized.length > 0) throw new Error("A two-player match does not deal ziffle key shares");
    return normalized;
  }
  if (normalized.length !== roster.length) {
    throw new Error("The match must contain one ziffle key-share deal per player");
  }
  const threshold = ziffleKeyShareThreshold(roster.length);
  for (let index = 0; index < roster.length; index += 1) {
    const seat = roster[index];
    const deal = normalized[index];
    const label = `ziffle key-share deal from player ${seat.player + 1}`;
    if (deal.dealer !== seat.player) throw new Error(`The match is missing the ${label}`);
    if (deal.threshold !== threshold || deal.commitmentsHex.length !== threshold) {
      throw new Error(`The ${label} must need every other player to rebuild the key`);
    }
    if (deal.commitmentsHex[0] !== seat.publicKeyHex) {
      throw new Error(`The ${label} is not bound to that player's ziffle key`);
    }
    const holders = roster.filter((key) => key.player !== seat.player).map((key) => key.player);
    if (
      deal.shares.length !== holders.length
      || deal.shares.some((share, shareIndex) =>
        share.holder !== holders[shareIndex] || !share.ephemeralHex || !share.cipherHex)
    ) {
      throw new Error(`The ${label} must hold one sealed share for every other player`);
    }
    const publicKeyHex = String(auditPublicKeyForSeat(seat.player) || "");
    if (!deal.signature || !publicKeyHex) {
      throw new Error(`The ${label} is not signed`);
    }
    const publicKey = await importAuditPublicKey(publicKeyHex, cryptoImpl);
    if (!await verifyAuditPayload(publicKey, dealStatement(context, deal), deal.signature, cryptoImpl)) {
      throw new Error(`The ${label} has an invalid signature`);
    }
  }
  return normalized;
}

function dealFor(deals, dealer) {
  const deal = normalizeZiffleKeyShareDeals(deals).find((entry) => entry.dealer === Number(dealer));
  if (!deal) throw new Error(`No ziffle key-share deal from player ${Number(dealer) + 1}`);
  return deal;
}

// Opens the share `dealer` sealed to `holder`, with the holder's secret key
// (its own, or one rebuilt from shares). Throws when the dealer cheated.
export async function openZiffleKeyShare(backend, { deals, context, dealer, holder, holderSecretKeyHex }) {
  const deal = dealFor(deals, dealer);
  const share = deal.shares.find((entry) => entry.holder === Number(holder));
  if (!share) throw new Error(`Player ${Number(holder) + 1} holds no share of player ${Number(dealer) + 1}'s key`);
  const opened = await backend.ziffleOpenKeyShare({
    context: String(context || ""),
    dealer: deal.dealer,
    commitmentsHex: deal.commitmentsHex,
    share,
    holderSecretKeyHex: String(holderSecretKeyHex || ""),
  });
  return { holder: Number(opened.holder), shareHex: String(opened.shareHex || "") };
}

// Keeps the opened shares that really belong to `dealer`'s deal.
export async function validZiffleKeyShares(backend, { deals, dealer, shares }) {
  const deal = dealFor(deals, dealer);
  const holders = new Set(deal.shares.map((share) => share.holder));
  const valid = new Map();
  for (const share of Array.isArray(shares) ? shares : []) {
    const holder = Number(share?.holder);
    const shareHex = String(share?.shareHex || "");
    if (!holders.has(holder) || !shareHex || valid.has(holder)) continue;
    const check = await backend.ziffleVerifyKeyShare({
      commitmentsHex: deal.commitmentsHex,
      holder,
      shareHex,
    });
    if (check?.valid) valid.set(holder, { holder, shareHex });
  }
  return [...valid.values()];
}

export function missingZiffleKeyShareHolders(deals, dealer, shares) {
  const known = new Set((Array.isArray(shares) ? shares : []).map((share) => Number(share?.holder)));
  return dealFor(deals, dealer).shares.map((share) => share.holder).filter((holder) => !known.has(holder));
}

// Rebuilds `dealer`'s secret key from one opened share per holder.
export async function recoverZiffleSecretKey(backend, { deals, keys, dealer, shares }) {
  const deal = dealFor(deals, dealer);
  const key = sortedKeys(keys).find((entry) => entry.player === Number(dealer));
  if (!key) throw new Error(`No ziffle key for player ${Number(dealer) + 1}`);
  const recovered = await backend.ziffleRecoverKey({
    publicKeyHex: key.publicKeyHex,
    commitmentsHex: deal.commitmentsHex,
    shares: (Array.isArray(shares) ? shares : []).map((share) => ({
      holder: Number(share.holder),
      shareHex: String(share.shareHex || ""),
    })),
  });
  return { publicKeyHex: key.publicKeyHex, secretKeyHex: String(recovered.secretKeyHex || "") };
}

export function ziffleKeyShareDealsEqual(left, right) {
  return canonicalJson(normalizeZiffleKeyShareDeal(left)) === canonicalJson(normalizeZiffleKeyShareDeal(right));
}
