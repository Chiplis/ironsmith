//! Claude package 26 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::{PlayerId, Zone};
use ironsmith::effect::Effect;

#[test]
fn charging_hooligan_checks_for_any_attacking_rat() {
    let definition = compile_to_runtime_definition(
        "Charging Hooligan",
        "Mana cost: {3}{R}\nType: Creature — Human Peasant\nPower/Toughness: 2/2\nWhenever this creature attacks, it gets +1/+0 until end of turn for each attacking creature. If a Rat is attacking, this creature gains trample until end of turn.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("PlayerControls"), "{debug}");
    assert!(debug.contains("Rat"), "{debug}");
    assert!(debug.contains("attacking: true"), "{debug}");
}

#[test]
fn cephalid_constable_target_count_is_the_combat_damage_amount() {
    let definition = compile_to_runtime_definition(
        "Cephalid Constable",
        "Mana cost: {1}{U}{U}\nType: Creature — Octopus Wizard\nPower/Toughness: 1/1\nWhenever this creature deals combat damage to a player, return up to that many target permanents that player controls to their owner's hand.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("WithCountValue"), "{debug}");
    assert!(debug.contains("EventValue(Amount)"), "{debug}");
}

#[test]
fn celestial_convergence_highest_life_wins_unless_tied_then_draw() {
    let definition = compile_to_runtime_definition(
        "Celestial Convergence",
        "Mana cost: {2}{W}{W}\nType: Enchantment\nThis enchantment enters with seven omen counters on it.\nAt the beginning of your upkeep, remove an omen counter from this enchantment. If there are no omen counters on this enchantment, the player with the highest life total wins the game. If two or more players are tied for highest life total, the game is a draw.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("CountPlayers(MostLifeTied)"), "{debug}");
    assert!(debug.contains("DrawTheGame"), "{debug}");
    assert!(debug.contains("Omen"), "{debug}");
}

#[test]
fn choice_of_damnations_opponent_chooses_number_then_keeps_that_many() {
    let definition = compile_to_runtime_definition(
        "Choice of Damnations",
        "Mana cost: {5}{B}\nType: Sorcery — Arcane\nTarget opponent chooses a number. You may have that player lose that much life. If you don't, that player sacrifices all but that many permanents of their choice.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("ChooseNumberEffect"), "{debug}");
    assert!(debug.contains("LoseLifeEffect"), "{debug}");
    // The kept permanents are chosen, then every other one is sacrificed.
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
    assert!(debug.contains("kept"), "{debug}");
}

#[test]
fn chandra_chill_returns_a_surveiled_noncreature_nonland_card() {
    let definition = compile_to_runtime_definition(
        "Chandra, Chill of Compliance",
        "Mana cost: {1}{U}{U}\nType: Legendary Planeswalker — Chandra\nLoyalty: 4\n+1: Surveil 1. If you put a noncreature, nonland card into your graveyard this way, put that card into your hand.\n+1: Add {U}. Spend this mana only to cast a noncreature spell.\n−X: Tap target artifact or creature. Put X stun counters on it.\n−6: You get an emblem with \"Whenever you cast a spell, draw a card.\"",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("SurveilEffect"), "{debug}");
    assert!(debug.contains("PutIntoGraveyard"), "{debug}");
    assert!(debug.contains("excluded_card_types: [Creature, Land]") || debug.contains("Creature, Land"), "{debug}");
}

#[test]
fn case_file_auditor_triggers_on_entering_or_solving_a_case() {
    let definition = compile_to_runtime_definition(
        "Case File Auditor",
        "Mana cost: {2}{W}\nType: Creature — Human Detective\nPower/Toughness: 1/3\nWhen this creature enters and whenever you solve a Case, look at the top six cards of your library. You may reveal an enchantment card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.\nYou may spend mana as though it were mana of any color to cast Case spells.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("SolveCase"), "{debug}");
}


#[test]
fn call_for_aid_forbids_attacking_the_target_and_sacrificing_the_stolen_creatures() {
    let definition = compile_to_runtime_definition(
        "Call for Aid",
        "Mana cost: {4}{R}\nType: Sorcery\nGain control of all creatures target opponent controls until end of turn. Untap those creatures. They gain haste until end of turn. You can't attack that player this turn. You can't sacrifice those creatures this turn.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("AttackPlayer {"), "{debug}");
    assert!(debug.contains("BeSacrificed("), "{debug}");
    // The protected player is the targeted opponent, not an unbound iteration.
    let attack = &debug[debug.find("AttackPlayer {").unwrap()..];
    assert!(attack.contains("player: AliasedTarget(Opponent)"), "{attack}");
}

#[test]
fn celestial_reunion_gates_battlefield_on_paid_behold_and_chosen_type() {
    let definition = compile_to_runtime_definition(
        "Celestial Reunion",
        "Mana cost: {X}{G}\nType: Sorcery\nAs an additional cost to cast this spell, you may choose a creature type and behold two creatures of that type.\nSearch your library for a creature card with mana value X or less, reveal it, put it into your hand, then shuffle. If this spell's additional cost was paid and the revealed card is the chosen type, put that card onto the battlefield instead of putting it into your hand.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("ThisSpellPaidLabel") && debug.contains("LocalRewriteEffect"), "{debug}");
    assert!(debug.contains("chosen_creature_type: true"), "{debug}");
    assert!(debug.contains("Battlefield"), "{debug}");
}

#[test]
fn chromatic_armor_prevents_damage_from_the_current_chosen_color() {
    let definition = compile_to_runtime_definition(
        "Chromatic Armor",
        "Mana cost: {1}{W}{U}\nType: Enchantment — Aura\nEnchant creature\nAs this Aura enters, choose a color.\nThis Aura enters with a sleight counter on it.\nPrevent all damage that would be dealt to enchanted creature by sources of the last chosen color.\n{X}: Put a sleight counter on this Aura and choose a color. X is the number of sleight counters on this Aura.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("PreventMatchingDamage"), "{debug}");
    assert!(debug.contains("ChooseColorEffect"), "{debug}");
}

#[test]
fn call_to_the_void_destroys_both_secret_choices_of_each_player() {
    let definition = compile_to_runtime_definition(
        "Call to the Void",
        "Mana cost: {4}{B}\nType: Sorcery\nEach player secretly chooses a creature they control and a creature they don't control. Then those choices are revealed. Destroy each creature chosen this way.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.matches("SecretChoiceEffect").count() >= 2, "{debug}");
    assert!(debug.contains("secret_own_choice"), "{debug}");
    assert!(debug.contains("secret_other_choice"), "{debug}");
    assert!(debug.contains("Destroy"), "{debug}");
}

#[test]
fn captured_by_the_consulate_redirects_single_target_spells_to_enchanted_creature() {
    let definition = compile_to_runtime_definition(
        "Captured by the Consulate",
        "Mana cost: {3}{W}\nType: Enchantment — Aura\nEnchant creature you don't control\nEnchanted creature can't attack.\nWhenever an opponent casts a spell, if it has a single target, change the target to enchanted creature if able.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("RetargetStackObjectEffect"), "{debug}");
    assert!(debug.contains("OneToFixed"), "{debug}");
    assert!(debug.contains("target_count"), "{debug}");
}

#[test]
fn camel_prevents_desert_damage_to_itself_and_its_band_while_attacking() {
    let definition = compile_to_runtime_definition(
        "Camel",
        "Mana cost: {W}\nType: Creature — Camel\nPower/Toughness: 0/1\nBanding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)\nAs long as this creature is attacking, prevent all damage Deserts would deal to this creature and to creatures banded with this creature.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("PreventMatchingDamage"), "{debug}");
    assert!(debug.contains("Desert"), "{debug}");
    assert!(debug.contains("BandedWithTagged"), "{debug}");
}

#[test]
fn chaotic_transformation_exiles_each_optional_target_separately() {
    let definition = compile_to_runtime_definition(
        "Chaotic Transformation",
        "Mana cost: {5}{R}\nType: Sorcery\nExile up to one target artifact, up to one target creature, up to one target enchantment, up to one target planeswalker, and/or up to one target land. For each permanent exiled this way, its controller reveals cards from the top of their library until they reveal a card that shares a card type with it, puts that card onto the battlefield, then shuffles.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.matches("ExileEffect {").count() >= 5, "{debug}");
    assert!(debug.contains("Land"), "{debug}");
}

#[test]
fn caged_sun_adds_chosen_color_when_your_land_produces_it() {
    let definition = compile_to_runtime_definition(
        "Caged Sun",
        "Mana cost: {6}\nType: Artifact\nAs this artifact enters, choose a color.\nCreatures you control of the chosen color get +1/+1.\nWhenever a land's ability causes you to add one or more mana of the chosen color, add an additional one mana of that color.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("TriggeringManaIncludesSourceChosenColor"), "{debug}");
    assert!(debug.contains("ManaFromSourceTrigger"), "{debug}");
    assert!(!debug.contains("TapForManaTrigger"), "{debug}");
    let ironsmith::ability::AbilityKind::Triggered(ability) = &definition.abilities[2].kind else {
        panic!("expected a triggered mana ability");
    };
    assert_eq!(ability.trigger.display(), "Whenever a land's ability causes you to add one or more mana of the chosen color");
    assert!(debug.contains("AddMana"), "{debug}");
}

#[test]
fn caged_sun_toph_mana_loop_and_finite_controls() {
    use ironsmith::{GameState, CardId, CardBuilder, Color};
    use ironsmith::types::CardType;
    use ironsmith::mana::ManaSymbol;
    use ironsmith::events::ManaAddedEvent;
    use ironsmith::triggers::{check_triggers, TriggerQueue};
    use ironsmith::game_loop::{put_triggers_on_stack_with_dm, GameLoopError};

    let sun = compile_to_runtime_definition("Caged Sun",
        "Mana cost: {6}\nType: Artifact\nAs this artifact enters, choose a color.\nCreatures you control of the chosen color get +1/+1.\nWhenever a land's ability causes you to add one or more mana of the chosen color, add an additional one mana of that color.", false).unwrap();
    let toph = compile_to_runtime_definition("Toph, the First Metalbender",
        "Mana cost: {1}{R}{G}{W}\nType: Legendary Creature — Human Ally\nPower/Toughness: 3/3\nNontoken artifacts you control are lands in addition to their other types.\nAt the beginning of your end step, earthbend 2.", false).unwrap();
    // Both ordinary land production and Sun's own triggered production must
    // match. Wrong colors, other recipients, and nonlands must not match.
    for (with_toph, source_is_sun, color, opponent, expected) in [
        (true, false, ManaSymbol::Green, false, 1),
        (true, true, ManaSymbol::Green, false, 1),
        (false, false, ManaSymbol::Green, false, 1),
        (false, true, ManaSymbol::Green, false, 0),
        (true, false, ManaSymbol::Blue, false, 0),
        (true, false, ManaSymbol::Green, true, 0),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let sun_id = game.create_object_from_definition(&sun, alice, Zone::Battlefield);
        game.set_chosen_color(sun_id, Color::Green);
        if with_toph { game.create_object_from_definition(&toph, alice, Zone::Battlefield); }
        let land = CardBuilder::new(CardId::new(), "Test Land")
            .card_types(vec![CardType::Land]).build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
        let source = if source_is_sun { sun_id } else { land_id };
        let recipient = if opponent { bob } else { alice };
        // No tap provenance: this also exercises non-tapping mana abilities.
        let event = ManaAddedEvent::trigger_event(source, alice, recipient, vec![color]);
        let entries = check_triggers(&game, &event);
        assert_eq!(entries.len(), expected, "toph={with_toph}, sun={source_is_sun}, color={color:?}, opponent={opponent}");
        let mut queue = TriggerQueue::new();
        for entry in entries { queue.add(entry); }
        let result = put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut ironsmith::decision::AutoPassDecisionMaker);
        if with_toph && expected == 1 {
            assert_eq!(result, Err(GameLoopError::MandatoryLoopDraw));
        } else {
            assert!(result.is_ok(), "{result:?}");
            assert_eq!(game.player(alice).unwrap().mana_pool.amount(ManaSymbol::Green), expected as u32);
        }
        assert!(game.stack.is_empty(), "mana triggers never use the stack");
    }

    // Exercise the user-facing activation route as well as trigger dispatch.
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    game.turn.priority_player = Some(alice);
    let sun_id = game.create_object_from_definition(&sun, alice, Zone::Battlefield);
    game.set_chosen_color(sun_id, Color::Green);
    game.create_object_from_definition(&toph, alice, Zone::Battlefield);
    let forest = CardBuilder::new(CardId::new(), "Forest")
        .card_types(vec![CardType::Land]).subtypes(vec![ironsmith::types::Subtype::Forest]).build();
    let land_id = game.create_object_from_card(&forest, alice, Zone::Battlefield);
    let action = ironsmith::decision::compute_legal_actions(&game, alice).unwrap().into_iter()
        .find(|action| matches!(action, ironsmith::decision::LegalAction::ActivateManaAbility { source, .. } if *source == land_id))
        .expect("the land's mana ability remains legal even though it causes a draw");
    let mut queue = TriggerQueue::new();
    let mut state = ironsmith::game_loop::PriorityLoopState::new(game.players_in_game());
    let progress = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game, &mut queue, &mut state,
        &ironsmith::game_loop::PriorityResponse::PriorityAction(action),
        &mut ironsmith::decision::AutoPassDecisionMaker,
    ).unwrap();
    assert!(matches!(progress, ironsmith::decision::GameProgress::GameOver(ironsmith::decision::GameResult::Draw)), "{progress:?}");

}
