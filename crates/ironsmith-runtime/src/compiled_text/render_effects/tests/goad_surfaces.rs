use super::*;

fn all_creatures_controlled_by(player: PlayerFilter) -> ChooseSpec {
    let mut creatures = ObjectFilter::creature();
    creatures.zone = Some(Zone::Battlefield);
    creatures.controller = Some(player);
    ChooseSpec::All(creatures)
}

#[test]
fn opponent_control_and_not_you_remain_distinct_in_goad_surfaces() {
    let opponents = Effect::goad(all_creatures_controlled_by(PlayerFilter::Opponent));
    assert_eq!(
        describe_effect(&opponents),
        "Goad all creatures your opponents control"
    );

    let not_you = Effect::goad(all_creatures_controlled_by(PlayerFilter::NotYou));
    assert_eq!(
        describe_effect(&not_you),
        "Goad all creatures you don't control"
    );
}

#[test]
fn linked_all_goaded_set_keeps_its_plural_restriction_back_reference() {
    let goad = Effect::goad(all_creatures_controlled_by(PlayerFilter::Opponent));
    let mut goaded_set = ObjectFilter::default();
    goaded_set
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: TagKey::from("goaded_0"),
            relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        });
    let cant_block = Effect::cant_until(
        crate::effect::Restriction::Block(goaded_set),
        Until::YourNextTurn,
    );
    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![goad]),
        crate::resolution::ResolutionSegment::from_effects(vec![cant_block]),
    ]);

    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "Goad all creatures your opponents control. Until your next turn, those creatures can't block"
    );
}
