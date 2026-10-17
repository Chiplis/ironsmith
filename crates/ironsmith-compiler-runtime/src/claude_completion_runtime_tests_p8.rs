//! Claude package 8 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext};
use ironsmith::effects::{EffectContext as ExecutionContext, EffectExecutor};
use ironsmith::target::PlayerFilter;
use ironsmith::{CounterType, GameState, PlayerId, Zone};

/// Accepts every "may" and picks one fixed option index.
struct P8Choices {
    accept: bool,
    option: usize,
}

impl DecisionMaker for P8Choices {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.accept
    }

    fn decide_options(&mut self, _game: &GameState, _ctx: &SelectOptionsContext) -> Vec<usize> {
        vec![self.option]
    }
}

fn p8_game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}

#[test]
fn p8_xenosquirrels_removes_counter_to_adjust_die_result() {
    let definition = compile_to_runtime_definition("Xenosquirrels",
        "Type: Creature — Alien Squirrel\nPower/Toughness: 1/1\nThis creature enters with two +1/+1 counters on it.\nAfter you roll a die, you may remove a +1/+1 counter from this creature. If you do, increase or decrease the result by 1.", false)
        .expect("Xenosquirrels should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("source_counter_cost: Some(PlusOnePlusOne)"), "{debug}");
    assert!(debug.contains("once_each_turn: false"), "{debug}");

    let alice = PlayerId::from_index(0);
    let mut game = p8_game();
    let squirrels = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.add_counters(squirrels, CounterType::PlusOnePlusOne, 1);
    let roll = ironsmith::effects::RollDieEffect::new(PlayerFilter::You, 6);

    let mut increase = P8Choices { accept: true, option: 0 };
    game.force_next_die_roll(5);
    let mut ctx = ExecutionContext::new_default(squirrels, alice).with_decision_maker(&mut increase);
    let first = roll.execute(&mut game, &mut ctx).expect("roll resolves");
    assert_eq!(first.as_count(), Some(6), "the removed counter raised the result");
    assert_eq!(game.counter_count(squirrels, CounterType::PlusOnePlusOne), 0);
    assert_eq!(game.player(alice).unwrap().life, 20, "no life is paid");

    game.force_next_die_roll(5);
    let second = roll.execute(&mut game, &mut ctx).expect("roll resolves");
    assert_eq!(second.as_count(), Some(5), "without a counter the adjustment isn't offered");
}

#[test]
fn p8_dice_totals_feed_entry_counters_and_total_mana_value_choices() {
    let hydra = compile_to_runtime_definition("Neverwinter Hydra",
        "Mana cost: {X}{X}{G}{G}\nType: Creature — Hydra\nPower/Toughness: 0/0\nAs this creature enters, roll X d6. It enters with a number of +1/+1 counters on it equal to the total of those results.\nTrample\nWard {4} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {4}.)", false)
        .expect("Neverwinter Hydra should compile");
    let debug = format!("{:?}", hydra.abilities);
    assert!(debug.contains("AsEntersEffectProgram"), "{debug}");
    assert!(debug.contains("RollDiceChooseResultEffect") && debug.contains("total: true"), "{debug}");
    assert!(debug.contains("count_value: Some("), "the die count is X: {debug}");
    assert!(debug.contains("PutCountersEffect") && debug.contains("EffectValue"),
        "the entry counters read the roll inside the entry program: {debug}");
    assert!(!debug.contains("EntersWithCountersValue"), "no detached static entry counters: {debug}");

    let pair = compile_to_runtime_definition("Pair o' Dice Lost",
        "Mana cost: {3}{G}{G}\nType: Instant\nRoll two six-sided dice. Return any number of cards with total mana value X or less from your graveyard to your hand, where X is the total of those results. Exile Pair o' Dice Lost.", false)
        .expect("Pair o' Dice Lost should compile");
    let debug = format!("{:?}", pair.spell_effect);
    assert!(debug.contains("RollDiceChooseResultEffect") && debug.contains("total: true"), "{debug}");
    assert!(debug.contains("ManaValue") && debug.contains("EffectValue"),
        "the total mana value cap reads the dice total: {debug}");

    // Engine: the total of two simultaneously rolled dice, with no choice.
    let alice = PlayerId::from_index(0);
    let mut game = p8_game();
    let source = game.new_object_id();
    game.force_next_die_roll(2);
    game.force_next_die_roll(5);
    let roll = ironsmith::effects::RollDiceChooseResultEffect::new(PlayerFilter::You, 2, 6).with_total(None);
    let mut choices = P8Choices { accept: false, option: 0 };
    let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut choices);
    let outcome = roll.execute(&mut game, &mut ctx).expect("dice resolve");
    assert_eq!(outcome.as_count(), Some(7));
    assert_eq!(game.turn_store.turn_history.completed_die_roll_count(alice), 2, "both dice were rolled");
}

#[test]
fn p8_deck_of_many_things_rows_follow_the_zero_or_less_branch() {
    let definition = compile_to_runtime_definition("The Deck of Many Things",
        "Mana cost: {5}\nType: Legendary Artifact\n{2}, {T}: Roll a d20 and subtract the number of cards in your hand. If the result is 0 or less, discard your hand.\n1—9 | Return a card at random from your graveyard to your hand.\n10—19 | Draw two cards.\n20 | Put a creature card from any graveyard onto the battlefield under your control. When that creature dies, its owner loses the game.", false)
        .expect("The Deck of Many Things should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("RollDieEffect") && debug.contains("Subtract"), "{debug}");
    assert!(debug.contains("DiscardHand") || debug.contains("Discard"), "{debug}");
    assert_eq!(debug.matches("BetweenInclusive").count() + debug.matches("Equal(20)").count() >= 3, true,
        "three numeric rows on the roll: {debug}");
    assert!(debug.contains("ScheduleDelayedTrigger") && debug.contains("Dies"),
        "the 20 row watches the returned creature dying: {debug}");
    assert!(debug.contains("LoseTheGame") || debug.contains("LoseGame"), "{debug}");
}

#[test]
fn p8_swinging_ship_adds_a_combat_after_the_first_with_an_untap_rider() {
    let definition = compile_to_runtime_definition("Swinging Ship",
        "Type: Artifact — Attraction\nVisit — After the first combat phase this turn, there's an additional combat phase. At the beginning of that combat, untap all creatures that attacked this turn.", false)
        .expect("Swinging Ship should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("FirstCombatPhaseOfTurn"), "{debug}");
    assert!(debug.contains("AdditionalPhases") && debug.contains("Combat"), "{debug}");
    assert!(debug.contains("EndOfCombat"), "before combat, the phase is added at the first end of combat: {debug}");
    assert!(debug.contains("BeginningOfCombat"), "the untap rider waits for that combat: {debug}");
    assert!(debug.contains("Untap"), "{debug}");
    assert!(!debug.contains("AdditionalPhase::Main") && !debug.contains("Main]"), "no extra main phase: {debug}");
}

#[test]
fn p8_pick_a_beeble_claims_its_prize_inside_the_visit() {
    let definition = compile_to_runtime_definition("Pick-a-Beeble",
        "Type: Artifact — Attraction\nVisit — Roll a six-sided die. Put a number of luck counters on Pick-a-Beeble equal to the result and create a Treasure token. Then if there are six or more luck counters on Pick-a-Beeble, claim the prize!\nPrize — Create two Treasure tokens, then sacrifice Pick-a-Beeble and open an Attraction.", false)
        .expect("Pick-a-Beeble should compile");
    let visit = format!("{:?}", definition.spell_effect);
    assert!(visit.contains("Luck"), "{visit}");
    assert!(visit.contains("ClaimAttractionPrize"), "claiming is a keyword action: {visit}");
    assert!(visit.contains("Fixed(2)") && visit.contains("Treasure"), "the prize program runs at the claim: {visit}");
    assert!(visit.contains("Sacrifice") && visit.contains("OpenAttraction"), "{visit}");
    let claim = visit.find("ClaimAttractionPrize").unwrap();
    let open = visit.find("OpenAttraction").unwrap();
    assert!(claim < open, "the prize follows the claim: {visit}");
}

#[test]
fn p8_tunnel_of_love_pairs_an_opponents_choice_with_yours() {
    let definition = compile_to_runtime_definition("Tunnel of Love",
        "Type: Artifact — Attraction\nVisit — Choose an opponent. They choose a creature they control, then you choose a creature you control. You may exile the chosen creatures. If you do, return them to the battlefield under their owners' control at the beginning of the next end step. Otherwise, the chosen creatures fight each other.", false)
        .expect("Tunnel of Love should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("ChoosePlayer") || debug.contains("ChooseOpponent") || debug.contains("ChosenPlayer"), "{debug}");
    assert_eq!(debug.matches("ChooseObjectsEffect").count(), 2, "one choice each: {debug}");
    assert!(debug.contains("MayEffect") && debug.contains("ExileEffect"), "{debug}");
    assert!(debug.contains("ScheduleDelayedTrigger") || debug.contains("Delayed"), "return at the next end step: {debug}");
    assert!(debug.contains("FightEffect"), "{debug}");
}

#[test]
fn p8_wee_champion_reads_the_kind_of_sticker_placed() {
    let definition = compile_to_runtime_definition("Wee Champion",
        "Type: Creature — Minotaur Child Guest\nPower/Toughness: 2/1\nWhenever you place a sticker, this creature gets +1/+1 until end of turn. If it's an art sticker, instead put a +1/+1 counter on this creature.", false)
        .expect("Wee Champion should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Sticker"), "{debug}");
    assert!(debug.contains("TriggeringKeywordActionIs(ArtSticker)"), "{debug}");
    assert!(debug.contains("PlusOnePlusOne"), "{debug}");
}

#[test]
fn p8_roxi_counts_and_distributes_art_stickers() {
    let definition = compile_to_runtime_definition("Roxi, Publicist to the Stars",
        "Mana cost: {1}{U}\nType: Legendary Creature — Human Employee\nPower/Toughness: */3\nFlying\nRoxi's power is equal to the number of permanents you control with an art sticker plus the number of cards in your graveyard with an art sticker.\nWhen Roxi enters, distribute up to two art stickers among one or two nonland permanents you own.", false)
        .expect("Roxi should compile");
    let debug = format!("{:?}", definition.abilities);
    assert_eq!(debug.matches("sticker: Some(ArtSticker)").count(), 2,
        "both counted sets require an art sticker: {debug}");
    assert!(debug.contains("PutStickerEffect") && debug.contains("ArtSticker"), "{debug}");
    assert!(debug.contains("RepeatEffects") || debug.contains("Repeat"), "two placements: {debug}");
    assert!(debug.contains("MayEffect"), "up to two: {debug}");

    // Engine: the CDA counts art-stickered permanents and graveyard cards.
    use ironsmith::events::KeywordActionKind;
    let alice = PlayerId::from_index(0);
    let mut game = p8_game();
    let roxi = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let plain = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Plain Rock")
        .card_types(vec![ironsmith::types::CardType::Artifact]).build();
    let rock = game.create_object_from_definition(&plain, alice, Zone::Battlefield);
    let dead = game.create_object_from_definition(&plain, alice, Zone::Graveyard);
    let _unstickered = game.create_object_from_definition(&plain, alice, Zone::Battlefield);
    game.put_sticker_on_object(rock, KeywordActionKind::ArtSticker);
    game.put_sticker_on_object(dead, KeywordActionKind::ArtSticker);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(roxi), Some(2));
}

#[test]
fn p8_rocketship_counts_name_stickers_starting_with_the_chosen_letter() {
    let definition = compile_to_runtime_definition("_____ _____ Rocketship",
        "Mana cost: {4}{U}\nType: Artifact — Vehicle\nPower/Toughness: 3/3\nFlying\nWhen this Vehicle enters, you may put up to two name stickers on it.\nWhenever this Vehicle attacks, choose a letter. This Vehicle gets +1/+1 until end of turn for each name sticker on it that begins with the chosen letter.\nCrew 2", false)
        .expect("Rocketship should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ChooseNamedOptionEffect") && debug.contains("\"Z\""), "26 letters: {debug}");
    assert!(debug.contains("NameStickersOnSourceStartingWithChosenLetter"), "{debug}");
    assert!(debug.contains("NameSticker") && debug.contains("RepeatEffects"), "up to two placements: {debug}");

    // Engine: only stickers beginning with the chosen letter count.
    let alice = PlayerId::from_index(0);
    let mut game = p8_game();
    let ship = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.put_name_sticker_on_object(ship, "Rapid");
    game.put_name_sticker_on_object(ship, "Rusty");
    game.put_name_sticker_on_object(ship, "Gleaming");
    game.set_chosen_named_option(ship, "R".to_string());
    assert_eq!(game.name_sticker_count_starting_with(ship, 'r'), 2);
    let value = ironsmith::effect::Value::NameStickersOnSourceStartingWithChosenLetter { surface: None };
    let ctx = ExecutionContext::new_default(ship, alice);
    assert_eq!(ironsmith::effects::helpers::resolve_value(&game, &value, &ctx).unwrap(), 2);
}

#[test]
fn p8_vedalken_squirrel_whacker_rolls_its_base_pt_and_swaps_results() {
    let definition = compile_to_runtime_definition("Vedalken Squirrel-Whacker",
        "Mana cost: {3}{U}\nType: Creature — Vedalken Guest\nPower/Toughness: */*\nAs this creature enters, roll a six-sided die twice. Its base power becomes the first result and its base toughness becomes the second result.\nIf you would roll one or more six-sided dice, instead roll them and you may exchange one result with this creature's base power or base toughness.", false)
        .expect("Vedalken Squirrel-Whacker should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("AsEntersEffectProgram"), "{debug}");
    assert!(debug.contains("in_order: true"), "two ordered rolls: {debug}");
    assert!(debug.contains("OtherNumber"), "the second result: {debug}");
    assert!(debug.contains("exchange_with_source_base_pt: true") && debug.contains("sides: Some(6)"), "{debug}");

    // Engine: exchanging a d6 result with the base power.
    struct Swap;
    impl DecisionMaker for Swap {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { true }
        fn decide_options(&mut self, _: &GameState, _: &SelectOptionsContext) -> Vec<usize> { vec![0] }
    }
    let alice = PlayerId::from_index(0);
    let mut game = p8_game();
    let body = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Exchange Host")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 4))
        .with_ability(ironsmith::ability::Ability::static_ability(
            ironsmith::StaticAbility::die_roll_result_exchange_with_source_base_pt(PlayerFilter::You, Some(6), "exchange"),
        ))
        .build();
    let host = game.create_object_from_definition(&body, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    game.force_next_die_roll(5);
    let mut dm = Swap;
    let mut ctx = ExecutionContext::new_default(host, alice).with_decision_maker(&mut dm);
    let outcome = ironsmith::effects::RollDieEffect::new(PlayerFilter::You, 6).execute(&mut game, &mut ctx).unwrap();
    assert_eq!(outcome.as_count(), Some(2), "the die takes the old base power");
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(host), Some(5), "the base power becomes the roll");
    game.force_next_die_roll(5);
    let d20 = ironsmith::effects::RollDieEffect::new(PlayerFilter::You, 20).execute(&mut game, &mut ctx).unwrap();
    assert_eq!(d20.as_count(), Some(5), "only six-sided dice qualify");
}

#[test]
fn p8_wizards_spellbook_rows_copy_the_exiled_card() {
    let definition = compile_to_runtime_definition("Wizard's Spellbook",
        "Mana cost: {5}{U}{U}\nType: Artifact — Book\n{T}: Exile target instant or sorcery card from a graveyard. Roll a d20. Activate only as a sorcery.\n1—9 | Copy that card. You may cast the copy.\n10—19 | Copy that card. You may cast the copy by paying {1} rather than paying its mana cost.\n20 | Copy each card exiled with this artifact. You may cast any number of the copies without paying their mana costs.", false)
        .expect("Wizard's Spellbook should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(!debug.contains("CopySpellEffect"), "a card is copied, not a spell: {debug}");
    assert!(debug.contains("RollDieEffect"), "{debug}");
    assert_eq!(debug.matches("as_copy: true").count(), 3, "each row casts a copy: {debug}");
    assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
    assert!(debug.contains("ForEachTagged") || debug.contains("ForEachTaggedEffect"), "row 20 copies each exiled card: {debug}");
}

#[test]
fn p8_pin_collection_places_and_shares_ability_stickers() {
    let definition = compile_to_runtime_definition("Pin Collection",
        "Mana cost: {X}{W}\nType: Artifact — Equipment\nWhen this Equipment enters, you may put an ability sticker with ticket cost X or less on it without paying that sticker's ticket cost.\nEquipped creature gets +1/+1 and has all abilities of ability stickers on this Equipment.\nEquip {2}", false)
        .expect("Pin Collection should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("AttachedHasSourceAbilityStickerAbilities"), "{debug}");
    assert!(debug.contains("AbilitySticker") && debug.contains("without_paying_ticket_cost: true"), "{debug}");
    assert!(debug.contains("max_ticket_cost: Some("), "{debug}");

    // Engine: an ability sticker from the sheet is paid for in tickets and
    // grants its ability to the stickered permanent.
    use ironsmith::static_abilities::StaticAbilityId;
    let alice = PlayerId::from_index(0);
    let mut game = p8_game();
    let body = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Sticker Host")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
        .build();
    let host = game.create_object_from_definition(&body, alice, Zone::Battlefield);
    let ticket = CounterType::Named("ticket".into());
    game.add_player_counters_with_source(alice, ticket.clone(), 3, None, None).unwrap();
    game.add_accessible_ability_sticker(alice, 3,
        vec![ironsmith::ability::Ability::static_ability(ironsmith::StaticAbility::flying())], "Flying");
    let put = ironsmith::effects::PutStickerEffect::new(
        ironsmith::target::ChooseSpec::SpecificObject(host),
        ironsmith::events::KeywordActionKind::AbilitySticker,
    );
    let mut ctx = ExecutionContext::new_default(host, alice);
    put.execute(&mut game, &mut ctx).unwrap();
    game.refresh_continuous_state().unwrap();
    assert!(game.object_has_static_ability_id(host, StaticAbilityId::Flying), "the sticker's ability");
    assert_eq!(game.player(alice).unwrap().counter_count(ticket), 0, "its ticket cost was paid");
    assert_eq!(game.ability_sticker_abilities_on_object(host).len(), 1);
}

#[test]
fn p8_wicker_picker_grants_sticker_kicker_to_creature_spells() {
    let definition = compile_to_runtime_definition("Wicker Picker",
        "Mana cost: {3}\nType: Artifact Creature — Scarecrow Guest\nPower/Toughness: 2/3\nCreature spells you cast have sticker kicker {1}. (You may pay an additional {1} as you cast a creature spell. If you do, you get {TK}, then you may put a sticker on it.)", false)
        .expect("Wicker Picker should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("GrantSpellKeyword") && debug.contains("StickerKicker"), "{debug}");
    assert!(debug.contains("Fixed("), "the {{1}} price: {debug}");
}

#[test]
fn p8_myra_binds_a_visit_trigger_to_the_chosen_attraction_and_exiled_card() {
    let definition = compile_to_runtime_definition("Myra the Magnificent",
        "Mana cost: {2}{U}{R}\nType: Legendary Creature — Human Performer\nPower/Toughness: 2/4\nWhenever you cast an instant or sorcery spell from your hand, open an Attraction.\n{X}, {T}: Exile target instant or sorcery card with mana value X from your graveyard and choose an Attraction you control that doesn't have a midway counter on it. Put a midway counter on it. For as long as that Attraction is on the battlefield, whenever you visit it, copy the exiled card. You may cast the copy without paying its mana cost.", false)
        .expect("Myra the Magnificent should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("OpenAttraction"), "{debug}");
    assert!(debug.contains("midway"), "{debug}");
    assert!(debug.contains("VisitAttraction"), "the delayed trigger watches visits: {debug}");
    assert!(debug.contains("ScheduleDelayedTrigger"), "{debug}");
    assert!(debug.contains("as_copy: true") && debug.contains("without_paying_mana_cost: true"), "{debug}");
    assert!(!debug.contains("CopySpellEffect"), "{debug}");
}
