use super::*;
use crate::zone::Zone;

fn rendered_counter_replacement(
    replacement_zone: Zone,
    placement: Option<ironsmith_core::ZoneReplacementLibraryPlacement>,
) -> String {
    let tag = TagKey::from("countered_spell");
    let producer = Effect::new(crate::effects::CounterEffect::new(ChooseSpec::target(
        ChooseSpec::Object(ObjectFilter::spell()),
    )))
    .tag(tag.clone());
    let mut replacement = crate::effects::RegisterZoneReplacementEffect::new(
        ChooseSpec::Tagged(tag),
        Some(Zone::Stack),
        Some(Zone::Graveyard),
        replacement_zone,
        crate::effects::ReplacementApplyMode::OneShot,
    );
    if let Some(placement) = placement {
        replacement = replacement.with_library_placement(placement);
    }
    describe_effect_list(&[producer, Effect::new(replacement)])
}

#[test]
fn countered_spell_replacement_preserves_destination_surfaces() {
    use ironsmith_core::ZoneReplacementLibraryPlacement::{Bottom, Top, TopOrBottom};

    assert_eq!(
        rendered_counter_replacement(Zone::Exile, None),
        "Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard"
    );
    assert_eq!(
        rendered_counter_replacement(Zone::Hand, None),
        "Counter target spell. If that spell is countered this way, put it into its owner's hand instead of into that player's graveyard"
    );
    assert_eq!(
        rendered_counter_replacement(Zone::Library, Some(Top)),
        "Counter target spell. If that spell is countered this way, put it on top of its owner's library instead of into that player's graveyard"
    );
    assert_eq!(
        rendered_counter_replacement(Zone::Library, Some(Bottom)),
        "Counter target spell. If that spell is countered this way, put it on the bottom of its owner's library instead of into that player's graveyard"
    );
    assert_eq!(
        rendered_counter_replacement(Zone::Library, Some(TopOrBottom)),
        "Counter target spell. If that spell is countered this way, put that card on your choice of the top or bottom of its owner's library instead of into that player's graveyard"
    );
}

#[test]
fn atomic_counter_permission_preserves_gate_verb_price_and_lifetime() {
    for (gate, antecedent) in [
        (ironsmith_core::CounterExileGate::AnySpell, "that spell"),
        (ironsmith_core::CounterExileGate::PermanentSpell, "a permanent spell"),
    ] {
        for (allow_land, verb) in [(true, "play it"), (false, "cast that card")] {
            let counter = crate::effects::CounterEffect::new(ChooseSpec::target(
                ChooseSpec::Object(ObjectFilter::spell()),
            ))
            .with_exile_permission(ironsmith_core::CounterExilePermission { gate, allow_land });
            let expected = format!(
                "Counter target spell. If {antecedent} is countered this way, exile it instead of putting it into its owner's graveyard. You may {verb} without paying its mana cost for as long as it remains exiled"
            );
            let effect = Effect::new(counter);
            assert_eq!(describe_effect(&effect), expected);
            assert_eq!(describe_effect_list(&[effect]), expected);
        }
    }
}

#[test]
fn atomic_counter_permission_keeps_the_original_counter_target_surface() {
    let counter = crate::effects::CounterEffect::new(ChooseSpec::target(
        ChooseSpec::Object(ObjectFilter::spell().in_zone(Zone::Graveyard)),
    ))
    .with_exile_permission(ironsmith_core::CounterExilePermission {
        gate: ironsmith_core::CounterExileGate::PermanentSpell,
        allow_land: false,
    });
    assert_eq!(
        describe_effect(&Effect::new(counter)),
        "Counter target spell cast from a graveyard. If a permanent spell is countered this way, exile it instead of putting it into its owner's graveyard. You may cast that card without paying its mana cost for as long as it remains exiled"
    );
    assert_eq!(
        describe_effect(&Effect::new(crate::effects::CounterEffect::new(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::spell())),
        ))),
        "Counter target spell"
    );
}

#[test]
fn plain_counter_compactions_do_not_erase_an_atomic_permission() {
    let counter = Effect::new(crate::effects::CounterEffect::new(ChooseSpec::target(
        ChooseSpec::Object(ObjectFilter::spell()),
    )).with_exile_permission(ironsmith_core::CounterExilePermission {
        gate: ironsmith_core::CounterExileGate::PermanentSpell,
        allow_land: false,
    }));
    let conditional = crate::effects::ConditionalEffect::new(
        crate::effect::Condition::TargetSpellCastOrderThisTurn(2),
        vec![counter.clone()], vec![],
    );
    assert!(describe_second_spell_counter_conditional(&conditional).is_none());
    let tag = TagKey::from("original_target");
    assert!(describe_conditional_action_on_tagged_target(
        &counter, &tag, &ChooseSpec::target(ChooseSpec::spell()), "target spell",
    ).is_none());
    let damage = Effect::deal_damage(1, ChooseSpec::target_creature());
    assert!(describe_counter_and_damage_sequence(&[counter.clone(), damage.clone()]).is_none());
    let sequence_text = describe_effect_list(&[counter.clone(), damage]);
    assert!(sequence_text.contains("for as long as it remains exiled"), "{sequence_text}");
    let unless = crate::effects::UnlessPaysEffect::new(
        vec![counter], PlayerFilter::Opponent,
        vec![crate::mana::ManaSymbol::Generic(3)],
    );
    for effect in [Effect::new(conditional), Effect::new(unless)] {
        let rendered = describe_effect(&effect);
        assert!(rendered.contains("If a permanent spell is countered this way"), "{rendered}");
        assert!(rendered.contains("You may cast that card without paying its mana cost for as long as it remains exiled"), "{rendered}");
    }
}
