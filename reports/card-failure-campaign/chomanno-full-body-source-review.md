# Cho-Manno, Revolutionary: bounded whole-body evidence gate

Source-only checkpoint. All authored verification is **UNRUN**. No build, test,
probe, formatter, corpus execution, code generation, or remote write was performed.
The workspace unadmitted recovery inventory identifies Oracle identity
`91af5e35-b3b8-43ce-b1ea-997ed74e4ad2` as a baseline parser failure and still
unaddressed. Prior measured compilation is not gameplay evidence. No accounting
admission, new measurement, or residual count is claimed.

## Frozen identity and bounded implementation

`fixtures/chomanno_full_body.json.fixture` transcribes the complete identity,
Oracle text, mana cost, type line, and 2/2 metadata from the local frozen
`cards-20261003.json.xz` record. The original shortened self-reference
“Cho-Manno” is retained verbatim; no fixture-side rewriting hides name resolution.

`crates/ironsmith-compiler-runtime/tests/chomanno_full_body.rs` separately invokes
strict direct runtime compilation and strict artifact compilation, rejects parse
loss, validates the artifact before and after JSON round-trip, checks equality,
and materializes the independently compiled artifact. Both routes must expose
exactly one native `PreventAllDamageToSelf` static ability, no spell program and
no unimplemented content. Neither route is synthesized from the other.

This is a bounded evidence addition, not a production parser/runtime change.
The existing generic self-prevention parser and `PreventAllDamageToSelf`
replacement implementation supply the intended route. `DamageToSelfMatcher`
checks exact recipient object identity and excludes player recipients. The tests
must still be run before calling this identity gameplay-proven or recovered.

## Authored runtime coverage

Each scenario runs against both independently compiled definitions:

- Repeatable combat and noncombat prevention, without activation or consumption.
- Damage-source controller and battlefield/stack/graveyard distinctions.
- Exact self rather than name-wide protection: a same-name abilityless object
  and both players remain unprotected.
- Actual prevented-damage events identify amount, prevention source, and current
  controller; unpreventable damage produces no false prevention event.
- Controller change, phasing out/in, type change, removal of abilities, graveyard
  inactivity, and return as a fresh incarnation with the printed ability restored.
- Native `execute_effect` damage execution keeps damage marked at zero across
  repeated hits, then records damage after ability removal.

The assignment-layer cases intentionally inspect replacement results even for
inactive recipients; they do not assert those recipients are legal spell targets.
The native effect case supplies the separate actual-damage execution check.

## Deferred verification

Future authorized execution should run the focused `chomanno_full_body`
integration target in `ironsmith-compiler-runtime`, then the appropriate aggregate
gates. No passing test, verified recovery, or central coverage update is asserted
by this source-only checkpoint. No wider production blocker was established by
source inspection; execution may reveal one.

## Independent-review hardening (still UNRUN)

Both compiled routes now also assert the exact `{2}{W}{W}` mana cost, Legendary
supertype, and Human/Rebel subtypes. The type-change scenario requires calculated
characteristics to contain exactly Artifact before checking continued prevention.
Every captured prevention event additionally verifies its damage source, target,
combat flag, absence of a created-shield ID, and its single application's source,
target, amount, and combat flag. These complement the existing exact amount,
prevention-source, and current-controller assertions. No production code changed.
