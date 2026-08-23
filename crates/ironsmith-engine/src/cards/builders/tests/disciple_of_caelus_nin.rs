#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const DISCIPLE_ORACLE: [&str; 2] = [
    "When this creature enters, starting with you, each player chooses up to five permanents they control. All permanents other than this creature that weren't chosen this way phase out.",
    "Permanents can't phase in.",
];

fn disciple_trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Disciple of Caelus Nin must retain its enters trigger")
}

fn simple_permanent(raw_id: u32, name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(raw_id), name)
        .card_types(vec![CardType::Artifact])
        .build()
}

fn phase_in_specific(
    game: &mut crate::game_state::GameState,
    source: ObjectId,
    controller: PlayerId,
    permanent: ObjectId,
) {
    let mut ctx = crate::effects::ExecutionContext::new_default(source, controller);
    crate::effects::PhaseInEffect::with_spec(ChooseSpec::SpecificObject(permanent))
        .execute(game, &mut ctx)
        .expect("the explicit phase-in action must resolve");
}

struct ChooseFiveOtherThanSource(ObjectId);

impl crate::decision::DecisionMaker for ChooseFiveOtherThanSource {
    fn decide_objects(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal && candidate.id != self.0)
            .map(|candidate| candidate.id)
            .take(ctx.max.unwrap_or(5))
            .collect()
    }
}

#[test]
fn disciple_renders_exact_oracle_and_retains_typed_choice_complement_rule() {
    let definition = parse_oracle_card_definition("Disciple of Caelus Nin");
    assert!(
        definition.spell_effect.is_none(),
        "the static phase-in prohibition must not become a resolving spell effect"
    );

    let triggered = disciple_trigger(&definition);
    let [choice_segment, phase_segment] = triggered.effects.segments.as_slice() else {
        panic!(
            "the two authored trigger sentences must remain distinct: {:#?}",
            triggered.effects
        );
    };
    let [player_effect] = choice_segment.default_effects.as_slice() else {
        panic!("the first sentence must contain one player iteration: {choice_segment:#?}");
    };
    let for_players = player_effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .expect("the choice must iterate over every player");
    assert_eq!(for_players.filter, PlayerFilter::Any);
    assert!(for_players.starting_with_controller);
    assert!(!for_players.stop_after_first_happened);
    let [choose_effect] = for_players.effects.as_slice() else {
        panic!("each player must make exactly one object choice: {for_players:#?}");
    };
    let choose = choose_effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("the per-player operation must remain a typed object choice");
    assert_eq!(choose.count, crate::effect::ChoiceCount::up_to(5));
    assert_eq!(choose.chooser, PlayerFilter::IteratedPlayer);
    assert_eq!(
        choose.filter,
        ObjectFilter::permanent_card()
            .in_zone(Zone::Battlefield)
            .controlled_by(PlayerFilter::IteratedPlayer)
    );
    let mut semantic_choice = choose.clone();
    semantic_choice.zone = None;
    assert_eq!(
        semantic_choice,
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::permanent_card()
                .in_zone(Zone::Battlefield)
                .controlled_by(PlayerFilter::IteratedPlayer),
            ChoiceCount::up_to(5),
            PlayerFilter::IteratedPlayer,
            choose.tag.clone(),
        ),
        "the choice must retain the exact generic participant-choice shape"
    );
    assert!(
        !choose.replace_tagged_objects,
        "each player's selection must accumulate in one shared result set"
    );

    let [phase_effect] = phase_segment.default_effects.as_slice() else {
        panic!("the second sentence must contain one phase-out action: {phase_segment:#?}");
    };
    let phase_out = phase_effect
        .downcast_ref::<crate::effects::PhaseOutEffect>()
        .expect("the complement consumer must remain typed phase out");
    let ChooseSpec::All(phase_filter) = &phase_out.spec else {
        panic!("the phase-out action must consume the full complement: {phase_out:#?}");
    };
    assert!(phase_filter.other, "Disciple itself must be excluded");
    assert_eq!(
        phase_filter.prior_effect_action_surface(),
        Some(ironsmith_core::PriorEffectAction::Chosen)
    );
    assert!(matches!(
        phase_filter.tagged_constraints.as_slice(),
        [constraint]
            if constraint.tag == choose.tag
                && constraint.relation
                    == crate::target::TaggedOpbjectRelation::IsNotTaggedObject
    ));
    let mut semantic_phase_filter = phase_filter.clone();
    semantic_phase_filter.other = false;
    semantic_phase_filter.source_surface = None;
    semantic_phase_filter.tagged_constraints.clear();
    semantic_phase_filter.set_prior_effect_action_surface(None);
    assert_eq!(
        semantic_phase_filter,
        ObjectFilter::permanent_card().in_zone(Zone::Battlefield),
        "the complement metadata must be the only difference from all battlefield permanents"
    );

    let phase_in_rule = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability
                .rule_restriction_parts()
                .map(|(restriction, _, _)| restriction),
            _ => None,
        })
        .expect("Disciple must have a generic static rule restriction");
    assert!(matches!(
        phase_in_rule,
        crate::effect::Restriction::PhaseIn(filter)
            if filter
                == &ObjectFilter::permanent_card().in_zone(Zone::Battlefield)
    ));
    assert_eq!(
        canonical_compiled_lines(&definition),
        DISCIPLE_ORACLE.map(str::to_string)
    );
}

#[test]
fn disciple_accumulates_multiplayer_choices_and_blocks_both_phase_in_paths() {
    let definition = parse_oracle_card_definition("Disciple of Caelus Nin");
    let triggered = disciple_trigger(&definition);
    let choice_tag = triggered.effects.segments[0].default_effects[0]
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .and_then(|players| {
            players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        })
        .expect("Disciple's participant choice must remain typed")
        .tag
        .clone();
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let mut permanents = Vec::new();
    for (player_index, player) in [alice, bob, charlie].into_iter().enumerate() {
        let controlled = (0..6)
            .map(|permanent_index| {
                let raw_id = 97_000 + (player_index as u32 * 10) + permanent_index;
                game.create_object_from_definition(
                    &simple_permanent(
                        raw_id,
                        &format!("P{player_index} Permanent {permanent_index}"),
                    ),
                    player,
                    Zone::Battlefield,
                )
            })
            .collect::<Vec<_>>();
        permanents.push(controlled);
    }
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let mut decisions = ChooseFiveOtherThanSource(source);
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Disciple's complete multiplayer choice procedure must resolve");
    let chosen = ctx.get_tagged_all(&choice_tag).cloned().unwrap_or_default();
    assert_eq!(
        chosen.len(),
        15,
        "the shared result tag must accumulate five choices per player: {chosen:#?}"
    );

    let mut unchosen = Vec::new();
    for controlled in &permanents {
        let phased = controlled
            .iter()
            .copied()
            .filter(|permanent| game.is_phased_out(*permanent))
            .collect::<Vec<_>>();
        assert_eq!(
            phased.len(),
            1,
            "each player must keep five of their six selectable permanents phased in"
        );
        unchosen.push(phased[0]);
    }
    assert!(
        !game.is_phased_out(source),
        "the source exclusion must keep Disciple phased in"
    );

    game.update_cant_effects();
    for permanent in &unchosen {
        phase_in_specific(&mut game, source, alice, *permanent);
        assert!(
            game.is_phased_out(*permanent),
            "the static rule must prohibit an explicit phase-in effect"
        );
    }

    for player in [alice, bob, charlie] {
        game.turn.active_player = player;
        crate::turn::execute_untap_step(&mut game);
    }
    assert!(
        unchosen
            .iter()
            .all(|permanent| game.is_phased_out(*permanent)),
        "the static rule must also prohibit turn-based phasing"
    );

    let moved_source = game
        .move_object(
            source,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
        )
        .expect("Disciple must leave the battlefield");
    game.update_cant_effects();
    for permanent in &unchosen {
        phase_in_specific(&mut game, moved_source, alice, *permanent);
        assert!(
            !game.is_phased_out(*permanent),
            "the prohibition must end when its source leaves"
        );
    }
}
