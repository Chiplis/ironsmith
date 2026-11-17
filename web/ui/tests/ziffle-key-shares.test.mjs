// Seat-key shares with the real verifier: a Verified match of three or four
// seats must keep drawing after seats are forfeited.
import test from "node:test";
import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { readFile } from "node:fs/promises";
import initVerifier, * as verifier from "../../wasm_demo/pkg/verifier.js";
import { createAuditSessionKey, exportAuditPublicKey } from "../src/lib/multiplayer-audit.js";
import {
  missingZiffleKeyShareHolders,
  openZiffleKeyShare,
  recoverZiffleSecretKey,
  signZiffleKeyShareDeal,
  validZiffleKeyShares,
  verifyZiffleKeyShareDeals,
  ziffleKeyShareDealRequest,
} from "../src/lib/ziffle-key-shares.js";
import {
  signZiffleShuffleStep,
  verifyZiffleCeremonyStepSignatures,
} from "../src/lib/ziffle-step-signatures.js";

await initVerifier({
  module_or_path: await readFile(new URL("../../wasm_demo/pkg/verifier_bg.wasm", import.meta.url)),
});

const DECK_COUNT = 10;
const hex = (byte) => byte.toString(16).padStart(2, "0").repeat(32);

async function table(playerCount, context) {
  const seats = [];
  for (let player = 0; player < playerCount; player += 1) {
    const ziffle = verifier.ziffleKeygen({ deckCount: DECK_COUNT, context, entropyHex: hex(0x11 + player) });
    const auditKeyPair = await createAuditSessionKey(webcrypto);
    seats.push({
      player,
      ziffle,
      auditKeyPair,
      auditPublicKey: await exportAuditPublicKey(auditKeyPair, webcrypto),
    });
  }
  const keys = seats.map((seat) => ({
    player: seat.player,
    publicKeyHex: seat.ziffle.publicKeyHex,
    ownershipProofHex: seat.ziffle.ownershipProofHex,
  }));
  const deals = [];
  for (const seat of seats) {
    const dealt = verifier.ziffleDealKeyShares(ziffleKeyShareDealRequest({
      context,
      keys,
      dealer: seat.player,
      keyPair: seat.ziffle,
      entropyHex: hex(0x51 + seat.player),
    }));
    deals.push(await signZiffleKeyShareDeal(seat.auditKeyPair, context, dealt, webcrypto));
  }
  const auditPublicKeyForSeat = (player) => seats[player]?.auditPublicKey || "";
  return { context, seats, keys, deals, auditPublicKeyForSeat };
}

const verifyDeals = ({ context, keys, deals, auditPublicKeyForSeat }, override = {}) =>
  verifyZiffleKeyShareDeals({ deals, keys, context, auditPublicKeyForSeat, ...override }, webcrypto);

// What one remaining seat does to rebuild `dealer`'s key: open what it can
// read itself, then take the shares the other remaining seats release.
async function rebuild({ context, keys, deals }, dealer, secrets, released = []) {
  const known = new Map();
  for (const [holder, holderSecretKeyHex] of secrets) {
    if (!missingZiffleKeyShareHolders(deals, dealer, [...known.values()]).includes(holder)) continue;
    const opened = await openZiffleKeyShare(verifier, { deals, context, dealer, holder, holderSecretKeyHex });
    known.set(opened.holder, opened);
  }
  for (const share of await validZiffleKeyShares(verifier, { deals, dealer, shares: released })) {
    known.set(share.holder, share);
  }
  assert.deepEqual(missingZiffleKeyShareHolders(deals, dealer, [...known.values()]), []);
  return recoverZiffleSecretKey(verifier, { deals, keys, dealer, shares: [...known.values()] });
}

const release = ({ context, deals }, dealer, seat) => openZiffleKeyShare(verifier, {
  deals, context, dealer, holder: seat.player, holderSecretKeyHex: seat.ziffle.secretKeyHex,
});

test("a match of three or more seats binds one honest deal per seat", async () => {
  const match = await table(4, "key-share-genesis");
  assert.equal((await verifyDeals(match)).length, 4);

  const tampered = (change) => {
    const deals = structuredClone(match.deals);
    change(deals);
    return verifyDeals(match, { deals });
  };
  await assert.rejects(tampered((deals) => deals.pop()), /one ziffle key-share deal per player/);
  await assert.rejects(
    tampered((deals) => { deals[1].commitmentsHex[0] = match.keys[2].publicKeyHex; }),
    /not bound to that player's ziffle key/,
  );
  await assert.rejects(
    tampered((deals) => { deals[1].commitmentsHex.pop(); deals[1].threshold -= 1; }),
    /every other player/,
  );
  await assert.rejects(tampered((deals) => deals[1].shares.pop()), /one sealed share for every other player/);
  await assert.rejects(
    tampered((deals) => { deals[1].shares[0].cipherHex = deals[1].shares[1].cipherHex; }),
    /invalid signature/,
  );
  // A swapped sealed share passes the public checks only if re-signed; its
  // holder still catches it on opening.
  const resigned = structuredClone(match.deals);
  resigned[1].shares[0].cipherHex = resigned[1].shares[1].cipherHex;
  resigned[1] = await signZiffleKeyShareDeal(match.seats[1].auditKeyPair, match.context, resigned[1], webcrypto);
  await verifyDeals(match, { deals: resigned });
  await assert.rejects(
    openZiffleKeyShare(verifier, {
      deals: resigned,
      context: match.context,
      dealer: 1,
      holder: 0,
      holderSecretKeyHex: match.seats[0].ziffle.secretKeyHex,
    }),
    /does not match its commitments/,
  );

  // A two-seat match ends when a seat leaves, so it carries no deals.
  const duel = { keys: match.keys.slice(0, 2) };
  await assert.rejects(verifyDeals(match, { ...duel, deals: match.deals.slice(0, 2) }), /two-player match does not deal/);
  assert.deepEqual(await verifyDeals(match, { ...duel, deals: [] }), []);
});

test("remaining seats rebuild forfeited keys and keep revealing through two forfeits", async () => {
  const match = await table(4, "key-share-play");
  const { seats, keys, context } = match;
  let steps = [];
  for (const seat of seats) {
    const step = verifier.ziffleBuildShuffleStep({
      deckCount: DECK_COUNT, context, keys, steps, shuffler: seat.player, entropyHex: hex(0x71 + seat.player),
    });
    steps = [...steps, { shuffler: seat.player, deckHex: step.deckHex, proofHex: step.proofHex }];
  }
  const { deckHash } = verifier.ziffleVerifyShuffle({ deckCount: DECK_COUNT, context, keys, steps });
  const tokensFor = (publicKeyHex, secretKeyHex, cardPositions) => verifier.ziffleBuildRevealTokens({
    deckCount: DECK_COUNT, deckHash, context, keys, steps, cardPositions, publicKeyHex, secretKeyHex,
    entropyHex: hex(0x91),
  });

  // Seat 0 is forfeited. Seat 3 rebuilds its key with what seats 1 and 2 release.
  const fromOne = await release(match, 0, seats[1]);
  const fromTwo = await release(match, 0, seats[2]);
  // Two of the three shares are not enough.
  await assert.rejects(
    recoverZiffleSecretKey(verifier, { ...match, dealer: 0, shares: [fromOne, fromTwo] }),
    /needs 3 shares/,
  );
  const keyZero = await rebuild(match, 0, [[3, seats[3].ziffle.secretKeyHex]], [fromOne, fromTwo]);
  assert.equal(keyZero.secretKeyHex, seats[0].ziffle.secretKeyHex);

  // A forged release is dropped rather than poisoning the rebuild.
  const forged = { holder: 1, shareHex: fromTwo.shareHex };
  assert.deepEqual(await validZiffleKeyShares(verifier, { deals: match.deals, dealer: 0, shares: [forged] }), []);

  // Seat 1 is forfeited next. Its key needs seat 0's share too, which opens
  // with seat 0's rebuilt key.
  const keyOne = await rebuild(
    match,
    1,
    [[3, seats[3].ziffle.secretKeyHex], [0, keyZero.secretKeyHex]],
    [await release(match, 1, seats[2])],
  );
  assert.equal(keyOne.secretKeyHex, seats[1].ziffle.secretKeyHex);

  // Seats 2 and 3 now draw: their own tokens plus tokens made with both rebuilt keys.
  const cardPositions = [0, 4];
  const tokens = [
    ...tokensFor(keyZero.publicKeyHex, keyZero.secretKeyHex, cardPositions),
    ...tokensFor(keyOne.publicKeyHex, keyOne.secretKeyHex, cardPositions),
    ...tokensFor(seats[2].ziffle.publicKeyHex, seats[2].ziffle.secretKeyHex, cardPositions),
    ...tokensFor(seats[3].ziffle.publicKeyHex, seats[3].ziffle.secretKeyHex, cardPositions),
  ];
  assert.deepEqual(tokens.map((token) => token.player), [0, 0, 1, 1, 2, 2, 3, 3]);
  const revealed = verifier.ziffleRevealCards({
    deckCount: DECK_COUNT, deckHash, context, keys, steps, cardPositions, tokens,
  });
  assert.equal(revealed.length, 2);
  // Without the forfeited seats' tokens nothing opens.
  assert.throws(() => verifier.ziffleRevealCards({
    deckCount: DECK_COUNT, deckHash, context, keys, steps, cardPositions,
    tokens: tokens.filter((token) => token.player >= 2),
  }), /missing ziffle reveal token for player 0/);
});

test("a remaining seat shuffles in place of a forfeited seat, and only then", async () => {
  const match = await table(3, "key-share-shuffle");
  const { seats, keys, context } = match;
  const ceremony = { owner: 1, zone: "library", deckCount: DECK_COUNT, context, keyContext: context, keys, steps: [] };
  // Seat 0 is forfeited; seat 1 assembles the shuffle and proxies for it.
  for (const seat of seats) {
    const proxy = seat.player === 0 ? 1 : null;
    const built = verifier.ziffleBuildShuffleStep({
      deckCount: DECK_COUNT, context, keys, steps: ceremony.steps, shuffler: seat.player,
      entropyHex: hex(0xa1 + seat.player),
    });
    const step = {
      shuffler: seat.player,
      deckHex: built.deckHex,
      proofHex: built.proofHex,
      ...(proxy == null ? {} : { proxy }),
    };
    const signer = seats[proxy ?? seat.player];
    step.signature = await signZiffleShuffleStep(signer.auditKeyPair, ceremony, ceremony.steps, step, webcrypto);
    ceremony.steps = [...ceremony.steps, step];
  }
  // The proof system accepts the proxied chain as an ordinary shuffle.
  assert.ok(verifier.ziffleVerifyShuffle({ deckCount: DECK_COUNT, context, keys, steps: ceremony.steps }).deckHash);

  const verify = (forfeitedSeats, steps = ceremony.steps) => verifyZiffleCeremonyStepSignatures(
    { ...ceremony, steps }, match.auditPublicKeyForSeat, webcrypto, { forfeitedSeats },
  );
  await verify([0]);
  // Nobody may shuffle for a seat that is still in the match.
  await assert.rejects(verify([]), /on behalf of a player who can still shuffle/);
  // A forfeited seat cannot be the proxy.
  await assert.rejects(verify([0, 1]), /on behalf of a player who can still shuffle/);
  // Dropping the proxy marker leaves a step its named shuffler never signed.
  const unmarked = ceremony.steps.map(({ proxy: _proxy, ...step }) => step);
  await assert.rejects(verify([0], unmarked), /invalid signature from player 1/);
});
