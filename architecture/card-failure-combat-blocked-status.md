# Blocked status and exact cast timing (source proposal, UNVALIDATED)

Five frozen candidates: Choking Vines, Curtain of Light, Dazzling Beauty, Fog Patch and Trap Runner. The additional timing-only fixtures remain partial until all their other bodies are reviewed.

## Shared operations

`BecomeBlocked` is a typed permanent-state action lowered to one native/compiled effect. It marks the complete selected attacking set before refreshing role-dependent continuous state and creating transition receipts. It creates no blocker, applies no prevention shield and emits no repeated transition for an already-blocked attacker. Legal already-blocked targets remain in the result set so Choking Vines still deals damage to every selected creature. Zero-X/empty sets are legitimate resolutions. Discovery failure restores the original game rather than leaving partial blocked flags.

Native blocked status was already represented separately from live blocker assignments. Both combat-damage paths read that status; ordinary blocked attackers with no blocker assign no player damage while trample can still assign damage. Object departure and combat reset retain their existing exact-incarnation cleanup owners.

New spell-timing facts are typed in the compiled model. Old label-only payloads keep their prior serialized representation and conversion path. The declare-blockers restriction requires the completed declaration; the broader after-blockers restriction spans later combat steps. Own/opponent turn predicates retain actual team relations. No textual label is used to execute a newly added timing fact.

## Authored, unrun gates

Exact tools payloads and direct/artifact gameplay cover the five full bodies, paid X/tap costs, no-target X=0, already-blocked targets, actual combat damage with trample, no fake blockers, immediate versus next-turn draw, target incarnation and phase/declaration legality. Native properties assert one real transition receipt per attacker and no duplicate events; typed-model JSON compatibility and team/timing properties are also authored.

Rules basis: CR 509.1h and 510.1, with trample CR 702.19, in the frozen official rules at https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt . All tests remain unrun under the user's implementation-first instruction; no build, compiler probe or corpus replay was executed.

## Bounded review correction

Blocked transition receipts now use one checked calculated-characteristics frame for the entire completed selected set, rather than raw printed snapshots. Each attacker gets a distinct child event provenance so history staging cannot overwrite another transition from the same effect. Authored regressions cover blocked-role type/color/P/T changes, observer matching and two independent staged receipts under a valid parent. The effect does not advertise simultaneous-player proposal support; that composition remains rejected before mutation until it has an outer-completion receipt owner.
