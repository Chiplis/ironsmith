use super::shard_00::*;
use super::*;

#[test]
pub(super) fn cant_untap_renders_distributive_compound_filter_subject() {
    let mut attacking = ObjectFilter::creature().in_zone(Zone::Battlefield);
    attacking.attacking = true;
    let mut blocking = ObjectFilter::creature().in_zone(Zone::Battlefield);
    blocking.blocking = true;
    let mut compound = ObjectFilter::default();
    compound.any_of = vec![attacking, blocking];
    let effect = Effect::cant_until(
        crate::effect::Restriction::Untap(compound),
        Until::ControllersNextUntapStep,
    );

    assert_eq!(
        describe_effect(&effect),
        "Each attacking creature and each blocking creature doesn't untap during its controller's next untap step"
    );
}

#[test]
pub(super) fn describe_effect_clause_list_compacts_draw_then_choose_sacrifice() {
    let tag = TagKey::from("sacrificed_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::permanent()
            .controlled_by(PlayerFilter::You)
            .in_zone(Zone::Battlefield),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Battlefield);
    let sacrifice = Effect::sacrifice_player(ObjectFilter::tagged(tag), 1, PlayerFilter::You);
    let effects = vec![
        Effect::draw(Value::Fixed(2)),
        Effect::new(choose),
        sacrifice,
    ];

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some("draw two cards, then sacrifice a permanent")
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_choose_sacrifice_then_source_damage() {
    let tag = TagKey::from("sacrificed_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::land()
            .controlled_by(PlayerFilter::You)
            .in_zone(Zone::Battlefield),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    );
    let mut sacrifice_filter = ObjectFilter::default();
    sacrifice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag,
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let sacrifice = crate::effects::zones::SacrificePlayerEffect::new(
        sacrifice_filter,
        Value::Fixed(1),
        PlayerFilter::You,
    );
    let damage =
        crate::effects::DealDamageEffect::new(Value::Fixed(1), ChooseSpec::SourceController);
    let effects = vec![
        Effect::new(choose),
        Effect::new(sacrifice),
        Effect::new(damage),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Sacrifice a land and deal 1 damage to you"
    );
}

#[test]
pub(super) fn describe_choose_then_move_to_library_accepts_iterated_move_targets() {
    let mut filter = ObjectFilter::default().in_zone(Zone::Hand);
    filter.owner = Some(PlayerFilter::IteratedPlayer);

    let choose = crate::effects::ChooseObjectsEffect::new(
        filter,
        ChoiceCount::exactly(3),
        PlayerFilter::target_player(),
        TagKey::from("__it__"),
    );
    let move_to_zone =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Library, true);

    let compact = describe_choose_then_move_to_library(&choose, &move_to_zone)
        .expect("iterated move-to-library should compact");
    assert_eq!(
        compact,
        "target player chooses three cards from their hand and puts them on top of their library in any order"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_target_player_hand_cards_top_any_order() {
    let target = Effect::new(crate::effects::TargetOnlyEffect::new(
        ChooseSpec::target_player(),
    ));
    let chosen_tag = TagKey::from("__it__");
    let mut filter = ObjectFilter::default().in_zone(Zone::Hand);
    filter.owner = Some(PlayerFilter::IteratedPlayer);
    let choose = crate::effects::ChooseObjectsEffect::new(
        filter,
        ChoiceCount::exactly(3),
        PlayerFilter::target_player(),
        chosen_tag.clone(),
    )
    .in_zone(Zone::Hand);
    let move_to_zone = crate::effects::TaggedEffect::new(
        TagKey::from("moved_0"),
        Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(chosen_tag),
            Zone::Library,
            true,
        )),
    );
    let effects = vec![target, Effect::new(choose), Effect::new(move_to_zone)];

    assert_eq!(
        describe_effect_list(&effects),
        "Target player chooses three cards from their hand and puts them on top of their library in any order"
    );
}

#[test]
pub(super) fn describe_effect_clause_list_compacts_reveal_hand_choose_nth_library() {
    let chosen_tag = TagKey::from("__it__");
    let look = crate::effects::LookAtHandEffect::reveal(ChooseSpec::target_player());
    let mut filter = ObjectFilter::default().in_zone(Zone::Hand);
    filter.owner = Some(PlayerFilter::target_player());
    filter.excluded_card_types.push(CardType::Land);
    let choose = crate::effects::ChooseObjectsEffect::new(
        filter,
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        chosen_tag.clone(),
    )
    .in_zone(Zone::Hand);
    let move_to_library = crate::effects::TaggedEffect::new(
        TagKey::from("moved_0"),
        Effect::new(crate::effects::MoveToLibraryNthFromTopEffect::new(
            ChooseSpec::Tagged(chosen_tag),
            Value::Fixed(3),
        )),
    );
    let effects = vec![
        Effect::new(look),
        Effect::new(choose),
        Effect::new(move_to_library),
    ];
    let expected = "Target player reveals their hand. You choose a nonland card from it. That player puts that card into their library third from the top";

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
    assert_eq!(describe_effect_list(&effects), expected);
}

#[test]
pub(super) fn describe_effect_list_compacts_choose_then_put_counter_on_each() {
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::permanent().controlled_by(PlayerFilter::NotYou),
        ChoiceCount::exactly(4),
        PlayerFilter::You,
        TagKey::from("__it__"),
    )
    .in_zone(Zone::Battlefield);

    let mut each_filter = ObjectFilter::permanent();
    each_filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("__it__"),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let for_each = crate::effects::ForEachObject::new(
        each_filter,
        vec![Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::Aim,
            1,
            ChooseSpec::Iterated,
        ))],
    );

    assert_eq!(
        describe_effect_list(&[Effect::new(choose), Effect::new(for_each)]),
        "Choose four permanents you don't control and put an aim counter on each of them"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_untap_attackers_then_additional_combat() {
    let mut attacking_creature = ObjectFilter::creature();
    attacking_creature.attacking = true;
    let effects = vec![
        Effect::new(crate::effects::UntapEffect::with_spec(ChooseSpec::All(
            attacking_creature,
        ))),
        Effect::new(crate::effects::AdditionalPhasesEffect::combat()),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Untap each attacking creature. After this phase, there is an additional combat phase"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_triggering_untap_then_remove_from_combat() {
    let triggering = TagKey::from("triggering");
    let effects = vec![
        Effect::tag_triggering_object(triggering.clone()),
        Effect::new(crate::effects::UntapEffect::with_spec(ChooseSpec::Tagged(
            triggering.clone(),
        )))
        .tag("untapped_0"),
        Effect::new(crate::effects::RemoveFromCombatEffect::with_spec(
            ChooseSpec::Tagged(triggering),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "untap it and remove it from combat"
    );
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some("untap it and remove it from combat")
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_remove_counter_then_no_counters_transform() {
    let effects = vec![
        Effect::new(crate::effects::RemoveCountersEffect::new(
            crate::object::CounterType::Ice,
            1,
            ChooseSpec::Source,
        )),
        Effect::new(crate::effects::ConditionalEffect::new(
            Condition::SourceHasNoCounter(crate::object::CounterType::Ice),
            vec![Effect::transform(ChooseSpec::Source)],
            Vec::new(),
        )),
    ];
    let expected =
        "Remove an ice counter from it. Then if it has no ice counters on it, transform it";

    assert_eq!(describe_effect_list(&effects), expected);
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
}

#[test]
pub(super) fn your_turn_followup_uses_an_independent_if_sentence() {
    let effects = vec![
        Effect::draw(7),
        Effect::new(crate::effects::ConditionalEffect::if_only(
            Condition::YourTurn,
            vec![Effect::scry(2)],
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "You draw seven cards. If it's your turn, scry 2"
    );
}

#[test]
pub(super) fn describe_for_each_counter_then_untap_accepts_tagged_current_object() {
    let tag = TagKey::from("counters_0");
    let counters = Effect::new(crate::effects::ForEachObject::new(
        ObjectFilter::creature().controlled_by(PlayerFilter::You),
        vec![Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            ChooseSpec::Iterated,
        ))],
    ))
    .tag(tag.clone());
    let mut untap_filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
    untap_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag,
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let untap = Effect::new(crate::effects::UntapEffect::with_spec(ChooseSpec::Object(
        untap_filter,
    )));

    assert_eq!(
        describe_put_counters_then_untap_them(&counters, &untap),
        Some("Put a +1/+1 counter on each creature you control. Untap those creatures".to_string())
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_put_counters_then_grant_same_filter() {
    let tag = TagKey::from("counters_0");
    let filter = ObjectFilter::creature()
        .token()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You);
    let put = Effect::new(crate::effects::ForEachObject::new(
        filter,
        vec![Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            ChooseSpec::Iterated,
        ))],
    ))
    .tag(tag.clone());
    let mut grant = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::trample(),
        ),
        Until::EndOfTurn,
    );
    grant.target_spec = Some(ChooseSpec::Tagged(tag));

    assert_eq!(
        describe_effect_list(&[put, Effect::new(grant)]),
        "Put a +1/+1 counter on each creature token you control. Those creatures gain trample until end of turn"
    );
}

#[test]
pub(super) fn ajani_style_adjacent_counter_and_grant_preserve_the_sentence_boundary() {
    let affected = TagKey::from("counters_0");
    let put = Effect::new(crate::effects::ForEachObject::new(
        ObjectFilter::creature()
            .controlled_by(PlayerFilter::You)
            .in_zone(Zone::Battlefield),
        vec![Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            ChooseSpec::Iterated,
        ))],
    ))
    .tag(affected.clone());
    let mut grant = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::vigilance(),
        ),
        Until::EndOfTurn,
    );
    grant.target_spec = Some(ChooseSpec::Tagged(affected));
    grant.set_quantifier_surface = Some(ironsmith_core::SetQuantifierSurface::Each);

    assert_eq!(
        describe_effect_list(&[put, Effect::new(grant)]),
        "Put a +1/+1 counter on each creature you control. Those creatures gain vigilance until end of turn"
    );
}

#[test]
pub(super) fn coordinated_counter_and_grant_use_the_authored_conjunction() {
    let affected = TagKey::from("counters_0");
    let put = Effect::new(crate::effects::ForEachObject::new(
        ObjectFilter::creature()
            .controlled_by(PlayerFilter::You)
            .in_zone(Zone::Battlefield),
        vec![Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            ChooseSpec::Iterated,
        ))],
    ))
    .tag(affected.clone());
    let mut grant = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::vigilance(),
        ),
        Until::EndOfTurn,
    );
    grant.target_spec = Some(ChooseSpec::Tagged(affected));
    grant.set_quantifier_surface = Some(ironsmith_core::SetQuantifierSurface::Each);
    let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
        put,
        Effect::new(grant),
    ]));

    assert_eq!(
        describe_effect(&sequence),
        "Put a +1/+1 counter on each creature you control and they gain vigilance until end of turn"
    );
}

#[test]
pub(super) fn adjacent_counter_then_keyword_for_same_direct_object_stays_sequential() {
    let triggering = TagKey::from("triggering");
    let put = Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::PlusOnePlusOne,
        1,
        ChooseSpec::Tagged(triggering.clone()),
    ));
    let mut grant = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(crate::static_abilities::StaticAbility::haste()),
        Until::EndOfTurn,
    );
    grant.target_spec = Some(ChooseSpec::Tagged(triggering));

    assert_eq!(
        describe_effect_list(&[put, Effect::new(grant)]),
        "Put a +1/+1 counter on it. It gains haste until end of turn"
    );
}

#[test]
pub(super) fn adjacent_counter_then_keyword_for_exact_affected_tag_stays_sequential() {
    let affected = TagKey::from("counters_0");
    let put = Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::PlusOnePlusOne,
        1,
        ChooseSpec::Target(Box::new(ChooseSpec::Object(
            ObjectFilter::creature()
                .controlled_by(PlayerFilter::You)
                .in_zone(Zone::Battlefield),
        ))),
    ))
    .tag(affected.clone());
    let mut grant = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::flying(),
        ),
        Until::EndOfTurn,
    );
    grant.target_spec = Some(ChooseSpec::Tagged(affected));

    assert_eq!(
        describe_effect_list(&[put, Effect::new(grant)]),
        "Put a +1/+1 counter on target creature you control. It gains flying until end of turn"
    );
}

#[test]
pub(super) fn coordinated_direct_counter_then_keyword_uses_the_authored_conjunction() {
    let triggering = TagKey::from("triggering");
    let put = Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::PlusOnePlusOne,
        1,
        ChooseSpec::Tagged(triggering.clone()),
    ));
    let mut grant = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(crate::static_abilities::StaticAbility::haste()),
        Until::EndOfTurn,
    );
    grant.target_spec = Some(ChooseSpec::Tagged(triggering));
    let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
        put,
        Effect::new(grant),
    ]));

    assert_eq!(
        describe_effect(&sequence),
        "Put a +1/+1 counter on it and it gains haste until end of turn"
    );
}

#[test]
pub(super) fn counter_then_keyword_compaction_rejects_a_distinct_tag() {
    let put = Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::PlusOnePlusOne,
        1,
        ChooseSpec::Tagged(TagKey::from("first_object")),
    ));
    let mut grant = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(crate::static_abilities::StaticAbility::haste()),
        Until::EndOfTurn,
    );
    grant.target_spec = Some(ChooseSpec::Tagged(TagKey::from("other_object")));

    assert_eq!(
        describe_put_counters_then_grant_same_filter(&[put, Effect::new(grant)]),
        None
    );
}

#[test]
pub(super) fn describe_effect_list_preserves_distinct_power_choice_complement() {
    let chosen_tag = TagKey::from("chosen_power_classes");
    let filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            filter.clone(),
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            chosen_tag.clone(),
        )
        .in_zone(Zone::Battlefield),
    );
    let repeat = Effect::repeat_effects(Value::DistinctPowers(filter.clone()), vec![choose]);
    let mut destroy_filter = filter;
    destroy_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_tag,
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
    let destroy = Effect::destroy_all(destroy_filter).tag("destroyed_power_complement");

    assert_eq!(
        describe_effect_list(&[repeat, destroy]),
        "For each different power among creatures on the battlefield, choose a creature with that power. Destroy each creature not chosen this way"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_return_with_counter_and_static_followups() {
    let triggering = TagKey::from("triggering");
    let mut return_to_battlefield = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(triggering.clone()),
        Zone::Battlefield,
        false,
    );
    return_to_battlefield.battlefield_controller = crate::effects::BattlefieldController::Owner;
    let return_effect = Effect::new(return_to_battlefield)
        .tag(TagKey::from("returned_0"))
        .tag(TagKey::from("returned_1"));

    let counters = Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::PlusOnePlusOne,
        1,
        ChooseSpec::Tagged(triggering.clone()),
    ))
    .tag(TagKey::from("counters_2"));

    let mut grant_flying = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::flying(),
        ),
        Until::Forever,
    );
    grant_flying.target_spec = Some(ChooseSpec::Tagged(triggering.clone()));

    let mut add_angel = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddSubtypes(vec![crate::types::Subtype::Angel]),
        Until::Forever,
    );
    add_angel.target_spec = Some(ChooseSpec::Tagged(triggering));

    assert_eq!(
        describe_effect_list(&[
            Effect::tag_triggering_object("triggering"),
            return_effect,
            counters,
            Effect::new(grant_flying).tag(TagKey::from("granted_3")),
            Effect::new(add_angel),
        ]),
        "Return that card to the battlefield under its owner's control with a +1/+1 counter on it. It has flying and is an Angel in addition to its other types"
    );
}

#[test]
pub(super) fn describe_return_followups_track_the_returned_object_result_tag() {
    let triggering = TagKey::from("triggering");
    let returned = TagKey::from("returned_0");
    let mut return_to_battlefield = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(triggering.clone()),
        Zone::Battlefield,
        false,
    );
    return_to_battlefield.battlefield_controller = crate::effects::BattlefieldController::Owner;
    let return_effect = Effect::new(return_to_battlefield).tag(returned.clone());
    let counters = Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::PlusOnePlusOne,
        2,
        ChooseSpec::Tagged(returned.clone()),
    ));
    let mut add_demon = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddSubtypes(vec![crate::types::Subtype::Demon]),
        Until::Forever,
    );
    add_demon.target_spec = Some(ChooseSpec::Tagged(returned));

    assert_eq!(
        describe_effect_list(&[
            Effect::tag_triggering_object(triggering),
            return_effect,
            counters,
            Effect::new(add_demon),
        ]),
        "Return that card to the battlefield under its owner's control with two +1/+1 counters on it. It is a Demon in addition to its other types"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_delayed_return_with_counter_and_static_followups() {
    let triggering = TagKey::from("triggering");
    let mut return_to_battlefield = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(triggering.clone()),
        Zone::Battlefield,
        false,
    );
    return_to_battlefield.battlefield_controller = crate::effects::BattlefieldController::You;
    let return_effect = Effect::new(return_to_battlefield)
        .tag(TagKey::from("returned_0"))
        .tag(TagKey::from("returned_1"));

    let counters = Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::PlusOnePlusOne,
        1,
        ChooseSpec::Tagged(triggering.clone()),
    ))
    .tag(TagKey::from("counters_2"));

    let schedule = crate::effects::ScheduleDelayedTriggerEffect::new(
        crate::triggers::Trigger::beginning_of_end_step(PlayerFilter::Any),
        vec![return_effect, counters],
        true,
        Vec::new(),
        PlayerFilter::You,
    );

    let mut add_black = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddColors(crate::color::ColorSet::BLACK),
        Until::Forever,
    );
    add_black.target_spec = Some(ChooseSpec::Tagged(triggering.clone()));

    let mut add_zombie = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddSubtypes(vec![crate::types::Subtype::Zombie]),
        Until::Forever,
    );
    add_zombie.target_spec = Some(ChooseSpec::Tagged(triggering));

    assert_eq!(
        describe_effect_list(&[
            Effect::tag_triggering_object("triggering"),
            Effect::new(schedule),
            Effect::new(add_black),
            Effect::new(add_zombie),
        ]),
        "Return that card to the battlefield under your control with a +1/+1 counter on it at the beginning of the next end step. That creature is a black Zombie in addition to its other colors and types"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_energy_pay_any_destroy_threshold() {
    let mut destroy_filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    destroy_filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
    ];
    destroy_filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqualExpr(Box::new(
        Value::EffectValue(crate::effect::EffectId(0)),
    )));

    let effects = vec![
        Effect::new(crate::effects::EnergyCountersEffect::new(
            Value::X,
            PlayerFilter::You,
        )),
        Effect::with_id(
            0,
            Effect::new(crate::effects::MayEffect::new_for_player(
                vec![Effect::new(crate::effects::PayAnyEnergyEffect::new(
                    ChooseSpec::Player(PlayerFilter::You),
                    0,
                ))],
                PlayerFilter::You,
            )),
        ),
        Effect::new(crate::effects::DestroyEffect::all(destroy_filter)).tag("destroyed_0"),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "you get X {E}, then you may pay any amount of {E}. Destroy each artifact, creature, and enchantment with mana value less than or equal to the amount of {E} paid this way"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_put_counter_then_goad_same_tagged_target() {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    let tagged_counters = Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::PlusOnePlusOne,
        1,
        target,
    ))
    .tag(TagKey::from("counters_0"));
    let goad = Effect::goad(ChooseSpec::Tagged(TagKey::from("counters_0")));

    assert_eq!(
        describe_effect_list(&[
            Effect::tag_triggering_object("triggering"),
            tagged_counters,
            goad
        ]),
        "Put a +1/+1 counter on target creature and goad it"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_put_counter_then_unblockable_same_source() {
    let counter_tag = TagKey::from("counters_0");
    let tagged_counters = Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::PlusOnePlusOne,
        1,
        ChooseSpec::Source.with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this creature".to_string(),
                ),
            ),
        ),
    ))
    .tag(counter_tag.clone());
    let target_only = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::Tagged(
        counter_tag,
    )));
    let unblockable = Effect::new(crate::effects::CantEffect::until_end_of_turn(
        crate::effect::Restriction::be_blocked(ObjectFilter::source()),
    ));

    assert_eq!(
        describe_effect_list(&[
            Effect::tag_triggering_object("triggering"),
            tagged_counters,
            target_only,
            unblockable
        ]),
        "Put a +1/+1 counter on this creature and it can't be blocked this turn"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_source_exile_with_all_source_counter_filters() {
    let source = ChooseSpec::Source.with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("this artifact".to_string()),
        ),
    );
    let mana_value = crate::filter::Comparison::LessThanOrEqualExpr(Box::new(
        Value::CountersOnSource(CounterType::Void),
    ));
    let battlefield_filter = ObjectFilter::creature()
        .with_type(CardType::Planeswalker)
        .with_mana_value(mana_value.clone());
    let graveyard_filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .with_type(CardType::Creature)
        .with_type(CardType::Planeswalker)
        .with_mana_value(mana_value);
    let effects = vec![
        Effect::new(crate::effects::MoveToZoneEffect::to_exile(source)),
        Effect::new(crate::effects::ExileEffect::all(battlefield_filter)),
        Effect::new(crate::effects::ExileEffect::all(graveyard_filter)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Exile this artifact, all creatures and planeswalkers with mana value less than or equal to the number of void counters on it, and all creature and planeswalker cards in graveyards with mana value less than or equal to the number of void counters on it"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_linked_same_source_damage() {
    let source = ChooseSpec::Source.with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("this artifact".to_string()),
        ),
    );
    let first = Effect::with_id(
        7,
        Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            source.clone(),
            Effect::deal_damage(
                Value::CountersOnSource(CounterType::Charge),
                ChooseSpec::target_player(),
            ),
        )),
    );
    let second = Effect::new(crate::effects::ExecuteWithSourceEffect::new(
        source,
        Effect::deal_damage(
            Value::EffectValue(crate::effect::EffectId(7)),
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()))
                .with_count(ChoiceCount::up_to(1)),
        ),
    ));

    assert_eq!(
        describe_effect_list(&[first, second]),
        "This artifact deals damage equal to the number of charge counters on it to target player and that much damage to up to one target creature"
    );
}

pub(super) fn keyword_and_unblockable_effects(
    target_filter: ObjectFilter,
    ability: crate::static_abilities::StaticAbility,
) -> Vec<Effect> {
    let target_tag = TagKey::from("targeted_0");
    let mut grant = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(ability),
        Until::EndOfTurn,
    );
    grant.target_spec = Some(ChooseSpec::target(ChooseSpec::Object(target_filter)));
    let target_only = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::Tagged(
        target_tag.clone(),
    )))
    .tag(TagKey::from("targeted_1"));
    let unblockable = Effect::new(crate::effects::CantEffect::until_end_of_turn(
        crate::effect::Restriction::be_blocked(ObjectFilter::tagged(target_tag)),
    ));

    vec![
        Effect::new(grant).tag(TagKey::from("granted_0")),
        target_only,
        unblockable,
    ]
}

#[test]
pub(super) fn describe_effect_list_compacts_keyword_then_unblockable_same_target() {
    let effects = keyword_and_unblockable_effects(
        ObjectFilter::creature().controlled_by(PlayerFilter::You),
        crate::static_abilities::StaticAbility::haste(),
    );

    assert_eq!(
        describe_effect_list(&effects),
        "Target creature you control gains haste until end of turn and can't be blocked this turn"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_lifelink_then_unblockable_power_limited_target() {
    let effects = keyword_and_unblockable_effects(
        ObjectFilter::creature()
            .controlled_by(PlayerFilter::You)
            .with_power(crate::filter::Comparison::LessThanOrEqual(2)),
        crate::static_abilities::StaticAbility::lifelink(),
    );

    assert_eq!(
        describe_effect_list(&effects),
        "Target creature with power 2 or less you control gains lifelink until end of turn and can't be blocked this turn"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_tap_defending_creature_then_goad_same_tagged_target() {
    let mut target_filter = ObjectFilter::creature();
    target_filter.controller = Some(PlayerFilter::Defending);
    let tagged_tap = Effect::tap(ChooseSpec::Object(target_filter)).tag(TagKey::from("tapped_0"));
    let goad = Effect::goad(ChooseSpec::Tagged(TagKey::from("tapped_0")));

    assert_eq!(
        describe_effect_list(&[
            Effect::tag_triggering_object("triggering"),
            tagged_tap,
            goad
        ]),
        "Tap target creature that player controls and goad it"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_targeted_haste_then_role_attachment() {
    let target =
        ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())).with_count(ChoiceCount {
            min: 1,
            max: Some(2),
            dynamic_x: false,
            up_to_x: false,
            random: false,
        });
    let haste =
        Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            target,
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::haste(),
            ),
            Until::EndOfTurn,
        ))
        .tag("granted_0");

    let created_tag = TagKey::from("created_1");
    let create = Effect::create_tokens_player(
        crate::cards::tokens::monster_role_token_definition(),
        1,
        PlayerFilter::IteratedPlayer,
    )
    .tag(created_tag.clone());
    let attach = Effect::attach_objects(ChooseSpec::Tagged(created_tag), ChooseSpec::Iterated);
    let role = Effect::for_each_tagged("targeted_0", vec![create, attach]);

    assert_eq!(
        describe_effect_list(&[haste, role]),
        "One or two target creatures each gain haste until end of turn. For each of those creatures, create a Monster Role token attached to it"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_attached_creature_sacrifice_then_token() {
    let tag = TagKey::from("enchanted");
    let tag_attached = Effect::new(crate::effects::TagAttachedToSourceEffect::new(tag.clone()));
    let mut enchanted = ObjectFilter::creature();
    enchanted.tagged_constraints.push(TaggedObjectConstraint {
        tag,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let sacrifice = Effect::new(crate::effects::SacrificeTargetEffect::new(
        ChooseSpec::Object(enchanted),
    ));
    let create = Effect::create_tokens(crate::cards::tokens::walker_token_definition(), 1);

    assert_eq!(
        describe_effect_list(&[tag_attached, sacrifice, create]),
        "enchanted creature's controller sacrifices it and you create 2/2 black Zombie creature token named Walker"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_tagged_token_copy_then_sacrifice() {
    let created = TagKey::from("created_0");
    let create = Effect::new(crate::effects::CreateTokenCopyEffect::new(
        ChooseSpec::Tagged(TagKey::from("triggering")),
        1,
        PlayerFilter::You,
    ))
    .tag(created.clone());
    let sacrifice = Effect::new(crate::effects::SacrificeTargetEffect::new(
        ChooseSpec::Tagged(created),
    ));

    assert_eq!(
        describe_effect_list(&[
            Effect::tag_triggering_object("triggering"),
            create,
            sacrifice
        ]),
        "Create a token that's a copy of it. Sacrifice that token"
    );
}

#[test]
pub(super) fn describe_token_copy_preserves_typed_antecedent_surface() {
    let target = ChooseSpec::Tagged(TagKey::from("triggering")).with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("that creature".to_string()),
        ),
    );
    let create = Effect::new(crate::effects::CreateTokenCopyEffect::new(
        target,
        1,
        PlayerFilter::You,
    ));

    assert_eq!(
        describe_effect(&create),
        "Create a token that's a copy of that creature"
    );
}

#[test]
pub(super) fn describe_destroy_random_countered_permanent_uses_those_permanents() {
    let mut filter = ObjectFilter::permanent().controlled_by(PlayerFilter::NotYou);
    filter.with_counter = Some(crate::filter::CounterConstraint::Typed(
        crate::object::CounterType::Named("aim"),
    ));
    let target = ChooseSpec::all(filter).with_count(ChoiceCount::exactly(1).at_random());

    assert_eq!(
        describe_effect(&Effect::new(crate::effects::DestroyEffect::with_spec(
            target
        ))),
        "Destroy one of those permanents at random"
    );
}

#[test]
pub(super) fn describe_for_players_choose_then_exile_compacts_iterated_move_targets() {
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature()
            .controlled_by(PlayerFilter::IteratedPlayer)
            .in_zone(Zone::Battlefield),
        ChoiceCount::exactly(1),
        PlayerFilter::IteratedPlayer,
        TagKey::from("__it__"),
    )
    .in_zone(Zone::Battlefield);
    let move_to_zone =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Exile, true);
    let for_players = crate::effects::ForPlayersEffect::new(
        PlayerFilter::Opponent,
        vec![Effect::new(choose), Effect::new(move_to_zone)],
    );

    let compact = describe_for_players_choose_then_exile(&for_players)
        .expect("for-player choose/exile should compact");
    assert_eq!(
        compact,
        "Each opponent chooses a creature they control and exiles it"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_each_opponent_graveyard_choice_to_your_battlefield() {
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature()
            .owned_by(PlayerFilter::IteratedPlayer)
            .in_zone(Zone::Graveyard),
        ChoiceCount::exactly(1),
        PlayerFilter::IteratedPlayer,
        TagKey::from("__it__"),
    )
    .in_zone(Zone::Graveyard);
    let for_players =
        crate::effects::ForPlayersEffect::new(PlayerFilter::Opponent, vec![Effect::new(choose)]);
    let move_to_zone = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(TagKey::from("__it__")),
        Zone::Battlefield,
        false,
    )
    .under_you_control();
    let effects = vec![
        Effect::new(for_players),
        Effect::new(move_to_zone).tag(TagKey::from("moved_0")),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Each opponent chooses a creature card in their graveyard. Put those cards onto the battlefield under your control"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_for_each_player_graveyard_choice_with_decayed_mods() {
    let chosen = TagKey::from("__it__");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature()
            .owned_by(PlayerFilter::IteratedPlayer)
            .in_zone(Zone::Graveyard),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        chosen.clone(),
    )
    .in_zone(Zone::Graveyard);
    let for_players =
        crate::effects::ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(choose)]);
    let move_to_zone = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(chosen.clone()),
        Zone::Battlefield,
        false,
    )
    .under_you_control();

    let mut add_black = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddColors(crate::color::ColorSet::BLACK),
        Until::Forever,
    );
    add_black.target_spec = Some(ChooseSpec::Tagged(chosen.clone()));

    let mut add_zombie = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddSubtypes(vec![Subtype::Zombie]),
        Until::Forever,
    );
    add_zombie.target_spec = Some(ChooseSpec::Tagged(chosen.clone()));

    let mut add_decayed = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::keyword_marker("decayed"),
        ),
        Until::Forever,
    );
    add_decayed.target_spec = Some(ChooseSpec::Tagged(chosen));
    add_decayed
        .additional_modifications
        .push(crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::cant_block(),
        ));

    let effects = vec![
        Effect::new(for_players),
        Effect::new(move_to_zone).tag(TagKey::from("moved_0")),
        Effect::new(add_black).tag(TagKey::from("colored_1")),
        Effect::new(add_zombie).tag(TagKey::from("subtyped_2")),
        Effect::new(add_decayed).tag(TagKey::from("granted_3")),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "For each player, choose a creature card in that player's graveyard. Put those cards onto the battlefield under your control. They're black Zombies in addition to their other colors and types and they gain decayed"
    );
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(
            "For each player, choose a creature card in that player's graveyard. Put those cards onto the battlefield under your control. They're black Zombies in addition to their other colors and types and they gain decayed"
        )
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_counted_looked_battlefield_rest_bottom() {
    let looked = TagKey::from("looked_0");
    let chosen = TagKey::from("chosen_0");
    let look = crate::effects::LookAtTopCardsEffect::new(
        PlayerFilter::You,
        Value::Fixed(7),
        looked.clone(),
    );
    let mut choose_filter = ObjectFilter::default()
        .with_type(CardType::Planeswalker)
        .in_zone(Zone::Library);
    choose_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let choose = crate::effects::ChooseObjectsEffect::new(
        choose_filter,
        ChoiceCount::up_to(2),
        PlayerFilter::You,
        chosen.clone(),
    )
    .in_zone(Zone::Library);
    let move_chosen = Effect::for_each_tagged(
        chosen.clone(),
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Battlefield,
            false,
        ))],
    );
    let rest = crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
        looked,
        Some(chosen),
        crate::effects::consult_helpers::LibraryBottomOrder::Random,
        PlayerFilter::You,
    );
    let effects = vec![
        Effect::new(look),
        Effect::new(choose),
        move_chosen,
        Effect::new(rest),
    ];
    let expected = "Look at the top seven cards of your library. Put up to two planeswalker cards from among them onto the battlefield. Put the rest on the bottom of your library in a random order";

    assert_eq!(describe_effect_list(&effects), expected);
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
}

#[test]
pub(super) fn target_opponent_reveal_choice_and_remainder_stay_one_collection_clause() {
    let revealed = TagKey::from("revealed");
    let chosen = TagKey::from("chosen");
    let target = Effect::new(crate::effects::TargetOnlyEffect::new(
        ChooseSpec::target_opponent(),
    ));
    let reveal = Effect::new(crate::effects::LookAtTopCardsEffect::revealing(
        PlayerFilter::target_opponent(),
        Value::X,
        revealed.clone(),
    ));

    let mut filter = ObjectFilter::permanent_card().in_zone(Zone::Library);
    filter.excluded_card_types.push(CardType::Land);
    filter.set_explicit_card_noun(true);
    filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqualExpr(Box::new(
        Value::X,
    )));
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: revealed.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            filter,
            ChoiceCount::up_to(1),
            PlayerFilter::You,
            chosen.clone(),
        )
        .in_zone(Zone::Library),
    );
    let move_chosen = Effect::new(crate::effects::ForEachTaggedEffect::new(
        chosen.clone(),
        vec![Effect::new(
            crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Battlefield, false)
                .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
                .under_you_control(),
        )],
    ));
    let remainder = Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            revealed,
            Some(chosen),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent)),
        ),
    );
    let effects = vec![target, reveal, choose, move_chosen, remainder];

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(
            "target opponent reveals the top X cards of their library. You may put a nonland permanent card with mana value X or less from among them onto the battlefield under your control. That player puts the rest on the bottom of their library in a random order"
        )
    );
}

#[test]
pub(super) fn describe_effect_clause_list_compacts_may_wrapped_looked_choice_then_shuffle() {
    let looked = TagKey::from("looked_0");
    let chosen = TagKey::from("chosen_0");
    let look = crate::effects::LookAtTopCardsEffect::new(
        PlayerFilter::You,
        Value::Fixed(5),
        looked.clone(),
    );
    let mut choose_filter = ObjectFilter::creature().in_zone(Zone::Library);
    choose_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked,
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            choose_filter,
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            chosen.clone(),
        )
        .in_zone(Zone::Library),
    );
    let move_chosen = Effect::for_each_tagged(
        chosen,
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Battlefield,
            false,
        ))],
    );
    let effects = vec![
        Effect::new(look),
        Effect::may_player(PlayerFilter::You, vec![choose, move_chosen]),
        Effect::shuffle_library_player(PlayerFilter::You),
    ];

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(
            "Look at the top five cards of your library. You may put a creature card from among them onto the battlefield. Then shuffle"
        )
    );
}

#[test]
pub(super) fn lowered_looked_group_moves_compact_two_stage_selection_and_graveyard_remainder() {
    let looked = TagKey::from("looked_0");
    let hand_choice = TagKey::from("hand_choice_0");
    let land_choice = TagKey::from("land_choice_0");
    let kept = TagKey::from("kept_0");
    let look = Effect::new(crate::effects::LookAtTopCardsEffect::new(
        PlayerFilter::You,
        Value::EventValue(EventValueSpec::Amount),
        looked.clone(),
    ));

    let mut hand_filter = ObjectFilter::default().in_zone(Zone::Library);
    hand_filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let choose_hand = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            hand_filter,
            ChoiceCount::up_to(1),
            PlayerFilter::You,
            hand_choice.clone(),
        )
        .in_zone(Zone::Library),
    );
    let move_hand = Effect::for_each_tagged(
        hand_choice.clone(),
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Hand,
            false,
        ))],
    )
    .tag_all(kept.clone());

    let mut land_filter = ObjectFilter::default()
        .with_type(CardType::Land)
        .in_zone(Zone::Library);
    land_filter.tagged_constraints.extend([
        TaggedObjectConstraint {
            tag: looked.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        },
        TaggedObjectConstraint {
            tag: hand_choice,
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        },
    ]);
    let choose_lands = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            land_filter,
            ChoiceCount::any_number(),
            PlayerFilter::You,
            land_choice.clone(),
        )
        .in_zone(Zone::Library),
    );
    let mut put_lands =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Battlefield, false);
    put_lands.enters_tapped = true;
    let move_lands =
        Effect::for_each_tagged(land_choice, vec![Effect::new(put_lands)]).tag_all(kept.clone());

    let mut iterated_is_kept = ObjectFilter::default();
    iterated_is_kept
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from("__it__"),
            relation: TaggedOpbjectRelation::SameStableId,
        });
    let rest = Effect::for_each_tagged(
        looked,
        vec![Effect::conditional(
            Condition::TaggedObjectMatches(kept, iterated_is_kept),
            Vec::new(),
            vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Iterated,
                Zone::Graveyard,
                false,
            ))],
        )],
    );
    let effects = vec![look, choose_hand, move_hand, choose_lands, move_lands, rest];
    let expected = "Look at that many cards from the top of your library. You may put one of them into your hand. Then put any number of land cards from among them onto the battlefield tapped and the rest into your graveyard";

    assert_eq!(describe_effect_list(&effects), expected);
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_linked_graveyard_choices_then_may_return() {
    let tag = TagKey::from("__it__");
    let first_choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature()
            .owned_by(PlayerFilter::Opponent)
            .in_zone(Zone::Graveyard),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Graveyard);
    let second_choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature()
            .owned_by(PlayerFilter::You)
            .in_zone(Zone::Graveyard),
        ChoiceCount::exactly(1),
        PlayerFilter::AliasedOwnerOf(crate::filter::ObjectRef::Tagged(tag.clone())),
        tag.clone(),
    )
    .in_zone(Zone::Graveyard);
    let may_return = Effect::may_player(
        PlayerFilter::You,
        vec![Effect::new(
            crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(tag),
                Zone::Battlefield,
                false,
            )
            .under_owner_control(),
        )],
    );
    let effects = vec![
        Effect::new(first_choose),
        Effect::new(second_choose),
        may_return,
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Choose a creature card in an opponent's graveyard, then that player chooses a creature card in your graveyard. You may return those cards to the battlefield under their owners' control"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_player_exile_creature_and_graveyard() {
    let player = PlayerFilter::target_opponent();
    let tag = TagKey::from("__it__");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature().controlled_by(player.clone()),
        ChoiceCount::exactly(1),
        player.clone(),
        tag.clone(),
    )
    .in_zone(Zone::Battlefield);
    let exile_chosen =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Exile, false);
    let graveyard = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(player);
    let exile_graveyard = crate::effects::ExileEffect::with_spec(ChooseSpec::All(graveyard));
    let effects = vec![
        Effect::new(choose),
        Effect::new(exile_chosen),
        Effect::new(exile_graveyard).tag(tag),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target opponent exiles a creature they control and their graveyard"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_two_optional_nonland_exile_targets() {
    fn nonland_permanent_filter(zone: Zone) -> ObjectFilter {
        let mut filter = ObjectFilter::nonland_permanent().in_zone(zone);
        filter.controller = Some(PlayerFilter::Any);
        filter.card_types = vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Land,
            CardType::Planeswalker,
            CardType::Battle,
        ];
        filter
    }

    let tag = TagKey::from("__sentence_helper_exiled_0");
    let battlefield_choose = crate::effects::ChooseObjectsEffect::new(
        nonland_permanent_filter(Zone::Battlefield),
        ChoiceCount::up_to(1),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Battlefield);
    let graveyard_choose = crate::effects::ChooseObjectsEffect::new(
        nonland_permanent_filter(Zone::Graveyard),
        ChoiceCount::up_to(1),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Graveyard);
    let exile_chosen =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Tagged(tag), Zone::Exile, false);
    let effects = vec![
        Effect::new(battlefield_choose),
        Effect::new(graveyard_choose),
        Effect::new(exile_chosen),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Exile up to one target nonland permanent and up to one target nonland permanent card from a graveyard"
    );
}

#[test]
pub(super) fn describe_source_exiled_cards_return_to_hand_uses_exiled_cards() {
    let mut exiled_filter = ObjectFilter::default().in_zone(Zone::Exile);
    exiled_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let effects = vec![Effect::new(crate::effects::ReturnToHandEffect::all(
        exiled_filter,
    ))];

    assert_eq!(
        describe_effect_list(&effects),
        "Return the exiled cards to their owners' hands"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_exile_then_gain_life_from_its_mana_value() {
    let exile = Effect::new(crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Object(
            ObjectFilter::permanent()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        )
        .with_count(ChoiceCount::exactly(1)),
        Zone::Exile,
        true,
    ));
    let gain = Effect::gain_life(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
        TagKey::from(crate::tag::SOURCE_EXILED_TAG),
    ))));

    assert_eq!(
        describe_effect_list(&[exile, gain]),
        "Exile a card from your graveyard. you gain life equal to its mana value"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_targeted_delayed_unblocked_trigger() {
    let tag = TagKey::from("__it__");
    let choose_filter = ObjectFilter::creature()
        .controlled_by(PlayerFilter::You)
        .in_zone(Zone::Battlefield);
    let choose = crate::effects::ChooseObjectsEffect::new(
        choose_filter.clone(),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Battlefield);

    let mut target_filter = choose_filter;
    target_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let triggering = TagKey::from("triggering");
    let delayed_effects = vec![
        Effect::may_player(
            PlayerFilter::You,
            vec![Effect::gain_life(Value::PowerOf(Box::new(
                ChooseSpec::Tagged(triggering.clone()),
            )))],
        ),
        Effect::if_then(
            crate::effect::EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::prevent_all_combat_damage_from(
                ChooseSpec::Tagged(triggering),
                Until::EndOfTurn,
            )],
        ),
    ];
    let schedule = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
        crate::triggers::Trigger::this_attacks_and_isnt_blocked(),
        delayed_effects,
        false,
        tag,
        PlayerFilter::You,
    )
    .with_target_filter(target_filter)
    .until_end_of_turn();

    let effects = vec![Effect::new(choose), Effect::new(schedule)];

    assert!(matches!(
        describe_effect_list(&effects).as_str(),
        "This turn, when target creature you control attacks and isn't blocked, you may gain life equal to its power. If you do, it assigns no combat damage this turn"
            | "This turn, when target creature you control attacks and isn't blocked, you may gain life equal to its power. If you do, prevent all combat damage that would be dealt by it this turn"
    ));
}

#[test]
pub(super) fn describe_assign_no_combat_damage_preserves_assignment_wording_and_duration() {
    let source = Effect::assign_no_combat_damage(ChooseSpec::Source, Until::EndOfTurn);
    assert_eq!(
        describe_effect_list(&[source]),
        "this creature assigns no combat damage this turn"
    );

    let tagged = Effect::assign_no_combat_damage(
        ChooseSpec::Tagged(TagKey::from("__it__")),
        Until::EndOfCombat,
    );
    assert_eq!(
        describe_effect_list(&[tagged]),
        "it assigns no combat damage this combat"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_optional_draw_then_drawn_card_choice() {
    let tag = TagKey::from("__it__");
    let may_draw = Effect::with_id(
        0,
        Effect::may_player(PlayerFilter::You, vec![Effect::draw(2)]),
    );

    let mut choose_filter = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::You);
    choose_filter.drawn_this_turn = true;
    let choose = crate::effects::ChooseObjectsEffect::new(
        choose_filter,
        ChoiceCount::exactly(2),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Hand);
    let if_chose = Effect::if_then(
        crate::effect::EffectId(0),
        EffectPredicate::Happened,
        vec![Effect::new(choose)],
    );

    let mut each_filter = ObjectFilter::default();
    each_filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let for_each = crate::effects::ForEachObject::new(
        each_filter,
        vec![Effect::unless_action(
            vec![Effect::move_to_zone(
                ChooseSpec::Tagged(tag),
                Zone::Library,
                true,
            )],
            vec![Effect::lose_life(4)],
            PlayerFilter::You,
        )],
    );

    assert_eq!(
        describe_effect_list(&[may_draw, if_chose, Effect::new(for_each)]),
        "You may draw two additional cards. If you do, choose two cards in your hand drawn this turn. For each of those cards, pay 4 life or put the card on top of your library"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_optional_draw_with_iterated_unless_alternative() {
    let tag = TagKey::from("__it__");
    let may_draw = Effect::with_id(
        0,
        Effect::may_player(PlayerFilter::You, vec![Effect::draw(2)]),
    );

    let mut choose_filter = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::You);
    choose_filter.drawn_this_turn = true;
    let choose = crate::effects::ChooseObjectsEffect::new(
        choose_filter,
        ChoiceCount::exactly(2),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Hand);
    let if_chose = Effect::if_then(
        crate::effect::EffectId(0),
        EffectPredicate::Happened,
        vec![Effect::new(choose)],
    );

    let mut each_filter = ObjectFilter::default();
    each_filter.tagged_constraints.push(TaggedObjectConstraint {
        tag,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let for_each = crate::effects::ForEachObject::new(
        each_filter,
        vec![Effect::unless_action(
            vec![Effect::lose_life(4)],
            vec![Effect::move_to_zone(
                ChooseSpec::Iterated,
                Zone::Library,
                true,
            )],
            PlayerFilter::You,
        )],
    );

    assert_eq!(
        describe_effect_list(&[may_draw, if_chose, Effect::new(for_each)]),
        "You may draw two additional cards. If you do, choose two cards in your hand drawn this turn. For each of those cards, pay 4 life or put the card on top of your library"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_chaos_mutation_shape() {
    let exiled_tag = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let revealed_tag = TagKey::from("__sentence_helper_revealed_l0_s0_e1");
    let matched_tag = TagKey::from("__sentence_helper_chosen_l0_s0_e1");
    let exile = Effect::exile_any_number(ChooseSpec::target_creature()).tag(exiled_tag.clone());
    let controller =
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(TagKey::from("__it__")));
    let for_each = crate::effects::ForEachTaggedEffect::new(
        exiled_tag,
        vec![
            Effect::consult_top_of_library(
                controller.clone(),
                crate::effects::consult_helpers::LibraryConsultMode::Reveal,
                ObjectFilter::creature().in_zone(Zone::Library),
                crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
                revealed_tag.clone(),
                matched_tag.clone(),
            ),
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(matched_tag.clone()),
                Zone::Battlefield,
                false,
            )),
            Effect::put_tagged_remainder_on_library_bottom(
                revealed_tag,
                Some(matched_tag),
                crate::effects::consult_helpers::LibraryBottomOrder::Random,
                controller,
            ),
        ],
    );
    let effects = vec![exile, Effect::new(for_each)];

    assert_eq!(
        describe_effect_list(&effects),
        "Exile any number of target creatures controlled by different players. For each creature exiled this way, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then puts the rest on the bottom of their library in a random order"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_consult_may_cast_revealed_remainder_bottom() {
    let revealed_tag = TagKey::from("__sentence_helper_revealed_l0_s0_e0");
    let matched_tag = TagKey::from("__sentence_helper_chosen_l0_s0_e0");
    let mut filter = ObjectFilter::default().in_zone(Zone::Library);
    filter.excluded_card_types.push(CardType::Land);
    filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(3));

    let consult = Effect::consult_top_of_library(
        PlayerFilter::You,
        crate::effects::consult_helpers::LibraryConsultMode::Reveal,
        filter,
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
        revealed_tag.clone(),
        matched_tag.clone(),
    );
    let may_cast = Effect::new(crate::effects::MayEffect::new(vec![Effect::new(
        crate::effects::CastTaggedEffect::new(matched_tag.clone(), PlayerFilter::You)
            .without_paying_mana_cost(),
    )]));
    let remainder = Effect::put_tagged_remainder_on_library_bottom(
        revealed_tag,
        None,
        crate::effects::consult_helpers::LibraryBottomOrder::Random,
        PlayerFilter::You,
    );
    let effects = vec![consult, may_cast, remainder];
    let expected = "you reveal cards from the top of your library until you reveal a nonland card with mana value 3 or less. you may cast that card without paying its mana cost. Put all revealed cards not cast this way on the bottom of your library in a random order";

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
    assert_eq!(describe_effect_list(&effects), expected);
}

#[test]
pub(super) fn describe_effect_list_compacts_exile_consult_battlefield_remainder_bottom() {
    let exiled_tag = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let matched_tag = TagKey::from("__sentence_helper_chosen_l0_s0_e0");
    let consult = Effect::consult_top_of_library(
        PlayerFilter::You,
        crate::effects::consult_helpers::LibraryConsultMode::Exile,
        ObjectFilter::default().with_type(CardType::Land),
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
        exiled_tag.clone(),
        matched_tag.clone(),
    );
    let move_match = Effect::new(crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(matched_tag.clone()),
        Zone::Battlefield,
        false,
    ));
    let remainder = Effect::put_tagged_remainder_on_library_bottom(
        exiled_tag,
        Some(matched_tag),
        crate::effects::consult_helpers::LibraryBottomOrder::Random,
        PlayerFilter::You,
    );

    assert_eq!(
        describe_effect_list(&[consult, move_match, remainder]),
        "you exile cards from the top of your library until you exile a land card. Put that card onto the battlefield and the rest on the bottom of your library in a random order"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_target_permanent_shuffle_reveal_permanent_card() {
    let moved_tag = TagKey::from("moved_0");
    let revealed_tag = TagKey::from("__sentence_helper_revealed_l0_s0_e1");
    let owner = PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(moved_tag.clone()));
    let move_target = crate::effects::TaggedEffect::new(
        moved_tag,
        Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::permanent_card().in_zone(Zone::Battlefield),
            )),
            Zone::Library,
            false,
        )),
    );
    let reveal = crate::effects::RevealTopEffect::tagged(owner.clone(), revealed_tag.clone());
    let conditional = crate::effects::ConditionalEffect::new(
        Condition::TaggedObjectMatches(revealed_tag.clone(), ObjectFilter::permanent_card()),
        vec![Effect::new(crate::effects::TaggedEffect::new(
            TagKey::from("moved_2"),
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(revealed_tag),
                Zone::Battlefield,
                false,
            )),
        ))],
        vec![],
    );
    let effects = vec![
        Effect::new(move_target),
        Effect::new(crate::effects::ShuffleLibraryEffect::new(owner)),
        Effect::new(reveal),
        Effect::new(conditional),
    ];
    let expected = "The owner of target permanent shuffles it into their library, then reveals the top card of their library. If it's a permanent card, they put it onto the battlefield";

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
    assert_eq!(describe_effect_list(&effects), expected);
}

#[test]
pub(super) fn describe_effect_list_compacts_multi_zone_aura_search_attach_conditional_shuffle() {
    let searched_tag = TagKey::from("searched_multi_zone");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default().with_subtype(Subtype::Aura),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        searched_tag.clone(),
    )
    .in_zones(vec![Zone::Graveyard, Zone::Hand, Zone::Library])
    .as_search();
    let move_and_attach = crate::effects::ForEachTaggedEffect::new(
        searched_tag.clone(),
        vec![
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(searched_tag.clone()),
                Zone::Battlefield,
                false,
            )),
            Effect::new(crate::effects::AttachObjectsEffect::new(
                ChooseSpec::All(ObjectFilter::tagged(searched_tag)),
                ChooseSpec::Source.with_surface_hint(
                    crate::target::ChooseSpecSurfaceHint::SourceReference(
                        crate::target::SourceReferenceSurface::ThisPermanentType(
                            "this creature".to_string(),
                        ),
                    ),
                ),
            )),
        ],
    );
    let may =
        crate::effects::MayEffect::new(vec![Effect::new(choose), Effect::new(move_and_attach)]);
    let effects = vec![
        Effect::new(may),
        Effect::new(crate::effects::ShuffleLibraryEffect::new(PlayerFilter::You)),
    ];
    let expected = "You may search your graveyard, hand, and/or library for an Aura card and put it onto the battlefield attached to this creature. If you search your library this way, shuffle";

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
    assert_eq!(describe_effect_list(&effects), expected);
}

#[test]
pub(super) fn describe_effect_list_compacts_nonbasic_name_multi_zone_search_reward() {
    let chosen_name_tag = TagKey::from("__chosen_name__");
    let searched_tag = TagKey::from("searched_multi_zone");

    let mut nonbasic_land_name_filter = ObjectFilter::default();
    let mut nonland_branch = ObjectFilter::default();
    nonland_branch.excluded_card_types.push(CardType::Land);
    let mut nonbasic_branch = ObjectFilter::default();
    nonbasic_branch
        .excluded_supertypes
        .push(crate::types::Supertype::Basic);
    nonbasic_land_name_filter.any_of = vec![nonland_branch, nonbasic_branch];

    let choose_name = crate::effects::ChooseCardNameEffect::new(
        PlayerFilter::You,
        Some(nonbasic_land_name_filter),
        chosen_name_tag.clone(),
    );
    let target_opponent = crate::effects::TargetOnlyEffect::new(ChooseSpec::target_opponent());

    let mut search_filter = ObjectFilter::default().owned_by(PlayerFilter::target_opponent());
    search_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_name_tag,
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });
    let choose = crate::effects::ChooseObjectsEffect::new(
        search_filter,
        ChoiceCount::any_number(),
        PlayerFilter::You,
        searched_tag.clone(),
    )
    .in_zones(vec![Zone::Graveyard, Zone::Hand, Zone::Library])
    .as_optional_search();

    let exile = crate::effects::ForEachTaggedEffect::new(
        searched_tag.clone(),
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(searched_tag.clone()),
            Zone::Exile,
            false,
        ))],
    );
    let shuffle = crate::effects::ShuffleLibraryEffect::new(PlayerFilter::target_opponent());

    let mut count_filter = ObjectFilter::default().in_zone(Zone::Hand);
    count_filter.owner = Some(PlayerFilter::target_opponent());
    count_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: searched_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let zombie = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Zombie")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .color_indicator(crate::color::ColorSet::BLACK)
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let create = crate::effects::CreateTokenEffect::new(
        zombie,
        Value::Count(count_filter),
        PlayerFilter::target_opponent(),
    );

    let effects = vec![
        Effect::new(choose_name),
        Effect::new(target_opponent),
        Effect::new(choose),
        Effect::new(exile),
        Effect::new(shuffle),
        Effect::new(create).tag(TagKey::from("created_1")),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Choose a card name other than a basic land card name. Search target opponent's graveyard, hand, and library for any number of cards with that name and exile them. That player shuffles, then creates a 2/2 black Zombie creature token for each card exiled from their hand this way"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_chosen_name_consult_after_top_exile() {
    let chosen_name_tag = TagKey::from("__chosen_name__");
    let revealed_tag = TagKey::from("revealed");
    let matched_tag = TagKey::from("matched");

    let choose_name =
        crate::effects::ChooseCardNameEffect::new(PlayerFilter::You, None, chosen_name_tag.clone());
    let exile_top =
        crate::effects::ExileTopOfLibraryEffect::new(Value::Fixed(6), PlayerFilter::You);
    let mut consult_filter = ObjectFilter::default();
    consult_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_name_tag,
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });
    let consult = crate::effects::ConsultTopOfLibraryEffect::new(
        PlayerFilter::You,
        crate::effects::consult_helpers::LibraryConsultMode::Reveal,
        consult_filter,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)),
        revealed_tag.clone(),
        matched_tag.clone(),
    );
    let move_match = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(matched_tag.clone()),
        Zone::Hand,
        false,
    );
    let mut iterated_is_not_match = ObjectFilter::default();
    iterated_is_not_match
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: matched_tag,
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let exile_remainder = crate::effects::ForEachTaggedEffect::new(
        revealed_tag,
        vec![Effect::new(crate::effects::ConditionalEffect::new(
            Condition::TaggedObjectMatches(TagKey::from("__it__"), iterated_is_not_match),
            vec![],
            vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Iterated,
                Zone::Exile,
                true,
            ))],
        ))],
    );
    let effects = vec![
        Effect::new(choose_name),
        Effect::new(exile_top),
        Effect::new(consult),
        Effect::new(move_match).tag(TagKey::from("moved_0")),
        Effect::new(exile_remainder),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Choose a card name. Exile the top six cards of your library, then reveal cards from the top of your library until you reveal a card with the chosen name. Put that card into your hand and exile all other cards revealed this way"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_reveal_hand_choose_discard_then_random() {
    let chosen_tag = TagKey::from("__it__");
    let look = crate::effects::LookAtHandEffect::reveal(ChooseSpec::target_opponent());
    let aliased_opponent = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent));
    let mut choice_filter = ObjectFilter::default().in_zone(Zone::Hand);
    choice_filter.owner = Some(aliased_opponent.clone());
    let choose = crate::effects::ChooseObjectsEffect::new(
        choice_filter,
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        chosen_tag.clone(),
    )
    .in_zone(Zone::Hand);
    let mut discard_filter = ObjectFilter::default().in_zone(Zone::Hand);
    discard_filter.owner = Some(aliased_opponent.clone());
    discard_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_tag,
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let discard_chosen = crate::effects::DiscardEffect::new_with_filter(
        Value::Fixed(1),
        aliased_opponent.clone(),
        false,
        Some(discard_filter),
    );
    let discard_random =
        crate::effects::DiscardEffect::new(Value::Fixed(1), aliased_opponent, true);

    let effects = vec![
        Effect::new(look),
        Effect::new(choose),
        Effect::new(discard_chosen),
        Effect::new(discard_random),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target opponent reveals their hand. You choose a card from it. That player discards that card, then discards a card at random"
    );
}

#[test]
pub(super) fn describe_draw_for_each_tagged_matching_preserves_hand_origin() {
    let tag = TagKey::from("searched_multi_zone");
    let mut filter = ObjectFilter::default().in_zone(Zone::Hand);
    filter.owner = Some(PlayerFilter::target_opponent());
    let draw = crate::effects::DrawForEachTaggedMatchingEffect::new(
        PlayerFilter::target_opponent(),
        tag,
        filter,
    );

    assert_eq!(
        describe_effect_list(&[Effect::new(draw)]),
        "target opponent draws a card for each card exiled from their hand this way"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_source_exiled_return_with_counters() {
    let moved_tag = TagKey::from("moved_0");
    let mut exiled_filter = ObjectFilter::creature().in_zone(Zone::Exile);
    exiled_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let move_to_zone = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Object(exiled_filter).with_count(ChoiceCount::exactly(1)),
        Zone::Battlefield,
        false,
    )
    .under_you_control();
    let effects = vec![
        Effect::sacrifice_source(),
        Effect::new(move_to_zone).tag(moved_tag.clone()),
        Effect::put_counters(
            crate::object::CounterType::PlusOnePlusOne,
            Value::Fixed(2),
            ChooseSpec::Tagged(moved_tag),
        ),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Sacrifice this enchantment, then put a creature card exiled with it onto the battlefield under your control with two additional +1/+1 counters on it"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_source_graveyard_return_with_counters() {
    let returned = TagKey::from("returned_0");
    let return_effect = Effect::return_from_graveyard_to_battlefield(ChooseSpec::Source, true)
        .tag(returned.clone())
        .tag(TagKey::from("returned_1"));
    let counters = Effect::put_counters(
        crate::object::CounterType::PlusOnePlusOne,
        Value::Fixed(2),
        ChooseSpec::Tagged(returned),
    );

    assert_eq!(
        describe_effect_list(&[return_effect, counters]),
        "Return this card from your graveyard to the battlefield tapped with two +1/+1 counters on it"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_target_graveyard_return_with_additional_counters() {
    let returned = TagKey::from("returned_0");
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You),
    ));
    let return_effect =
        Effect::return_from_graveyard_to_battlefield(target, false).tag(returned.clone());
    let counters = Effect::put_counters(
        crate::object::CounterType::PlusOnePlusOne,
        Value::Fixed(2),
        ChooseSpec::Tagged(returned),
    )
    .tag(TagKey::from("counters_1"));

    assert_eq!(
        describe_effect_list(&[return_effect, counters]),
        "Return target creature card from your graveyard to the battlefield with two additional +1/+1 counters on it"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_created_role_token_attachment() {
    let created = TagKey::from("created_0");
    let role = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cursed Role")
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Role])
        .build();
    let create = Effect::new(crate::effects::CreateTokenEffect::new(
        role,
        Value::Fixed(1),
        PlayerFilter::You,
    ))
    .tag(created.clone());
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature().controlled_by(PlayerFilter::You),
    ))
    .with_count(ChoiceCount::up_to(1));
    let attach = Effect::new(crate::effects::AttachObjectsEffect::new(
        ChooseSpec::Tagged(created),
        target,
    ));

    assert_eq!(
        describe_effect_list(&[create, attach]),
        "Create a Cursed Role token attached to up to one target creature you control"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_put_onto_battlefield_attached() {
    let moved_tag = TagKey::from("moved_0");
    let move_to_zone = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Object(
            ObjectFilter::default()
                .with_subtype(Subtype::Aura)
                .in_zone(Zone::Hand)
                .owned_by(PlayerFilter::You),
        )
        .with_count(ChoiceCount::exactly(1)),
        Zone::Battlefield,
        false,
    );
    let mut moved_filter = ObjectFilter::default();
    moved_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: moved_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let effects = vec![
        Effect::new(move_to_zone).tag(moved_tag),
        Effect::new(crate::effects::AttachObjectsEffect::new(
            ChooseSpec::All(moved_filter),
            ChooseSpec::Source,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Put an Aura card in your hand onto the battlefield attached to this source"
    );
}

#[test]
pub(super) fn describe_attach_any_number_equipment_uses_non_target_plural_surface() {
    let attach = Effect::new(crate::effects::AttachObjectsEffect::new(
        ChooseSpec::All(
            ObjectFilter::default()
                .with_subtype(Subtype::Equipment)
                .controlled_by(PlayerFilter::You),
        )
        .with_count(ChoiceCount::any_number()),
        ChooseSpec::Tagged(TagKey::from("__it__")),
    ));

    assert_eq!(
        describe_effect_list(&[attach]),
        "Attach any number of Equipment you control to it"
    );
}

#[test]
pub(super) fn describe_continuous_choose_attach_sequence_preserves_sentence_boundaries() {
    let vehicle_filter = ObjectFilter::default()
        .with_subtype(Subtype::Vehicle)
        .controlled_by(PlayerFilter::You);
    let mut animate = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Filter(vehicle_filter.clone()),
        crate::continuous::Modification::AddCardTypes(vec![CardType::Artifact, CardType::Creature]),
        Until::EndOfTurn,
    );
    animate.target_spec = Some(ChooseSpec::Object(vehicle_filter));

    let chosen = TagKey::from("__it__");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .with_subtype(Subtype::Dwarf)
            .controlled_by(PlayerFilter::You)
            .in_zone(Zone::Battlefield),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        chosen.clone(),
    )
    .in_zone(Zone::Battlefield);
    let attach = crate::effects::AttachObjectsEffect::new(
        ChooseSpec::All(
            ObjectFilter::default()
                .with_subtype(Subtype::Equipment)
                .controlled_by(PlayerFilter::You),
        )
        .with_count(ChoiceCount::any_number()),
        ChooseSpec::Tagged(chosen),
    );

    let effects = vec![
        Effect::new(animate),
        Effect::new(choose),
        Effect::new(attach),
    ];
    let expected = "Vehicles you control become artifact creatures until end of turn. Choose a Dwarf you control. Attach any number of Equipment you control to it";

    assert_eq!(describe_effect_list(&effects), expected);
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_search_reveal_move_then_shuffle() {
    let tag = TagKey::from("searched");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::land()
            .with_subtype(Subtype::Plains)
            .in_zone(Zone::Library),
        ChoiceCount::up_to_dynamic_x(),
        PlayerFilter::You,
        tag.clone(),
    )
    .with_count_value(Value::PlayersWhoControlMoreThanYou(ObjectFilter::land()))
    .in_zone(Zone::Library)
    .as_optional_search();
    let effects = vec![
        Effect::new(choose),
        Effect::new(crate::effects::RevealTaggedEffect::new(tag.clone())),
        Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(tag),
            Zone::Hand,
            false,
        )),
        Effect::new(crate::effects::ShuffleLibraryEffect::new(PlayerFilter::You)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Search your library for up to X Plains cards, where X is the number of players who control more lands than you. Reveal those cards, put them into your hand, then shuffle"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_search_two_split_hand_graveyard() {
    let searched = TagKey::from("searched");
    let hand = TagKey::from("hand");
    let search = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default().in_zone(Zone::Library),
        ChoiceCount::exactly(2),
        PlayerFilter::You,
        searched.clone(),
    )
    .in_zone(Zone::Library)
    .as_search();
    let mut hand_filter = ObjectFilter::default().in_zone(Zone::Library);
    hand_filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: searched.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let choose_hand = crate::effects::ChooseObjectsEffect::new(
        hand_filter,
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        hand.clone(),
    )
    .in_zone(Zone::Library);
    let rest = crate::effects::ForEachTaggedEffect::new(
        searched,
        vec![Effect::new(crate::effects::ConditionalEffect::new(
            Condition::TaggedObjectMatches(
                hand.clone(),
                ObjectFilter::default().same_stable_id_as_tagged(TagKey::from("__it__")),
            ),
            vec![],
            vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Iterated,
                Zone::Graveyard,
                false,
            ))],
        ))],
    );
    let effects = vec![
        Effect::new(search),
        Effect::new(choose_hand),
        Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(hand),
            Zone::Hand,
            false,
        )),
        Effect::new(rest),
        Effect::new(crate::effects::ShuffleLibraryEffect::new(PlayerFilter::You)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Search your library for two cards. Put one into your hand and the other into your graveyard. Then shuffle"
    );
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(
            "Search your library for two cards. Put one into your hand and the other into your graveyard. Then shuffle"
        )
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_optional_search_battlefield_hand_partition() {
    let searched = TagKey::from("searched_split");
    let battlefield = TagKey::from("searched_split_battlefield");
    let mut search_filter = ObjectFilter::land()
        .with_supertype(Supertype::Basic)
        .in_zone(Zone::Library);
    search_filter.owner = Some(PlayerFilter::You);
    let search = crate::effects::ChooseObjectsEffect::new(
        search_filter,
        ChoiceCount::up_to(2),
        PlayerFilter::You,
        searched.clone(),
    )
    .in_zone(Zone::Library)
    .as_optional_search();
    let choose_battlefield = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::tagged(searched.clone()).in_zone(Zone::Library),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        battlefield.clone(),
    )
    .in_zone(Zone::Library);
    let put_battlefield = crate::effects::ForEachTaggedEffect::new(
        battlefield.clone(),
        vec![Effect::new(
            crate::effects::PutOntoBattlefieldEffect::you_control(ChooseSpec::Iterated, true),
        )],
    );
    let put_hand = crate::effects::ForEachTaggedEffect::new(
        searched.clone(),
        vec![Effect::new(crate::effects::ConditionalEffect::new(
            Condition::TaggedObjectMatches(
                battlefield,
                ObjectFilter::default().same_stable_id_as_tagged(TagKey::from("__it__")),
            ),
            vec![],
            vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Iterated,
                Zone::Hand,
                false,
            ))],
        ))],
    );
    let effects = vec![
        Effect::new(search),
        Effect::new(crate::effects::RevealTaggedEffect::new(searched)),
        Effect::new(choose_battlefield),
        Effect::new(put_battlefield),
        Effect::new(put_hand),
        Effect::new(crate::effects::ShuffleLibraryEffect::new(PlayerFilter::You)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle"
    );

    let mut with_scry = effects;
    with_scry.push(Effect::new(crate::effects::ScryEffect::you(1)));
    assert_eq!(
        describe_effect_list(&with_scry),
        "Search your library for up to two basic land cards, reveal those cards, and put one onto the battlefield tapped and the other into your hand. Shuffle, then scry 1"
    );
}

#[test]
pub(super) fn unselected_remainder_accepts_iterated_membership_condition_orientation() {
    let looked = TagKey::from("revealed");
    let chosen = TagKey::from("matched");
    let rest = crate::effects::ForEachTaggedEffect::new(
        looked.clone(),
        vec![Effect::new(crate::effects::ConditionalEffect::new(
            Condition::TaggedObjectMatches(
                TagKey::from("__it__"),
                ObjectFilter::default()
                    .match_tagged(chosen.clone(), TaggedOpbjectRelation::IsTaggedObject),
            ),
            vec![],
            vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Iterated,
                Zone::Graveyard,
                false,
            ))],
        ))],
    );

    assert!(for_each_moves_unselected_to_zone(
        &rest,
        looked.as_str(),
        chosen.as_str(),
        Zone::Graveyard,
    ));
}

#[test]
pub(super) fn describe_effect_list_compacts_search_two_split_with_broad_chosen_zone() {
    let searched = TagKey::from("searched");
    let hand = TagKey::from("hand");
    let search = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default().in_zone(Zone::Library),
        ChoiceCount::exactly(2),
        PlayerFilter::You,
        searched.clone(),
    )
    .in_zone(Zone::Library)
    .as_search();
    let hand_filter = ObjectFilter::default()
        .in_zone(Zone::Library)
        .match_tagged(searched.clone(), TaggedOpbjectRelation::IsTaggedObject);
    let choose_hand = crate::effects::ChooseObjectsEffect::new(
        hand_filter.clone(),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        hand.clone(),
    )
    .in_zones(vec![
        Zone::Battlefield,
        Zone::Hand,
        Zone::Graveyard,
        Zone::Library,
        Zone::Exile,
    ]);
    let rest = crate::effects::ForEachTaggedEffect::new(
        searched,
        vec![Effect::new(crate::effects::ConditionalEffect::new(
            Condition::TaggedObjectMatches(
                hand.clone(),
                ObjectFilter::default().same_stable_id_as_tagged(TagKey::from("__it__")),
            ),
            vec![],
            vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Iterated,
                Zone::Graveyard,
                false,
            ))],
        ))],
    );
    let effects = vec![
        Effect::new(search),
        Effect::new(choose_hand),
        Effect::new(crate::effects::TaggedEffect::new(
            TagKey::from("moved_0"),
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(hand),
                Zone::Hand,
                false,
            )),
        )),
        Effect::new(rest),
        Effect::new(crate::effects::ShuffleLibraryEffect::new(PlayerFilter::You)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Search your library for two cards. Put one into your hand and the other into your graveyard. Then shuffle"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_reveal_top_choice_rest_graveyard_structurally() {
    let looked = TagKey::from("looked");
    let chosen = TagKey::from("chosen");
    let mut choice_filter =
        ObjectFilter::default().match_tagged(looked.clone(), TaggedOpbjectRelation::IsTaggedObject);
    choice_filter.any_of = vec![
        ObjectFilter::default().with_type(CardType::Creature),
        ObjectFilter::default().with_type(CardType::Land),
    ];
    let choose = crate::effects::ChooseObjectsEffect::new(
        choice_filter,
        ChoiceCount::up_to(1),
        PlayerFilter::You,
        chosen.clone(),
    )
    .in_zone(Zone::Library);
    let return_chosen = crate::effects::ForEachTaggedEffect::new(
        chosen.clone(),
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Hand,
            false,
        ))],
    );
    let move_rest = crate::effects::ForEachTaggedEffect::new(
        looked.clone(),
        vec![Effect::new(crate::effects::ConditionalEffect::new(
            Condition::TaggedObjectMatches(
                chosen,
                ObjectFilter::default().same_stable_id_as_tagged(TagKey::from("__it__")),
            ),
            vec![],
            vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Iterated,
                Zone::Graveyard,
                false,
            ))],
        ))],
    );
    let effects = vec![
        Effect::new(crate::effects::LookAtTopCardsEffect::new(
            PlayerFilter::You,
            Value::Fixed(5),
            looked.clone(),
        )),
        Effect::new(crate::effects::RevealTaggedEffect::new(looked)),
        Effect::new(choose),
        Effect::new(return_chosen),
        Effect::new(move_rest),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Reveal the top five cards of your library. You may put a creature or land card from among them into your hand. Put the rest into your graveyard"
    );
}

#[test]
pub(super) fn structural_provoke_keyword_uses_trigger_and_effect_shape() {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature().controlled_by(PlayerFilter::Defending),
    ));
    let mut must_block = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::must_block(),
        ),
        Until::EndOfCombat,
    );
    must_block.target_spec = Some(target.clone());
    let triggered = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::this_attacks(),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::new(crate::effects::UntapEffect::with_spec(target.clone())),
            Effect::new(must_block),
        ]),
        choices: vec![target],
        intervening_if: None,
        presentation_label: None,
    };

    assert_eq!(
        describe_structural_provoke_keyword(&triggered),
        Some("Provoke".to_string())
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_gain_control_untap_haste_structurally() {
    let controlled = TagKey::from("controlled_0");
    let untapped = TagKey::from("untapped_1");
    let mut control = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(crate::static_abilities::StaticAbility::haste()),
        Until::EndOfTurn,
    );
    control.target_spec = Some(ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature().in_zone(Zone::Battlefield),
    )));
    control.modification = None;
    control.runtime_modifications =
        vec![crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController];

    let mut haste = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(crate::static_abilities::StaticAbility::haste()),
        Until::EndOfTurn,
    );
    haste.target_spec = Some(ChooseSpec::Tagged(untapped.clone()).with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("it".to_string()),
        ),
    ));

    let untap_target = ChooseSpec::Object(
        ObjectFilter::creature()
            .in_zone(Zone::Battlefield)
            .match_tagged(controlled.clone(), TaggedOpbjectRelation::IsTaggedObject),
    )
    .with_surface_hint(crate::target::ChooseSpecSurfaceHint::SourceReference(
        crate::target::SourceReferenceSurface::ThisPermanentType("that creature".to_string()),
    ));

    let effects = vec![
        Effect::new(crate::effects::TaggedEffect::new(
            controlled.clone(),
            Effect::new(control),
        )),
        Effect::new(crate::effects::TaggedEffect::new(
            untapped,
            Effect::new(crate::effects::UntapEffect::with_spec(untap_target)),
        )),
        Effect::new(crate::effects::TaggedEffect::new(
            "granted",
            Effect::new(haste),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn"
    );
}

#[test]
pub(super) fn draw_effect_preserves_fixed_and_dynamic_additional_card_surfaces() {
    let fixed = Effect::new(crate::effects::DrawCardsEffect::you(
        Value::Fixed(2).with_surface_hint(ValueSurfaceHint::AdditionalCards),
    ));
    assert_eq!(describe_effect(&fixed), "Draw two additional cards");

    let source = ChooseSpec::Source.with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ShortName("ED-E".to_string()),
        ),
    );
    let dynamic = Effect::new(crate::effects::DrawCardsEffect::you(
        Value::CountersOn(Box::new(source), Some(CounterType::Quest))
            .with_surface_hint(ValueSurfaceHint::AdditionalCards),
    ));
    assert_eq!(
        describe_effect(&dynamic),
        "Draw an additional card for each quest counter on ED-E"
    );
}

#[test]
pub(super) fn draw_then_additional_draw_keeps_temporal_sequence_punctuation() {
    let source = ChooseSpec::Source.with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ShortName("ED-E".to_string()),
        ),
    );
    let effects = vec![
        Effect::new(crate::effects::DrawCardsEffect::you(1)),
        Effect::new(crate::effects::DrawCardsEffect::you(
            Value::CountersOn(Box::new(source), Some(CounterType::Quest))
                .with_surface_hint(ValueSurfaceHint::AdditionalCards),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Draw a card, then draw an additional card for each quest counter on ED-E"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_gain_control_then_untap_before_other_effects() {
    let controlled = TagKey::from("controlled");
    let mut control = crate::effects::ApplyContinuousEffect::new_runtime(
        crate::continuous::EffectTarget::Source,
        crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
        Until::EndOfTurn,
    );
    control.target_spec = Some(ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature(),
    )));
    let effects = vec![
        Effect::new(crate::effects::TaggedEffect::new(
            controlled.clone(),
            Effect::new(control),
        )),
        Effect::new(crate::effects::TaggedEffect::new(
            "untapped",
            Effect::new(crate::effects::UntapEffect::with_spec(ChooseSpec::Tagged(
                controlled,
            ))),
        )),
        Effect::new(crate::effects::GainLifeEffect::you(Value::Fixed(2))),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Gain control of target creature until end of turn. Untap that creature. Gain 2 life"
    );
}

#[test]
pub(super) fn describe_effect_clause_list_dispatches_gain_control_then_untap_structurally() {
    let controlled = TagKey::from("controlled");
    let mut control = crate::effects::ApplyContinuousEffect::new_runtime(
        crate::continuous::EffectTarget::Source,
        crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
        Until::EndOfTurn,
    );
    control.target_spec = Some(ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature(),
    )));
    let effects = vec![
        Effect::new(crate::effects::TaggedEffect::new(
            controlled.clone(),
            Effect::new(control),
        )),
        Effect::new(crate::effects::TaggedEffect::new(
            "untapped",
            Effect::new(crate::effects::UntapEffect::with_spec(ChooseSpec::Tagged(
                controlled,
            ))),
        )),
        Effect::new(crate::effects::GainLifeEffect::you(Value::Fixed(2))),
    ];

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some("gain control of target creature until end of turn. Untap that creature. Gain 2 life")
    );
}

#[test]
pub(super) fn describe_effect_clause_list_dispatches_untap_then_gain_control_structurally() {
    let untapped = TagKey::from("untapped");
    let mut control = crate::effects::ApplyContinuousEffect::new_runtime(
        crate::continuous::EffectTarget::Source,
        crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
        Until::EndOfTurn,
    );
    control.target_spec = Some(ChooseSpec::Tagged(untapped.clone()));
    let effects = vec![
        Effect::new(crate::effects::TaggedEffect::new(
            untapped,
            Effect::new(crate::effects::UntapEffect::with_spec(ChooseSpec::target(
                ChooseSpec::Object(ObjectFilter::creature()),
            ))),
        )),
        Effect::new(crate::effects::TaggedEffect::new(
            "controlled",
            Effect::new(control),
        )),
        Effect::new(crate::effects::GainLifeEffect::you(Value::Fixed(2))),
    ];

    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some("untap target creature and gain control of it until end of turn. Gain 2 life")
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_gain_control_create_token_attach_sequence() {
    let controlled = TagKey::from("controlled_0");
    let created = TagKey::from("created_1");
    let mut control = crate::effects::ApplyContinuousEffect::new_runtime(
        crate::continuous::EffectTarget::Source,
        crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
        Until::Forever,
    );
    control.target_spec = Some(ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default().with_subtype(Subtype::Equipment),
    )));

    let token =
        crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Phyrexian Germ")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Phyrexian, Subtype::Germ])
            .color_indicator(crate::color::ColorSet::BLACK)
            .power_toughness(crate::card::PowerToughness::fixed(0, 0))
            .build();
    let mut equipment_filter = ObjectFilter::default().with_subtype(Subtype::Equipment);
    equipment_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: controlled.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let effects = vec![
        Effect::new(crate::effects::TaggedEffect::new(
            controlled,
            Effect::new(control),
        )),
        Effect::create_tokens(token, 1).tag(created.clone()),
        Effect::new(crate::effects::AttachObjectsEffect::new(
            ChooseSpec::All(equipment_filter),
            ChooseSpec::Tagged(created),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Gain control of target Equipment, then create a 0/0 black Phyrexian Germ creature token and attach that Equipment to it"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_damage_this_way_regeneration_structurally() {
    let damaged = TagKey::from("damaged_0");
    let mut creature_filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
    creature_filter
        .excluded_static_abilities
        .push(crate::static_abilities::StaticAbilityId::Flying);

    let cant_filter = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .match_tagged(damaged.clone(), TaggedOpbjectRelation::IsTaggedObject);
    let effects = vec![
        Effect::for_each(
            creature_filter,
            vec![Effect::deal_damage(Value::Fixed(3), ChooseSpec::Iterated).tag(damaged)],
        ),
        Effect::new(crate::effects::ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::deal_damage(
                Value::Fixed(3),
                ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            )],
        )),
        Effect::new(crate::effects::CantEffect::until_end_of_turn(
            crate::effect::Restriction::be_regenerated(cant_filter),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Deal 3 damage to each creature without flying and each player. Creatures dealt damage this way can't be regenerated this turn"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_choose_top_exile_play_structurally() {
    let exiled = TagKey::from("exiled");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default().owned_by(PlayerFilter::You),
        1,
        PlayerFilter::You,
        exiled.clone(),
    )
    .in_zone(Zone::Library)
    .top_only();
    let effects = vec![
        Effect::new(choose),
        Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(
            exiled.clone(),
        ))),
        Effect::new(crate::effects::GrantPlayTaggedEffect::new(
            exiled,
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
            true,
            false,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Exile the top card of your library. You may play that card this turn"
    );
}

#[test]
pub(super) fn describe_draw_replacement_preserves_iterated_player_for_typed_exile_top() {
    let exiled = TagKey::from("exiled");
    let replacement_effects = vec![
        Effect::new(
            crate::effects::ExileTopOfLibraryEffect::new(
                Value::Fixed(1),
                PlayerFilter::IteratedPlayer,
            )
            .tag_moved(exiled.clone()),
        ),
        Effect::new(crate::effects::GrantPlayTaggedEffect::new(
            exiled,
            PlayerFilter::IteratedPlayer,
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
            true,
            false,
        )),
    ];
    let register = Effect::new(crate::effects::RegisterDrawReplacementEffect::new(
        PlayerFilter::IteratedPlayer,
        replacement_effects,
        crate::effects::ReplacementApplyMode::OneShot,
    ));

    assert_eq!(
        describe_effect(&register),
        "The next time they would draw a card this turn, instead they exile the top card of their library. They may play it this turn"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_choose_top_exile_play_until_next_end_step() {
    let exiled = TagKey::from("__sentence_helper_exiled_l0_s0_e2");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default().owned_by(PlayerFilter::You),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        exiled.clone(),
    )
    .in_zone(Zone::Library)
    .top_only();
    let effects = vec![
        Effect::new(choose),
        Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(
            exiled,
        ))),
        Effect::new(crate::effects::GrantPlayTaggedEffect::new(
            TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep,
            true,
            false,
        )),
    ];

    let expected =
        "Exile the top card of your library. You may play that card until your next end step";
    assert_eq!(describe_effect_list(&effects), expected);
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
}

#[test]
pub(super) fn describe_effect_list_keeps_target_modifications_and_exile_permission_as_sentences() {
    let pumped = TagKey::from("pumped_0");
    let mut pump = crate::effects::ApplyContinuousEffect::new_runtime(
        crate::continuous::EffectTarget::Source,
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
            power: Value::Fixed(3),
            toughness: Value::Fixed(1),
        },
        Until::EndOfTurn,
    );
    pump.target_spec = Some(ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature().in_zone(Zone::Battlefield),
    )));
    pump.require_creature_target = true;

    let mut haste = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Source,
        crate::continuous::Modification::AddAbility(crate::static_abilities::StaticAbility::haste()),
        Until::EndOfTurn,
    );
    haste.target_spec = Some(ChooseSpec::Tagged(pumped.clone()));

    let exiled = TagKey::from("__sentence_helper_exiled_l0_s1_e0");
    let effects = vec![
        Effect::new(pump).tag(pumped),
        Effect::new(haste).tag(TagKey::from("granted_1")),
        Effect::new(
            crate::effects::ExileTopOfLibraryEffect::new(Value::Fixed(1), PlayerFilter::You)
                .tag_moved(exiled.clone()),
        ),
        Effect::new(crate::effects::GrantPlayTaggedEffect::new(
            exiled,
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep,
            true,
            false,
        )),
    ];
    let expected = "Target creature gets +3/+1 and gains haste until end of turn. Exile the top card of your library. You may play it until your next end step";

    assert_eq!(describe_effect_list(&effects), expected);
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected)
    );
}

#[test]
pub(super) fn implicit_target_opponent_is_consumed_by_exile_top_action() {
    let target = Effect::new(crate::effects::TargetOnlyEffect::new(
        ChooseSpec::target_opponent(),
    ));
    let exile = Effect::new(crate::effects::ExileTopOfLibraryEffect::new(
        Value::Fixed(3),
        PlayerFilter::target_opponent(),
    ));

    let rendered = describe_effect_list(&[target, exile]);
    assert_eq!(
        rendered,
        "Exile the top three cards of target opponent's library"
    );
    assert!(!rendered.contains("Choose target opponent"), "{rendered}");
}

#[test]
pub(super) fn describe_effect_list_compacts_exile_top_choose_one_then_play_chosen_card() {
    let exiled = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let chosen = TagKey::from("__it__");
    let exile = crate::effects::ExileTopOfLibraryEffect::new(Value::Fixed(2), PlayerFilter::You)
        .tag_moved(exiled.clone());
    let mut choice_filter = ObjectFilter::default().in_zone(Zone::Exile);
    choice_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: exiled,
            relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        });
    let choose = crate::effects::ChooseObjectsEffect::new(
        choice_filter,
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        chosen.clone(),
    )
    .in_zone(Zone::Exile);
    let grant = crate::effects::GrantPlayTaggedEffect::new(
        chosen,
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        true,
        false,
    );
    let effects = vec![Effect::new(exile), Effect::new(choose), Effect::new(grant)];

    assert_eq!(
        describe_effect_list(&effects),
        "Exile the top two cards of your library. Choose one of them. Until end of turn, you may play that card"
    );
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(
            "Exile the top two cards of your library. Choose one of them. Until end of turn, you may play that card"
        )
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_choose_top_exile_play_prefix_before_rest() {
    let exiled = TagKey::from("exiled");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default().owned_by(PlayerFilter::You),
        1,
        PlayerFilter::You,
        exiled.clone(),
    )
    .in_zone(Zone::Library)
    .top_only();
    let mut nonland_exiled = ObjectFilter::default().in_zone(Zone::Exile);
    nonland_exiled.excluded_card_types.push(CardType::Land);
    let effects = vec![
        Effect::new(choose),
        Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(
            exiled.clone(),
        ))),
        Effect::new(crate::effects::GrantPlayTaggedEffect::new(
            exiled,
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
            true,
            false,
        )),
        Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::Object(
            nonland_exiled,
        ))),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Exile the top card of your library. You may play that card this turn. Then exile a nonland card in exile"
    );
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(
            "Exile the top card of your library. You may play that card this turn. Then exile a nonland card in exile"
        )
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_search_reveal_conditional_move_then_shuffle() {
    let tag = TagKey::from("searched");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::land()
            .with_supertype(Supertype::Basic)
            .with_subtype(Subtype::Plains)
            .in_zone(Zone::Library),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Library)
    .as_search();
    let conditional = crate::effects::ConditionalEffect::new(
        Condition::PlayerControlsMoreThanYou {
            player: PlayerFilter::Opponent,
            filter: ObjectFilter::land(),
        },
        vec![Effect::new(
            crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(tag.clone()),
                Zone::Battlefield,
                false,
            )
            .tapped(),
        )],
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(tag.clone()),
            Zone::Hand,
            false,
        ))],
    );
    let effects = vec![
        Effect::new(choose),
        Effect::new(crate::effects::RevealTaggedEffect::new(tag)),
        Effect::new(conditional),
        Effect::new(crate::effects::ShuffleLibraryEffect::new(PlayerFilter::You)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Search your library for a basic Plains card and reveal it. If an opponent controls more lands than you, put it onto the battlefield tapped. Otherwise, put it into your hand. Then shuffle"
    );
}

#[test]
pub(super) fn describe_effect_list_compacts_may_search_reveal_conditional_move_then_shuffle() {
    let tag = TagKey::from("searched");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::artifact().in_zone(Zone::Library),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Library)
    .as_search();
    let conditional = crate::effects::ConditionalEffect::new(
        Condition::TaggedObjectMatches(
            tag.clone(),
            ObjectFilter::default().with_mana_value(crate::filter::Comparison::LessThanOrEqual(2)),
        ),
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(tag.clone()),
            Zone::Battlefield,
            false,
        ))],
        vec![Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(tag.clone()),
            Zone::Hand,
            false,
        ))],
    );
    let effects = vec![
        Effect::new(crate::effects::MayEffect::new(vec![
            Effect::new(choose),
            Effect::new(crate::effects::RevealTaggedEffect::new(tag)),
        ])),
        Effect::new(conditional),
        Effect::new(crate::effects::ShuffleLibraryEffect::new(PlayerFilter::You)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "You may search your library for an artifact card and reveal it. Put it onto the battlefield if its mana value is 2 or less. Otherwise, put it into your hand. If you search your library this way, shuffle"
    );
}

#[test]
pub(super) fn attached_helper_tags_do_not_render_as_player_facing_actions() {
    let tag = TagKey::from("equipped");
    let mut equipped = ObjectFilter::creature();
    equipped.tagged_constraints.push(TaggedObjectConstraint {
        tag: tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let effects = vec![
        Effect::new(crate::effects::TagAttachedToSourceEffect::new(tag)),
        Effect::regenerate(ChooseSpec::Object(equipped), Until::EndOfTurn),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Regenerate equipped creature"
    );
}

#[test]
pub(super) fn phase_out_equipped_creature_uses_oracle_subject_verb_surface() {
    let mut equipped = ObjectFilter::creature();
    equipped.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("equipped"),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    assert_eq!(
        describe_effect(&Effect::new(crate::effects::PhaseOutEffect::with_spec(
            ChooseSpec::Object(equipped),
        ))),
        "Equipped creature phases out"
    );
}

#[test]
pub(super) fn describe_shuffle_objects_into_library_uses_owner_and_graveyard_wording() {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::artifact().controlled_by(PlayerFilter::Opponent),
    ));
    let owner_shuffle = Effect::new(crate::effects::ShuffleObjectsIntoLibraryEffect::new(
        target,
        PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target),
    ));
    assert_eq!(
        describe_effect(&owner_shuffle),
        "The owner of target artifact an opponent controls shuffles it into their library"
    );

    let graveyard_target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You),
    ))
    .with_count(ChoiceCount::up_to(4));
    let graveyard_shuffle = Effect::new(crate::effects::ShuffleObjectsIntoLibraryEffect::new(
        graveyard_target,
        PlayerFilter::You,
    ));
    assert_eq!(
        describe_effect(&graveyard_shuffle),
        "Shuffle up to four target cards from your graveyard into your library"
    );
}

#[test]
pub(super) fn describe_owner_library_destination_uses_imperative_owner_surface() {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::permanent()
            .nontoken()
            .controlled_by(PlayerFilter::You),
    ));
    let targeted = Effect::new(
        crate::effects::ShuffleObjectsIntoLibraryEffect::new(
            target,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target),
        )
        .with_owner_library_destination(),
    );
    assert_eq!(
        describe_effect(&targeted),
        "Shuffle target nontoken permanent you control into its owner's library"
    );

    let triggering = TagKey::from("triggering");
    let tagged = Effect::new(
        crate::effects::ShuffleObjectsIntoLibraryEffect::new(
            ChooseSpec::Tagged(triggering.clone()),
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(triggering)),
        )
        .with_owner_library_destination(),
    );
    assert_eq!(
        describe_effect(&tagged),
        "Shuffle it into its owner's library"
    );
}

#[test]
pub(super) fn describe_unless_any_player_pays_search_sequence_uses_prefix() {
    let tag = TagKey::from("searched");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default().in_zone(Zone::Library),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    )
    .in_zone(Zone::Library)
    .as_search();
    let sequence = crate::effects::SequenceEffect::new(vec![
        Effect::new(choose),
        Effect::new(crate::effects::ForEachTaggedEffect::new(
            tag,
            vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Iterated,
                Zone::Hand,
                false,
            ))],
        )),
        Effect::new(crate::effects::ShuffleLibraryEffect::new(PlayerFilter::You)),
    ]);
    let effect = Effect::unless_pays(
        vec![Effect::new(sequence)],
        PlayerFilter::Any,
        vec![crate::mana::ManaSymbol::Generic(2)],
    );

    assert_eq!(
        describe_effect(&effect),
        "Unless any player pays {2}, search your library for a card, put it into your hand, then shuffle"
    );
}

#[test]
pub(super) fn describe_unless_any_player_pays_direct_search_uses_prefix() {
    let search = crate::effects::SearchLibraryEffect::to_hand(
        ObjectFilter::default()
            .in_zone(Zone::Library)
            .owned_by(PlayerFilter::You),
        PlayerFilter::You,
        false,
    );
    let effect = Effect::unless_pays(
        vec![Effect::new(search)],
        PlayerFilter::Any,
        vec![crate::mana::ManaSymbol::Generic(2)],
    );

    assert_eq!(
        describe_effect(&effect),
        "Unless any player pays {2}, search your library for a card, put it into your hand, then shuffle"
    );
}

#[test]
pub(super) fn return_subtype_cards_from_your_graveyard_compacts_do_the_same_list() {
    let subtype_return = |subtype| {
        Effect::new(crate::effects::ReturnFromGraveyardToHandEffect::new(
            ChooseSpec::Object(
                ObjectFilter::default()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You)
                    .with_subtype(subtype),
            )
            .with_count(ChoiceCount::exactly(1)),
            false,
        ))
        .tag(format!("{subtype:?}").to_ascii_lowercase())
    };
    let effects = vec![
        subtype_return(Subtype::Pirate),
        subtype_return(Subtype::Vampire),
        subtype_return(Subtype::Dinosaur),
        subtype_return(Subtype::Merfolk),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Return a Pirate card from your graveyard to your hand, then do the same for Vampire, Dinosaur, and Merfolk"
    );
}

#[test]
pub(super) fn source_return_from_contextual_graveyard_uses_card_surface() {
    let effect = Effect::new(
        crate::effects::ReturnFromGraveyardToHandEffect::new(ChooseSpec::Source, false)
            .with_graveyard_player_surface(PlayerFilter::You)
            .with_destination_player_surface(PlayerFilter::You),
    );

    assert_eq!(
        describe_effect(&effect),
        "Return this card from your graveyard to your hand"
    );
}

#[test]
pub(super) fn describe_may_unless_pay_mana_uses_or_surface() {
    let effect = Effect::may_single(Effect::unless_action(
        vec![Effect::discard_player_filtered(
            Value::Fixed(1),
            PlayerFilter::You,
            false,
            None,
        )],
        vec![Effect::new(crate::effects::PayManaEffect::new(
            crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]),
            ChooseSpec::Player(PlayerFilter::You),
        ))],
        PlayerFilter::You,
    ));

    assert_eq!(
        describe_effect(&effect),
        "You may discard a card or pay {2}"
    );
}

#[test]
pub(super) fn describe_unless_source_damage_matches_tagged_primary_target_controller() {
    let tag = TagKey::from("damaged_0");
    let effect = Effect::unless_action(
        vec![Effect::deal_damage(3, ChooseSpec::target_creature()).tag(tag.clone())],
        vec![Effect::deal_damage(
            5,
            ChooseSpec::Player(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)),
        )],
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag)),
    );

    assert_eq!(
        describe_effect(&effect),
        "Deal 3 damage to target creature unless that object's controller has this source deal 5 damage to them"
    );
}

#[test]
pub(super) fn describe_unless_pays_one_of_sacrifice_or_life_uses_action_surface() {
    let cost = crate::cost::TotalCost::one_of(vec![
        crate::cost::TotalCost::from_cost(crate::costs::Cost::sacrifice(
            ObjectFilter::default().with_type(CardType::Creature),
        )),
        crate::cost::TotalCost::from_cost(crate::costs::Cost::life(3)),
    ]);
    let effect = Effect::unless_pays_total_cost(
        vec![Effect::draw(Value::Fixed(1))],
        PlayerFilter::target_opponent(),
        cost,
    );

    assert_eq!(
        describe_effect(&effect),
        "you draw a card unless target opponent sacrifices a creature or pays 3 life"
    );
}

#[test]
pub(super) fn describe_become_creature_type_choice_uses_each_for_all_creatures() {
    let effect = Effect::new(crate::effects::BecomeCreatureTypeChoiceEffect::new(
        ChooseSpec::Object(ObjectFilter::creature()),
        Until::EndOfTurn,
        vec![Subtype::Wall],
    ));

    let rendered = describe_effect(&effect);
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower
            == "choose a creature type other than wall. each creature becomes that type until end of turn",
        "expected plural creature-type choice wording, got {rendered}"
    );
}
