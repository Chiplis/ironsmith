#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::CardDefinition;
use crate::decision::DecisionMaker;
use crate::effect::Effect;
use crate::effects::{
    ExecutionContext, MayEffect, ReturnFromGraveyardToHandEffect, execute_effect,
};
use crate::events::cause::EventCause;
use crate::events::{DamageEvent, DamageTarget};

const NEYAM_ORACLE: &str = "Rogue Trader — Whenever Neyam Shai Murad deals combat damage to a player, you may have that player return target permanent card from their graveyard to their hand. If you do, that player chooses a permanent card in your graveyard, then you put it onto the battlefield under your control.";

fn rogue_trader(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Neyam should have its Rogue Trader triggered ability")
}

fn nested_return_to_hand(effect: &Effect) -> Option<&ReturnFromGraveyardToHandEffect> {
    if let Some(returned) = effect.downcast_ref::<ReturnFromGraveyardToHandEffect>() {
        return Some(returned);
    }
    if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
        return nested_return_to_hand(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
        return nested_return_to_hand(&with_id.effect);
    }
    if let Some(may) = effect.downcast_ref::<MayEffect>() {
        return may.effects.iter().find_map(nested_return_to_hand);
    }
    None
}

#[test]
fn neyam_preserves_causative_actor_chooser_result_identity_and_exact_text() {
    let definition = parse_oracle_card_definition("Neyam Shai Murad");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        NEYAM_ORACLE
    );

    let triggered = rogue_trader(&definition);
    let [target] = triggered.choices.as_slice() else {
        panic!("Rogue Trader should declare exactly one target: {triggered:#?}");
    };
    let ChooseSpec::Object(target_filter) = target.base() else {
        panic!("the declared target should be a permanent card: {target:#?}");
    };
    assert_eq!(target_filter.zone, Some(Zone::Graveyard));
    assert_eq!(target_filter.owner, Some(PlayerFilter::DamagedPlayer));

    let [return_segment, choice_segment] = triggered.effects.segments.as_slice() else {
        panic!("Rogue Trader should retain its may/if-you-do boundary: {triggered:#?}");
    };
    let [with_id_effect] = return_segment.default_effects.as_slice() else {
        panic!("the optional return should have one result-tracked effect");
    };
    let with_id = with_id_effect
        .downcast_ref::<WithIdEffect>()
        .expect("the optional return should publish whether it happened");
    let may = with_id
        .effect
        .downcast_ref::<MayEffect>()
        .expect("the first instruction should remain optional");
    assert_eq!(may.decider, Some(PlayerFilter::You));
    let returned = may
        .effects
        .iter()
        .find_map(nested_return_to_hand)
        .expect("the accepted instruction should return the targeted card");
    assert_eq!(
        returned.actor_surface,
        Some(PlayerFilter::DamagedPlayer),
        "the damaged player, not the ability controller, performs the causative return"
    );
    assert_eq!(
        returned.graveyard_player_surface,
        Some(PlayerFilter::DamagedPlayer)
    );
    assert_eq!(
        returned.destination_player_surface,
        Some(PlayerFilter::DamagedPlayer)
    );

    let [if_effect] = choice_segment.default_effects.as_slice() else {
        panic!("the successful branch should have one conditional");
    };
    let branch = if_effect
        .downcast_ref::<IfEffect>()
        .expect("the second sentence should depend on the return happening");
    assert_eq!(branch.condition, with_id.id);
    let [choose_effect, move_effect] = branch.then.as_slice() else {
        panic!("the successful branch should choose, then move, one card: {branch:#?}");
    };
    let choose = choose_effect
        .downcast_ref::<ChooseObjectsEffect>()
        .expect("that player should make an explicit graveyard choice");
    let PlayerFilter::AliasedOwnerOf(ObjectRef::Tagged(returned_tag)) = &choose.chooser else {
        panic!("the chooser should be the owner of the exact returned card: {choose:#?}");
    };
    assert_eq!(choose.filter.zone, Some(Zone::Graveyard));
    assert_eq!(choose.filter.owner, Some(PlayerFilter::You));
    assert_eq!(choose.count, crate::effect::ChoiceCount::exactly(1));

    let move_to_zone = move_effect
        .downcast_ref::<MoveToZoneEffect>()
        .expect("the chosen card should move to the battlefield");
    assert!(
        matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag == &choose.tag
        ),
        "the move must consume the exact chooser result: {move_to_zone:#?}"
    );
    assert_eq!(move_to_zone.actor_surface, Some(PlayerFilter::You));
    assert_eq!(
        move_to_zone.battlefield_controller,
        crate::effects::BattlefieldController::You
    );
    assert!(
        may.effects.iter().any(|effect| {
            effect
                .downcast_ref::<TaggedEffect>()
                .is_some_and(|tagged| &tagged.tag == returned_tag)
        }),
        "the later chooser alias must point at the return's actual result tag"
    );
}

#[derive(Debug)]
struct NeyamDecisionMaker {
    chosen: ObjectId,
    boolean_players: Vec<PlayerId>,
    object_choice_players: Vec<PlayerId>,
    observed_candidates: Vec<(ObjectId, bool)>,
}

impl DecisionMaker for NeyamDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.boolean_players.push(ctx.player);
        true
    }

    fn decide_objects(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.object_choice_players.push(ctx.player);
        self.observed_candidates = ctx
            .candidates
            .iter()
            .map(|candidate| (candidate.id, candidate.legal))
            .collect();
        assert!(
            self.observed_candidates
                .iter()
                .any(|(candidate, legal)| *candidate == self.chosen && *legal),
            "the requested permanent should be a legal graveyard choice: {ctx:#?}"
        );
        vec![self.chosen]
    }
}

fn artifact(raw_id: u32, name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(raw_id), name)
        .card_types(vec![CardType::Artifact])
        .build()
}

fn current_zone(game: &crate::game_state::GameState, stable_id: StableId) -> Zone {
    let current = game
        .find_object_by_stable_id(stable_id)
        .expect("the card should retain stable identity");
    game.object(current)
        .expect("the current object should exist")
        .zone
}

#[test]
fn neyam_runtime_uses_the_damaged_player_and_moves_only_their_exact_choice() {
    let definition = parse_oracle_card_definition("Neyam Shai Murad");
    let triggered = rogue_trader(&definition);
    let [return_segment, choice_segment] = triggered.effects.segments.as_slice() else {
        panic!("Rogue Trader should retain two resolution segments");
    };

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let returned = game.create_object_from_definition(
        &artifact(98_001, "Bob's Returned Permanent"),
        bob,
        Zone::Graveyard,
    );
    let chosen = game.create_object_from_definition(
        &artifact(98_002, "Alice's Chosen Permanent"),
        alice,
        Zone::Graveyard,
    );
    let unchosen = game.create_object_from_definition(
        &artifact(98_003, "Alice's Other Permanent"),
        alice,
        Zone::Graveyard,
    );
    let wrong_owner = game.create_object_from_definition(
        &artifact(98_004, "Bob's Wrong-Graveyard Permanent"),
        bob,
        Zone::Graveyard,
    );
    let returned_stable = game.object(returned).expect("returned card").stable_id;
    let chosen_stable = game.object(chosen).expect("chosen card").stable_id;
    let unchosen_stable = game.object(unchosen).expect("unchosen card").stable_id;
    let wrong_owner_stable = game
        .object(wrong_owner)
        .expect("wrong-owner card")
        .stable_id;

    let event = crate::triggers::TriggerEvent::new_with_provenance(
        DamageEvent::with_cause(
            source,
            DamageTarget::Player(bob),
            3,
            true,
            EventCause::combat_damage(source),
        ),
        crate::ProvNodeId::default(),
    );
    let mut decisions = NeyamDecisionMaker {
        chosen,
        boolean_players: Vec::new(),
        object_choice_players: Vec::new(),
        observed_candidates: Vec::new(),
    };
    let mut ctx = ExecutionContext::new(source, alice, &mut decisions)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(returned)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: triggered.choices[0].clone(),
            range: 0..1,
        }])
        .with_triggering_event(event);
    ctx.snapshot_targets(&game);

    for effect in &return_segment.default_effects {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("the accepted causative return should resolve");
    }
    for effect in &choice_segment.default_effects {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("the linked chooser/result branch should resolve");
    }
    drop(ctx);

    assert_eq!(
        decisions.boolean_players,
        [alice],
        "Alice decides whether to make the offer"
    );
    assert_eq!(
        decisions.object_choice_players,
        [bob],
        "the player damaged by Neyam must make the later graveyard choice"
    );
    assert!(
        decisions
            .observed_candidates
            .iter()
            .find(|(candidate, _)| *candidate == wrong_owner)
            .is_none_or(|(_, legal)| !legal),
        "Bob must not be able to choose a permanent from Bob's graveyard"
    );
    assert_eq!(current_zone(&game, returned_stable), Zone::Hand);
    assert_eq!(current_zone(&game, chosen_stable), Zone::Battlefield);
    assert_eq!(current_zone(&game, unchosen_stable), Zone::Graveyard);
    assert_eq!(current_zone(&game, wrong_owner_stable), Zone::Graveyard);
    let chosen_current = game
        .find_object_by_stable_id(chosen_stable)
        .expect("chosen card should remain findable");
    assert_eq!(
        game.controller_of_id(chosen_current),
        Some(alice),
        "only Bob's exact choice should enter under Alice's control"
    );
}
