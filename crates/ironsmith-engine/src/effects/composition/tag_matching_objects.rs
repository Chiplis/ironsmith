//! Tag all objects matching a filter across one or more zones.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::zone::Zone;
pub type TagMatchingObjectsEffect = ironsmith_core::TagMatchingObjectsEffect;

fn effect_zones(effect: &TagMatchingObjectsEffect) -> Vec<Zone> {
    let mut zones = Vec::new();
    if let Some(zone) = effect.zone.or(effect.filter.zone) {
        zones.push(zone);
    }
    for zone in &effect.additional_zones {
        if !zones.contains(zone) {
            zones.push(*zone);
        }
    }
    if zones.is_empty() {
        zones.push(Zone::Battlefield);
    }
    zones
}

impl EffectExecutor for TagMatchingObjectsEffect {
    fn cost_choice_bindings(&self) -> crate::effects::CostChoiceBindings {
        let required = if self.source_tags.is_empty() {
            crate::effects::CostChoiceBindings::from_spec(&ChooseSpec::All(self.filter.clone())).required
        } else {
            self.source_tags.clone()
        };
        crate::effects::CostChoiceBindings { required, published: vec![self.tag.clone()] }
    }
    fn is_object_selection_prelude(&self) -> bool { true }
    fn visit_prepared_selection_bindings(&self, visitor: &mut dyn FnMut(crate::effects::PreparedSelectionBinding)) {
        visitor(crate::effects::PreparedSelectionBinding::ObjectTag(self.tag.clone()));
    }
    fn as_cost_executable(&self) -> Option<&dyn crate::effects::CostExecutableEffect> { Some(self) }
    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        if !self.source_tags.is_empty() {
            return Vec::new();
        }
        effect_zones(self)
            .into_iter()
            .map(|zone| {
                let mut filter = self.filter.clone();
                filter.zone = Some(zone);
                ChooseSpec::All(filter)
            })
            .collect()
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut excluded_stable_ids = Vec::new();
        for tag in &self.excluded_stable_card_tags {
            let captured = ctx.get_tagged_all(tag).ok_or_else(|| {
                ExecutionError::IncompleteEvidence(format!(
                    "collection complement has no selected-card tag {tag}"
                ))
            })?;
            excluded_stable_ids.extend(captured.iter().map(|snapshot| snapshot.stable_id));
        }
        if !self.source_tags.is_empty() {
            let exact_sources=self.source_tags.iter().all(|tag|ctx.exact_object_reference_tags.contains(tag));
            let mut snapshots = Vec::new();
            let filter_ctx = ctx.filter_context(game);
            for source_tag in &self.source_tags {
                let Some(source) = ctx.get_tagged_all(source_tag) else {
                    if self.filter_source_snapshots {
                        return Err(ExecutionError::IncompleteEvidence(
                            "filtered tagged capture has no producer collection".into(),
                        ));
                    }
                    continue;
                };
                for snapshot in source {
                    if excluded_stable_ids.contains(&snapshot.stable_id) {
                        continue;
                    }
                    if self.filter_source_snapshots
                        && !self.filter.matches_snapshot(snapshot, &filter_ctx, game)
                    {
                        continue;
                    }
                    if !snapshots
                        .iter()
                        .any(|existing: &ObjectSnapshot| if self.filter_source_snapshots || exact_sources {existing.object_id==snapshot.object_id}else{existing.stable_id == snapshot.stable_id})
                    {
                        snapshots.push(snapshot.clone());
                    }
                }
            }
            let object_ids = snapshots
                .iter()
                .map(|snapshot| snapshot.object_id)
                .collect::<Vec<_>>();
            ctx.set_tagged_objects(self.tag.clone(), snapshots);
            if exact_sources {ctx.exact_object_reference_tags.insert(self.tag.clone());}
            return Ok(EffectOutcome::with_objects(object_ids));
        }

        // "a spell that was cast this turn" names every spell cast this
        // turn, including those that have already resolved or left the
        // stack; the turn's cast history records them.
        if self.filter.cast_this_turn && effect_zones(self) == [Zone::Stack] {
            let filter_ctx = ctx.filter_context(game);
            let snapshots = game
                .turn_store
                .turn_history
                .spell_cast_snapshot_history()
                .iter()
                .filter(|snapshot| {
                    !excluded_stable_ids.contains(&snapshot.stable_id)
                        && self.filter.matches_snapshot(snapshot, &filter_ctx, game)
                })
                .cloned()
                .collect::<Vec<_>>();
            let object_ids = snapshots
                .iter()
                .map(|snapshot| snapshot.object_id)
                .collect::<Vec<_>>();
            ctx.set_tagged_objects(self.tag.clone(), snapshots);
            return Ok(EffectOutcome::with_objects(object_ids));
        }

        let mut object_ids = Vec::new();
        for zone in effect_zones(self) {
            let mut filter = self.filter.clone();
            filter.zone = Some(zone);
            for object_id in resolve_objects_from_spec(game, &ChooseSpec::All(filter), ctx)? {
                if !object_ids.contains(&object_id) {
                    object_ids.push(object_id);
                }
            }
        }

        object_ids.retain(|id| game.object(*id).is_some_and(|object| {
            !excluded_stable_ids.contains(&object.stable_id)
        }));
        let snapshots = object_ids
            .iter()
            .filter_map(|id| {
                game.object(*id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))
            })
            .collect::<Vec<_>>();
        ctx.set_tagged_objects(self.tag.clone(), snapshots);
        Ok(EffectOutcome::with_objects(object_ids))
    }
}

impl crate::effects::CostExecutableEffect for TagMatchingObjectsEffect {
    fn preview_payment_bindings(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        // A pure capture constrained by an already selected packet is also a
        // deterministic binding. Publish its exact filtered subset before a
        // later cost asks for it; no action or player choice is performed.
        if self.source_tags.is_empty() {
            let required = self.cost_choice_bindings().required;
            if required.is_empty() {
                return Ok(());
            }
            for tag in required {
                if ctx.get_tagged_all(&tag).is_none() {
                    return Err(ExecutionError::IncompleteEvidence(format!(
                        "filtered capture has no selected-card tag {tag}")));
                }
            }
            let mut ids = Vec::new();
            for zone in effect_zones(self) {
                let mut filter = self.filter.clone();
                filter.zone = Some(zone);
                for id in resolve_objects_from_spec(game, &ChooseSpec::All(filter), ctx)? {
                    if !ids.contains(&id) { ids.push(id); }
                }
            }
            let mut excluded = Vec::new();
            for tag in &self.excluded_stable_card_tags {
                let snapshots = ctx.get_tagged_all(tag).ok_or_else(||
                    ExecutionError::IncompleteEvidence(format!("collection complement has no selected-card tag {tag}")))?;
                excluded.extend(snapshots.iter().map(|snapshot| snapshot.stable_id));
            }
            let snapshots = ids.into_iter().filter_map(|id| game.object(id))
                .filter(|object| !excluded.contains(&object.stable_id))
                .map(|object| ObjectSnapshot::from_object(object, game)).collect();
            ctx.set_tagged_objects(self.tag.clone(), snapshots);
            return Ok(());
        }
        let exact_sources=self.source_tags.iter().all(|tag|ctx.exact_object_reference_tags.contains(tag));
        let mut excluded = Vec::new();
        for tag in &self.excluded_stable_card_tags {
            let snapshots = ctx.get_tagged_all(tag).ok_or_else(||
                ExecutionError::IncompleteEvidence(format!("collection complement has no selected-card tag {tag}")))?;
            excluded.extend(snapshots.iter().map(|snapshot| snapshot.stable_id));
        }
        let mut snapshots: Vec<ObjectSnapshot> = Vec::new();
        let filter_ctx = ctx.filter_context(game);
        for tag in &self.source_tags {
            let Some(source) = ctx.get_tagged_all(tag) else {
                if self.filter_source_snapshots {
                    return Err(ExecutionError::IncompleteEvidence(
                        "filtered tagged capture has no producer collection".into()));
                }
                continue;
            };
            for snapshot in source {
                if excluded.contains(&snapshot.stable_id)
                    || (self.filter_source_snapshots
                        && !self.filter.matches_snapshot(snapshot, &filter_ctx, game))
                    || snapshots.iter().any(|item| if self.filter_source_snapshots || exact_sources {item.object_id==snapshot.object_id}else{item.stable_id == snapshot.stable_id})
                {
                    continue;
                }
                snapshots.push(snapshot.clone());
            }
        }
        ctx.set_tagged_objects(self.tag.clone(), snapshots);
        if exact_sources {ctx.exact_object_reference_tags.insert(self.tag.clone());}
        Ok(())
    }

    fn can_execute_as_cost(&self, _: &GameState, _: crate::ids::ObjectId, _: crate::ids::PlayerId) -> Result<(), crate::effects::CostValidationError> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::test_prelude::*;
    use crate::types::CardType;

    fn make_card(name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(99), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Creature])
            .build()
    }

    #[test]
    fn cost_preview_publishes_a_zone_subset_and_known_empty_subset_without_actions() {
        use crate::effects::CostExecutableEffect;
        use crate::filter::ObjectFilter;
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_card(&make_card("Source"), alice, Zone::Battlefield);
        let chosen = game.create_object_from_card(&make_card("Chosen"), alice, Zone::Battlefield);
        let outsider = game.create_object_from_card(&make_card("Unchosen"), alice, Zone::Battlefield);
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_objects("selected", vec![ObjectSnapshot::from_object_id(&game, chosen).unwrap()]);
        let capture = TagMatchingObjectsEffect::new(
            ObjectFilter::tagged("selected").in_zone(Zone::Battlefield), "subset");
        assert_eq!(capture.cost_choice_bindings().required, vec![crate::tag::TagKey::from("selected")]);
        capture.preview_payment_bindings(&game, &mut ctx).unwrap();
        let subset = ctx.get_tagged_all("subset").unwrap();
        assert_eq!(subset.iter().map(|object| object.object_id).collect::<Vec<_>>(), vec![chosen]);
        assert!(game.object(chosen).is_some());
        assert!(game.object(outsider).is_some());
        TagMatchingObjectsEffect::new(ObjectFilter::tagged("selected").in_zone(Zone::Graveyard), "empty")
            .preview_payment_bindings(&game, &mut ctx).unwrap();
        assert!(ctx.get_tagged_all("empty").unwrap().is_empty());
        let missing = TagMatchingObjectsEffect::new(ObjectFilter::tagged("absent"), "unproven");
        assert!(matches!(missing.preview_payment_bindings(&game, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
        assert!(ctx.get_tagged_all("unproven").is_none());
    }

    #[test]
    fn searched_library_cards_moved_to_exile_yield_exact_castable_remainder() {
        use crate::filter::{ObjectFilter, TaggedOpbjectRelation};
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_card(&make_card("Source"), alice, Zone::Battlefield);
        let picked = game.create_object_from_card(&make_card("Picked"), alice, Zone::Library);
        let remaining = game.create_object_from_card(&make_card("Remaining"), alice, Zone::Library);
        let outsider = game.create_object_from_card(&make_card("Outsider"), alice, Zone::Exile);
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_objects("searched", vec![
            ObjectSnapshot::from_object_id(&game, picked).unwrap(),
            ObjectSnapshot::from_object_id(&game, remaining).unwrap(),
        ]);
        // Search tags are captured in Library; the move creates new exile
        // object IDs. The producer intentionally follows the stable cards to
        // Exile, then captures their *current* exact object IDs in `rest`.
        let picked_exile = game.move_object_by_effect(picked, Zone::Exile).unwrap();
        let remaining_exile = game.move_object_by_effect(remaining, Zone::Exile).unwrap();
        ctx.set_tagged_objects("picked", vec![
            ObjectSnapshot::from_object_id(&game, picked_exile).unwrap(),
        ]);
        game.move_object_by_effect(picked_exile, Zone::Library).unwrap();
        let producer = TagMatchingObjectsEffect::new(
            ObjectFilter::tagged("searched").in_zone(Zone::Exile), "rest",
        ).in_zone(Zone::Exile).excluding_stable_cards_tagged("picked");
        producer.execute(&mut game, &mut ctx).unwrap();
        let rest = ctx.get_tagged_all("rest").unwrap();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].object_id, remaining_exile);
        let choice = ObjectFilter::tagged("rest")
            .in_zone(Zone::Exile)
            .match_tagged("rest", TaggedOpbjectRelation::SameObjectId);
        {
            let filter_ctx = ctx.filter_context(&game);
            assert!(choice.matches(game.object(remaining_exile).unwrap(), &filter_ctx, &game));
            assert!(!choice.matches(game.object(outsider).unwrap(), &filter_ctx, &game));
        }
        let hand = game.move_object_by_effect(remaining_exile, Zone::Hand).unwrap();
        let returned = game.move_object_by_effect(hand, Zone::Exile).unwrap();
        assert_ne!(returned, remaining_exile);
        let filter_ctx = ctx.filter_context(&game);
        assert!(!choice.matches(game.object(returned).unwrap(), &filter_ctx, &game));
    }

    #[test]
    fn redirected_opponent_pick_still_is_not_in_the_other_exiled_cards() {
        use crate::filter::ObjectFilter;
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_card(&make_card("Source"), alice, Zone::Battlefield);
        let picked = game.create_object_from_card(&make_card("Picked"), alice, Zone::Library);
        let other = game.create_object_from_card(&make_card("Other"), alice, Zone::Library);
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_objects("searched", vec![
            ObjectSnapshot::from_object_id(&game, picked).unwrap(),
            ObjectSnapshot::from_object_id(&game, other).unwrap(),
        ]);
        let picked_exile = game.move_object_by_effect(picked, Zone::Exile).unwrap();
        let other_exile = game.move_object_by_effect(other, Zone::Exile).unwrap();
        ctx.set_tagged_objects("picked", vec![
            ObjectSnapshot::from_object_id(&game, picked_exile).unwrap(),
        ]);
        let producer = TagMatchingObjectsEffect::new(
            ObjectFilter::tagged("searched").in_zone(Zone::Exile), "rest",
        ).in_zone(Zone::Exile).excluding_stable_cards_tagged("picked");
        // The opponent's pick is excluded even when its attempted shuffle
        // simply does not move it out of Exile.
        producer.execute(&mut game, &mut ctx).unwrap();
        let rest = ctx.get_tagged_all("rest").unwrap();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].object_id, other_exile);
        // Model a redirected move ending in Exile under a new object ID.
        // The chosen card remains a physical card in the zone, but is still
        // not one of "the other cards."
        let redirected_intermediate = game.move_object_by_effect(picked_exile, Zone::Hand).unwrap();
        let redirected_exile = game.move_object_by_effect(redirected_intermediate, Zone::Exile).unwrap();
        assert_ne!(redirected_exile, picked_exile);
        producer.execute(&mut game, &mut ctx).unwrap();
        let rest = ctx.get_tagged_all("rest").unwrap();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].object_id, other_exile);
        assert_ne!(rest[0].object_id, redirected_exile);
    }

    #[test]
    fn filtered_captured_union_keeps_snapshot_identity_without_rescanning() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let own = game.create_object_from_card(&make_card("Own captured card"), alice, Zone::Hand);
        let foreign = game.create_object_from_card(&make_card("Foreign captured card"), bob, Zone::Hand);
        let source = game.create_object_from_card(&make_card("Uncaptured card"), alice, Zone::Battlefield);
        let own_snapshot = ObjectSnapshot::from_object_id(&game, own).unwrap();
        let captured = vec![own_snapshot.clone(), ObjectSnapshot::from_object_id(&game, foreign).unwrap()];
        let moved = game.move_object_by_effect(own, Zone::Exile).unwrap();
        assert_ne!(moved, own);
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_objects("prior", captured);
        let mut filter = crate::target::ObjectFilter::default();
        filter.owner = Some(crate::target::PlayerFilter::You);
        let filtered = TagMatchingObjectsEffect::new(filter.clone(), "own")
            .from_filtered_tagged_sources(["prior".into()]);
        filtered.execute(&mut game, &mut ctx).unwrap();
        let subset = ctx.get_tagged_all("own").unwrap();
        assert_eq!(subset.len(), 1);
        assert_eq!(subset[0].object_id, own_snapshot.object_id);
        assert_eq!(subset[0].stable_id, own_snapshot.stable_id);
        let legacy = TagMatchingObjectsEffect::new(filter.clone(), "legacy")
            .from_tagged_sources(["prior".into()]);
        legacy.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(ctx.get_tagged_all("legacy").unwrap().len(), 2);
        let missing = TagMatchingObjectsEffect::new(filter.clone(), "missing")
            .from_filtered_tagged_sources(["absent_producer".into()]);
        assert!(matches!(missing.execute(&mut game, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
        ctx.set_tagged_objects("empty", Vec::new());
        TagMatchingObjectsEffect::new(filter, "empty_subset")
            .from_filtered_tagged_sources(["empty".into()])
            .execute(&mut game, &mut ctx).unwrap();
        assert!(ctx.get_tagged_all("empty_subset").unwrap().is_empty());
    }

    #[test]
    fn tags_matching_objects_across_multiple_hidden_zones() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let hand_card_id = game.new_object_id();
        let graveyard_card_id = game.new_object_id();
        let hand_card = make_card("Hand Creature");
        let graveyard_card = make_card("Graveyard Creature");
        game.add_object(Object::from_card(
            hand_card_id,
            &hand_card,
            alice,
            Zone::Hand,
        ));
        game.add_object(Object::from_card(
            graveyard_card_id,
            &graveyard_card,
            alice,
            Zone::Graveyard,
        ));
        game.player_mut(alice)
            .expect("player")
            .hand
            .push(hand_card_id);
        game.player_mut(alice)
            .expect("player")
            .graveyard
            .push(graveyard_card_id);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = TagMatchingObjectsEffect::new(ObjectFilter::creature(), "matched")
            .in_zones(vec![Zone::Hand, Zone::Graveyard]);

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");
        let crate::effect::OutcomeValue::Objects(ids) = outcome.value else {
            panic!("expected tagged object ids");
        };
        assert_eq!(ids.len(), 2);

        let tagged = ctx
            .get_tagged_all("matched")
            .expect("tagged snapshots should be stored");
        assert_eq!(tagged.len(), 2);
    }

    #[test]
    fn tags_matching_objects_on_battlefield_when_no_zone_is_specified() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let creature_id =
            game.create_object_from_card(&make_card("Battlefield Bear"), alice, Zone::Battlefield);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = TagMatchingObjectsEffect::new(ObjectFilter::creature(), "matched");

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");
        let crate::effect::OutcomeValue::Objects(ids) = outcome.value else {
            panic!("expected tagged object ids");
        };
        assert_eq!(ids, vec![creature_id]);

        let tagged = ctx
            .get_tagged_all("matched")
            .expect("tagged snapshots should be stored");
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].object_id, creature_id);
    }
    #[test]
    fn exact_parent_capture_alias_preserves_incarnations_and_local_writes_shadow_its_policy(){
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);let actor=crate::ids::PlayerId(0);
        let source=game.create_object_from_card(&make_card("Captured card"),actor,Zone::Exile);
        let first=ObjectSnapshot::from_object_id(&game,source).unwrap();let mut second=first.clone();second.object_id=crate::ids::ObjectId::from_raw(991_000_001);
        let mut ctx=ExecutionContext::new_default(source,actor);ctx.set_tagged_objects("parent",vec![first.clone(),second]);ctx.exact_object_reference_tags.insert("parent".into());
        let effect=TagMatchingObjectsEffect::new(crate::target::ObjectFilter::default(),"alias").from_tagged_sources(vec!["parent".into()]);
        effect.execute(&mut game,&mut ctx).unwrap();assert_eq!(ctx.get_tagged_all("alias").unwrap().len(),2);assert!(ctx.exact_object_reference_tags.contains(&crate::tag::TagKey::from("alias")));
        ctx.set_tagged_objects("alias",vec![first]);assert!(!ctx.exact_object_reference_tags.contains(&crate::tag::TagKey::from("alias")),"an ordinary local producer shadows the imported alias and its policy");
        crate::effects::CostExecutableEffect::preview_payment_bindings(&effect,&game,&mut ctx).unwrap();assert_eq!(ctx.get_tagged_all("alias").unwrap().len(),2);assert!(ctx.exact_object_reference_tags.contains(&crate::tag::TagKey::from("alias")));
        ctx.exact_object_reference_tags.clear();effect.execute(&mut game,&mut ctx).unwrap();assert_eq!(ctx.get_tagged_all("alias").unwrap().len(),1,"ordinary physical-card unions preserve their existing stable identity semantics");
    }

}
