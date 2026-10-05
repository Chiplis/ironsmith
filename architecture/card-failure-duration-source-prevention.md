# Targeted source prevention and complete durations

Status: source-only / UNVALIDATED. No compiler, build, test or executable probe was run.

The exact frozen twelve-card inventory is in
`fixtures/duration_source_prevention.json.fixture`, including metadata, oracle
identities, baseline hashes and original diagnostics. This checkpoint proposes
seven complete bodies: Dromoka's Command, Inquisitor's Snare, Dovin, Hand of Control,
Kiora, the Crashing Wave, Old Fat Spider Can't See Me, Hallow, and Gideon of the Trials.

## Representation and execution

A complete targeted-source reader retains the target selector and the authored
expiry. It recognizes active/passive outgoing damage and the bidirectional
“to and dealt by” form. A carried duration binds `Forever`; the reader never
substitutes end-of-turn for next-turn or source-presence duration.

The semantic action and existing core `PreventAllDamageEffect` append a
`protect_source_target` flag. One target declaration/resolution creates the
outgoing identity-bound shield and the incoming shield for exactly that same
incarnation, both in the existing shield manager. The incoming shield does not
limit the damage source. It retains combat/noncombat classification when present.
No creature ability is granted; source departure, ability loss or target
controller changes do not remove ordinary turn-duration shields.

The direct executor establishes checked current characteristics inside the
existing resource transaction. Conditional durations materialize exact source
references before registration and do not start after the duration is already
false. Phasing explicitly ends battlefield-presence shields and the existing
continuous-effect presence latches, even if no characteristic query happens
between phasing out and back in. The latter is required for Old Fat Spider's
chapter-I hexproof as well as chapter-II prevention. No duplicate prevention
store was introduced.

Runtime regressions use metadata-bearing direct compilation and serialized
artifact materialization, real cast/loyalty announcement, simultaneous Fight,
actual damage, public movement/phasing, and public turn progression. They cover
one target in both directions, noncombat/combat/unpreventable damage, unrelated
sources/recipients, exact departed-source LKI versus blink, original-controller
expiry, both Dromoka mode pairs, Snare's color-dependent destruction, Dovin's tax,
Kiora's other loyalty modes, and all four Saga chapters. Typed grammar tests
reject duplicate targets, unknown riders and dropped secondary sentences.

## Still partial (not counted)

- Auriok Replica, Burrenton Forge-Tender, Pay No Heed, Prahv and Rith's Charm:
  the shared chosen-source domain currently omits some CR 609.7a eligible
  referred-to objects/face-up command-zone objects, and needs the explicitly
  permitted permanent-spell-to-resulting-permanent link. Arbitrary stable-card
  following would be wrong. This checkpoint does not claim those five.


The specified properties on a chosen-source prevention effect must still be
checked when damage would occur (CR 609.7b/615.9). No color/source-property filter
was removed. Rules source: [official Comprehensive Rules, June 19 2026](https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf).

## Unlimited prevention follow-ups and the permitted spell transition

Hallow now binds its gain to `EventValue::Amount` inside the existing shield's
deferred program. The core payload is generic over its child effect type;
semantic/native child visitors, typed interpreter, artifact decoder/card-graph
mapper, native encoder and renderer retain the complete program. It gains only
when positive preventable damage is actually prevented, once for each actual
prevention, under the shield controller. No immediate gain is emitted.

CR 400.7c additionally requires the shield to cover the permanent its selected
spell becomes. The existing Stack-to-Battlefield movement owner records exactly
that successor in the existing damage filter, retaining the original spell ID
as well. Battlefield departure/reentry cannot update that exception. This also
provides the transition primitive required by the still-partial source-choice
cohort, without claiming its unresolved eligible-object domain.

Authored Hallow scenarios cover direct/artifact bodies, no gain at registration,
multiple recipient/events, unpreventable damage, the actual resolving creature
spell transition, exact departed-source LKI, blink rejection, expiry, invalid
target, simultaneous unprevented originals before the gain, and transactional
rollback when the deferred gain exceeds the native life representation.

## Conditional command-zone game rules

The two native game-outcome restrictions now supply a typed conditional
`RuleRestriction` adapter. This preserves their controller/opponent scopes and
works from a command-zone emblem. The previous generic conditional fallback
would create a battlefield self-ability grant, unsuitable for the emblem.
Gideon's exact full body has authored direct/artifact tests for all three loyalty
abilities, continued planeswalker type/indestructibility during animation,
current control of any qualifying Gideon planeswalker, source ability loss,
phasing, and an attempted loss/opponent win while protected. The emblem's
condition resumes when true again; it is not a source-lifetime duration.

### Numeric boundary correction

A fully prevented replacement-expanded damage event can have a valid u32 amount
above i32::MAX even though no surviving damage assignment reaches the later
scalar guard. Damage/prevented-damage and life-event Amount values now use the
checked scalar conversion and propagate ResourceLimitExceeded. Authored negative
scenarios set the incoming damage to i32::MAX + 1 and u32::MAX before Hallow
prevents it, and require complete state/history/one-shot rollback rather than a
wrapped zero gain. The maximum is an explicit host representation limit, never
a rules cap.
