# Source-filtered targeting prohibitions

UNVALIDATED source work. No builds, compilation, test execution or replay was
performed. Eight exact baseline failures are proposed: Dense Foliage,
Fiendslayer Paladin, Shanna, Sisay's Legacy, Ground Seal, Silent Gravestone,
Underworld Cerberus, Thrun, Breaker of Silence and Display of Dominance.
The full frozen inputs are in `fixtures/source_filtered_targeting.json.fixture`,
from corpus SHA-256
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.
Measured totals remain 40 recovered and 3,193 unresolved unique identities.

## Grammar and executable meaning

The complete source-filtered negated-restriction reading now accepts unqualified
spells, unqualified spells-or-abilities, single-kind controller tails and paired
spell/source qualifiers with explicit controller tails. Existing typed
`Restriction::BeTargeted` / `BeTargetedFrom` values are retained. A spell-only
filter remains explicitly `StackObjectKind::Spell`; an ability-only filter is
`Ability`. The paired Thrun form requires equal complete spell/source qualities,
like the existing Gaea's Revenge form; mismatched qualifiers remain rejected.
No general `unless` history or partial tail is silently consumed.

A stack-kind source filter describes the targeting spell or ability, including
its controller. An untyped source-quality filter describes its actual source.
This preserves the distinction between Shanna's opponent-controlled abilities
and Thrun's abilities from opponent-controlled nongreen sources. Changing a
permanent's controller after its ability goes on the stack does not change the
ability's controller. Current source characteristics are used while available;
retained source information is used after it leaves or phases out.

## Native legality enforcement

- Target restriction materialization enumerates all current objects and honors
  the target filter's actual zone, including graveyards.
- Explicit prohibitions are checked before the ordinary nonbattlefield early
  return. They bind both players and cannot be bypassed by permissions to ignore
  hexproof or shroud; those are distinct abilities.
- Source roles are evaluated before stripping stack-only structural constraints
  for a source-characteristic match. Prospective spell casting is identified by
  the existing derived-view marker. Cast triggers still count as abilities even
  while their source is a spell on the stack. Costless/copied spells remain spells.
- Stack-entry target reassignment and resolution validation now propagate the
  entry's explicit spell role. Both kinds of entries retain source snapshots;
  snapshot presence alone is not proof of an ability.
- Both native static-restriction scan paths exclude phased-out hosts, and
  phased-out ability sources use their retained characteristics.
- A graveyard card leaving and returning is a new object, so the rebuilt tracker
  cannot retain a prohibition tied to its obsolete incarnation.

Display of Dominance retains the existing temporary rule restriction. Its
recipient filter remains dynamic: later permanents and controller changes during
the turn are included; cleanup expires the rule. This follows the distinction
between characteristic changes and rule changes in CR 611.2c. Target legality
is checked again at resolution under CR 608.2b. See the official
[Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf).

One grammar, two native targeting and nine direct/restored-artifact runtime
scenarios are authored and unrun. They cover all complete cards, prospective
casting, spell/ability/controller distinctions, cast-trigger source roles,
source/host phase-out and departure, graveyard current incarnations, permissions
that ignore hexproof/shroud, later Display recipients and cleanup, and actual
spell/activated/triggered program resolution after target legality changes.

Dennick's composite identity is not counted because both faces still need a
separate full-face review. Lurker's conditional combat history, attachment bans,
and mismatched source-quality disjunctions remain outside this subset.
