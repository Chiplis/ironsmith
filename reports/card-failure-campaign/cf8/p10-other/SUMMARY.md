# cf8 p10-other — fixer summary

157 frozen cards; branch `cf8/p10-other`. Nothing built or run (campaign policy). The prebuilt
probe was used for triage until it disappeared mid-session; later fixes are source-reasoned.
Ledger: 27 `source-proposed`, 130 `blocked`, 0 untriaged, 0 `semantic-fix-collateral`.

## Clusters fixed (source-proposed)
- **delayed-damage-watchers** (Spiritualize, Paladin of Prahv, Glyph of Life, Lyra, The Last Ronin; Niko Aris partial):
  appended `DelayedTriggerSpec::{DealsDamage, DealsDamageTo, AttacksAlone}` + engine interpretation;
  lowering arms (generic / this-turn / duration-scoped) watching tagged objects (recipient-watched
  for DealsDamageTo); `ThisDealsCombatDamageToPlayer` → `DealsCombatDamageToPlayer{source: source()}`;
  grammar declares a targeted event subject ("whenever target creature deals damage") via
  `TagReferenced` explicit target + `match_tagged`, instead of matching any creature.
- **holder-relative-play-permission** (Suspend Aggression, Expedited Inheritance, March of Reckless Joy):
  "until (the) end of their next turn" in a permission tail = holder's next-turn lifetime;
  "up to N of those cards/them" → shared `max_plays`.
- **first-turns-cast-prohibition** (Serra Avenger, Jace Reawakened, Spider-Man 2099): early static
  reading → typed `ThisSpellCastTiming::NotDuringYourFirstTurns(3)` (appended core variant), engine
  counts the caster's turns taken (`turns_taken_by`, CR 500).
- **look-at-referenced-hand** (Port Inspector, Lay Bare): hand owners "defending player's" / "its controller's".
- **owner-same-name-cast-restriction** (Reflector Mage): `OwnerOf(tagged It)` subject + `SameNameAsTagged(It)` spell filter.
- **x-payment-maximum** (Shanna): "X can't be greater than N" followup sets `x_maximum` on the preceding `{X}` payment.
- **explicit-target-exile-pair** (Grip of Desolation): two coordinated exiles of independent targets.
- **cast-restriction partial**: "permanent spells" subject (Codie still blocked).

Tests (UNRUN): `crates/ironsmith-compiler-runtime/tests/{delayed_damage_event_watchers,
holder_relative_play_permissions, first_turns_cast_prohibition, look_at_referenced_players_hand,
owner_same_name_cast_restriction, x_payment_maximum_followup, explicit_target_exile_pairs}.rs`,
helpers in `tests/cf8_p10_support/mod.rs`. Structural assertions on both routes; no full
gameplay scenario for the delayed watchers.

- **object-restriction** (Blossombind, Revoke Privileges, Bound in Gold, Intercessor's Arrest,
  Goblin Brawler, Anti-Magic Aura, Consecrate Land): appended `Restriction::{BecomeUntapped,
  AttackBlockOrCrew, BeAttachedBy}`. BecomeUntapped feeds `cant_untap` + a new
  `cant_become_untapped` set that `GameState::untap` refuses; AttackBlockOrCrew adds attack/block
  bans and a `cant_crew` set excluded from crew candidates; BeAttachedBy is checked in
  `attachment_can_attach_to_target` (attach legality and SBA 704.5m/n).

- **library-look-put** (Coral Fighters, Dimir Machinations): library owner "defending player's";
  "put the rest back in any order" reuses `ReorderLibraryTopEffect` over the looked-at tag.
- **cast-restriction** (Proft): `ThisSpellCastRestrictionKind.condition` (appended serde-default
  field) → engine `ThisSpellCastCondition::Condition`, evaluated with the spell as source.
  Rakdos still blocked: its short self-name isn't normalized to a self-reference.
- **combat-restriction** (Bontu): attack/block-unless requirement falls back to the shared static
  condition grammar.

## Silent miscompile fixed
- Library placement ("put X and target Y on top/bottom of their owners' libraries") kept only one
  operand (source dropped the target) or merged two targets into one type union. It now splits
  two independently named references into two moves (CR 115.1d). Corpus grep (read-only
  cards.json) found only Void Stalker with this put shape (still blocked on "those players
  shuffle"); return/exile/shuffle pair cards (Aether Tradewinds, Churning Eddy, Peel from Reality,
  Rite of Undoing, Floodpits Drowner, Snow Hound, Wizard Mentor, Sandman, ...) go through other
  readers and were not changed. No `semantic-fix-collateral` rows.

## Root-cause hunt: If you do / Otherwise + duration (still open)
Ruled out (source reading): conditional-sentence-family, the `otherwise` pre-rule and
`try_merge_otherwise_into_previous_conditional`, post-parse followups, result-gate otherwise
binding in reference resolution, `parse_effect_sentences_lexed` finalization passes. The failure is
specific to an explicit "until end of turn" inside the Otherwise sentence; the reported
ETB-counter error is a fallback reading's diagnostic. Needs one traced debug build.

## Gameplay tests added
`delayed_damage_event_watchers_gameplay.rs` (deals-damage watcher: only the watched object,
combat and noncombat, expiry; attacks-alone: lone attacker fires, two attackers don't),
`cant_become_untapped.rs` (untap effect and primitive both refused).

## Risks
- Schema: appended `DelayedTriggerSpec::{DealsDamage, DealsDamageTo, AttacksAlone}`, `ThisSpellCastTiming::NotDuringYourFirstTurns`, `ThisSpellCastRestrictionKind.condition` field, `Restriction::{BecomeUntapped, AttackBlockOrCrew, BeAttachedBy}`; FORMAT_VERSION/descriptor NOT bumped —
  needs the coordinated boundary.
- Duration-scoped "whenever target creature deals combat damage …" elsewhere now declares/watches
  the target (fixes a silent target loss; old any-creature expectations would change).
- Jace Reawakened / Spider-Man 2099 other lines not re-probed after the binary vanished.

## Blocked by missing mechanic (see ledger for per-card gaps)
If-you-do/Otherwise with explicit duration (Pippin's Bravery, Insatiable Appetite, Spitting Slug);
"this mana" retention until end of combat (Avatar Roku, Fire Lord Ozai, Tundra Fumarole);
energy payment forms (Lightning Runner, Behemoth); Adventure-from-graveyard (Hildibrand, Mosswood);
condition/payment cast restrictions (Proft, Rakdos, Hogaak, Enthralling Hold, Dream Leash, Codie);
player restrictions (Angel of Jubilation, Karn's Sylex, Solemnity, City in a Bottle, Limited
Resources, Overwhelming Splendor, Call for Aid, Damping Engine, Shaman's Trance, Djinn);
object restrictions (crew, enchanted-by-Auras, equipped, untap, counters; 10 cards); combat
history/conditional combat restrictions & attack taxes (15 cards); targeting/cause/damage rules
(9); delayed conditional returns/destroys (5 + Niko); extra steps/damage assignment (6);
note/draft/secret choice (9); look/put/move library manipulation (27); misc singletons.

Pre-existing bug observed: "Put this creature and target creature on top of their owners'
libraries" drops the target; "Put target creature and target land …" collapses to an or-union.
