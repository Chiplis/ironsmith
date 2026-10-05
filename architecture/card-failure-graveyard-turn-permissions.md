# Per-turn graveyard permissions and recipient abilities

UNVALIDATED source proposal. No build, compilation, parser probe, formatting,
test or corpus replay ran. Every authored scenario remains unrun. Measured
campaign counts are unchanged: 40 verified / 3,193 unresolved unique cards.

## Exact bodies

- Kagha, Shadow Archdruid (`f78db9ad-d4a3-4e49-a76e-bd31124b7837`): attack grants
  deathtouch until end of turn and mills two. One shared land/permanent-spell
  permission during its controller's turn checks the exact current graveyard
  incarnation's library origin this turn.
- Raul, Trouble Shooter (`d247a86d-917d-4833-82c4-876943d54e2c`): one spell during
  its controller's turn must have an actual current-turn CardMilledEvent naming
  that destination incarnation in the graveyard. A generic library move does
  not qualify. The tap activation retains the each-player mill body.
- Serra Paragon (`ecd8d7f4-7b11-4097-afc0-ef3c81edd90c`): flying plus one shared
  graveyard land/permanent-spell use; only the spell arm has mana value <= 3.
  The recipient gains a real battlefield-to-graveyard trigger that tracks the
  departing incarnation, exiles it if still there, and gains two life even if
  that card subsequently leaves before the trigger resolves.
- The Eighth Doctor (`0956bc16-ff04-4e96-8059-ef62afa2405f`): entry mills three;
  the shared land/permanent-spell permission requires historic characteristics
  on the selected face. The recipient gains a removable static zone-change
  replacement from battlefield to any destination other than the replacement
  exile; phasing is not a zone change.

Complete frozen bodies are in `fixtures/graveyard_turn_permissions.json.fixture`,
SHA-256 `d0ec1a890a01c5d4fc8acc244b65c0764e423c107111ee9228fd31b639007872`.

## Shared boundaries

The grammar consumes exact origin qualifiers and complete quoted recipient
riders. All routes use the existing source-qualified GrantPermissionIdentity,
turn usage owner, face selection, permissions and payment pipeline. Repeated
land/spell origins remain one GrantSpec and one allowance. They do not grant
extra land plays, relax timing or authorize another player's graveyard.

The new static-program vector defaults empty when decoding a legacy compiled
GrantSpec. Runtime retained grants still require the captured rider field.
`permanent_this_way_grants` carries complete static payloads through compiler,
artifact, native grant, retained grant and interpreter mappings. A quoted
trigger uses the existing source-relative inline ability materialization. It
is not an immediate effect or a reflexive permission-use trigger. Cast proposals
capture the exact origin and price rider occurrences before costs can remove
the provider; the composite-price route preserves both obligations.

Persistent recipient registrations reuse the existing per-object ability
occurrence carrier, with an explicit absent end-turn expiry. Old finite grants
remain finite; retained data must include the expiry field, so omitted data
cannot silently become indefinite. The new registrations are noncopiable and
clear on an ordinary zone change, preserving only the Stack→Battlefield passage
of the original permanent spell. Copies of that spell do not inherit these
applied recipient riders. Control changes, phasing and cleanup preserve the
original object's grant, and ordinary ability removal can suppress it.

A land receipt captures the selected payload before entry processing. Its
native announcement binding attaches it to the original special-action entry
before entry counters and additional replacement programs. A redirected or
independent replacement movement cannot transfer it. The scoped binding is
cloned by native savepoints, erased by completion, and makes checkpoint export
fail closed while pending through the existing announcement completeness gate.
Existing signed-genesis replay remains the cross-worker recovery route.

The new mill qualification reads exact destination IDs plus the recorded public
graveyard destination. Both continuous-query history-dependency owners include
this flag. A later return of the same physical card does not reuse the old mill
receipt. Turn reset, ownership and source activity remain enforced by existing
history/grant owners.

Authored gates cover all four whole direct/artifact bodies; real attack/entry/
activation mill producers; mill versus arbitrary library movement; owner,
turn, source phasing and shared land/spell budgets; selected MDFC faces; persistent
land/cast recipients, provider sacrifice as a cast cost, later control and zone
changes, suppression, spell copying, composite origin/price budgets, delayed
Serra trigger resolution, and pending land-entry rollback. These are source
assertions awaiting deferred validation, not measured recoveries.
