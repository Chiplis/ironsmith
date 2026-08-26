use super::*;

#[test]
fn synthetic_target_with_one_value_consumer_folds_into_that_action() {
    let target = ChooseSpec::target_creature();
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(target.clone())).tag("targeted_0"),
        Effect::new(crate::effects::GainLifeEffect::you(Value::PowerOf(
            Box::new(target),
        ))),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "You gain life equal to target creature's power"
    );
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some("you gain life equal to target creature's power")
    );
}

#[test]
fn synthetic_target_draw_preserves_typed_for_each_surface() {
    let opponent = PlayerFilter::Target(Box::new(PlayerFilter::Opponent));
    let target = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Opponent));
    let mut tapped_creatures = ObjectFilter::creature().controlled_by(opponent);
    tapped_creatures.tapped = true;
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(target)).tag("targeted_player_0"),
        Effect::new(crate::effects::DrawCardsEffect::you(
            Value::Count(tapped_creatures).with_surface_hint(ValueSurfaceHint::ForEach),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Draw a card for each tapped creature target opponent controls"
    );
}

#[test]
fn synthetic_target_with_two_consumers_folds_when_first_consumer_names_target() {
    let tag = TagKey::from("targeted_0");
    let target = ChooseSpec::target_creature();
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(target.clone())).tag(tag.clone()),
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            target,
        )),
        Effect::new(crate::effects::DrawCardsEffect::you(Value::PowerOf(
            Box::new(ChooseSpec::Tagged(tag)),
        ))),
    ];

    let rendered = describe_effect_list(&effects);
    assert!(
        !rendered.starts_with("Choose target creature"),
        "{rendered}"
    );
    let lowercase = rendered.to_ascii_lowercase();
    assert!(lowercase.contains("put a +1/+1 counter on target creature"));
    assert!(
        lowercase.contains("draw cards equal to that creature's power")
            || lowercase.contains("draw cards equal to its power")
    );

    let clause = describe_effect_clause_list(&effects).expect("clause rendering");
    assert!(!clause.starts_with("choose target creature"), "{clause}");
}

#[test]
fn synthetic_target_with_anaphoric_consumers_retains_declaration() {
    let tag = TagKey::from("targeted_0");
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_creature(),
        ))
        .tag(tag.clone()),
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            ChooseSpec::Tagged(tag.clone()),
        )),
        Effect::new(crate::effects::DrawCardsEffect::you(Value::PowerOf(
            Box::new(ChooseSpec::Tagged(tag)),
        ))),
    ];

    let rendered = describe_effect_list(&effects);
    assert!(rendered.starts_with("Choose target creature"), "{rendered}");
}

#[test]
fn synthetic_target_with_coordinated_continuous_consumers_becomes_the_shared_subject() {
    let tag = TagKey::from("targeted_0");
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_creature(),
        ))
        .tag(tag.clone()),
        Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Tagged(tag.clone()),
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::flying(),
            ),
            Until::EndOfTurn,
        ))
        .tag("granted_1"),
        Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Tagged(tag),
            crate::continuous::Modification::ModifyPowerToughness {
                power: 1,
                toughness: 1,
            },
            Until::EndOfTurn,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target creature gains flying and gets +1/+1 until end of turn"
    );
}

#[test]
fn synthetic_player_target_folds_into_shared_controller_consumers() {
    let tag = TagKey::from("targeted_player_0");
    let target = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any));
    let creatures =
        ObjectFilter::creature().controlled_by(PlayerFilter::Target(Box::new(PlayerFilter::Any)));
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(target)),
        Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::All(creatures.clone()),
            crate::continuous::Modification::ModifyPowerToughness {
                power: 2,
                toughness: 0,
            },
            Until::EndOfTurn,
        ))
        .tag(tag.clone()),
        Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::All(creatures),
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::haste(),
            ),
            Until::EndOfTurn,
        ))
        .tag(tag),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "All creatures target player controls get +2/+0 and gain haste until end of turn"
    );
}

#[test]
fn torrent_of_souls_keeps_its_inline_player_target() {
    const ORACLE: &str = "Return up to one target creature card from your graveyard to the battlefield if {B} was spent to cast this spell. Creatures target player controls get +2/+0 and gain haste until end of turn if {R} was spent to cast this spell.";
    let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Torrent of Souls",
    )
    .parse_text(ORACLE)
    .expect("Torrent of Souls should compile through the public route");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [
            "Return up to one target creature card from your graveyard to the battlefield if {B} was spent to cast this spell.",
            "Creatures target player controls get +2/+0 and gain haste until end of turn if {R} was spent to cast this spell.",
        ]
    );
}

#[test]
fn synthetic_target_with_one_attached_object_consumer_folds_the_anchor() {
    let tag = TagKey::from("targeted_0");
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default()
            .with_type(CardType::Land)
            .in_zone(Zone::Battlefield),
    ));
    let mut attached_auras = ObjectFilter::default()
        .with_subtype(Subtype::Aura)
        .in_zone(Zone::Battlefield);
    attached_auras
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: tag.clone(),
            relation: TaggedOpbjectRelation::AttachedToTaggedObject,
        });
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(target)).tag(tag),
        Effect::new(crate::effects::DestroyEffect::all(attached_auras)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Destroy all Auras attached to target land"
    );
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some("destroy all Auras attached to target land")
    );
}

#[test]
fn synthetic_spell_target_folds_into_controller_and_mana_value_damage() {
    let tag = TagKey::from("targeted_0");
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_spell(),
        ))
        .tag(tag.clone()),
        Effect::deal_damage(
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(tag.clone())))
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
            ChooseSpec::Player(PlayerFilter::ControllerOf(
                crate::filter::ObjectRef::Tagged(tag),
            )),
        ),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Deal damage to target spell's controller equal to that spell's mana value"
    );
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some("deal damage to target spell's controller equal to that spell's mana value")
    );
}

fn conditional_counter_spell_with_mana_value(
    comparison: ironsmith_core::FilterComparison,
) -> Vec<Effect> {
    let tag = TagKey::from("countered_0");
    let target = ChooseSpec::target_spell();
    let mut condition_filter = ObjectFilter::default();
    condition_filter.mana_value = Some(comparison);
    vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(target.clone())).tag(tag.clone()),
        Effect::new(crate::effects::ConditionalEffect::if_only(
            Condition::TaggedObjectMatches(tag.clone(), condition_filter),
            vec![Effect::counter(target).tag(tag)],
        )),
    ]
}

#[test]
fn synthetic_counter_spell_fixed_mana_value_gate_uses_target_possessive() {
    let effects = conditional_counter_spell_with_mana_value(
        ironsmith_core::FilterComparison::LessThanOrEqual(2),
    );

    assert_eq!(
        describe_effect_list(&effects),
        "Counter target spell if its mana value is 2 or less"
    );
}

#[test]
fn synthetic_counter_spell_dynamic_mana_value_gate_uses_target_possessive() {
    let effects = conditional_counter_spell_with_mana_value(
        ironsmith_core::FilterComparison::EqualExpr(Box::new(Value::X)),
    );

    assert_eq!(
        describe_effect_list(&effects),
        "Counter target spell if its mana value is X"
    );
}

fn conditional_graveyard_return_with_ally_count(action_target: ChooseSpec) -> Vec<Effect> {
    let tag = TagKey::from("targeted_0");
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature()
            .owned_by(PlayerFilter::Opponent)
            .in_zone(Zone::Graveyard),
    ));
    let tagged_reference = ChooseSpec::Tagged(tag.clone()).with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("it".to_string()),
        ),
    );
    let allies = ObjectFilter::default()
        .with_subtype(Subtype::Ally)
        .controlled_by(PlayerFilter::You);
    let condition = Condition::ValueComparison {
        left: Value::ManaValueOf(Box::new(tagged_reference)),
        operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
        right: Value::Count(allies),
    };
    let move_to_battlefield =
        crate::effects::MoveToZoneEffect::new(action_target, Zone::Battlefield, false)
            .under_you_control()
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put);

    vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(target)).tag(tag),
        Effect::new(crate::effects::ConditionalEffect::if_only(
            condition,
            vec![Effect::new(move_to_battlefield)],
        )),
    ]
}

#[test]
fn synthetic_graveyard_target_folds_into_dynamic_battlefield_return() {
    let action_target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature()
            .owned_by(PlayerFilter::Opponent)
            .in_zone(Zone::Graveyard),
    ));
    let effects = conditional_graveyard_return_with_ally_count(action_target);

    assert_eq!(
        describe_effect_list(&effects),
        "Put target creature card from an opponent's graveyard onto the battlefield under your control if its mana value is less than or equal to the number of Allies you control"
    );
}

#[test]
fn synthetic_graveyard_target_does_not_fold_a_different_action_target() {
    let action_target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature()
            .owned_by(PlayerFilter::You)
            .in_zone(Zone::Graveyard),
    ));
    let effects = conditional_graveyard_return_with_ally_count(action_target);
    let rendered = describe_effect_list(&effects);

    assert_ne!(
        rendered,
        "Put target creature card from an opponent's graveyard onto the battlefield under your control if its mana value is less than or equal to the number of Allies you control"
    );
    assert!(
        rendered.starts_with("Choose target creature card"),
        "{rendered}"
    );
}

#[test]
fn shared_creature_source_power_damage_keeps_one_authored_sentence() {
    const TEXT: &str = "Target creature you control deals X damage to any other target and X damage to itself, where X is its power.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Self-Destruct")
        .card_types(vec![CardType::Sorcery])
        .parse_text(TEXT)
        .expect("shared creature-source damage should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}
