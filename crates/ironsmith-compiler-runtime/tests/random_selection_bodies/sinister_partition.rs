use super::*;
struct Targets(Vec<ObjectId>);
impl DecisionMaker for Targets {
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        assert!(self.0.iter().all(|id| context.requirements.iter().any(|requirement| requirement.legal_targets.contains(&Target::Object(*id)))));
        self.0.iter().map(|id| Target::Object(*id)).collect()
    }
    fn decide_objects(&mut self, _: &GameState, _: &SelectObjectsContext) -> Vec<ObjectId> {
        panic!("the announced graveyard set is partitioned randomly at resolution");
    }
}
fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut Targets) {
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red, 10);
    let spell = game.create_object_from_definition(definition, A, Zone::Hand);
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: spell, from_zone: Zone::Hand,
            casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
        }), dm).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none());
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].targets.len(), 3);
    assert_eq!(game.irreversible_random_count(), 0, "the random subset is not selected during announcement");
}
#[test]
fn sinister_declares_three_then_partitions_only_the_surviving_announced_incarnations() {
    for definition in definitions("Sinister Waltz") {
        for lost in 0..=3 {
            let mut game = game();
            let mut targets = Vec::new();
            let mut identities = Vec::new();
            for index in 0..3 {
                let id = creature(&mut game, A, Zone::Graveyard, &format!("Target {index}"), "Bear", 2);
                targets.push(id);
                identities.push(game.object(id).unwrap().stable_id);
            }
            let unchosen = creature(&mut game, A, Zone::Graveyard, "Never targeted", "Bear", 2);
            let foreign = creature(&mut game, B, Zone::Graveyard, "Foreign", "Bear", 2);
            let mut dm = Targets(targets.clone());
            cast(&mut game, &definition, &mut dm);
            for id in targets.iter().take(lost) {
                let exiled = game.move_object_by_effect(*id, Zone::Exile).unwrap();
                game.move_object_by_effect(exiled, Zone::Graveyard).unwrap();
            }
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            let survivors = 3 - lost;
            let returned = identities.iter().filter(|identity| game.find_object_by_stable_id(**identity)
                .is_some_and(|id| game.object(id).unwrap().zone == Zone::Battlefield)).count();
            assert_eq!(returned, survivors.min(2));
            assert_eq!(game.player(A).unwrap().library.len(), usize::from(survivors == 3));
            assert_eq!(game.irreversible_random_count(), u64::from(survivors > 0));
            for identity in identities.iter().take(lost) {
                let current = game.find_object_by_stable_id(*identity).unwrap();
                assert_eq!(game.object(current).unwrap().zone, Zone::Graveyard, "fresh incarnations are no longer announced targets");
            }
            assert_eq!(game.object(unchosen).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.object(foreign).unwrap().zone, Zone::Graveyard);
        }
    }
}
#[test]
fn sinister_complement_excludes_selected_cards_even_when_their_returns_are_prevented() {
    for definition in definitions("Sinister Waltz") {
        let mut game = game();
        let targets: Vec<_> = (0..3).map(|index| creature(&mut game, A, Zone::Graveyard,
            &format!("Target {index}"), "Bear", 2)).collect();
        let mut dm = Targets(targets.clone());
        cast(&mut game, &definition, &mut dm);
        let spell = game.stack[0].object_id;
        game.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(spell, A,
                ironsmith::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                    ironsmith::target::ObjectFilter::creature()),
                ironsmith::replacement::ReplacementAction::Prevent));
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.battlefield.is_empty());
        assert_eq!(game.player(A).unwrap().library.len(), 1);
        assert_eq!(targets.iter().filter(|id| game.object(**id).is_some_and(|object| object.zone == Zone::Graveyard)).count(), 2);
        assert_eq!(game.irreversible_random_count(), 1);
    }
}
#[test]
fn replacement_added_movement_cannot_make_the_remainder_follow_a_fresh_incarnation() {
    for definition in definitions("Sinister Waltz") {
        for blink_back in [false, true] {
            let mut game = game();
            let targets: Vec<_> = (0..3).map(|index| creature(&mut game, A, Zone::Graveyard,
                &format!("Target {index}"), "Bear", 2)).collect();
            let identities: Vec<_> = targets.iter().map(|id| game.object(*id).unwrap().stable_id).collect();
            let mut dm = Targets(targets);
            cast(&mut game, &definition, &mut dm);
            let source = game.stack[0].object_id;
            let mut additions = vec![Effect::move_to_zone(ChooseSpec::All(
                ironsmith::target::ObjectFilter::creature().in_zone(Zone::Graveyard)
                    .owned_by(ironsmith::target::PlayerFilter::You)), Zone::Exile, false)];
            if blink_back {
                additions.push(Effect::move_to_zone(ChooseSpec::All(
                    ironsmith::target::ObjectFilter::creature().in_zone(Zone::Exile)
                        .owned_by(ironsmith::target::PlayerFilter::You)), Zone::Graveyard, false));
            }
            game.effect_store.replacement_effects.add_one_shot_effect(
                ironsmith::replacement::ReplacementEffect::with_matcher(source, A,
                    ironsmith::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                        ironsmith::target::ObjectFilter::creature()),
                    ironsmith::replacement::ReplacementAction::Additionally(additions)));
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.battlefield.len(), 2);
            assert!(game.player(A).unwrap().library.is_empty(),
                "Sinister may move only the original unselected incarnation");
            let remainder = identities.iter().filter_map(|identity| game.find_object_by_stable_id(*identity))
                .find(|id| game.object(*id).unwrap().zone != Zone::Battlefield).unwrap();
            assert_eq!(game.object(remainder).unwrap().zone, if blink_back { Zone::Graveyard } else { Zone::Exile });
            assert_eq!(game.irreversible_random_count(), 1);
        }
    }
}
