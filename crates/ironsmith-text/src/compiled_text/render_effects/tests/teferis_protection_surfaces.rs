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

#[test]
fn phase_out_and_shared_next_turn_protection_keep_authored_coordination() {
    let phase = Effect::new(crate::effects::PhaseOutEffect::all(
        ObjectFilter::permanent_card()
            .in_zone(Zone::Battlefield)
            .you_control(),
    ));
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
    let effects = [phase, life_lock, cant_target, prevent_damage];
    let refs = effects.iter().collect::<Vec<_>>();

    assert_eq!(
        describe_phase_out_then_life_lock_and_protection(&refs),
        Some(
            "All permanents you control phase out, and until your next turn, your life total can't change and you gain protection from everything".to_string()
        )
    );

    let wrong_phase = Effect::new(crate::effects::PhaseOutEffect::all(
        ObjectFilter::permanent_card()
            .in_zone(Zone::Battlefield)
            .opponent_controls(),
    ));
    let near_miss = [&wrong_phase, &effects[1], &effects[2], &effects[3]];
    assert_eq!(
        describe_phase_out_then_life_lock_and_protection(&near_miss),
        None
    );
}

#[test]
fn complete_protection_program_keeps_authored_sentence_and_line_boundaries() {
    let oracle = "Until your next turn, your life total can't change and you gain protection from everything. All permanents you control phase out.\nExile Teferi's Protection.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Teferi's Protection")
            .card_types(vec![CardType::Instant])
            .parse_text(oracle)
            .expect("Teferi's Protection should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}
