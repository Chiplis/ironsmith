# Delayed player attack declarations: source-only proposal

Status: source-authored, UNVALIDATED / UNRUN. Based on b48c47847092989dbb2eee11db7f2a94ac74223b. No builds, tests, compiler probes, corpus execution, formatters, publication, or coverage/ledger credit. Surviving changes were recovered and inspected in full; the two additional source corrections below were made without execution.

## Exact proposed scope

- Dalkovan Encampment — 33a90122-7280-4481-9b97-5879194cae40
- Jaya, Fiery Negotiator — 2458aa66-5b20-4811-a4a4-8375ad0a6498
- Roads Go Ever, Ever On — 3cd3cdc9-2ed2-4cdf-b58a-12c3a2ef218b

The full source bodies and metadata were copied from the October 7 retained `cards-current.json`, matched by Oracle ID. All three have the measured unsupported delayed PlayerAttackDeclaration diagnostic. A fix is proposed, not a measured recovery.

## Shared implementation

1. Add typed DelayedTriggerSpec::PlayerAttackDeclaration carrying attacker, defender, and all four PlayerAttackGrouping modes. Derive-based serde and TagKeyWalk preserve its typed fields; there is no text reparsing or creature-attack approximation.
2. Lower the existing semantic event into that shared variant. Runtime interpretation constructs the existing PlayerAttackDeclarationTrigger, whose native projection already preserves the same fields.
3. Send the complete directly emitted attack-declaration action to delayed matching. The ordinary simultaneous queue checks only object abilities; previously it did not visit delayed registrations for these events. Record the action and queue ordinary abilities first, then match delayed registrations, following the existing reported-event adapter's order.
4. Coalesce actor/defender groups within each delayed registration and simultaneous action. Separate registrations and later combats remain distinct; one-shot consumption occurs after grouping. Pair mode stays ungrouped because the emitter already produces distinct pairs/categories and its matcher excludes non-direct pairs.
5. Treat the declaration as a new combat observation when scheduling, so an event-relative defending-player reference is not incorrectly bound before that future combat.

6. Extend this-turn delayed reference imports to PlayerAttackDeclaration. The older helper recognized the previous creature-attack variants but omitted the new player declaration; Jaya's chosen creature must be imported from the registering body rather than inferred from a player event with no object.

This retains duration/one-shot ownership, uses the existing tagged target capture and incarnation checks for Jaya, and preserves Dalkovan's nested token-sacrifice scheduling. No new duration or target machinery is introduced.

## Authored validation (all UNRUN)

`crates/ironsmith-compiler-runtime/tests/delayed_player_attack_declarations.rs`:

- Independently compile each full exact source through direct runtime and artifact routes, reject parse loss, JSON round-trip/validate, materialize, inspect the native schedule and retained typed trigger projection.
- Round-trip every grouping mode with non-default player filters.
- Dalkovan: split player/planeswalker targets queue once, created attacking tokens do not retrigger, later planeswalker-only and battle-only combats trigger again, each token batch gets a next-end-step sacrifice, original attackers remain, registration expires next turn.
- Jaya: frozen chosen target, no targets selected for the delayed ability, no replacement target after a blink while it is on the stack, source departure does not cancel the delayed ability, and removing an attacker before resolution changes damage from two to one. A later combat repeats the retained registration.
- Roads: resolve chapter IV's full authored body, source departure, explicit new target selection each combat, and a Plains entering after target selection increases that resolution's bonus.
- Two identical registrations remain separate on both independently compiled routes; empty/opponent declarations do not match.
- Native actor/defender/pair grouping checks exact representative pairs, direct-player versus non-direct categories, non-default actor/defender filters, and one-shot consumption after grouping.
- The production declaration queue preserves ordinary attack triggers, delayed singular-creature attack triggers, and delayed one-or-more-creatures attack triggers exactly once through pending-event drains.

Runtime body scenarios intentionally resolve the selected complete ability program rather than paying activation costs or dispatching Saga lore chapters. Those unrelated systems are not claimed validated. Independent direct/artifact compilation includes every ability in each original body.

## Admission and unresolved work

All three exact IDs are proposed source candidates only. Validated admissions: none. Held pending the coordinated gate: Dalkovan Encampment, Jaya, Fiery Negotiator, and Roads Go Ever, Ever On. Static source review found no additional unresolved implementation issue within this narrow bridge; compilation and behavioral correctness remain unverified. The new target/reference and queue-order expectations have not been executed.

## Compatibility and coordinated gate requirements

Do not publish these sources under the unchanged current boundary. This adds a serde enum variant to the ScheduleDelayedTriggerEffect wire payload and changes live delayed-combat semantics. The coordinator owns whether the successor is appended to the still-unpublished 12/8/25 packet or becomes a later boundary after its publication; this worktree intentionally changes no boundary/version/hash constants.

The existing ScheduleDelayedTriggerEffect entry in `ironsmith-artifact-effect-decoder/effect-registry.tsv`, its stack_event decoder/card graph mapping, engine artifact materializer, and shared effect interpreter already transport the generic payload. No new decoder family or generated dispatch entry is needed. Native PlayerAttackDeclarationTrigger projection already exists in `continuous/text_change_triggers.rs`.

At the authorized coordinated gate: regenerate descriptor/hash and any affected goldens/catalogs from original source, reject older artifact/savepoint envelopes per the integrated compatibility policy, and run the new focused suite plus existing player_attack_declarations, combat_participant_savepoint_tests, delayed-trigger/creature-one-or-more combat regressions, native projection, artifact decoder/materializer, and compatibility rejection suites. Old compiled artifacts must not be relabeled as migrated. Do not award recovery credit until exact IDs compile losslessly and the full gate passes.
