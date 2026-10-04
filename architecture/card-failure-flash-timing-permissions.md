# Scoped Flash timing: eight source proposals

UNVALIDATED. No compilation, build, parser probe, tests, formatting or corpus
replay ran. Measured campaign results remain 40 verified and 3,193 unresolved
unique cards. The scenarios below are authored gates awaiting validation.

## Complete frozen cohort

- Quick Sliver: `530a5740-041c-42df-8fe0-fcea87258038`
- Tidal Barracuda: `edfc0a46-414e-467d-9f27-e42d08ec127a`
- Vernal Equinox: `2241ef7d-3e9d-44ee-88ac-e8ad6fb2444e`
- Bard's Company: `5c5bfbb2-0e63-4e43-b441-c4878983288f`
- Illusion Spinners: `b608288e-1755-4738-b828-9df6998bae8d`
- Scarring Memories: `63957ec0-89e6-4be2-8740-846ca56ee48c`
- Fated Clash: `610d9f58-4182-49cb-855c-9b6030e7f714`
- Timely Ward: `8e589dca-d31e-4d2f-81b1-18e5f9198795`

All complete source rows are in `fixtures/flash_timing_permissions.json.fixture`,
SHA-256 `661bf766cc6d67b1f7da54894a69cebb714826e13d11d4410931df8a5604ab10`.

## Timing and origin are separate

The generic permission grammar already returns typed beneficiary/filter data;
its static consumer no longer only admits the exact You/noncreature variant.
Global Flash timing now uses a spell-domain grant, represented by the existing
Flash ability payload in Zone::Stack. This describes the proposed spell, not an
origin from which a card can be cast. It never produces PlayFrom authority.
The Hand-only native constructor stays Hand-limited; its meaning is unchanged.

The timing query checks only the chosen prospective face. A global spell-domain
grant receives a stack view of that face; an actual zone-limited card grant
receives its current origin, or the captured origin after proposal movement.
The former hardcoded Hand query and fallback to an unrelated front-face view
are removed. Consequently an independently authorized graveyard/exile/library
cast can receive global timing, and a Hand-only grant cannot lend timing to a
graveyard card. Global timing does not grant Flash as a battlefield keyword.

Static grant activation, controller/beneficiary assignment, source phasing and
source departure still use the existing continuous grant owner. Cast prohibitions
and sorcery-only rules remain authoritative. Completed proposals check timing
at CR 601.2e before costs; no new after-payment revocation owner was added.
The first-spell and combined free-and-flash grammar paths also select the
spell-domain timing constructor, while retaining their independent cost scopes.

## Self conditions and Aura targets

Trailing ordinary battlefield count/control predicates reuse the existing
labeled conditional Flash representation. Complete existential combat-state
clauses produce independent count predicates, including the conjunction of an
attacker and a blocker; one creature need not satisfy both. X, paid, Behold,
Teamwork, target-dependent and next-spell consumable surfaces remain outside
this state reader. Existing specialized target-dependent Flash still owns
Timely Ward. Its existential target preview now synthesizes the same Aura
attachment effect used by cast legality, instead of requiring a spell program
that an ordinary Aura does not have. Announced targets are rechecked by the
existing completed-proposal owner; later target changes are not timing checks.

## Whole-body authored coverage

Direct and restored-artifact scenarios include every complete body, global
caster and type domains, independent origins, source phasing, explicit Hand
scope, and MDFC face negatives. Barracuda's opponent prohibition still blocks
an instant on its controller's turn. Timely Ward needs an eligible commander,
keeps its actual Aura attachment, and grants only that creature indestructible.
Bard's Company performs real entry/attack Recruit draw/discard/token actions and
its other-creature anthem; a discarded land does not create a token. Illusion
Spinners retains flying and live untapped hexproof. Scarring Memories uses a
real attacking legend, then keeps sacrifice/discard/life loss on the targeted
opponent. Fated Clash requires both combat states, grants indestructibility to
the exact two targets before its wrath, destroys the other creatures, and lets
the granted protection expire normally. Nothing here establishes runtime closure.


## Cast-time count follow-up

The existing cast-time interpreter had unconditionally rejected CountComparison,
including ordinary control-count predicates. The bounded correction evaluates
MatchingFilter battlefield counts on a checked continuous query, excludes phased
objects, checks the exact count before the scalar comparison, and retains errors
through the public Result API. Existing bool adapters latch an incomplete result
for the outer legality or cast transaction rather than publishing it as a complete
negative. Other count domains are not enabled by this change. A direct checked
condition/discovery-failure scenario is authored and unrun.
