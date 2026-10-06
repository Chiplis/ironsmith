//! Native current-versus-captured program contracts. All authored, unrun.
use super::*;
use crate::card::{CardBuilder, PowerToughness};
use crate::decision::SelectFirstDecisionMaker;
use crate::effects::{EffectContext, execute_effect};
use crate::game_state::{GameState, StackEntry, Target, TargetAssignment};
use crate::ids::CardId;
use crate::snapshot::SpellProgramState;
use ironsmith_core::TextChange;

const A: PlayerId = PlayerId::from_index(0);

fn game() -> GameState { GameState::new(vec!["A".into(), "B".into()], 20) }
fn creature(game: &mut GameState, color: ColorSet) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Color witness")
        .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Human])
        .color_indicator(color).power_toughness(PowerToughness::fixed(2, 5)).build();
    game.create_object_from_card(&card, A, Zone::Battlefield)
}
fn red_target() -> ChooseSpec {
    let mut filter = ObjectFilter::creature(); filter.colors = Some(ColorSet::RED);
    ChooseSpec::target(ChooseSpec::Object(filter))
}
fn program() -> crate::resolution::ResolutionProgram {
    vec![crate::effect::Effect::deal_damage(3, red_target())].into()
}
fn spell(game: &mut GameState, body: crate::resolution::ResolutionProgram) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Red instructions")
        .card_types(vec![CardType::Instant]).build();
    let id = game.create_object_from_card(&card, A, Zone::Stack);
    game.object_mut(id).unwrap().spell_effect = Some(body.into());
    id
}
fn rewrite(game: &mut GameState, id: ObjectId, duration: Until) {
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(id, A, vec![id],
        Modification::RewriteText(TextChange::color(Color::Red, Color::Blue).unwrap())).until(duration));
}
fn colors(body: &crate::resolution::ResolutionProgram) -> ColorSet {
    let damage = body[0].downcast_ref::<crate::effects::DealDamageEffect>().unwrap();
    let ChooseSpec::Object(filter) = damage.target.base() else { panic!("object predicate"); };
    filter.colors.unwrap()
}
fn entry(id: ObjectId, target: ObjectId) -> StackEntry {
    StackEntry::new(id, A).with_targets(vec![Target::Object(target)])
        .with_target_assignments(vec![TargetAssignment { spec: red_target(), range: 0..1 }])
}

#[test]
fn a_spell_rereads_current_program_and_target_spec_without_mutating_copiable_text() {
    let mut game = game();
    let red = creature(&mut game, ColorSet::RED);
    let id = spell(&mut game, program());
    let original = game.object(id).unwrap().spell_effect_owned().unwrap();
    let saved_entry = entry(id, red);
    game.push_to_stack(saved_entry.clone());
    rewrite(&mut game, id, Until::EndOfTurn);
    game.refresh_continuous_state().unwrap();
    assert_eq!(colors(&game.current_spell_program(id).unwrap()), ColorSet::BLUE);
    assert_eq!(colors(&game.object(id).unwrap().spell_effect_owned().unwrap()), ColorSet::RED);
    assert_eq!(colors(&original), ColorSet::RED);
    let assignments = crate::game_loop::current_stack_entry_target_assignments(&game, &saved_entry).unwrap();
    let ChooseSpec::Object(current) = assignments[0].spec.base() else { panic!("current object"); };
    assert_eq!(current.colors, Some(ColorSet::BLUE));
    assert_eq!(saved_entry.target_assignments[0].spec, red_target());
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.damage_on(red), 0, "the original red target is now illegal");
}

#[test]
fn captured_ability_and_later_activation_read_different_immutable_text() {
    let mut game = game();
    let red = creature(&mut game, ColorSet::RED);
    let blue = creature(&mut game, ColorSet::BLUE);
    let host = creature(&mut game, ColorSet::COLORLESS);
    game.object_mut(host).unwrap().abilities = Arc::new(vec![Ability::activated(
        crate::cost::TotalCost::free(), program())]);
    let old = match &game.object(host).unwrap().abilities[0].kind {
        AbilityKind::Activated(ability) => ability.effects.clone(), _ => panic!("activated"),
    };
    game.push_to_stack(StackEntry::ability(host, A, old.clone())
        .with_targets(vec![Target::Object(red)])
        .with_target_assignments(vec![TargetAssignment { spec: red_target(), range: 0..1 }]));
    rewrite(&mut game, host, Until::Forever);
    game.refresh_continuous_state().unwrap();
    let current = game.calculated_characteristics(host).unwrap();
    let AbilityKind::Activated(ability) = &current.abilities[0].kind else { panic!("activated"); };
    assert_eq!(colors(&ability.effects), ColorSet::BLUE);
    assert_eq!(colors(&old), ColorSet::RED);
    let later = ability.effects.clone();
    let later_spec = later[0].0.get_target_spec().unwrap().clone();
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.damage_on(red), 3);
    game.push_to_stack(StackEntry::ability(host, A, later)
        .with_targets(vec![Target::Object(blue)])
        .with_target_assignments(vec![TargetAssignment { spec: later_spec, range: 0..1 }]));
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.damage_on(blue), 3);
}

#[test]
fn spell_copy_keeps_original_text_and_expiry_restores_original_program() {
    let mut game = game();
    let red = creature(&mut game, ColorSet::RED);
    let id = spell(&mut game, program());
    game.push_to_stack(entry(id, red));
    rewrite(&mut game, id, Until::EndOfTurn);
    game.refresh_continuous_state().unwrap();
    let copy = crate::effect::Effect::copy_spell(ChooseSpec::SpecificObject(id));
    execute_effect(&mut game, &copy, &mut EffectContext::new_default(id, A)).unwrap();
    let copied_id = game.stack.last().unwrap().object_id;
    assert_ne!(copied_id, id);
    assert_eq!(colors(&game.current_spell_program(copied_id).unwrap()), ColorSet::RED);
    assert_eq!(colors(&game.current_spell_program(id).unwrap()), ColorSet::BLUE);
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state().unwrap();
    assert_eq!(colors(&game.current_spell_program(id).unwrap()), ColorSet::RED);
}

#[test]
fn frozen_copy_program_is_not_reloaded_from_a_changed_donor() {
    let mut game = game();
    let donor = spell(&mut game, program());
    let values = CopiableValues::from_object(game.object(donor).unwrap());
    game.object_mut(donor).unwrap().spell_effect = Some(crate::resolution::ResolutionProgram::from_effects(vec![crate::effect::Effect::draw(9)]).into());
    let recipient = spell(&mut game, Default::default());
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(donor, A,
        vec![recipient], Modification::CopyOf { target_id: donor, copiable_values: Box::new(values),
            preserve_source_abilities: false, name_override: None, name_override_surface: None,
            add_supertypes: vec![] }));
    rewrite(&mut game, recipient, Until::Forever);
    game.refresh_continuous_state().unwrap();
    assert_eq!(colors(&game.current_spell_program(recipient).unwrap()), ColorSet::BLUE);
    let values = copiable_values_with_effects(recipient, game.objects_map(),
        game.effect_store.continuous_effects.effects(), &game.battlefield, game.commander_objects(), &game).unwrap();
    let SpellProgramState::Present(program) = values.spell_effect else { panic!("frozen program"); };
    assert_eq!(colors(&program), ColorSet::RED);
}

#[test]
fn missing_historical_program_evidence_never_becomes_known_absence() {
    let mut game = game();
    let donor = spell(&mut game, program());
    let mut values = CopiableValues::from_object(game.object(donor).unwrap());
    values.spell_effect = SpellProgramState::Unavailable;
    let recipient = spell(&mut game, Default::default());
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(donor, A,
        vec![recipient], Modification::CopyOf { target_id: donor, copiable_values: Box::new(values),
            preserve_source_abilities: false, name_override: None, name_override_surface: None,
            add_supertypes: vec![] }));
    game.refresh_continuous_state().unwrap();
    assert!(matches!(game.current_spell_program(recipient), Err(
        crate::static_ability_processor::StaticEffectDiscoveryError::TextChangeDomain(
            text_changes::TextChangeDomainError::SpellProgram))));
}

#[test]
fn type_line_mana_changes_while_authored_mana_symbols_and_occurrences_are_preserved() {
    let mut game = game();
    let card = CardBuilder::new(CardId::new(), "Typed land witness")
        .card_types(vec![CardType::Land]).subtypes(vec![Subtype::Forest]).build();
    let land = game.create_object_from_card(&card, A, Zone::Battlefield);
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(land, A,
        vec![land], Modification::RewriteText(TextChange::basic_land_type(Subtype::Forest, Subtype::Island).unwrap())));
    game.refresh_continuous_state().unwrap();
    let chars = game.calculated_characteristics(land).unwrap();
    assert!(chars.subtypes.contains(&Subtype::Island));
    assert!(!chars.subtypes.contains(&Subtype::Forest));
    let mana: Vec<_> = chars.abilities.iter().filter_map(|ability| {
        if let AbilityKind::Activated(ability) = &ability.kind { ability.mana_output.clone() } else { None }
    }).flatten().collect();
    assert_eq!(mana, vec![crate::mana::ManaSymbol::Blue]);
    let mut authored = GameState::new(vec!["A".into(), "B".into()], 20);
    let land = authored.create_object_from_card(&card, A, Zone::Battlefield);
    authored.object_mut(land).unwrap().abilities = Arc::new(vec![Ability::mana(
        crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()), vec![crate::mana::ManaSymbol::Green])]);
    authored.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(land, A,
        vec![land], Modification::RewriteText(TextChange::basic_land_type(Subtype::Forest, Subtype::Island).unwrap())));
    authored.refresh_continuous_state().unwrap();
    let current = authored.calculated_characteristics(land).unwrap();
    let mana: Vec<_> = current.abilities.iter().filter_map(|ability| {
        if let AbilityKind::Activated(ability) = &ability.kind { ability.mana_output.clone() } else { None }
    }).flatten().collect();
    assert_eq!(mana, vec![crate::mana::ManaSymbol::Green, crate::mana::ManaSymbol::Blue]);
    assert_eq!(current.abilities.origin(0), Some(&AbilityOrigin::Printed(0)));
    assert_eq!(current.abilities.origin(1), Some(&AbilityOrigin::IntrinsicBasicLandMana(Subtype::Island)));
}

fn legacy_copy_fixture() -> (GameState, ObjectId, ContinuousEffectId, CopiableValues) {
    let mut game = game();
    let red = creature(&mut game, ColorSet::RED);
    let donor = spell(&mut game, vec![crate::effect::Effect::destroy(red_target())].into());
    let known = CopiableValues::from_object(game.object(donor).unwrap());
    let mut missing = known.clone(); missing.spell_effect = SpellProgramState::Unavailable;
    let copied = spell(&mut game, Default::default());
    game.push_to_stack(entry(copied, red));
    let registered = game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(
        donor, A, vec![copied], Modification::CopyOf { target_id: donor, copiable_values: Box::new(missing),
            preserve_source_abilities: false, name_override: None, name_override_surface: None,
            add_supertypes: vec![] }));
    (game, copied, registered, known)
}

fn restore_known_program(game: &mut GameState, copied: ObjectId, old_effect: ContinuousEffectId, known: CopiableValues) {
    game.effect_store.continuous_effects.remove_effect(old_effect);
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(copied, A,
        vec![copied], Modification::CopyOf { target_id: copied, copiable_values: Box::new(known),
            preserve_source_abilities: false, name_override: None, name_override_surface: None,
            add_supertypes: vec![] }));
    game.refresh_continuous_state().unwrap();
}

#[test]
fn retargeting_missing_copy_program_is_incomplete_and_can_retry_with_exact_evidence() {
    let (mut game, copied, registered, known) = legacy_copy_fixture();
    let saved = game.stack.last().unwrap().targets.clone();
    let effect = crate::effect::Effect::new(crate::effects::RetargetStackObjectEffect::new(
        ChooseSpec::SpecificObject(copied)));
    let result = execute_effect(&mut game, &effect, &mut EffectContext::new_default(copied, A));
    assert!(matches!(result, Err(crate::effects::ExecutionError::ContinuousDiscovery(
        crate::static_ability_processor::StaticEffectDiscoveryError::TextChangeDomain(
            text_changes::TextChangeDomainError::SpellProgram)))));
    assert_eq!(game.stack.last().unwrap().targets, saved);
    restore_known_program(&mut game, copied, registered, known);
    assert!(execute_effect(&mut game, &effect, &mut EffectContext::new_default(copied, A)).is_ok());
}

#[test]
fn a_negated_program_filter_cannot_turn_missing_copy_evidence_into_success() {
    #[derive(Debug, Clone)]
    struct QueryDestroy { spell: ObjectId, negate: bool }
    impl crate::effects::EffectExecutor for QueryDestroy {
        fn execute(&self, game: &mut GameState, ctx: &mut crate::effects::ExecutionContext)
            -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError>
        {
            use crate::filter::ObjectFilterExt as _;
            let mut filter = ObjectFilter::default();
            filter.zone = Some(Zone::Stack);
            filter.would_destroy_object = Some(Box::new(ObjectFilter::creature()));
            let matched = filter.matches(game.object(self.spell).unwrap(), &ctx.filter_context(game), game);
            if matched != self.negate { game.player_mut(A).unwrap().life += 5; }
            Ok(crate::effect::EffectOutcome::resolved())
        }
    }
    for negate in [false, true] {
        let (mut game, copied, registered, known) = legacy_copy_fixture();
        let effect = crate::effect::Effect::new(QueryDestroy { spell: copied, negate });
        let result = execute_effect(&mut game, &effect, &mut EffectContext::new_default(copied, A));
        assert!(matches!(result, Err(crate::effects::ExecutionError::ContinuousDiscovery(
            crate::static_ability_processor::StaticEffectDiscoveryError::TextChangeDomain(
                text_changes::TextChangeDomainError::SpellProgram)))));
        assert_eq!(game.player(A).unwrap().life, 20, "neither false nor its negation commits partial work");
        restore_known_program(&mut game, copied, registered, known);
        execute_effect(&mut game, &effect, &mut EffectContext::new_default(copied, A)).unwrap();
        assert_eq!(game.player(A).unwrap().life, if negate { 20 } else { 25 });
    }
}

#[test]
fn copying_a_layer_one_spell_uses_frozen_program_before_text_and_requires_evidence() {
    let (mut game, copied, registered, known) = legacy_copy_fixture();
    let effect = crate::effect::Effect::copy_spell(ChooseSpec::SpecificObject(copied));
    let before = game.stack.len();
    assert!(matches!(execute_effect(&mut game, &effect, &mut EffectContext::new_default(copied, A)),
        Err(crate::effects::ExecutionError::ContinuousDiscovery(
            crate::static_ability_processor::StaticEffectDiscoveryError::TextChangeDomain(
                text_changes::TextChangeDomainError::SpellProgram)))));
    assert_eq!(game.stack.len(), before, "no copy is allocated from missing evidence");
    restore_known_program(&mut game, copied, registered, known.clone());
    rewrite(&mut game, copied, Until::Forever);
    game.refresh_continuous_state().unwrap();
    execute_effect(&mut game, &effect, &mut EffectContext::new_default(copied, A)).unwrap();
    let copy = game.stack.last().unwrap().object_id;
    assert_ne!(copy, copied);
    let body = game.current_spell_program(copy).unwrap();
    let destroy = body[0].downcast_ref::<crate::effects::DestroyEffect>().expect("frozen destroy body, not raw empty body");
    let ChooseSpec::Object(filter) = destroy.spec.base() else { panic!("object"); };
    assert_eq!(filter.colors, Some(ColorSet::RED), "layer-three blue replacement is not copied");
    let mut raw = game.object(copied).unwrap().clone();
    raw.copy_copiable_values_from_values(&known);
    assert!(raw.spell_effect_owned().unwrap().has_complete_definition());
    let mut missing = known; missing.spell_effect = SpellProgramState::Unavailable;
    raw.copy_copiable_values_from_values(&missing);
    assert!(!raw.spell_effect_owned().unwrap().has_complete_definition());
    assert!(CopiableValues::from_object(&raw).spell_effect.is_unavailable());
    assert!(crate::cards::generated_definition_has_unimplemented_content(&raw.to_card_definition()));
}

#[test]
fn multi_slot_retarget_rereads_original_specs_once_and_keeps_modes_and_ranges() {
    #[derive(Default)]
    struct NewTargets(Vec<Target>);
    impl crate::decision::DecisionMaker for NewTargets {
        fn decide_targets(&mut self, _: &GameState, _: &crate::decisions::context::TargetsContext) -> Vec<Target> {
            self.0.clone()
        }
    }
    let mut game = game();
    let red = creature(&mut game, ColorSet::RED);
    let red_flying = creature(&mut game, ColorSet::RED);
    let blue = creature(&mut game, ColorSet::BLUE);
    let blue_flying = creature(&mut game, ColorSet::BLUE);
    let green = creature(&mut game, ColorSet::GREEN);
    for id in [red_flying, blue_flying] {
        game.object_mut(id).unwrap().abilities = Arc::new(vec![Ability::static_ability(StaticAbility::flying())]);
    }
    let first = red_target();
    let mut second_filter = ObjectFilter::creature();
    second_filter.colors = Some(ColorSet::RED);
    second_filter.static_abilities.push(StaticAbilityId::Flying);
    let second = ChooseSpec::target(ChooseSpec::Object(second_filter));
    let modal = crate::effects::ChooseModeEffect::new(vec![
        ironsmith_core::EffectMode::new("Unchosen", vec![crate::effect::Effect::gain_life(1)]),
        ironsmith_core::EffectMode::new("Chosen", vec![
            crate::effect::Effect::deal_damage(1, first.clone()),
            crate::effect::Effect::deal_damage(1, second.clone()),
        ]),
    ], Value::Fixed(1), Value::Fixed(1), false);
    let id = spell(&mut game, vec![crate::effect::Effect::new(modal)].into());
    let mut saved = StackEntry::new(id, A).with_targets(vec![Target::Object(red), Target::Object(red_flying)])
        .with_target_assignments(vec![TargetAssignment { spec: first.clone(), range: 0..1 },
            TargetAssignment { spec: second.clone(), range: 1..2 }]);
    saved.chosen_modes = Some(vec![1]);
    game.push_to_stack(saved.clone());
    for (from, to) in [(Color::Blue, Color::Green), (Color::Red, Color::Blue)] {
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(id, A, vec![id],
            Modification::RewriteText(TextChange::color(from, to).unwrap())));
    }
    game.refresh_continuous_state().unwrap();
    for _ in 0..3 {
        let reread = crate::game_loop::current_stack_entry_target_assignments(&game, &saved).unwrap();
        assert_eq!(reread.iter().map(|assignment| assignment.range.clone()).collect::<Vec<_>>(), vec![0..1, 1..2]);
        for assignment in reread {
            let ChooseSpec::Object(filter) = assignment.spec.base() else { panic!("object"); };
            assert_eq!(filter.colors, Some(ColorSet::BLUE), "a second application would incorrectly produce green");
        }
        let view = crate::derived_view::DerivedGameView::new(&game);
        let legal = crate::game_loop::stack_entry_assignment_legal_targets(&game, &saved, 0, &view).unwrap();
        assert!(legal.legal_targets.contains(&Target::Object(blue)));
        assert!(!legal.legal_targets.contains(&Target::Object(green)));
    }
    let retarget = crate::effect::Effect::new(crate::effects::RetargetStackObjectEffect::new(ChooseSpec::SpecificObject(id)));
    let mut choices = NewTargets(vec![Target::Object(blue), Target::Object(blue_flying)]);
    execute_effect(&mut game, &retarget, &mut EffectContext::new(blue, A, &mut choices)).unwrap();
    let current = game.stack.last().unwrap();
    assert_eq!(current.targets, vec![Target::Object(blue), Target::Object(blue_flying)]);
    assert_eq!(current.chosen_modes, Some(vec![1]));
    assert_eq!(current.target_assignments[0].spec, first, "saved declarations remain original");
    assert_eq!(current.target_assignments[1].spec, second);
    for assignment in crate::game_loop::current_stack_entry_target_assignments(&game, current).unwrap() {
        let ChooseSpec::Object(filter) = assignment.spec.base() else { panic!("object"); };
        assert_eq!(filter.colors, Some(ColorSet::BLUE));
    }
}

#[test]
fn departing_copied_spell_keeps_layer_one_program_in_its_exact_lki() {
    let (mut game, copied, registered, known) = legacy_copy_fixture();
    restore_known_program(&mut game, copied, registered, known);
    let raw = game.object(copied).unwrap().clone();
    let saved_entry = game.stack.last().unwrap().clone();
    game.turn_store.cast_spell_lki.insert(copied, Arc::new((raw, saved_entry)));
    rewrite(&mut game, copied, Until::Forever);
    game.refresh_continuous_state().unwrap();
    game.move_object_by_effect(copied, Zone::Graveyard).unwrap();
    let (last_object, last_entry) = game.turn_store.cast_spell_lki.get(&copied).unwrap().as_ref();
    assert_eq!(last_object.id, copied);
    assert_eq!(last_entry.object_id, copied);
    let program = last_object.spell_effect_owned().unwrap();
    let destroy = program[0].downcast_ref::<crate::effects::DestroyEffect>().expect("frozen copied program");
    let ChooseSpec::Object(filter) = destroy.spec.base() else { panic!("object"); };
    assert_eq!(filter.colors, Some(ColorSet::RED), "uncopiable text changes are excluded from future copies");
}

#[test]
fn live_and_departed_bestow_spell_copies_keep_attachment_program_and_overlay_lifetime() {
    for departed in [false, true] {
        for target_leaves in [false, true] {
            let mut game = game();
            let target = creature(&mut game, ColorSet::RED);
            let card = CardBuilder::new(CardId::new(), "Bestow source")
                .card_types(vec![CardType::Enchantment, CardType::Creature])
                .subtypes(vec![Subtype::Human]).power_toughness(PowerToughness::fixed(2, 3)).build();
            let source = game.create_object_from_card(&card, A, Zone::Stack);
            game.object_mut(source).unwrap().apply_bestow_cast_overlay();
            let target_spec = game.object(source).unwrap().aura_attach_filter_owned().unwrap().target_spec();
            let saved = StackEntry::new(source, A).with_targets(vec![Target::Object(target)])
                .with_target_assignments(vec![TargetAssignment { spec: target_spec, range: 0..1 }]);
            let source_object = game.object(source).unwrap().clone();
            game.turn_store.cast_spell_lki.insert(source, Arc::new((source_object, saved.clone())));
            game.push_to_stack(saved);
            if departed { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            execute_effect(&mut game, &crate::effect::Effect::copy_spell(ChooseSpec::Source),
                &mut EffectContext::new_default(source, A)).unwrap();
            let copy = game.stack.last().unwrap().object_id;
            assert_ne!(copy, source);
            let object = game.object(copy).unwrap();
            assert!(object.is_bestow_overlay_active());
            assert!(object.subtypes.contains(&Subtype::Aura));
            assert!(!object.card_types.contains(&CardType::Creature));
            assert!(game.current_spell_program(copy).unwrap()[0]
                .downcast_ref::<crate::effects::AttachToEffect>().is_some());
            let mut restored = object.clone();
            assert!(restored.end_bestow_cast_overlay());
            assert!(restored.card_types.contains(&CardType::Creature));
            assert!(restored.materialized_text_box_abilities().iter().all(|ability|
                !matches!(&ability.kind, AbilityKind::Static(ability) if ability.enchant_filter().is_some())),
                "ending Bestow removes only its metadata-owned enchant occurrence");
            let stable = object.stable_id;
            if target_leaves { game.move_object_by_effect(target, Zone::Graveyard).unwrap(); }
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            let resolved = game.find_object_by_stable_id(stable).and_then(|id| game.object(id)).unwrap();
            assert_eq!(resolved.zone, Zone::Battlefield);
            if target_leaves {
                assert!(resolved.card_types.contains(&CardType::Creature));
                assert!(!resolved.subtypes.contains(&Subtype::Aura));
                assert_eq!(resolved.attached_to, None);
            } else {
                assert!(resolved.subtypes.contains(&Subtype::Aura));
                assert_eq!(resolved.attached_to, Some(crate::object::AttachmentTarget::Object(target)));
            }
        }
    }
}
