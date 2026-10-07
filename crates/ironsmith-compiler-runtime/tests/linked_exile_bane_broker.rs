//! Frozen complete Bane Alley Broker scenarios, source-authored and UNRUN.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::SelectObjectsContext;
use ironsmith::effects::{EffectContext, ExecutionError, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);
fn body() -> String {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/linked_exile_static_permissions.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Bane Alley Broker").unwrap();
    format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(), row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(), row["oracle_text"].as_str().unwrap())
}
fn definitions() -> [CardDefinition; 2] {
    let text = body();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Bane Alley Broker", &text, false));
    let direct = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Bane Alley Broker", &text, false));
    let (artifact, _) = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap(); let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap(); assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    game.turn.phase = Phase::FirstMain; game.turn.step = None; game.turn.active_player = A; game.turn.priority_player = Some(A); game
}
fn card(game: &mut GameState, player: PlayerId, zone: Zone) -> ObjectId {
    let definition = compile_to_runtime_definition("Kept card", "Mana cost: {0}\nType: Artifact", false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
fn source(game: &mut GameState, definition: &CardDefinition, player: PlayerId) -> ObjectId {
    let source = game.create_object_from_definition(definition, player, Zone::Battlefield); game.remove_summoning_sickness(source); source
}
#[derive(Default)]
struct Choices { pick: Option<ObjectId>, chooser: Option<PlayerId>, forbidden: Vec<ObjectId>, pause: bool, pending: bool, questions: usize }
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool { true }
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        assert!(self.forbidden.iter().all(|id| !ctx.candidates.iter().any(|candidate| candidate.id == *id && candidate.legal)));
        if let Some(pick) = self.pick {
            assert!(ctx.candidates.iter().any(|candidate| candidate.id == pick && candidate.legal));
            if let Some(chooser) = self.chooser { assert_eq!(ctx.player, chooser); }
            self.questions += 1;
            if self.pause { self.pending = true; return vec![]; }
            vec![pick]
        } else { SelectFirstDecisionMaker.decide_objects(game, ctx) }
    }
}
fn activation(game: &GameState, source: ObjectId, player: PlayerId, ordinal: usize) -> Option<LegalAction> {
    let index = game.current_abilities(source)?.iter().enumerate().filter(|(_, ability)| matches!(ability.kind, AbilityKind::Activated(_))).nth(ordinal)?.0;
    compute_legal_actions(game, player).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::ActivateAbility { source: id, ability_index } if *id == source && *ability_index == index))
}
fn activate(game: &mut GameState, source: ObjectId, player: PlayerId, ordinal: usize) {
    game.turn.priority_player = Some(player);
    let action = activation(game, source, player, ordinal).expect("full-body activation is legal");
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut SelectFirstDecisionMaker).unwrap();
    for _ in 0..32 {
        if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending activation has a decision"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut SelectFirstDecisionMaker).unwrap();
    }
    assert!(!state.has_pending_action()); assert!(game.is_tapped(source)); assert!(game.stack.last().unwrap().linked_exile_owner.is_some());
}
fn resolve(game: &mut GameState, choices: &mut Choices) { resolve_stack_entry_with(game, choices).unwrap(); }
fn mana(game: &mut GameState, player: PlayerId) {
    game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Blue, 1); game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Black, 1);
}
fn produce(game: &mut GameState, source: ObjectId, player: PlayerId, ordinal: usize, pick: ObjectId) -> (ObjectId, ironsmith::linked_exile::LinkedExileOwner) {
    let stable = game.object(pick).unwrap().stable_id; card(game, player, Zone::Library); game.untap(source);
    activate(game, source, player, ordinal); let owner = game.stack.last().unwrap().linked_exile_owner.clone().unwrap();
    resolve(game, &mut Choices { pick: Some(pick), chooser: Some(player), ..Default::default() });
    let member = game.find_object_by_stable_id(stable).unwrap(); assert_eq!(game.object(member).unwrap().zone, Zone::Exile); assert!(game.is_face_down(member));
    (member, owner)
}
fn look(game: &GameState, member: ObjectId, player: PlayerId) -> bool { game.can_player_look_at_face_down_exiled_card(member, player) }
fn no_play(game: &GameState, member: ObjectId, player: PlayerId) {
    assert!(!game.effect_store.grant_registry.card_can_play_from_zone(game, member, Zone::Exile, player));
}
fn control(game: &mut GameState, source: ObjectId, player: PlayerId) {
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::gain_control(source, A, source, player));
    game.refresh_continuous_state().unwrap(); game.remove_summoning_sickness(source);
}

#[test]
fn complete_body_has_three_linked_members_and_inspection_never_grants_casting() {
    for definition in definitions() {
        assert_eq!(definition.abilities.len(), 3); assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let pairs = definition.abilities.iter().filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(ability) => Some(ability.effects.linked_exile_pair.unwrap()),
            AbilityKind::Static(ability) => Some(ability.source_exiled_inspection_pair().unwrap().unwrap()), _ => None,
        }).collect::<Vec<_>>();
        assert_eq!(pairs.len(), 3); assert!(pairs.iter().all(|pair| pair == &pairs[0]));
        let mut game = game(); let source = source(&mut game, &definition, A); let victim = card(&mut game, A, Zone::Hand);
        let before = game.player(A).unwrap().hand.len(); let (member, owner) = produce(&mut game, source, A, 0, victim);
        assert_eq!(game.player(A).unwrap().hand.len(), before, "draw one, then exile one");
        assert_eq!(game.linked_exile_pair_members(&owner).unwrap(), &[member]);
        assert!(look(&game, member, A)); assert!(!look(&game, member, B)); no_play(&game, member, A);
        mana(&mut game, A); assert!(activation(&game, source, A, 1).is_none(), "return also requires tapping");
        game.untap(source); activate(&mut game, source, A, 1);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve(&mut game, &mut Choices::default()); assert!(game.linked_exile_pair_members(&owner).unwrap().is_empty());
        assert_eq!(game.player(A).unwrap().hand.len(), before + 1);
    }
}

#[test]
fn singular_return_ignores_foreign_links_and_known_empty_pair_never_adopts_the_union() {
    for definition in definitions() {
        let mut game = game(); let source = source(&mut game, &definition, A);
        let first = card(&mut game, A, Zone::Hand); let second = card(&mut game, A, Zone::Hand);
        let (first, owner) = produce(&mut game, source, A, 0, first); let (second, _) = produce(&mut game, source, A, 0, second);
        let unrelated = card(&mut game, B, Zone::Exile); game.set_face_down(unrelated); game.add_exiled_with_source_link(source, unrelated);
        assert!(!look(&game, unrelated, A));
        for (pick, remaining) in [(first, 1), (second, 0)] {
            game.untap(source); mana(&mut game, A); activate(&mut game, source, A, 1);
            resolve(&mut game, &mut Choices { pick: Some(pick), forbidden: vec![unrelated], ..Default::default() });
            assert_eq!(game.linked_exile_pair_members(&owner).unwrap().len(), remaining);
        }
        let before = game.player(A).unwrap().hand.len(); game.untap(source); mana(&mut game, A); activate(&mut game, source, A, 1);
        resolve(&mut game, &mut Choices { forbidden: vec![unrelated], ..Default::default() });
        assert_eq!(game.player(A).unwrap().hand.len(), before); assert_eq!(game.object(unrelated).unwrap().zone, Zone::Exile);
    }
}

#[test]
fn a_later_controller_returns_to_the_cards_owner_and_prior_inspection_persists() {
    for definition in definitions() {
        let mut game = game(); let source = source(&mut game, &definition, B); let victim = card(&mut game, B, Zone::Hand);
        let stable = game.object(victim).unwrap().stable_id; let (member, _) = produce(&mut game, source, B, 0, victim);
        control(&mut game, source, A); assert!(look(&game, member, A)); assert!(look(&game, member, B)); no_play(&game, member, A);
        let before_a = game.player(A).unwrap().hand.len(); let before_b = game.player(B).unwrap().hand.len();
        game.untap(source); mana(&mut game, A); activate(&mut game, source, A, 1); resolve(&mut game, &mut Choices::default());
        let returned = game.find_object_by_stable_id(stable).unwrap(); assert_eq!(game.object(returned).unwrap().owner, B);
        assert_eq!(game.object(returned).unwrap().zone, Zone::Hand); assert_eq!(game.player(B).unwrap().hand.len(), before_b + 1);
        assert_eq!(game.player(A).unwrap().hand.len(), before_a);
    }
}

#[test]
fn copied_pending_returns_keep_old_host_members_after_blink_and_do_not_rebind_new_abilities() {
    for definition in definitions() {
        let mut game = game(); let source = source(&mut game, &definition, A); let first = card(&mut game, A, Zone::Hand); let second = card(&mut game, A, Zone::Hand);
        let (first, owner) = produce(&mut game, source, A, 0, first); let (second, _) = produce(&mut game, source, A, 0, second);
        game.untap(source); mana(&mut game, A); activate(&mut game, source, A, 1); let entry = game.stack.last().unwrap().target_id();
        execute_effect(&mut game, &Effect::copy_spell(ChooseSpec::SpecificObject(entry)), &mut EffectContext::new(source, A, &mut SelectFirstDecisionMaker)).unwrap();
        assert_eq!(game.stack.len(), 2); assert!(game.stack.iter().all(|entry| entry.linked_exile_owner.as_ref() == Some(&owner)));
        let hand = game.move_object_by_game_rule(source, Zone::Hand).unwrap(); let returned = game.move_object_by_game_rule(hand, Zone::Battlefield).unwrap();
        control(&mut game, returned, C); assert!(!look(&game, first, C)); assert!(look(&game, first, A));
        game = game.clone(); resolve(&mut game, &mut Choices { pick: Some(first), ..Default::default() });
        resolve(&mut game, &mut Choices { pick: Some(second), ..Default::default() });
        assert!(game.linked_exile_pair_members(&owner).unwrap().is_empty());
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
    }
}

#[test]
fn independently_borrowed_activated_pairs_cannot_return_each_others_exiles() {
    for definition in definitions() {
        let mut game = game(); let host_def = compile_to_runtime_definition("Borrowing host", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
        let host = source(&mut game, &host_def, A);
        let donors = [game.create_object_from_definition(&definition, A, Zone::Exile), game.create_object_from_definition(&definition, A, Zone::Exile)];
        for donor in donors { game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(host, A, EffectTarget::Specific(host),
            Modification::CopyActivatedAbilities { filter: ObjectFilter::specific(donor), counter: None, include_mana: true,
                only_loyalty: false, exclude_source_name: false, exclude_source_id: true, force_once_each_turn: false })); }
        game.refresh_continuous_state().unwrap(); let first = card(&mut game, A, Zone::Hand); let second = card(&mut game, A, Zone::Hand);
        let (first, first_owner) = produce(&mut game, host, A, 0, first); let (second, second_owner) = produce(&mut game, host, A, 2, second);
        assert_ne!(first_owner, second_owner); assert!(!look(&game, first, A)); assert!(!look(&game, second, A));
        game.untap(host); mana(&mut game, A); activate(&mut game, host, A, 1);
        resolve(&mut game, &mut Choices { forbidden: vec![second], ..Default::default() });
        assert!(game.linked_exile_pair_members(&first_owner).unwrap().is_empty()); assert_eq!(game.linked_exile_pair_members(&second_owner).unwrap(), &[second]);
        game.untap(host); mana(&mut game, A); activate(&mut game, host, A, 3); resolve(&mut game, &mut Choices::default());
        assert!(game.linked_exile_pair_members(&second_owner).unwrap().is_empty());
    }
}

#[test]
fn pending_private_choice_rolls_back_draw_and_exile_and_recovers_from_native_state() {
    for definition in definitions() {
        let mut game = game(); let source = source(&mut game, &definition, A); let victim = card(&mut game, A, Zone::Hand); let drawn = card(&mut game, A, Zone::Library);
        activate(&mut game, source, A, 0); let owner = game.stack[0].linked_exile_owner.clone().unwrap(); let next_id = game.next_object_id_counter();
        let mut choices = Choices { pick: Some(victim), chooser: Some(A), pause: true, ..Default::default() };
        resolve(&mut game, &mut choices); assert!(choices.pending);
        assert_eq!(game.object(drawn).unwrap().zone, Zone::Library); assert_eq!(game.object(victim).unwrap().zone, Zone::Hand);
        assert!(game.exile.is_empty()); assert!(game.linked_exile_pair_members(&owner).unwrap().is_empty()); assert_eq!(game.next_object_id_counter(), next_id);
        assert!(game.is_tapped(source), "an already paid activation cost remains paid");
        game = game.clone(); resolve(&mut game, &mut Choices { pick: Some(victim), chooser: Some(A), ..Default::default() });
        let members = game.linked_exile_pair_members(&owner).unwrap(); assert_eq!(members.len(), 1); assert!(look(&game, members[0], A));
    }
}

#[test]
fn source_only_import_and_lost_pending_owner_fail_instead_of_returning_foreign_cards() {
    for definition in definitions() {
        let mut game = game(); let source = source(&mut game, &definition, A); let victim = card(&mut game, A, Zone::Hand); let (member, _) = produce(&mut game, source, A, 0, victim);
        game.untap(source); mana(&mut game, A); activate(&mut game, source, A, 1); let saved = game.clone();
        game.replace_exiled_with_source_links(std::collections::HashMap::from([(source, vec![member])]));
        assert!(matches!(resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker),
            Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(ExecutionError::IncompleteEvidence(_)))));
        assert_eq!(game.object(member).unwrap().zone, Zone::Exile);
        game = saved.clone(); game.stack[0].linked_exile_owner = None;
        assert!(matches!(resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker),
            Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(ExecutionError::IncompleteEvidence(_)))));
        game = saved; resolve(&mut game, &mut Choices::default()); assert!(game.object(member).is_none());
    }
}

#[test]
fn direct_native_return_context_overrides_an_unrelated_source_wide_tag() {
    for definition in definitions() {
        let mut game = game(); let source = source(&mut game, &definition, A); let victim = card(&mut game, A, Zone::Hand); let (member, owner) = produce(&mut game, source, A, 0, victim);
        let foreign = card(&mut game, B, Zone::Exile); game.add_exiled_with_source_link(source, foreign);
        let returned = definition.abilities.iter().filter_map(|ability| match &ability.kind { AbilityKind::Activated(ability) => Some(ability), _ => None }).nth(1).unwrap();
        let foreign_snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(foreign).unwrap(), &game);
        let mut choices = Choices { forbidden: vec![foreign], ..Default::default() };
        let mut ctx = EffectContext::new(source, A, &mut choices); ctx.linked_exile_owner = Some(owner);
        ctx.set_tagged_objects(ironsmith::tag::SOURCE_EXILED_TAG, vec![foreign_snapshot]);
        for effect in returned.effects.all_effects() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        assert!(game.object(member).is_none()); assert_eq!(game.object(foreign).unwrap().zone, Zone::Exile);
    }
}

#[test]
fn unrelated_extra_body_and_exiling_cost_leave_the_standalone_inspector_unbound() {
    for text in [format!("{}\n{{1}}: Exile target card from a graveyard.", body()),
        body().replace("{U}{B}, {T}:", "{U}{B}, Exile a card from your graveyard:")] {
        let definition = compile_to_runtime_definition("Unproved inspection group", text, false).unwrap();
        assert!(definition.abilities.iter().filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => ability.source_exiled_inspection_pair(), _ => None,
        }).all(|pair| pair.is_none()));
    }
}

#[test]
fn a_missing_inspector_pair_is_incomplete_evidence_and_a_victim_blink_breaks_return_membership() {
    for definition in definitions() {
        let mut incomplete = definition.clone();
        for ability in &mut incomplete.abilities {
            if let AbilityKind::Static(static_ability) = &mut ability.kind {
                let mut model = static_ability.compiled_model().unwrap().clone();
                if let ironsmith_core::StaticAbilityPayload::LookAtSourceExiledCards { pair, .. } = &mut model.payload { *pair = None; }
                *static_ability = ironsmith::static_abilities::StaticAbility::from_model(model);
            }
        }
        let mut missing = game(); source(&mut missing, &incomplete, A);
        assert!(matches!(compute_legal_actions(&missing, A), Err(ExecutionError::IncompleteEvidence(_))));

        let mut game = game(); let source = source(&mut game, &definition, A); let victim = card(&mut game, A, Zone::Hand);
        let (member, owner) = produce(&mut game, source, A, 0, victim);
        let hand = game.move_object_by_game_rule(member, Zone::Hand).unwrap();
        let again = game.move_object_by_game_rule(hand, Zone::Exile).unwrap(); game.set_face_down(again);
        assert!(game.linked_exile_pair_members(&owner).unwrap().is_empty()); assert!(!look(&game, again, A));
        game.untap(source); mana(&mut game, A); activate(&mut game, source, A, 1); resolve(&mut game, &mut Choices::default());
        assert_eq!(game.object(again).unwrap().zone, Zone::Exile);
    }
}

#[test]
fn return_requires_both_blue_and_black_even_when_two_mana_are_available() {
    for definition in definitions() {
        for symbol in [ManaSymbol::Colorless, ManaSymbol::Blue] {
            let mut game = game(); let source = source(&mut game, &definition, A);
            let victim = card(&mut game, A, Zone::Hand); let (member, owner) = produce(&mut game, source, A, 0, victim);
            game.untap(source); game.player_mut(A).unwrap().mana_pool.add(symbol, 2);
            let before = game.player(A).unwrap().mana_pool.clone();
            assert!(activation(&game, source, A, 1).is_none(), "two mana cannot replace the missing colors");
            assert!(!game.is_tapped(source)); assert_eq!(game.player(A).unwrap().mana_pool, before);
            assert!(game.stack.is_empty()); assert_eq!(game.linked_exile_pair_members(&owner).unwrap(), &[member]);
            // A stale or forged action must hit the same payment gate before any cost survives.
            let ability_index = game.current_abilities(source).unwrap().iter().enumerate()
                .filter(|(_, ability)| matches!(ability.kind, AbilityKind::Activated(_))).nth(1).unwrap().0;
            let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(3);
            assert!(apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                &PriorityResponse::PriorityAction(LegalAction::ActivateAbility { source, ability_index }),
                &mut SelectFirstDecisionMaker).is_err());
            assert!(!game.is_tapped(source)); assert_eq!(game.player(A).unwrap().mana_pool, before);
            assert!(game.stack.is_empty()); assert_eq!(game.linked_exile_pair_members(&owner).unwrap(), &[member]);
        }
    }
}

#[test]
fn canonical_reader_and_both_activations_reparse_into_one_complete_pair() {
    for definition in definitions() {
        let rendered = ironsmith_text::canonical_compiled_lines(&definition).join("\n");
        assert!(rendered.contains("You may look at cards exiled with"), "{rendered}");
        let text = format!("Mana cost: {{1}}{{U}}{{B}}\nType: Creature — Human Rogue\nPower/Toughness: 0/3\n{rendered}");
        let (restored, loss) = ironsmith_compiler::parse_loss::capture(||
            compile_to_runtime_definition("Bane Alley Broker", text, false));
        let restored = restored.unwrap_or_else(|error| panic!("{rendered}: {error}"));
        assert!(!loss.is_lossy(), "{rendered}: {}", loss.reasons_text());
        assert_eq!(restored.abilities.len(), 3);
        let pairs = restored.abilities.iter().filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(ability) => Some(ability.effects.linked_exile_pair.unwrap()),
            AbilityKind::Static(ability) => Some(ability.source_exiled_inspection_pair().unwrap().unwrap()), _ => None,
        }).collect::<Vec<_>>();
        assert_eq!(pairs.len(), 3); assert!(pairs.iter().all(|pair| pair == &pairs[0]));
        let mut game = game(); let source = source(&mut game, &restored, A); let victim = card(&mut game, A, Zone::Hand);
        let (member, owner) = produce(&mut game, source, A, 0, victim); assert!(look(&game, member, A)); no_play(&game, member, A);
        game.untap(source); mana(&mut game, A); activate(&mut game, source, A, 1); resolve(&mut game, &mut Choices::default());
        assert!(game.linked_exile_pair_members(&owner).unwrap().is_empty());
    }
}
