//! Source-authored only. All scenarios remain UNRUN under the deferred gate.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack, resolve_stack_entry,
};
use ironsmith::game_state::{Phase, StackEntry, Step, TargetAssignment};
use ironsmith::mana::ManaSymbol;
use ironsmith::special_actions::{ActionError, SpecialAction, can_perform_check};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    turn(&mut game, A);
    for player in [A, B] {
        for mana in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            game.player_mut(player).unwrap().mana_pool.add(mana, 20);
        }
    }
    game
}
fn turn(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
}
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/scoped_action_prohibitions.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap()
    );
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn resource(
    game: &mut GameState,
    player: PlayerId,
    name: &str,
    text: &str,
    zone: Zone,
) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
fn land(game: &mut GameState, player: PlayerId, name: &str, zone: Zone) -> ObjectId {
    resource(game, player, name, "Type: Land", zone)
}
fn library(game: &mut GameState, player: PlayerId, count: usize) {
    for index in 0..count {
        resource(
            game,
            player,
            &format!("Library {index}"),
            "Mana cost: {0}\nType: Artifact",
            Zone::Library,
        );
    }
}
fn can_land(game: &GameState, player: PlayerId, id: ObjectId) -> bool {
    can_perform_check(&SpecialAction::PlayLand { card_id: id }, game, player).is_ok()
}
fn can_cast(game: &GameState, player: PlayerId, id: ObjectId) -> bool {
    compute_legal_actions(game, player)
        .unwrap()
        .iter()
        .any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
}
fn can_activate(game: &GameState, player: PlayerId, id: ObjectId) -> bool {
    compute_legal_actions(game, player).unwrap().iter().any(
        |action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == id),
    )
}
fn settle(game: &mut GameState) {
    for _ in 0..20 {
        put_triggers_on_stack(game, &mut TriggerQueue::new()).unwrap();
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry(game).unwrap();
    }
    panic!("unexpected repeated stack program");
}
struct TargetChoice(Target);
impl DecisionMaker for TargetChoice {
    fn decide_targets(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        assert_eq!(context.requirements.len(), 1);
        assert!(context.requirements[0].legal_targets.contains(&self.0));
        vec![self.0]
    }
}
fn activate(game: &mut GameState, source: ObjectId, target: Option<Target>) {
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: candidate, .. } if *candidate == source)).unwrap();
    let mut target_dm = TargetChoice(target.unwrap_or(Target::Player(A)));
    let mut first = SelectFirstDecisionMaker;
    let dm = &mut target_dm;
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut result = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..20 {
        if state.pending_activation.is_none() {
            break;
        }
        if let ironsmith::GameProgress::NeedsDecisionCtx(context) = result {
            result =
                apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
        } else {
            break;
        }
    }
    resolve_stack_entry(game).unwrap();
}
fn cast_player_spell(game: &mut GameState, definition: &CardDefinition, target: PlayerId) {
    let source = game.create_object_from_definition(definition, A, Zone::Stack);
    let requirements = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
        game,
        definition.spell_effect.as_ref().unwrap(),
        A,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), 1);
    assert!(
        requirements[0]
            .legal_targets
            .contains(&Target::Player(target))
    );
    game.push_to_stack(
        StackEntry::new(source, A)
            .with_targets(vec![Target::Player(target)])
            .with_target_assignments(vec![TargetAssignment {
                spec: requirements[0].spec.clone(),
                range: 0..1,
            }]),
    );
    resolve_stack_entry(game).unwrap();
}
#[test]
fn fourteen_full_oracle_bodies_preserve_direct_and_restored_artifacts() {
    for row in rows() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"].as_str().unwrap());
        }
    }
}
#[test]
fn mining_bans_only_its_controller_tracks_host_lifetime_and_keeps_sacrifice_draw_cost() {
    for definition in definitions("Aggressive Mining") {
        let mut game = game();
        library(&mut game, A, 3);
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = land(&mut game, A, "Own land", Zone::Hand);
        let other = land(&mut game, B, "Other land", Zone::Hand);
        assert_eq!(
            can_perform_check(&SpecialAction::PlayLand { card_id: own }, &game, A),
            Err(ActionError::LandPlayProhibited)
        );
        turn(&mut game, B);
        assert!(can_land(&game, B, other));
        turn(&mut game, A);
        game.phase_out(host);
        assert!(can_land(&game, A, own));
        game.phase_in(host);
        assert!(!can_land(&game, A, own));
        let cost_land = land(&mut game, A, "Sacrificed land", Zone::Battlefield);
        activate(&mut game, host, None);
        assert!(game.object(cost_land).is_none());
        assert_eq!(game.player(A).unwrap().hand.len(), 3);
        land(&mut game, A, "Second cost land", Zone::Battlefield);
        assert!(
            !can_activate(&game, A, host),
            "once each turn survives the legality change"
        );
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        assert!(can_land(&game, A, own));
    }
}
#[test]
fn miner_registration_survives_its_sacrifice_and_binds_the_announced_player_until_cleanup() {
    for definition in definitions("Pardic Miner") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = land(&mut game, A, "Own", Zone::Hand);
        let other = land(&mut game, B, "Other", Zone::Hand);
        activate(&mut game, host, Some(Target::Player(B)));
        assert!(game.object(host).is_none());
        assert!(can_land(&game, A, own));
        turn(&mut game, B);
        assert!(!can_land(&game, B, other));
        let later = land(&mut game, B, "Acquired later", Zone::Hand);
        assert!(!can_land(&game, B, later));
        game.cleanup_restrictions_end_of_turn();
        assert!(can_land(&game, B, other));
        assert!(can_land(&game, B, later));
    }
}
#[test]
fn turf_wound_draws_for_its_caster_and_solfatara_delays_its_draw_until_the_next_upkeep() {
    for name in ["Turf Wound", "Solfatara"] {
        for definition in definitions(name) {
            let mut game = game();
            library(&mut game, A, 3);
            let other = land(&mut game, B, "Restricted land", Zone::Hand);
            cast_player_spell(&mut game, &definition, B);
            assert_eq!(
                game.player(A).unwrap().hand.len(),
                usize::from(name == "Turf Wound")
            );
            turn(&mut game, B);
            assert!(!can_land(&game, B, other));
            game.cleanup_restrictions_end_of_turn();
            assert!(can_land(&game, B, other));
            if name == "Solfatara" {
                game.turn.turn_number += 1;
                game.turn.phase = Phase::Beginning;
                game.turn.step = Some(Step::Upkeep);
                let mut queue = TriggerQueue::new();
                ironsmith::game_loop::generate_and_queue_step_triggers(&mut game, &mut queue);
                put_triggers_on_stack(&mut game, &mut queue).unwrap();
                settle(&mut game);
                assert_eq!(game.player(A).unwrap().hand.len(), 1);
                assert_eq!(game.player(B).unwrap().hand, vec![other]);
            }
        }
    }
}
#[test]
fn frenzy_bans_both_hand_actions_but_allows_the_actual_top_library_card() {
    for definition in definitions("Experimental Frenzy") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let hand_land = land(&mut game, A, "Hand land", Zone::Hand);
        let hand_spell = resource(
            &mut game,
            A,
            "Hand spell",
            "Mana cost: {0}\nType: Artifact",
            Zone::Hand,
        );
        let top_land = land(&mut game, A, "Top land", Zone::Library);
        assert!(!can_land(&game, A, hand_land));
        assert!(!can_cast(&game, A, hand_spell));
        assert!(can_land(&game, A, top_land));
        game.move_object_by_effect(top_land, Zone::Graveyard)
            .unwrap();
        let top_spell = resource(
            &mut game,
            A,
            "Top spell",
            "Mana cost: {0}\nType: Artifact",
            Zone::Library,
        );
        assert!(can_cast(&game, A, top_spell));
        activate(&mut game, host, None);
        assert!(game.object(host).is_none());
        assert!(can_land(&game, A, hand_land));
        assert!(can_cast(&game, A, hand_spell));
        assert!(
            !can_cast(&game, A, top_spell),
            "the source's permission leaves with it"
        );
    }
}
#[test]
fn cornered_market_uses_current_nontoken_names_and_only_nonbasic_lands() {
    for definition in definitions("Cornered Market") {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let permanent = resource(
            &mut game,
            B,
            "Shared name",
            "Mana cost: {0}\nType: Artifact",
            Zone::Battlefield,
        );
        let spell = resource(
            &mut game,
            A,
            "Shared name",
            "Mana cost: {0}\nType: Artifact",
            Zone::Hand,
        );
        let land = land(&mut game, A, "Shared name", Zone::Hand);
        let basic = resource(&mut game, A, "Shared name", "Type: Basic Land", Zone::Hand);
        assert!(!can_cast(&game, A, spell));
        assert!(!can_land(&game, A, land));
        assert!(can_land(&game, A, basic));
        game.phase_out(permanent);
        assert!(can_cast(&game, A, spell));
        assert!(can_land(&game, A, land));
        game.phase_in(permanent);
        assert!(!can_cast(&game, A, spell));
        game.object_mut(permanent).unwrap().kind = ironsmith::object::ObjectKind::Token;
        assert!(can_cast(&game, A, spell));
        assert!(can_land(&game, A, land));
    }
}
#[test]
fn ashes_stops_both_graveyard_actions_but_not_hand_casts_and_keeps_its_death_trigger() {
    for definition in definitions("Ashes of the Abhorrent") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let grave_spell = resource(
            &mut game,
            A,
            "Flashback probe",
            "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.\nFlashback {0}",
            Zone::Graveyard,
        );
        let hand_spell = resource(
            &mut game,
            A,
            "Hand probe",
            "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.",
            Zone::Hand,
        );
        let skeleton = resource(
            &mut game,
            A,
            "Reassembling Skeleton",
            "Mana cost: {1}{B}\nType: Creature — Skeleton Warrior\nPower/Toughness: 1/1\n{1}{B}: Return this card from your graveyard to the battlefield tapped.",
            Zone::Graveyard,
        );
        assert!(!can_cast(&game, A, grave_spell));
        assert!(can_cast(&game, A, hand_spell));
        assert!(!can_activate(&game, A, skeleton));
        game.phase_out(host);
        assert!(can_cast(&game, A, grave_spell));
        assert!(can_activate(&game, A, skeleton));
        game.phase_in(host);
        let victim = resource(
            &mut game,
            B,
            "Dying creature",
            "Type: Creature\nPower/Toughness: 1/1",
            Zone::Battlefield,
        );
        game.move_object_by_effect(victim, Zone::Graveyard).unwrap();
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 21);
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        assert!(can_cast(&game, A, grave_spell));
        assert!(can_activate(&game, A, skeleton));
    }
}

fn enter(game: &mut GameState, old: ObjectId, dm: &mut impl DecisionMaker) -> ObjectId {
    let receipt = game
        .move_object_with_etb_processing_with_dm(old, Zone::Battlefield, dm)
        .unwrap();
    assert!(!receipt.pending);
    assert!(
        receipt.programs.is_empty(),
        "this fixture must not discard added entry programs"
    );
    receipt.original.into_result().unwrap().new_id
}
struct BlueChoice;
impl DecisionMaker for BlueChoice {
    fn decide_colors(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::ColorsContext,
    ) -> Vec<ironsmith::color::Color> {
        vec![ironsmith::color::Color::Blue; context.count as usize]
    }
}
#[test]
fn iona_uses_the_actual_entry_color_choice_and_current_controller() {
    for definition in definitions("Iona, Shield of Emeria") {
        let mut game = game();
        let old = game.create_object_from_definition(&definition, A, Zone::Hand);
        let host = enter(&mut game, old, &mut BlueChoice);
        assert_eq!(game.chosen_color(host), Some(ironsmith::color::Color::Blue));
        let own_blue = resource(
            &mut game,
            A,
            "Own blue",
            "Mana cost: {U}\nType: Sorcery\nYou gain 1 life.",
            Zone::Hand,
        );
        let blue = resource(
            &mut game,
            B,
            "Blue",
            "Mana cost: {U}\nType: Sorcery\nYou gain 1 life.",
            Zone::Hand,
        );
        let green = resource(
            &mut game,
            B,
            "Green",
            "Mana cost: {G}\nType: Sorcery\nYou gain 1 life.",
            Zone::Hand,
        );
        assert!(can_cast(&game, A, own_blue));
        turn(&mut game, B);
        assert!(!can_cast(&game, B, blue));
        assert!(can_cast(&game, B, green));
        game.phase_out(host);
        assert!(can_cast(&game, B, blue));
        game.phase_in(host);
        game.set_current_controller(host, B).unwrap();
        assert!(can_cast(&game, B, blue));
        turn(&mut game, A);
        assert!(!can_cast(&game, A, own_blue));
    }
}
#[test]
fn llawan_returns_blue_opposing_creatures_and_intersects_color_and_type_for_casts() {
    for definition in definitions("Llawan, Cephalid Empress") {
        let mut game = game();
        let blue_permanent = resource(
            &mut game,
            B,
            "Blue creature",
            "Mana cost: {U}\nType: Creature\nPower/Toughness: 1/1",
            Zone::Battlefield,
        );
        let green_permanent = resource(
            &mut game,
            B,
            "Green creature",
            "Mana cost: {G}\nType: Creature\nPower/Toughness: 1/1",
            Zone::Battlefield,
        );
        let old = game.create_object_from_definition(&definition, A, Zone::Hand);
        let host = enter(&mut game, old, &mut SelectFirstDecisionMaker);
        settle(&mut game);
        assert!(game.object(blue_permanent).is_none());
        assert!(game.object(green_permanent).is_some());
        turn(&mut game, B);
        let blue = game.player(B).unwrap().hand[0];
        let blue_noncreature = resource(
            &mut game,
            B,
            "Blue artifact",
            "Mana cost: {U}\nType: Artifact",
            Zone::Hand,
        );
        let green = resource(
            &mut game,
            B,
            "Green card",
            "Mana cost: {G}\nType: Creature\nPower/Toughness: 1/1",
            Zone::Hand,
        );
        assert!(!can_cast(&game, B, blue));
        assert!(can_cast(&game, B, blue_noncreature));
        assert!(can_cast(&game, B, green));
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        assert!(can_cast(&game, B, blue));
    }
}
#[test]
fn linked_exile_bans_follow_only_this_sources_current_exiled_card_and_current_controller() {
    for name in ["Exclusion Ritual", "Ixalan's Binding"] {
        for definition in definitions(name) {
            let mut game = game();
            let victim = resource(
                &mut game,
                B,
                "Linked name",
                "Mana cost: {0}\nType: Artifact",
                Zone::Battlefield,
            );
            let host_old = game.create_object_from_definition(&definition, A, Zone::Hand);
            let host = enter(&mut game, host_old, &mut SelectFirstDecisionMaker);
            let mut queue = TriggerQueue::new();
            let mut dm = TargetChoice(Target::Object(victim));
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
                .unwrap();
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(game.object(victim).is_none());
            let linked = game.get_exiled_with_source_links(host)[0];
            let own = resource(
                &mut game,
                A,
                "Linked name",
                "Mana cost: {0}\nType: Artifact",
                Zone::Hand,
            );
            let other = resource(
                &mut game,
                B,
                "Linked name",
                "Mana cost: {0}\nType: Artifact",
                Zone::Hand,
            );
            assert_eq!(can_cast(&game, A, own), name == "Ixalan's Binding");
            turn(&mut game, B);
            assert!(!can_cast(&game, B, other));
            game.phase_out(host);
            assert!(can_cast(&game, B, other));
            game.phase_in(host);
            if name == "Ixalan's Binding" {
                game.set_current_controller(host, B).unwrap();
                assert!(can_cast(&game, B, other));
                turn(&mut game, A);
                assert!(!can_cast(&game, A, own));
                game.set_current_controller(host, A).unwrap();
                turn(&mut game, B);
            }
            game.move_object_by_effect(linked, Zone::Graveyard).unwrap();
            assert!(
                can_cast(&game, B, other),
                "a departed exiled incarnation no longer supplies a name"
            );
        }
    }
}
#[test]
fn immortalsun_blocks_only_loyalty_and_keeps_draw_cost_and_anthem_bodies() {
    for definition in definitions("The Immortal Sun") {
        let mut game = game();
        library(&mut game, A, 4);
        let walker = resource(
            &mut game,
            A,
            "Two-ability walker",
            "Mana cost: {0}\nType: Planeswalker — Jace\nLoyalty: 5\n+1: You gain 1 life.\n{0}: You gain 2 life.",
            Zone::Battlefield,
        );
        let count = |game: &GameState| {
            compute_legal_actions(game, A).unwrap().iter().filter(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == walker)).count()
        };
        assert_eq!(count(&game), 2);
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(
            count(&game),
            1,
            "nonloyalty ability of the same planeswalker stays usable"
        );
        let creature = resource(
            &mut game,
            A,
            "Anthem probe",
            "Type: Creature\nPower/Toughness: 2/2",
            Zone::Battlefield,
        );
        assert_eq!(game.calculated_power(creature), Some(3));
        assert_eq!(game.calculated_toughness(creature), Some(3));
        let spell = resource(
            &mut game,
            A,
            "Reduced artifact",
            "Mana cost: {1}\nType: Artifact",
            Zone::Hand,
        );
        game.player_mut(A).unwrap().mana_pool.empty();
        assert!(can_cast(&game, A, spell));
        // The starting player skips their ordinary draw on the first turn.
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        let mut runner = ironsmith::turn_runner::TurnRunner::from_state_for_sync(
            ironsmith::turn_runner::TurnState::Draw,
        );
        let mut queue = TriggerQueue::new();
        runner.advance(&mut game, &mut queue).unwrap();
        put_triggers_on_stack(&mut game, &mut queue).unwrap();
        settle(&mut game);
        assert_eq!(
            game.player(A).unwrap().hand.len(),
            3,
            "one existing spell plus the ordinary and additional draw"
        );
        turn(&mut game, A);
        game.phase_out(host);
        assert_eq!(count(&game), 2);
        assert!(!can_cast(&game, A, spell));
        assert_eq!(game.calculated_power(creature), Some(2));
    }
}
#[test]
fn tomik_blocks_opponents_graveyard_land_plays_without_changing_permissions_or_target_roles() {
    for definition in definitions("Tomik, Distinguished Advokist") {
        let mut game = game();
        resource(
            &mut game,
            A,
            "Own graveyard permission",
            "Type: Artifact\nYou may play lands from your graveyard.",
            Zone::Battlefield,
        );
        resource(
            &mut game,
            B,
            "Other graveyard permission",
            "Type: Artifact\nYou may play lands from your graveyard.",
            Zone::Battlefield,
        );
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = land(&mut game, A, "Own grave land", Zone::Graveyard);
        let other = land(&mut game, B, "Other grave land", Zone::Graveyard);
        let hand = land(&mut game, B, "Other hand land", Zone::Hand);
        assert!(can_land(&game, A, own));
        turn(&mut game, B);
        assert!(!can_land(&game, B, other));
        assert!(can_land(&game, B, hand));
        let opponent_spell = resource(&mut game, B, "Opponent spell", "Type: Instant", Zone::Stack);
        let own_spell = resource(&mut game, A, "Own spell", "Type: Instant", Zone::Stack);
        game.refresh_continuous_state().unwrap();
        assert!(!ironsmith::targeting::can_target_object(&game, own, opponent_spell, B).is_legal());
        assert!(ironsmith::targeting::can_target_object(&game, other, own_spell, A).is_legal());
        game.phase_out(host);
        assert!(can_land(&game, B, other));
        game.phase_in(host);
        game.set_current_controller(host, B).unwrap();
        assert!(can_land(&game, B, other));
        turn(&mut game, A);
        assert!(!can_land(&game, A, own));
    }
}
#[test]
fn territorial_dispute_bans_everyones_lands_and_its_unpaid_upkeep_removes_the_rule() {
    for definition in definitions("Territorial Dispute") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = land(&mut game, A, "Own land", Zone::Hand);
        let other = land(&mut game, B, "Other land", Zone::Hand);
        assert!(!can_land(&game, A, own));
        turn(&mut game, B);
        assert!(!can_land(&game, B, other));
        turn(&mut game, A);
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::generate_and_queue_step_triggers(&mut game, &mut queue);
        put_triggers_on_stack(&mut game, &mut queue).unwrap();
        settle(&mut game);
        assert!(game.object(host).is_none());
        turn(&mut game, A);
        assert!(can_land(&game, A, own));
        turn(&mut game, B);
        assert!(can_land(&game, B, other));
    }
}
#[test]
fn graveyard_activation_prohibition_also_reaches_mana_ability_sources() {
    for definition in definitions("Ashes of the Abhorrent") {
        let mut game = game();
        let mut mana = compile_to_runtime_definition(
            "Graveyard mana probe",
            "Type: Artifact\n{0}: Add {B}.",
            false,
        )
        .unwrap();
        for ability in &mut mana.abilities {
            if matches!(ability.kind, AbilityKind::Activated(_)) {
                ability.functional_zones = vec![Zone::Graveyard];
            }
        }
        let source = game.create_object_from_definition(&mana, A, Zone::Graveyard);
        let index = mana
            .abilities
            .iter()
            .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
            .unwrap();
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: source,
            ability_index: index,
        };
        game.refresh_continuous_state().unwrap();
        assert!(can_perform_check(&action, &game, A).is_ok());
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert!(can_perform_check(&action, &game, A).is_err());
        game.phase_out(host);
        game.refresh_continuous_state().unwrap();
        assert!(can_perform_check(&action, &game, A).is_ok());
    }
}
#[test]
fn land_restriction_checks_the_chosen_mdfc_face_name() {
    use ironsmith::card::LinkedFaceLayout;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, CardType};
    for definition in definitions("Cornered Market") {
        for front_type in [CardType::Land, CardType::Sorcery] {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let front_id = CardId::new();
            let back_id = CardId::new();
            let front = CardDefinitionBuilder::new(front_id, "Unrelated front")
                .card_types(vec![front_type])
                .other_face(back_id)
                .other_face_name("Restricted back")
                .linked_face_layout(LinkedFaceLayout::TransformLike)
                .build();
            let back = CardDefinitionBuilder::new(back_id, "Restricted back")
                .card_types(vec![CardType::Land])
                .other_face(front_id)
                .other_face_name("Unrelated front")
                .linked_face_layout(LinkedFaceLayout::TransformLike)
                .build();
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);
            let candidate = game.create_object_from_definition(&front, A, Zone::Hand);
            let same = land(&mut game, B, "Restricted back", Zone::Battlefield);
            let action = if front_type == CardType::Land {
                assert!(
                    can_perform_check(&SpecialAction::PlayLand { card_id: candidate }, &game, A)
                        .is_ok()
                );
                SpecialAction::PlayLandBackFace { card_id: candidate }
            } else {
                SpecialAction::PlayLand { card_id: candidate }
            };
            assert_eq!(
                can_perform_check(&action, &game, A),
                Err(ActionError::LandPlayProhibited)
            );
            game.move_object_by_effect(same, Zone::Graveyard).unwrap();
            assert!(can_perform_check(&action, &game, A).is_ok());
            assert_eq!(
                game.object(candidate).unwrap().name.as_ref(),
                "Unrelated front",
                "legality queries never commit the alternate face"
            );
        }
    }
}
