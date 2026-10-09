# cf8 p10-other — fixer summary

157 frozen cards; branch `cf8/p10-other`. Nothing built or run (campaign policy). The prebuilt
probe was used for triage until it disappeared mid-session; later fixes are source-reasoned.
Ledger (after round 7): 38 `source-proposed`, 119 `blocked`, 1 `semantic-fix-collateral` (Gomazoa), 0 untriaged.

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
  Rakdos: preprocessing now treats 'cast <short name>' as a self reference (round 3).
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

## Round 3: dependants from other packages (p10-owned mechanisms still missing)
- look/reveal top N + cast from among with a dynamic mana-value bound + rest to bottom in random
  order: Cosmic Cube, Invasion of Alara, Plargg and Nassari, Sunbird's Invocation, Talent of the
  Telepath (cast-from-among exists in permission_facts/tagged_surface; the dynamic bound and the
  combined procedure do not).
- "players can cast spells only during their own turns" (+ activation): City of Solitude, Fires of
  Invention (also needs "no more than two spells each turn").
- exile-until with cumulative mana value / per-opponent exile-until-nonland + free cast: Dream
  Harvest, Tasha's Hideous Laughter, Fevered Suspicion.
- other cast restrictions: Mana Maze, Moonhold, Rock Jockey, Haakon, Null Chamber, Ward of Bones,
  Suffocation, Angelic Arbiter, Peace Talks; draw-from-bottom (River Song); face-down look grant
  (Spy Network); treasure per card put into graveyard this way (Dihada).
None of these were implemented this round.

## Round 4: p10-owned mechanisms built (source-only, UNRUN)
1. **General attack taxes** (CR 508.1g-h, 611.2a) — commit 83474bef6.
   Static `AttackCost` gains serde-default `planeswalkers_only` (Onakke Oathkeeper). New appended
   `Restriction::AttackTax(AttackTaxRule { attackers, defenders: AttackTaxDefenders
   {Controller, ControllerOrPlaneswalkers, ControllerPlaneswalkers, Anyone}, mana_per_attacker: Value,
   life_per_attacker })` installed by resolving effects ("this turn" War Tax, "until your next turn"
   Sivitri); {X} is fixed at resolution in `normalize_restriction_for_resolution`. The engine folds
   active restriction-store taxes into `imposed_attack_costs_for_target`, so legality preview, cost
   locking and payment (mana and life) reuse the existing atomic attack-cost payer. Grammar:
   `parse_general_attack_tax_tokens` (cant_shapes/attack_tax.rs), wired into the cant-effect clause
   and the static negated-restriction reader. Cards: War Tax, Sivitri, Onakke (ledger source-proposed).
   Test: `tests/general_attack_taxes.rs` (structure + gameplay).
2. **Own-turn casting/activation + counted spell cap** — commit 444cf24de.
   City of Solitude: three statics over non-active players (cast prohibition, non-mana activation
   prohibition, all abilities of permanents they control incl. mana abilities). Fires of Invention:
   `cast_spells(Excluding{You, Active})` + appended `Restriction::CastMoreThanNSpellsEachTurn
   {player, spells, maximum}` with a counted cast-limit tracker enforced in `violates_any_cast_limit`.
   Test: `tests/own_turn_cast_restrictions.rs`. (Cards owned by p03.)
3. **Library procedures**
   - Looked cast-from-among with a trailing dynamic mana-value cap ("from among them with mana value
     less than or equal to the greatest power among attacking creatures you control") and the
     "cards revealed this way" collection alias — commit 5567be313. Cosmic Cube, Sunbird's Invocation
     (p04). Test: `tests/looked_cast_dynamic_cap.rs`.
   - Per-opponent exile-until with the perfect-tense cumulative stop ("until they have exiled cards
     with total mana value N or greater [this way]") wrapped in ForEachOpponent — commit aac0a364a.
     Tasha's Hideous Laughter; Dream Harvest's first sentence (its "cast cards exiled this way"
     permission sentence is not verified). Test: `tests/each_opponent_exile_until_total.rs`. (p02.)
   - Draw from the bottom (River Song) — commit 7d0522adb: appended `Restriction::DrawFromBottom`,
     tracker set consulted by `GameState::next_draw_card` (both draw primitives). Static rule
     `parse_you_draw_cards_from_bottom_line`. Test: `tests/draw_from_bottom_rule.rs`. (p07; its
     "Spoilers" trigger line not verified, and routing past the statement probe is unverified.)
   - Put onto the battlefield attacking a named player (CR 508.4) — commit 0cfd17627: appended
     `MoveToZoneAttackTargetMode::Player`, AST `battlefield_attack_player_only`, destination shape
     `attack_target` ("attacking that opponent" player-only; "that player or a planeswalker they
     control" the existing mode). Kaalia (source-proposed). Test: `tests/enter_attacking_named_player.rs`.

### Still blocked (p10-owned), precise gaps
- Void Stalker / Vortex Elemental: deduplicated "those players shuffle" over moved objects' owners.
- Gonti: look/exile/permission held by a non-you player (the damaging creature's controller).
- Aetherplasm: `MoveToZoneEffect` lacks `enters_blocking`.
- Jace, Multiverse Architect: attack prohibition against a planeswalker subtype set.
- Invasion of Alara, Plargg and Nassari: exile-until-two-matches / per-player exile-until with an
  opponent's choice, "one of those two" cast + hand split; Talent of the Telepath: spell-mastery
  "up to two ... instead of one" count replacement across the reveal procedure; Fevered Suspicion:
  "from among those nonland cards" (per-opponent match tag) free-cast collection.
- Mana Maze (most-recent-spell color), Moonhold (mana-spent-gated pair), Rock Jockey (cast/land
  cross history), Haakon (cast only from graveyard), Null Chamber (two-player name choice), Ward of
  Bones (per-type comparative player restrictions), Suffocation (damage-by-red-spell history),
  Angelic Arbiter (per-opponent history-conditioned restrictions), Peace Talks ("this turn and next
  turn" duration + player/permanent untargetability), Spy Network, Dihada: not built this round.

### Risks (round 4)
- Schema appends (no FORMAT_VERSION bump): `Restriction::{AttackTax, CastMoreThanNSpellsEachTurn,
  DrawFromBottom}`, `AttackTaxRule`/`AttackTaxDefenders` (core value_model), `StaticAbilityPayload::
  AttackCost.planeswalkers_only` (serde default), `MoveToZoneAttackTargetMode::Player`, AST
  `ZoneMoveActionAst::MoveToZone.battlefield_attack_player_only`.
- New `CantEffectTracker` fields `cant_cast_more_than`, `draws_from_bottom` (merge/clear updated).
- Own-turn restrictions rely on the tracker being recomputed when the active player changes (same
  assumption as the existing Dosan rule).
- City of Solitude leaves a gap: mana abilities activated from a hand (Elvish Spirit Guide) by a
  non-active player are not prohibited (the object prohibition covers permanents only).
- Debug-substring assertions in the new tests may need tightening after the first run.

## Round 5 (coordinator follow-up; source-only, UNRUN)
- **Tracker recompute confirmed**: `GameState::next_turn_single_lane_with_extra_turn_override` sets
  `turn.active_player` and ends with `update_cant_effects()` (turns_and_tracking.rs, "Printed static
  restrictions can switch on or off solely because the turn changed"); every
  `refresh_continuous_state` also rebuilds the tracker. No fix needed; a real-turn-change test was
  added (`own_turn_cast_restrictions.rs::city_of_solitude_follows_the_active_player_across_a_real_turn_change`).
1. **City of Solitude gap closed** (26ca3ea34): appended `Restriction::ActivateAbilities(PlayerFilter)`
   (every activation, mana abilities included, any zone). Tracker `cant_activate_abilities`;
   `can_activate_non_mana_abilities` consults it, and the mana-ability legality paths
   (`can_activate_mana_ability_with_cost_checks`, the payment precheck, the simple battlefield
   mana output) check `GameState::can_activate_abilities(player)`. City now lowers to cast ban +
   this player ban (replacing the two narrower statics). Test covers a hand mana ability (Elvish
   Spirit Guide shape).
2. **"Those players shuffle" once** (ab9105b19): the owners-library sentence reader now covers
   "this creature and each creature it's blocking" (Gomazoa), "... blocking or blocked by it"
   (Vortex, `in_combat_with_source`) and two independent references (Void Stalker). All objects move
   under one outcome tag, then one `ShuffleLibraryEffect(OwnerOf(tag))`; the engine shuffles each
   distinct owner exactly once in APNAP order (`distinct_tagged_owners`, CR 701.24a). Gomazoa was a
   silent miscompile (shuffled between moves) — logged as semantic-fix-collateral.
3. **Gonti**: NOT built — precise gap in the ledger (look owner vs viewer split in the AST).
4. **Enters blocking** (c99ec3c0a): `MoveToZoneEffect.enters_blocking: Option<ChooseSpec>`
   (appended, serde default) applied after battlefield entry through the shared
   `put_onto_battlefield_blocking` (CR 509.4); AST `battlefield_blocking`, destination shape
   "blocking that creature". Aetherplasm source-proposed.
5. **Jace** (ae481cb5c): appended `Restriction::AttackPermanents{attackers, permanents}`; tracker
   `cant_attack_permanents` checked in `can_attack_target_with_view` (CR 508.1b); negated tail
   "can't attack <planeswalker/battle filter>". **Zara** (b33600dfc): hand procedure opens on a
   defending player's hand look and binds "put a creature card from it ..." to that hand.
6. Invasion of Alara, Plargg and Nassari, Talent of the Telepath, Fevered Suspicion, Dream Harvest
   second sentence: NOT built this round.
7. Single-card restrictions (Mana Maze, Moonhold, Rock Jockey, Haakon, Null Chamber, Ward of Bones,
   Suffocation, Angelic Arbiter, Peace Talks), Spy Network, Dihada: NOT built. Findings: Haakon's
   "but not from anywhere else" can't use `Condition::SourceIsInZone` because the cast condition is
   re-evaluated on the stack; Peace Talks needs an "end of next turn" duration (no `Until` variant).

### Round 5 risks
- New appended schema: `Restriction::{ActivateAbilities, AttackPermanents}`,
  `MoveToZoneEffect.enters_blocking`, AST `MoveToZone.battlefield_blocking`; tracker fields
  `cant_activate_abilities`, `cant_attack_permanents`.
- `ShuffleLibraryEffect` with `OwnerOf(Tagged)` over a multi-object tag now shuffles every distinct
  owner (previously only the first object's owner). Any other card lowering "its owner shuffles"
  over a multi-object tag changes accordingly (intended).
- Owners-library reader replaces the Gomazoa-only reader; its "it's blocked by" negative test
  still declines.

## Round 6 (coordinator follow-up; source-only, UNRUN)
- **Gonti, Night Minister** (6c6e71d40, p10 ledger source-proposed): a 2/3-sentence pair
  procedure (`pair_procedure/viewer_face_down_play.rs`) builds the existing
  `PlayerLooksAtTopCardsOfLibrary` (viewer = triggering source's controller, library owner = that
  opponent) -> `LookAtTopCardsEffect.viewer`; the following face-down exile keeps that viewer's
  private view through the engine's remembered look viewers (`remember_face_down_exile_viewers`,
  the viewer-produced private-view path); the play permission (+ any-type mana rider) is held by the
  same player. No new AST variant.
- **Item 6 procedures**: Fevered Suspicion + Dream Harvest's second sentence (e42006d38):
  each-opponent consult followed by free casts over the aggregated stopping cards / a
  until-end-of-turn free-cast permission over every card exiled this way. Plargg and Nassari
  (703b7fb80): each-player consult, an opponent's exclusion choice, then up to N free casts among
  the other exiled nonland cards. NOT built: Invasion of Alara (exile until two matches, "one of
  those two" cast + hand split), Talent of the Telepath (spell-mastery line replaces the count of a
  statement on another line).
- **Item 7 singles**: Peace Talks (ce975c885): appended `RestrictionDurationSurface::ThisTurnAndNextTurn`;
  the engine extends the restriction's end-of-turn expiry by one turn (CR 611.2a); sentence
  reading for the leading "This turn and next turn," with the attack ban plus player and permanent
  untargetability by spells or activated abilities. Haakon (157f77348): graveyard cast permission +
  `only_if(SourceIsInZone(Graveyard))`, whose zone check now reads the proposed card's own zone
  (CR 601.3e) instead of a stack-time source lookup. Rock Jockey (d8cf48bc2): "can't cast this if
  <condition>" (negated cast condition) + land-play ban conditioned on source cast-and-entered this
  turn. Ward of Bones (4ae9b7a07): per-type `OpponentWithMoreControlledObjectsThan` cast/land
  restrictions incl. "The same is true for ...". Spy Network (2463e3b95): sentence reading keeps the
  listed hand/top-card/face-down look whole before comma splitting.
  NOT built: Mana Maze (most-recent-spell color history), Moonhold (two target-sharing restrictions
  each gated on a mana-spent predicate), Null Chamber (you and an opponent each name a card),
  Suffocation (damage-by-red-spell history + "the last such spell's controller"), Angelic Arbiter
  (player filters "cast a spell this turn" / "attacked with a creature this turn"), Dihada (the
  frozen error is a loyalty-cost counter quantity, unrelated to the Treasure count; not triaged).

### Round 6 risks
- `RestrictionDurationSurface::ThisTurnAndNextTurn` changes behaviour (expiry), not just text; the
  text renderers have no arm for it yet (fall back to their default wording).
- The Haakon zone check special-cases `Condition::SourceIsInZone` inside cast-time restrictions.
- New pair shapes (head "its"/"each") claim 2-3 sentence programs; they decline unless every
  sentence matches exactly.

## Round 7 (coordinator follow-up; source-only, UNRUN)
- **Peace Talks rendering** (65c309896): `CantEffect` with `ThisTurnAndNextTurn` renders "This turn
  and next turn, ..." (effect_impl/late.rs), and an effect-list renderer joins the card's three
  restrictions into the printed single clause list (effect_lists.rs). Sibling-surface renderers
  (clause_and_ability_surfaces, costs_and_triggers) only special-case `LeadingUntilEndOfTurn` /
  `LeadingUntilYourNextTurn` for coordinated target bundles and need no arm.
- **Angelic Arbiter** (666544f80): appended `PlayerFilter::TurnHistory(PlayerTurnHistoryFilter::
  {CastSpell, AttackedWithCreature})`, matched from turn history (`spell_cast_snapshot_history`,
  `creatures_attacked_by_player_this_turn`) in every game-aware matcher; arms added at all
  exhaustive PlayerFilter sites. Statics: attack ban / cast ban on `Excluding{TurnHistory, your team}`.
- **Mana Maze** (a2b0049bd): `ObjectFilter.shares_color_with_last_spell_cast_this_turn` (serde
  default) matched against the turn's latest cast snapshot; static cast ban for all players.
- **Moonhold** (36588e8f4): "<target> can't X this turn if {C} was spent ... and can't Y this turn if
  {D} was spent ..." -> one TargetOnly + two resolution-time `Conditional`s each gating its own
  end-of-turn restriction on its mana-spent predicate.
- **Null Chamber** (365695130): `ChooseCardNameAsEnters.{opponent_also_chooses,
  exclude_basic_land_names}` (serde defaults); the engine asks the next opponent for a second name and
  rejects basic land names (CR 205.4c); `PlayLandsMatching` now expands "{chosen name}" per recorded
  name like `CastSpellsMatching`; static reader for "Spells with the chosen names can't be cast and
  lands with the chosen names can't be played."
- **Dihada triage**: the frozen error is a correct rejection. Its −3 has no memory-producing effect
  between the loyalty cost and "Create a Treasure token for each card put into your graveyard this
  way", so the pending PutIntoGraveyard count resolves against the loyalty counter-cost producer
  (`bind_counter_cost_quantity`) and is refused. Fix belongs to the reveal/partition program (make the
  "the rest into your graveyard" move a PutIntoGraveyard memory producer); not done here.
- NOT built: **Suffocation** (cast condition is expressible as `Value::DamageHistory` >= 1, but the
  damage recipient "the controller of the last red instant or sorcery spell that dealt damage to you
  this turn" needs a player reference resolved from damage records); **Invasion of Alara** (exile
  until two matches; "one of those two" free cast + "one of them" to hand + rest to bottom);
  **Talent of the Telepath** (a spell-mastery line on another line replaces the count of the cast
  statement).

### Round 7 risks
- New appended schema: `PlayerFilter::TurnHistory`, `PlayerTurnHistoryFilter`,
  `ObjectFilter.shares_color_with_last_spell_cast_this_turn`, `ChooseCardNameAsEnters` fields.
  ObjectFilter has ~470 struct-literal sites; any exhaustive one without `..` needs the new field.
- `PlayLandsMatching` with "{chosen name}" now expands names (previously never matched).
