# Single-source damage reference quantities

Status: **UNVALIDATED source proposal**, six exact full-card candidates. No builds, compiler runs or tests were executed.

Fixture: `fixtures/damage_reference_quantities.json.fixture`.

| Quantity | Exact cards | Proposed semantic binding |
| --- | --- | --- |
| Definite prior creature's power | Cinder Cloud; Kaervek's Purge | Existing target tag: calculated live characteristics while the same object remains; exact departure LKI after destruction |
| Definite artifact's mana value | Viashino Heretic | The targeted artifact, including current value if destruction fails and battlefield LKI when a prototype/copy leaves |
| Chosen tap-cost object's power | Unerring Sling | Existing `tap_cost_0` identity, distinct from `{T}` on the artifact; current power or exact departed incarnation |
| Power of the card returned this way | Morgue Burst | Explicit return result identity in its destination, rather than immutable graveyard characteristics |
| Source's loyalty as damage amount | Compel Brutality | Loyalty counters on the announced planeswalker damage source, distinct from the spell and recipient |

The frozen stack07 diagnostics for all six were `missing damage amount`. Expected proposed outcome: full metadata-bearing strict compilation with no loss and executable values, through both direct and JSON-round-tripped artifacts. Any independent blocker exposed by later permitted validation keeps the card open.

The shared value grammar now accepts bounded definite possessive characteristics, a tapped creature's cost-linked power, a return-linked card characteristic, and `its loyalty`. The explicit-source damage reader binds the latter possessive to its own source, using the existing `CountersOn` and `ExecuteWithSource` mechanisms. There are no new serialized variants or engine marker abilities.

A compiler-only returned-object quantity alias is exported only by a tagged return instruction, resolved before executable lowering, and rejected without a producer. The explicit result-object contract already used by the return executor names the new hand incarnation. This is intentionally different from destroy's old-zone LKI contract. The alias persists across an intervening unrelated target and is replaced by a later return; it is not a stable-card fallback.

Authored coverage includes a normal tools aggregate; a normal compiler-runtime target with six tests (artifact transport plus five gameplay scenarios); and typed grammar/reference assertions. Scenarios exercise pumped departure power, white/death conditions and indestructibility, controller versus owner, a returned graveyard-count creature whose power changes on moving to hand, prototype mana value, actual tap-cost payment followed by a pump and a forbidden later blink, artifact lifelink proving the damage source, and both Compel Brutality modes with live power/loyalty changes after casting.

Whipkeeper remains open: damage already dealt to an exact object this turn requires a turn-history quantity, not merely currently marked damage. The four multi-source power cards are separately diagnosed in commit `cec350d7`; they remain uncounted pending complete simultaneous damage/prevention ownership.
