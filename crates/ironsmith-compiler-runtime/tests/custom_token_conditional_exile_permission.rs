//! Resolving control-duration permissions expire permanently and retain their mana rider.
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm, resolve_stack_entry_with};
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_artifact(name, text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn offered(game: &GameState, id: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
}

#[test]
fn control_duration_starts_only_when_true_and_never_resumes_after_failure() {
    for (initially_owned, lose_and_regain) in [(false, false), (true, true), (true, false)] {
    let text = "Mana cost: {0}\nType: Sorcery\nExile the top card of target opponent's library. You may cast it as long as you control a Warrior, and you may spend mana as though it were mana of any color to cast it.";
    for definition in definitions("Conditional exile program", text) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        let warrior = ironsmith_compiler_runtime::compile_to_runtime_definition("Warrior", "Type: Creature — Warrior\nPower/Toughness: 2/2", false).unwrap();
        game.create_object_from_definition(&warrior, B, Zone::Battlefield);
        let own = initially_owned.then(|| game.create_object_from_definition(&warrior, A, Zone::Battlefield));
        let card = ironsmith_compiler_runtime::compile_to_runtime_definition("Blue card", "Mana cost: {U}\nType: Instant\nYou gain 1 life.", false).unwrap();
        let card = game.create_object_from_definition(&card, B, Zone::Library);
        let stable = game.object(card).unwrap().stable_id;
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let mut context = EffectContext::new_default(source, A).with_targets(vec![ResolvedTarget::Player(B)]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() { execute_effect(&mut game, effect, &mut context).unwrap(); }
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red, 1);
        if let Some(own) = own {
            assert!(offered(&game, exiled).is_some(), "a duration starts when its control requirement is true");
            if lose_and_regain {
                game.move_object_by_effect(own, Zone::Graveyard).unwrap();
                game.refresh_continuous_state().unwrap();
                // Deliberately do not query permission while the condition is false.
                game.create_object_from_definition(&warrior, A, Zone::Battlefield);
                game.refresh_continuous_state().unwrap();
                assert!(offered(&game, exiled).is_none(), "an ended duration cannot restart after a gap without queries");
                continue;
            }
        } else {
            game.create_object_from_definition(&warrior, A, Zone::Battlefield);
            game.refresh_continuous_state().unwrap();
            assert!(offered(&game, exiled).is_none(), "a duration initially false never starts; opponent Warrior is irrelevant");
            continue;
        }
        assert!(!compute_legal_actions(&game, B).unwrap().into_iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if spell_id == exiled)), "the owner does not receive the permission");
        let action = offered(&game, exiled).unwrap();
        let mut state = PriorityLoopState::new(2);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..64 {
            if !state.has_pending_action() { break; }
            let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &context, &mut dm).unwrap();
        }
        assert!(!state.has_pending_action());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().life, 21);
        assert_eq!(game.player(B).unwrap().life, 20);
    }
}
    }

#[test]
fn full_creeds_desperado_composes_nonmana_price_attack_condition_and_conditional_permission() {
    let text = "Mana cost: {2}{R}\nType: Creature — Spirit Warrior\nPower/Toughness: 3/2\nYou may exile exactly two red cards from your hand and/or graveyard and sacrifice a creature rather than pay this spell's mana cost.\nHaste\nWhenever Creed's Desperado attacks, if defending player has more cards in hand than you, exile the top card of that player's library. You may cast it as long as you control a Warrior, and you may spend mana as though it were mana of any color to cast it.";
    for definition in definitions("Creed's Desperado", text) {
        assert_eq!(definition.alternative_casts.len(), 1);
        assert!(definition.abilities.iter().any(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_))));
    }
}

#[test]
fn independently_stated_exile_duration_with_if_control_gate_can_resume() {
    let text = "Mana cost: {0}\nType: Sorcery\nExile the top card of target opponent's library. For as long as it remains exiled, you may cast it if you control a Warrior.";
    for definition in definitions("Retained conditional permission", text) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        let warrior = ironsmith_compiler_runtime::compile_to_runtime_definition("Warrior", "Type: Creature — Warrior\nPower/Toughness: 2/2", false).unwrap();
        let card = ironsmith_compiler_runtime::compile_to_runtime_definition("Free card", "Mana cost: {0}\nType: Instant\nYou gain 1 life.", false).unwrap();
        let card = game.create_object_from_definition(&card, B, Zone::Library);
        let stable = game.object(card).unwrap().stable_id;
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let mut context = EffectContext::new_default(source, A).with_targets(vec![ResolvedTarget::Player(B)]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() { execute_effect(&mut game, effect, &mut context).unwrap(); }
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert!(offered(&game, exiled).is_none());
        let own = game.create_object_from_definition(&warrior, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert!(offered(&game, exiled).is_some(), "a use condition can first become true after the grant resolves");
        game.move_object_by_effect(own, Zone::Graveyard).unwrap();
        game.refresh_continuous_state().unwrap();
        game.create_object_from_definition(&warrior, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert!(offered(&game, exiled).is_some(), "an if-control gate resumes within its independent exile duration");
    }
}
