//! Claude package 44 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::types::{CardType, Subtype};
use ironsmith::{GameState, PlayerId, Zone};

fn p44_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"))
}

const EXILED_DOOMSAYER: &str = "Mana cost: {1}{W}\nType: Creature — Human Cleric\nPower/Toughness: 1/2\nAll morph costs cost {2} more. (This doesn't affect the cost to cast creature spells face down.)";

#[test]
fn exiled_doomsayer_increases_every_players_morph_costs() {
    use ironsmith::special_actions::{TurnFaceUpMethod, turn_face_up_cost_display};
    let definition = p44_compile("Exiled Doomsayer", EXILED_DOOMSAYER);
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("MorphCostIncrease"), "{debug}");
    assert!(debug.contains("generic_increase: 2"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let morph_card = CardBuilder::new(CardId::new(), "Morph witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mana_morph = game.create_object_from_card(&morph_card, alice, Zone::Battlefield);
    game.object_mut(mana_morph).unwrap().abilities_mut().push(
        ironsmith::ability::Ability::static_ability(ironsmith::static_abilities::StaticAbility::morph(
            ironsmith::cost::TotalCost::mana(ironsmith::mana::ManaCost::from_symbols(vec![
                ironsmith::mana::ManaSymbol::Green,
            ])),
        )),
    );
    game.set_face_down(mana_morph);
    let life_morph = game.create_object_from_card(&morph_card, alice, Zone::Battlefield);
    game.object_mut(life_morph).unwrap().abilities_mut().push(
        ironsmith::ability::Ability::static_ability(ironsmith::static_abilities::StaticAbility::morph(
            ironsmith::cost::TotalCost::from_cost(ironsmith::costs::Cost::life(5)),
        )),
    );
    game.set_face_down(life_morph);
    game.refresh_continuous_state().unwrap();
    let before_mana = turn_face_up_cost_display(&game, mana_morph, TurnFaceUpMethod::TurnFaceUpAbility)
        .unwrap()
        .expect("morph cost");
    let before_life = turn_face_up_cost_display(&game, life_morph, TurnFaceUpMethod::TurnFaceUpAbility)
        .unwrap()
        .expect("morph cost");
    assert!(!before_mana.contains("{2}"), "{before_mana}");
    assert!(!before_life.contains("{2}"), "{before_life}");

    // An opponent's Doomsayer still taxes Alice's morph costs.
    game.create_object_from_definition(&definition, bob, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let after_mana = turn_face_up_cost_display(&game, mana_morph, TurnFaceUpMethod::TurnFaceUpAbility)
        .unwrap()
        .expect("morph cost");
    let after_life = turn_face_up_cost_display(&game, life_morph, TurnFaceUpMethod::TurnFaceUpAbility)
        .unwrap()
        .expect("morph cost");
    assert!(after_mana.contains("{2}") && after_mana.contains("{G}"), "{after_mana}");
    assert!(after_life.contains("{2}"), "a non-mana morph cost still costs {{2}} more: {after_life}");
}

const EVELYN: &str = "Mana cost: {2}{U/B}{B}{B/R}\nType: Legendary Creature — Vampire Rogue\nPower/Toughness: 2/5\nFlash\nWhenever Evelyn or another Vampire you control enters, exile the top card of each player's library with a collection counter on it.\nOnce each turn, you may play a card from exile with a collection counter on it if it was exiled by an ability you controlled, and you may spend mana as though it were mana of any color to cast it.";

#[test]
fn evelyn_exiles_each_players_top_card_with_a_collection_counter_and_may_play_them() {
    let definition = p44_compile("Evelyn, the Covetous", EVELYN);
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(triggered) => Some(triggered.clone()),
            _ => None,
        })
        .expect("enters trigger");
    let grant = format!("{:?}", definition.abilities);
    assert!(grant.contains("__exiled_by_you__"), "{grant}");
    assert!(grant.contains("OnceEachTurn"), "{grant}");
    assert!(grant.contains("AnyColor"), "{grant}");
    assert!(grant.contains("collection"), "{grant}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    let card = |name: &str| {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .build()
    };
    for player in [alice, bob, cara] {
        game.create_object_from_card(&card("Bottom"), player, Zone::Library);
        game.create_object_from_card(&card("Top"), player, Zone::Library);
    }
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    for effect in triggered.effects.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let exiled: Vec<_> = game.exile.iter().copied().collect();
    assert_eq!(exiled.len(), 3, "one card from each player's library");
    let collection = ironsmith::object::CounterType::Named("collection".into());
    for id in &exiled {
        assert_eq!(game.counter_count(*id, collection.clone()), 1, "each exiled card has one counter");
    }
    for player in [alice, bob, cara] {
        assert_eq!(game.player(player).unwrap().library.len(), 1);
    }
}

const ERIETTE: &str = "Mana cost: {1}{W}{U}{B}\nType: Legendary Creature — Human Warlock\nPower/Toughness: 2/4\nLifelink\nWhenever an Aura you control becomes attached to a nonland permanent an opponent controls with mana value less than or equal to that Aura's mana value, gain control of that permanent for as long as that Aura is attached to it.";

#[test]
fn eriette_steals_permanents_whose_mana_value_fits_under_the_attached_aura() {
    use ironsmith::mana::{ManaCost, ManaSymbol};
    let definition = p44_compile("Eriette, the Beguiler", ERIETTE);
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ManaValueLteTagged"), "{debug}");
    assert!(debug.contains("__trigger_attachment__"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let aura_card = p44_compile("Two-drop Aura", "Mana cost: {2}\nType: Enchantment — Aura\nEnchant creature");
    let creature = |name: &str, generic: u8| {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(generic)]))
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    };
    let big = game.create_object_from_card(&creature("Three-drop", 3), bob, Zone::Battlefield);
    let small = game.create_object_from_card(&creature("Two-drop", 2), bob, Zone::Battlefield);
    let big_aura = game.create_object_from_definition(&aura_card, alice, Zone::Battlefield);
    let small_aura = game.create_object_from_definition(&aura_card, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();

    let mut queue = ironsmith::triggers::TriggerQueue::new();
    assert!(game.attach_object_to_target(big_aura, ironsmith::object::AttachmentTarget::Object(big)));
    ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 0, "mana value 3 exceeds the Aura's mana value 2");

    assert!(game.attach_object_to_target(small_aura, ironsmith::object::AttachmentTarget::Object(small)));
    ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1, "mana value 2 fits under the Aura");
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.controller_of(game.object(small).unwrap()), alice, "Eriette steals the permanent, not the Aura");
    assert_eq!(game.controller_of(game.object(small_aura).unwrap()), alice);

    assert!(game.detach_object_from_current_target(small_aura));
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.controller_of(game.object(small).unwrap()), bob, "control ends once the Aura is unattached");
}

const EMISSARY_OF_GRUDGES: &str = "Mana cost: {5}{R}\nType: Creature — Efreet\nPower/Toughness: 6/5\nFlying, haste\nAs this creature enters, secretly choose an opponent.\nReveal the player you chose: Choose new targets for target spell or ability if it's controlled by the chosen player and if it targets you or a permanent you control. Activate only once.";

#[test]
fn emissary_of_grudges_keeps_both_retarget_conditions() {
    let definition = p44_compile("Emissary of Grudges", EMISSARY_OF_GRUDGES);
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => Some(format!("{activated:?}")),
            _ => None,
        })
        .expect("retarget ability");
    assert!(activated.contains("RetargetStackObjectEffect"), "{activated}");
    assert!(activated.contains("ConditionalEffect"), "{activated}");
    assert!(activated.contains("ChosenPlayer"), "controlled by the chosen player: {activated}");
    assert!(activated.contains("targets_player: Some(You)"), "targets you: {activated}");
    assert!(activated.contains("targets_any_of: true"), "you or a permanent you control: {activated}");
    assert!(activated.contains("SpellOrAbility"), "{activated}");
    assert!(activated.contains("MaxActivationsPerObject"), "{activated}");
}

const ENDURING_ANGEL_FRONT: &str = "Mana cost: {2}{W}{W}{W}\nType: Creature — Angel\nPower/Toughness: 3/3\nFlying, double strike\nYou have hexproof.\nIf your life total would be reduced to 0 or less, instead transform this creature and your life total becomes 3. Then if this creature didn't transform this way, you lose the game.";

#[test]
fn enduring_angel_replaces_a_lethal_life_reduction() {
    let definition = p44_compile("Enduring Angel", ENDURING_ANGEL_FRONT);
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("LifeReducedToZeroOrLess"), "{debug}");
    assert!(debug.contains("TransformEffect"), "{debug}");
    assert!(debug.contains("LoseTheGameEffect"), "{debug}");
    assert!(debug.find("SetLifeTotalEffect").unwrap() < debug.find("LoseTheGameEffect").unwrap(),
        "life is set before testing the transform failure: {debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let spell = CardBuilder::new(CardId::new(), "Life loss witness")
        .card_types(vec![CardType::Sorcery])
        .build();
    let source = game.create_object_from_card(&spell, bob, Zone::Stack);
    game.refresh_continuous_state().unwrap();

    // A nonlethal loss is not replaced.
    let lose = |amount: i32| {
        Effect::new(ironsmith::effects::LoseLifeEffect::new(
            amount,
            ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(alice)),
        ))
    };
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, bob);
    ironsmith::effects::execute_effect(&mut game, &lose(5), &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(game.player(alice).unwrap().life, 15);

    // A lethal loss sets the life total to 3 instead. This front-face-only
    // definition cannot transform, so the fallback loses the game.
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, bob);
    ironsmith::effects::execute_effect(&mut game, &lose(20), &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(game.player(alice).unwrap().life, 3, "the reduction is replaced");
    assert!(game.player(alice).unwrap().has_lost, "an Angel that didn't transform loses the game");
}

const ERESTOR: &str = "Mana cost: {1}{G}{U}\nType: Legendary Creature — Elf Noble\nPower/Toughness: 2/4\nWhenever players finish voting, each opponent who voted for a choice you voted for creates a Treasure token. You scry X, where X is the number of opponents who voted for a choice you didn't vote for. Draw a card.";

#[test]
fn erestor_reads_both_vote_relation_groups() {
    let definition = p44_compile("Erestor of the Council", ERESTOR);
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("voted_with_you"), "Treasure for opponents who agreed: {debug}");
    assert!(debug.contains("voted_against_you"), "scry count from opponents who disagreed: {debug}");
    assert!(debug.contains("CountPlayers"), "{debug}");
    assert!(debug.contains("Treasure"), "{debug}");
    assert!(debug.contains("Scry"), "{debug}");
}

const EQUIPOISE: &str = "Mana cost: {2}{W}\nType: Enchantment\nAt the beginning of your upkeep, for each land target player controls in excess of the number you control, choose a land that player controls, then the chosen permanents phase out. Repeat this process for artifacts and creatures. (While they're phased out, they're treated as though they don't exist. They phase in before that player untaps during their next untap step.)";

#[test]
fn equipoise_phases_out_the_excess_of_each_card_type() {
    let definition = p44_compile("Equipoise", EQUIPOISE);
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(triggered) => Some(format!("{triggered:?}")),
            _ => None,
        })
        .expect("upkeep trigger");
    assert_eq!(triggered.matches("PhaseOut").count() >= 3, true, "{triggered}");
    assert_eq!(triggered.matches("ChooseObjectsEffect").count(), 3, "one choice per card type: {triggered}");
    for card_type in ["Land", "Artifact", "Creature"] {
        assert!(triggered.contains(card_type), "{card_type}: {triggered}");
    }
    assert!(triggered.contains("Scaled"), "count in excess of your own: {triggered}");
}
