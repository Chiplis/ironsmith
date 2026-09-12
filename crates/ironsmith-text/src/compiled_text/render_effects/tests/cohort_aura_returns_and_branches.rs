use super::*;

struct AcceptChoices;

impl crate::decision::DecisionMaker for AcceptChoices {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }

    fn decide_objects(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .take(ctx.max.unwrap_or(ctx.candidates.len()))
            .map(|candidate| candidate.id)
            .collect()
    }
}

#[test]
fn cohort_created_aura_retains_enchant_and_umbra_armor() {
    let spell = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Mantle Maker")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Create a white Aura enchantment token named Mantle attached to another target permanent. The token has enchant permanent and umbra armor.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
    let permanent = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let target = game.create_object_from_definition(&permanent, alice, Zone::Battlefield);
    let mut ctx = crate::effects::EffectContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    for effect in spell
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects()
    {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    let aura = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .find(|object| object.name == "Mantle")
        .unwrap();
    assert_eq!(
        aura.attached_to,
        Some(crate::object::AttachmentTarget::Object(target))
    );
    assert!(aura.abilities.iter().any(|ability| matches!(&ability.kind,
        crate::ability::AbilityKind::Static(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::UmbraArmor)));
    assert!(
        crate::compiled_text::compiled_text_lines(&spell)
            .join("\n")
            .contains("enchant permanent and umbra armor")
    );
    assert_eq!(
        crate::events::processing::process_destroy_full(&mut game, target, None),
        crate::events::processing::DestroyResult::Replaced,
    );
    assert!(game.battlefield.contains(&target));
    assert!(
        !game
            .battlefield
            .iter()
            .any(|id| game.object(*id).unwrap().name == "Mantle")
    );
}

#[test]
fn cohort_return_same_subtype_preserves_order_and_replaces_exclusion() {
    let spell = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Restoration")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Return all non-Aura enchantment cards from your graveyard to the battlefield, then do the same for Aura cards.").unwrap();
    let effects = spell
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects();
    assert_eq!(effects.len(), 2, "{effects:#?}");
    let first = unwrap_basic_tag_wrappers(&effects[0])
        .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
        .unwrap();
    let second = unwrap_basic_tag_wrappers(&effects[1])
        .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
        .unwrap();
    assert!(
        first
            .filter
            .excluded_subtypes
            .contains(&crate::types::Subtype::Aura)
    );
    assert_eq!(second.filter.subtypes, [crate::types::Subtype::Aura]);
    assert!(
        !second
            .filter
            .excluded_subtypes
            .contains(&crate::types::Subtype::Aura)
    );
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
    let permanent = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sanctuary")
        .card_types(vec![CardType::Enchantment])
        .build();
    let aura = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Blessing")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![crate::types::Subtype::Aura])
        .parse_text("Enchant enchantment")
        .unwrap();
    game.create_object_from_definition(&permanent, alice, Zone::Graveyard);
    game.create_object_from_definition(&aura, alice, Zone::Graveyard);
    let mut decisions = AcceptChoices;
    let mut ctx = crate::effects::EffectContext::new(source, alice, &mut decisions);
    crate::effects::execute_effect(&mut game, &effects[0], &mut ctx).unwrap();
    assert_eq!(game.battlefield.len(), 1);
    crate::effects::execute_effect(&mut game, &effects[1], &mut ctx).unwrap();
    assert_eq!(game.battlefield.len(), 2);
    let returned_aura = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .find(|object| object.name == "Blessing")
        .unwrap();
    assert!(returned_aura.attached_to.is_some());
}

#[test]
fn cohort_scaled_entry_counters_use_actual_sacrificed_count() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Hungry Visitor")
        .card_types(vec![CardType::Creature])
        .parse_text("As this creature enters, you may sacrifice any number of creatures and/or planeswalkers. If you do, it enters with twice that many +1/+1 counters on it.").unwrap();
    let crate::ability::AbilityKind::Static(ability) = &card.abilities[0].kind else {
        panic!("static")
    };
    let ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram { program, .. } =
        &ability.compiled_model().unwrap().payload
    else {
        panic!("entry program")
    };
    for count in [0, 1, 3] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&card, alice, Zone::Stack);
        let fodder = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Fodder")
            .card_types(vec![CardType::Creature])
            .build();
        for _ in 0..count {
            game.create_object_from_definition(&fodder, alice, Zone::Battlefield);
        }
        let mut decisions = AcceptChoices;
        let mut ctx = crate::effects::EffectContext::new(source, alice, &mut decisions);
        ctx.replacement.entry_counter_source = Some(source);
        for effect in program.flattened_default_effects() {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().graveyard.len(), count);
        assert_eq!(
            game.object(source)
                .unwrap()
                .counters
                .get(&crate::CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or_default(),
            2 * count as u32
        );
    }
    assert!(
        crate::compiled_text::compiled_text_lines(&card)
            .join("\n")
            .contains("twice that many")
    );
}

#[test]
fn cohort_otherwise_after_positive_result_tests_choice_not_branch_outcome() {
    let spell = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Decision Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("You may gain 1 life. If you do, draw zero cards. Otherwise, gain 5 life.")
        .unwrap();
    for accept in [true, false] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
        let mut yes = AcceptChoices;
        let mut no = crate::decision::AutoPassDecisionMaker;
        let decisions: &mut dyn crate::decision::DecisionMaker =
            if accept { &mut yes } else { &mut no };
        let mut ctx = crate::effects::EffectContext::new(source, alice, decisions);
        for effect in spell
            .spell_effect
            .as_ref()
            .unwrap()
            .flattened_default_effects()
        {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(
            game.player(alice).unwrap().life,
            if accept { 21 } else { 25 }
        );
    }
}

#[test]
fn cohort_searched_permanent_gets_haste_and_is_exiled_by_delayed_trigger() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Scaled Summoner")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .parse_text("{1}{R}{R}, {T}: Search your library for a Dragon permanent card, put that card onto the battlefield, then shuffle. That Dragon gains haste until end of turn. Exile it at the beginning of the next end step.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let dragon = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Dragon")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Dragon])
        .power_toughness(crate::card::PowerToughness::fixed(5, 5))
        .parse_text("Shroud").unwrap();
    game.create_object_from_definition(&dragon, alice, Zone::Library);
    let crate::ability::AbilityKind::Activated(ability) = &card.abilities[0].kind else {
        panic!("activated")
    };
    let mut decisions = AcceptChoices;
    let mut ctx = crate::effects::EffectContext::new(source, alice, &mut decisions);
    for effect in ability.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    let dragon = *game
        .battlefield
        .iter()
        .find(|id| game.object(**id).unwrap().name == "Dragon")
        .unwrap();
    let stable = game.object(dragon).unwrap().stable_id;
    assert!(
        game.current_has_static_ability_id(dragon, crate::static_abilities::StaticAbilityId::Haste)
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(bob),
        crate::provenance::ProvNodeId::default(),
    );
    let triggers = crate::triggers::check_delayed_triggers(&mut game, &event);
    assert_eq!(triggers.len(), 1);
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in triggers {
        queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert!(game.stack.last().unwrap().targets.is_empty());
    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert!(
        game.exile
            .iter()
            .any(|id| game.object(*id).unwrap().stable_id == stable)
    );
    assert!(crate::triggers::check_delayed_triggers(&mut game, &event).is_empty());
}
