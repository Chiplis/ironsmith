# Filtered static damage prevention (source-only candidate)

## Status and frozen membership

This tranche is **UNVALIDATED**. No Rust build, compilation, test, or corpus replay
was executed for it, following the deferred-validation workflow. The last measured
campaign result remains 40 recovered unique cards and 3,193 unique unresolved
cards. The nine cards below are proposed candidates, not verified recoveries.

The exact Oracle data in `fixtures/static_damage_prevention.json.fixture` comes
from the frozen 32,209-entry campaign corpus, original SHA-256
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.
Each of these nine identities is unsupported in the committed original
`baseline-e8740178.snapshot.json.gz`:

- Daunting Defender
- Djeru, With Eyes Open
- Shield of the Realm
- Temple Altisaur
- Hyperion, Supreme Hero
- Callous Giant
- Shield of the Avatar
- Hedron-Field Purists
- Rem Karolus, Stalwart Slayer

## Shared implementation

`PreventMatchingDamageSpec` retains typed damage-source filters, player/object
recipient filters, combat/noncombat restrictions, an optional inclusive damage
threshold, and a typed prevention amount. Amounts distinguish all damage, a
fixed/dynamic reduction, and prevention of damage above an authored cap. The new
static ID, payload, and runtime replacement action are appended for serialization
compatibility; integration must keep any other already-appended variants first.

The named grammar consumes the complete single-sentence replacement. It reuses
the existing typed damage-source and object/value grammars, including attached
recipients and player/object unions. The existing fixed-to-you production retains
its position and behavior. No card names or unsupported-diagnostic suppression
are part of recognition.

The runtime reuses `DamageAmountReplacementMatcher`, adding a typed inclusive
maximum to its existing source/recipient/combat gates. Damage-source matching
keeps the existing live-object/last-known-information behavior. Dynamic reduction
amounts are evaluated when damage is proposed, in the prevention ability's source
and current controller context. An all-but cap prevents the excess; it is not a
SetTo damage replacement.

Actual prevention queues `DamagePreventedEvent` with the real amount and ability
source/controller. Unpreventable damage remains unchanged and emits no false
prevention event. A reduction larger than the proposed damage prevents only that
damage. Zero actual prevention emits no prevention event.

## Authored regression coverage (not executed)

- Complete frozen inputs through strict compilation, JSON artifact validation and
  restored runtime definitions, without lossy parsing.
- General-language shapes under synthetic names, typed source/recipient filters,
  inclusive threshold boundaries and player/object unions.
- Combat versus noncombat filtering, source control, ability-source removal,
  actual prevention amounts, and unpreventable damage.
- Attached-recipient changes, changing creature counts, and changing the
  prevention source's controller while it remains attached to another player's
  creature.
- Complete-tail rejection of optional prevention, appended effects, and
  multi-sentence prevention follow-ups, including whole-input fail-closed guards.

Deferred execution targets:

- Grammar: `cargo test -p ironsmith-compiler-grammar damage_prevention`
- Runtime: `cargo test -p ironsmith-compiler-runtime --test static_damage_prevention`
- The eventual full frozen-corpus and supported-card regression gates.

## Explicitly unclaimed neighbors

Battletide Alchemist's optional prevention, Cover of Winter's shared prevention
allocation, Plated Pegasus's alternative repeated-source tail, and the
follow-up programs on Hostility, Purity, Swans of Bryn Argoll, and The Mindskinner
are not included in the proposed count. They need complete optional-choice,
follow-up, player/source binding, or additional grammar semantics. In particular,
CR 615.12 follow-ups can still happen when damage cannot be prevented; a plain
prevention payload is not a substitute. No runtime closure is claimed for those
cards.

Cover of Winter is not a surface-only extension. Its [official Gatherer rulings](https://gatherer.wizards.com/Pages/Card/Details.aspx?multiverseid=121140)
require the player to divide one source's prevention allowance across its
simultaneous recipients. Independent per-target reductions would over-prevent.
The new reader explicitly rejects `and/or` and `one or more` recipients until a
source-grouped allocation path is implemented. This restriction does not exclude
any of the nine proposed first-tranche inputs.
