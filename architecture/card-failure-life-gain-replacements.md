# Persistent life-gain replacements

## Source-only status

UNVALIDATED. Builds, compilation, tests and replay remain deferred. The measured
campaign remains 40 recovered and 3,193 unresolved unique cards. Nine additional
baseline-failing identities are proposed from the exact frozen corpus, not
verified recoveries:

- Angel of Vitality
- Bilbo, Birthday Celebrant
- Cleric Class
- Heron of Hope
- Honor Troll
- Knight of Dawn's Light
- Leyline of Hope
- Pest Rescuer
- Phial of Galadriel

Their full Oracle inputs are in `fixtures/life_gain_replacements.json.fixture`,
from dataset SHA-256
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.

## Shared implementation

The appended `AddLifeGainReplacement` ID/payload represents the additional amount,
player filter and complete display. Its native carrier uses the ordinary
`ConditionalWouldChangeLifeMatcher` and `EventModification::Add` machinery.
There is one modified life-gain event, not a separate extra life gain. The
existing replacement application keys and CR 616 choice/order loop apply each
instance once and allow the affected player to order addition and doubling.
A zero-amount or prohibited life-gain event is not manufactured by the modifier.
Conditional ability wrappers retain dynamic source/controller conditions.

A named complete grammar owns additive and doubled life-gain replacement clauses,
including a `while` condition. It rejects changed recipients, unconsumed actions,
unknown conditional tails and values outside the executable signed count domain.
Phial of Galadriel's draw replacement also accepts its trailing “instead” marker;
the existing leading-marker form remains supported. Exactly one marker and the
complete draw action are required. Both draw forms lower to the existing real
conditional replacement, whose suppression scope prevents recursive replacement
of its own extra draws.

## Secondary clauses and deferred checks

The other card text follows existing typed paths: threshold-based P/T conditions,
starting-life-relative comparisons, opening-hand actions, Class designations and
level-gated abilities, source-exile costs, life-threshold activation gates,
variable-cardinality library search, lifelink, pumps and Pest token/death triggers.
These are included in the exact full-card strict/artifact regression fixture.
Phial is counted only with both the life and draw bodies implemented.

Authored, unrun runtime scenarios compare direct and JSON-restored artifacts;
check actual event amounts, zero/prohibited gains, controller changes, source
removal, independent modifier instances, both Add/Double orders and Phial's
life-threshold/empty-hand boundaries for both players. Grammar scenarios retain
conditions and reject omitted/duplicated replacement markers and truncated text.
No claims of compilation or gameplay success are made until the deferred gate.

The separate five-card one-shot conditional “gain N instead” extension is
recorded in `card-failure-conditional-life-gain.md`. It uses self-replacement
resolution programs; the life-gain leaf does not discard “instead” and accidentally
add the replacement gain to the original gain.
