# Dynamic token quantities: five exact frozen sources

Status: **UNVALIDATED source proposal**. No Rust builds, compiler runs, or tests
were executed. All five fixtures exactly match the frozen source metadata and
Oracle fields. All five were `parse_failed` in the stack07 frozen snapshot,
with `unsupported dynamic token count in create clause` as their primary
failure. Expected after integration: strict compilation with metadata intact,
no loss diagnostics, and executable full bodies in both direct and serialized
artifact paths. This is not measured coverage.

## Exact membership and reusable changes

- **Emissary Green** (`c43e7fa5-ff93-42d9-af79-ee417a8e41a7`): composable
  `number of <option> votes` reads the existing typed `VoteCount`, so ordinary
  scaling preserves twice the profit votes while the second instruction reads
  security votes. It uses the same resolution's voting result, including extra
  votes, rather than counting players or permanents. The complete attack
  trigger, voting procedure, Treasure creation, and per-creature counters are
  retained.
- **Sorin, Grim Nemesis** (`0a01dd05-e289-489f-b3ad-88e15e157bd0`): `highest life
  total among all players` reads `LifeTotal(MostLifeTied)`. All matching players
  have the same maximum, so this existing scalar evaluator neither sums ties
  nor asks a player to choose a quantity. Both text renderers preserve that
  phrase. His other abilities retain reveal-to-hand/mana-value life loss and
  announced loyalty X damage/life gain.
- **Shilgengar, Sire of Famine** (`71ac4d8c-5b58-4ea8-996b-f0289717c4bb`): the
  terminal `instead` belongs to the enclosing conditional consequence, not its
  numeric toughness expression. The sentence composer turns a singular
  sacrifice-qualified result gate into an action/filter-qualified prior-count
  comparison and retains one mutually exclusive self-replacement program.
  Local resolution sacrifices have precedence; otherwise the existing
  `sacrifice_cost_N` snapshots supply the predicate and the pronoun's exact
  paid-object toughness. The cost bridge accepts `You` as well as passive
  wording because `sacrifice_cost_precheck` explicitly rejects any other payer;
  opponent and iterated-player scopes remain unsupported by this bridge.
  No new runtime marker is introduced. The second ability keeps its actual
  hybrid mana/six-Blood payment, graveyard return with finality, and subtype
  addition scoped only to returned creatures.
- **Lacerate Flesh** (`201c8de7-9675-475b-8f9e-7b1935234488`): exercises the
  already-integrated typed excess-damage query through generic creation's
  `where X`/value-expression path. No second excess metric or permissive count
  fallback is added. Blood creation uses actual post-prevention damage excess;
  an illegal sole target counters the whole spell.
- **Waking the Trolls** (`c8cdabc4-e147-491b-9c44-96e57f6ec3b4`): exercises the
  already-integrated typed comparison/difference binding, scoped to the chosen
  opponent and this ability's controller. All three Saga chapters are retained,
  including the graveyard land's owner versus its new battlefield controller.

The shared conditional parser still passes the original sentence tokens to
self-replacement classification. It removes only an outer leading/terminal
`instead` from the consequence parser, leaving quoted ability text intact.
Unsupported result predicates in an authored self-replacement explicitly reject;
they cannot fall through as additive default-plus-conditional programs. Negative
regressions include opponent/iterated-player sacrifice, discard, and grouped
shared-characteristic sacrifice gates.

## Authored, unrun regressions

- Compiler grammar: named vote/scaled vote and highest-life values, negative
  unsupported opponent-maximum surface, and typed sacrifice-qualified
  self-replacement topology.
- Compiler resolution: only passive/actual-payer scopes bind cost snapshots;
  a real local sacrifice remains the prior producer ahead of imported cost tags.
- Normal compiler-runtime integration target: full five-card strict direct/JSON
  artifact loop plus eight gameplay properties. They use actual attack, cast,
  activation/payment, counter, life, damage/prevention, and zone actions.
  Checks include zero profit with nonzero security, additional votes, source
  controller changes, response-time maxima/ties, 501 tokens, all three Saga
  chapters, current selected-opponent differences, paid-object departure
  toughness, forbidden blink-follow, non-Angel fallback, six Blood sacrificed,
  returned-only Vampire additions, and finality replacement on later death.
- Normal tools integration target: all five metadata-bearing exact payloads
  must be `StrictCompiled` without loss or metadata-free fallback.

## Boundaries

This branch starts at the source-reviewed token-resource-cap closure
`8b83799f`; it does not reinstate the removed silent 500-token truncation.
The 501-token scenario remains unrun. Existing native integer/engine-resource
limits retain their separately recorded correctness constraints. No claim of
unbounded mathematical cardinality or completed deferred runtime validation is
made here.

Sovereign Okinec Ahau remains partial at grouped counter-event precomputation
and replacement/commit semantics. Mathemagics remains partial at exact exponent
and bounded execution semantics. Neither is part of this five-card proposal.
