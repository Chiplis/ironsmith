# cf8 p10-other — fixer summary

157 frozen cards; branch `cf8/p10-other`. Nothing built or run (campaign policy). The prebuilt
probe was used for triage until it disappeared mid-session; later fixes are source-reasoned.
Ledger: 16 `source-proposed`, 141 `blocked`, 0 untriaged.

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
  reading → label `not during your first 3 turns` → engine `ThisSpellCastCondition::NotDuringYourFirstTurns`.
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

## Risks
- Schema: three appended `DelayedTriggerSpec` variants; FORMAT_VERSION/descriptor NOT bumped —
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
