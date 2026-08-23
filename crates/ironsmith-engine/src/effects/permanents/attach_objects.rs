//! Attach arbitrary objects to a target object or player.

use super::{
    attach_battlefield_object_to_target, attachment_can_attach_to_target,
    choose_color_as_becomes_attached,
};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    resolve_objects_for_effect, resolve_single_object_from_spec, resolve_single_target_from_spec,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::AttachmentTarget;
use crate::target::ChooseSpec;
use crate::zone::Zone;
pub use ironsmith_core::AttachObjectsEffect;

fn resolve_attachment_target(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<AttachmentTarget, ExecutionError> {
    if let ChooseSpec::Tagged(tag) = spec.base()
        && let Some(tagged) = ctx.get_tagged_all(tag)
    {
        for snapshot in tagged {
            if let Some(current_id) = game.find_object_by_stable_id(snapshot.stable_id)
                && game
                    .object(current_id)
                    .is_some_and(|object| object.zone == Zone::Battlefield)
            {
                return Ok(AttachmentTarget::Object(current_id));
            }
        }
    }

    let resolved = match spec.base() {
        ChooseSpec::Player(_)
        | ChooseSpec::SpecificPlayer(_)
        | ChooseSpec::SourceController
        | ChooseSpec::SourceOwner
        | ChooseSpec::EachPlayer(_)
        | ChooseSpec::ObjectOrPlayer(_, _)
        | ChooseSpec::PlayerOrPlaneswalker(_)
        | ChooseSpec::AttackedPlayerOrPlaneswalker
        | ChooseSpec::AnyTarget
        | ChooseSpec::AnyOtherTarget
        | ChooseSpec::Iterated => resolve_single_target_from_spec(game, spec, ctx)?,
        // A counted object target can legitimately resolve to an empty set
        // when its minimum is zero. Keep that absence in the object domain so
        // it becomes `InvalidTarget` (a no-op for this instruction) instead
        // of retrying the same spec as a player and surfacing a type error.
        _ => crate::effects::ResolvedTarget::Object(resolve_single_object_from_spec(
            game, spec, ctx,
        )?),
    };

    match resolved {
        crate::effects::ResolvedTarget::Object(id) => Ok(AttachmentTarget::Object(id)),
        crate::effects::ResolvedTarget::Player(id) => Ok(AttachmentTarget::Player(id)),
    }
}

/// Effect that attaches one or more objects to a destination object.
impl EffectExecutor for AttachObjectsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.individual_targets {
            let object_ids = resolve_objects_for_effect(game, ctx, &self.objects)?;
            if object_ids.is_empty() {
                return Ok(EffectOutcome::count(0));
            }

            // Collect every choice before mutating attachments. An interactive
            // decision maker can therefore pause on any one Aura and safely
            // replay earlier choices without partially applying the effect.
            let mut assignments = Vec::new();
            for object_id in object_ids {
                let candidate_ids = crate::effects::helpers::preview_object_ids_for_choose_spec(
                    game,
                    &self.target,
                    ctx,
                )
                .unwrap_or_default();
                let candidates = candidate_ids
                    .into_iter()
                    .filter(|candidate_id| {
                        attachment_can_attach_to_target(
                            game,
                            object_id,
                            AttachmentTarget::Object(*candidate_id),
                        )
                    })
                    .filter_map(|candidate_id| {
                        game.object(candidate_id).map(|object| {
                            crate::decisions::context::SelectableObject::new(
                                candidate_id,
                                object.name.clone(),
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                if candidates.is_empty() {
                    continue;
                }

                let attachment_name = game
                    .object(object_id)
                    .map(|object| object.name.to_string())
                    .unwrap_or_else(|| "attachment".to_string());
                let choice = crate::decisions::context::SelectObjectsContext::new(
                    ctx.controller,
                    Some(ctx.source),
                    format!("Choose what to attach {attachment_name} to"),
                    candidates,
                    1,
                    Some(1),
                )
                .require_explicit_choice();
                let chosen = ctx
                    .decision_maker
                    .decide_objects(game, &choice)
                    .into_iter()
                    .next();
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                let Some(target_id) = chosen.filter(|chosen_id| {
                    choice
                        .candidates
                        .iter()
                        .any(|candidate| candidate.legal && candidate.id == *chosen_id)
                }) else {
                    continue;
                };
                assignments.push((object_id, AttachmentTarget::Object(target_id)));
            }

            let mut attached_count = 0i32;
            for (object_id, target) in assignments {
                if attach_battlefield_object_to_target(game, object_id, target) {
                    choose_color_as_becomes_attached(game, ctx, object_id, target);
                    attached_count += 1;
                }
            }
            return Ok(EffectOutcome::count(attached_count));
        }

        let target = match resolve_attachment_target(game, &self.target, ctx) {
            Ok(target) => target,
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(err) => return Err(err),
        };
        if !game.attachment_target_exists_on_battlefield(target) {
            return Ok(EffectOutcome::target_invalid());
        }

        let object_ids = resolve_objects_for_effect(game, ctx, &self.objects)?;
        if object_ids.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut attached_count = 0i32;
        let mut attempted_source = false;
        for object_id in object_ids {
            attempted_source |= object_id == ctx.source;
            if attach_battlefield_object_to_target(game, object_id, target) {
                choose_color_as_becomes_attached(game, ctx, object_id, target);
                attached_count += 1;
            }
        }
        if attached_count == 0
            && !attempted_source
            && attach_battlefield_object_to_target(game, ctx.source, target)
        {
            choose_color_as_becomes_attached(game, ctx, ctx.source, target);
            attached_count += 1;
        }

        Ok(EffectOutcome::count(attached_count))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.target.is_target().then_some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "object to attach to"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectObjectsContext;
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::ObjectFilter;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    fn create_land(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Forest])
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    fn create_equipment(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    fn create_marker_artifact(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Artifact])
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    fn create_aura(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build();
        let mut object = Object::from_card(id, &card, controller, Zone::Battlefield);
        object.aura_attach_filter =
            Some(crate::object::AuraAttachmentFilter::from(ObjectFilter::creature()).into());
        game.add_object(object);
        id
    }

    #[test]
    fn optional_object_destination_with_no_announced_target_is_a_no_op() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let equipment = create_equipment(&mut game, "Optional Equipment", alice);
        let original_host = create_creature(&mut game, "Original Host", alice);
        assert!(attach_battlefield_object_to_target(
            &mut game,
            equipment,
            AttachmentTarget::Object(original_host),
        ));

        let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()))
            .with_count(crate::effect::ChoiceCount::up_to(1));
        let effect = AttachObjectsEffect::new(ChooseSpec::SpecificObject(equipment), target);
        let mut ctx = ExecutionContext::new_default(equipment, alice);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("declining an optional attachment target should not error");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::TargetInvalid);
        assert_eq!(
            game.object(equipment).and_then(|object| object.attached_to),
            Some(AttachmentTarget::Object(original_host)),
            "a declined reattachment must leave the Equipment where it was"
        );
    }

    #[test]
    fn test_attach_objects_illegal_equipment_target_does_not_move() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let equipment = create_equipment(&mut game, "Test Equipment", alice);
        let creature = create_creature(&mut game, "Bear", alice);
        let land = create_land(&mut game, "Forest", alice);

        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            equipment,
            AttachmentTarget::Object(creature),
        );
        let original_timestamp = game
            .effect_store
            .continuous_effects
            .get_attachment_timestamp(equipment)
            .expect("equipment should gain a timestamp when first attached");

        let mut ctx = ExecutionContext::new_default(equipment, alice)
            .with_targets(vec![ResolvedTarget::Object(land)]);
        let effect = AttachObjectsEffect::new(
            ChooseSpec::SpecificObject(equipment),
            ChooseSpec::target_permanent(),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(result.count_or_zero(), 0);
        assert_eq!(
            game.object(equipment).and_then(|object| object.attached_to),
            Some(crate::object::AttachmentTarget::Object(creature)),
            "illegal reattach should leave the equipment on its original creature"
        );
        assert_eq!(
            game.effect_store
                .continuous_effects
                .get_attachment_timestamp(equipment),
            Some(original_timestamp),
            "illegal attach attempts should not create a new timestamp"
        );
    }

    #[test]
    fn test_attach_objects_non_attachment_object_does_not_move() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let marker = create_marker_artifact(&mut game, "Marker", alice);
        let creature = create_creature(&mut game, "Bear", alice);

        let mut ctx = ExecutionContext::new_default(marker, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);
        let effect = AttachObjectsEffect::new(
            ChooseSpec::SpecificObject(marker),
            ChooseSpec::target_creature(),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(result.count_or_zero(), 0);
        assert_eq!(
            game.object(marker).and_then(|object| object.attached_to),
            None,
            "objects that are not Auras, Equipment, or Fortifications should remain unattached"
        );
        assert!(
            !game
                .object(creature)
                .expect("creature should exist")
                .attachments
                .contains(&marker),
            "the target should not gain a fake attachment link"
        );
    }

    #[test]
    fn test_attach_objects_to_same_target_is_no_op() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let aura = create_aura(&mut game, "Pacifism", alice);
        let creature = create_creature(&mut game, "Bear", alice);

        assert!(
            crate::effects::permanents::attach_battlefield_object_to_target(
                &mut game,
                aura,
                AttachmentTarget::Object(creature),
            )
        );
        let original_timestamp = game
            .effect_store
            .continuous_effects
            .get_attachment_timestamp(aura)
            .expect("aura should gain a timestamp when first attached");

        let mut ctx = ExecutionContext::new_default(aura, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);
        let effect = AttachObjectsEffect::new(
            ChooseSpec::SpecificObject(aura),
            ChooseSpec::target_creature(),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(result.count_or_zero(), 0);
        assert_eq!(
            game.effect_store
                .continuous_effects
                .get_attachment_timestamp(aura),
            Some(original_timestamp),
            "reattaching to the same object should not create a new timestamp"
        );
        assert_eq!(
            game.object(aura).and_then(|object| object.attached_to),
            Some(crate::object::AttachmentTarget::Object(creature))
        );
    }

    #[test]
    fn test_attach_objects_aura_to_player_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let aura = create_aura(&mut game, "Curse", alice);
        game.object_mut(aura)
            .expect("aura should exist")
            .aura_attach_filter = Some(
            crate::object::AuraAttachmentFilter::from(crate::target::PlayerFilter::Any).into(),
        );

        let mut ctx = ExecutionContext::new_default(aura, alice)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        let effect = AttachObjectsEffect::new(
            ChooseSpec::SpecificObject(aura),
            ChooseSpec::target_player(),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(result.count_or_zero(), 1);
        assert_eq!(
            game.object(aura).and_then(|object| object.attached_to),
            Some(AttachmentTarget::Player(bob))
        );
        assert!(
            game.player(bob)
                .expect("bob should exist")
                .attachments
                .contains(&aura),
            "the enchanted player should record the Aura attachment"
        );
    }

    #[test]
    fn test_attach_objects_tagged_target_follows_returned_battlefield_object() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let equipment = create_equipment(&mut game, "Test Equipment", alice);
        let creature = create_creature(&mut game, "Bear", alice);
        let graveyard_id = game
            .move_object_by_effect(creature, Zone::Graveyard)
            .expect("creature should move to graveyard");
        let graveyard_snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(graveyard_id).unwrap(), &game);
        let returned_id = game
            .move_object_by_effect(graveyard_id, Zone::Battlefield)
            .expect("creature should return to battlefield");
        let mut ctx = ExecutionContext::new_default(equipment, alice);
        ctx.tag_object("returned", graveyard_snapshot);

        let effect = AttachObjectsEffect::new(
            ChooseSpec::SpecificObject(equipment),
            ChooseSpec::Tagged("returned".into()),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(result.count_or_zero(), 1);
        assert_eq!(
            game.object(equipment).and_then(|object| object.attached_to),
            Some(AttachmentTarget::Object(returned_id))
        );
        assert!(
            game.object(returned_id)
                .expect("returned creature should exist")
                .attachments
                .contains(&equipment)
        );
    }

    #[test]
    fn test_attach_objects_source_to_original_tag_after_return_sequence() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let equipment = create_equipment(&mut game, "Test Equipment", alice);
        let original = create_creature(&mut game, "Bear", alice);
        let graveyard_id = game
            .move_object_by_effect(original, Zone::Graveyard)
            .expect("creature should move to graveyard");
        let stable_id = game.object(graveyard_id).unwrap().stable_id;
        let graveyard_snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(graveyard_id).unwrap(), &game);
        let mut ctx = ExecutionContext::new_default(equipment, alice);
        ctx.tag_object("triggering", graveyard_snapshot);

        let move_effect = crate::effect::Effect::new(crate::effects::TaggedEffect::new(
            "returned_0",
            crate::effect::Effect::new(
                crate::effects::MoveToZoneEffect::new(
                    ChooseSpec::Tagged("triggering".into()),
                    Zone::Battlefield,
                    false,
                )
                .under_you_control(),
            ),
        ));
        crate::effects::execute_effect(&mut game, &move_effect, &mut ctx)
            .expect("return effect should resolve");
        let returned_id = game
            .find_object_by_stable_id(stable_id)
            .expect("returned creature should keep stable id");

        let attach_effect =
            AttachObjectsEffect::new(ChooseSpec::Source, ChooseSpec::Tagged("triggering".into()));
        let result = attach_effect
            .execute(&mut game, &mut ctx)
            .expect("attach should resolve");

        assert_eq!(result.count_or_zero(), 1);
        assert_eq!(
            game.object(equipment).and_then(|object| object.attached_to),
            Some(AttachmentTarget::Object(returned_id))
        );
    }

    #[test]
    fn individual_targets_choose_a_legal_destination_for_each_attachment() {
        struct OrderedChoices {
            choices: Vec<ObjectId>,
            next: usize,
        }

        impl DecisionMaker for OrderedChoices {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &SelectObjectsContext,
            ) -> Vec<ObjectId> {
                let chosen = self.choices[self.next];
                self.next += 1;
                assert!(
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.legal && candidate.id == chosen),
                    "scripted attachment destination must be legal: {ctx:?}"
                );
                vec![chosen]
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first_aura = create_aura(&mut game, "First Aura", alice);
        let second_aura = create_aura(&mut game, "Second Aura", alice);
        let first_creature = create_creature(&mut game, "First Bear", alice);
        let second_creature = create_creature(&mut game, "Second Bear", alice);
        let mut aura_filter = ObjectFilter::default().in_zone(Zone::Battlefield);
        aura_filter.subtypes.push(Subtype::Aura);
        let destination = ObjectFilter::creature()
            .in_zone(Zone::Battlefield)
            .controlled_by(crate::target::PlayerFilter::You);
        let effect = AttachObjectsEffect::new(
            ChooseSpec::All(aura_filter),
            ChooseSpec::Object(destination),
        )
        .with_individual_targets();
        let mut decisions = OrderedChoices {
            choices: vec![first_creature, second_creature],
            next: 0,
        };
        let mut ctx =
            ExecutionContext::new_default(first_aura, alice).with_decision_maker(&mut decisions);

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("individual attachment choices should resolve");

        assert_eq!(outcome.count_or_zero(), 2);
        assert_eq!(
            game.object(first_aura)
                .and_then(|object| object.attached_to),
            Some(AttachmentTarget::Object(first_creature))
        );
        assert_eq!(
            game.object(second_aura)
                .and_then(|object| object.attached_to),
            Some(AttachmentTarget::Object(second_creature))
        );
    }

    #[test]
    fn formerly_attached_equipment_resolves_from_departed_source_lki() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Counter Inheritance Source", alice);
        let target = create_creature(&mut game, "Counter Inheritance Target", alice);
        let former_equipment = create_equipment(&mut game, "Former Equipment", alice);
        let unrelated_equipment = create_equipment(&mut game, "Unrelated Equipment", alice);

        assert!(
            crate::effects::permanents::attach_battlefield_object_to_target(
                &mut game,
                former_equipment,
                AttachmentTarget::Object(source),
            )
        );
        let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(source).expect("source should exist"),
            &game,
        );
        game.move_object_by_effect(source, Zone::Graveyard)
            .expect("sacrificed source should leave the battlefield");
        assert!(
            game.object(source).is_none(),
            "the attachment's former battlefield host must no longer be a live object"
        );

        let mut formerly_attached_equipment = ObjectFilter::default()
            .in_zone(Zone::Battlefield)
            .with_subtype(Subtype::Equipment);
        formerly_attached_equipment.attached_to_object = Some(Box::new(ObjectFilter::source()));
        let effect = AttachObjectsEffect::new(
            ChooseSpec::All(formerly_attached_equipment),
            ChooseSpec::SpecificObject(target),
        );
        let mut ctx =
            ExecutionContext::new_default(source, alice).with_source_snapshot(source_snapshot);

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("former attachment should resolve from source LKI");

        assert_eq!(outcome.count_or_zero(), 1);
        assert_eq!(
            game.object(former_equipment)
                .and_then(|object| object.attached_to),
            Some(AttachmentTarget::Object(target))
        );
        assert_eq!(
            game.object(unrelated_equipment)
                .and_then(|object| object.attached_to),
            None,
            "Equipment absent from the source snapshot must not be attached"
        );
    }
}
