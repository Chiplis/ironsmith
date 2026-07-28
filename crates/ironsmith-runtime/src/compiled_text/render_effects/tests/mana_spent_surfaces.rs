use super::*;

#[test]
fn repeated_mana_symbol_count_preserves_group_and_cast_reference() {
    let count = Value::DividedRoundedDown(
        Box::new(Value::ManaSymbolSpentToCastThisSpell {
            symbol: crate::mana::ManaSymbol::Blue,
            reference: ironsmith_core::ManaSpentCastReferenceSurface::It,
        }),
        2,
    )
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let repeat = Effect::new(crate::effects::RepeatEffectsEffect::new(
        count,
        vec![Effect::new(crate::effects::DrawCardsEffect::new(
            1,
            PlayerFilter::You,
        ))],
    ));

    assert_eq!(
        describe_effect(&repeat),
        "For each {U}{U} spent to cast it, you draw a card"
    );
}

#[test]
fn single_mana_symbol_count_preserves_this_creature_reference() {
    let count = Value::ManaSymbolSpentToCastThisSpell {
        symbol: crate::mana::ManaSymbol::Green,
        reference: ironsmith_core::ManaSpentCastReferenceSurface::ThisCreature,
    }
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let repeat = Effect::new(crate::effects::RepeatEffectsEffect::new(
        count,
        vec![Effect::gain_life(1)],
    ));

    assert_eq!(
        describe_effect(&repeat),
        "For each {G} spent to cast this creature, you gain 1 life"
    );
}

#[test]
fn scry_where_x_mana_symbol_then_draw_preserves_the_binding() {
    let count = Value::ManaSymbolSpentToCastThisSpell {
        symbol: crate::mana::ManaSymbol::Snow,
        reference: ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell,
    }
    .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let sequence = Effect::new(crate::effects::SequenceEffect::comma_then(vec![
        Effect::scry(count),
        Effect::draw(Value::Fixed(3)),
    ]));

    assert_eq!(
        describe_effect(&sequence),
        "Scry X, where X is the amount of {S} spent to cast this spell, then draw three cards"
    );
}
