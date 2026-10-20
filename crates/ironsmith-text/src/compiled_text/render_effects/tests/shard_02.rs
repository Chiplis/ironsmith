use super::*;

fn ordered_type_sacrifice_choice(types: [CardType; 3]) -> ironsmith_core::ChooseModeEffect<Effect> {
    let orders = [
        [0, 1, 2], [0, 2, 1], [1, 0, 2],
        [1, 2, 0], [2, 0, 1], [2, 1, 0],
    ];
    let modes = orders.into_iter().map(|order| {
        let effects = order.into_iter().enumerate().map(|(rank, index)| {
            let tag = TagKey::from(format!("chosen_{rank}_{index}"));
            Effect::new(crate::effects::ForPlayersEffect {
                filter: PlayerFilter::Any,
                effects: vec![
                    Effect::new(crate::effects::ChooseObjectsEffect::new(
                        ObjectFilter::default()
                            .with_type(types[index])
                            .controlled_by(PlayerFilter::IteratedPlayer)
                            .in_zone(Zone::Battlefield),
                        ironsmith_core::ChoiceCount::exactly(rank + 1),
                        PlayerFilter::IteratedPlayer,
                        tag.clone(),
                    )),
                    Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
                        ObjectFilter::tagged(tag),
                        Value::Fixed((rank + 1) as i32),
                        PlayerFilter::IteratedPlayer,
                    )),
                ],
                sequential: false,
                starting_with_controller: false,
                stop_after_first_happened: false,
            })
        }).collect();
        ironsmith_core::EffectMode::new("untrusted mode label", effects)
    }).collect();
    ironsmith_core::ChooseModeEffect::choose_one(modes).with_chooser(PlayerFilter::You)
}

#[test]
fn parsed_ordered_type_sacrifices_keep_the_order_instruction() {
    let oracle = "Choose an order for artifacts, creatures, and lands. Each player sacrifices one permanent of their choice of the first type, sacrifices two of their choice of the second type, then sacrifices three of their choice of the third type.";
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(), "Ordered Sacrifice Probe",
    )
    .card_types(vec![CardType::Sorcery])
    .parse_text(oracle)
    .expect("ordered-type sacrifice should compile");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("Choose an order for artifacts, creatures, and lands"), "{rendered}");
    assert!(rendered.contains("sacrifices three of their choice of the third type"), "{rendered}");
    assert!(!rendered.contains("• Artifacts, then"), "{rendered}");
}

#[test]
fn ordered_type_sacrifice_modes_render_the_common_order_instruction() {
    let choose = ordered_type_sacrifice_choice([
        CardType::Artifact, CardType::Creature, CardType::Land,
    ]);
    let effect = Effect::new(choose);
    let rendered = describe_effect(&effect);
    assert_eq!(
        rendered,
        "Choose an order for artifacts, creatures, and lands. Each player sacrifices one permanent of their choice of the first type, sacrifices two of their choice of the second type, then sacrifices three of their choice of the third type"
    );
    assert!(!rendered.contains("untrusted mode label"), "{rendered}");
    assert_eq!(describe_effect_list(&[effect]), rendered);
}

#[test]
fn ordered_type_sacrifice_modes_generalize_and_reject_changed_execution() {
    let types = [CardType::Enchantment, CardType::Battle, CardType::Land];
    let choose = ordered_type_sacrifice_choice(types);
    let rendered = describe_effect_list(&[Effect::new(choose.clone())]);
    assert!(rendered.starts_with("Choose an order for enchantments, battles, and lands."), "{rendered}");

    let mut duplicate = choose.clone();
    duplicate.modes[5] = duplicate.modes[0].clone();
    assert!(super::super::single_effects_early::describe_compact_choose_mode_branch(&[Effect::new(duplicate.clone())]).is_none());
    assert!(!describe_effect(&Effect::new(duplicate)).starts_with("Choose an order for"));

    let mut wrong_count = choose.clone();
    let mut for_players = wrong_count.modes[0].effects[1]
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .expect("second ordered step")
        .clone();
    let mut sacrifice = for_players.effects[1]
        .downcast_ref::<crate::effects::zones::SacrificePlayerEffect>()
        .expect("sacrifice step")
        .clone();
    sacrifice.count = Value::Fixed(3);
    for_players.effects[1] = Effect::new(sacrifice);
    wrong_count.modes[0].effects[1] = Effect::new(for_players);
    assert!(super::super::single_effects_early::describe_compact_choose_mode_branch(&[Effect::new(wrong_count)]).is_none());

    let mut optional_choice = choose;
    optional_choice.optional_choice = true;
    assert!(super::super::single_effects_early::describe_compact_choose_mode_branch(&[Effect::new(optional_choice)]).is_none());
}

#[test]
fn unique_reveal_winner_retargets_only_without_a_tie_or_same_name_trigger() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Contest Device",
    )
    .card_types(vec![CardType::Enchantment])
    .parse_text(
        "Whenever a player chooses one or more targets, each player reveals the top card of their library. The player who reveals the card with the greatest mana value may change the target or targets. If two or more cards are tied for greatest, the target or targets remain unchanged. Changing targets this way doesn't trigger abilities of permanents named Contest Device.",
    )
    .expect("unique-winner reveal contest should compile");
    let structure = format!("{:#?}", definition.abilities);
    assert!(structure.contains("KeepGreatestManaValuePlayersEffect"), "{structure}");
    assert!(format!("{:?}", definition.abilities).contains("Equal(1)"), "{structure}");
    assert!(structure.contains("doesnt_trigger_same_named_sources: true"), "{structure}");
    assert!(!structure.contains("RepeatProcessEffect"), "{structure}");

    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains("If two or more cards are tied for greatest, the target or targets remain unchanged"),
        "{rendered}"
    );
    assert!(
        rendered.contains("Changing targets this way doesn't trigger abilities of permanents named Contest Device"),
        "{rendered}"
    );
    assert!(!rendered.contains("repeat this process"), "{rendered}");
}

#[test]
fn each_player_random_pile_sacrifice_keeps_order_and_complement() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Three Pile Probe",
    )
    .card_types(vec![CardType::Sorcery])
    .parse_text("Starting with you, each player separates all permanents they control into three piles. Then each player chooses one of their piles at random and sacrifices those permanents.")
    .expect("per-player pile procedure compiles");
    let effects = &definition.spell_effect.as_ref().expect("spell effect").segments[0].default_effects;
    let separation = effects[0]
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .expect("per-player separation");
    assert!(separation.starting_with_controller, "{effects:#?}");
    assert_eq!(separation.effects.len(), 2, "the third pile is the complement");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("Starting with you, each player separates all permanents they control into three piles"), "{rendered}");
    assert!(rendered.contains("Then each player chooses one of their piles at random and sacrifices those permanents"), "{rendered}");
    assert!(!rendered.contains("• Pile 1"), "{rendered}");
}

#[test]
fn random_pile_compactor_preserves_a_nonrandom_choice() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Two Pile Probe",
    )
    .card_types(vec![CardType::Sorcery])
    .parse_text("Each player separates all permanents they control into two piles. Then each player chooses one of their piles at random and sacrifices those permanents.")
    .expect("two-pile procedure compiles");
    let mut effects = definition.spell_effect.as_ref().expect("spell effect").segments[0].default_effects.clone();
    let for_players = effects[1]
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .expect("per-player sacrifice");
    let mut for_players = for_players.clone();
    let choice = for_players.effects[0]
        .downcast_ref::<crate::effects::ChooseModeEffect>()
        .expect("random pile choice");
    let mut choice = choice.clone();
    choice.random = false;
    for_players.effects[0] = Effect::new(choice);
    effects[1] = Effect::new(for_players);
    let rendered = describe_effect_list(&effects[..2]);
    assert!(!rendered.contains("separates all permanents they control into two piles"), "{rendered}");
}

#[test]
fn ordered_dice_with_identical_result_tables_render_each_result_once() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Ordered Dice Probe",
    )
    .card_types(vec![CardType::Creature])
    .parse_text("At the beginning of combat on your turn, roll two six-sided dice. For each result of 1, this creature gets +1/+1 until end of turn. For each other result, it gains the indicated ability until end of turn. If you rolled doubles, it also gains double strike until end of turn.\n• 2 — menace\n• 3 — vigilance\n• 4 — lifelink\n• 5 — flying\n• 6 — indestructible")
    .expect("ordered die result table compiles");
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("ordered die table must be triggered");
    };
    assert!(super::super::roll_result_surfaces::describe_per_die_result_ability_table_program(
        &triggered.effects
    ).is_some(), "the typed renderer must recognize the complete program");
    let mut mismatched = triggered.effects.clone();
    let second_die_two = mismatched.segments[0].default_effects[8]
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .expect("second die result-two branch");
    let mut second_die_two = second_die_two.clone();
    if let Condition::ValueComparison { right, .. } = &mut second_die_two.condition {
        *right = Value::Fixed(3);
    } else {
        panic!("result branch must compare a die result");
    }
    mismatched.segments[0].default_effects[8] = Effect::new(second_die_two);
    // Rebuild the program to refresh its flattened default-effect cache.
    let mismatched = crate::resolution::ResolutionProgram::new(mismatched.segments);
    assert!(super::super::roll_result_surfaces::describe_per_die_result_ability_table_program(
        &mismatched
    ).is_none(), "a non-exhaustive second table must not use the compact surface");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("roll two six-sided dice"), "{rendered}");
    assert!(rendered.contains("For each other result, it gains the indicated ability"), "{rendered}");
    assert!(rendered.contains("If you rolled doubles, it also gains double strike"), "{rendered}");
    assert!(rendered.contains("• 6 — indestructible"), "{rendered}");
    assert!(!rendered.contains("the other result is equal to"), "{rendered}");
}

#[test]
fn repeated_source_loyalty_allowances_render_the_extra_count_once() {
    let grant = Effect::new(crate::effects::GrantLoyaltyActivationAllowanceEffect::new(
        crate::effects::LoyaltyActivationScope::Source,
        crate::effects::LoyaltyActivationAllowance::ExtraActivation,
    ));
    let prefix = Effect::new(crate::effects::PutCountersEffect::new(
        crate::object::CounterType::Loyalty,
        Value::Fixed(1),
        ChooseSpec::Source,
    ));
    let effect = Effect::new(crate::effects::SequenceEffect::comma_then(vec![
        prefix.clone(), grant.clone(), grant.clone(),
    ]));
    let rendered = describe_effect(&effect);
    assert!(rendered.contains("two more times this turn"), "{rendered}");
    assert_eq!(rendered.matches("two more times this turn").count(), 1);

    let different = Effect::new(crate::effects::GrantLoyaltyActivationAllowanceEffect::new(
        crate::effects::LoyaltyActivationScope::ControlledPlaneswalkers { subtype: None },
        crate::effects::LoyaltyActivationAllowance::ExtraActivation,
    ));
    let near_miss = Effect::new(crate::effects::SequenceEffect::comma_then(vec![
        prefix, grant, different,
    ]));
    assert!(!describe_effect(&near_miss).contains("two more times this turn"));
}

#[test]
fn rounded_life_token_size_is_shared_with_its_damage() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Rounded Token Probe",
    )
    .card_types(vec![CardType::Enchantment])
    .subtypes(vec![crate::types::Subtype::Saga])
    .parse_text("I, II — This Saga deals 2 damage to each opponent and you gain 2 life.\nIII — Create an X/X black Nightmare Horror creature token, where X is half your life total, rounded up. It deals X damage to you.")
    .expect("rounded token-size Saga compiles");
    let program = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        crate::ability::AbilityKind::Triggered(triggered) => Some(&triggered.effects),
        _ => None,
    }).last().expect("final chapter ability");
    assert!(super::super::effect_lists::describe_rounded_token_damage(
        program.flattened_default_effects(),
    ).is_some(), "the parsed token/damage chain must match the typed guard: {program:#?}");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("where X is half your life total, rounded up"), "{rendered}");
    assert!(rendered.contains("It deals X damage to you"), "{rendered}");
    assert!(!rendered.contains("Round up each time"), "{rendered}");
}

#[test]
fn linked_entry_life_payment_names_the_token_size_basis() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Entry Payment Probe",
    )
    .card_types(vec![CardType::Artifact])
    .parse_text("As this artifact enters, pay any amount of life.\n{4}, {T}: Create an X/X black Phyrexian Minion creature token, where X is the life paid as this artifact entered.")
    .expect("linked entry payment compiles");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("where X is the life paid as this artifact entered"), "{rendered}");
    assert!(!rendered.contains("last chosen number"), "{rendered}");
}

#[test]
fn matching_conditional_token_repeat_uses_a_token_backreference() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Token Repeat Probe",
    )
    .card_types(vec![CardType::Sorcery])
    .parse_text("Corrupted — Create an X/X green Phyrexian Wurm creature token with trample and toxic 1, where X is the amount of mana spent to cast this spell. Then for each opponent with three or more poison counters, you create another one of those tokens.\nFlashback {8}{G}{G}")
    .expect("conditional token repeat compiles");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("you create another one of those tokens"), "{rendered}");
    assert_eq!(rendered.matches("where X is the amount of mana spent to cast this spell").count(), 1, "{rendered}");
}

#[test]
fn counter_choice_keeps_reflexive_followup_in_compiled_text() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Counter Choice Probe",
    )
    .card_types(vec![CardType::Creature])
    .parse_text(
        "Whenever this creature enters or attacks, put a charge counter on it or remove one from it. When you remove a counter this way, choose one —\n• This creature deals 4 damage to any target.\n• This creature gains lifelink and indestructible until end of turn.",
    )
    .expect("counter choice with a reflexive continuation should compile");

    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("put a charge counter on it or remove"), "{rendered}");
    assert!(rendered.contains("When you remove a charge counter this way"), "{rendered}");
    assert!(rendered.contains("deals 4 damage to any target"), "{rendered}");
    assert!(rendered.contains("gains lifelink and indestructible until end of turn"), "{rendered}");
}

#[test]
fn protection_choice_granted_to_player_and_controlled_permanents_keeps_both_recipients() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Protection Choice Probe",
    )
    .card_types(vec![CardType::Instant])
    .parse_text(
        "Target permanent you control gains protection from the color of your choice until end of turn.\nFateful hour — If you have 5 or less life, instead you and each permanent you control gain protection from the color of your choice until end of turn.",
    )
    .expect("replacement protection choice should compile");

    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("you and each permanent you control gain protection from the color of your choice until end of turn"), "{rendered}");
    assert!(!rendered.contains("• White"), "{rendered}");
}

#[test]
fn named_card_guess_keeps_optional_reveal_and_wrong_guess_reward() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Guessing Device",
    )
    .card_types(vec![CardType::Artifact])
    .parse_text(
        "{2}, {T}: Choose a card name. Target opponent guesses whether a card with that name is in your hand. You may reveal your hand. If you do and your opponent guessed wrong, draw a card.",
    )
    .expect("named-card guess should compile");

    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("Target opponent guesses whether a card with that name is in your hand"), "{rendered}");
    assert!(rendered.contains("If you do and your opponent guessed wrong, draw a card"), "{rendered}");
    assert!(!rendered.contains("• It's in their hand"), "{rendered}");
}

#[test]
fn put_into_graveyard_history_uses_the_structured_owner_for_each_zone() {
    let your_history =
        Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::PutIntoGraveyard {
            owner: PlayerFilter::You,
            from: vec![Zone::Hand, Zone::Library],
        });
    assert_eq!(
        describe_value(&your_history),
        "the number of cards put into your graveyard from your hand or library this turn"
    );

    let their_history =
        Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::PutIntoGraveyard {
            owner: PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)),
            from: vec![Zone::Library, Zone::Hand],
        });
    assert_eq!(
        describe_value(&their_history),
        "the number of cards put into their graveyard from their hand or library this turn"
    );
}

#[test]
fn draw_then_gain_shared_history_value_renders_one_where_x_basis() {
    let shared = Value::TurnHistoryCount(
        ironsmith_core::TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(PlayerFilter::You),
    )
    .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let effects = vec![
        Effect::new(crate::effects::DrawCardsEffect::new(
            shared.clone(),
            PlayerFilter::You,
        )),
        Effect::new(crate::effects::GainLifeEffect::you(shared)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "You draw X cards and gain X life, where X is the number of colors among permanents you control and spells you've cast this turn"
    );
}

#[test]
fn gain_then_put_counters_shared_value_renders_one_where_x_basis() {
    let shared = Value::PowerOf(Box::new(
        ChooseSpec::Tagged(TagKey::from("sacrificed_0")).with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SacrificedObject(
                ironsmith_core::SacrificedObjectKind::Creature,
            ),
        ),
    ))
    .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature()
            .you_control()
            .in_zone(Zone::Battlefield),
    ));
    let effects = vec![
        Effect::new(crate::effects::GainLifeEffect::you(shared.clone())),
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            shared,
            target,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "You gain X life and put X +1/+1 counters on target creature you control, where X is the sacrificed creature's power"
    );

    let coordinated = Effect::new(crate::effects::SequenceEffect::coordinated(effects));
    assert_eq!(
        describe_effect(&coordinated),
        "You gain X life and put X +1/+1 counters on target creature you control, where X is the sacrificed creature's power"
    );
}

#[test]
fn any_number_from_hand_then_linked_draw_keeps_order_and_offset_surface() {
    let hand = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(PlayerFilter::You),
        ..Default::default()
    };
    let move_to_library = crate::effects::MoveToZoneEffect::to_bottom_of_library(
        ChooseSpec::Object(hand).with_count(ChoiceCount::any_number()),
    )
    .with_destination_player_surface(PlayerFilter::You);
    let effects = vec![
        Effect::with_id(7, Effect::new(move_to_library).tag(TagKey::from("moved_7"))),
        Effect::new(crate::effects::DrawCardsEffect::new(
            Value::EffectValueOffset(crate::effect::EffectId(7), 1),
            PlayerFilter::You,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Put any number of cards from your hand on the bottom of your library, then draw that many cards plus one"
    );
}

#[test]
fn whole_hand_then_linked_draw_keeps_order_and_any_order_surface() {
    let hand = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(PlayerFilter::You),
        ..Default::default()
    };
    let move_to_library =
        crate::effects::MoveToZoneEffect::to_bottom_of_library(ChooseSpec::All(hand))
            .with_library_order(ironsmith_core::LibraryPlacementOrder::ChosenBy(
                PlayerFilter::You,
            ))
            .with_destination_player_surface(PlayerFilter::You);
    let effects = vec![
        Effect::with_id(9, Effect::new(move_to_library)),
        Effect::new(crate::effects::DrawCardsEffect::new(
            Value::EffectValue(crate::effect::EffectId(9))
                .with_surface_hint(ValueSurfaceHint::ThatManyCards),
            PlayerFilter::You,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Put the cards in your hand on the bottom of your library in any order, then draw that many cards"
    );
}

#[test]
fn must_block_set_then_assignment_control_keeps_anaphoric_surface() {
    let must_block = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Filter(
            ObjectFilter::creature()
                .controlled_by(PlayerFilter::Opponent)
                .in_zone(Zone::Battlefield),
        ),
        crate::continuous::Modification::AddAbility(
            crate::static_abilities::StaticAbility::must_block(),
        ),
        Until::EndOfTurn,
    );
    let effects = vec![
        Effect::new(must_block).tag("granted_0"),
        Effect::control_combat_choices_this_turn(false, true),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Creatures your opponents control block this turn if able, and you choose how those creatures block"
    );
}

#[test]
fn draw_then_gain_different_history_values_do_not_share_a_where_x_basis() {
    let draw = Value::TurnHistoryCount(
        ironsmith_core::TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(PlayerFilter::You),
    )
    .with_surface_hint(ValueSurfaceHint::WhereXIs);
    let gain = Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::TokensCreated(
        PlayerFilter::You,
    ))
    .with_surface_hint(ValueSurfaceHint::WhereXIs);

    assert!(
        describe_same_actor_draw_then_gain(
            &Effect::new(crate::effects::DrawCardsEffect::new(
                draw,
                PlayerFilter::You,
            )),
            &Effect::new(crate::effects::GainLifeEffect::you(gain)),
        )
        .is_none()
    );
}

#[test]
fn permanent_gain_life_restriction_uses_rest_of_game_surface() {
    let effect = Effect::cant_until(
        crate::effect::Restriction::GainLife(PlayerFilter::DamagedPlayer),
        Until::Forever,
    );

    assert_eq!(
        describe_effect(&effect),
        "that player can't gain life for the rest of the game"
    );
}

#[test]
fn for_each_object_uses_iteration_pronouns_only_inside_the_loop() {
    let effect = Effect::for_each(
        ObjectFilter::creature(),
        vec![Effect::destroy(ChooseSpec::Iterated)],
    );

    assert_eq!(describe_effect(&effect), "For each creature, destroy it");
    assert_eq!(
        describe_effect(&Effect::destroy(ChooseSpec::Iterated)),
        "Destroy that object"
    );
}

#[test]
fn skip_draw_step_uses_controller_grammar_and_possessive() {
    assert_eq!(
        describe_effect(&Effect::new(crate::effects::SkipDrawStepEffect::you())),
        "you skip your next draw step"
    );
}

#[test]
fn tapped_target_group_freeze_keeps_plural_reference() {
    let tapped = TagKey::from("tapped_group");
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()))
        .with_count(ChoiceCount::up_to(2));
    let tap = Effect::tap(target).tag(tapped.clone());
    let mut filter = ObjectFilter::creature();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: tapped,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let cant = Effect::cant_until(
        crate::effect::Restriction::Untap(filter),
        Until::ControllersNextUntapStep,
    );

    assert_eq!(
        describe_effect_list(&[tap, cant]),
        "Tap up to two target creatures. Those creatures don't untap during their controllers' next untap steps"
    );
}

#[test]
fn consult_first_match_to_hand_keeps_oracle_sentence_boundary() {
    let revealed = TagKey::from("revealed_until_match");
    let matched = TagKey::from("matched_until_match");
    let effects = vec![
        Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
            PlayerFilter::You,
            crate::effects::consult_helpers::LibraryConsultMode::Reveal,
            ObjectFilter::creature(),
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
            revealed.clone(),
            matched.clone(),
        )),
        Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(matched.clone()),
            Zone::Hand,
            false,
        )),
        Effect::new(
            crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                revealed,
                Some(matched),
                crate::effects::consult_helpers::LibraryBottomOrder::Random,
                PlayerFilter::You,
            ),
        ),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Reveal cards from the top of your library until you reveal a creature card. Put that card into your hand and the rest on the bottom of your library in a random order"
    );
}

fn reveal_put_all_matching_rest_bottom_effects(
    count: Value,
    mut filter: ObjectFilter,
    enters_tapped: bool,
) -> Vec<Effect> {
    let revealed = TagKey::from("revealed");
    let matching = TagKey::from("matching");
    filter.zone = None;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: revealed.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let look = Effect::new(crate::effects::LookAtTopCardsEffect::new(
        PlayerFilter::You,
        count,
        revealed.clone(),
    ));
    let reveal = Effect::new(crate::effects::RevealTaggedEffect::new(revealed.clone()));
    let tag_matching = Effect::new(
        crate::effects::TagMatchingObjectsEffect::new(filter, matching.clone())
            .in_zone(Zone::Library),
    );
    let mut move_to_battlefield =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Battlefield, false);
    move_to_battlefield.enters_tapped = enters_tapped;
    let move_matching = Effect::new(crate::effects::ForEachTaggedEffect::new(
        matching.clone(),
        vec![Effect::new(move_to_battlefield)],
    ));
    let remainder = Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            revealed,
            Some(matching),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::You,
        ),
    );

    vec![look, reveal, tag_matching, move_matching, remainder]
}

#[test]
fn put_all_matching_lands_from_revealed_set_preserves_tapped_destination_and_random_order() {
    let effects = reveal_put_all_matching_rest_bottom_effects(Value::X, ObjectFilter::land(), true);

    assert_eq!(
        describe_effect_list(&effects),
        "Reveal the top X cards of your library. Put all land cards from among them onto the battlefield tapped and the rest on the bottom of your library in a random order"
    );
}

#[test]
fn put_all_matching_goblin_creatures_from_revealed_set_preserves_filter() {
    let filter = ObjectFilter::creature()
        .with_subtype(Subtype::Goblin)
        .with_mana_value(crate::filter::Comparison::LessThanOrEqual(5));
    let effects = reveal_put_all_matching_rest_bottom_effects(Value::Fixed(6), filter, false);

    assert_eq!(
        describe_effect_list(&effects),
        "Reveal the top six cards of your library. Put all Goblin creature cards with mana value 5 or less from among them onto the battlefield and the rest on the bottom of your library in a random order"
    );
}

fn reveal_choose_lands_to_battlefield_prefix(
    count: Value,
    choice_count: ChoiceCount,
    enters_tapped: bool,
) -> (Vec<Effect>, TagKey, TagKey) {
    let revealed = TagKey::from("revealed_counted");
    let chosen = TagKey::from("chosen_counted");
    // The lowered real-card shape carries the exact looked-card membership in
    // the filter while retaining broad legacy zone metadata on the choice.
    // Compaction must follow the tag, not treat the first legacy zone as the
    // semantic source of the candidates.
    let mut filter = ObjectFilter::land();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: revealed.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let choose = crate::effects::ChooseObjectsEffect::new(
        filter,
        choice_count,
        PlayerFilter::You,
        chosen.clone(),
    )
    .in_zones(vec![
        Zone::Battlefield,
        Zone::Hand,
        Zone::Graveyard,
        Zone::Library,
        Zone::Exile,
    ])
    .reveal();
    let mut move_to_battlefield =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Battlefield, false);
    move_to_battlefield.enters_tapped = enters_tapped;

    (
        vec![
            Effect::new(crate::effects::LookAtTopCardsEffect::new(
                PlayerFilter::You,
                count,
                revealed.clone(),
            )),
            Effect::new(crate::effects::RevealTaggedEffect::new(revealed.clone())),
            Effect::new(choose),
            Effect::new(crate::effects::ForEachTaggedEffect::new(
                chosen.clone(),
                vec![Effect::new(move_to_battlefield)],
            )),
        ],
        revealed,
        chosen,
    )
}

#[test]
fn counted_revealed_subset_to_battlefield_compacts_typed_bottom_remainder() {
    let (mut effects, revealed, chosen) =
        reveal_choose_lands_to_battlefield_prefix(Value::Fixed(6), ChoiceCount::up_to(1), true);
    effects.push(Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            revealed,
            Some(chosen),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::You,
        ),
    ));

    assert_eq!(
        describe_effect_list(&effects),
        "Reveal the top six cards of your library. Put up to one land card from among them onto the battlefield tapped. Put the rest on the bottom of your library in a random order"
    );
}

#[test]
fn counted_revealed_subset_to_battlefield_compacts_complement_to_graveyard() {
    let (mut effects, revealed, chosen) =
        reveal_choose_lands_to_battlefield_prefix(Value::Fixed(5), ChoiceCount::up_to(1), false);
    let mut iterated_is_chosen = ObjectFilter::default();
    iterated_is_chosen
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from("__it__"),
            relation: TaggedOpbjectRelation::SameStableId,
        });
    effects.push(Effect::new(crate::effects::ForEachTaggedEffect::new(
        revealed,
        vec![Effect::new(crate::effects::ConditionalEffect::new(
            Condition::TaggedObjectMatches(chosen, iterated_is_chosen),
            Vec::new(),
            vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Iterated,
                Zone::Graveyard,
                false,
            ))],
        ))],
    )));

    assert_eq!(
        describe_effect_list(&effects),
        "Reveal the top five cards of your library. Put up to one land card from among them onto the battlefield. Put the rest into your graveyard"
    );
}

#[test]
fn counted_revealed_subset_compacts_dynamic_total_mana_value_constraint() {
    let (mut effects, revealed, chosen) =
        reveal_choose_lands_to_battlefield_prefix(Value::Fixed(8), ChoiceCount::up_to(2), false);
    let choose = effects[2]
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("real-shape choice");
    let mut choose = choose.clone();
    choose.filter = ObjectFilter::artifact();
    choose.filter.excluded_card_types.push(CardType::Creature);
    choose
        .filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: revealed.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    choose.aggregate_constraint = Some(
        crate::effect::ChoiceAggregateConstraint::total_mana_value_at_most(Value::ManaValueOf(
            Box::new(
                ChooseSpec::Tagged(TagKey::from("sacrifice_cost_0")).with_surface_hint(
                    crate::target::ChooseSpecSurfaceHint::SourceReference(
                        crate::target::SourceReferenceSurface::ThisPermanentType(
                            "the sacrificed artifact".to_string(),
                        ),
                    ),
                ),
            ),
        )),
    );
    effects[2] = Effect::new(choose);
    effects.push(Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            revealed,
            Some(chosen),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::You,
        ),
    ));

    assert_eq!(
        describe_effect_list(&effects),
        "Reveal the top eight cards of your library. Put up to two noncreature artifact cards with total mana value less than or equal to the sacrificed artifact's mana value from among them onto the battlefield. Put the rest on the bottom of your library in a random order"
    );
}

#[test]
fn counter_and_draw_followups_preserve_conjoined_clause_composition() {
    let draw_then_counter = vec![
        Effect::draw(Value::Fixed(1)),
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::Charge,
            1,
            ChooseSpec::Source,
        )),
    ];
    let counter_then_draw = vec![
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::Stun,
            3,
            ChooseSpec::Source,
        )),
        Effect::draw(Value::Fixed(3)),
    ];

    assert_eq!(
        describe_effect_list(&draw_then_counter),
        "Draw a card and put a charge counter on this source"
    );
    assert_eq!(
        describe_effect_clause_list(&draw_then_counter).as_deref(),
        Some("draw a card and put a charge counter on this source")
    );
    assert_eq!(
        describe_effect_list(&counter_then_draw),
        "Put three stun counters on this source and draw three cards"
    );

    let draw_lose_then_counter = vec![
        Effect::draw(Value::Fixed(1)),
        Effect::new(crate::effects::LoseLifeEffect::you(1)),
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::Named("plan".into()),
            1,
            ChooseSpec::Source,
        )),
    ];
    assert_eq!(
        describe_effect_clause_list(&draw_lose_then_counter).as_deref(),
        Some("draw a card, lose 1 life, and put a plan counter on this source")
    );

    let scry_then_counter = vec![
        Effect::scry(Value::Fixed(1)),
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::Oil,
            1,
            ChooseSpec::Source,
        )),
    ];
    assert_eq!(
        describe_effect_list(&scry_then_counter),
        "Scry 1 and put an oil counter on this source"
    );
}

#[test]
fn discard_result_draw_sequence_preserves_up_to_and_that_many_surface() {
    let effect_id = crate::effect::EffectId(7);
    let discard = Effect::with_id(
        effect_id.0,
        Effect::new(
            crate::effects::DiscardEffect::new(2, PlayerFilter::You, false).with_any_number(true),
        ),
    );
    let draw = Effect::new(crate::effects::DrawCardsEffect::new(
        Value::EffectMetric {
            effect_id,
            source: crate::effect::EffectMetricSource::Outcome,
            metric: crate::effect::EffectMetric::Count,
        },
        PlayerFilter::You,
    ));

    assert_eq!(
        describe_effect_list(&[discard, draw]),
        "Discard up to two cards, then draw that many cards"
    );
}

#[test]
fn targeted_damage_does_not_rebind_an_explicit_you_discard_draw_sequence() {
    let discard_id = crate::effect::EffectId(41);
    let hand_count =
        Value::CardsInHand(PlayerFilter::You).with_surface_hint(ValueSurfaceHint::AllCardsInHand);
    let damage = Effect::deal_damage(
        Value::Count(
            ObjectFilter::default()
                .in_zone(Zone::Hand)
                .owned_by(PlayerFilter::You),
        ),
        ChooseSpec::AnyTarget,
    )
    .tag("damaged_0");
    let discard = Effect::with_id(
        discard_id.0,
        Effect::new(crate::effects::DiscardEffect::new(
            hand_count,
            PlayerFilter::You,
            false,
        )),
    );
    let draw = Effect::new(crate::effects::DrawCardsEffect::new(
        Value::EffectMetric {
            effect_id: discard_id,
            source: crate::effect::EffectMetricSource::Outcome,
            metric: crate::effect::EffectMetric::Count,
        },
        PlayerFilter::You,
    ));

    let rendered = describe_pre_clause_structural_effect_list(&[damage, discard, draw]).unwrap();
    assert!(
        rendered.contains(". Discard all the cards in your hand, then draw that many cards"),
        "{rendered}"
    );
    assert!(!rendered.contains("that player discards"), "{rendered}");
}

#[test]
fn selection_then_draw_keeps_later_actions_in_separate_sentences() {
    let scry_draw_lose = vec![
        Effect::scry(Value::Fixed(2)),
        Effect::draw(Value::Fixed(2)),
        Effect::new(crate::effects::LoseLifeEffect::you(2)),
    ];
    assert_eq!(
        describe_effect_clause_list(&scry_draw_lose).as_deref(),
        Some("scry 2, then draw two cards. You lose 2 life")
    );

    let surveil_draw_damage = vec![
        Effect::surveil(Value::Fixed(2)),
        Effect::draw(Value::Fixed(2)),
        Effect::new(crate::effects::DealDamageEffect::new(
            Value::Fixed(2),
            ChooseSpec::Player(PlayerFilter::You),
        )),
    ];
    assert_eq!(
        describe_effect_clause_list(&surveil_draw_damage).as_deref(),
        Some("surveil 2, then draw two cards. Deal 2 damage to you")
    );

    // A bare draw/life-loss pair is intentionally outside this structural
    // rule; many Oracle cards conjoin exactly those two instructions.
    let conjoined_draw_lose = vec![
        Effect::draw(Value::Fixed(2)),
        Effect::new(crate::effects::LoseLifeEffect::you(2)),
    ];
    assert_eq!(
        describe_effect_clause_list(&conjoined_draw_lose).as_deref(),
        Some("draw two cards and lose 2 life")
    );
}

#[test]
fn discard_hand_then_fixed_draw_stays_a_then_sequence() {
    let effects = vec![
        Effect::new(crate::effects::DiscardHandEffect::you()),
        Effect::new(crate::effects::DrawCardsEffect::you(8)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Discard your hand, then draw eight cards"
    );
}

#[test]
fn damaged_player_discard_draw_uses_the_contextual_that_player_subject() {
    let effect_id = crate::effect::EffectId(3);
    let discard = Effect::with_id(
        effect_id.0,
        Effect::new(crate::effects::DiscardEffect::new(
            Value::CardsInHand(PlayerFilter::DamagedPlayer),
            PlayerFilter::DamagedPlayer,
            false,
        )),
    );
    let draw = Effect::new(crate::effects::DrawCardsEffect::new(
        Value::EffectValue(effect_id),
        PlayerFilter::DamagedPlayer,
    ));

    assert_eq!(
        describe_effect_list(&[discard, draw]),
        "that player discards all the cards in their hand, then draws that many cards"
    );
}

#[test]
fn explicit_all_cards_in_hand_surface_survives_context_rebinding() {
    let effect_id = crate::effect::EffectId(31);
    let discard = Effect::with_id(
        effect_id.0,
        Effect::new(crate::effects::DiscardEffect::new(
            Value::CardsInHand(PlayerFilter::IteratedPlayer)
                .with_surface_hint(ValueSurfaceHint::AllCardsInHand),
            PlayerFilter::DamagedPlayer,
            false,
        )),
    );
    let draw = Effect::new(crate::effects::DrawCardsEffect::new(
        Value::EffectValue(effect_id),
        PlayerFilter::DamagedPlayer,
    ));

    assert_eq!(
        describe_effect_list(&[discard, draw]),
        "that player discards all the cards in their hand, then draws that many cards"
    );
}

#[test]
fn move_to_zone_preserves_an_explicit_contextual_destination_surface() {
    let tagged = ChooseSpec::Tagged(crate::TagKey::from("revealed_0"));
    let contextual = Effect::new(
        crate::effects::MoveToZoneEffect::new(tagged.clone(), Zone::Graveyard, false)
            .with_destination_player_surface(PlayerFilter::You),
    );
    let canonical = Effect::new(crate::effects::MoveToZoneEffect::new(
        tagged,
        Zone::Graveyard,
        false,
    ));

    assert_eq!(describe_effect(&contextual), "Put it into your graveyard");
    assert_eq!(
        describe_effect(&canonical),
        "Put it into its owner's graveyard"
    );
}

fn target_player_look_then_may_move(
    reference_surface: ironsmith_core::DestinationPlayerReferenceSurface,
) -> Vec<Effect> {
    let looked = crate::TagKey::from("looked");
    let target_player = PlayerFilter::target_player();
    vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_player(),
        )),
        Effect::new(crate::effects::LookAtTopCardsEffect::new(
            target_player.clone(),
            Value::Fixed(1),
            looked.clone(),
        )),
        Effect::may(vec![Effect::new(
            crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(looked),
                Zone::Graveyard,
                false,
            )
            .with_destination_player_surface(target_player)
            .with_destination_player_reference_surface(reference_surface),
        )]),
    ]
}

#[test]
fn target_player_look_bundle_preserves_destination_reference_surface() {
    assert_eq!(
        describe_effect_list(&target_player_look_then_may_move(
            ironsmith_core::DestinationPlayerReferenceSurface::Pronoun,
        )),
        "Look at the top card of target player's library. You may put that card into their graveyard"
    );
    assert_eq!(
        describe_effect_list(&target_player_look_then_may_move(
            ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer,
        )),
        "Look at the top card of target player's library. You may put that card into that player's graveyard"
    );
}

#[test]
fn return_to_hand_preserves_an_explicit_contextual_destination_surface() {
    let tagged = ChooseSpec::Tagged(crate::TagKey::from("returned_0"));
    let contextual = Effect::new(
        crate::effects::ReturnToHandEffect::with_spec(tagged.clone())
            .with_destination_player_surface(PlayerFilter::You),
    );
    let canonical = Effect::new(crate::effects::ReturnToHandEffect::with_spec(
        tagged.clone(),
    ));
    let contextual_from_graveyard = Effect::new(
        crate::effects::ReturnFromGraveyardToHandEffect::new(tagged, false)
            .with_destination_player_surface(PlayerFilter::DamagedPlayer),
    );

    assert_eq!(describe_effect(&contextual), "Return it to your hand");
    assert_eq!(describe_effect(&canonical), "Return it to its owner's hand");
    assert_eq!(
        describe_effect(&contextual_from_graveyard),
        "Return it from a graveyard to their hand"
    );
}

#[test]
fn destination_first_targeted_return_preserves_tagged_lesser_mana_surface() {
    let mut filter = ObjectFilter::artifact()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    filter.set_explicit_card_noun(true);
    filter.set_return_destination_first_surface(true);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("triggering"),
        relation: TaggedOpbjectRelation::ManaValueLtTagged,
    });
    let effect = Effect::new(
        crate::effects::ReturnFromGraveyardToHandEffect::new(
            ChooseSpec::target(ChooseSpec::Object(filter)),
            false,
        )
        .with_graveyard_player_surface(PlayerFilter::You)
        .with_destination_player_surface(PlayerFilter::You),
    );

    assert_eq!(
        describe_effect(&effect),
        "Return to your hand target artifact card in your graveyard with lesser mana value"
    );
}

#[test]
fn aliased_target_actions_compact_under_the_original_target_subject() {
    let alias = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent));
    let effects = vec![
        Effect::new(crate::effects::DiscardEffect::new(
            Value::Fixed(2),
            PlayerFilter::target_opponent(),
            false,
        )),
        Effect::new(crate::effects::MillEffect::new(
            Value::Fixed(2),
            alias.clone(),
        )),
        Effect::new(crate::effects::LoseLifeEffect::with_filter(
            Value::Fixed(2),
            alias,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target opponent discards two cards, mills two cards, and loses 2 life"
    );
}

#[test]
fn discard_then_aliased_graveyard_exile_keeps_the_then_surface() {
    let alias = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any));
    let discard = Effect::new(crate::effects::DiscardEffect::new(
        Value::Fixed(2),
        PlayerFilter::target_player(),
        false,
    ));
    let exile = Effect::new(crate::effects::ExileEffect::all(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(alias),
    ));

    assert_eq!(
        describe_effect_list(&[discard, exile]),
        "Target player discards two cards. Then exile that player's graveyard"
    );
}

#[test]
fn target_hand_count_uses_the_same_players_pronoun() {
    let hand = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)));
    let draw = crate::effects::DrawCardsEffect::new(
        Value::Count(hand).with_surface_hint(ValueSurfaceHint::EqualTo),
        PlayerFilter::target_player(),
    );

    assert_eq!(
        describe_draw_for_each(&draw).as_deref(),
        Some("target player draws cards equal to the number of cards in their hand")
    );
}

#[test]
fn reveal_hand_then_count_that_players_hand_keeps_both_surfaces() {
    let hand = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)));
    let effects = vec![
        Effect::new(crate::effects::LookAtHandEffect::reveal(
            ChooseSpec::target_player(),
        )),
        Effect::new(crate::effects::GainLifeEffect::you(Value::Count(hand))),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target player reveals their hand. You gain life equal to the number of cards in that player's hand"
    );
}

#[test]
fn followup_owner_and_damage_references_do_not_retarget() {
    let opponent_alias = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent));
    let owned_nonland = ObjectFilter::default()
        .without_type(CardType::Land)
        .owned_by(opponent_alias);
    assert!(owned_nonland.description().contains("that player owns"));
    assert!(!owned_nonland.description().contains("target opponent owns"));

    let player_alias = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any));
    let damage = Effect::new(crate::effects::DealDamageEffect::new(
        Value::Fixed(4),
        ChooseSpec::Player(player_alias),
    ));
    assert_eq!(describe_effect(&damage), "Deal 4 damage to that player");
}

#[test]
fn unless_damage_keeps_the_decider_causative() {
    let player_alias = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any));
    let effect = Effect::new(crate::effects::UnlessActionEffect::new(
        vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(
                ChooseSpec::target_player(),
            )),
            Effect::new(crate::effects::DiscardEffect::new(
                Value::Fixed(2),
                PlayerFilter::target_player(),
                true,
            )),
        ],
        vec![Effect::new(crate::effects::DealDamageEffect::new(
            Value::Fixed(4),
            ChooseSpec::Player(PlayerFilter::target_player()),
        ))],
        player_alias,
    ));

    assert_eq!(
        describe_effect(&effect),
        "Target player discards two cards at random unless that player has this source deal 4 damage to them"
    );
}

#[test]
fn reveal_choose_exile_then_cast_uses_the_selected_opponents_owner_alias() {
    let chosen = TagKey::from("__it__");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .without_type(CardType::Land)
            .owned_by(PlayerFilter::target_opponent()),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        chosen.clone(),
    )
    .in_zones(vec![Zone::Graveyard, Zone::Hand]);
    let effects = vec![
        Effect::new(crate::effects::LookAtHandEffect::reveal(
            ChooseSpec::target_opponent(),
        )),
        Effect::new(choose),
        Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(chosen),
            Zone::Exile,
            true,
        )),
        Effect::new(crate::effects::GrantPlayTaggedEffect::new(
            "__source_exiled__",
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled,
            false,
            true,
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target opponent reveals their hand. You choose a nonland card from that player's graveyard or hand and exile it. You may cast that card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell"
    );
}

#[test]
fn target_only_plus_revealing_look_introduces_the_target() {
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_opponent(),
        )),
        Effect::new(crate::effects::LookAtTopCardsEffect::revealing(
            PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent)),
            Value::Fixed(2),
            crate::TagKey::from("revealed"),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Target opponent reveals the top two cards of their library"
    );
}

#[test]
fn draw_then_conditional_discard_renders_positive_unless_condition() {
    let effects = vec![
        Effect::draw(Value::Fixed(2)),
        Effect::new(crate::effects::ConditionalEffect::new(
            Condition::Not(Box::new(Condition::AttackedThisTurn)),
            vec![Effect::new(crate::effects::DiscardEffect::new(
                Value::Fixed(1),
                PlayerFilter::You,
                false,
            ))],
            Vec::new(),
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Draw two cards, then discard a card unless you attacked this turn"
    );
}

#[test]
fn source_sentence_leading_then_surface_renders_the_explicit_connective() {
    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::draw(Value::Fixed(1))]),
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::new(
            crate::effects::SequenceEffect::sentence_leading_then(vec![Effect::gain_life(1)]),
        )]),
    ]);

    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "you draw a card. Then you gain 1 life"
    );
}

#[test]
fn ordinary_sequential_sentence_does_not_gain_a_then_connective() {
    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::draw(Value::Fixed(1))]),
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::new(
            crate::effects::SequenceEffect::new(vec![Effect::gain_life(1)]),
        )]),
    ]);

    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "you draw a card. you gain 1 life"
    );
}

#[test]
fn modal_modes_with_shared_spells_cast_value_render_one_x_preamble() {
    let spells_cast = Value::SpellsCastThisTurn(PlayerFilter::You);
    let choose = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new(
            "Scry X.",
            vec![Effect::new(crate::effects::ScryEffect::you(
                spells_cast.clone(),
            ))],
        ),
        ironsmith_core::EffectMode::new(
            "This creature deals X damage to target creature.",
            vec![
                Effect::new(crate::effects::DealDamageEffect::new(
                    spells_cast.clone(),
                    ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
                ))
                .tag(TagKey::from("damaged_0")),
            ],
        ),
        ironsmith_core::EffectMode::new(
            "You gain X life.",
            vec![Effect::new(crate::effects::GainLifeEffect::you(
                spells_cast,
            ))],
        ),
    ]);

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Choose one. X is the number of spells you've cast this turn —\n\
         • Scry X.\n\
         • This creature deals X damage to target creature.\n\
         • You gain X life."
    );
}

#[test]
fn modal_modes_with_different_spells_cast_values_do_not_invent_shared_x() {
    let choose = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new(
            "Scry X.",
            vec![Effect::new(crate::effects::ScryEffect::you(
                Value::SpellsCastThisTurn(PlayerFilter::You),
            ))],
        ),
        ironsmith_core::EffectMode::new(
            "You gain X life.",
            vec![Effect::new(crate::effects::GainLifeEffect::you(
                Value::SpellsCastThisTurn(PlayerFilter::Opponent),
            ))],
        ),
    ]);

    let rendered = describe_effect(&Effect::new(choose));
    assert!(rendered.starts_with("Choose one —"), "{rendered}");
    assert!(!rendered.contains(". X is "), "{rendered}");
}

#[test]
fn modal_common_return_suffix_renders_once_before_target_modes() {
    let return_mode = |card_type, source_text| {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
        filter.card_types = vec![card_type];
        filter.set_explicit_card_noun(true);
        let returned = crate::effects::ReturnFromGraveyardToHandEffect::new(
            ChooseSpec::target(ChooseSpec::Object(filter)),
            false,
        )
        .with_graveyard_player_surface(PlayerFilter::You)
        .with_destination_player_surface(PlayerFilter::You);
        ironsmith_core::EffectMode::new(source_text, vec![Effect::new(returned)])
    };
    let choose = ironsmith_core::ChooseModeEffect::choose_up_to(
        2,
        0,
        vec![
            return_mode(CardType::Artifact, "Target artifact card."),
            return_mode(CardType::Creature, "Target creature card."),
            return_mode(CardType::Enchantment, "Target enchantment card."),
            return_mode(CardType::Land, "Target land card."),
        ],
    )
    .with_common_suffix_effect_count(1);

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Choose up to two. Return those cards from your graveyard to your hand.\n\
• Target artifact card.\n\
• Target creature card.\n\
• Target enchantment card.\n\
• Target land card."
    );
}

#[test]
fn modal_common_return_suffix_parses_bare_target_bullets_through_public_route() {
    let oracle = "Choose up to two. Return those cards from your graveyard to your hand.\n\
• Target artifact card.\n\
• Target creature card.\n\
• Target enchantment card.\n\
• Target land card.";
    let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Common Modal Target Suffix Probe",
    )
    .card_types(vec![CardType::Sorcery])
    .parse_text(oracle)
    .expect("a common return suffix should specialize every bare target bullet");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );

    let unsupported = crate::compiler_test_support::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Bare Modal Target Near Miss",
    )
    .card_types(vec![CardType::Sorcery])
    .parse_text("Choose up to two.\n• Target artifact card.\n• Target creature card.");
    assert!(
        unsupported.is_err(),
        "bare targets without a typed common action must remain unsupported"
    );
}

#[test]
fn repeated_modal_choices_keep_each_mode_on_a_bullet_line() {
    let choose = ironsmith_core::ChooseModeEffect::choose_exactly(
        Value::Fixed(4),
        vec![
            ironsmith_core::EffectMode::new("Draw a card.", vec![Effect::draw(Value::Fixed(1))]),
            ironsmith_core::EffectMode::new(
                "You gain 1 life.",
                vec![Effect::gain_life(Value::Fixed(1))],
            ),
        ],
    )
    .with_repeated_modes();

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Choose four. You may choose the same mode more than once.\n\
         • Draw a card.\n\
         • You gain 1 life."
    );
}

#[test]
fn modal_common_prefix_renders_once_in_the_header() {
    let choose = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new("Draw a card.", vec![Effect::draw(Value::Fixed(1))]),
        ironsmith_core::EffectMode::new(
            "You gain 1 life.",
            vec![Effect::gain_life(Value::Fixed(1))],
        ),
    ])
    .with_common_prefix_effects(vec![Effect::gain_life(Value::Fixed(3))]);

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Choose one and you gain 3 life.\n• Draw a card.\n• You gain 1 life."
    );

    let ordinary = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new("Draw a card.", vec![Effect::draw(Value::Fixed(1))]),
        ironsmith_core::EffectMode::new(
            "You gain 1 life.",
            vec![Effect::gain_life(Value::Fixed(1))],
        ),
    ]);
    assert_eq!(
        describe_effect(&Effect::new(ordinary)),
        "Choose one —\n• Draw a card.\n• You gain 1 life."
    );
}

#[test]
fn inline_named_token_creation_choice_renders_as_authored_or_instruction() {
    let named_artifact_token = |name, subtype| {
        crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
            .token()
            .card_types(vec![crate::types::CardType::Artifact])
            .subtypes(vec![subtype])
            .build()
    };
    let choose = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new(
            "",
            vec![Effect::new(crate::effects::CreateTokenEffect::one(
                named_artifact_token("Food", crate::types::Subtype::Food),
            ))],
        ),
        ironsmith_core::EffectMode::new(
            "",
            vec![Effect::new(crate::effects::CreateTokenEffect::one(
                named_artifact_token("Treasure", crate::types::Subtype::Treasure),
            ))],
        ),
    ])
    .with_chooser(PlayerFilter::You);

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Create a Food token or a Treasure token"
    );
}

#[test]
fn inline_destroy_all_choice_renders_as_authored_or_instruction() {
    let choose = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new(
            "",
            vec![Effect::new(crate::effects::DestroyEffect::all(
                ObjectFilter::default().with_type(crate::types::CardType::Land),
            ))],
        ),
        ironsmith_core::EffectMode::new(
            "",
            vec![Effect::new(crate::effects::DestroyEffect::all(
                ObjectFilter::creature(),
            ))],
        ),
    ])
    .with_chooser(PlayerFilter::You);

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Destroy all lands or all creatures"
    );
}

#[test]
fn inline_destroy_all_choice_keeps_shared_no_regeneration_followup() {
    let choose = ironsmith_core::ChooseModeEffect::choose_one(vec![
        ironsmith_core::EffectMode::new(
            "",
            vec![Effect::new(
                crate::effects::DestroyNoRegenerationEffect::all(
                    ObjectFilter::default().with_type(crate::types::CardType::Land),
                ),
            )],
        ),
        ironsmith_core::EffectMode::new(
            "",
            vec![Effect::new(
                crate::effects::DestroyNoRegenerationEffect::all(ObjectFilter::creature()),
            )],
        ),
    ])
    .with_chooser(PlayerFilter::You);

    assert_eq!(
        describe_effect(&Effect::new(choose)),
        "Destroy all lands or all creatures. They can't be regenerated"
    );
}

fn source_exiled_filter() -> ObjectFilter {
    ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile)
}

#[test]
fn source_exiled_move_preserves_all_source_type_and_plural_owners() {
    let surface = ironsmith_core::ExiledWithSourceMoveSurface {
        verb: ironsmith_core::ExiledWithSourceMoveVerbSurface::Put,
        subject: ironsmith_core::ExiledWithSourceSubjectSurface::AllCards,
        source: ironsmith_core::ExiledWithSourceReferenceSurface::Source(
            crate::target::SourceReferenceSurface::ThisPermanentType("this creature".to_string()),
        ),
        destination: ironsmith_core::ExiledWithSourceDestinationSurface::TheirOwners,
    };
    let effect = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::All(source_exiled_filter()),
        Zone::Hand,
        false,
    )
    .with_exiled_with_source_surface(surface);

    assert_eq!(
        describe_effect(&Effect::new(effect)),
        "Put all cards exiled with this creature into their owners' hands"
    );
}

#[test]
fn source_exiled_return_preserves_one_card_and_contextual_hand() {
    let surface = ironsmith_core::ExiledWithSourceMoveSurface {
        verb: ironsmith_core::ExiledWithSourceMoveVerbSurface::Put,
        subject: ironsmith_core::ExiledWithSourceSubjectSurface::OneCard,
        source: ironsmith_core::ExiledWithSourceReferenceSurface::Source(
            crate::target::SourceReferenceSurface::ThisPermanentType(
                "this enchantment".to_string(),
            ),
        ),
        destination: ironsmith_core::ExiledWithSourceDestinationSurface::ContextualPlayer,
    };
    let effect = crate::effects::ReturnToHandEffect::all(source_exiled_filter())
        .with_destination_player_surface(PlayerFilter::You)
        .with_exiled_with_source_surface(surface);

    assert_eq!(
        describe_effect(&Effect::new(effect)),
        "Put a card exiled with this enchantment into your hand"
    );
}

#[test]
fn source_exiled_move_preserves_each_it_and_singular_owner() {
    let surface = ironsmith_core::ExiledWithSourceMoveSurface {
        verb: ironsmith_core::ExiledWithSourceMoveVerbSurface::Put,
        subject: ironsmith_core::ExiledWithSourceSubjectSurface::EachCard,
        source: ironsmith_core::ExiledWithSourceReferenceSurface::It,
        destination: ironsmith_core::ExiledWithSourceDestinationSurface::ItsOwner,
    };
    let effect = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::All(source_exiled_filter()),
        Zone::Graveyard,
        false,
    )
    .with_exiled_with_source_surface(surface);

    assert_eq!(
        describe_effect(&Effect::new(effect)),
        "Put each card exiled with it into its owner's graveyard"
    );
}

#[test]
fn source_exiled_battlefield_move_preserves_typed_subject_and_controller() {
    let surface = ironsmith_core::ExiledWithSourceMoveSurface {
        verb: ironsmith_core::ExiledWithSourceMoveVerbSurface::Put,
        subject: ironsmith_core::ExiledWithSourceSubjectSurface::Custom(
            "each creature card".to_string(),
        ),
        source: ironsmith_core::ExiledWithSourceReferenceSurface::Source(
            crate::target::SourceReferenceSurface::ThisPermanentType("this artifact".to_string()),
        ),
        destination: ironsmith_core::ExiledWithSourceDestinationSurface::ContextualPlayer,
    };
    let effect = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::All(source_exiled_filter()),
        Zone::Battlefield,
        false,
    )
    .tapped()
    .under_you_control()
    .with_exiled_with_source_surface(surface);

    assert_eq!(
        describe_effect(&Effect::new(effect)),
        "Put each creature card exiled with this artifact onto the battlefield tapped under your control"
    );
}

#[test]
fn source_exiled_return_preserves_definite_singular_without_source_suffix() {
    let surface = ironsmith_core::ExiledWithSourceMoveSurface {
        verb: ironsmith_core::ExiledWithSourceMoveVerbSurface::Return,
        subject: ironsmith_core::ExiledWithSourceSubjectSurface::TheExiledCard,
        source: ironsmith_core::ExiledWithSourceReferenceSurface::Omitted,
        destination: ironsmith_core::ExiledWithSourceDestinationSurface::ItsOwner,
    };
    let effect = crate::effects::ReturnToHandEffect::all(source_exiled_filter())
        .with_exiled_with_source_surface(surface);

    assert_eq!(
        describe_effect(&Effect::new(effect)),
        "Return the exiled card to its owner's hand"
    );
}

#[test]
fn behold_surfaces_preserve_subtype_optional_verb_and_condition_provenance() {
    let behold = crate::effects::BeholdEffect::you(Subtype::Dragon, 1);
    assert_eq!(
        describe_effect(&Effect::new(behold.clone())),
        "Behold a Dragon"
    );

    let optional_cost = crate::cost::OptionalCost::custom(
        "As an additional cost to cast this spell, you may behold a Dragon.",
        crate::cost::TotalCost::from_cost(crate::costs::Cost::effect(behold)),
    );
    assert_eq!(
        describe_optional_cost_line(&optional_cost),
        "As an additional cost to cast this spell, you may behold a Dragon"
    );

    let typed_behold = crate::cost::OptionalCostRef::with_discriminator(
        crate::cost::OptionalCostKind::Behold,
        "Dragon",
    );
    assert_eq!(
        describe_condition(&Condition::ThisSpellPaidLabel(typed_behold.clone())),
        "a Dragon was beheld"
    );
    assert_eq!(
        describe_condition(&Condition::Or(
            Box::new(Condition::ThisSpellPaidLabel(typed_behold)),
            Box::new(Condition::PlayerControls {
                player: PlayerFilter::You,
                filter: ObjectFilter::default().with_subtype(Subtype::Dragon),
            }),
        )),
        "you revealed a Dragon card or controlled a Dragon as you cast this spell"
    );
    assert_eq!(
        describe_condition(&Condition::ThisSpellPaidLabel(
            crate::cost::OptionalCostKind::Behold.into(),
        )),
        "this spell's behold cost was paid"
    );
}
#[test]
fn demonstrative_for_each_damage_keeps_each_of_those_surface() {
    let tag = TagKey::from("chosen_targets");
    let mut filter = ObjectFilter::creature()
        .in_zone(Zone::Battlefield)
        .match_tagged(tag, crate::filter::TaggedOpbjectRelation::IsTaggedObject);
    filter.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Those));
    let damage = Effect::new(crate::effects::ForEachObject::new(
        filter,
        vec![Effect::new(crate::effects::DealDamageEffect::new(
            Value::X,
            ChooseSpec::Iterated,
        ))],
    ));

    assert_eq!(
        describe_effect(&damage),
        "Deal X damage to each of those creatures"
    );
}

#[test]
fn blitzwing_random_keyword_choice_keeps_random_marker() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(), "Blitzwing, Adaptive Assailant",
    )
    .card_types(vec![CardType::Artifact])
    .parse_text("At the beginning of combat on your turn, choose flying or indestructible at random. Blitzwing gains that ability until end of turn.")
    .expect("random keyword choice should compile");
    let ability = definition.abilities.iter().find_map(|ability| {
        if let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind {
            Some(triggered)
        } else { None }
    }).unwrap();
    let effects = ability.effects.flattened_default_effects();
    let random = effects.iter().find(|effect|
        effect.downcast_ref::<crate::effects::ChooseModeEffect>().is_some()).unwrap();
    assert!(describe_compact_keyword_change_choice(random).is_none(),
        "the earlier generic change renderer must not hide randomness");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("choose flying or indestructible at random"), "{rendered}");
    assert!(rendered.contains("gains that ability until end of turn"), "{rendered}");
    assert!(!rendered.contains("your choice of flying or indestructible"), "{rendered}");
}

#[test]
fn attached_aura_count_damage_names_each_recipient_and_keeps_multiplier() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(), "Baki's Curse",
    )
    .card_types(vec![CardType::Sorcery])
    .parse_text("Baki's Curse deals 2 damage to each creature for each Aura attached to that creature.")
    .expect("per-recipient attachment count should compile");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("deals 2 damage to each creature for each Aura attached to that creature"), "{rendered}");
    assert!(!rendered.contains("attached to its"), "{rendered}");

    let mut count_filter = ObjectFilter::default()
        .with_subtype(crate::types::Subtype::Aura)
        .in_zone(Zone::Battlefield);
    count_filter.tagged_constraints.push(crate::filter::TaggedObjectConstraint {
        tag: TagKey::from("__it__"),
        relation: crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject,
    });
    let amount = Value::Scaled(
        Box::new(Value::Count(count_filter.clone()).with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach)),
        2,
    );
    let mut damage = crate::effects::DealDamageEachEffect {
        amount,
        filter: ObjectFilter::creature().in_zone(Zone::Battlefield),
    };
    assert!(super::super::single_effects_early::describe_attached_count_damage_each(&damage).is_some());
    count_filter.tagged_constraints[0].relation = crate::filter::TaggedOpbjectRelation::IsTaggedObject;
    damage.amount = Value::Scaled(
        Box::new(Value::Count(count_filter).with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach)),
        2,
    );
    assert!(super::super::single_effects_early::describe_attached_count_damage_each(&damage).is_none());
}

#[test]
fn compared_land_count_difference_damage_keeps_recipient_before_amount() {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(), "Spiteful Repossession",
    )
    .card_types(vec![CardType::Sorcery])
    .parse_text("Spiteful Repossession deals damage to each opponent who controls more lands than you equal to the difference. Then create a number of Treasure tokens equal to the damage dealt this way.")
    .expect("difference-valued damage should compile");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("deals damage to each opponent who controls more lands than you equal to the difference"), "{rendered}");
    assert!(!rendered.contains("lands that player controls minus"), "{rendered}");

    let left = Value::Count(ObjectFilter::land().controlled_by(PlayerFilter::IteratedPlayer).in_zone(Zone::Battlefield));
    let right = Value::Count(ObjectFilter::land().controlled_by(PlayerFilter::You).in_zone(Zone::Battlefield));
    let condition = crate::effect::Condition::ValueComparison {
        left: left.clone(),
        operator: crate::effect::ValueComparisonOperator::GreaterThan,
        right: right.clone(),
    };
    let amount = Value::Add(Box::new(left.clone()), Box::new(Value::Scaled(Box::new(right.clone()), -1)));
    assert!(super::super::single_effects_early::is_positive_comparison_difference_damage(&condition, &amount));
    let reversed = Value::Add(Box::new(right), Box::new(Value::Scaled(Box::new(left), -1)));
    assert!(!super::super::single_effects_early::is_positive_comparison_difference_damage(&condition, &reversed));
}
