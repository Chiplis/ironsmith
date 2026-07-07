//! Transform effect implementation.

use crate::card::LinkedFaceLayout;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_single_object_for_effect, resolve_tagged_object_id};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::other::{ConvertedEvent, TransformedEvent};
use crate::game_state::GameState;
use crate::static_abilities::StaticAbilityId;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
use crate::types::CardType;
use crate::zone::Zone;
pub use ironsmith_core::TransformEffect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransformLikeAction {
    Transform,
    Convert,
}

impl TransformLikeAction {
    fn event(
        self,
        target_id: crate::ids::ObjectId,
        provenance: crate::provenance::ProvNodeId,
    ) -> TriggerEvent {
        match self {
            Self::Transform => {
                TriggerEvent::new_with_provenance(TransformedEvent::new(target_id), provenance)
            }
            Self::Convert => {
                TriggerEvent::new_with_provenance(ConvertedEvent::new(target_id), provenance)
            }
        }
    }

    fn target_description(self) -> &'static str {
        match self {
            Self::Transform => "permanent to transform",
            Self::Convert => "permanent to convert",
        }
    }
}

/// Effect that transforms a double-faced permanent.
///
/// Swaps a DFC (double-faced card) to its other visible face.
/// This is separate from hidden face-down state used by morph, manifest, and
/// similar mechanics.
///
/// # Fields
///
/// * `target` - The permanent to transform
///
/// # Example
///
/// ```ignore
/// // Transform target permanent
/// let effect = TransformEffect::new(ChooseSpec::permanent());
///
/// // Transform this permanent (the source)
/// let effect = TransformEffect::source();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ConvertEffect {
    /// The targeting specification.
    pub target: ChooseSpec,
}

impl ConvertEffect {
    /// Create a new convert effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    /// Create an effect that converts the source permanent.
    pub fn source() -> Self {
        Self::new(ChooseSpec::Source)
    }

    /// Create an effect that converts target permanent.
    pub fn target_permanent() -> Self {
        Self::new(ChooseSpec::permanent())
    }
}

fn source_transform_like_action_is_stale(
    game: &GameState,
    ctx: &ExecutionContext,
    target_id: crate::ids::ObjectId,
) -> bool {
    target_id == ctx.source
        && ctx
            .source_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.transform_count != game.transform_count(target_id))
}

fn execute_transform_like_action(
    action: TransformLikeAction,
    target: &ChooseSpec,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    let target_id = if let ChooseSpec::Tagged(tag) = target {
        ctx.get_tagged_all(tag)
            .and_then(|snapshots| {
                snapshots
                    .iter()
                    .find_map(|snapshot| resolve_tagged_object_id(game, snapshot))
            })
            .ok_or(ExecutionError::InvalidTarget)?
    } else {
        resolve_single_object_for_effect(game, ctx, target)?
    };

    if source_transform_like_action_is_stale(game, ctx, target_id) {
        return Ok(EffectOutcome::resolved());
    }

    if !game.can_transform(target_id) {
        return Ok(EffectOutcome::resolved());
    }

    game.refresh_continuous_state();
    if !game.can_transform(target_id) {
        return Ok(EffectOutcome::resolved());
    }

    let Some(target) = game.object(target_id) else {
        return Ok(EffectOutcome::resolved());
    };
    if target.zone != Zone::Battlefield
        || target.linked_face_layout != LinkedFaceLayout::TransformLike
    {
        return Ok(EffectOutcome::resolved());
    }
    if matches!(action, TransformLikeAction::Transform)
        && (target.has_static_ability_id(StaticAbilityId::Daybound)
            || target.has_static_ability_id(StaticAbilityId::Nightbound))
    {
        return Ok(EffectOutcome::resolved());
    }

    let Some(other_def) = game
        .linked_face_definition_by_name_or_id(target.other_face_name.as_deref(), target.other_face)
    else {
        return Ok(EffectOutcome::resolved());
    };
    if other_def.card.card_types.contains(&CardType::Instant)
        || other_def.card.card_types.contains(&CardType::Sorcery)
    {
        return Ok(EffectOutcome::resolved());
    }

    if let Some(obj) = game.object_mut(target_id) {
        obj.apply_definition_face(&other_def);
    }
    game.mark_transformed(target_id);

    Ok(EffectOutcome::resolved().with_event(action.event(target_id, ctx.provenance)))
}

impl EffectExecutor for TransformEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        execute_transform_like_action(TransformLikeAction::Transform, &self.target, game, ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        TransformLikeAction::Transform.target_description()
    }
}

impl EffectExecutor for ConvertEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        execute_transform_like_action(TransformLikeAction::Convert, &self.target, game, ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        TransformLikeAction::Convert.target_description()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{LinkedFaceLayout, PowerToughness};
    use crate::cards::{CardDefinition, CardDefinitionBuilder};
    use crate::effects::ExecutionContext;
    use crate::events::EventKind;
    use crate::events::combat::CreatureAttackedEvent;
    use crate::events::phase::EndOfCombatEvent;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::snapshot::ObjectSnapshot;
    use crate::triggers::{
        AttackEventTarget, TransformsTrigger, TriggerContext, TriggerEvent, TriggerMatcher,
        TriggerQueue, check_triggers,
    };
    use crate::types::{CardType, Subtype};
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn runtime_custom_registry_test_guard() -> MutexGuard<'static, ()> {
        static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        GUARD
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("transform registry tests should acquire the runtime custom-card test mutex")
    }

    fn register_transform_pair(
        front_id: CardId,
        front_name: &str,
        back_id: CardId,
        back_name: &str,
        back_types: Vec<CardType>,
        back_text: &str,
    ) -> CardDefinition {
        let mut front = CardDefinitionBuilder::new(front_id, front_name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human, Subtype::Scout])
            .power_toughness(PowerToughness::fixed(2, 2))
            .parse_text("Vigilance")
            .expect("front face should parse");
        front.card.other_face = Some(back_id);
        front.card.other_face_name = Some(back_name.to_string());
        front.card.linked_face_layout = LinkedFaceLayout::TransformLike;

        let mut back_builder = CardDefinitionBuilder::new(back_id, back_name)
            .card_types(back_types)
            .oracle_text(back_text);
        if back_text == "Trample" {
            back_builder = back_builder
                .subtypes(vec![Subtype::Werewolf])
                .power_toughness(PowerToughness::fixed(4, 4));
        }
        let mut back = back_builder
            .parse_text(back_text)
            .expect("back face should parse");
        back.card.other_face = Some(front_id);
        back.card.other_face_name = Some(front_name.to_string());
        back.card.linked_face_layout = LinkedFaceLayout::TransformLike;

        crate::cards::register_runtime_custom_card(front.clone());
        crate::cards::register_runtime_custom_card(back);
        front
    }

    fn register_conquerors_galleon_pair(front_id: CardId, back_id: CardId) -> CardDefinition {
        let mut front = CardDefinitionBuilder::new(front_id, "Conqueror's Galleon")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Vehicle])
            .power_toughness(PowerToughness::fixed(2, 10))
            .parse_text(
                "When this Vehicle attacks, exile it at end of combat, then return it to the battlefield transformed under your control.\nCrew 4 (Tap any number of creatures you control with total power 4 or more: This Vehicle becomes an artifact creature until end of turn.)",
            )
            .expect("front face should parse");
        front.card.other_face = Some(back_id);
        front.card.other_face_name = Some("Conqueror's Foothold".to_string());
        front.card.linked_face_layout = LinkedFaceLayout::TransformLike;

        let mut back = CardDefinitionBuilder::new(back_id, "Conqueror's Foothold")
            .card_types(vec![CardType::Land])
            .build();
        back.card.other_face = Some(front_id);
        back.card.other_face_name = Some("Conqueror's Galleon".to_string());
        back.card.linked_face_layout = LinkedFaceLayout::TransformLike;

        crate::cards::register_runtime_custom_card(front.clone());
        crate::cards::register_runtime_custom_card(back);
        front
    }

    fn register_harvest_hand_pair(front_id: CardId, back_id: CardId) -> CardDefinition {
        let mut front = CardDefinitionBuilder::new(front_id, "Harvest Hand")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Scarecrow])
            .power_toughness(PowerToughness::fixed(2, 2))
            .parse_text(
                "When this creature dies, return it to the battlefield transformed under your control.",
            )
            .expect("front face should parse");
        front.card.other_face = Some(back_id);
        front.card.other_face_name = Some("Scrounged Scythe".to_string());
        front.card.linked_face_layout = LinkedFaceLayout::TransformLike;

        let mut back = CardDefinitionBuilder::new(back_id, "Scrounged Scythe")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .parse_text("Equipped creature gets +1/+1.\nEquip {2}")
            .expect("back face should parse");
        back.card.other_face = Some(front_id);
        back.card.other_face_name = Some("Harvest Hand".to_string());
        back.card.linked_face_layout = LinkedFaceLayout::TransformLike;

        crate::cards::register_runtime_custom_card(front.clone());
        crate::cards::register_runtime_custom_card(back);
        front
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn transform_swaps_faces_and_refreshes_timestamp() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_100),
            "Trail Scout",
            CardId::from_raw(79_101),
            "Moonlit Howler",
            vec![CardType::Creature],
            "Trample",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        let before_ts = game
            .effect_store
            .continuous_effects
            .get_entry_timestamp(source)
            .expect("battlefield permanent should have an entry timestamp");

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = TransformEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("transform should execute");

        assert_eq!(outcome.events.len(), 1);
        assert!(!game.is_face_down(source));
        assert_eq!(game.transform_count(source), 1);
        let after_ts = game
            .effect_store
            .continuous_effects
            .get_entry_timestamp(source)
            .expect("transformed permanent should keep an entry timestamp");
        assert!(
            after_ts > before_ts,
            "transformed permanents should get a fresh timestamp"
        );

        let object = game.object(source).expect("source permanent should exist");
        assert_eq!(object.name, "Moonlit Howler");
        assert_eq!(object.card_types, vec![CardType::Creature]);
        assert_eq!(object.subtypes, vec![Subtype::Werewolf]);
        assert_eq!(object.base_power.map(|value| value.base_value()), Some(4));
        assert_eq!(
            object.base_toughness.map(|value| value.base_value()),
            Some(4)
        );
        assert_eq!(object.compiled_card_text.as_ref(), "Trample");

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = TransformEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("transform back should execute");

        assert_eq!(outcome.events.len(), 1);
        assert!(!game.is_face_down(source));
        assert_eq!(game.transform_count(source), 2);
        let object = game
            .object(source)
            .expect("source permanent should still exist");
        assert_eq!(object.name, "Trail Scout");
        assert_eq!(object.subtypes, vec![Subtype::Human, Subtype::Scout]);
        assert_eq!(object.compiled_card_text.as_ref(), "Vigilance");
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn conquerors_galleon_returns_transformed_at_end_of_combat() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front =
            register_conquerors_galleon_pair(CardId::from_raw(79_200), CardId::from_raw(79_201));
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);

        assert!(
            game.object(source)
                .unwrap()
                .abilities
                .iter()
                .any(|ability| matches!(&ability.kind, crate::ability::AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("attacks"))),
            "Conqueror's Galleon should have an attack trigger"
        );

        let attack_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(source, AttackEventTarget::Player(PlayerId::from_index(1))),
            crate::provenance::ProvNodeId::default(),
        );
        let mut trigger_queue = TriggerQueue::new();
        for trigger in check_triggers(&game, &attack_event) {
            trigger_queue.add(trigger);
        }
        crate::put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("should queue attack trigger");
        while !game.stack_is_empty() {
            crate::resolve_stack_entry(&mut game).expect("resolve attack trigger");
        }

        assert!(
            game.battlefield.iter().any(|&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Conqueror's Galleon")
            }),
            "Galleon should remain on the battlefield until end of combat"
        );
        assert_eq!(
            game.effect_store.delayed_triggers.len(),
            1,
            "attack trigger should schedule one delayed end-of-combat trigger"
        );
        assert!(
            game.exile.is_empty(),
            "Galleon should not exile immediately"
        );

        let end_of_combat_event = TriggerEvent::new_with_provenance(
            EndOfCombatEvent::new(),
            crate::provenance::ProvNodeId::default(),
        );
        let mut trigger_queue = TriggerQueue::new();
        for trigger in crate::triggers::check_delayed_triggers(&mut game, &end_of_combat_event) {
            trigger_queue.add(trigger);
        }
        crate::put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("should queue delayed end-of-combat trigger");
        while !game.stack_is_empty() {
            crate::resolve_stack_entry(&mut game).expect("resolve delayed end-of-combat trigger");
        }

        let foothold_id = game
            .battlefield
            .iter()
            .copied()
            .find(|&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Conqueror's Foothold")
            })
            .expect("Conqueror's Foothold should return to the battlefield");
        assert!(
            !game.battlefield.iter().any(|&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Conqueror's Galleon")
            }),
            "front face should leave the battlefield once the delayed trigger resolves"
        );
        assert!(
            !game.is_face_down(foothold_id),
            "returned permanent should transform into the visible Foothold face"
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn harvest_hand_returns_transformed_when_it_dies() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_harvest_hand_pair(CardId::from_raw(79_202), CardId::from_raw(79_203));
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);

        game.mark_damage(source, 2);

        let mut trigger_queue = TriggerQueue::new();
        crate::check_and_apply_sbas(&mut game, &mut trigger_queue)
            .expect("Harvest Hand should die and queue its trigger");
        assert!(
            !trigger_queue.is_empty(),
            "Harvest Hand's dies trigger should be queued after lethal damage"
        );

        crate::put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("should put Harvest Hand's dies trigger on the stack");
        while !game.stack_is_empty() {
            crate::resolve_stack_entry(&mut game).expect("resolve Harvest Hand dies trigger");
        }

        assert!(
            game.player(alice)
                .expect("Alice should exist")
                .graveyard
                .is_empty(),
            "Harvest Hand should not stay in the graveyard after its trigger resolves"
        );
        let scythe_id = game
            .battlefield
            .iter()
            .copied()
            .find(|&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Scrounged Scythe")
            })
            .expect("Scrounged Scythe should return to the battlefield");
        assert!(
            !game.is_face_down(scythe_id),
            "returned Harvest Hand should be on its visible back face"
        );
        assert!(
            !game.battlefield.iter().any(|&id| game
                .object(id)
                .is_some_and(|obj| obj.name == "Harvest Hand")),
            "front face should no longer be on the battlefield after transforming"
        );
    }

    #[test]
    fn transform_requires_a_transform_like_permanent() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let normal = CardDefinitionBuilder::new(CardId::from_raw(79_102), "Ordinary Bear")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let source = game.create_object_from_definition(&normal, alice, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = TransformEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("non-dfc transform should resolve as a no-op");

        assert!(outcome.events.is_empty());
        assert!(!game.is_face_down(source));
        assert_eq!(game.transform_count(source), 0);
        assert_eq!(
            game.object(source).expect("source should still exist").name,
            "Ordinary Bear"
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn transform_does_nothing_if_other_face_is_an_instant_or_sorcery() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_103),
            "Test Alchemist",
            CardId::from_raw(79_104),
            "Forbidden Formula",
            vec![CardType::Sorcery],
            "Draw a card.",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = TransformEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("illegal transform should resolve as a no-op");

        assert!(outcome.events.is_empty());
        assert!(!game.is_face_down(source));
        assert_eq!(game.transform_count(source), 0);
        assert_eq!(
            game.object(source).expect("source should still exist").name,
            "Test Alchemist"
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn transform_source_ability_fizzles_if_source_already_transformed_since_it_was_stacked() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_105),
            "Twilit Ranger",
            CardId::from_raw(79_106),
            "Midnight Stalker",
            vec![CardType::Creature],
            "Trample",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        let snapshot =
            ObjectSnapshot::from_object(game.object(source).expect("source should exist"), &game);

        let mut first_ctx = ExecutionContext::new_default(source, alice);
        TransformEffect::source()
            .execute(&mut game, &mut first_ctx)
            .expect("first transform should succeed");
        assert!(!game.is_face_down(source));

        let mut stale_ctx =
            ExecutionContext::new_default(source, alice).with_source_snapshot(snapshot);
        let outcome = TransformEffect::source()
            .execute(&mut game, &mut stale_ctx)
            .expect("stale self-transform should resolve as a no-op");

        assert!(outcome.events.is_empty());
        assert!(!game.is_face_down(source));
        assert_eq!(game.transform_count(source), 1);
        assert_eq!(
            game.object(source).expect("source should still exist").name,
            "Midnight Stalker"
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn convert_swaps_faces_emits_converted_event_and_not_transform_event() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_107),
            "Autobot Engineer",
            CardId::from_raw(79_108),
            "Autobot Racer",
            vec![CardType::Artifact],
            "Haste",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        let before_ts = game
            .effect_store
            .continuous_effects
            .get_entry_timestamp(source)
            .expect("battlefield permanent should have an entry timestamp");

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = ConvertEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("convert should execute");

        assert_eq!(outcome.events.len(), 1);
        assert_eq!(outcome.events[0].kind(), EventKind::Converted);
        assert!(outcome.events[0].downcast::<ConvertedEvent>().is_some());
        assert!(outcome.events[0].downcast::<TransformedEvent>().is_none());
        assert!(!game.is_face_down(source));
        assert_eq!(game.transform_count(source), 1);
        assert_eq!(
            game.object(source)
                .expect("source permanent should exist")
                .name,
            "Autobot Racer"
        );
        let after_ts = game
            .effect_store
            .continuous_effects
            .get_entry_timestamp(source)
            .expect("converted permanent should keep an entry timestamp");
        assert!(after_ts > before_ts);

        let trigger = TransformsTrigger::new();
        let trigger_ctx = TriggerContext::for_source(source, alice, &game);
        assert!(
            !trigger.matches(&outcome.events[0], &trigger_ctx),
            "convert should not satisfy transform-only triggers"
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn convert_respects_cant_transform_restrictions() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_109),
            "Ground Patrol",
            CardId::from_raw(79_110),
            "Sky Convoy",
            vec![CardType::Artifact],
            "Flying",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        game.effect_store.cant_effects.cant_transform.insert(source);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = ConvertEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("restricted convert should resolve as a no-op");

        assert!(outcome.events.is_empty());
        assert!(!game.is_face_down(source));
        assert_eq!(game.transform_count(source), 0);
        assert_eq!(
            game.object(source).expect("source should still exist").name,
            "Ground Patrol"
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn transform_uses_game_local_linked_face_cache_after_runtime_registry_is_cleared() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_113),
            "Cache Runner",
            CardId::from_raw(79_114),
            "Cache Cruiser",
            vec![CardType::Artifact],
            "Flying",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);

        crate::cards::clear_runtime_custom_cards();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = TransformEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("transform should still resolve from the game-local linked-face cache");

        assert_eq!(outcome.events.len(), 1);
        assert!(!game.is_face_down(source));
        assert_eq!(
            game.object(source).expect("source should still exist").name,
            "Cache Cruiser"
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn convert_source_ability_fizzles_if_source_already_transformed_since_it_was_stacked() {
        let _guard = runtime_custom_registry_test_guard();
        crate::cards::clear_runtime_custom_cards();

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front = register_transform_pair(
            CardId::from_raw(79_111),
            "Signal Runner",
            CardId::from_raw(79_112),
            "Signal Cruiser",
            vec![CardType::Artifact],
            "Flying",
        );
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        let snapshot =
            ObjectSnapshot::from_object(game.object(source).expect("source should exist"), &game);

        let mut first_ctx = ExecutionContext::new_default(source, alice);
        TransformEffect::source()
            .execute(&mut game, &mut first_ctx)
            .expect("first transform should succeed");
        assert!(!game.is_face_down(source));

        let mut stale_ctx =
            ExecutionContext::new_default(source, alice).with_source_snapshot(snapshot);
        let outcome = ConvertEffect::source()
            .execute(&mut game, &mut stale_ctx)
            .expect("stale self-convert should resolve as a no-op");

        assert!(outcome.events.is_empty());
        assert!(!game.is_face_down(source));
        assert_eq!(game.transform_count(source), 1);
        assert_eq!(
            game.object(source).expect("source should still exist").name,
            "Signal Cruiser"
        );
    }
}
