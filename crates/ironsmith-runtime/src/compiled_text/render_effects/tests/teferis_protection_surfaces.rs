use super::*;

#[test]
fn life_lock_and_protection_share_one_leading_duration() {
    let life_lock = Effect::new(crate::effects::CantEffect::new(
        crate::effect::Restriction::ChangeLifeTotal(PlayerFilter::You),
        Until::YourNextTurn,
    ));
    let cant_target = Effect::new(crate::effects::CantEffect::new(
        crate::effect::Restriction::BeTargetedPlayer(PlayerFilter::You),
        Until::YourNextTurn,
    ));
    let prevent_damage = Effect::new(crate::effects::PreventAllDamageToTargetEffect::new(
        ChooseSpec::SourceController,
        Until::YourNextTurn,
    ));
    let sequence = Effect::new(crate::effects::SequenceEffect::new(vec![
        life_lock,
        cant_target,
        prevent_damage,
    ]));

    assert_eq!(
        describe_effect(&sequence),
        "Until your next turn, your life total can't change and you gain protection from everything"
    );
}

#[test]
fn an_all_selection_uses_the_subject_verb_phase_out_surface() {
    let effect = Effect::new(crate::effects::PhaseOutEffect::all(
        ObjectFilter::permanent().you_control(),
    ));

    assert_eq!(
        describe_effect(&effect),
        "All permanents you control phase out"
    );
}
