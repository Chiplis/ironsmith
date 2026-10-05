# Per-unit life quantities

Status: **UNVALIDATED** source proposal. No build, compilation, test, or compiler
probe was executed. Exact identities and full programs are in
`fixtures/life_unit_quantities.json.fixture`. This batch depends on the preceding
filtered-life-event/temporary-listener batch.

Six complete printed-program proposals: False Cure, Cradle of Vitality, Lich's
Tomb, Lich's Mastery, Oath of Lim-Dûl, and Transcendence. Their secondary programs
are retained, with public scenarios for Cradle's optional payment, Mastery's
gain/draw and source-departure loss, Tomb's zero-life exception, Oath's paid draw,
and Transcendence's real twenty-life state trigger.

## Exact amount evidence

The grammar parses `for each N life you/they/that player gained/lost` into a
numeric prior-effect metric query retaining action direction and participant.
It is never an object-count filter or a turn-total query. Complete quantities
reject unknown tails; grouped units use integer division with a positive divisor.

Life-event evidence and compatible lexical life producers have their own typed
reference-frame state. An intervening mana payment or other result gate cannot
steal the amount. A compatible local life instruction wins before the triggering
event; transparent re-annotation retains the same outcome ID. Optional local
instructions keep their exact result even if declined. A same-direction,
same-participant event proof is required for an event fallback, and every event
alternative must provide the same proof. Mixed gain/loss or damage events do not
substitute through a generic numeric-amount capability.

The appended EventValueSpec::LifeChange retains direction and controller-versus-
affected-player scope in serialized executable programs. Runtime evaluation
requires the corresponding real life event and reads its captured post-replacement
amount; a generic event-count override cannot replace it. Local outcome metrics
filter actual life notifications by the requested participant before summing,
instead of silently ignoring the query's player filter. Typed audit contracts
track gain/loss/controller facts separately from arbitrary numeric events.

## Instruction cardinality

Counter/life clauses scale one action by the captured amount. Tomb selects the
required legal permanents once and sacrifices that group in one batch. Mastery
selects from a typed union of controlled battlefield permanents and owned hand/
graveyard cards, then exiles the complete chosen set together. A stolen permanent
can qualify by control; another player's private-zone cards cannot. Exiling the
source within that batch does not cancel the remaining selected objects.

Oath's unless-discard program instead uses the existing repeated-instruction
executor. Its count resolves once before the first decision; every iteration
retains its own legal discard-or-sacrifice choice and the source exclusion.
This distinction follows CR 608.2f: multiple-object actions are simultaneous when
possible, whereas dependent choice/action programs are considered individually.
The official [June 2026 rules PDF](https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf)
contains that rule and the corresponding simultaneous and sequential examples.

## Deferred scenarios

`life_unit_quantities` has full direct/artifact fixtures and scenarios for all
six bodies, actual paid False Cure casting, repeated captured life gains and
expiration, post-replacement Cradle amounts versus mana paid, rejected payment,
Tomb's grouped deaths, Mastery's mixed-zone selection and self-exile follow-up,
Oath's mixed payment outcomes and paid draw, Transcendence's state-trigger loss,
local-producer precedence across unrelated payments, and direction/participant
negative cases. Grammar and reference-frame regressions are also authored.

Deferred command: `cargo test -p ironsmith-compiler-runtime --test life_unit_quantities`.
All regressions remain unrun until the campaign's validation gate.
