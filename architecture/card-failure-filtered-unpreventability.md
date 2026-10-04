# Source-filtered unpreventable damage

UNVALIDATED source work. No build, compilation, test or corpus replay was run.
Two exact baseline identities are proposed: Excruciator and Questing Beast. Their
complete frozen text is in `fixtures/filtered_unpreventability.json.fixture`,
from corpus SHA-256
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.
The measured campaign remains 40 recovered and 3,193 unresolved unique cards.

## Typed rule and real processing

A complete named grammar owns `[Combat] damage that would be dealt by <sources>
can't be prevented`. The source is a full typed object filter; the combat gate is
independent. `Restriction::PreventDamageFrom` is appended to the existing enum
for ordinal compatibility and carried by the ordinary `RuleRestriction` static
ability, not a keyword/presentation marker. Rendering, tag/reference resolution,
iterated-player validation and source-reference rewriting include the new field.

Native restriction refresh registers the source filter with the active host's
controller, host identity and bound tags. Damage processing checks source-scoped
prohibitions along with existing global/combat-only rules. It compares live
source characteristics when the source is present and retained damage-source
information after departure or phase-out. A resolving spell's source remains a
spell when its stack entry has already been popped. The retained damage source
is independent of the restriction host: it cannot keep a departed or phased-out
host's battlefield static active. Control changes rebind a live static's scope.

The resulting event is unpreventable through the existing prevention engine.
This does not skip all replacement processing: non-prevention modifications
still apply, and existing CR 615.12 additional-action behavior remains intact.
No positive DamagePreventedEvent is emitted and finite shields are not spent
when zero damage was prevented. Simultaneous finite-shield allocation also
excludes source-prohibited damage, so it cannot consume capacity that belongs
to a different preventable event. The independently supplied spell-local
`unpreventable` flag is retained; a filtered static does not replace it.

Questing Beast's restriction covers only combat damage from its controller's
creatures. Its planeswalker trigger deals noncombat damage and remains outside
that rule. Wizards' [Throne of Eldraine release notes](https://magic.wizards.com/en/news/feature/throne-eldraine-release-notes-2019-09-20)
confirm both distinctions.

## Deferred evidence and explicit partials

One grammar scenario and six direct/restored-artifact runtime scenarios are
authored and unrun. They cover exact complete cards, self-source versus unrelated
source, both damage kinds, live controllers, creature versus noncreature source,
explicit spell-local unpreventability, source LKI versus host lifetime, finite
simultaneous shield capacity, actual prevented events and genuine damage
reductions that still apply. These are proposed coverage, not verified recovery.

Malignus shares the new damage body but its half-highest-opponent-life
characteristic-defining P/T remains separately unsupported, so it is not counted.
Lava Burst and Whippoorwill forbid redirection as well as prevention; those riders
remain unimplemented and their full lines remain rejected. Volcano Hellion's
echo/amount-choice gaps, Alchemist's Gambit's extra-turn scope, split-face
Insult/Injury, Isengard Unleashed's paired replacement sentence and Everlasting
Torment's global wither substitution are outside this cohort.
