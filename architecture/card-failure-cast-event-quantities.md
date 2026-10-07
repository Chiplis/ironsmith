# Completed spell-cast quantities

Status: **UNVALIDATED implementation-first source proposal**. No builds, tests,
compiler probes, formatters, or corpus execution were run. The coordinator owns
whole-card coverage accounting; this branch changes no campaign counts.

The exact frozen e874 baseline and `cards-20261003.json.xz` identify three complete
bodies, retained with their metadata and diagnostics in
`fixtures/cast_event_quantities.json.fixture`:

- Imminent Doom (`6ded956d-30ed-49ca-8653-cbaaae676b0c`)
- Namor the Sub-Mariner (`dff11626-ad4d-443a-b7db-f508790b5fd2`)
- Voracious Bibliophile (`dcc323d6-ddae-4fd9-9c9e-466b07895d73`)

The third card is **Voracious Bibliophile**, not Biblioplex Assistant. The latter
already compiled in the frozen baseline. All three selected failures report that
an event-derived amount requires a compatible trigger or prior effect.

## Typed grammar and ownership

A shared named rule in both object-filter registries consumes a complete counted target or colored
mana-symbol suffix. It owns the symbol color before the ordinary noun reader can
mistake it for a spell color. Unknown continuations of these recognized suffixes
are committed errors, including trailing punctuation or mana symbols. The typed
`mana_symbol_count` field is appended after the previously published serialized
ObjectFilter fields and survives semantic merging,
serialization, matching, and rendering; each hybrid/Phyrexian pip containing the
named color contributes one symbol independently of payment.

`CastEventQuantity` distinguishes mana value, colored mana symbols, and distinct
chosen targets. `EventValueSpec::CastSpell` is a serialized value owner, not a
presentation string or the mutable generic event-amount slot. Shared semantic
trigger inspection selects a single proven quantity. Conflicting quantified
restrictions and differently quantified union arms do not acquire an arbitrary
cast quantity. The pre-existing target-subset admission query now has one shared
semantic owner.

Reference environments and lowering frames retain the cast quantity. Original
unqualified quantity references remain attached to the cast across sibling
instructions; an explicit prior-result reference or a pinned result scope still
belongs to its actual producer. Replacement-added instructions retain their
separate generic amount. Text rendering preserves the ordinary demonstrative.

## Completed event evidence

The priority cast path, ordinary effect-driven cast publication, and the
search-library cast publication use one completion constructor. It snapshots the
exact spell incarnation and chosen target slots after the cast transaction.
Missing snapshots, a different ObjectId/zone, missing chosen targets, and missing
announced X are `IncompleteEvidence` failures. Boolean admission records them
through the existing incomplete-execution latch, and the checked queue/stack
boundary rolls back rather than silently losing a trigger. Resolution also stops
before negative-result followups.
An explicitly empty target list and a mana cost absent from a known snapshot are
real zeroes.

Mana value reads the selected cast face and announced X, including repeated X
symbols. A face-down spell has zero mana value and zero colored cost symbols.
Printed cost symbols never become mana spent; cost reductions, free casting,
hybrid payment choices, and Phyrexian life payment do not change the symbol count.
Counts use u128 intermediates, checked i64 scalar conversion, and the existing
checked narrowing boundary for characteristic/cost consumers. Imminent Doom's
source-counter admission compares the captured mana value without i32 narrowing.
Its later damage uses that completed cast even if source counters change.

The bounded plain characteristic filters of these three cards match the same
completed snapshot as their quantities. A delayed reported event retains
Namor's noncreature test after its spell changes type or leaves the stack.
Relation/history filters retain their established owners; the bounded predicate
rejects additional fields by default.

The shared completed-cast publication boundary captures actual trigger matches
before an effect returns its cast receipt or performs cleanup. It reuses the
existing trigger queue, delayed-trigger checks, history recording, receipt
capture marker, and incomplete-execution rollback scope. Priority casts append
that captured queue; effect-driven publishers defer it. Re-reporting the marked
receipt does not match it again. Other pending events and held batches are not
drained by this cast boundary, and resolution-time intervening-if predicates
remain under the existing resolution owner.

This also preserves Imminent Doom's observer counter comparison when reporting
is held and a later instruction changes its counters. The official
[Hour of Devastation release notes](https://magic.wizards.com/en/news/feature/hour-devastation-release-notes-2017-06-30)
confirm that Doom's damage uses its counter quantity when the ability triggered.
The new values bind the three cards' event-demonstrative quantities only;
explicit generic “that spell's mana value” reads retain their current/LKI owner.

The caster remains the event's captured caster. Existing source ObjectId/LKI
ownership governs damage and the counter tail, so leaving and returning does not
redirect the original ability to the new source incarnation. The distinct-target
value counts each player/object once even when multiple target slots name it.
The existing target-arity predicate continues to distinguish target slots for
single-target eligibility.

## Authored verification

The independent `ironsmith-tools` and `ironsmith-compiler-runtime` integration
files named `cast_event_quantities` cover full frozen payloads, direct compilation,
serialized artifact materialization from a separate compiler invocation, and native priority/effect-driven cast
transactions. They retain Doom's entry counter and later counter tail, Namor's
flying and dynamic Merfolk power plus exact 1/1 blue Merfolk tokens, and
Bibliophile's flying/vigilance and draw instruction.

Scenarios cover announced X, post-cast counter changes, caster/type/zero-symbol
negatives, split-face hybrid/Phyrexian costs, free casting, source/spell/target
departure, duplicate slots and mixed target kinds, copies, later retargeting,
missing evidence, real zeroes, wide mana values, and replacement-added amounts
versus original cast-bound siblings. Local grammar and resolver contracts cover
complete suffix consumption, color ownership, ambiguous quantities, explicit
prior-result precedence, frame transfer, both public filter readers rejecting
nonword tails, checked incomplete admission, rollback before negative branches,
native reported-cast admission after spell departure or a type change, and
held Doom reporting after a counter change with duplicate receipt publication
and exact once-per-cast history.

These are authored source contracts, not passing claims. No additional card or
unsupported body is promoted by this proposal; whole-card recovery remains
subject to independent source review and the deferred execution gate.

## Namor's subtype-only power count

The secondary body uses `parse_characteristic_defining_stat_value` in
`keyword_static/mod.rs`, the shared `parse_number_of_value` in
`grammar/shared_util/value_expr/value_expr_core.rs`, and the simple object-filter
reader. Its Merfolk atom sets `subtypes = [Merfolk]`; the controller suffix sets
`controller = You`; neither `card_types` nor `all_card_types` gains a Creature
restriction. The resulting `Value::Count` reaches `LayerValueContext::count`;
`for_each_filter_candidate` defaults its unzoned plain filter to the battlefield.

An authored regression inspects this exact typed power filter on independently
compiled direct/artifact full Namor bodies. It then adds a controlled noncreature
Kindred Enchantment — Merfolk, verifies the power increase, excludes another
player's Merfolk and Merfolk in hand/graveyard, and checks control/zone changes.
No production filter issue was found in this bounded source trace. The regression
remains unrun under the execution gate.
