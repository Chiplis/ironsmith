use super::*;

const CHAPTER_II: &str = "II — When you next cast a creature spell this turn, that creature enters with an additional +1/+1 counter on it.";

fn compile_saga_chapter(text: &str) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Entry Counter Probe")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .parse_text(text)
        .expect("next-cast Saga chapter should compile")
}

#[test]
fn next_creature_spell_entry_counter_uses_a_stable_identity_replacement() {
    let text = CHAPTER_II;
    let definition = compile_saga_chapter(text);
    let debug = format!("{definition:#?}");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
    assert!(
        debug.contains("RegisterNextBatchEnterWithCountersEffect"),
        "{debug}"
    );
    assert!(debug.contains("same_stable_id_tag: Some"), "{debug}");
    assert!(!debug.contains("PutCountersEffect"), "{debug}");
}

#[test]
fn ordinary_next_cast_counter_placement_is_not_promoted_to_entry_replacement() {
    let text = "II — When you next cast a creature spell this turn, put a +1/+1 counter on it.";
    let definition = compile_saga_chapter(text);
    let debug = format!("{definition:#?}");

    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(
        !debug.contains("RegisterNextBatchEnterWithCountersEffect"),
        "{debug}"
    );
}

fn resolve_entry_counter_chapter(
    definition: &crate::cards::CardDefinition,
) -> (crate::game_state::GameState, crate::ids::ObjectId) {
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    game.turn.active_player = alice;
    let source = game.create_object_from_definition(definition, alice, Zone::Battlefield);
    game.object_mut(source)
        .unwrap()
        .counters
        .insert(crate::CounterType::Lore, 1);

    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::add_saga_lore_counters(&mut game, &mut queue).unwrap();
    assert_eq!(
        queue.entries.len(),
        1,
        "the second lore counter triggers chapter II"
    );
    assert_eq!(
        queue.entries[0].ability.trigger.saga_chapters(),
        Some(&[2][..])
    );
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    (game, source)
}

fn cast_and_resolve_entry_counter_fixture(
    game: &mut crate::game_state::GameState,
    caster: crate::ids::PlayerId,
    card_type: CardType,
    expected_triggers: usize,
) -> crate::ids::ObjectId {
    let card = crate::cards::builders::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Cast Fixture",
    )
    .card_types(vec![card_type])
    .mana_cost(crate::mana::ManaCost::from_symbols(vec![
        crate::mana::ManaSymbol::Generic(0),
    ]))
    .power_toughness(crate::card::PowerToughness::fixed(2, 2))
    .flash()
    .build();
    let hand_card = game.create_object_from_definition(&card, caster, Zone::Hand);
    let stable_id = game.object(hand_card).unwrap().stable_id;
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(caster);
    let action = crate::decision::compute_legal_actions(game, caster).expect("fixture has complete replacement state")
        .into_iter()
        .find(|action| matches!(action, crate::decision::LegalAction::CastSpell { spell_id, .. } if *spell_id == hand_card))
        .expect("the fixture should be castable from hand");
    let mut queue = crate::triggers::TriggerQueue::new();
    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut progress = crate::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..12 {
        if game.stack.iter().any(|entry| !entry.is_ability) {
            break;
        }
        let crate::decision::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("casting the fixture stalled: {progress:?}");
        };
        progress = crate::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &ctx, &mut dm,
        )
        .unwrap();
    }
    crate::game_loop::put_triggers_on_stack(game, &mut queue).unwrap();
    assert_eq!(
        game.stack.len(),
        1 + expected_triggers,
        "the real casting path must put the chapter's delayed trigger above the creature spell"
    );
    let spell = game.stack[0].object_id;
    assert!(!game.stack[0].is_ability);
    for _ in 0..expected_triggers {
        crate::game_loop::resolve_stack_entry(game).unwrap();
    }
    assert_eq!(game.object(spell).unwrap().zone, Zone::Stack);
    assert_eq!(
        game.object(spell)
            .unwrap()
            .counters
            .get(&crate::CounterType::PlusOnePlusOne),
        None,
        "the cast trigger must arrange entry counters, not put counters on the spell"
    );
    crate::game_loop::resolve_stack_entry(game).unwrap();
    let entered = game.find_object_by_stable_id(stable_id).unwrap();
    assert_ne!(
        entered, spell,
        "resolving the spell creates a new zone object"
    );
    assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
    entered
}

fn plus_one_counters(game: &crate::game_state::GameState, id: crate::ids::ObjectId) -> u32 {
    game.object(id)
        .unwrap()
        .counters
        .get(&crate::CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0)
}

#[test]
fn chapter_ii_creature_enters_with_counter_even_after_saga_leaves() {
    let definition = compile_saga_chapter(CHAPTER_II);
    for remove_source in [false, true] {
        let (mut game, source) = resolve_entry_counter_chapter(&definition);
        let alice = game.players[0].id;
        if remove_source {
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        }
        let entered =
            cast_and_resolve_entry_counter_fixture(&mut game, alice, CardType::Creature, 1);
        assert_eq!(
            plus_one_counters(&game, entered),
            1,
            "source removed={remove_source}"
        );
    }
}

#[test]
fn chapter_ii_ignores_other_casts_and_rewards_only_the_first_own_creature() {
    let definition = compile_saga_chapter(CHAPTER_II);
    let (mut game, _) = resolve_entry_counter_chapter(&definition);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    for (caster, card_type, expected_triggers, expected_counters) in [
        (bob, CardType::Creature, 0, 0),
        (alice, CardType::Artifact, 0, 0),
        (alice, CardType::Creature, 1, 1),
        (alice, CardType::Creature, 0, 0),
    ] {
        let entered =
            cast_and_resolve_entry_counter_fixture(&mut game, caster, card_type, expected_triggers);
        assert_eq!(plus_one_counters(&game, entered), expected_counters);
    }
}

#[test]
fn chapter_ii_unused_creature_cast_trigger_expires_after_this_turn() {
    let definition = compile_saga_chapter(CHAPTER_II);
    let (mut game, _) = resolve_entry_counter_chapter(&definition);
    let alice = game.players[0].id;
    game.turn.turn_number += 1;
    let entered = cast_and_resolve_entry_counter_fixture(&mut game, alice, CardType::Creature, 0);
    assert_eq!(plus_one_counters(&game, entered), 0);
}

#[test]
fn chapter_ii_applies_to_a_creature_cast_while_another_spell_resolves() {
    let definition = compile_saga_chapter(CHAPTER_II);
    let (mut game, _) = resolve_entry_counter_chapter(&definition);
    let alice = game.players[0].id;
    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Free Cast Creature")
        .card_types(vec![CardType::Creature])
        .mana_cost(crate::mana::ManaCost::from_symbols(vec![
            crate::mana::ManaSymbol::Generic(5),
        ]))
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let hand_card = game.create_object_from_card(&creature, alice, Zone::Hand);
    let stable_id = game.object(hand_card).unwrap().stable_id;
    let free_cast =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Free Cast Effect")
            .card_types(vec![CardType::Sorcery])
            .parse_text(
                "You may cast a creature spell from your hand without paying its mana cost.",
            )
            .unwrap();
    let source = game.create_object_from_definition(&free_cast, alice, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(source, alice));
    crate::game_loop::resolve_stack_entry_with(
        &mut game,
        &mut crate::decision::SelectFirstDecisionMaker,
    )
    .unwrap();
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(
        game.stack.len(),
        2,
        "casting during resolution must queue the chapter's delayed trigger above the creature"
    );
    assert!(!game.stack[0].is_ability);
    assert!(game.stack[1].is_ability);
    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
    let entered = game.find_object_by_stable_id(stable_id).unwrap();
    assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
    assert_eq!(plus_one_counters(&game, entered), 1);
}
