# Live battlefield population conditions

UNVALIDATED source work. No build, compiler, test or corpus replay was run.
Eleven exact frozen baseline failures are proposed: Goham Djinn, Halam Djinn,
Ruham Djinn, Sulam Djinn, Zanam Djinn, Knight of Grace, Knight of Malice, Kavu
Runner, Skittish Kavu, Vexing Beetle and Tenacious Hunter. Full inputs are in
`fixtures/battlefield_population_conditions.json.fixture`, from corpus SHA-256
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.
Measured campaign totals remain 40 recovered and 3,193 unresolved unique cards.

A named grammar reads complete population clauses and emits existing typed
`Value::Count` comparisons and Boolean conjunctions. No presentation marker,
new runtime carrier or card-name branch is used. The four shared surfaces are:

- A specified color is most common among all permanents, including ties.
- Any player controls a matching permanent.
- No opponent controls a matching permanent.
- A creature has a specified counter on it.

For color prevalence, the specified color must actually be present and its count
must be at least each of the other four colors' counts. Multicolored permanents
count toward each of their colors. Colorless objects have no contribution.
All controllers' battlefield permanents count; nonbattlefield objects and
phased-out objects do not. The counted color is the authored color, independent
of changes to the source's own color. The existing condition dependency walker
examines both comparison values and Boolean arms, and existing count evaluation
reads current characteristics, so layer-5 color changes remain relevant to
conditional ability and power/toughness effects.

Opponent absence is a zero count over all current opponents in the source's
controller context. It is not an arbitrary choice of one opponent. Any-player
presence has no controller restriction. Counter presence is a live positive
counter filter over creatures, including the source when applicable. These
global conditions gate the original affected subject; they do not grant the
source's bonuses or keywords to the population being counted. Existing native
conditional continuous effects continue to apply/revoke the bonuses and real
keywords.

Two grammar and six runtime scenarios are authored and unrun. Both direct and
JSON-restored artifacts are covered: complete Oracle bodies, the five-color
cycle, ties, multicolored permanents, all players, graveyard/phase-out scope,
live controller changes, color-changing layers, absence among all opponents,
creature versus noncreature counter holders and live keyword revocation.
Top-card ability borrowing, draft dependencies, lifetime damage history,
attachment/temporary restrictions and graveyard targeting restrictions remain
outside this subset.
