use super::*;

const EXPECTED: &str = "Starting with you, each player chooses up to five permanents they control. All permanents other than this creature that weren't chosen this way phase out";

fn program(
    starting_with_controller: bool,
    phase_tag: TagKey,
    relation: TaggedOpbjectRelation,
    other: bool,
    redundant_choice_zone: bool,
) -> crate::resolution::ResolutionProgram {
    let choice_tag = TagKey::from("chosen_0");
    let mut choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::permanent_card()
            .in_zone(Zone::Battlefield)
            .controlled_by(PlayerFilter::IteratedPlayer),
        ChoiceCount::up_to(5),
        PlayerFilter::IteratedPlayer,
        choice_tag,
    );
    if redundant_choice_zone {
        choose.zone = Some(Zone::Battlefield);
    }
    let choose = Effect::new(choose);
    let mut for_players = crate::effects::ForPlayersEffect::new(PlayerFilter::Any, vec![choose]);
    for_players.starting_with_controller = starting_with_controller;

    let mut phase_filter = ObjectFilter::permanent_card()
        .in_zone(Zone::Battlefield)
        .match_tagged(phase_tag, relation);
    phase_filter.other = other;
    phase_filter.source_surface = Some(crate::target::SourceReferenceSurface::ThisPermanentType(
        "this creature".to_string(),
    ));
    phase_filter.set_prior_effect_action_surface(Some(ironsmith_core::PriorEffectAction::Chosen));
    crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::new(for_players)]),
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::new(
            crate::effects::PhaseOutEffect::all(phase_filter),
        )]),
    ])
}

#[test]
fn exact_participant_choice_complement_phase_out_renders_canonical_surface() {
    for redundant_choice_zone in [false, true] {
        let program = program(
            true,
            TagKey::from("chosen_0"),
            TaggedOpbjectRelation::IsNotTaggedObject,
            true,
            redundant_choice_zone,
        );
        assert_eq!(
            super::super::ast_render::describe_resolution_program(&program),
            EXPECTED
        );
    }
}

#[test]
fn participant_choice_phase_out_renderer_rejects_near_miss_bindings() {
    let near_misses = [
        program(
            true,
            TagKey::from("chosen_0"),
            TaggedOpbjectRelation::IsTaggedObject,
            true,
            false,
        ),
        program(
            true,
            TagKey::from("unrelated"),
            TaggedOpbjectRelation::IsNotTaggedObject,
            true,
            false,
        ),
        program(
            true,
            TagKey::from("chosen_0"),
            TaggedOpbjectRelation::IsNotTaggedObject,
            false,
            false,
        ),
        program(
            false,
            TagKey::from("chosen_0"),
            TaggedOpbjectRelation::IsNotTaggedObject,
            true,
            false,
        ),
    ];

    for near_miss in near_misses {
        assert_ne!(
            super::super::ast_render::describe_resolution_program(&near_miss),
            EXPECTED
        );
    }
}
