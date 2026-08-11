#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::*;

#[test]
pub(super) fn clown_car_parses_roll_x_six_sided_dice_with_odd_even_result_clauses() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Clown Car")
        .parse_text(
            "When this Vehicle enters, roll X six-sided dice. For each odd result, create a 1/1 white Clown Robot artifact creature token. For each even result, put a +1/+1 counter on this Vehicle.\nCrew 2",
        )
        .expect("Clown Car text should parse");
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("RepeatEffects")
            && debug.contains("RollDieEffect")
            && debug.contains("OneOf([1, 3, 5])")
            && debug.contains("OneOf([2, 4, 6])")
            && debug.contains("CreateToken")
            && debug.contains("PutCounter")
            && debug.contains("Clown")
            && debug.contains("Robot"),
        "expected repeat roll plus odd/even result branches for Clown Car, got {debug}"
    );
}

#[test]
pub(super) fn mill_then_compound_payment_if_you_do_choice_uses_milled_cards() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ripples-like Enchantment")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of your first main phase, mill three cards. Then you may pay {1} and 3 life. If you do, put a card from among those cards into your hand.",
        )
        .expect("milled-card payment follow-up should parse");
    let debug = format!("{:#?}", def.abilities);

    assert!(debug.contains("TaggedEffect"), "{debug}");
    assert!(debug.contains("milled_0"), "{debug}");
    assert!(debug.contains("PayManaEffect"), "{debug}");
    assert!(debug.contains("PayLifeEffect"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
    assert!(
        !debug.contains("Library"),
        "milled-card choice should not look back into the library: {debug}"
    );
}

#[test]
pub(super) fn fixed_life_payment_parser_preserves_payment_action() {
    let tokens =
        lex_line("That player pays 2 life.", 0).expect("fixed life-payment sentence should lex");
    let effects =
        parse_effect_sentence_lexed(&tokens).expect("fixed life-payment sentence should parse");

    let [EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
        panic!("expected one subject-verb payment, got {effects:#?}");
    };
    assert!(
        matches!(
            &subject_verb.action,
            SubjectVerbActionAst::PayLife {
                amount: Value::Fixed(2)
            }
        ),
        "authored payment must not collapse into ordinary life loss: {subject_verb:#?}"
    );
}

#[test]
pub(super) fn typed_backup_actions_preserve_boundaries_and_never_grant_generated_backup_triggers() {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Multiple Backup")
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Backup 1\nFlying\nBackup 2\nVigilance".to_string(),
        false,
    )
    .expect("typed Backup keyword lines should lower");

    assert_eq!(definition.abilities.len(), 4);
    let ability_ids = |abilities: &[crate::ability::Ability]| {
        abilities
            .iter()
            .map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => ability.id(),
                other => panic!("Backup should grant only actual trailing abilities: {other:?}"),
            })
            .collect::<Vec<_>>()
    };
    let backup_at = |index: usize| {
        let AbilityKind::Triggered(triggered) = &definition.abilities[index].kind else {
            panic!("expected Backup trigger at ability {index}");
        };
        let effects = triggered.effects.to_vec();
        let backup = effects
            .first()
            .and_then(|effect| {
                effect.downcast_ref::<crate::effects::BackupEffect<crate::ability::Ability>>()
            })
            .expect("generated ETB ability should contain BackupEffect");
        (backup.amount, ability_ids(&backup.granted_abilities))
    };

    assert_eq!(
        backup_at(0),
        (1, vec![StaticAbilityId::Flying, StaticAbilityId::Vigilance])
    );
    assert!(matches!(
        &definition.abilities[1].kind,
        AbilityKind::Static(ability) if ability.id() == StaticAbilityId::Flying
    ));
    assert_eq!(backup_at(2), (2, vec![StaticAbilityId::Vigilance]));
    assert!(matches!(
        &definition.abilities[3].kind,
        AbilityKind::Static(ability) if ability.id() == StaticAbilityId::Vigilance
    ));
}

#[test]
pub(super) fn typed_cipher_action_appends_resolution_effect_without_marker_ability() {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Typed Cipher")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) =
        parse_text_with_annotations_lowered(builder, "Draw a card.\nCipher".to_string(), false)
            .expect("typed Cipher keyword line should lower");

    assert!(
        definition.abilities.is_empty(),
        "Cipher must not survive lowering as a marker ability"
    );
    let effects = definition
        .spell_effect
        .as_ref()
        .expect("Cipher spell should have a resolution program")
        .to_vec();
    assert_eq!(
        effects
            .iter()
            .filter(|effect| effect
                .downcast_ref::<crate::effects::CipherEffect>()
                .is_some())
            .count(),
        1,
        "Cipher should append exactly one typed resolution effect"
    );
    let segments = &definition
        .spell_effect
        .as_ref()
        .expect("Cipher spell should have a resolution program")
        .segments;
    assert_eq!(segments.len(), 2, "Cipher should retain its source line");
    assert!(
        segments[1].starts_new_source_line,
        "Cipher should begin a new authored source line"
    );
}

#[test]
pub(super) fn exile_play_event_followups_lower_as_reflexive_or_delayed_triggers() {
    let reflexive = CardDefinitionBuilder::new(CardId::new(), "Reflexive Exile Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{2}{R}, {T}: Exile the top card of your library. You may play that card this turn. When you exile a nonland card this way, this creature deals damage equal to the exiled card's mana value to any target.",
        )
        .expect("nonland exile followup should parse");
    let reflexive_debug = format!("{:#?}", reflexive.abilities);
    assert!(
        reflexive_debug.contains("ReflexiveTriggerEffect")
            && reflexive_debug.contains("AffectedObjectMatchesCardType")
            && reflexive_debug.contains("Land")
            && reflexive_debug.contains("negated: true"),
        "{reflexive_debug}"
    );

    let delayed = CardDefinitionBuilder::new(CardId::new(), "Delayed Play Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "{2}{R}: Exile the top card of your library. You may play that card this turn. When you play a card this way, this enchantment deals 2 damage to each player.",
        )
        .expect("play-this-way followup should parse");
    let delayed_debug = format!("{:#?}", delayed.abilities);
    assert!(
        delayed_debug.contains("ScheduleDelayedTriggerEffect")
            && delayed_debug.contains("SpellCast")
            && delayed_debug.contains("PlayerPlaysLand"),
        "{delayed_debug}"
    );
}

#[test]
pub(super) fn repeated_payment_reflexive_count_stays_on_the_enter_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Repeated Payment Adversary")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, you may pay {1}{U} any number of times. \
             When you pay this cost one or more times, put that many +1/+1 counters on this creature, then up to that many other target artifacts, creatures, and/or enchantments phase out.",
        )
        .expect("repeated-payment reflexive trigger should parse");
    let debug = format!("{:#?}", def);

    assert!(debug.contains("RepeatProcessEffect"), "{debug}");
    assert!(debug.contains("ReflexiveTriggerEffect"), "{debug}");
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(debug.contains("PhaseOutEffect"), "{debug}");
    assert!(debug.contains("WithCountValue"), "{debug}");
    assert!(
        !debug.contains("spell_effect: Some"),
        "the reflexive continuation must stay on the enters trigger: {debug}"
    );
}

#[test]
pub(super) fn until_end_of_turn_instant_timing_payment_becomes_repeatable_special_action() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Repeatable Prevention Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Prevent the next X damage that would be dealt to any target this turn. Until end of turn, you may pay {1} any time you could cast an instant. If you do, prevent the next 1 damage that would be dealt to that permanent or player this turn.",
        )
        .expect("duration-scoped repeatable payment should parse");
    let debug = format!("{def:#?}");

    assert!(
        debug.contains("GrantRepeatableManaPaymentActionUntilEndOfTurnEffect"),
        "{debug}"
    );
    assert!(debug.contains("cost: ManaCost"), "{debug}");
    assert!(debug.matches("PreventDamageEffect").count() >= 2, "{debug}");
    assert!(debug.matches("target: AnyTarget").count() >= 2, "{debug}");
    assert!(!debug.contains("MayEffect"), "{debug}");
    assert!(!debug.contains("IfEffect"), "{debug}");
}

#[test]
pub(super) fn typed_counter_where_x_carries_into_payment_and_result_followup() {
    fn is_source_plus_one_counter_count(value: &Value) -> bool {
        match value {
            Value::SurfaceHinted { value, .. } => is_source_plus_one_counter_count(value),
            Value::CountersOnSource(CounterType::PlusOnePlusOne) => true,
            _ => false,
        }
    }

    let builder = CardDefinitionBuilder::new(CardId::new(), "Primordial Ooze")
        .card_types(vec![CardType::Creature]);
    let (document, _) = parse_text_to_semantic_document(
        builder.clone(),
        "At the beginning of your upkeep, put a +1/+1 counter on this creature. Then you may pay {X}, where X is the number of +1/+1 counters on it. If you don't, tap this creature and it deals X damage to you.".to_string(),
        false,
    )
    .expect("semantic parse should succeed before reference preparation");
    let effects = document
        .items
        .iter()
        .find_map(|item| rewrite_direct_triggered_chunk(item).map(|(_, effects, _)| effects))
        .expect("expected one typed triggered ability");
    fn source_sentence_effects<'a>(effect: &'a EffectAst) -> &'a [EffectAst] {
        match effect {
            EffectAst::SourceSentence { effects, .. } => effects.as_slice(),
            other => std::slice::from_ref(other),
        }
    }
    let payment_sentence = source_sentence_effects(&effects[1]);
    let EffectAst::MayByPlayer {
        effects: payment_effects,
        ..
    } = payment_sentence
        .first()
        .expect("payment sentence should contain an effect")
    else {
        panic!("expected optional X payment: {effects:#?}");
    };
    assert!(matches!(
        payment_effects.as_slice(),
        [EffectAst::SubjectVerb(subject_verb)]
            if matches!(
                &subject_verb.action,
                SubjectVerbActionAst::PayMana { x_value: Some(value), .. }
                    if is_source_plus_one_counter_count(value)
            )
    ));
    let decline_sentence = source_sentence_effects(&effects[2]);
    let EffectAst::IfResult {
        effects: decline_effects,
        ..
    } = decline_sentence
        .first()
        .expect("decline sentence should contain an effect")
    else {
        panic!("expected decline follow-up: {effects:#?}");
    };
    fn contains_typed_x_damage(effects: &[EffectAst]) -> bool {
        effects.iter().any(|effect| match effect {
            EffectAst::SubjectVerb(subject_verb) => matches!(
                &subject_verb.action,
                SubjectVerbActionAst::DealDamage { amount, .. }
                    | SubjectVerbActionAst::DealDamageEqualToPower { amount, .. }
                    if is_source_plus_one_counter_count(amount)
            ),
            EffectAst::Coordinated { effects, .. } => contains_typed_x_damage(effects),
            _ => false,
        })
    }
    assert!(
        contains_typed_x_damage(decline_effects),
        "expected the decline damage to reuse the typed X binding: {decline_effects:#?}"
    );

    builder
        .parse_text(
            "At the beginning of your upkeep, put a +1/+1 counter on this creature. Then you may pay {X}, where X is the number of +1/+1 counters on it. If you don't, tap this creature and it deals X damage to you.",
        )
        .expect("typed counter-defined X should survive preparation and lowering");
}

#[test]
pub(super) fn arcee_sharpshooter_counter_removal_cost_binds_that_much_damage() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Arcee, Sharpshooter")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text(
            "{1}, Remove one or more +1/+1 counters from Arcee: It deals that much damage to target creature. Convert Arcee.",
        )
        .expect("Arcee's counter-removal amount should lower as activation X");
    let debug = format!("{:#?}", definition.abilities);
    assert!(
        debug.contains("RemoveAnyCountersAmongEffect")
            && debug.contains("min_count: 1")
            && debug.contains("dynamic_count: true"),
        "{debug}"
    );
    assert!(
        debug.contains("DealDamageEffect") && debug.contains("amount: X"),
        "{debug}"
    );
}

#[test]
pub(super) fn living_metal_lowers_to_the_reusable_keyword_ability() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Living Metal Vehicle")
        .card_types(vec![CardType::Artifact])
        .parse_text("Living metal (During your turn, this Vehicle is also a creature.)")
        .expect("living metal and its reminder text should parse");
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("LivingMetal"), "{debug}");
}

#[test]
pub(super) fn arcee_acrobatic_coupe_binds_that_many_to_qualifying_spell_targets() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Arcee, Acrobatic Coupe")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever you cast a spell that targets one or more creatures or Vehicles you control, put that many +1/+1 counters on Arcee. Convert Arcee.",
        )
        .expect("Arcee's spell-target trigger should bind that many to its matching targets");
    let debug = format!("{:#?}", definition.abilities);
    assert!(
        debug.contains("SpellCastTrigger") || debug.contains("SpellCastQualified"),
        "{debug}"
    );
    assert!(debug.contains("targets_object: Some"), "{debug}");
    assert!(
        debug.contains("Creature") && debug.contains("Vehicle"),
        "{debug}"
    );
    assert!(debug.contains("type_or_subtype_union: true"), "{debug}");
    assert!(
        debug.contains("EventValue") && debug.contains("Amount"),
        "{debug}"
    );
    assert!(debug.contains("PutCountersEffect"), "{debug}");
}

#[test]
pub(super) fn geistflame_reservoir_counter_removal_cost_binds_that_much_damage() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Geistflame Reservoir")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{1}{R}, {T}, Remove any number of charge counters from this artifact: It deals that much damage to any target.",
        )
        .expect("Geistflame Reservoir's counter-removal amount should lower as activation X");
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("RemoveAnyCountersFromSource"), "{debug}");
    assert!(
        debug.contains("DealDamageEffect") && debug.contains("amount: X"),
        "{debug}"
    );
}

#[test]
pub(super) fn dragonspark_reactor_reuses_first_damage_amount_for_second_target() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Dragonspark Reactor")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{4}, Sacrifice this artifact: It deals damage equal to the number of charge counters on it to target player and that much damage to up to one target creature.",
        )
        .expect("Dragonspark Reactor's second damage amount should reuse the first amount");
    let debug = format!("{:#?}", definition.abilities);
    let compact = debug.split_whitespace().collect::<String>();
    assert!(debug.matches("DealDamageEffect").count() >= 2, "{debug}");
    assert!(
        debug.contains("Charge") && debug.contains("EffectValue"),
        "{debug}"
    );
    assert_eq!(
        compact
            .matches("ExecuteWithSourceEffect{source:SurfaceHinted{spec:Source")
            .count(),
        2,
        "both conjoined damage effects must use the sacrificed artifact as their source: {debug}"
    );
    assert!(
        !compact.contains("zone:Some(Hand)"),
        "the second damage source must remain the sacrificed artifact, not a card in the damaged player's hand: {debug}"
    );
}

#[test]
pub(super) fn bionic_blow_parses_anaphoric_power_damage_with_optional_other_target() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Bionic Blow")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Target creature you control gets +X/+0 until end of turn. Then it deals damage equal to its power to up to one other target creature.",
        )
        .expect("Bionic Blow's anaphoric power damage should parse structurally");
    let debug = format!("{:#?}", definition.spell_effect);
    assert!(debug.contains("DealDamageEffect"), "{debug}");
    assert!(debug.contains("PowerOf"), "{debug}");
}

#[test]
pub(super) fn ink_dissolver_kinship_keeps_shared_creature_type_condition() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Ink Dissolver")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Kinship — At the beginning of your upkeep, you may look at the top card of your library. If it shares a creature type with this creature, you may reveal it. If you do, each opponent mills three cards.",
        )
        .expect("Ink Dissolver's Kinship condition should parse structurally");
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("TaggedObjectMatches"), "{debug}");
    assert!(
        debug.contains("shares_creature_type_with_source: true"),
        "{debug}"
    );
}

#[test]
pub(super) fn daxos_permission_keeps_exiled_card_and_any_color_mana_suffix() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Daxos of Meletis")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature deals combat damage to a player, exile the top card of that player's library. You gain life equal to that card's mana value. Until end of turn, you may cast that card and you may spend mana as though it were mana of any color to cast that spell.",
        )
        .expect("Daxos's exile, life-gain, and cast-permission chain should parse");
    let debug = format!("{:#?}", definition.abilities);
    let compact = debug.split_whitespace().collect::<String>();
    assert!(debug.contains("GainLifeEffect"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
    assert!(
        compact.contains("GrantPlayTaggedEffect{tag:TagKey(\"__sentence_helper_exiled"),
        "{debug}"
    );
    assert!(debug.contains("allow_any_color_for_cast: true"), "{debug}");
}
