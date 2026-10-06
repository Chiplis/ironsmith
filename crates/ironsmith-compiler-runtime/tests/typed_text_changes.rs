//! Layer-3 owner contracts, not complete-body claims for text-changing cards.
//! Authored without running compiler, artifact, engine, or test execution.
use ironsmith::ability::{Ability, AbilityKind, ProtectionFrom};
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{ContinuousEffect, Modification};
use ironsmith::effect::{Effect, Until};
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, ExecutionError, ResolvedTarget, execute_effect};
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::{Color, ColorSet, GameState, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::{CompiledCardArtifact, WireContinuousModification, WireContinuousTarget, WireEffect, WireRuntimeModification};
use ironsmith_core::TextChange;

const A: PlayerId = PlayerId::from_index(0);

fn targets() -> [CardDefinition; 2] {
    let text = "Mana cost: {B}\nType: Creature — Human Wizard\nPower/Toughness: 2/3\nProtection from black\nIslandwalk";
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_runtime_definition("Black Human", text, false));
    let direct = direct.unwrap();
    assert!(!direct_loss.is_lossy());
    let (compiled, artifact_loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler_runtime::compile_to_artifact("Black Human", text, false));
    let (artifact, _) = compiled.unwrap();
    assert!(!artifact_loss.is_lossy());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}

fn application(spec: ChooseSpec, change: TextChange, artifact: bool) -> Effect {
    if !artifact {
        return Effect::new(ApplyContinuousEffect::with_spec(spec, Modification::RewriteText(change), Until::EndOfTurn));
    }
    type WireApply = ironsmith_core::ApplyContinuousEffect<WireContinuousTarget,
        WireContinuousModification, WireRuntimeModification, ironsmith_core::Condition>;
    let payload = WireApply::with_spec(spec, WireContinuousModification::RewriteText(change), Until::EndOfTurn);
    let wire = WireEffect::new("ApplyContinuousEffect", serde_json::to_value(payload).unwrap());
    let restored: WireEffect = serde_json::from_str(&serde_json::to_string(&wire).unwrap()).unwrap();
    ironsmith_runtime_catalog::artifact_materializer::materialize_effect(restored).unwrap()
}

#[test]
fn direct_and_artifact_targets_accept_the_same_native_and_wire_layer_operation() {
    for target in targets() { for artifact in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let id = game.create_object_from_definition(&target, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let before = game.calculated_characteristics(id).unwrap();
        let effect = application(ChooseSpec::SpecificObject(id),
            TextChange::color(Color::Black, Color::Blue).unwrap(), artifact);
        execute_effect(&mut game, &effect, &mut EffectContext::new_default(id, A)).unwrap();
        let after = game.calculated_characteristics(id).unwrap();
        let old = before.static_abilities.iter().find(|ability| ability.protection_from().is_some()).unwrap();
        let new = after.static_abilities.iter().find(|ability| ability.protection_from().is_some()).unwrap();
        assert_eq!(new.protection_from(), Some(&ProtectionFrom::Color(ColorSet::BLUE)));
        assert_eq!(new.instance_id(), old.instance_id());
        assert_eq!(after.name, before.name);
        assert_eq!(after.colors, before.colors);
        assert_eq!(after.mana_cost, before.mana_cost);
        assert_eq!(after.subtypes, before.subtypes);
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state().unwrap();
        let expired = game.calculated_characteristics(id).unwrap();
        assert_eq!(expired.static_abilities.iter().find_map(|ability| ability.protection_from()),
            Some(&ProtectionFrom::Color(ColorSet::BLACK)));
    }}
}

#[test]
fn timestamped_replacements_compose_and_do_not_change_current_color() {
    for target in targets() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let id = game.create_object_from_definition(&target, A, Zone::Battlefield);
        for (from, to) in [(Color::Black, Color::Blue), (Color::Blue, Color::Green)] {
            game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(id, A, vec![id],
                Modification::RewriteText(TextChange::color(from, to).unwrap())));
        }
        game.refresh_continuous_state().unwrap();
        let after = game.calculated_characteristics(id).unwrap();
        assert_eq!(after.colors, ColorSet::BLACK);
        assert_eq!(after.static_abilities.iter().find_map(|ability| ability.protection_from()),
            Some(&ProtectionFrom::Color(ColorSet::GREEN)));
    }
}

#[test]
fn target_recheck_uses_current_types_before_registering_a_text_change() {
    for target in targets() { for artifact in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let id = game.create_object_from_definition(&target, A, Zone::Battlefield);
        let effect = application(ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            TextChange::creature_type(Subtype::Human, Subtype::Elf).unwrap(), artifact);
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(id, A, vec![id],
            Modification::SetCardTypes(vec![ironsmith::CardType::Artifact])));
        game.refresh_continuous_state().unwrap();
        let count = game.effect_store.continuous_effects.effects().len();
        let outcome = execute_effect(&mut game, &effect, &mut EffectContext::new_default(id, A)
            .with_targets(vec![ResolvedTarget::Object(id)])).unwrap();
        assert_eq!(outcome.status, ironsmith::effect::OutcomeStatus::TargetInvalid);
        assert_eq!(game.effect_store.continuous_effects.effects().len(), count);
    }}
}

#[test]
fn unsupported_program_rolls_back_registered_text_effects_and_context_receipts() {
    for target in targets() { for artifact in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let id = game.create_object_from_definition(&target, A, Zone::Battlefield);
        std::sync::Arc::make_mut(&mut game.object_mut(id).unwrap().abilities)
            .push(Ability::activated(ironsmith::cost::TotalCost::free(), vec![Effect::draw(1)]));
        game.refresh_continuous_state().unwrap();
        let count = game.effect_store.continuous_effects.effects().len();
        let timestamp = game.effect_store.continuous_effects.current_timestamp();
        let effect = application(ChooseSpec::SpecificObject(id),
            TextChange::creature_type(Subtype::Human, Subtype::Vampire).unwrap(), artifact);
        let mut ctx = EffectContext::new_default(id, A);
        let result = execute_effect(&mut game, &effect, &mut ctx);
        assert!(matches!(result, Err(ExecutionError::ContinuousDiscovery(
            ironsmith::static_ability_processor::StaticEffectDiscoveryError::TextChangeDomain(_)))));
        assert_eq!(game.effect_store.continuous_effects.effects().len(), count);
        assert_eq!(game.effect_store.continuous_effects.current_timestamp(), timestamp);
        assert!(ctx.created_continuous_effects.is_empty());
        let chars = game.calculated_characteristics(id).unwrap();
        assert!(chars.subtypes.contains(&Subtype::Human));
        assert!(chars.abilities.iter().any(|ability| matches!(ability.kind, AbilityKind::Activated(_))));
    }}
}
