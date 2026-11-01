//! Rule-only Idolize and source-backed Profit/Forebode behavior, transported through artifacts.
use ironsmith::ability::AbilityKind;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm, resolve_stack_entry_with};
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const GOLD: &str = "Create a colorless Gold artifact token with \"Sacrifice this artifact: Add one mana of any color.\"";
const OMEN: &str = "Create a tapped colorless Omen artifact token with \"{T}, Discard a card, Sacrifice this artifact: Draw a card.\"";
const IDOL: &str = "Create a colorless Idol artifact token with \"{1}, Sacrifice this artifact: Put a +1/+1 counter on target creature. Activate only as a sorcery.\"";
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
    }));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, definition) = result.unwrap_or_else(|e| panic!("{name}: {e}"));
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [definition, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn token(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let source = game.create_object_from_definition(definition, A, Zone::Stack);
    let before = game.battlefield.clone();
    let mut context = EffectContext::new_default(source, A);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() { execute_effect(game, effect, &mut context).unwrap(); }
    *game.battlefield.iter().find(|id| !before.contains(id)).expect("resource token")
}
fn offered(game: &GameState, source: ObjectId) -> bool {
    compute_legal_actions(game, A).unwrap().iter().any(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source))
}
fn activate(game: &mut GameState, source: ObjectId) {
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).expect("legal resource activation");
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..64 {
        if state.pending_activation.is_none() && !game.stack.is_empty() { return; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm).unwrap();
    }
    panic!("resource activation failed to finish");
}
#[test]
fn profit_gold_sacrifices_for_mana_without_tapping_or_using_stack() {
    for definition in definitions("Profit action", &format!("Type: Sorcery\n{GOLD}.")) {
        let mut game = game();
        let gold = token(&mut game, &definition);
        // Gold lacks a tap cost: it still works after an unrelated effect taps it.
        game.tap(gold);
        let ability_index = game.object(gold).unwrap().abilities.iter().position(|a| matches!(a.kind, AbilityKind::Activated(_))).unwrap();
        let stack_before = game.stack.len();
        ironsmith::special_actions::perform_activate_mana_ability(&mut game, A, gold, ability_index, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.stack.len(), stack_before, "Gold's activation is a mana ability");
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
        assert!(!game.battlefield.contains(&gold));
    }
}
#[test]
fn forebode_omen_enters_tapped_and_pays_discard_tap_and_sacrifice_before_drawing() {
    for definition in definitions("Forebode action", &format!("Type: Sorcery\n{OMEN}.")) {
        let mut game = game();
        let omen = token(&mut game, &definition);
        assert!(game.is_tapped(omen));
        let blank = ironsmith_compiler_runtime::compile_to_runtime_definition("Spare parchment", "Type: Sorcery", false).unwrap();
        let discard = game.create_object_from_definition(&blank, A, Zone::Hand);
        let discard_stable = game.object(discard).unwrap().stable_id;
        let draw = game.create_object_from_definition(&blank, A, Zone::Library);
        let draw_stable = game.object(draw).unwrap().stable_id;
        assert!(!offered(&game, omen), "tapped Omen cannot pay its tap cost");
        game.untap(omen);
        activate(&mut game, omen);
        assert!(!game.battlefield.contains(&omen), "sacrifice is paid before resolution");
        assert_eq!(game.object(game.find_object_by_stable_id(discard_stable).unwrap()).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(draw).unwrap().zone, Zone::Library, "draw waits for resolution");
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.object(game.find_object_by_stable_id(draw_stable).unwrap()).unwrap().zone, Zone::Hand);
    }
}
#[test]
fn idolize_rule_only_idol_cost_and_sorcery_timing_are_preserved() {
    for definition in definitions("Idolize rule-only action", &format!("Type: Sorcery\n{IDOL}.")) {
        let mut game = game();
        let idol = token(&mut game, &definition);
        let creature = ironsmith_compiler_runtime::compile_to_runtime_definition("Creature", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
        let target = game.create_object_from_definition(&creature, A, Zone::Battlefield);
        assert!(!offered(&game, idol), "Idol requires one mana");
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        assert!(!offered(&game, idol), "Idol activates only as a sorcery");
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        activate(&mut game, idol);
        assert!(!game.battlefield.contains(&idol));
        assert_eq!(game.counter_count(target, ironsmith::object::CounterType::PlusOnePlusOne), 0);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.counter_count(target, ironsmith::object::CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn scoped_omen_taxonomy_preserves_official_spell_and_artifact_type_changes() {
    use ironsmith::types::{CardType, Subtype, SubtypeFamily};
    assert!(Subtype::Omen.belongs_to_family(SubtypeFamily::Spell));
    assert!(!Subtype::Omen.belongs_to_family(SubtypeFamily::Artifact));
    assert!(Subtype::ArtifactOmen.belongs_to_family(SubtypeFamily::Artifact));
    assert!(!Subtype::ArtifactOmen.belongs_to_family(SubtypeFamily::Spell));
    for text in ["Type: Sorcery — Omen", "Type: Artifact — Omen"] {
        for definition in definitions("Scoped Omen", text) {
            let mut game = game();
            let official = definition.card.card_types.contains(&CardType::Sorcery);
            let id = game.create_object_from_definition(&definition, A, if official {Zone::Stack} else {Zone::Battlefield});
            assert_eq!(game.object(id).unwrap().subtypes, vec![if official {Subtype::Omen} else {Subtype::ArtifactOmen}]);
            let effect = ironsmith::Effect::new(ironsmith::effects::ApplyContinuousEffect::new(
                ironsmith::continuous::EffectTarget::Specific(id),
                ironsmith::continuous::Modification::SetCardTypes(vec![CardType::Artifact]),
                ironsmith::effect::Until::EndOfTurn,
            ));
            execute_effect(&mut game, &effect, &mut EffectContext::new_default(id, A)).unwrap();
            let changed = game.current_characteristics(id).unwrap();
            assert!(changed.card_types.contains(&CardType::Artifact), "type-changing effect must actually apply before testing subtype retention");
            // CR205.1a preserves instant/sorcery types when setting other
            // card types, so the official spell subtype correctly survives.
            assert_eq!(changed.subtypes.contains(&Subtype::Omen), official);
            assert_eq!(changed.subtypes.contains(&Subtype::ArtifactOmen), !official);
            let remove_spell_type = ironsmith::Effect::new(ironsmith::effects::ApplyContinuousEffect::new(
                ironsmith::continuous::EffectTarget::Specific(id),
                ironsmith::continuous::Modification::RemoveCardTypes(vec![CardType::Sorcery]),
                ironsmith::effect::Until::EndOfTurn,
            ));
            execute_effect(&mut game, &remove_spell_type, &mut EffectContext::new_default(id, A)).unwrap();
            let changed = game.current_characteristics(id).unwrap();
            assert!(!changed.subtypes.contains(&Subtype::Omen), "explicitly losing the spell type removes its spell subtype");
            assert_eq!(changed.subtypes.contains(&Subtype::ArtifactOmen), !official);
        }
    }
}

#[test]
fn subtype_label_filters_preserve_names_negation_and_conjunction() {
    let named = definitions("Named recovery", "Type: Sorcery\nReturn all cards named Omen of the Sea from your graveyard to your hand.");
    for recovery in named {
        let mut game = game();
        let correct = ironsmith_compiler_runtime::compile_to_runtime_definition("Omen of the Sea", "Type: Enchantment", false).unwrap();
        let wrong = ironsmith_compiler_runtime::compile_to_runtime_definition("Different Omen", "Type: Artifact — Omen", false).unwrap();
        let expected = game.create_object_from_definition(&correct, A, Zone::Graveyard);
        let expected_stable = game.object(expected).unwrap().stable_id;
        let other = game.create_object_from_definition(&wrong, A, Zone::Graveyard);
        let source = game.create_object_from_definition(&recovery, A, Zone::Stack);
        let mut context = EffectContext::new_default(source, A);
        for effect in recovery.spell_effect.as_ref().unwrap().all_effects() { execute_effect(&mut game, effect, &mut context).unwrap(); }
        assert_eq!(game.object(game.find_object_by_stable_id(expected_stable).unwrap()).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(other).unwrap().zone, Zone::Graveyard, "Omen inside a proper name is never a subtype selector");
    }
    for (rules, expected) in [("Untap each non-Omen artifact you control.", [false, false, true]), ("Untap each Omen Equipment you control.", [false, true, false])] {
        for action in definitions("Subtype selector", &format!("Type: Sorcery\n{rules}")) {
            let mut game = game();
            let mut ids = Vec::new();
            for (name, typeline) in [("Omen", "Artifact — Omen"), ("Omen Gear", "Artifact — Omen Equipment"), ("Gear", "Artifact — Equipment")] {
                let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(name, format!("Type: {typeline}"), false).unwrap();
                let id = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                game.tap(id);
                ids.push(id);
            }
            let source = game.create_object_from_definition(&action, A, Zone::Stack);
            let mut context = EffectContext::new_default(source, A);
            for effect in action.spell_effect.as_ref().unwrap().all_effects() { execute_effect(&mut game, effect, &mut context).unwrap(); }
            for (id, untapped) in ids.into_iter().zip(expected) { assert_eq!(!game.is_tapped(id), untapped, "{rules}"); }
        }
    }
}

#[test]
fn profit_optional_player_is_departing_creatures_controller_not_owner_or_spell_controller() {
    struct ControllerChoice { expected: PlayerId, accept: bool, offered: usize }
    impl DecisionMaker for ControllerChoice {
        fn decide_boolean(&mut self, _: &GameState, context: &ironsmith::decisions::context::BooleanContext) -> bool {
            assert_eq!(context.player, self.expected, "zone-change LKI determines the chooser");
            self.offered += 1;
            self.accept
        }
    }
    for relation in ["controller", "owner"] {
      let b = PlayerId(1);
      let expected = if relation == "controller" { b } else { A };
      for action in definitions("Turn to Gold", &format!("Type: Sorcery\nExile target creature. Its {relation} may {GOLD}.")) {
        for accept in [false, true] {
            let mut game = game();
            let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Borrowed creature", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
            let creature = game.create_object_from_definition(&body, A, Zone::Battlefield);
            let stable = game.object(creature).unwrap().stable_id;
            game.set_current_controller(creature, b).unwrap();
            let source = game.create_object_from_definition(&action, A, Zone::Stack);
            let mut dm = ControllerChoice { expected, accept, offered: 0 };
            let mut context = EffectContext::new_default(source, A)
                .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(creature)])
                .with_decision_maker(&mut dm);
            for effect in action.spell_effect.as_ref().unwrap().all_effects() { execute_effect(&mut game, effect, &mut context).unwrap(); }
            drop(context);
            assert_eq!(dm.offered, 1);
            assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Exile);
            let gold: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|o| o.subtypes.contains(&ironsmith::types::Subtype::Gold)).collect();
            assert_eq!(gold.len(), usize::from(accept));
            if let Some(token) = gold.first() { assert_eq!(game.controller_of(token), expected); assert_eq!(token.owner, expected); }
        }
    }
  }
}
