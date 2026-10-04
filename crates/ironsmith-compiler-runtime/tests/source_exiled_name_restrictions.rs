//! UNVALIDATED exact-card regressions for source-linked exiled-name restrictions.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, TargetsContext};
use ironsmith::events::combat::CreatureBlockedEvent;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
// Frozen IDs: Circu c87d547b-00a4-4fdd-ade4-f3f032e5ea3b; Godsend dd91f4f2-ea46-48c5-8a80-0dc4ae5b79c4.
const CIRCU: &str = "Mana cost: {2}{U}{B}\nType: Legendary Creature — Human Wizard\nPower/Toughness: 2/3\nWhenever you cast a blue spell, exile the top card of target player's library.\nWhenever you cast a black spell, exile the top card of target player's library.\nYour opponents can't cast spells with the same name as a card exiled with Circu.";
const GODSEND: &str = "Mana cost: {1}{W}{W}\nType: Legendary Artifact — Equipment\nEquipped creature gets +3/+3.\nWhenever equipped creature blocks or becomes blocked by one or more creatures, you may exile one of those creatures.\nYour opponents can't cast spells with the same name as a card exiled with Godsend.\nEquip {3}";
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let transported = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, transported);
    [direct, materialize_artifact(&transported).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, kind: CardType) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), name)
            .card_types(vec![kind])
            .mana_cost(ManaCost::new())
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        owner,
        zone,
    )
}
struct Choices {
    target: Target,
    object: Option<ObjectId>,
    accept: bool,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        assert!(
            ctx.requirements
                .iter()
                .all(|requirement| requirement.legal_targets.contains(&self.target))
        );
        vec![self.target]
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.object.filter(|id| {
            ctx.candidates
                .iter()
                .any(|candidate| candidate.id == *id && candidate.legal)
        }) {
            return vec![id];
        }
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.accept
    }
}
fn announce(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..24 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("unfinished announcement: {progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn castable(game: &mut GameState, player: PlayerId, spell: ObjectId) -> bool {
    game.turn.priority_player = Some(player);
    game.refresh_continuous_state().unwrap();
    compute_legal_actions(game, player).unwrap().iter().any(
        |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell),
    )
}
#[test]
fn circu_actual_cast_trigger_links_the_target_library_card_and_restricts_only_its_current_opponents()
 {
    for definition in definitions("Circu, Dimir Lobotomist", CIRCU) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let library = card(&mut game, B, Zone::Library, "Named ban", CardType::Instant);
        let stable = game.object(library).unwrap().stable_id;
        let catalyst = compile_to_runtime_definition(
            "Blue catalyst",
            "Mana cost: {U}\nType: Instant\nYou gain 1 life.",
            false,
        )
        .unwrap();
        let catalyst = game.create_object_from_definition(&catalyst, A, Zone::Hand);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == catalyst)).unwrap();
        let mut dm = Choices {
            target: Target::Player(B),
            object: None,
            accept: true,
        };
        announce(&mut game, action, &mut dm);
        assert_eq!(
            game.stack.len(),
            2,
            "one blue cast produces one Circu trigger above the spell"
        );
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        assert!(game.get_exiled_with_source_links(source).contains(&exiled));
        let alice = card(&mut game, A, Zone::Hand, "Named ban", CardType::Instant);
        let bob = card(&mut game, B, Zone::Hand, "Named ban", CardType::Instant);
        let cara = card(&mut game, C, Zone::Hand, "Named ban", CardType::Instant);
        let other = card(
            &mut game,
            B,
            Zone::Hand,
            "Different name",
            CardType::Instant,
        );
        assert!(castable(&mut game, A, alice));
        assert!(!castable(&mut game, B, bob));
        assert!(!castable(&mut game, C, cara));
        assert!(castable(&mut game, B, other));
        game.set_current_controller(source, C).unwrap();
        assert!(!castable(&mut game, A, alice));
        assert!(castable(&mut game, C, cara));
        game.set_current_controller(source, A).unwrap();
        let moved = game.move_object_by_effect(exiled, Zone::Hand).unwrap();
        let returned = game.move_object_by_effect(moved, Zone::Exile).unwrap();
        assert_ne!(returned, exiled);
        assert!(
            castable(&mut game, B, bob),
            "an unlinked later exile incarnation is not the named card"
        );
    }
}
#[test]
fn godsend_actual_equip_and_block_trigger_use_the_equipment_link_even_if_the_source_later_leaves() {
    for definition in definitions("Godsend", GODSEND) {
        for leave_before_resolution in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let equipped = card(
                &mut game,
                A,
                Zone::Battlefield,
                "Equipped blocker",
                CardType::Creature,
            );
            let attacker = card(
                &mut game,
                B,
                Zone::Battlefield,
                "Ban this attacker",
                CardType::Creature,
            );
            let attacker_stable = game.object(attacker).unwrap().stable_id;
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 3);
            let equip = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
            let mut dm = Choices {
                target: Target::Object(equipped),
                object: Some(attacker),
                accept: true,
            };
            announce(&mut game, equip, &mut dm);
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.object(source).unwrap().attached_to, Some(ironsmith::object::AttachmentTarget::Object(equipped)));
            assert_eq!(game.current_power(equipped), Some(5));
            let event = TriggerEvent::new(CreatureBlockedEvent::new(equipped, attacker), Default::default());
            let mut queue = TriggerQueue::new();
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            assert_eq!(game.stack.len(), 1);
            let departed = leave_before_resolution
                .then(|| game.move_object_by_effect(source, Zone::Graveyard).unwrap());
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            let exiled = game.find_object_by_stable_id(attacker_stable).unwrap();
            assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
            assert!(game.get_exiled_with_source_links(source).contains(&exiled));
            let spell = card(
                &mut game,
                B,
                Zone::Hand,
                "Ban this attacker",
                CardType::Instant,
            );
            assert_eq!(castable(&mut game, B, spell), leave_before_resolution);
            let departed = departed
                .unwrap_or_else(|| game.move_object_by_effect(source, Zone::Graveyard).unwrap());
            assert!(castable(&mut game, B, spell));
            let new_source = game
                .move_object_by_effect(departed, Zone::Battlefield)
                .unwrap();
            assert!(game.get_exiled_with_source_links(new_source).is_empty());
            assert!(
                castable(&mut game, B, spell),
                "a new equipment incarnation does not inherit old exiles"
            );
        }
    }
}

#[test]
fn linked_name_filters_share_split_names_but_never_match_nameless_cards() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::target::ChooseSpec;
    for (name, text) in [("Circu, Dimir Lobotomist", CIRCU), ("Godsend", GODSEND)] {
        for definition in definitions(name, text) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let split = card(
                &mut game,
                B,
                Zone::Graveyard,
                "First half // Second half",
                CardType::Instant,
            );
            let nameless = card(&mut game, B, Zone::Graveyard, "", CardType::Instant);
            for id in [split, nameless] {
                let mut ctx = EffectContext::new_default(source, A);
                execute_effect(
                    &mut game,
                    &Effect::exile(ChooseSpec::SpecificObject(id)),
                    &mut ctx,
                )
                .unwrap();
            }
            let matching_half = card(&mut game, B, Zone::Hand, "Second half", CardType::Instant);
            let no_name = card(&mut game, B, Zone::Hand, "", CardType::Instant);
            assert!(!castable(&mut game, B, matching_half));
            assert!(
                castable(&mut game, B, no_name),
                "nameless objects do not share a name, even with another nameless object"
            );
        }
    }
}
