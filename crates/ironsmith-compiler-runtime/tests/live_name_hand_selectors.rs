//! UNVALIDATED exact-card tests for live comparison-set name selectors.
use ironsmith::card::{CardBuilder, LinkedFaceLayout, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{SelectObjectsContext, TargetsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::filter::{ObjectCharacteristic, ObjectCharacteristicRelation};
use ironsmith::target::{
    ObjectFilter, ObjectRef, PlayerFilter,
};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Supertype, Target, Zone,
};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
// Frozen identities: Key 76c036df-44f5-4021-b7fc-656147351341; Hint 4db59824-6bcb-4707-977b-0ff699d8662b.
const KEY: &str = "Mana cost: {1}\nType: Artifact\n{2}, {T}: Target creature can't be blocked this turn.\n{1}, {T}, Discard a legendary card with the same name as a legendary permanent you control: Draw two cards.";
const HINT: &str = "Mana cost: {2}{B}\nType: Sorcery\nTarget player reveals their hand. That player discards all nonland cards with the same name as another card in their hand.";
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 8);
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Black, 1);
    game
}
fn card(
    game: &mut GameState,
    owner: PlayerId,
    zone: Zone,
    name: &str,
    kind: CardType,
    legendary: bool,
) -> ObjectId {
    let mut builder = CardBuilder::new(CardId::new(), name)
        .card_types(vec![kind])
        .power_toughness(PowerToughness::fixed(2, 2));
    if legendary {
        builder = builder.supertypes(vec![Supertype::Legendary]);
    }
    game.create_object_from_card(&builder.build(), owner, zone)
}
struct Choices {
    target: Target,
    discard: Option<ObjectId>,
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
        if let Some(id) = self.discard {
            assert!(
                ctx.candidates
                    .iter()
                    .any(|candidate| candidate.id == id && candidate.legal)
            );
            return vec![id];
        }
        SelectFirstDecisionMaker.decide_objects(game, ctx)
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
            panic!("unfinished payment: {progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    assert_eq!(game.stack.len(), 1);
}
fn key_action(game: &GameState, source: ObjectId, ability_index: usize) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, ability_index: index } if *id == source && *index == ability_index))
}
#[test]
fn key_uses_live_controlled_legendary_names_and_pays_the_exact_hand_card() {
    for definition in definitions("Key to the Side-Door", KEY) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let hand = card(
            &mut game,
            A,
            Zone::Hand,
            "Shared legend",
            CardType::Creature,
            true,
        );
        let stable = game.object(hand).unwrap().stable_id;
        let donor = card(
            &mut game,
            B,
            Zone::Battlefield,
            "Shared legend",
            CardType::Creature,
            true,
        );
        let hand_before = game.player(A).unwrap().hand.clone();
        let mana_before = game.player(A).unwrap().mana_pool.clone();
        assert!(
            key_action(&game, source, 1).is_none(),
            "an opponent-controlled legend is not a donor"
        );
        assert_eq!(game.player(A).unwrap().hand, hand_before);
        assert_eq!(game.player(A).unwrap().mana_pool, mana_before);
        game.set_current_controller(donor, A).unwrap();
        assert!(
            key_action(&game, source, 1).is_some(),
            "ownership does not matter for a controlled donor"
        );
        let departed = game.move_object_by_effect(donor, Zone::Graveyard).unwrap();
        assert!(
            key_action(&game, source, 1).is_none(),
            "the reference is a live battlefield set"
        );
        let donor = game
            .move_object_by_effect(departed, Zone::Battlefield)
            .unwrap();
        game.set_current_controller(donor, A).unwrap();
        for name in ["First draw", "Second draw"] {
            card(&mut game, A, Zone::Library, name, CardType::Land, false);
        }
        let action = key_action(&game, source, 1).unwrap();
        announce(
            &mut game,
            action,
            &mut Choices {
                target: Target::Player(A),
                discard: Some(hand),
            },
        );
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Graveyard
        );
        assert!(game.player(A).unwrap().hand.is_empty());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 8);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
    }
}
#[test]
fn key_preserves_its_unblockable_ability_and_rejects_wrong_legendary_or_nameless_matches() {
    for definition in definitions("Key to the Side-Door", KEY) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = card(
            &mut game,
            B,
            Zone::Battlefield,
            "Target",
            CardType::Creature,
            false,
        );
        let action = key_action(&game, source, 0).unwrap();
        announce(
            &mut game,
            action,
            &mut Choices {
                target: Target::Object(target),
                discard: None,
            },
        );
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(!game.can_be_blocked(target));
        game.next_turn();
        assert!(game.can_be_blocked(target));
        game.turn.priority_player = Some(A);
        game.untap(source);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 4);
        card(
            &mut game,
            A,
            Zone::Hand,
            "Not legendary",
            CardType::Creature,
            false,
        );
        card(
            &mut game,
            A,
            Zone::Battlefield,
            "Not legendary",
            CardType::Creature,
            true,
        );
        card(
            &mut game,
            A,
            Zone::Hand,
            "Nonlegendary donor",
            CardType::Creature,
            true,
        );
        card(
            &mut game,
            A,
            Zone::Battlefield,
            "Nonlegendary donor",
            CardType::Creature,
            false,
        );
        card(&mut game, A, Zone::Hand, "", CardType::Creature, true);
        card(
            &mut game,
            A,
            Zone::Battlefield,
            "",
            CardType::Creature,
            true,
        );
        assert!(key_action(&game, source, 1).is_none());
        let split = card(
            &mut game,
            A,
            Zone::Hand,
            "First half",
            CardType::Instant,
            true,
        );
        let object = game.object_mut(split).unwrap();
        object.linked_face_layout = LinkedFaceLayout::Split;
        object.other_face_name = Some("Second half".into());
        card(
            &mut game,
            A,
            Zone::Battlefield,
            "Second half",
            CardType::Artifact,
            true,
        );
        assert!(
            key_action(&game, source, 1).is_some(),
            "either split name qualifies"
        );
    }
}
#[test]
fn hint_reveals_the_target_hand_and_discards_the_complete_frozen_nonland_duplicate_set() {
    for definition in definitions("Hint of Insanity", HINT) {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let pair1 = card(&mut game, B, Zone::Hand, "Pair", CardType::Instant, false);
        let pair2 = card(&mut game, B, Zone::Hand, "Pair", CardType::Creature, false);
        let shares_land = card(
            &mut game,
            B,
            Zone::Hand,
            "Land match",
            CardType::Artifact,
            false,
        );
        let land = card(
            &mut game,
            B,
            Zone::Hand,
            "Land match",
            CardType::Land,
            false,
        );
        let singleton = card(
            &mut game,
            B,
            Zone::Hand,
            "Other players only",
            CardType::Instant,
            false,
        );
        let alice = card(
            &mut game,
            A,
            Zone::Hand,
            "Other players only",
            CardType::Instant,
            false,
        );
        let cara = card(
            &mut game,
            C,
            Zone::Hand,
            "Other players only",
            CardType::Instant,
            false,
        );
        let blank1 = card(&mut game, B, Zone::Hand, "", CardType::Instant, false);
        let blank2 = card(&mut game, B, Zone::Hand, "", CardType::Instant, false);
        let split = card(&mut game, B, Zone::Hand, "Left", CardType::Instant, false);
        let object = game.object_mut(split).unwrap();
        object.linked_face_layout = LinkedFaceLayout::Split;
        object.other_face_name = Some("Right".into());
        let half = card(&mut game, B, Zone::Hand, "Right", CardType::Instant, false);
        let discarded =
            [pair1, pair2, shares_land, split, half].map(|id| game.object(id).unwrap().stable_id);
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).unwrap();
        let mut dm = Choices {
            target: Target::Player(B),
            discard: None,
        };
        announce(&mut game, action, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        for stable in discarded {
            assert_eq!(
                game.object(game.find_object_by_stable_id(stable).unwrap())
                    .unwrap()
                    .zone,
                Zone::Graveyard
            );
        }
        for id in [land, singleton, blank1, blank2, alice, cara] {
            assert_eq!(game.object(id).unwrap().zone, Zone::Hand);
        }
        assert_eq!(game.player(B).unwrap().hand.len(), 4);
    }
}
#[test]
fn older_characteristic_relation_payloads_keep_inclusive_semantics() {
    let relation = ObjectCharacteristicRelation::shares(
        vec![ObjectCharacteristic::Color],
        ObjectFilter::default(),
    );
    let mut json = serde_json::to_value(&relation).unwrap();
    json.as_object_mut().unwrap().remove("exclude_candidate");
    let restored: ObjectCharacteristicRelation = serde_json::from_value(json).unwrap();
    assert_eq!(relation, restored);
    assert!(!restored.exclude_candidate);
    let inner = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(PlayerFilter::OwnerOf(ObjectRef::FilterCandidate)),
        ..Default::default()
    };
    let relation = ObjectCharacteristicRelation::shares(vec![ObjectCharacteristic::Name], inner)
        .excluding_candidate();
    assert_eq!(
        relation.comparison_description(),
        "another card in their hand"
    );
}
