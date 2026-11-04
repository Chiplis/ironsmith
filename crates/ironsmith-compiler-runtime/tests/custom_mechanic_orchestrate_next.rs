//! Independent ordinary preparation, derived cross-card payment, and a
//! one-shot trigger after successful spell resolution. Requires spell-resolution patch.
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith::card::CardBuilder;
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::types::CardType;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::special_actions::{SpecialAction, can_perform_check, perform};
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn definitions(text: &str) -> [ironsmith::cards::CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Preparation audit", text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let direct = direct.unwrap();
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Preparation audit", text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    let restored = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game
}
fn preparation_action(game:&GameState,card:ironsmith::ObjectId)->SpecialAction {
 if let Some((index,method))=game.object(card).unwrap().alternative_casts.iter().enumerate().find(|(_,method)|method.exile_preparation().is_some()) {
  return SpecialAction::PrepareExile {card_id:card,method_index:index,preparation:method.exile_preparation().unwrap().clone()};
 }
 let grants=game.effect_store.grant_registry.granted_alternative_casts_for_card(game,card,Zone::Hand,A);
 let (index,grant)=grants.iter().enumerate().find(|(_,grant)|grant.method.exile_preparation().is_some()).expect("exact granted preparation");
 SpecialAction::PrepareGrantedExile {card_id:card,grant:ironsmith::alternative_cast::GrantSelection {identity:grant.permission_identity.clone().unwrap(),source:grant.source_id,index},preparation:grant.method.exile_preparation().unwrap().clone()}
}
fn resolution_event(game: &mut GameState, controller: PlayerId) -> ironsmith::triggers::TriggerEvent {
    let card = CardBuilder::new(CardId::new(), "Resolved spell witness").card_types(vec![CardType::Instant]).build();
    let spell = game.create_object_from_card(&card, controller, Zone::Stack);
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(spell).unwrap(), game);
    ironsmith::triggers::TriggerEvent::new(ironsmith::events::spells::SpellResolvedEvent::new(spell, controller, snapshot), Default::default())
}
#[test]
fn intrinsic_preparation_is_immediate_then_one_shot_resolution_trigger_casts_card() {
    for definition in definitions("Mana cost: {2}\nType: Sorcery\nDraw a card.\nYou may pay {G} and exile this card from your hand as a sorcery. When you next resolve a spell, cast this card from exile without paying its mana cost.") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 1);
        let action = preparation_action(&game,source);
        assert!(can_perform_check(&action, &game, A).is_ok());
        perform(action, &mut game, A, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.stack.is_empty(), "preparation uses no stack");
        let exiled = *game.exile.iter().next().unwrap();
        assert!(!game.is_plotted_by(exiled, A), "no Plot designation or later-turn permission");
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        let opponent = resolution_event(&mut game, B);
        assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &opponent).is_empty());
        let own = resolution_event(&mut game, A);
        let entries = ironsmith::triggers::check_delayed_triggers(&mut game, &own);
        assert_eq!(entries.len(), 1);
        assert!(game.effect_store.delayed_triggers.is_empty());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for entry in entries { queue.add(entry); }
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.stack.last().unwrap().is_ability, "the delayed trigger has a response window");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.stack.iter().any(|entry| !entry.is_ability && game.object(entry.object_id).is_some_and(|card| card.name == "Preparation audit")));
        assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &own).is_empty());
    }
}
#[test]
fn derived_preparation_grant_uses_each_matching_hand_cards_reduced_mana_cost() {
    for definition in definitions("Type: Enchantment\nEach instant and sorcery card in your hand has \"You may pay its mana cost reduced by {2} and exile this card from your hand as a sorcery. When you next resolve a spell, cast this card from exile without paying its mana cost.\".") {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let eligible = CardBuilder::new(CardId::new(), "Granted instant").card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(3), ManaSymbol::Green])).build();
        let eligible = game.create_object_from_card(&eligible, A, Zone::Hand);
        let creature = CardBuilder::new(CardId::new(), "Ungrantable creature").card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::new()).build();
        let creature = game.create_object_from_card(&creature, A, Zone::Hand);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 1);
        assert!(can_perform_check(&preparation_action(&game,eligible), &game, A).is_err(), "one more generic is needed");
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
        assert!(can_perform_check(&preparation_action(&game,eligible), &game, A).is_ok());
        assert!(!ironsmith::decision::compute_legal_actions(&game,A).unwrap().iter().any(|action|matches!(action,ironsmith::decision::LegalAction::SpecialAction(SpecialAction::PrepareGrantedExile{card_id,..}) if *card_id==creature)));
        let action=preparation_action(&game,eligible);
        perform(action, &mut game, A, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        assert_eq!(game.object(creature).unwrap().zone, Zone::Hand);
    }
}

#[test]
fn delayed_preparation_trigger_is_consumed_even_when_countered_and_does_not_relink_new_incarnation() {
    let text = "Mana cost: {2}\nType: Sorcery\nDraw a card.\nYou may pay {0} and exile this card from your hand as a sorcery. When you next resolve a spell, cast this card from exile without paying its mana cost.";
    for definition in definitions(text) {
        for leave_exile in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            game.turn.phase = ironsmith::game_state::Phase::Combat;
            assert!(can_perform_check(&preparation_action(&game,source), &game, A).is_err());
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            let action=preparation_action(&game,source);
            perform(action, &mut game, A, &mut SelectFirstDecisionMaker).unwrap();
            let exiled = *game.exile.iter().next().unwrap();
            if leave_exile {
                let hand = game.move_object_by_game_rule(exiled, Zone::Hand).unwrap();
                game.move_object_by_game_rule(hand, Zone::Exile).unwrap();
            }
            let own = resolution_event(&mut game, A);
            let entries = ironsmith::triggers::check_delayed_triggers(&mut game, &own);
            assert_eq!(entries.len(), 1);
            if leave_exile {
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                for entry in entries { queue.add(entry); }
                ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
                assert!(game.stack.is_empty(), "the new exile incarnation cannot use the old trigger");
            }
            // Dropping the other triggered entry models its being countered:
            // consuming the trigger is independent of resolving its effects.
            assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &own).is_empty());
        }
    }
}
#[test]
fn every_exact_orchestrate_source_variant_materializes_full_source_faces_on_both_routes() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-orchestrate-next.json")).unwrap();
    let rows = fixture["mechanics"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    for row in rows {
        assert!(row["original_reminder"].as_str().unwrap().contains("may cast"));
        assert!(row["executable_reminder"].as_str().unwrap().contains("When you next resolve a spell, cast"));
        for card in row["representative_cards"].as_array().unwrap() {
            let mut found = false;
            for face in card["executable_faces"].as_array().unwrap() {
                let text = face["executable_text"].as_str().unwrap();
                for definition in definitions(text) {
                    let debug = format!("{definition:?}");
                    found |= debug.contains("on_next_spell_resolution: true") || debug.contains("ExilePreparationFromCardManaCost");
                }
            }
            assert!(found, "{}", card["name"]);
        }
    }
}
