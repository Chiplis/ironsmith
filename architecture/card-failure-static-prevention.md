# Filtered static damage prevention (source-only candidate)

## Status and frozen membership

This tranche is **UNVALIDATED**. No Rust build, compilation, test, or corpus replay
was executed for it, following the deferred-validation workflow. The last measured
campaign result remains 40 recovered unique cards and 3,193 unique unresolved
cards. Eighteen identities below are proposed candidates, not verified recoveries.

The exact Oracle data in `fixtures/static_damage_prevention.json.fixture` comes
from the frozen 32,209-entry campaign corpus, original SHA-256
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.
All eighteen fixture identities are unsupported in the committed original
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
- Plated Pegasus
- Vigor
- Phyrexian Hydra
- Purity
- Swans of Bryn Argoll
- Hostility
- Angel of Suffering
- The Mindskinner
- Unbreathing Horde

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
- Complete-tail rejection of optional prevention, unrelated appended effects,
  reflexive triggers and unrecognized follow-ups, including whole-input
  fail-closed guards.

Deferred execution targets:

- Grammar: `cargo test -p ironsmith-compiler-grammar damage_prevention`
- Runtime: `cargo test -p ironsmith-compiler-runtime --test static_damage_prevention`
- The eventual full frozen-corpus and supported-card regression gates.

## Explicitly unclaimed neighbors

Battletide Alchemist's optional prevention, Cover of Winter's shared prevention
allocation, Phyrexian Vindicator's reflexive trigger and Immortal Coil's repeated
exile choices are not included in the proposed count. They need complete optional-choice,
follow-up, player/source binding, or additional grammar semantics. In particular,
CR 615.12 follow-ups can still happen when damage cannot be prevented; a plain
prevention payload is not a substitute. No runtime closure is claimed for those
cards.

Cover of Winter is not a surface-only extension. Its [official Gatherer rulings](https://gatherer.wizards.com/Pages/Card/Details.aspx?multiverseid=121140)
require the player to divide one source's prevention allowance across its
simultaneous recipients. Independent per-target reductions would over-prevent.
The new reader explicitly rejects `and/or` and `one or more` recipients until a
source-grouped allocation path is implemented. This restriction does not exclude
any of the eighteen proposed inputs.

The repeated-spell extension accepts the complete `prevent N damage that spell
would deal to that ...` tail only when the event source is a spell and its
repeated recipient filters equal the original event filters. It proposes Plated
Pegasus as one additional candidate. An authored simultaneous four-recipient
scenario checks independent per-recipient reduction; this remains unexecuted.

Vigor's existing complete production is exposed through its actual `if` lexical
head. The general per-prevented-damage counter sentence now retains its typed
counter sign/type, including Phyrexian Hydra's -1/-1 counters. Both reuse existing
`PreventDamageThen` execution, so unpreventable damage runs the additional part
with amount zero and emits no false prevention event. Two exact baseline failures
are added; their grammar and direct/artifact counter scenarios are authored,
unrun source coverage only.

The additional all-damage follow-up payload now owns complete gain-life,
damage-source-controller draw, and single-token-per-prevented-damage programs.
Purity, Swans of Bryn Argoll, and Hostility add three source-only candidates.
The generic payload keeps player/object recipient filters, the damage source,
combat restrictions and effect programs separate. It delegates to existing
`PreventDamageThen`, which invokes the additional effects with actual prevented
amount zero for unpreventable damage and emits CR615.13 events only for positive
prevention.

Damage-source LKI is distinct from prevention-ability-source LKI. The former is
now retained on the damage envelope and pending follow-up, then restored on the
follow-up event. `TagTriggeringSourceEffect` prefers a current non-phased source,
then the event's matching snapshot, then matching departed-object history. This
allows Swans to refer to the actual damage source's controller after that source
has departed. Snapshot retention validates source identity. Authored direct and
artifact scenarios include supplied LKI after turn-history clearing, counter
signs, token characteristics and actual-zero prevention; none were executed.

The conjoined mill extension explicitly uses `PreventionFollowUpAmount::Proposed`
for “that many” referring to the damage at the time this prevention applies.
Existing explicit “damage prevented this way” payloads retain `Prevented` as the
default. The appended `PreventDamageThenFromProposedAmount` action stores the
actual prevention separately from the amount exposed to its additional effects;
unpreventable damage still mills, while no false `DamagePreventedEvent` is sent.

This distinction follows [CR 615.12](https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf)
and the explicit Angel of Suffering ruling in the [official New Capenna release notes](https://magic.wizards.com/en/news/feature/streets-new-capenna-release-notes-2022-04-20).
Angel of Suffering and The Mindskinner add two exact baseline identities to the
source-only proposed set. Authored unrun scenarios cover empty libraries,
unpreventable damage, combat, source control, all opponents in multiplayer,
non-opponent recipients and two competing prevention instances. No optional,
reflexive or multi-action original-damage program is claimed by this extension.

Unbreathing Horde adds a subject-first self-damage production without weakening
whole-clause checks. Its existing fixed-counter-removal carrier now uses true
prevention plus a proposed-amount additional action, rather than a plain
`Instead` replacement. Consequently unpreventable damage is dealt while the
counter-removal action still runs, prevention emits its actual-amount event, and
having zero counters does not make damage unpreventable. Existing one-damage-per-
counter shields retain their separate typed action. Grammar and direct/artifact
scenarios for these distinctions are authored and unrun.

A follow-up source correction separates three previously conflated counter
semantics. A genuine “put counters instead” program retains a non-prevention
replacement and can replace unpreventable damage. Conjoined “prevent ... and put
that many” uses the proposed amount; the explicit “for each damage prevented this
way” form uses the actual amount. The latter two generate prevention events only
for actual prevention. Their source and artifact scenarios distinguish all three
paths under both preventable and unpreventable damage, and remain unrun. No new
candidate identity is claimed by this semantic correction; Anti-Venom was already
proposed by the earlier source-context family.

Unbreathing Horde's entry expression is now explicitly represented as the sum of
two typed counts: other Zombies controlled on the battlefield, plus Zombie cards
owned in the graveyard. The dual-for-each entry grammar accepts the shared
counter clause before “and each”; it retains the existing fully repeated clause.
The generic domain-union fallback already recognizes similar disjoint-zone
language, so this is an explicit additive ownership path rather than evidence
that the previous fallback failed at runtime. Source review traced both filters
through `EntersWithCountersValue` into pre-entry value evaluation, before the
old object changes zones. A whole-card direct/artifact scenario uses the actual
ETB operation from both stack and graveyard, then proposes damage and checks the
counter removal. This completes the source review required to propose Horde;
it does not establish executed correctness or a measured recovery.
