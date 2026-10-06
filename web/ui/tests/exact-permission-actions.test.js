// Source-authored regression cases; execution is deliberately deferred.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { webcrypto } from "node:crypto";
import {
  castingMethodOrigin, findPriorityActionForCommand, priorityCommandForAction,
  resolveSyncedCommand, serializePriorityCommand,
} from "../src/lib/sync-commands.js";
import { canonicalWireJson } from "../src/lib/wire-json.js";
import { actionRefObjectId } from "../src/lib/sync-object-identity.js";
import { buildPriorityActionGroups } from "../src/lib/priority-action-groups.js";
import {
  canonicalJson, createAuditSessionKey, sha256Hex, signAuditPayload, verifyAuditPayload,
} from "../src/lib/multiplayer-audit.js";

const shared = readFileSync(new URL("../src/hooks/peer-lobby/shared.js", import.meta.url), "utf8");
function declaration(name) {
  const start = shared.indexOf(`export function ${name}(`);
  const end = shared.indexOf("\nexport ", start + 1);
  assert.ok(start >= 0 && end > start, `${name} is present`);
  return shared.slice(start, end).replace(/^export /, "");
}
const domain = shared.match(/export const ACTION_INTENT_DOMAIN = ("[^"]+");/);
assert.ok(domain);
const { collectCommandObjectIds, isFaceDownCastCommand, signedActionIntentPayload } = new Function(
  "castingMethodOrigin", "actionRefObjectId", "ACTION_INTENT_DOMAIN",
  ["cloneMultiplayerPayload", "isFaceDownCastCommand", "isForetellCommand", "collectCommandObjectIds", "signedActionIntentPayload"]
    .map(declaration).join("\n")
    + "\nreturn { collectCommandObjectIds, isFaceDownCastCommand, signedActionIntentPayload };",
)(castingMethodOrigin, actionRefObjectId, JSON.parse(domain[1]));

function method(index = 0, origin = { kind: "play_from", source: 12, zone: "exile", use_alternative: null }) {
  return { kind: "exact_permission", origin, permission: { source: 12, index } };
}
function action(casting_method, index = 3) {
  return { index, kind: "cast_spell", object_id: 42, from_zone: "exile", label: "Cast Example (from exile)",
    action_ref: { kind: "cast_spell", spell_id: 42, from_zone: "exile", casting_method } };
}

test("old unmarked play-from command bytes and hashes stay unchanged", async () => {
  const old = action(method().origin);
  const expected = '{"action_ref":{"casting_method":{"kind":"play_from","source":12,"use_alternative":null,"zone":"exile"},"from_zone":"exile","kind":"cast_spell","spell_id":42},"object_id":42,"type":"priority_action"}';
  const command = serializePriorityCommand(priorityCommandForAction(old), { kind: "priority", actions: [old] });
  assert.equal(canonicalWireJson(command), expected);
  assert.equal(await sha256Hex(canonicalJson(command), webcrypto), await sha256Hex(expected, webcrypto));
  assert.deepEqual(resolveSyncedCommand(command), command);
});

test("same-source exact grants remain distinct and stale menu indices cannot select another ref", () => {
  const first = action(method(0), 8), second = action(method(1), 2);
  const decision = { kind: "priority", analysis_complete: true, actions: [second, first] };
  const command = JSON.parse(JSON.stringify(priorityCommandForAction(first)));
  command.action_index = second.index;
  assert.equal(findPriorityActionForCommand(decision, command), first);
  assert.equal(findPriorityActionForCommand({ ...decision, actions: [second] }, command), null);
  assert.throws(() => serializePriorityCommand(command, { ...decision, actions: [second] }), /no longer available/);
  for (const mutate of [
    ref => { ref.casting_method.permission.source = 13; },
    ref => { ref.casting_method.permission.index = 2; },
    ref => { ref.casting_method.origin.source = 13; },
    ref => { ref.casting_method.origin.zone = "graveyard"; },
    ref => { ref.casting_method.origin.use_alternative = 0; },
    ref => { ref.casting_method = ref.casting_method.origin; },
  ]) {
    const forged = structuredClone(command);
    mutate(forged.action_ref);
    assert.equal(findPriorityActionForCommand(decision, forged), null);
  }
});

test("deferred and synced commands retain the entire exact permission selector", () => {
  const selected = action(method(1));
  const command = serializePriorityCommand(priorityCommandForAction(selected),
    { kind: "priority", analysis_complete: false, actions: [] }, new Map([[42, 7]]));
  assert.deepEqual(command.action_ref, selected.action_ref);
  assert.deepEqual(resolveSyncedCommand(JSON.parse(JSON.stringify(command))), command);
});

test("wrapped face-down casts stay private and pass full refs to engine regeneration", () => {
  for (const face_down_kind of ["morph", "megamorph", "disguise", "permission"]) {
    const origin = { kind: "face_down_play_from", source: 12, zone: "exile", face_down_kind,
      ...(face_down_kind === "permission" ? { face_down_permission_source: 24 } : {}) };
    const selected = action(method(1, origin));
    const command = priorityCommandForAction(selected);
    const synthetic = findPriorityActionForCommand({ kind: "priority", actions: [] }, command);
    assert.equal(synthetic.synthetic_face_down_cast, true);
    assert.deepEqual(synthetic.action_ref, selected.action_ref);
    assert.equal(isFaceDownCastCommand(command), true);
    assert.deepEqual([...collectCommandObjectIds(command)], []);
    assert.equal(isFaceDownCastCommand({ type: "priority_action", actionRef: selected.action_ref }), true);
    delete origin.face_down_kind;
    assert.equal(findPriorityActionForCommand({ kind: "priority", actions: [] }, command), null);
  }
  assert.equal(isFaceDownCastCommand(priorityCommandForAction(action(method()))), false);
  assert.deepEqual([...collectCommandObjectIds(priorityCommandForAction(action(method())))], [42]);
});

test("alternative-price origin permissions survive recursive face-down inspection", () => {
  const selected = action({ kind: "alternative_price",
    origin: { kind: "face_down_play_from", source: 12, zone: "exile", face_down_kind: "disguise" },
    origin_permission: { source: 12, index: 1 }, price: { source: 24, index: 2 } });
  const command = priorityCommandForAction(selected);
  assert.equal(isFaceDownCastCommand(command), true);
  assert.deepEqual([...collectCommandObjectIds(command)], []);
  assert.deepEqual(findPriorityActionForCommand({ kind: "priority", actions: [] }, command).action_ref,
    selected.action_ref, "inspection must not unwrap a command into an authorized origin");
});

test("signed intents bind exact selectors, origin fields, and independent price receipts", async () => {
  const keys = await createAuditSessionKey(webcrypto);
  for (const selected of [action(method()), action({ kind: "alternative_price", origin: method().origin,
    origin_permission: { source: 12, index: 0 }, price: { source: 24, index: 1 } })]) {
    const command = serializePriorityCommand(priorityCommandForAction(selected), { kind: "priority", actions: [selected] });
    const payload = signedActionIntentPayload({ matchId: "permission-test", seq: 4, actorIndex: 0,
      prevStateHash: "before", preActionPublicCheckpointHash: "checkpoint", command });
    const signature = await signAuditPayload(keys, payload, webcrypto);
    assert.deepEqual(payload.command.action_ref, selected.action_ref);
    assert.equal(await verifyAuditPayload(keys.publicKey, JSON.parse(JSON.stringify(payload)), signature, webcrypto), true);
    for (const mutate of [
      method => { (method.permission || method.origin_permission).index += 1; },
      method => { (method.permission || method.origin_permission).source += 1; },
      method => { method.origin.zone = "graveyard"; },
      method => { method.origin.use_alternative = 0; },
      method => { if (method.price) method.price.index += 1; else method.kind = "play_from"; },
    ]) {
      const tampered = structuredClone(payload);
      mutate(tampered.command.action_ref.casting_method);
      assert.equal(await verifyAuditPayload(keys.publicKey, tampered, signature, webcrypto), false);
    }
  }
});

test("exact origin preferences preserve every grant choice and separate price selection", () => {
  const priced = action({ kind: "alternative_price", origin: method().origin,
    origin_permission: { source: 12, index: 0 }, price: { source: 24, index: 0 } }, 0);
  const other = action(method(1, { kind: "split_other_half_play_from", source: 12, zone: "exile", use_alternative: null }), 1);
  const primary = action(method(0), 2);
  const [group] = buildPriorityActionGroups([priced, other, primary]);
  assert.equal(group.firstAction, primary);
  assert.deepEqual(group.actions, [priced, other, primary]);
  assert.deepEqual([...group.actionIndices], [0, 1, 2]);
});
