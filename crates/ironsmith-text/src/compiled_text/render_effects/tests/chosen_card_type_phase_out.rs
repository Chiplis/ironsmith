use super::*;

#[test]
fn typed_card_type_choice_and_phase_out_render_as_one_correlated_procedure() {
    let choose = Effect::choose_card_type(
        PlayerFilter::IteratedPlayer,
        vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Land,
            CardType::Enchantment,
        ],
    );
    let mut filter = ObjectFilter::default()
        .in_zone(Zone::Battlefield)
        .nontoken();
    filter.chosen_card_type = true;
    filter.excluded_subtypes.push(Subtype::Aura);
    let phase_out = Effect::new(crate::effects::PhaseOutEffect::all(filter));

    assert_eq!(
        describe_effect_list(&[choose, phase_out]),
        "That player chooses artifact, creature, land, or non-Aura enchantment. All nontoken permanents of that type phase out"
    );

    let choose = Effect::choose_card_type(
        PlayerFilter::IteratedPlayer,
        vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Land,
            CardType::Enchantment,
        ],
    );
    let mut filter = ObjectFilter::default()
        .in_zone(Zone::Battlefield)
        .nontoken();
    filter.chosen_card_type = true;
    filter.excluded_subtypes.push(Subtype::Aura);
    let phase_out = Effect::new(crate::effects::PhaseOutEffect::all(filter));
    assert_eq!(
        describe_effect_clause_list(&[choose, phase_out]).as_deref(),
        Some(
            "that player chooses artifact, creature, land, or non-Aura enchantment. All nontoken permanents of that type phase out"
        )
    );
}

#[test]
fn phase_out_compaction_rejects_an_uncorrelated_permanent_filter() {
    let choose = Effect::choose_card_type(
        PlayerFilter::You,
        vec![CardType::Artifact, CardType::Creature],
    );
    let phase_out = Effect::new(crate::effects::PhaseOutEffect::all(
        ObjectFilter::permanent()
            .in_zone(Zone::Battlefield)
            .nontoken(),
    ));

    assert_ne!(
        describe_effect_list(&[choose, phase_out]),
        "You choose artifact or creature. All nontoken permanents of that type phase out"
    );
}
