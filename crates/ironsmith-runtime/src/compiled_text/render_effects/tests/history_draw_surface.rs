use super::*;

fn outside_hand_spell_history() -> Value {
    let mut spell = ObjectFilter::default();
    spell.stack_kind = Some(crate::filter::StackObjectKind::Spell);
    Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::SpellsCast {
        player: PlayerFilter::You,
        filter: spell,
        from_zone: None,
        from_outside_hand: true,
        exclude_source: false,
        before_triggering_spell: false,
    })
}

#[test]
fn typed_death_history_for_each_draw_uses_singular_event_surface() {
    let mut zubera = ObjectFilter::default();
    zubera.subtypes.push(Subtype::Zubera);
    let count = Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::Died(zubera))
        .with_surface_hint(ValueSurfaceHint::ForEach);
    let draw = Effect::new(crate::effects::DrawCardsEffect::you(count));

    assert_eq!(
        describe_effect(&draw),
        "you draw a card for each Zubera that died this turn"
    );
}

#[test]
fn typed_spell_history_for_each_draw_preserves_spell_and_cast_origin() {
    let count = outside_hand_spell_history().with_surface_hint(ValueSurfaceHint::ForEach);
    let draw = Effect::new(crate::effects::DrawCardsEffect::you(count));

    assert_eq!(
        describe_effect(&draw),
        "you draw a card for each spell you've cast from anywhere other than your hand this turn"
    );
}

#[test]
fn fixed_plus_spell_history_damage_keeps_where_x_across_quantified_targets() {
    let amount = Value::Add(
        Box::new(Value::Fixed(1)),
        Box::new(outside_hand_spell_history()),
    )
    .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let player_damage = Effect::deal_damage(
        amount.clone(),
        ChooseSpec::Player(PlayerFilter::IteratedPlayer),
    );
    let object_damage = Effect::for_each(
        ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
        vec![Effect::deal_damage(amount, ChooseSpec::Iterated)],
    );
    let damage = Effect::for_players(PlayerFilter::Opponent, vec![player_damage, object_damage]);

    assert_eq!(
        describe_effect(&damage),
        "Deal X damage to each opponent and each creature they control, where X is 1 plus the number of spells you've cast from anywhere other than your hand this turn"
    );
}

#[test]
fn triggering_spell_history_renders_boundary_and_instant_sorcery_surface() {
    let mut spell = ObjectFilter::default();
    spell.card_types = vec![CardType::Instant, CardType::Sorcery];
    spell.stack_kind = Some(crate::filter::StackObjectKind::Spell);
    let count = Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::SpellsCast {
        player: PlayerFilter::You,
        filter: spell,
        from_zone: None,
        from_outside_hand: false,
        exclude_source: true,
        before_triggering_spell: true,
    });
    let copy = Effect::new(crate::effects::CopySpellEffect::new(
        ChooseSpec::Source,
        count,
    ));

    assert_eq!(
        describe_effect(&copy),
        "Copy it for each other instant and sorcery spell you've cast before it this turn"
    );
}

#[test]
fn multi_copy_retarget_sequence_uses_plural_copy_reference() {
    let mut spell = ObjectFilter::default();
    spell.card_types = vec![CardType::Instant, CardType::Sorcery];
    spell.stack_kind = Some(crate::filter::StackObjectKind::Spell);
    let count = Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::SpellsCast {
        player: PlayerFilter::You,
        filter: spell,
        from_zone: None,
        from_outside_hand: false,
        exclude_source: true,
        before_triggering_spell: true,
    });
    let copied = TagKey::from("__copied_stack_object__");
    let copy = Effect::with_id(
        0,
        Effect::new(crate::effects::CopySpellEffect::new(
            ChooseSpec::Source,
            count,
        )),
    )
    .tag(copied.clone());
    let retarget = Effect::new(crate::effects::RetargetStackObjectEffect::new(
        ChooseSpec::Tagged(copied),
    ));

    assert_eq!(
        describe_effect_list(&[copy, Effect::may(vec![retarget])]),
        "Copy it for each other instant and sorcery spell you've cast before it this turn. You may choose new targets for the copies"
    );
}

#[test]
fn token_copy_count_keeps_where_x_history_surface() {
    let mut spell = ObjectFilter::default();
    spell.card_types = vec![CardType::Instant, CardType::Sorcery];
    spell.stack_kind = Some(crate::filter::StackObjectKind::Spell);
    let history = Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::SpellsCast {
        player: PlayerFilter::You,
        filter: spell,
        from_zone: None,
        from_outside_hand: false,
        exclude_source: false,
        before_triggering_spell: false,
    });
    let count = Value::Add(Box::new(Value::Fixed(1)), Box::new(history))
        .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let mut target = ObjectFilter::creature().controlled_by(PlayerFilter::You);
    target.other = true;
    let create = Effect::new(crate::effects::CreateTokenCopyEffect::new(
        ChooseSpec::target(ChooseSpec::Object(target)),
        count,
        PlayerFilter::You,
    ));

    assert_eq!(
        describe_effect(&create),
        "Create X tokens that are copies of another target creature you control, where X is 1 plus the number of instant and sorcery spells you've cast this turn"
    );
}
