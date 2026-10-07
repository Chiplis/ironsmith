# Player hexproof retains the targeting controller

UNVALIDATED source work based on `6ca01fa1616181766b654f4b3f4ee6cc74ed8757`.
No builds, compilation, probes, tests, formatters, generated output, publication,
version changes, or coverage-ledger edits were performed.

## Separate owners

Player hexproof now lowers to `Restriction::PlayerHexproofFrom(player, qualities)`.
The implicit opponent relationship is no longer hidden in a physical source's
controller filter. The new native tracker evaluates `game.are_opponents` from
each protected player to the retained spell/ability controller. This includes
grants to all players and team games.

Qualified hexproof still matches color/type and other authored qualities against
the actual live source. An absent or phased-out source uses its retained
snapshot. Plain hexproof works when neither a live object nor a snapshot exists.
The snapshot's controller does not replace the ability's retained controller.

Protection and independent source-filtered prohibitions retain
`BeTargetedPlayerFrom` and match the physical source, including its current/LKI
controller. A permission to ignore hexproof belongs to the retained targeting
controller and cannot bypass protection or an independent prohibition. The
existing shroud prohibition remains independent.

`GameState::can_target_player_from_source_or_snapshot` receives both roles
explicitly. All three former computation helper calls and every source-aware
player arm of the public effect runtime validator now use it. Player,
ObjectOrPlayer, PlayerOrPlaneswalker, AnyTarget, and AnyOtherTarget retain this
context. The stack resolution/assignment callers already pass the retained
entry controller and source snapshot; their typed error, target-assignment,
combat, event, tag, and value contracts remain intact. The old source-only
convenience API is retained for callers asking about a live source's own
controller; it is no longer used by execution-context player targeting.

Temporary hexproof grants bind execution-only player references before recording
the duration, preserving `ExecutionError::InvalidTarget` on a missing target.
Source-quality choices are also bound during that resolution. Reference
resolution, iterated-player validation, text-change, counter-followup, synthetic
target folding, and compiled-text renderers retain the new typed restriction and
source-quality filter. Existing Everybody Lives and combined player/object
hexproof rendering patterns now require the distinct hexproof model.

## Authored tests, all unrun

`targeting/player_subject_tests.rs` covers retained controller after source theft,
absence with/without a snapshot, phasing, all-player team grants, live versus
retained source color, source-controller protection, independent protection
under an ignore-hexproof permission, permission ownership, shroud, and temporary
grant target/error preservation. The existing qualified-hexproof unit now uses
the distinct hexproof owner.

`qualified_mixed_target_lists.rs` compiles complete frozen All Will Be One and
Nicol Bolas, God-Pharaoh bodies directly and through serialized artifacts. Added
scenarios require an A-controlled trigger or -4/+2 ability targeting B to fizzle
after B gains hexproof and steals the source, including source departure and
phasing. Additional compiled/artifact witnesses cover source-quality protection,
all-player team grants, and duration expiration. The removed-target scenario
now follows the chosen object's stable identity into its graveyard incarnation
and checks surviving creature/planeswalker sentinels, rather than accepting zero
damage on an absent ID.

## Bounded prior-proposal impact

The authoritative campaign ledger consulted was the current
`ironsmith-card-campaign/fixtures/card-failure-campaign/source-coverage.json`.
Its All Will Be One and Nicol Bolas, God-Pharaoh entries are still `unaddressed`;
this lane does not describe them as restorations or change their status.

The following is a bounded source-confirmed subset, not an exhaustive inventory
of player-targeted cards. Each entry is already
`implemented_pending_full_corpus_validation` in that ledger; each listed
fixture's complete Oracle body was compared with frozen `cards-20261003.json.xz`
and matches exactly. The shared-owner defect on the base warrants a player
hexproof hold; this patch removes only that particular source-level hold,
subject to independent review and deferred validation. No new whole-body
promotion or verified recovery is claimed.

- Aether Revolt, family `additive-damage-replacements`,
  `fixtures/additive_damage_replacements.json.fixture`: its energy-receipt trigger
  deals damage to any target. A stolen live enchantment must not make its old
  controller's trigger legal against its new controller's hexproof. The Revolt
  replacement remains a separate source-owned instruction.
- Ancient Cellarspawn, family `player-life-damage-history-quantities`,
  `fixtures/life_history_quantities.json.fixture`: its completed-cast trigger
  targets an opponent for the mana-value/payment difference. Source theft cannot
  bypass that player's hexproof. The cost reduction and quantity binding are
  outside this correction.
- Heretic's Punishment, family `declared-any-target-programs`,
  `fixtures/declared_any_target_programs.json.fixture`: its activation declares
  any target before milling and damage. If that player becomes illegal after
  source theft, the ability must fizzle before milling as well as damage.
- Runebound Wolf, family `aggregate-damage-quantities`,
  `fixtures/aggregate_damage_quantities.json.fixture`: its activated ability
  targets an opponent, while the Wolves/Werewolves quantity retains its own
  established owner. The source's new controller cannot bypass player hexproof.
- Stonehorn Dignitary, family `scheduled-turn-skips`,
  `fixtures/scheduled_turn_skips.json.fixture`: its enter trigger targets an
  opponent. If that player gains hexproof and steals the creature, the pending
  old-controller trigger must not schedule a combat skip.

Other corpus candidates such as Circu, Dimir Lobotomist, Custodi Lich, Dovin,
Architect of Law, and Ral Zarek were not given a complete source-path review in
this bounded pass and receive no hold-removal or whole-body claim here. Keeper
of the Dead retains its ledger's dependent-target/full-body hold; Roiling Horror
retains its exact life-total-difference CDA hold. This targeting change does not
resolve either independent prerequisite.

## Serialization and API handoff

`PlayerHexproofFrom` is appended to the serialized `Restriction` enum, preserving
existing variant ordinals. Existing generic model materialization carries it to
the native restriction tracker; the tracker itself is derived, not a new saved
gameplay field. No signed/public checkpoint layout or versions were edited.

Old artifacts encoded player hexproof as the same `BeTargetedPlayerFrom` value
used for source-based protection. Runtime code cannot safely infer or rewrite
those legacy instances. When validation resumes, regenerate affected artifacts
from source and establish a coordinated reader/writer compatibility boundary
before publication. Old readers cannot consume the new enum variant, and old
hexproof artifacts do not acquire the corrected semantics merely by loading
them in a new reader. Any artifact/cache/version/handshake decisions remain with
the coordinating lane; this source commit is not a release-ready artifact.
