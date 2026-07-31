#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::*;
use crate::target::ObjectFilter;

#[test]
pub(super) fn legendary_creatures_gain_typed_bands_with_other_quality() {
    let tokens = lex_line(
        "Red legendary creatures have bands with other legendary creatures.",
        0,
    )
    .expect("bands-with-other grant should lex");
    let parsed = super::super::keyword_static::parse_granted_keyword_static_line(&tokens)
        .expect("bands-with-other grant should parse")
        .expect("bands-with-other grant should be recognized");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("BandsWithOther"), "{debug}");
    assert!(debug.contains("Legendary"), "{debug}");
}

#[test]
pub(super) fn loses_all_bands_with_other_lowers_to_typed_ability_removal() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Shelkin Brownie Probe")
        .parse_text(
            "{T}: Target creature loses all \"bands with other\" abilities until end of turn.",
        )
        .expect("bands-with-other loss should parse");
    let debug = format!("{definition:?}");

    assert!(debug.contains("RemoveAbility"), "{debug}");
    assert!(debug.contains("BandsWithOther"), "{debug}");
}

#[test]
pub(super) fn wolves_of_the_hunt_token_gets_typed_bands_with_other_quality() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Master of the Hunt Probe")
        .parse_text(
            "{2}{G}{G}: Create a 1/1 green Wolf creature token named Wolves of the Hunt. It has \"bands with other creatures named Wolves of the Hunt.\"",
        )
        .expect("Wolves of the Hunt token definition should parse");
    let debug = format!("{definition:?}");

    assert!(debug.contains("BandsWithOther"), "{debug}");
    assert!(debug.contains("Wolves of the Hunt"), "{debug}");
    assert!(!debug.contains("KeywordFallbackText"), "{debug}");
}

#[test]
pub(super) fn rewrite_anthem_grant_static_parses_miracle_reduction_tail_without_word_view() {
    let tokens = lex_line(
        "Each enchantment card in your hand has miracle. Its miracle cost is equal to its mana cost reduced by {4}.",
        0,
    )
    .expect("rewrite lexer should classify granted miracle static line");

    let parsed = super::super::keyword_static::parse_granted_keyword_static_line(&tokens)
        .expect("granted miracle static line should parse")
        .expect("granted miracle static line should be recognized");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("MiracleFromCardManaCostReducedBy") && debug.contains("reduction: 4"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_anthem_static_condition_normalizes_apostrophe_shapes() {
    let tokens = lex_line("It's enchanted", 0)
        .expect("rewrite lexer should classify static-condition clause");

    let parsed = super::super::keyword_static::parse_static_condition_clause(&tokens)
        .expect("static-condition clause should parse");

    assert!(matches!(parsed, crate::ConditionExpr::SourceIsEnchanted));
}

#[test]
pub(super) fn rewrite_anthem_static_status_condition_uses_subject_status_capture() {
    for (text, expected) in [
        (
            "this permanent is tapped",
            crate::ConditionExpr::SourceIsTapped,
        ),
        ("it is attacking", crate::ConditionExpr::SourceIsAttacking),
        (
            "equipped creature is untapped",
            crate::ConditionExpr::EquippedCreatureUntapped,
        ),
    ] {
        let tokens = lex_line(text, 0)
            .expect("rewrite lexer should classify subject-status static-condition clause");

        let parsed = super::super::keyword_static::parse_static_condition_clause(&tokens)
            .expect("subject-status static-condition clause should parse");

        assert_eq!(parsed, expected, "{text}");
    }
}

#[test]
pub(super) fn rewrite_anthem_static_condition_preserves_attacking_alone_semantics() {
    let tokens = lex_line("it's attacking alone", 0)
        .expect("rewrite lexer should classify attacking-alone condition");
    let parsed = super::super::keyword_static::parse_static_condition_clause(&tokens)
        .expect("attacking-alone condition should parse");
    let mut attacking_creatures = ObjectFilter::creature();
    attacking_creatures.attacking = true;

    assert_eq!(
        parsed,
        crate::ConditionExpr::And(
            Box::new(crate::ConditionExpr::SourceIsAttacking),
            Box::new(crate::ConditionExpr::CountComparison {
                count: crate::static_abilities::AnthemCountExpression::MatchingFilter(
                    attacking_creatures,
                ),
                comparison: crate::effect::Comparison::Equal(1),
                display: Some("no other creatures are attacking".to_string()),
            }),
        )
    );
}

#[test]
pub(super) fn rewrite_unblockable_line_keeps_full_attacking_alone_condition() {
    let tokens = lex_line(
        "This creature can't be blocked as long as it's attacking alone.",
        0,
    )
    .expect("conditional unblockable line should lex");
    let parsed =
        super::super::keyword_static::parse_subject_cant_be_blocked_as_long_as_condition_line(
            &tokens,
        )
        .expect("conditional unblockable line should parse")
        .expect("conditional unblockable line should be recognized");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Unblockable"), "{debug}");
    assert!(debug.contains("SourceIsAttacking"), "{debug}");
    assert!(debug.contains("CountComparison"), "{debug}");
    assert!(debug.contains("attacking: true"), "{debug}");
    assert!(debug.contains("Equal(1)"), "{debug}");
}

#[test]
pub(super) fn rewrite_anthem_static_descriptor_condition_uses_subject_descriptor_capture() {
    let vehicle_tokens = lex_line("enchanted permanent is a vehicle", 0)
        .expect("rewrite lexer should classify enchanted-permanent descriptor condition");
    let vehicle = super::super::keyword_static::parse_static_condition_clause(&vehicle_tokens)
        .expect("enchanted-permanent descriptor condition should parse");
    assert_eq!(vehicle, crate::ConditionExpr::EnchantedPermanentIsVehicle);

    let color_tokens = lex_line("enchanted creature is red", 0)
        .expect("rewrite lexer should classify attached-object descriptor condition");
    let color = super::super::keyword_static::parse_static_condition_clause(&color_tokens)
        .expect("attached-object descriptor condition should parse");
    assert!(matches!(
        color,
        crate::ConditionExpr::AttachedToSourceMatches(filter)
            if filter.colors == Some(crate::color::ColorSet::RED)
    ));
}

#[test]
pub(super) fn rewrite_anthem_static_player_status_condition_uses_player_status_capture() {
    for (text, expected) in [
        (
            "You're the monarch",
            crate::ConditionExpr::PlayerIsMonarch {
                player: crate::target::PlayerFilter::You,
            },
        ),
        (
            "you have the initiative",
            crate::ConditionExpr::PlayerHasInitiative {
                player: crate::target::PlayerFilter::You,
            },
        ),
        (
            "you have maximum speed",
            crate::ConditionExpr::ValueComparison {
                left: crate::effect::Value::Speed(crate::target::PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(4),
            },
        ),
    ] {
        let tokens = lex_line(text, 0)
            .expect("rewrite lexer should classify player-status static-condition clause");

        let parsed = super::super::keyword_static::parse_static_condition_clause(&tokens)
            .expect("player-status static-condition clause should parse");

        assert_eq!(parsed, expected, "{text}");
    }
}

#[test]
pub(super) fn rewrite_anthem_static_player_achievement_condition_uses_achievement_capture() {
    for text in ["you have the city's blessing", "you've completed a dungeon"] {
        let tokens = lex_line(text, 0)
            .expect("rewrite lexer should classify player-achievement static-condition clause");

        let parsed = super::super::keyword_static::parse_static_condition_clause(&tokens)
            .expect("player-achievement static-condition clause should parse");
        let debug = format!("{parsed:?}");

        assert!(
            debug.contains("PlayerHasCitysBlessing") || debug.contains("PlayerCompletedDungeon"),
            "{text}: {debug}"
        );
    }

    let tokens = lex_line("you have completed Lost Mine of Phandelver", 0)
        .expect("rewrite lexer should classify named-dungeon static-condition clause");
    let parsed = super::super::keyword_static::parse_static_condition_clause(&tokens)
        .expect("named-dungeon static-condition clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("PlayerCompletedDungeon"), "{debug}");
    assert!(debug.contains("Lost Mine of Phandelver"), "{debug}");
}

#[test]
pub(super) fn rewrite_anthem_static_ownership_condition_uses_owner_capture() {
    let tokens = lex_line("As long as you own two or more artifacts", 0)
        .expect("rewrite lexer should classify ownership static-condition clause");

    let parsed = super::super::keyword_static::parse_static_condition_clause(&tokens)
        .expect("ownership static-condition clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("CountComparison"), "{debug}");
    assert!(debug.contains("GreaterThanOrEqual(2)"), "{debug}");
    assert!(debug.contains("owner: Some(You)"), "{debug}");
    assert!(debug.contains("Artifact"), "{debug}");
}

#[test]
pub(super) fn rewrite_static_condition_parses_multicolor_devotion_comparison() {
    let tokens = lex_line("Your devotion to blue and red is less than seven", 0)
        .expect("rewrite lexer should classify devotion condition clause");

    let parsed = super::super::keyword_static::parse_static_condition_clause(&tokens)
        .expect("devotion condition clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ValueComparison"), "{debug}");
    assert!(debug.contains("LessThan"), "{debug}");
    assert!(
        debug.contains("Devotion { player: You, color: Blue }"),
        "{debug}"
    );
    assert!(
        debug.contains("Devotion { player: You, color: Red }"),
        "{debug}"
    );
    assert!(debug.contains("Fixed(7)"), "{debug}");
}

#[test]
pub(super) fn rewrite_anthem_subject_parses_enchanted_player_controls() {
    let tokens = lex_line("Creatures enchanted player controls", 0)
        .expect("rewrite lexer should classify enchanted-player-controls subject");

    let parsed = super::super::keyword_static::parse_anthem_subject(&tokens)
        .expect("enchanted-player-controls subject should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("card_types: [Creature]"), "{debug}");
    assert!(
        debug.contains("controller: Some(TaggedPlayer(TagKey(\"enchanted\")))"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_anthem_subject_preserves_typed_commander_and_attacking_token_filters() {
    for (subject_text, is_commander, token, attacking) in [
        ("Commanders you control", true, false, false),
        ("Attacking tokens you control", false, true, true),
    ] {
        let tokens =
            lex_line(subject_text, 0).expect("rewrite lexer should classify typed anthem subject");
        let (parsed, loss) = crate::parse_loss::capture(|| {
            super::super::keyword_static::parse_anthem_subject(&tokens)
        });
        let parsed = parsed.expect("typed anthem subject should parse");
        let super::super::keyword_static::AnthemSubjectAst::Filter(filter) = parsed else {
            panic!("{subject_text}: expected typed object filter subject");
        };

        assert!(
            !loss.is_lossy(),
            "{}: {}",
            subject_text,
            loss.reasons_text()
        );
        assert_eq!(filter.zone, Some(Zone::Battlefield), "{subject_text}");
        assert_eq!(
            filter.controller,
            Some(crate::PlayerFilter::You),
            "{subject_text}"
        );
        assert_eq!(filter.is_commander, is_commander, "{subject_text}");
        assert_eq!(filter.token, token, "{subject_text}");
        assert_eq!(filter.attacking, attacking, "{subject_text}");
    }
}

#[test]
pub(super) fn rewrite_anthem_subject_preserves_outer_aura_and_typed_attachment_host() {
    let tokens = lex_line("Auras attached to permanents you control", 0)
        .expect("rewrite lexer should classify attached Aura anthem subject");

    let parsed = super::super::keyword_static::parse_anthem_subject(&tokens)
        .expect("attached Aura anthem subject should parse");
    let super::super::keyword_static::AnthemSubjectAst::Filter(filter) = parsed else {
        panic!("expected typed object filter subject");
    };

    assert_eq!(filter.subtypes, vec![Subtype::Aura]);
    let host = filter
        .attached_to_object
        .as_deref()
        .expect("typed permanent attachment host");
    assert_eq!(host.zone, Some(Zone::Battlefield));
    assert_eq!(host.controller, Some(crate::target::PlayerFilter::You));
}

#[test]
pub(super) fn rewrite_anthem_subject_rejects_speculative_non_subject_fragments_without_loss() {
    for fragment in [
        "all abilities and",
        "you draw two cards lose 2 life and",
        "as long as enchanted permanent is an equipment it",
    ] {
        let tokens =
            lex_line(fragment, 0).expect("rewrite lexer should classify speculative fragment");
        let (parsed, loss) = crate::parse_loss::capture(|| {
            super::super::keyword_static::parse_anthem_subject(&tokens)
        });

        assert!(parsed.is_err(), "{fragment}: {parsed:#?}");
        assert!(!loss.is_lossy(), "{}: {}", fragment, loss.reasons_text());
    }
}

#[test]
pub(super) fn rewrite_representative_suffix_recovery_cards_compile_without_parse_loss() {
    for (name, text) in [
        (
            "Typed Commander Grant",
            "During your turn, commanders you control have indestructible.",
        ),
        (
            "Typed Attacking Token Grant",
            "Attacking tokens you control have deathtouch.",
        ),
        (
            "Typed Lose Abilities Transform",
            "Enchanted creature loses all abilities and is a blue Frog creature with base power and toughness 1/1.",
        ),
        (
            "Typed Effect Sequence",
            "You draw two cards, lose 2 life, and get {E}{E}.",
        ),
        (
            "Typed Conditional Grant",
            "As long as enchanted permanent is an Equipment, it has \"Equipped creature gets +1/+1 and has trample.\"",
        ),
    ] {
        let (compiled, loss) = crate::parse_loss::capture(|| {
            super::super::compile_card_text(
                CardDefinitionBuilder::new(CardId::from_raw(1), name),
                text,
                false,
            )
        });

        let compiled = compiled.unwrap_or_else(|err| panic!("{name}: {err:?}"));
        assert!(!loss.is_lossy(), "{}: {}", name, loss.reasons_text());
        if name == "Typed Conditional Grant" {
            let debug = format!("{compiled:#?}");
            assert!(!debug.contains("__it__"), "{debug}");
            assert!(
                debug.contains("\"enchanted\"") && debug.contains("\"equipped\""),
                "{debug}"
            );
        }
    }
}

#[test]
pub(super) fn rewrite_player_counter_conditional_anthem_compiles_without_parse_loss() {
    let text = "As long as an opponent has three or more poison counters, enchanted creature gets an additional +1/+0 and has first strike.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(
                CardId::from_raw(1),
                "Typed Player Counter Conditional Anthem",
            )
            .card_types(vec![CardType::Enchantment]),
            text,
            false,
        )
    });

    compiled.expect("player-counter conditional anthem should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
}

#[test]
pub(super) fn rewrite_conditional_vehicle_type_identity_lowers_as_static_with_condition() {
    for (name, text, condition_fragments, expects_source_surface) in [
        (
            "Grond, the Gatebreaker",
            "As long as it's your turn and you control an Army, Grond is an artifact creature.",
            &["YourTurn", "Army"][..],
            false,
        ),
        (
            "Phoenix Fleet Airship",
            "As long as you control eight or more permanents named Phoenix Fleet Airship, this Vehicle is an artifact creature.",
            &["Phoenix Fleet Airship", "comparison: GreaterThanOrEqual"][..],
            true,
        ),
    ] {
        let (compiled, loss) = crate::parse_loss::capture(|| {
            super::super::compile_card_text(
                CardDefinitionBuilder::new(CardId::from_raw(1), name)
                    .card_types(vec![CardType::Artifact])
                    .subtypes(vec![Subtype::Vehicle]),
                text,
                false,
            )
        });
        let compiled = compiled.unwrap_or_else(|err| panic!("{name}: {err:?}"));
        assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
        assert!(
            compiled.definition.spell_effect.is_none(),
            "a conditional static identity must not become a spell-resolution effect: {name}: {:#?}",
            compiled.definition.spell_effect
        );

        let (filter, card_types, condition) = compiled
            .definition
            .abilities
            .iter()
            .find_map(|ability| {
                let AbilityKind::Static(static_ability) = &ability.kind else {
                    return None;
                };
                let StaticAbilityPayload::Conditional { ability, condition } =
                    &static_ability.payload
                else {
                    return None;
                };
                let StaticAbilityPayload::SetCardTypes { filter, card_types } = &ability.payload
                else {
                    return None;
                };
                Some((filter, card_types, condition))
            })
            .unwrap_or_else(|| panic!("{name}: expected conditioned SetCardTypes ability"));

        assert!(filter.source, "{name}: {filter:#?}");
        assert_eq!(
            filter.source_surface.is_some(),
            expects_source_surface,
            "{name}: {filter:#?}"
        );
        assert_eq!(
            card_types.as_slice(),
            &[CardType::Artifact, CardType::Creature],
            "{name}"
        );
        let condition_debug = format!("{condition:#?}");
        let condition_debug_lower = condition_debug.to_ascii_lowercase();
        for fragment in condition_fragments {
            assert!(
                condition_debug_lower.contains(&fragment.to_ascii_lowercase()),
                "{name}: missing condition fragment '{fragment}': {condition_debug}"
            );
        }
    }
}

#[test]
pub(super) fn rewrite_tagged_plural_pump_after_untap_compiles_without_parse_loss() {
    for (name, text) in [
        (
            "Typed Fancy Footwork",
            "Untap one or two target creatures. They each get +2/+2 until end of turn.",
        ),
        (
            "Typed Join Forces",
            "Untap up to two target creatures. They each get +2/+2 until end of turn.",
        ),
    ] {
        let (compiled, loss) = crate::parse_loss::capture(|| {
            super::super::compile_card_text(
                CardDefinitionBuilder::new(CardId::from_raw(1), name),
                text,
                false,
            )
        });

        compiled.unwrap_or_else(|err| panic!("{name}: {err:?}"));
        assert!(!loss.is_lossy(), "{}: {}", name, loss.reasons_text());
    }
}

#[test]
pub(super) fn rewrite_unpreventable_damage_followup_marks_previous_damage() {
    let text =
        "This deals 4 damage to target player or planeswalker. The damage can't be prevented.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Unpreventable Damage"),
            text,
            false,
        )
    });
    let compiled = compiled.expect("unpreventable damage rider should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let damage = compiled
        .definition
        .spell_effect
        .as_ref()
        .expect("damage statement should lower as a spell program")
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::DealDamageEffect>())
        .expect("damage statement should retain its typed damage effect");
    assert!(
        damage.unpreventable,
        "damage rider should set the typed unpreventable field: {damage:#?}"
    );
}

#[test]
pub(super) fn self_replacement_damage_keeps_its_unpreventable_rider() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Replacement Damage Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "This spell deals 3 damage to any target. If this spell was kicked, instead it deals 10 damage to that permanent or player and the damage can't be prevented.",
        )
        .expect("replacement damage rider should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(debug.contains("SelfReplacementBranch"), "{debug}");
    assert!(debug.contains("unpreventable: true"), "{debug}");
}

#[test]
pub(super) fn rewrite_conditional_self_damage_prevention_preserves_counter_amount() {
    for (name, text, dynamic_amount) in [
        (
            "Typed Oathsworn Counter Prevention",
            "If damage would be dealt to this creature while it has a +1/+1 counter on it, prevent that damage and remove a +1/+1 counter from it.",
            false,
        ),
        (
            "Typed Conjurant Counter Prevention",
            "If damage would be dealt to this creature while it has a +1/+1 counter on it, prevent that damage and remove that many +1/+1 counters from it.",
            true,
        ),
    ] {
        let (compiled, loss) = crate::parse_loss::capture(|| {
            super::super::compile_card_text(
                CardDefinitionBuilder::new(CardId::from_raw(1), name)
                    .card_types(vec![CardType::Creature]),
                text,
                false,
            )
        });
        let compiled = compiled.unwrap_or_else(|err| panic!("{name}: {err:?}"));
        assert!(!loss.is_lossy(), "{}: {}", name, loss.reasons_text());
        let amount = compiled
            .definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => match &static_ability.payload {
                    StaticAbilityPayload::Conditional { ability, .. } => match &ability.payload {
                        StaticAbilityPayload::PreventDamageToSelfRemoveCounter {
                            counter_type: CounterType::PlusOnePlusOne,
                            amount,
                            ..
                        } => Some(amount),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            })
            .unwrap_or_else(|| panic!("{name}: expected conditional counter prevention"));
        if dynamic_amount {
            assert_eq!(
                amount,
                &Value::EventValue(crate::effect::EventValueSpec::Amount)
            );
        } else {
            assert_eq!(amount, &Value::Fixed(1));
        }
    }
}

#[test]
pub(super) fn rewrite_counter_prevention_keeps_each_player_counter_followup() {
    let text = "If damage would be dealt to this creature while it has a +1/+1 counter on it, prevent that damage, remove that many +1/+1 counters from it, then give each player a rad counter for each +1/+1 counter removed this way.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Counter Prevention Followup")
                .card_types(vec![CardType::Creature]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("counter-derived player-counter follow-up should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let follow_up = compiled
        .definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => match &static_ability.payload {
                StaticAbilityPayload::Conditional { ability, .. } => match &ability.payload {
                    StaticAbilityPayload::PreventDamageToSelfRemoveCounter {
                        follow_up, ..
                    } => *follow_up,
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })
        .expect("expected typed counter-removal follow-up");
    assert_eq!(
        follow_up,
        ironsmith_core::CounterRemovalFollowUp::EachPlayerGetsCounters {
            counter_type: CounterType::Rad,
            counters_per_removed: 1,
        }
    );
}

#[test]
pub(super) fn rewrite_possessive_self_counters_move_from_source_lki() {
    let text = "{1}, Sacrifice this creature: Target creature you control gains indestructible until end of turn. Put this's counters on that creature and attach an Equipment that was attached to this creature to that creature.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Counter Inheritance")
                .card_types(vec![CardType::Creature]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("possessive source counter transfer should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let debug = format!("{:#?}", compiled.definition.abilities);
    assert!(debug.contains("MoveAllCountersEffect"), "{debug}");
    assert!(debug.contains("from: Source"), "{debug}");
}

#[test]
pub(super) fn rewrite_conditional_self_damage_prevention_can_precede_triggered_followup() {
    let text = "If damage would be dealt to this creature while it has a +1/+1 counter on it, prevent that damage and remove that many +1/+1 counters from it. When one or more counters are removed from this creature this way, it deals that much damage to any target.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Prevention Followup")
                .card_types(vec![CardType::Creature]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("static prevention and triggered followup should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    assert!(
        compiled
            .definition
            .abilities
            .iter()
            .any(|ability| matches!(&ability.kind, AbilityKind::Static(_))),
        "expected the prevention sentence to remain a static ability"
    );
    assert!(
        compiled
            .definition
            .abilities
            .iter()
            .any(|ability| matches!(&ability.kind, AbilityKind::Triggered(_))),
        "expected the counter-removal sentence to remain a triggered ability"
    );

    let triggered = compiled
        .definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected typed counter-removal trigger");
    let ironsmith_core::TriggerKind::CounterRemovedFrom(counter_removed) = &triggered.trigger.kind
    else {
        panic!(
            "expected CounterRemovedFrom trigger, got {:#?}",
            triggered.trigger
        );
    };
    assert!(counter_removed.filter.source);
    assert!(counter_removed.one_or_more);
    assert!(counter_removed.caused_by_source);
    assert_eq!(
        triggered.trigger.intro_surface,
        Some(ironsmith_core::trigger_model::TriggerIntroSurface::When)
    );

    let damage = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::DealDamageEffect>(effect))
        .expect("expected a real damage effect rather than a target-only placeholder");
    assert!(matches!(
        damage.amount.unhinted(),
        Value::EventValue(crate::effect::EventValueSpec::Amount)
    ));
    assert!(
        damage
            .amount
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay)
    );
    assert!(matches!(
        damage.target.base(),
        crate::target::ChooseSpec::AnyTarget
    ));
}

#[test]
pub(super) fn triggered_conditional_preserves_leading_duration_for_compound_effects() {
    let text = "Whenever this creature attacks, if you control three or more Dragons, until end of turn, this creature becomes a Dragon with base power and toughness 5/5 and gains flying.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Conditional Duration")
                .card_types(vec![CardType::Creature]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("conditional compound duration should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let debug = format!("{:#?}", compiled.definition.abilities);
    assert!(
        debug.matches("until: EndOfTurn").count() >= 2,
        "both compound effects should retain the leading duration: {debug}"
    );
}

#[test]
pub(super) fn rewrite_scaled_dynamic_target_count_reaches_typed_target_ast() {
    let tokens = lex_line("up to twice X target cards from graveyards", 0).unwrap();
    let target = super::super::util::parse_target_phrase(&tokens).expect("dynamic target phrase");
    assert!(matches!(
        target,
        crate::cards::builders::TargetAst::WithCountValue(
            _,
            count,
            Value::XTimes(2)
        ) if count.is_up_to_dynamic_x()
    ));

    let text = "Choose one —\n• Target creature gets -X/-X until end of turn. You gain X life.\n• Exile up to twice X target cards from graveyards.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Erebos Intervention")
                .card_types(vec![CardType::Instant]),
            text,
            false,
        )
    });
    compiled.expect("scaled dynamic target modal spell should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
}

#[test]
pub(super) fn rewrite_hyphenated_artifact_creature_token_compiles_without_loss() {
    let text = "{7}, {T}: Create a 2/2 colorless Assembly-Worker artifact creature token.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Assembly Worker Factory")
                .card_types(vec![CardType::Land]),
            text,
            false,
        )
    });
    compiled.expect("hyphenated artifact creature token should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
}

#[test]
pub(super) fn rewrite_kicked_counter_entry_preserves_quoted_defender_permission() {
    let text = "Kicker {1}{W}\nIf this creature was kicked, it enters with a +1/+1 counter on it and with \"This creature can attack as though it didn't have defender.\"";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Prison Barricade")
                .card_types(vec![CardType::Creature]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("kicked counter entry with ability should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let debug = format!("{:#?}", compiled.definition.abilities);
    assert!(debug.contains("ThisSpellWasKicked"), "{debug}");
    assert!(debug.contains("CanAttackAsThoughNoDefender"), "{debug}");
}

#[test]
pub(super) fn rewrite_target_mana_value_where_x_pump_compiles_without_loss() {
    let text = "Target creature gains trample and gets +X/+0 until end of turn, where X is that creature's mana value.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Surge Strength")
                .card_types(vec![CardType::Instant]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("target mana-value where-X pump should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let debug = format!("{:#?}", compiled.definition);
    assert!(debug.contains("ManaValueOf"), "{debug}");
}

#[test]
pub(super) fn rewrite_all_graveyard_card_types_where_x_keeps_typed_value() {
    let text = "Whenever a creature you control attacks alone, it gets +X/+X until end of turn, where X is the number of card types among cards in all graveyards.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Card Types Among")
                .card_types(vec![CardType::Artifact]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("card-types-among where-X trigger should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let debug = format!("{:#?}", compiled.definition);
    assert!(debug.contains("CardTypesAmong"), "{debug}");
    assert!(debug.contains("zone: Some(\n"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
}

#[test]
pub(super) fn rewrite_counter_entry_counts_loyalty_counters_across_controlled_planeswalkers() {
    let text = "This creature enters with a +1/+1 counter on it for each loyalty counter on planeswalkers you control.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Loyalty Counter Entry")
                .card_types(vec![CardType::Creature]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("loyalty-counter entry value should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let debug = format!("{:#?}", compiled.definition);
    assert!(debug.contains("CountersOn"), "{debug}");
    assert!(debug.contains("ForEach"), "{debug}");
    assert!(debug.contains("Loyalty"), "{debug}");
    assert!(debug.contains("Planeswalker"), "{debug}");
    assert!(debug.contains("controller: Some(\n"), "{debug}");
}

#[test]
pub(super) fn rewrite_created_token_tap_mana_ability_keeps_spending_restriction() {
    let text = "Create a 1/1 red Wizard creature token with \"{T}: Add {R}. Spend this mana only to cast a planeswalker spell.\"";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Restricted Mana Token")
                .card_types(vec![CardType::Sorcery]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("restricted token mana ability should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let debug = format!("{:#?}", compiled.definition);
    assert!(debug.contains("mana_output: Some"), "{debug}");
    assert!(debug.contains("Red"), "{debug}");
    assert!(debug.contains("mana_usage_restrictions"), "{debug}");
    assert!(debug.contains("Planeswalker"), "{debug}");
}

#[test]
pub(super) fn rewrite_created_token_followup_keeps_tap_mana_ability() {
    let text = "{G}, {T}, Discard a card: Create a 1/1 green Elf Druid creature token named Llanowar Elves. It has \"{T}: Add {G}.\"";
    let effect_text = "Create a 1/1 green Elf Druid creature token named Llanowar Elves. It has \"{T}: Add {G}.\"";
    let effect_tokens = lex_line(effect_text, 0).expect("token followup should lex");
    let sentences = split_lexed_sentences(&effect_tokens);
    assert_eq!(sentences.len(), 2, "expected creation plus reminder");
    assert!(
        crate::runtime_backend::grammar::token_definitions::parse_token_tap_mana_ability_tokens(
            sentences[1]
        )
        .is_some(),
        "typed reminder grammar should preserve the quoted tap-mana ability"
    );
    let effect_ast = super::super::clause_support::parse_effect_sentences_lexed(&effect_tokens)
        .expect("token followup should parse");
    let token_definition = effect_ast.iter().find_map(|effect| match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::CreateTokenWithMods { definition, .. } => Some(definition),
            _ => None,
        },
        _ => None,
    });
    let Some(crate::runtime_backend::token_definition::TokenDefinitionSpec::Creature(creature)) =
        token_definition
    else {
        panic!("expected creature token definition, got {effect_ast:#?}");
    };
    assert!(
        creature.rules.tap_mana_ability.is_some(),
        "expected typed tap-mana rule, got {creature:#?}"
    );

    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Typed Token Ability Followup")
                .card_types(vec![CardType::Creature]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("token ability followup should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let debug = format!("{:#?}", compiled.definition);
    assert!(debug.contains("mana_output: Some"), "{debug}");
    assert!(debug.contains("Green"), "{debug}");
}

#[test]
pub(super) fn rewrite_verb_handlers_parse_look_normalizes_target_player_apostrophe_shapes() {
    let tokens = lex_line("Look at target player's hand.", 0)
        .expect("rewrite lexer should classify target player's hand look clause");

    let parsed =
        super::super::parse_effect_clause_lexed(&tokens).expect("look-at-hand clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("LookAtHand"), "{debug}");
    assert!(debug.contains("Player(Target(Any)"), "{debug}");
}

#[test]
pub(super) fn rewrite_verb_handlers_parse_look_normalizes_owner_apostrophe_shapes() {
    let tokens = lex_line("Look at the top card of its owner's library.", 0)
        .expect("rewrite lexer should classify owner-library look clause");

    let parsed = super::super::parse_effect_clause_lexed(&tokens)
        .expect("owner-library look clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("player: ItsOwner"), "{debug}");
    assert!(debug.contains("count: Fixed(1)"), "{debug}");
}

#[test]
pub(super) fn rewrite_subject_verb_primitives_delayed_next_upkeep_unless_pays_normalizes_player_apostrophe_shapes()
 {
    let tokens = lex_line(
        "Exile that creature at the beginning of that player's next upkeep unless they pay {2}.",
        0,
    )
    .expect("rewrite lexer should classify delayed next-upkeep unless sentence");

    let parsed = super::super::parse_sentence_delayed_next_step_unless_pays(
        super::super::effect_sentences::SubjectVerbPrimitiveClause::new(&tokens),
    )
    .expect("delayed next-upkeep unless sentence should parse")
    .expect("delayed next-upkeep unless sentence should be recognized");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("DelayedUntilNextUpkeep"), "{debug}");
    assert!(debug.contains("player: That"), "{debug}");
    assert!(debug.contains("UnlessPays"), "{debug}");
}

#[test]
pub(super) fn rewrite_token_copy_followup_recognizes_next_upkeep_sacrifice() {
    let tokens = lex_line(
        "Sacrifice that token at the beginning of the next upkeep.",
        0,
    )
    .expect("rewrite lexer should classify token-copy next-upkeep cleanup");

    let followup =
        crate::runtime_backend::effect_sentences::parse_token_copy_followup_sentence_lexed(&tokens)
            .expect("token-copy next-upkeep sacrifice should be recognized");

    assert_eq!(
        followup,
        super::super::effect_sentences::TokenCopyFollowup::SacrificeAtNextUpkeep
    );
}

#[test]
pub(super) fn rewrite_subject_verb_primitives_unless_clause_normalizes_controller_apostrophe_shapes()
 {
    let tokens = lex_line("Draw a card unless that spell's controller pays {2}.", 0)
        .expect("rewrite lexer should classify unless-controller sentence");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("unless-controller sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("UnlessPays"), "{debug}");
    assert!(debug.contains("player: ItsController"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
}

#[test]
pub(super) fn rewrite_destroy_unless_dynamic_life_cost_tracks_target_toughness() {
    let tokens = lex_line(
        "Destroy target creature unless its controller pays life equal to its toughness.",
        0,
    )
    .expect("rewrite lexer should classify dynamic destroy-unless payment");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("dynamic destroy-unless clause should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("UnlessPays"), "{debug}");
    assert!(debug.contains("player: ItsController"), "{debug}");
    assert!(debug.contains("ToughnessOf"), "{debug}");
    assert!(debug.contains("Target(Object"), "{debug}");
}

#[test]
pub(super) fn rewrite_zone_handlers_sacrifice_choice_suffix_normalizes_pronoun_phrase() {
    let tokens = lex_line(
        "Target opponent sacrifices a creature of his or her choice.",
        0,
    )
    .expect("rewrite lexer should classify sacrifice-choice sentence");

    let parsed =
        parse_effect_sentence_lexed(&tokens).expect("sacrifice-choice sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Sacrifice"), "{debug}");
    assert!(debug.contains("player: TargetOpponent"), "{debug}");
    assert!(debug.contains("card_types: [Creature]"), "{debug}");
}

#[test]
pub(super) fn rewrite_keyword_static_routes_spell_cost_modifier_filters_through_grammar_entrypoint()
{
    let tokens = lex_line("Artifact spells you cast cost {2} less to cast.", 0)
        .expect("rewrite lexer should classify spell cost modifier line");

    let parsed = super::super::keyword_static::parse_spells_cost_modifier_line(&tokens)
        .expect("spell cost modifier line should parse")
        .expect("spell cost modifier should be recognized");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("CostReduction") || debug.contains("cost") || debug.contains("less to cast"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_keyword_static_routes_trigger_duplication_source_filters_through_grammar_entrypoint()
 {
    let tokens = lex_line(
        "if a triggered ability of artifact creatures you control triggers, it triggers an additional time.",
        0,
    )
    .expect("rewrite lexer should classify trigger-duplication static line");

    let parsed = super::super::keyword_static::parse_trigger_duplication_line_ast(&tokens)
        .expect("trigger-duplication static line should parse")
        .expect("trigger-duplication static line should be recognized");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("duplicate") || debug.contains("Duplicate") || debug.contains("trigger"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_keyword_static_routes_trigger_duplication_event_filters_through_grammar_entrypoint()
 {
    let tokens = lex_line(
        "if turning artifact creatures you control face up causes an ability of a permanent you control to trigger, that ability triggers an additional time.",
        0,
    )
    .expect("rewrite lexer should classify trigger-duplication event line");

    let parsed = super::super::keyword_static::parse_trigger_duplication_line_ast(&tokens)
        .expect("trigger-duplication event line should parse")
        .expect("trigger-duplication event line should be recognized");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("duplicate") || debug.contains("Duplicate") || debug.contains("trigger"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_grammar_trigger_duplication_as_long_as_prefix_splitter_matches_static_shape()
{
    let tokens = lex_line(
        "As long as you control an artifact, if a triggered ability of artifact creatures you control triggers, it triggers an additional time.",
        0,
    )
    .expect("rewrite lexer should classify conditional trigger-duplication static line");

    let spec = super::super::grammar::abilities::split_as_long_as_condition_prefix_lexed(&tokens)
        .expect("grammar-owned as-long-as prefix splitter should match");
    assert_eq!(
        crate::runtime_backend::token_word_refs(spec.condition_tokens),
        vec!["you", "control", "an", "artifact"],
    );
    assert_eq!(
        crate::runtime_backend::token_word_refs(spec.remainder_tokens),
        vec![
            "if",
            "a",
            "triggered",
            "ability",
            "of",
            "artifact",
            "creatures",
            "you",
            "control",
            "triggers",
            "it",
            "triggers",
            "an",
            "additional",
            "time",
        ],
    );

    let parsed = super::super::keyword_static::parse_trigger_duplication_line_ast(&tokens)
        .expect("conditional trigger-duplication static line should parse")
        .expect("conditional trigger-duplication static line should be recognized");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConditionalStaticAbility"), "{debug}");
    assert!(debug.contains("you control an artifact"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_next_spell_cascade_grants_parse_natively() {
    let single_tokens = lex_line(
        "The next noncreature spell you cast this turn has cascade.",
        0,
    )
    .expect("rewrite lexer should classify single next-spell cascade grant");
    let dual_tokens = lex_line(
        "The next instant spell and the next sorcery spell you cast this turn each have cascade.",
        0,
    )
    .expect("rewrite lexer should classify dual next-spell cascade grant");

    let single_effects =
        super::super::effect_sentences::parse_effect_sentence_lexed(&single_tokens)
            .expect("single next-spell cascade grant should parse");
    let dual_effects = super::super::effect_sentences::parse_effect_sentence_lexed(&dual_tokens)
        .expect("dual next-spell cascade grant should parse");

    assert!(matches!(
        single_effects.as_slice(),
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action:
                    crate::cards::builders::SubjectVerbActionAst::GrantNextSpellAbilityThisTurn { .. },
                ..
            },
        )]
    ));
    let dual_grants = match dual_effects.as_slice() {
        [crate::cards::builders::EffectAst::Coordinated { effects, .. }] => effects,
        other => panic!("expected coordinated grants, got {other:#?}"),
    };
    assert_eq!(
        dual_grants.len(),
        2,
        "expected one grant per next-spell lane"
    );
    assert!(
        dual_grants.iter().all(|effect| matches!(
        effect,
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action:
                    crate::cards::builders::SubjectVerbActionAst::GrantNextSpellAbilityThisTurn {
                        ..
                    },
                ..
            },
        )
    ))
    );
}

#[test]
pub(super) fn rewrite_lexed_next_spell_cant_be_countered_grant_parses_natively() {
    let tokens = lex_line(
        "The next instant or sorcery spell you cast this turn can't be countered.",
        0,
    )
    .expect("rewrite lexer should classify next-spell uncounterable grant");

    let effects = super::super::effect_sentences::parse_effect_sentence_lexed(&tokens)
        .expect("next-spell uncounterable grant should parse");

    assert!(matches!(
        effects.as_slice(),
        [crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action:
                    crate::cards::builders::SubjectVerbActionAst::GrantNextSpellAbilityThisTurn { .. },
                ..
            },
        )]
    ));
}

#[test]
pub(super) fn rewrite_lexed_cycling_parser_ignores_static_grant_clause_prefixes() {
    let tokens = lex_line(
        "Each Sliver card in each player's hand has slivercycling {3}.",
        0,
    )
    .expect("rewrite lexer should classify granted cycling clause");

    assert!(
        super::super::parse_cycling_line_lexed(&tokens)
            .expect("cycling parser should inspect granted clause")
            .is_none()
    );
}

#[test]
pub(super) fn filter_keyword_constraint_accepts_cycling_variant_words() {
    use super::super::front_end::shared::util::{
        FilterKeywordConstraint, parse_filter_keyword_constraint_words,
    };

    assert_eq!(
        parse_filter_keyword_constraint_words(&["cycling"]),
        Some((FilterKeywordConstraint::Marker("cycling"), 1))
    );
    assert_eq!(
        parse_filter_keyword_constraint_words(&["slivercycling"]),
        Some((FilterKeywordConstraint::Marker("cycling"), 1))
    );
    assert_eq!(
        parse_filter_keyword_constraint_words(&["basic", "landcycling"]),
        Some((FilterKeywordConstraint::Marker("cycling"), 2))
    );
}

#[test]
pub(super) fn rewrite_search_library_head_splitter_tracks_direct_may_and_rejects_early_may() {
    let direct_may = lex_line(
        "Target player may search their library for a card, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify direct-may search text");
    let split =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&direct_may)
            .expect("grammar-owned search head splitter should match direct may");

    assert_eq!(
        render_token_slice(split.subject_tokens),
        "Target player",
        "subject tokens should stop before direct may"
    );
    assert!(split.sentence_has_direct_may);
    assert_eq!(
        render_token_slice(split.search_tokens),
        "search their library for a card, then shuffle.",
        "search tokens should start at the search verb"
    );

    let leading_chain = lex_line(
        "Discard a card, then search your library for a creature card, reveal it, put it into your hand, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify leading-chain search text");
    let split =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&leading_chain)
            .expect("grammar-owned search head splitter should match plain search");
    assert!(!split.sentence_has_direct_may);
    assert_eq!(
        render_token_slice(split.subject_tokens),
        "Discard a card, then",
        "subject tokens should preserve the leading chain before search"
    );

    let early_may = lex_line(
        "You may draw a card, then search your library for a creature card, reveal it, put it into your hand, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify early-may search text");
    assert!(
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&early_may)
            .is_none(),
        "non-direct may before search should stay out of the search-family parser"
    );
}

#[test]
pub(super) fn rewrite_search_library_head_splitter_ignores_quoted_emblem_search_text() {
    let tokens = lex_line(
        r#"You get an emblem with "At the beginning of your end step, you may search your library for a creature card, put it onto the battlefield, then shuffle.""#,
        0,
    )
    .expect("rewrite lexer should classify quoted emblem search text");

    assert!(
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&tokens).is_none(),
        "search-family parsing should ignore search text inside emblem quotes"
    );

    let effects = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("quoted emblem search text should parse as an emblem effect");
    match effects.as_slice() {
        [crate::cards::builders::EffectAst::SubjectVerb(subject_verb)] => {
            match &subject_verb.action {
                crate::cards::builders::SubjectVerbActionAst::CreateEmblem { emblem } => assert!(
                    emblem.text.contains("may search your library"),
                    "emblem text should retain the quoted search clause, got {}",
                    emblem.text
                ),
                other => panic!("expected a CreateEmblem action, got {other:#?}"),
            }
        }
        other => panic!("expected a single CreateEmblem effect, got {other:#?}"),
    }
}

#[test]
pub(super) fn rewrite_trailing_if_splitter_ignores_quoted_emblem_conditionals() {
    let tokens = lex_line(
        r#"You get an emblem with "At the beginning of combat on your turn, put three +1/+1 counters on target artifact you control. If it's not a creature, it becomes a 0/0 Robot artifact creature.""#,
        0,
    )
    .expect("rewrite lexer should classify quoted emblem conditional text");

    assert!(
        super::super::grammar::structure::split_trailing_if_clause_lexed(&tokens).is_none(),
        "trailing-if splitter should ignore conditional text inside emblem quotes"
    );

    let effects = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("quoted emblem conditional text should parse as an emblem effect");
    match effects.as_slice() {
        [crate::cards::builders::EffectAst::SubjectVerb(subject_verb)] => {
            match &subject_verb.action {
                crate::cards::builders::SubjectVerbActionAst::CreateEmblem { emblem } => assert!(
                    emblem
                        .text
                        .to_ascii_lowercase()
                        .contains("if it's not a creature"),
                    "emblem text should retain the quoted conditional sentence, got {}",
                    emblem.text
                ),
                other => panic!("expected a CreateEmblem action, got {other:#?}"),
            }
        }
        other => panic!("expected a single CreateEmblem effect, got {other:#?}"),
    }
}

pub(super) fn compile_typed_emblem(text: &str) -> crate::effect::EmblemDescription {
    let definition = CardDefinitionBuilder::new(CardId::from_raw(1), "Emblem Test Spell")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("emblem spell should compile");
    definition
        .spell_effect
        .as_ref()
        .expect("sorcery should have a spell effect")
        .flattened_default_effects()
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::CreateEmblemEffect>()
                .map(|effect| effect.emblem.clone())
        })
        .expect("spell should create an emblem")
}

#[test]
pub(super) fn rewrite_emblem_payload_lowers_a_typed_triggered_ability() {
    let emblem =
        compile_typed_emblem(r#"You get an emblem with "Whenever you cast a spell, draw a card.""#);
    assert!(matches!(
        emblem.abilities.as_slice(),
        [crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(_),
            ..
        }]
    ));
}

#[test]
pub(super) fn rewrite_emblem_payload_lowers_a_typed_static_ability() {
    let emblem = compile_typed_emblem(r#"You get an emblem with "You have no maximum hand size.""#);
    assert!(matches!(
        emblem.abilities.as_slice(),
        [crate::ability::Ability {
            kind: crate::ability::AbilityKind::Static(_),
            ..
        }]
    ));
}

#[test]
pub(super) fn rewrite_emblem_payload_lowers_a_typed_activated_ability() {
    let emblem = compile_typed_emblem(r#"You get an emblem with "{T}: Draw a card.""#);
    assert!(matches!(
        emblem.abilities.as_slice(),
        [crate::ability::Ability {
            kind: crate::ability::AbilityKind::Activated(_),
            ..
        }]
    ));
}

#[test]
pub(super) fn rewrite_emblem_payload_lowers_multiple_quoted_abilities() {
    let emblem = compile_typed_emblem(
        r#"You get an emblem with "You have no maximum hand size." and "{T}: Draw a card.""#,
    );
    assert_eq!(emblem.abilities.len(), 2, "{emblem:#?}");
    assert!(matches!(
        &emblem.abilities[0].kind,
        crate::ability::AbilityKind::Static(_)
    ));
    assert!(matches!(
        &emblem.abilities[1].kind,
        crate::ability::AbilityKind::Activated(_)
    ));
}

#[test]
pub(super) fn rewrite_intuition_search_stays_card_based_in_compiled_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Intuition Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Search your library for three cards and reveal them. Target opponent chooses one. Put that card into your hand and the rest into your graveyard. Then shuffle.",
        )
        .expect("intuition-style divvy spell should parse");

    let rendered = format!("{def:#?}");
    assert!(
        rendered.contains("ChooseObjectsEffect")
            && rendered.contains("Library")
            && rendered.contains("MoveToZoneEffect")
            && rendered.contains("Graveyard"),
        "expected Intuition to lower to card-based search/divvy effects, got {rendered}"
    );
}

#[test]
pub(super) fn dynamic_top_library_count_lowers_to_prior_effect_metric() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dynamic Reveal Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Sacrifice any number of lands. Reveal the top X cards of your library, where X is the number of lands sacrificed this way.",
        )
        .expect("dynamic reveal count should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("WithIdEffect")
            && debug.contains("EffectMetric")
            && debug.contains("AffectedObjects")
            && debug.contains("LookAtTopCardsEffect")
            && debug.contains("reveal: true"),
        "expected reveal count to bind to prior sacrifice metric, got {debug}"
    );
}

#[test]
pub(super) fn dynamic_draw_count_lowers_destroyed_this_way_to_prior_effect_metric() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Destroyed Draw Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy all creatures. Draw a card for each creature destroyed this way.")
        .expect("destroyed-this-way draw count should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("WithIdEffect")
            && debug.contains("PriorEffectMetric")
            && debug.contains("Count")
            && debug.contains("destroyed_0")
            && debug.contains("DrawCardsEffect"),
        "expected draw count to bind to prior destroy metric, got {debug}"
    );
}

#[test]
pub(super) fn draw_equal_to_removed_counters_keeps_typed_prior_effect_metric() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Removed Counter Draw Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, you may remove up to three stun counters from among all permanents. Draw cards equal to the number of stun counters removed this way.",
        )
        .expect("removed-counter draw count should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("RemoveUpToCountersEffect")
            && debug.contains("DrawCardsEffect")
            && debug.contains("PriorEffectMetric")
            && debug.contains("action: Some(\n")
            && debug.contains("Removed")
            && debug.contains("counter_type: Some(\n")
            && debug.contains("Stun"),
        "expected the draw count to retain the exact removed-counter result, got {debug}"
    );
    assert!(!debug.contains("PendingPriorEffectMetric"), "{debug}");
}

#[test]
pub(super) fn counted_consult_stop_keeps_sacrificed_subtype_provenance() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Counted Consult Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Sacrifice X Zombies, then reveal cards from the top of your library until you reveal a number of Zombie creature cards equal to the number of Zombies sacrificed this way. Put those cards onto the battlefield and the rest on the bottom of your library in a random order.",
        )
        .expect("counted consult should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("ConsultTopOfLibraryEffect")
            && debug.contains("MatchCount")
            && debug.contains("PriorEffectMetric")
            && debug.contains("Sacrificed")
            && debug.contains("Zombie"),
        "expected the consult stop to retain the exact sacrificed-Zombie count, got {debug}"
    );
    assert!(!debug.contains("PendingPriorEffectMetric"), "{debug}");
}

#[test]
pub(super) fn greatest_mana_value_exiled_this_way_resolves_to_the_exile_result() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Royal Funeral Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "When this enchantment enters, exile up to two target legendary creature cards from your graveyard. You draw X cards and you lose X life, where X is the greatest mana value among cards exiled this way.",
        )
        .expect("exiled-this-way aggregate should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ExileEffect")
            && debug.contains("PriorEffectMetric")
            && debug.contains("GreatestManaValue")
            && debug.contains("Exiled"),
        "expected the greatest-mana-value query to bind to the prior exile result, got {debug}"
    );
    assert!(!debug.contains("PendingPriorEffectMetric"), "{debug}");
}

#[test]
pub(super) fn tap_cost_power_keeps_the_typed_tapped_this_way_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Impelled Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Tap an untapped red creature you control other than this creature: This creature gets +X/+0 until end of turn, where X is the power of the creature tapped this way.",
        )
        .expect("tap-cost characteristic reference should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("tap_cost_0")
            && debug.contains("CharacteristicOfObjectThisWay")
            && debug.contains("Creature")
            && debug.contains("Tapped"),
        "expected the power value to retain typed tap-cost provenance, got {debug}"
    );
}

#[test]
pub(super) fn died_this_way_count_binds_to_the_destroy_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hellfire Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy all nonblack creatures. This spell deals X plus 3 damage to you, where X is the number of creatures that died this way.",
        )
        .expect("destroy-result damage count should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("DestroyEffect")
            && debug.contains("PriorEffectMetric")
            && debug.contains("Destroyed")
            && debug.contains("DiedThisWay"),
        "expected the died count to bind to the prior destroy result, got {debug}"
    );
    assert!(!debug.contains("PendingPriorEffectMetric"), "{debug}");
}

#[test]
pub(super) fn scry_amount_binds_the_dynamic_counter_target_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Elrond Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever you scry, put a +1/+1 counter on each of up to X target creatures, where X is the number of cards looked at while scrying this way.",
        )
        .expect("scry-derived dynamic target count should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("PutCountersEffect")
            && debug.contains("WithCountValue")
            && debug.contains("EventValue")
            && debug.contains("CardsLookedAtWhileScryingThisWay"),
        "expected the scry amount to remain attached to the target count, got {debug}"
    );
}

#[test]
pub(super) fn generic_damage_count_lowers_tapped_this_way_to_typed_prior_effect_metric() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tapped Damage Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Tap all untapped creatures. This spell deals damage to target player equal to the number of creatures tapped this way.",
        )
        .expect("generic tapped-this-way damage count should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("WithIdEffect")
            && debug.contains("TapEffect")
            && debug.contains("DealDamageEffect")
            && debug.contains("PriorEffectMetric")
            && debug.contains("AffectedObjects")
            && debug.contains("Tapped")
            && debug.contains("EqualTo"),
        "expected generic damage count to bind to the prior tap effect, got {debug}"
    );
}

#[test]
pub(super) fn raiding_party_per_player_tapped_count_uses_partitioned_metric_scope() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Raiding Party Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Sacrifice an Orc: Each player may tap any number of untapped white creatures they control. For each creature tapped this way, that player chooses up to two Plains. Then destroy all Plains that weren't chosen this way by any player.",
        )
        .expect("per-player tapped-this-way choice sequence should parse");

    let debug = format!("{:#?}", def.abilities);
    let compact = debug.split_whitespace().collect::<String>();
    assert!(
        debug.matches("ForPlayersEffect").count() >= 2
            && debug.contains("WithIdEffect")
            && debug.contains("TapEffect")
            && debug.contains("RepeatEffectsEffect")
            && debug.contains("ChooseObjectsEffect")
            && debug.contains("PriorEffectMetric")
            && compact.contains("IteratedPlayer"),
        "expected a per-player repeat over the matching tap-result partition, got {debug}"
    );
    assert!(!debug.contains("PendingPriorEffectMetric"), "{debug}");
}

#[test]
pub(super) fn upkeep_participant_dynamic_choice_survives_full_trigger_parsing() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Upkeep Choice Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of each player's upkeep, that player chooses a permanent for each card in their graveyard, then untaps those permanents.",
        )
        .expect("participant choice trigger should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(debug.contains("BeginningOfUpkeep"), "{debug}");
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.contains("UntapEffect"), "{debug}");
    assert!(debug.contains("ForEach"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
    assert!(debug.contains("IteratedPlayer"), "{debug}");
}

#[test]
pub(super) fn direct_then_each_other_player_choices_form_one_durable_collection() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Participant Choice Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose a nonland permanent you don't control, then each other player chooses a nonland permanent they don't control that hasn't been chosen this way. Destroy all other nonland permanents.",
        )
        .expect("direct and participant choices should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(debug.matches("ChooseObjectsEffect").count() >= 2, "{debug}");
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("NotYou"), "{debug}");
    assert!(debug.contains("DestroyEffect"), "{debug}");
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
    assert!(debug.contains("__chosen_objects__"), "{debug}");
}

#[test]
pub(super) fn filtered_mill_draw_counts_bind_graveyard_and_concrete_mill_tag() {
    for text in [
        "Target player mills four cards. You draw a card for each creature card put into their graveyard this way.",
        "At the beginning of your upkeep, target opponent mills three cards, then you draw a card for each land card put into their graveyard this way.",
    ] {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Filtered Mill Draw Variant")
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .expect("filtered mill-result draw count should parse");
        let debug = format!("{def:#?}");

        assert!(debug.contains("DrawCardsEffect"), "{debug}");
        assert!(debug.contains("Graveyard"), "{debug}");
        assert!(debug.contains("\"milled_0\""), "{debug}");
        assert!(!debug.contains("\"__it__\""), "{debug}");
    }
}

#[test]
pub(super) fn scoped_revealed_this_way_choice_does_not_default_to_battlefield() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Scoped Reveal Choice Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Reveal the top eight cards of your library. Choose any number of artifact and/or land cards revealed this way.",
        )
        .expect("scoped revealed-card choice should parse");

    let rendered = format!("{def:#?}");
    let compact = rendered.split_whitespace().collect::<String>();
    assert!(
        compact.contains("tag:TagKey(\"__sentence_helper_revealed")
            && compact.contains("relation:IsTaggedObject")
            && compact.contains("zone:None")
            && compact.contains("additional_zones:[Hand,Graveyard,Library,Exile"),
        "expected scoped revealed-card choice to stay tied to the revealed collection across hidden zones, got {rendered}"
    );
}

#[test]
pub(super) fn each_player_milled_this_way_choice_stays_tied_to_milled_cards() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Stitcher Geralf Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Wizard])
        .power_toughness(crate::card::PowerToughness::fixed(3, 4))
        .parse_text(
            "{2}{U}, {T}: Each player mills three cards. Exile up to two creature cards put into graveyards this way. Create an X/X blue Zombie creature token, where X is the total power of the cards exiled this way.",
        )
        .expect("Stitcher Geralf-style activated ability should parse");

    let rendered = format!("{def:#?}");
    assert!(
        rendered.matches("__sentence_helper_milled").count() >= 2
            && rendered.contains("tagged_constraints")
            && rendered.contains("relation: IsTaggedObject")
            && rendered.contains("Graveyard")
            && rendered.contains("__sentence_helper_exiled")
            && rendered.contains("SetBasePowerToughnessEffect")
            && rendered.contains("TotalPower"),
        "expected milled-this-way choice and X/X token sizing to stay tied to the milled/exiled helper tags, got {rendered}"
    );
}

#[test]
pub(super) fn rewrite_sequence_registry_matches_revealed_land_nonland_split_bottom_bundle() {
    let sentences = registry_sentence_inputs(
        "Reveal the top X cards of your library, where X is the number of lands sacrificed this way. Choose any number of artifact and/or land cards revealed this way. Put all nonland cards chosen this way onto the battlefield, then put all land cards chosen this way onto the battlefield tapped, then put the rest on the bottom of your library in a random order.",
    );

    let matched =
        super::super::effect_sentences::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("registry lookup should not error")
            .expect("registry should match revealed land/nonland split bundle");
    let debug = format!("{:#?}", matched.effects);

    assert_eq!(
        matched.name,
        "reveal-top-choose-any-revealed-land-nonland-split-rest-bottom"
    );
    assert_eq!(matched.consumed_sentences, 3);
    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("reveal: true"), "{debug}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(debug.contains("ItMatches"), "{debug}");
    assert!(debug.contains("PutOntoBattlefield"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn look_at_top_reveal_up_to_cards_bargain_branch_tracks_revealed_subset() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Thunderous Debut Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Bargain\nLook at the top twenty cards of your library. You may reveal up to two creature cards from among them. If this spell was bargained, put the revealed cards onto the battlefield. Otherwise, put the revealed cards into your hand. Then shuffle.",
        )
        .expect("bargain reveal subset branch should parse");

    let debug = format!("{def:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(
        debug.contains("ChooseObjectsEffect")
            && compact.contains("max:Some(2")
            && compact.contains("card_types:[Creature")
            && compact.contains("ThisSpellPaidLabel(OptionalCostRef{kind:Bargain")
            && compact.contains("zone:Battlefield")
            && compact.contains("zone:Hand"),
        "expected revealed creature subset to drive bargain battlefield/hand branch, got {debug}"
    );
}

#[test]
pub(super) fn bare_exiled_cards_in_sequence_bind_to_recent_exiled_result() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Recent Exiled Cards Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy X target artifacts and/or creatures. For each permanent destroyed this way, its controller reveals cards from the top of their library until an artifact or creature card is revealed and exiles that card. Those players put the exiled cards onto the battlefield, then shuffle.",
        )
        .expect("recent exiled-cards sequence should parse");

    let rendered = format!("{def:#?}");
    let effects = def
        .spell_effect
        .as_ref()
        .expect("destroy sequence should lower as a spell program")
        .flattened_default_effects();
    assert!(
        rendered.contains("ForEachTaggedEffect")
            && rendered.contains("ForEachControllerOfTaggedEffect")
            && rendered.contains("ConsultTopOfLibraryEffect")
            && rendered.contains("Artifact")
            && rendered.contains("Creature")
            && rendered.contains("zone: Exile")
            && rendered.contains("zone: Battlefield")
            && rendered.contains("__exiled_collection")
            && !rendered.contains("__source_exiled__"),
        "expected destroyed permanents to drive a staged reveal/exile collection, got {rendered}"
    );

    let [
        destroy_effect,
        collected_loop_effect,
        battlefield_effect,
        shuffle_effect,
    ] = effects
    else {
        panic!("expected four staged effects, got {effects:#?}");
    };
    let destroyed_tag = destroy_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("destroy result should be tagged")
        .tag
        .clone();
    let collected = collected_loop_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("per-object loop should collect all of its exile outcomes");
    let collection_tag = &collected.tag;
    let per_object = collected
        .effect
        .downcast_ref::<crate::effects::ForEachTaggedEffect<crate::effect::Effect>>()
        .expect("consult and exile should remain in the per-object loop");
    assert_eq!(per_object.tag, destroyed_tag);
    assert_eq!(per_object.effects.len(), 2, "{per_object:#?}");
    assert!(
        per_object.effects[0]
            .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()
            .is_some()
            && (per_object.effects[1]
                .downcast_ref::<crate::effects::ExileEffect>()
                .is_some()
                || per_object.effects[1]
                    .downcast_ref::<crate::effects::MoveToZoneEffect>()
                    .is_some_and(|move_to_zone| move_to_zone.zone == Zone::Exile)),
        "{per_object:#?}"
    );

    let battlefield = battlefield_effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .expect("the complete exiled collection should move after the loop");
    assert!(
        battlefield.zone == Zone::Battlefield
            && matches!(
                battlefield.target.base(),
                crate::target::ChooseSpec::Tagged(tag) if tag == collection_tag
            ),
        "{battlefield:#?}"
    );
    let shuffle_by_controller = shuffle_effect
        .downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect<crate::effect::Effect>>()
        .expect("participating controllers should each shuffle once");
    assert_eq!(shuffle_by_controller.tag, destroyed_tag);
    assert!(
        matches!(
            shuffle_by_controller.effects.as_slice(),
            [effect]
                if effect
                    .downcast_ref::<crate::effects::ShuffleLibraryEffect>()
                    .is_some_and(|shuffle| {
                        shuffle.player == crate::target::PlayerFilter::IteratedPlayer
                    })
        ),
        "{shuffle_by_controller:#?}"
    );
}

#[test]
pub(super) fn result_metric_dynamic_token_count_uses_prior_search_exile_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Myr Incubator Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{6}, {T}, Sacrifice this artifact: Search your library for any number of artifact cards, exile them, then create that many 1/1 colorless Myr artifact creature tokens. Then shuffle.",
        )
        .expect("Myr Incubator-style dynamic token count should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("WithIdEffect")
            && debug.contains("SequenceEffect")
            && debug.contains("EffectValue")
            && debug.contains("CreateTokenEffect"),
        "expected token count to reference the prior search/exile effect result, got {debug}"
    );
}

#[test]
pub(super) fn incubate_where_x_twice_lowers_to_amount_and_count_values() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Glistening Dawn Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Incubate X twice, where X is the number of lands you control.")
        .expect("incubate where-X amount should parse");

    let debug = format!("{def:#?}");
    let compact = debug.split_whitespace().collect::<String>();
    assert!(
        debug.contains("IncubateEffect")
            && compact.contains("count:Fixed(2")
            && debug.contains("WhereXIs")
            && compact.contains("card_types:[Land"),
        "expected incubate amount to bind to land count and count to be twice, got {debug}"
    );
}

#[test]
pub(super) fn incubate_its_controller_binds_controller_and_mana_value() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Excise Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile target nonland permanent. Its controller incubates X, where X is its mana value.",
        )
        .expect("target controller incubate clause should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("IncubateEffect")
            && debug.contains("controller: ControllerOf")
            && debug.contains("ManaValueOf")
            && debug.contains("WhereXIs"),
        "expected incubate to use target controller and mana-value amount, got {debug}"
    );
}

#[test]
pub(super) fn destroy_target_nonland_permanent_with_life_equal_to_mana_value_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Feed the Swarm Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy target creature or enchantment an opponent controls. You lose life equal to that permanent's mana value.",
        )
        .expect("life-loss equal to that permanent's mana value should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("DestroyEffect")
            && debug.contains("LoseLifeEffect")
            && debug.contains("ManaValueOf")
            && debug.contains("Tagged")
            && debug.contains("it"),
        "expected lose-life amount to use the destroyed permanent's mana value, got {debug}"
    );
}

#[test]
pub(super) fn equal_to_dynamic_token_count_can_use_opponent_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Opponent Count Token Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, create a number of 1/1 black Rat creature tokens equal to the number of opponents you have.",
        )
        .expect("opponent-count token creation should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("CreateTokenEffect")
            && debug.contains("CountPlayers")
            && debug.contains("Opponent"),
        "expected dynamic create count to use opponent count, got {debug}"
    );
}

#[test]
pub(super) fn token_with_each_opponent_trigger_validates_inner_iterated_player_binding() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wizard Token Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, create a 0/1 black Wizard creature token with \"Whenever you cast a noncreature spell, this token deals 1 damage to each opponent.\"",
        )
        .expect("quoted token trigger with each-opponent damage should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("CreateTokenEffect")
            && debug.contains("SpellCast")
            && debug.contains("ForPlayersEffect")
            && debug.contains("IteratedPlayer"),
        "expected created token trigger to keep its own each-opponent binding, got {debug}"
    );
}

#[test]
pub(super) fn where_x_life_total_can_drive_create_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Life Total Token Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Create X 2/2 white Cat creature tokens, where X is your life total.")
        .expect("life-total where-X token creation should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("CreateTokenEffect") && debug.contains("LifeTotal") && debug.contains("You"),
        "expected dynamic create count to use your life total, got {debug}"
    );
}

#[test]
pub(super) fn quoted_token_rules_keep_outer_where_x_value_bindings() {
    let mycotyrant =
        CardDefinitionBuilder::new(CardId::from_raw(1), "Descend Token Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "At the beginning of your end step, create X 1/1 black Fungus creature tokens with \"This token can't block,\" where X is the number of times you descended this turn.",
            )
            .expect("quoted token rule should preserve the outer descend count");
    let mycotyrant_debug = format!("{mycotyrant:#?}");
    assert!(
        mycotyrant_debug.contains("CreateTokenEffect")
            && mycotyrant_debug.contains("TurnHistoryCount")
            && mycotyrant_debug.contains("Descended"),
        "{mycotyrant_debug}"
    );

    let colony = CardDefinitionBuilder::new(CardId::from_raw(2), "Damage Token Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature dies, create X 1/1 black Rat creature tokens with \"This token can't block,\" where X is the amount of damage dealt to it this turn.",
        )
        .expect("quoted token rule should preserve the outer source-damage total");
    let colony_debug = format!("{colony:#?}");
    assert!(
        colony_debug.contains("CreateTokenEffect")
            && colony_debug.contains("TurnHistoryCount")
            && colony_debug.contains("DamageDealtToSource"),
        "{colony_debug}"
    );

    let tend = CardDefinitionBuilder::new(CardId::from_raw(3), "Sacrifice Token Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "As an additional cost to cast this spell, sacrifice a creature.\nCreate X 1/1 black and green Pest creature tokens with \"When this token dies, you gain 1 life,\" where X is the sacrificed creature's power.",
        )
        .expect("quoted token rule should preserve the outer sacrificed-power value");
    let tend_debug = format!("{tend:#?}");
    assert!(
        tend_debug.contains("CreateTokenEffect")
            && tend_debug.contains("PowerOf")
            && tend_debug.contains("sacrificed_0"),
        "{tend_debug}"
    );
}

#[test]
pub(super) fn where_x_named_graveyard_count_binds_return_to_hand_target_count() {
    let text = "Return up to X target creatures to their owners' hands, where X is one plus the number of cards named Aether Burst in all graveyards as you cast this spell.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify return where-X sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("return-to-hand where-X target count should parse");
    let debug = format!("{parsed:#?}");

    assert!(
        debug.contains("ReturnToHand")
            && debug.contains("WithCountValue")
            && debug.contains("WhereXIs")
            && debug.contains("aether burst"),
        "expected return target count to bind to named-card graveyard where-X value, got {debug}"
    );
}

#[test]
pub(super) fn where_x_colors_that_creature_was_binds_return_to_hand_target_count() {
    let text = "Return up to X cards from your graveyard to your hand, where X is the number of colors that creature was.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify return where-X sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("return-to-hand color-count where-X target count should parse");
    let debug = format!("{parsed:#?}");

    assert!(
        debug.contains("ReturnToHand")
            && debug.contains("WithCountValue")
            && debug.contains("ColorsAmong")
            && debug.contains("sacrificed_0"),
        "expected return target count to bind to sacrificed-creature color count, got {debug}"
    );
}

#[test]
pub(super) fn comma_then_keyword_action_splits_before_subject_verb_followup() {
    let text = "Amass Orcs X, then Goblins and Orcs you control gain double strike and haste until end of turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify amass comma-then line");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("amass comma-then sentence should split into sibling effects");
    let debug = format!("{parsed:#?}");

    assert!(
        debug.contains("Amass")
            && debug.contains("DoubleStrike")
            && debug.contains("Haste")
            && debug.contains("Goblin")
            && debug.contains("Orc"),
        "expected amass effect plus Goblin/Orc ability grant, got {debug}"
    );
}

#[test]
pub(super) fn comma_then_target_player_create_carries_that_player_count_to_pump() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dark Salvation Variant")
        .mana_cost(super::super::util::parse_scryfall_mana_cost("{X}{X}{B}").unwrap())
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target player creates X 2/2 black Zombie creature tokens, then up to one target creature gets -1/-1 until end of turn for each Zombie that player controls.",
        )
        .expect("target-player create then that-player pump should lower");

    let debug = format!("{def:#?}");
    let compact_debug = format!("{def:?}");
    assert!(
        debug.contains("CreateTokenEffect")
            && debug.contains("ModifyPowerToughnessForEachEffect")
            && compact_debug.contains("Target(Any)")
            && !compact_debug.contains("IteratedPlayer"),
        "expected pump count to bind that-player reference to prior target player, got {debug}"
    );
}

#[test]
pub(super) fn where_x_half_life_create_token_keeps_rounded_up_tail() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rounded Life Token Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create an X/X black Nightmare Horror creature token, where X is half your life total, rounded up. It deals X damage to you.",
        )
        .expect("half-life rounded-up where-X token creation should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("CreateTokenEffect")
            && debug.contains("HalfLifeTotalRoundedUp")
            && debug.contains("DealDamageEffect"),
        "expected rounded-up half-life create token plus X damage, got {debug}"
    );
}

#[test]
pub(super) fn partner_with_keyword_line_lowers_to_partner_with_marker_and_search_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Partner With Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Partner with Frodo, Adventurous Hobbit (When this creature enters, target player may put Frodo, Adventurous Hobbit into their hand from their library, then shuffle.)",
        )
        .expect("partner-with keyword line should parse");

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("partnerwith")
            && debug.contains("searchlibraryeffect")
            && debug.contains("frodo, adventurous hobbit")
            && debug.contains("destination: hand")
            && debug.contains("reveal: true")
            && debug.contains("search_mode: exact"),
        "expected partner-with to lower to a PartnerWith marker plus named-card search trigger, got {debug}"
    );
}

#[test]
pub(super) fn optional_cost_sacrifice_copy_count_uses_times_paid_runtime_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Plumb Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "As an additional cost to cast this spell, you may sacrifice one or more creatures. When you do, copy this spell for each creature sacrificed this way.\nYou draw a card and you lose 1 life.",
        )
        .expect("Plumb-style optional-cost copy count should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("CopySpellEffect")
            && debug.contains("TimesPaidLabel")
            && debug.contains("ThisSpellPaidLabel"),
        "expected copy count to use the optional sacrifice cost's runtime payment count, got {debug}"
    );
}

#[test]
pub(super) fn exile_cost_count_can_drive_dynamic_token_power_toughness() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Exile Cost Token Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{1}{B}, Exile one or more creature cards from your graveyard: Create a tapped X/X black Zombie Horror creature token, where X is twice the number of cards exiled this way.",
        )
        .expect("exile-cost X/X token should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("exile_cost_0")
            && debug.contains("SetBasePowerToughnessEffect")
            && debug.contains("Scaled(")
            && debug.contains("Count("),
        "expected token power/toughness to count cards exiled as an activation cost, got {debug}"
    );
}

#[test]
pub(super) fn life_lost_this_way_lowers_to_prior_effect_metric() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Life Lost Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Each opponent loses 1 life. You gain life equal to the life lost this way.")
        .expect("life-lost-this-way should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("WithIdEffect")
            && debug.contains("EffectMetric")
            && debug.contains("LifeLost"),
        "expected life gain to bind to prior life-loss metric, got {debug}"
    );
}

#[test]
pub(super) fn standalone_each_opponent_poison_counter_is_a_statement_line() {
    let tokens = lex_line("Each opponent gets a poison counter.", 0)
        .expect("poison-counter line should lex");
    assert!(
        parse_effect_sentence_lexed(&tokens).is_ok_and(|effects| !effects.is_empty()),
        "poison-counter line should produce typed effects"
    );
    assert!(
        super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
            .expect("static probe should not error")
            .is_none(),
        "poison-counter line should not be claimed as static"
    );
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Poison Counter Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Each opponent gets a poison counter.")
        .expect("standalone poison-counter statement should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("PoisonCountersEffect"),
        "expected poison effect, got {debug}"
    );
}

#[test]
pub(super) fn dance_of_the_manse_strict_parser_lowers_returned_permanent_animation() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dance of the Manse")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::White],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return up to X target artifact and/or non-Aura enchantment cards each with mana value X or less from your graveyard to the battlefield. If X is 6 or more, those permanents are 4/4 creatures in addition to their other types.",
        )
        .expect("Dance of the Manse should parse strictly");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("ReturnFromGraveyardToBattlefieldEffect")
            && debug.contains("XValueAtLeast")
            && debug.contains("ApplyContinuousEffect")
            && debug.contains("AddCardTypes")
            && debug.contains("SetPowerToughness"),
        "expected Dance of the Manse to return cards and conditionally animate returned permanents, got {debug}"
    );
}

#[test]
pub(super) fn gain_life_equal_to_the_power_of_target_creature_you_control_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wall of Reverence Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nAt the beginning of your end step, you may gain life equal to the power of target creature you control.",
        )
        .expect("target creature power life gain should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("GainLifeEffect") && debug.contains("PowerOf") && debug.contains("Target("),
        "expected life gain amount to use target creature power, got {debug}"
    );
}

#[test]
pub(super) fn dynamic_return_count_lowers_to_prior_effect_metric_choice() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dynamic Return Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Sacrifice any number of permanents. Return that many creature cards from your graveyard to the battlefield.",
        )
        .expect("dynamic return count should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("WithIdEffect")
            && debug.contains("ChooseObjectsEffect")
            && debug.contains("ReturnFromGraveyardToBattlefieldEffect")
            && (debug.contains("EffectValue") || debug.contains("EffectMetric")),
        "expected return count to bind to prior sacrifice metric choice, got {debug}"
    );
}

#[test]
pub(super) fn unresolved_event_value_without_prior_effect_is_parse_error() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Unbound Dynamic Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Return that many creature cards from your graveyard to the battlefield.")
        .expect_err("unbound dynamic return count should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("event-derived amount requires a compatible trigger")
            || message
                .contains("event-derived amount requires a compatible trigger or prior effect"),
        "expected unresolved dynamic value parse error, got {message}"
    );
}

#[test]
pub(super) fn rewrite_search_library_clause_marker_scan_tracks_destination_boundaries() {
    let reveal_put_shuffle = lex_line(
        "Search your library for a creature card, reveal it, put it into your hand, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify reveal/put/shuffle search text");
    let search_tokens = super::super::grammar::effects::split_search_library_sentence_head_lexed(
        &reveal_put_shuffle,
    )
    .expect("search head splitter should match reveal/put/shuffle search")
    .search_tokens;
    let markers =
        super::super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
            .expect("grammar-owned clause markers should parse reveal/put/shuffle search");

    assert_eq!(markers.for_idx, 3);
    assert!(markers.put_idx.is_some());
    assert!(markers.reveal_idx.is_some());
    assert!(markers.shuffle_idx.is_some());
    assert!(markers.has_explicit_destination);
    assert_eq!(markers.filter_boundary, markers.put_idx.unwrap());

    let exile_it = lex_line(
        "Search target opponent's library for a card and exile it face down.",
        0,
    )
    .expect("rewrite lexer should classify exile search text");
    let search_tokens =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&exile_it)
            .expect("search head splitter should match exile search")
            .search_tokens;
    let markers =
        super::super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
            .expect("grammar-owned clause markers should parse exile search");

    assert!(markers.exile_idx.is_some());
    assert!(markers.has_explicit_destination);
    assert_eq!(markers.filter_boundary, markers.exile_idx.unwrap());
}

#[test]
pub(super) fn rewrite_search_library_filter_boundary_scan_stops_before_reveal_or_then() {
    let reveal_put_shuffle = lex_line(
        "Search your library for a creature card, reveal it, put it into your hand, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify reveal/put/shuffle search text");
    let search_tokens = super::super::grammar::effects::split_search_library_sentence_head_lexed(
        &reveal_put_shuffle,
    )
    .expect("search head splitter should match reveal/put/shuffle search")
    .search_tokens;
    let markers =
        super::super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
            .expect("grammar-owned clause markers should parse reveal/put/shuffle search");
    let boundary = super::super::grammar::effects::find_search_library_filter_boundary_lexed(
        search_tokens,
        markers.for_idx,
        markers.filter_boundary,
    );

    assert_eq!(
        render_token_slice(&search_tokens[markers.for_idx + 1..boundary.filter_end]),
        "a creature card",
        "filter boundary should stop before the reveal clause"
    );

    let face_down_exile = lex_line(
        "Search target opponent's library for a card and exile it face down.",
        0,
    )
    .expect("rewrite lexer should classify exile search text");
    let search_tokens =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&face_down_exile)
            .expect("search head splitter should match exile search")
            .search_tokens;
    let markers =
        super::super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
            .expect("grammar-owned clause markers should parse exile search");
    let boundary = super::super::grammar::effects::find_search_library_filter_boundary_lexed(
        search_tokens,
        markers.for_idx,
        markers.filter_boundary,
    );

    assert_eq!(
        render_token_slice(&search_tokens[markers.for_idx + 1..boundary.filter_end]),
        "a card",
        "filter boundary should stop before the exile-it destination clause"
    );
}

#[test]
pub(super) fn rewrite_search_library_discard_followup_scan_finds_clause_before_shuffle() {
    let discard_then_shuffle = lex_line(
        "Search your library for a basic land card, put it onto the battlefield tapped, then discard a card, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify discard-before-shuffle search text");
    let search_tokens = super::super::grammar::effects::split_search_library_sentence_head_lexed(
        &discard_then_shuffle,
    )
    .expect("search head splitter should match discard-before-shuffle search")
    .search_tokens;
    let markers =
        super::super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
            .expect("grammar-owned clause markers should parse discard-before-shuffle search");
    let followup =
        super::super::grammar::effects::find_search_library_discard_before_shuffle_followup_lexed(
            search_tokens,
            markers.put_idx,
        )
        .expect("discard-before-shuffle helper should find the discard clause");

    assert_eq!(
        render_token_slice(&search_tokens[followup.discard_idx..followup.discard_end]),
        "discard a card",
        "discard followup should stop before the trailing shuffle clause"
    );
    assert!(followup.shuffle_idx > followup.discard_end);
}

#[test]
pub(super) fn rewrite_search_library_trailing_life_followup_scan_returns_life_clause_only() {
    let trailing_life = lex_line(
        "Search your library for a card, put that card into your hand, then shuffle and you gain 3 life.",
        0,
    )
    .expect("rewrite lexer should classify trailing-life search text");
    let search_tokens =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&trailing_life)
            .expect("search head splitter should match trailing-life search")
            .search_tokens;
    let markers =
        super::super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
            .expect("grammar-owned clause markers should parse trailing-life search");
    let trailing_tokens =
        super::super::grammar::effects::find_search_library_trailing_life_followup_lexed(
            search_tokens,
            markers.put_idx.unwrap_or(markers.filter_boundary),
        )
        .expect("trailing-life helper should find the life-gain clause");

    assert_eq!(
        render_token_slice(trailing_tokens),
        "you gain 3 life.",
        "trailing-life helper should strip the leading and-marker"
    );
}

#[test]
pub(super) fn rewrite_search_library_trailing_create_followup_preserves_create_before_shuffle() {
    let text = "Search your library for any number of artifact cards, exile them, then create that many 1/1 colorless Myr artifact creature tokens. Then shuffle.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify search-create text");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("search-create sequence should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("SearchLibrary"), "{debug}");
    assert!(debug.contains("CreateTokenWithMods"), "{debug}");
    assert!(debug.contains("ShuffleLibrary"), "{debug}");
}

#[test]
pub(super) fn rewrite_search_library_effect_routing_tracks_destination_and_flags() {
    let reveal_put_shuffle = lex_line(
        "Search your library for a creature card, reveal it, put it onto the battlefield tapped, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify routed battlefield search text");
    let search_tokens = super::super::grammar::effects::split_search_library_sentence_head_lexed(
        &reveal_put_shuffle,
    )
    .expect("search head splitter should match routed battlefield search")
    .search_tokens;
    let markers =
        super::super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
            .expect("grammar-owned clause markers should parse routed battlefield search");
    let routing = super::super::grammar::effects::derive_search_library_effect_routing_lexed(
        &reveal_put_shuffle,
        search_tokens,
        markers,
        false,
    );

    assert_eq!(routing.destination, crate::zone::Zone::Battlefield);
    assert!(routing.reveal);
    assert!(routing.shuffle);
    assert!(routing.has_tapped_modifier);
    assert!(!routing.face_down_exile);
    assert!(!routing.split_battlefield_and_hand);

    let split_destination = lex_line(
        "Search your library for two basic land cards, put one onto the battlefield tapped and the other into your hand, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify split-destination search text");
    let search_tokens = super::super::grammar::effects::split_search_library_sentence_head_lexed(
        &split_destination,
    )
    .expect("search head splitter should match split-destination search")
    .search_tokens;
    let markers =
        super::super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
            .expect("grammar-owned clause markers should parse split-destination search");
    let routing = super::super::grammar::effects::derive_search_library_effect_routing_lexed(
        &split_destination,
        search_tokens,
        markers,
        false,
    );

    assert!(routing.split_battlefield_and_hand);
    assert!(routing.shuffle);
    assert!(routing.has_tapped_modifier);

    let face_down_exile = lex_line(
        "Search target opponent's library for a card and exile it face down.",
        0,
    )
    .expect("rewrite lexer should classify face-down exile search text");
    let search_tokens =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&face_down_exile)
            .expect("search head splitter should match face-down exile search")
            .search_tokens;
    let markers =
        super::super::grammar::effects::scan_search_library_clause_markers_lexed(search_tokens)
            .expect("grammar-owned clause markers should parse face-down exile search");
    let routing = super::super::grammar::effects::derive_search_library_effect_routing_lexed(
        &face_down_exile,
        search_tokens,
        markers,
        false,
    );

    assert_eq!(routing.destination, crate::zone::Zone::Exile);
    assert!(routing.face_down_exile);
    assert!(!routing.shuffle);
}

#[test]
pub(super) fn rewrite_split_destination_search_uses_one_tagged_partition() {
    let lexed = lex_line(
        "Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle.",
        0,
    )
    .expect("split-destination search should lex");
    let effects = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("split-destination search should parse");
    let debug = format!("{effects:#?}");
    let partition = match effects.as_slice() {
        [EffectAst::CommaThen { effects }] => effects.as_slice(),
        effects => effects,
    };

    let searches = partition
        .iter()
        .filter_map(|effect| match effect {
            EffectAst::ChooseObjectsAcrossZones { count, .. } => Some(count),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(searches, vec![&ChoiceCount::up_to(2)], "{debug}");
    assert_eq!(debug.matches("RevealTagged").count(), 1, "{debug}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(debug.contains("PutTaggedRemainderInZone"), "{debug}");
    assert!(debug.contains("zone: Hand"), "{debug}");
    assert_eq!(debug.matches("ShuffleLibrary").count(), 1, "{debug}");
}

#[test]
pub(super) fn search_filter_and_or_keeps_basic_land_and_gate_as_separate_branches() {
    let tokens =
        lex_line("basic land cards and/or Gate cards", 0).expect("and/or search filter should lex");
    let filter =
        super::super::search_library_support::parse_search_library_disjunction_filter(&tokens)
            .expect("and/or search filter should parse");

    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert_eq!(
        filter.union_surface.connective(),
        crate::filter::ObjectFilterUnionConnective::AndOr
    );
    assert!(filter.any_of.iter().any(|branch| {
        branch.card_types.contains(&CardType::Land)
            && branch.supertypes.contains(&crate::types::Supertype::Basic)
    }));
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.subtypes.contains(&Subtype::Gate))
    );
}

#[test]
pub(super) fn rewrite_search_library_subject_routing_tracks_zone_owner_prefixes() {
    let target_opponent_multi_zone = lex_line(
        "Search target opponent's graveyard, hand, and library for a card.",
        0,
    )
    .expect("rewrite lexer should classify target-opponent multi-zone search text");
    let search_tokens = super::super::grammar::effects::split_search_library_sentence_head_lexed(
        &target_opponent_multi_zone,
    )
    .expect("search head splitter should match target-opponent multi-zone search")
    .search_tokens;
    let routing = super::super::grammar::effects::derive_search_library_subject_routing_lexed(
        search_tokens,
        crate::cards::builders::PlayerAst::You,
    )
    .expect("subject routing helper should parse target-opponent multi-zone prefix");

    assert_eq!(routing.player, crate::cards::builders::PlayerAst::That);
    assert!(routing.search_player_target.is_some());
    assert_eq!(
        routing.search_zones_override,
        Some(vec![
            crate::zone::Zone::Graveyard,
            crate::zone::Zone::Hand,
            crate::zone::Zone::Library,
        ])
    );

    let its_controller = lex_line("Search its controller's library for a card.", 0)
        .expect("rewrite lexer should classify controller-owned search text");
    let search_tokens =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&its_controller)
            .expect("search head splitter should match controller-owned search")
            .search_tokens;
    let routing = super::super::grammar::effects::derive_search_library_subject_routing_lexed(
        search_tokens,
        crate::cards::builders::PlayerAst::You,
    )
    .expect("subject routing helper should parse controller-owned prefix");

    assert_eq!(
        routing.player,
        crate::cards::builders::PlayerAst::ItsController
    );
    assert!(routing.search_player_target.is_none());
    assert!(routing.search_zones_override.is_none());

    let its_controller_possessive =
        lex_line("Its controller may search their library for a card.", 0)
            .expect("rewrite lexer should classify controller subject search text");
    let head = super::super::grammar::effects::split_search_library_sentence_head_lexed(
        &its_controller_possessive,
    )
    .expect("search head splitter should match controller subject search");
    let routing = super::super::grammar::effects::derive_search_library_subject_routing_lexed(
        head.search_tokens,
        crate::cards::builders::PlayerAst::ItsController,
    )
    .expect("subject routing helper should parse controller subject possessive search");
    assert_eq!(
        routing.forced_library_owner,
        Some(crate::target::PlayerFilter::ControllerOf(
            crate::filter::ObjectRef::Target
        ))
    );

    let its_owner_possessive = lex_line("Its owner may search their library for a card.", 0)
        .expect("rewrite lexer should classify owner subject search text");
    let head = super::super::grammar::effects::split_search_library_sentence_head_lexed(
        &its_owner_possessive,
    )
    .expect("search head splitter should match owner subject search");
    let routing = super::super::grammar::effects::derive_search_library_subject_routing_lexed(
        head.search_tokens,
        crate::cards::builders::PlayerAst::ItsOwner,
    )
    .expect("subject routing helper should parse owner subject possessive search");
    assert_eq!(
        routing.forced_library_owner,
        Some(crate::target::PlayerFilter::OwnerOf(
            crate::filter::ObjectRef::Target
        ))
    );

    let your_multi_zone = lex_line(
        "Search your graveyard, hand, and library for a creature card.",
        0,
    )
    .expect("rewrite lexer should classify your multi-zone search text");
    let search_tokens =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&your_multi_zone)
            .expect("search head splitter should match your multi-zone search")
            .search_tokens;
    let routing = super::super::grammar::effects::derive_search_library_subject_routing_lexed(
        search_tokens,
        crate::cards::builders::PlayerAst::You,
    )
    .expect("subject routing helper should parse your multi-zone prefix");

    assert_eq!(
        routing.search_zones_override,
        Some(vec![
            crate::zone::Zone::Graveyard,
            crate::zone::Zone::Hand,
            crate::zone::Zone::Library,
        ])
    );

    let each_player_search = lex_line("Each player may search their library for a card.", 0)
        .expect("rewrite lexer should classify each-player search text");
    let head = super::super::grammar::effects::split_search_library_sentence_head_lexed(
        &each_player_search,
    )
    .expect("search head splitter should match each-player may-search text");
    assert_eq!(
        super::super::grammar::effects::search_library_subject_player_iteration_filter_lexed(
            head.subject_tokens
        ),
        Some(crate::target::PlayerFilter::Any)
    );
    let routing = super::super::grammar::effects::derive_search_library_subject_routing_lexed(
        head.search_tokens,
        crate::cards::builders::PlayerAst::That,
    )
    .expect("subject routing helper should parse iterated-player library search");
    assert_eq!(
        routing.forced_library_owner,
        Some(crate::target::PlayerFilter::IteratedPlayer)
    );
}

#[test]
pub(super) fn rewrite_search_library_count_prefix_parser_tracks_search_modes() {
    let any_number = lex_line("search your library for any number of creature cards", 0)
        .expect("rewrite lexer should classify any-number search text");
    let count_tokens =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&any_number)
            .expect("search head splitter should match any-number search")
            .search_tokens[4..8]
            .to_vec();
    let parsed =
        super::super::grammar::effects::parse_search_library_count_prefix_lexed(&count_tokens);

    assert_eq!(
        format!("{:?}", parsed.count),
        format!("{:?}", crate::cards::builders::ChoiceCount::any_number())
    );
    assert_eq!(
        parsed.search_mode,
        crate::effect::SearchSelectionMode::Optional
    );
    assert_eq!(parsed.count_used, 3);

    let up_to_x = lex_line("search your library for up to X cards", 0)
        .expect("rewrite lexer should classify up-to-x search text");
    let count_tokens =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&up_to_x)
            .expect("search head splitter should match up-to-x search")
            .search_tokens[4..7]
            .to_vec();
    let parsed =
        super::super::grammar::effects::parse_search_library_count_prefix_lexed(&count_tokens);

    assert_eq!(
        format!("{:?}", parsed.count),
        format!(
            "{:?}",
            crate::cards::builders::ChoiceCount::up_to_dynamic_x()
        )
    );
    assert_eq!(
        parsed.search_mode,
        crate::effect::SearchSelectionMode::Optional
    );
    assert_eq!(parsed.count_used, 3);

    let all_cards = lex_line("search your library for all cards", 0)
        .expect("rewrite lexer should classify all-cards search text");
    let count_tokens =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&all_cards)
            .expect("search head splitter should match all-cards search")
            .search_tokens[4..5]
            .to_vec();
    let parsed =
        super::super::grammar::effects::parse_search_library_count_prefix_lexed(&count_tokens);

    assert_eq!(
        format!("{:?}", parsed.count),
        format!("{:?}", crate::cards::builders::ChoiceCount::any_number())
    );
    assert_eq!(
        parsed.search_mode,
        crate::effect::SearchSelectionMode::AllMatching
    );
    assert_eq!(parsed.count_used, 1);

    let exact_three = lex_line("search your library for three cards", 0)
        .expect("rewrite lexer should classify exact-count search text");
    let count_tokens =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&exact_three)
            .expect("search head splitter should match exact-count search")
            .search_tokens[4..6]
            .to_vec();
    let parsed =
        super::super::grammar::effects::parse_search_library_count_prefix_lexed(&count_tokens);

    assert_eq!(
        parsed.count,
        crate::cards::builders::ChoiceCount::exactly(3)
    );
    assert_eq!(
        parsed.search_mode,
        crate::effect::SearchSelectionMode::Exact
    );
    assert_eq!(parsed.count_used, 1);
}

#[test]
pub(super) fn rewrite_search_library_same_name_tail_parser_splits_reference_suffixes() {
    let chosen_name = lex_line("artifact card with the chosen name", 0)
        .expect("rewrite lexer should classify chosen-name filter text");
    let parsed = super::super::grammar::effects::parse_search_library_same_name_reference_lexed(
        &chosen_name,
        chosen_name.clone(),
        &render_token_slice(&chosen_name),
    )
    .expect("same-name helper should parse chosen-name suffix");

    assert_eq!(
        render_token_slice(&parsed.filter_tokens),
        "artifact card",
        "chosen-name suffix should be removed from the base filter"
    );
    assert!(matches!(
        parsed.same_name_reference,
        Some(super::super::grammar::effects::SearchLibrarySameNameReference::Tagged(_))
    ));

    let target_reference = lex_line("creature card with the same name as target creature", 0)
        .expect("rewrite lexer should classify target same-name filter text");
    let parsed = super::super::grammar::effects::parse_search_library_same_name_reference_lexed(
        &target_reference,
        target_reference.clone(),
        &render_token_slice(&target_reference),
    )
    .expect("same-name helper should parse target-reference suffix");

    assert_eq!(
        render_token_slice(&parsed.filter_tokens),
        "creature card",
        "target same-name suffix should be removed from the base filter"
    );
    assert!(matches!(
        parsed.same_name_reference,
        Some(super::super::grammar::effects::SearchLibrarySameNameReference::Target(_))
    ));

    let exiled_reference = lex_line("cards with the same name as the exiled card", 0)
        .expect("rewrite lexer should classify source-exiled same-name filter text");
    let parsed = super::super::grammar::effects::parse_search_library_same_name_reference_lexed(
        &exiled_reference,
        exiled_reference.clone(),
        &render_token_slice(&exiled_reference),
    )
    .expect("same-name helper should preserve a source-exiled reference");

    assert_eq!(render_token_slice(&parsed.filter_tokens), "cards");
    assert!(matches!(
        parsed.same_name_reference,
        Some(super::super::grammar::effects::SearchLibrarySameNameReference::Tagged(tag))
            if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
    ));
}

#[test]
pub(super) fn rewrite_search_library_helper_parsers_track_mana_and_same_name_suffixes() {
    let mana_tokens = lex_line("artifact card with mana value 2 or 3", 0)
        .expect("rewrite lexer should classify mana-value helper input");
    let (base_filter, constraint) =
        super::super::extract_search_library_mana_constraint(&mana_tokens)
            .expect("mana-value helper should split base filter and clause");
    assert_eq!(token_word_refs(&base_filter), vec!["artifact", "card"]);
    assert!(matches!(
        constraint,
        super::super::SearchLibraryManaConstraint::OneOf(values)
            if values == vec![2, 3]
    ));

    let same_name_tokens = lex_line("creature card with the same name as that card", 0)
        .expect("rewrite lexer should classify same-name helper input");
    let (base_filter, reference_tokens) =
        super::super::split_search_same_name_reference_filter(&same_name_tokens)
            .expect("same-name helper should split reference suffix");
    assert_eq!(token_word_refs(&base_filter), vec!["creature", "card"]);
    assert_eq!(token_word_refs(&reference_tokens), vec!["that", "card"]);

    let different_name_tokens = lex_line(
        "Curse card that doesn't have the same name as a Curse attached to enchanted player",
        0,
    )
    .expect("rewrite lexer should classify negated same-name helper input");
    let (base_filter, reference_tokens) =
        super::super::split_search_different_name_reference_filter(&different_name_tokens)
            .expect("different-name helper should split reference suffix");
    assert_eq!(token_word_refs(&base_filter), vec!["Curse", "card"]);
    assert_eq!(
        token_word_refs(&reference_tokens),
        vec!["a", "Curse", "attached", "to", "enchanted", "player"]
    );
}

#[test]
pub(super) fn curse_of_misfortunes_search_excludes_names_of_attached_curses() {
    let text = "Enchant player\nAt the beginning of enchanted player's upkeep, you may search your library for a Curse card that doesn't have the same name as a Curse attached to enchanted player, put it onto the battlefield attached to that player, then shuffle.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Curse of Misfortunes")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Curse])
        .parse_text(text)
        .expect("Curse of Misfortunes should parse");
    let debug = format!("{:#?}", def.abilities);
    let compact = debug.split_whitespace().collect::<String>();

    assert!(debug.contains("TagMatchingObjectsEffect"), "{debug}");
    assert!(debug.contains("DifferentNameFromTagged"), "{debug}");
    assert!(debug.contains("attached_to_player: Some"), "{debug}");
    assert!(debug.contains("\"enchanted\""), "{debug}");
    assert!(
        compact.contains("target:Target(Player(TaggedPlayer(TagKey(\"enchanted\""),
        "the attachment's `that player` must resolve to the enchanted player: {debug}"
    );
}

#[test]
pub(super) fn activated_multi_zone_search_shuffle_is_gated_on_searching_library() {
    let text = "{T}, Sacrifice three Clerics: Search your graveyard, hand, and/or library for a card named Scion of Darkness and put it onto the battlefield. If you search your library this way, shuffle.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dark Supplicant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Cleric])
        .parse_text(text)
        .expect("Dark Supplicant-style activated search should parse");
    let debug = format!("{:#?}", def.abilities);

    assert!(debug.contains("WithIdEffect"), "{debug}");
    assert!(debug.contains("IfEffect"), "{debug}");
    assert!(debug.contains("SearchedLibrary"), "{debug}");
    assert!(debug.contains("ShuffleLibraryEffect"), "{debug}");
}

#[test]
pub(super) fn academy_researchers_puts_aura_from_hand_onto_battlefield_attached() {
    let text = "When this creature enters, you may put an Aura card from your hand onto the battlefield attached to this creature.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Academy Researchers")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("Academy Researchers should parse");
    let debug = format!("{:#?}", def.abilities);

    assert!(debug.contains("MoveToZoneEffect"), "{debug}");
    assert!(debug.contains("zone: Some("), "{debug}");
    assert!(debug.contains("Hand"), "{debug}");
    assert!(debug.contains("Aura"), "{debug}");
    assert!(debug.contains("AttachObjectsEffect"), "{debug}");
    assert!(debug.contains("this creature"), "{debug}");
}

#[test]
pub(super) fn cruel_reality_fallback_life_loss_targets_that_player() {
    let text = "Enchant player\nAt the beginning of enchanted player's upkeep, that player sacrifices a creature or planeswalker of their choice. If the player can't, they lose 5 life.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cruel Reality")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Curse])
        .parse_text(text)
        .expect("Cruel Reality should parse");
    let debug = format!("{:#?}", def.abilities);

    assert!(debug.contains("IfEffect"), "{debug}");
    assert!(debug.contains("DidNotHappen"), "{debug}");
    assert!(debug.contains("LoseLifeEffect"), "{debug}");
    assert!(
        debug.contains("player: IteratedPlayer")
            || (debug.contains("TaggedPlayer") && debug.contains("enchanted")),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_object_filter_parser_handles_same_name_as_the_spell_reference() {
    let tokens = lex_line("cards in all graveyards with the same name as the spell", 0)
        .expect("rewrite lexer should classify same-name spell filter text");

    let filter = super::super::parse_object_filter_lexed(&tokens, false)
        .expect("object filter parser should bind same-name spell references");
    let debug = format!("{filter:?}");

    assert!(
        debug.contains("zone: Some(Graveyard)") && debug.contains("SameNameAsTagged"),
        "expected graveyard same-name tagged filter, got {debug}"
    );
}

#[test]
pub(super) fn library_search_resolves_same_name_it_to_the_revealed_card_tag() {
    let text = "Reveal a creature card in your hand. Search your library for a card with the same name as that card, reveal it, put it into your hand, then shuffle.";
    let (compiled, loss) = crate::parse_loss::capture(|| {
        super::super::compile_card_text(
            CardDefinitionBuilder::new(CardId::from_raw(1), "Same Name Search Probe")
                .card_types(vec![CardType::Instant]),
            text,
            false,
        )
    });
    let compiled = compiled.expect("same-name search should compile");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());

    let effects = compiled
        .definition
        .spell_effect
        .as_ref()
        .expect("probe should compile a spell program")
        .flattened_default_effects();
    let revealed_tag = effects
        .iter()
        .find_map(|effect| {
            super::find_nested_effect::<crate::effects::RevealTaggedEffect>(effect)
                .map(|reveal| reveal.tag.clone())
        })
        .expect("reveal should preserve its concrete chosen-card tag");
    let search = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::SearchLibraryEffect>(effect))
        .expect("single-card library search should use SearchLibraryEffect");
    let same_name = search
        .filter
        .tagged_constraints
        .iter()
        .find(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
        })
        .expect("search filter should retain the same-name relation");

    assert_eq!(same_name.tag, revealed_tag);
    assert_ne!(same_name.tag.as_str(), crate::cards::builders::IT_TAG);
}

#[test]
pub(super) fn rewrite_search_library_object_filter_parser_handles_named_and_disjunction_shapes() {
    let named_filter = lex_line("artifact card named Sol Ring", 0)
        .expect("rewrite lexer should classify named search filter text");
    let parsed = super::super::grammar::effects::parse_search_library_object_filter_lexed(
        &named_filter,
        &render_token_slice(&named_filter),
    )
    .expect("search-library object-filter helper should parse named filter");

    assert_eq!(parsed.name.as_deref(), Some("sol ring"));

    let leading_article_name = lex_line("card named The Unspeakable", 0)
        .expect("rewrite lexer should classify leading-article card name text");
    let parsed = super::super::grammar::effects::parse_search_library_object_filter_lexed(
        &leading_article_name,
        &render_token_slice(&leading_article_name),
    )
    .expect("search-library object-filter helper should preserve card-name articles");

    assert_eq!(parsed.name.as_deref(), Some("the unspeakable"));

    let counted_named = lex_line("exactly two artifact cards named Sol Ring", 0)
        .expect("rewrite lexer should classify counted named search filter text");
    let parsed = super::super::grammar::effects::parse_search_library_object_filter_lexed(
        &counted_named,
        &render_token_slice(&counted_named),
    )
    .expect("search-library object-filter helper should strip count prefixes before named filters");

    assert_eq!(parsed.name.as_deref(), Some("sol ring"));
    assert_eq!(parsed.card_types, vec![CardType::Artifact]);

    let negated_named = lex_line("artifact card not named Sol Ring", 0)
        .expect("rewrite lexer should classify negated named search filter text");
    let parsed = super::super::grammar::effects::parse_search_library_object_filter_lexed(
        &negated_named,
        &render_token_slice(&negated_named),
    )
    .expect("search-library object-filter helper should parse negated named filter");

    assert_eq!(parsed.excluded_name.as_deref(), Some("sol ring"));
    assert_eq!(parsed.card_types, vec![CardType::Artifact]);

    let disjunction = lex_line("artifact or enchantment card", 0)
        .expect("rewrite lexer should classify disjunction search filter text");
    let parsed = super::super::grammar::effects::parse_search_library_object_filter_lexed(
        &disjunction,
        &render_token_slice(&disjunction),
    )
    .expect("search-library object-filter helper should parse disjunction filter");

    assert!(
        !parsed.any_of.is_empty(),
        "disjunction search filter should retain any_of branches"
    );

    let different_names = lex_line("cards with different names", 0)
        .expect("rewrite lexer should classify different-names search filter text");
    let parsed = super::super::grammar::effects::parse_search_library_object_filter_lexed(
        &different_names,
        &render_token_slice(&different_names),
    )
    .expect("search-library object-filter helper should parse bare different-names filters");

    assert!(
        parsed.distinct_names,
        "different-names search filter should be represented structurally"
    );
}

#[test]
pub(super) fn rewrite_grammar_mana_group_slash_marker_probe_matches_keyword_shape() {
    let tokens = lex_line("Prototype {3}{U} 2/2", 0)
        .expect("rewrite lexer should classify slash-marker keyword line");
    assert!(
        super::super::grammar::abilities::is_mana_group_slash_marker_line_lexed(&tokens),
        "mana-group slash marker probe should recognize slash-bearing keyword line"
    );
}

#[test]
pub(super) fn prototype_keyword_lowering_retains_typed_cast_characteristics() {
    let definition = CardDefinitionBuilder::new(CardId::from_raw(88_003), "Prototype Probe")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(crate::PowerToughness::fixed(6, 4))
        .parse_text("Prototype {2}{R} — 3/2")
        .expect("typed Prototype keyword should lower");

    let method = definition
        .alternative_casts
        .first()
        .expect("Prototype should create an alternative casting method");
    assert_eq!(
        method.mana_cost().map(|cost| cost.to_oracle()),
        Some("{2}{R}".to_string())
    );
    assert_eq!(
        method.prototype_power_toughness(),
        Some(crate::PowerToughness::fixed(3, 2)),
        "the grammar's typed P/T must survive lowering without display-text reparsing"
    );
}

#[test]
pub(super) fn rewrite_search_library_leading_prelude_and_top_probe_helpers_cover_remaining_shapes()
{
    fn subject_starts_effect(tokens: &[super::super::OwnedLexToken]) -> bool {
        super::super::effect_sentences::find_verb(tokens).is_some()
    }

    let leading_chain = lex_line(
        "Discard a card, then search your library for a creature card, reveal it, put it into your hand, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify leading-chain search text");
    let head_split =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&leading_chain)
            .expect("search head splitter should match leading-chain search");
    let prelude =
        super::super::grammar::effects::parse_search_library_leading_effect_prelude_lexed(
            head_split.subject_tokens,
            subject_starts_effect,
            super::super::effect_sentences::parse_effect_chain_with_subject_verb_primitives_lexed,
        )
        .expect("leading-prelude helper should parse the pre-search effect chain");

    assert!(prelude.subject_tokens.is_empty());
    assert!(
        !prelude.leading_effects.is_empty(),
        "leading-prelude helper should lift the leading effect chain"
    );

    let direct_subject = lex_line(
        "Target player may search their library for a card, then shuffle.",
        0,
    )
    .expect("rewrite lexer should classify direct-subject search text");
    let head_split =
        super::super::grammar::effects::split_search_library_sentence_head_lexed(&direct_subject)
            .expect("search head splitter should match direct-subject search");
    let prelude =
        super::super::grammar::effects::parse_search_library_leading_effect_prelude_lexed(
            head_split.subject_tokens,
            subject_starts_effect,
            super::super::effect_sentences::parse_effect_chain_with_subject_verb_primitives_lexed,
        )
        .expect("leading-prelude helper should leave plain subjects alone");

    assert_eq!(render_token_slice(prelude.subject_tokens), "Target player");
    assert!(prelude.leading_effects.is_empty());

    let unsupported_top = lex_line("Search your library for the third card from the top.", 0)
        .expect("rewrite lexer should classify nth-from-top search text");
    let unsupported_words = crate::runtime_backend::token_word_refs(&unsupported_top);
    assert!(
        super::super::grammar::effects::search_library_has_unsupported_top_position_probe(
            &unsupported_words
        ),
        "nth-from-top search text should stay rejected by the grammar-owned top-position probe"
    );

    let allowed_top = lex_line(
        "Search your library for a card and put that card on top of library.",
        0,
    )
    .expect("rewrite lexer should classify on-top-of-library search text");
    let allowed_words = crate::runtime_backend::token_word_refs(&allowed_top);
    assert!(
        !super::super::grammar::effects::search_library_has_unsupported_top_position_probe(
            &allowed_words
        ),
        "explicit on-top-of-library destination text should not trip the rejection probe"
    );

    let allowed_nth_put = lex_line(
        "Search your library for a card, then shuffle and put that card third from the top.",
        0,
    )
    .expect("rewrite lexer should classify searched-card nth-from-top placement text");
    let allowed_nth_words = crate::runtime_backend::token_word_refs(&allowed_nth_put);
    assert!(
        !super::super::grammar::effects::search_library_has_unsupported_top_position_probe(
            &allowed_nth_words
        ),
        "searched-card nth-from-top placement should be supported"
    );
}

#[test]
pub(super) fn rewrite_search_library_head_body_helpers_cover_wrap_and_search_verb_probes() {
    let wrap_subject =
        lex_line("each of them", 0).expect("rewrite lexer should classify wrap-subject text");
    assert!(
        super::super::grammar::effects::search_library_subject_wraps_each_target_player_lexed(
            &wrap_subject
        ),
        "`each of them` should trigger the wrap helper"
    );

    let plain_subject =
        lex_line("target player", 0).expect("rewrite lexer should classify plain subject text");
    assert!(
        !super::super::grammar::effects::search_library_subject_wraps_each_target_player_lexed(
            &plain_subject
        ),
        "plain subjects should not trigger the wrap helper"
    );

    let search_tokens = lex_line("search your library for a card", 0)
        .expect("rewrite lexer should classify search-verb text");
    assert!(
        super::super::grammar::effects::search_library_starts_with_search_verb_lexed(
            &search_tokens
        ),
        "search tokens should satisfy the search-verb sanity helper"
    );

    let non_search_tokens =
        lex_line("draw a card", 0).expect("rewrite lexer should classify non-search text");
    assert!(
        !super::super::grammar::effects::search_library_starts_with_search_verb_lexed(
            &non_search_tokens
        ),
        "non-search text should fail the search-verb sanity helper"
    );
}

#[test]
pub(super) fn rewrite_cant_sentence_negation_helpers_cover_supported_and_rejected_guards() {
    let supported = lex_line("Target artifact doesn't untap", 0)
        .expect("rewrite lexer should classify supported cant clause");
    let lowered =
        super::super::grammar::effects::cant_sentence_clause_tokens_for_restriction_scan_lexed(
            &supported,
        );
    assert_eq!(
        super::super::grammar::effects::find_cant_sentence_negation_span_lexed(&lowered),
        Some((2, 3))
    );
    assert_eq!(
        super::super::token_word_refs(&lowered),
        vec!["Target", "artifact", "doesn't", "untap"]
    );
    assert!(
        super::super::grammar::effects::cant_sentence_has_supported_negation_gate_lexed(&lowered),
        "plain cant clause should pass the negation gate"
    );

    let rejected = lex_line("Target artifact and target creature don't untap", 0)
        .expect("rewrite lexer should classify rejected cant clause");
    let lowered =
        super::super::grammar::effects::cant_sentence_clause_tokens_for_restriction_scan_lexed(
            &rejected,
        );
    assert!(
        !super::super::grammar::effects::cant_sentence_has_supported_negation_gate_lexed(&lowered),
        "clauses with an `and` before the negation span should stay rejected"
    );

    let split_negation = lex_line("Target artifact can not attack", 0)
        .expect("rewrite lexer should classify split-negation cant clause");
    assert_eq!(
        super::super::grammar::effects::find_cant_sentence_negation_span_lexed(&split_negation),
        Some((2, 4))
    );
}

#[test]
pub(super) fn rewrite_cant_sentence_next_turn_prefix_splitter_tracks_supported_suffixes() {
    let player_apostrophe = lex_line(
        "Each opponent can't cast instant or sorcery spells during that player's next turn.",
        0,
    )
    .expect("rewrite lexer should classify next-turn silence text");
    let prefix = super::super::grammar::effects::split_cant_sentence_next_turn_prefix_lexed(
        &player_apostrophe,
    )
    .expect("next-turn splitter should match apostrophe suffix");

    assert_eq!(
        super::super::token_word_refs(&prefix),
        vec![
            "Each", "opponent", "can't", "cast", "instant", "or", "sorcery", "spells",
        ]
    );

    let split_apostrophe = lex_line(
        "Each opponent can't cast instant or sorcery spells during that player s next turn.",
        0,
    )
    .expect("rewrite lexer should classify split-apostrophe next-turn silence text");
    assert!(
        super::super::grammar::effects::split_cant_sentence_next_turn_prefix_lexed(
            &split_apostrophe
        )
        .is_some(),
        "next-turn splitter should also match split-apostrophe suffixes"
    );

    let untap_step = lex_line(
        "Target artifact doesn't untap during its controller's next untap step.",
        0,
    )
    .expect("rewrite lexer should classify untap-step restriction text");
    assert!(
        super::super::grammar::effects::split_cant_sentence_next_turn_prefix_lexed(&untap_step)
            .is_none(),
        "non-next-turn restriction text should stay out of the next-turn prefix helper"
    );
}

#[test]
pub(super) fn rewrite_cant_sentence_clause_preparation_helper_tracks_supported_and_rejected_shapes()
{
    let untap_step = lex_line(
        "Target artifact doesn't untap during its controller's next untap step.",
        0,
    )
    .expect("rewrite lexer should classify supported untap-step restriction text");
    let prepared =
        super::super::grammar::effects::prepare_cant_sentence_restriction_clause_lexed(&untap_step)
            .expect("cant clause preparation helper should not error")
            .expect("cant clause preparation helper should keep supported untap-step text");

    assert_eq!(
        super::super::token_word_refs(&prepared.clause_tokens),
        vec!["Target", "artifact", "doesn't", "untap"]
    );

    let positive_clause = lex_line(
        "Target artifact untaps during its controller's next untap step.",
        0,
    )
    .expect("rewrite lexer should classify positive untap-step text");
    assert!(
        super::super::grammar::effects::prepare_cant_sentence_restriction_clause_lexed(
            &positive_clause
        )
        .expect("cant clause preparation helper should not error")
        .is_none(),
        "clauses without a negation span should stay out of the prepared cant-clause helper"
    );
}

#[test]
pub(super) fn rewrite_cant_sentence_source_tapped_duration_probe_tracks_supported_shapes() {
    let supported = lex_line(
        "Target creature can't attack for as long as this artifact remains tapped.",
        0,
    )
    .expect("rewrite lexer should classify source-tapped duration text");
    assert!(
        super::super::grammar::effects::cant_sentence_has_source_remains_tapped_duration(
            &supported
        ),
        "source-tapped duration helper should recognize supported for-as-long-as remains-tapped text"
    );

    let unsupported = lex_line("Target creature can't attack until end of turn.", 0)
        .expect("rewrite lexer should classify simple cant sentence");
    assert!(
        !super::super::grammar::effects::cant_sentence_has_source_remains_tapped_duration(
            &unsupported
        ),
        "non-source-tapped cant text should stay out of the remains-tapped helper"
    );
}

#[test]
pub(super) fn rewrite_lexed_cant_sentence_marks_source_tapped_duration_condition() {
    let text = "Target creature can't attack for as long as this artifact remains tapped.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify source-tapped cant text");

    let parsed = parse_cant_effect_sentence_lexed(&lexed)
        .expect("lexed source-tapped cant sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SourceIsTapped"), "{debug}");
}

#[test]
pub(super) fn rewrite_cant_sentence_preserves_distributive_compound_subject_filter() {
    let lexed = lex_line(
        "Each attacking creature and each blocking creature doesn't untap during its controller's next untap step.",
        0,
    )
    .expect("rewrite lexer should classify compound untap restriction");
    let parsed = parse_cant_effect_sentence_lexed(&lexed)
        .expect("compound untap restriction should parse")
        .expect("compound untap restriction should be recognized");

    let [EffectAst::SubjectVerb(effect)] = parsed.as_slice() else {
        panic!("expected one subject-verb restriction, got {parsed:#?}");
    };
    let SubjectVerbActionAst::Cant {
        restriction: crate::effect::Restriction::Untap(filter),
        ..
    } = &effect.action
    else {
        panic!("expected untap restriction, got {effect:#?}");
    };
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(filter.any_of.iter().any(|branch| branch.attacking));
    assert!(filter.any_of.iter().any(|branch| branch.blocking));
}

#[test]
pub(super) fn rewrite_effect_sentence_routes_search_library_family_through_grammar_entrypoint() {
    let text = "Search your library for a creature card with mana value 3 or less, reveal it, put it into your hand, then shuffle.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify search-library text");

    let grammar = super::super::effect_sentences::parse_search_library_sentence_lexed(&lexed)
        .expect("grammar-owned search-library sentence should parse")
        .unwrap_or_default();
    let sentence = parse_effect_sentence_lexed(&lexed).expect("effect sentence parser");

    assert_eq!(format!("{sentence:?}"), format!("{grammar:?}"));
}

#[test]
pub(super) fn rewrite_lexed_spell_filter_preserves_comparison_shapes() {
    for text in [
        "noncreature spells with mana value equal to 3",
        "creature spells with power or toughness 2 or less",
    ] {
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify comparison spell filter");
        let filter = super::super::parse_spell_filter_lexed(&tokens);
        let debug = format!("{filter:?}");

        if text.contains("mana value equal to 3") {
            assert!(debug.contains("excluded_card_types: [Creature]"), "{debug}");
            assert!(debug.contains("mana_value: Some(Equal(3))"), "{debug}");
        } else {
            assert!(debug.contains("any_of"), "{debug}");
            assert!(debug.contains("LessThanOrEqual(2)"), "{debug}");
        }
    }
}

#[test]
pub(super) fn rewrite_lexed_search_library_sentence_parses_shared_mana_value_constraint() {
    let text = "Search your library for a creature card with mana value 3 or less, reveal it, put it into your hand, then shuffle.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify search-library text");

    let parsed = super::super::parse_search_library_sentence_lexed(&lexed)
        .expect("lexed search-library sentence should parse")
        .expect("search-library sentence should produce effects");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("LessThanOrEqual(3)"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_search_library_sentence_parses_disjunction_filter_via_grammar_separator_helper()
 {
    let text = "Search your library for an artifact, enchantment, or creature card, reveal it, put it into your hand, then shuffle.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify search-library disjunction");

    let parsed = super::super::parse_search_library_sentence_lexed(&lexed)
        .expect("lexed search-library sentence should parse")
        .expect("search-library sentence should produce effects");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("any_of"), "{debug}");
    assert!(debug.contains("Artifact"), "{debug}");
    assert!(debug.contains("Enchantment"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
}

#[test]
pub(super) fn rewrite_gain_ability_keyword_lists_route_through_grammar_separator_helpers() {
    let text = "Target creature gains flying and vigilance until end of turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify gain-ability keyword list");

    let parsed = super::super::parse_effect_sentence_lexed(&lexed)
        .expect("gain-ability sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Flying"), "{debug}");
    assert!(debug.contains("Vigilance"), "{debug}");
}

#[test]
pub(super) fn rewrite_gain_ability_choice_list_routes_or_split_through_grammar_separator_helper() {
    let tokens = lex_line("your choice of flying, vigilance, or trample", 0)
        .expect("rewrite lexer should classify gain-ability choice list");

    let parsed = super::super::parse_choice_of_abilities(&tokens)
        .expect("choice-of-abilities helper should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Flying"), "{debug}");
    assert!(debug.contains("Vigilance"), "{debug}");
    assert!(debug.contains("Trample"), "{debug}");
}

#[test]
pub(super) fn rewrite_activation_line_routes_period_split_through_grammar_separator_helper() {
    let tokens = lex_line("{T}: Add {G}. Activate only during your turn.", 0)
        .expect("rewrite lexer should classify activated line with trailing restriction");

    let parsed = super::super::parse_activated_line(&tokens)
        .expect("activated line should parse")
        .expect("activated line should produce an ability");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("DuringYourTurn"), "{debug}");
}

#[test]
pub(super) fn rewrite_activation_line_collects_any_player_restriction_from_token_view() {
    let tokens = lex_line("{T}: Add {C}. Any player may activate this ability.", 0)
        .expect("rewrite lexer should classify activated line with any-player restriction");

    let parsed = super::super::parse_activated_line(&tokens)
        .expect("activated line should parse")
        .expect("activated line should produce an ability");

    match parsed.kind() {
        crate::ability::AbilityKind::Activated(activated) => {
            let restrictions = activated
                .additional_restrictions
                .iter()
                .map(|restriction| restriction.to_ascii_lowercase())
                .collect::<Vec<_>>();
            assert!(
                restrictions
                    .iter()
                    .any(|restriction| restriction == "any player may activate this ability"),
                "{restrictions:?}"
            );
        }
        other => panic!("expected activated ability, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_activation_line_collects_sentence_modifiers_via_activated_sentence_module() {
    let tokens = lex_line(
        "{T}: Add {C}. The next noncreature spell you cast this turn costs {2} less to cast. Spend this mana only to cast artifact spells of the chosen type and that spell can't be countered. Any player may activate this ability. Activate only once each turn.",
        0,
    )
    .expect("rewrite lexer should classify activated line with sentence modifiers");

    let parsed = super::super::parse_activated_line(&tokens)
        .expect("activated line should parse")
        .expect("activated line should produce an ability");
    let debug = format!("{parsed:#?}");

    match parsed.kind() {
        crate::ability::AbilityKind::Activated(activated) => {
            assert_eq!(
                activated.timing,
                crate::ability::ActivationTiming::OncePerTurn
            );
            assert!(matches!(
                activated.mana_usage_restrictions.as_slice(),
                [crate::ability::ManaUsageRestriction::CastSpell {
                    card_types,
                    subtype_requirement: Some(
                        crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource
                    ),
                    restrict_to_matching_spell: true,
                    grant_uncounterable: true,
                    enters_with_counters,
                    granted_abilities,
                }] if card_types == &vec![CardType::Artifact]
                    && enters_with_counters.is_empty()
                    && granted_abilities.is_empty()
            ));
            assert!(
                activated.additional_restrictions.iter().any(|restriction| {
                    restriction.eq_ignore_ascii_case("any player may activate this ability")
                }),
                "{:?}",
                activated.additional_restrictions
            );
        }
        other => panic!("expected activated ability, got {other:?}"),
    }

    assert!(debug.contains("ReduceNextSpellCostThisTurn"), "{debug}");
    assert!(debug.contains("excluded_card_types"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
}

#[test]
pub(super) fn rewrite_activation_line_parses_biophagus_style_conditional_mana_bonus() {
    let tokens = lex_line(
        "{T}: Add one mana of any color. If this mana is spent to cast a creature spell, that creature enters with an additional +1/+1 counter on it.",
        0,
    )
    .expect("rewrite lexer should classify Biophagus-style mana bonus");

    let parsed = super::super::parse_activated_line(&tokens)
        .expect("Biophagus-style line should parse")
        .expect("Biophagus-style line should produce an ability");

    match parsed.kind() {
        crate::ability::AbilityKind::Activated(activated) => {
            assert!(matches!(
                activated.mana_usage_restrictions.as_slice(),
                [crate::ability::ManaUsageRestriction::CastSpell {
                    card_types,
                    subtype_requirement: None,
                    restrict_to_matching_spell: false,
                    grant_uncounterable: false,
                    enters_with_counters,
                    granted_abilities,
                }] if card_types == &vec![CardType::Creature]
                    && enters_with_counters
                        == &vec![(crate::object::CounterType::PlusOnePlusOne, 1)]
                    && granted_abilities.is_empty()
            ));
        }
        other => panic!("expected activated ability, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_activation_line_parses_spent_on_spell_static_ability_bonus() {
    let tokens = lex_line(
        "{R}, {T}, Exert this land: Add {R}{R}. If that mana is spent on a creature spell, it gains haste until end of turn.",
        0,
    )
    .expect("rewrite lexer should classify Arena of Glory-style mana bonus");

    let parsed = super::super::parse_activated_line(&tokens)
        .expect("Arena of Glory-style line should parse")
        .expect("Arena of Glory-style line should produce an ability");

    match parsed.kind() {
        crate::ability::AbilityKind::Activated(activated) => {
            assert!(matches!(
                activated.mana_usage_restrictions.as_slice(),
                [crate::ability::ManaUsageRestriction::CastSpell {
                    card_types,
                    subtype_requirement: None,
                    restrict_to_matching_spell: false,
                    grant_uncounterable: false,
                    enters_with_counters,
                    granted_abilities,
                }] if card_types == &vec![CardType::Creature]
                    && enters_with_counters.is_empty()
                    && granted_abilities == &vec![crate::static_abilities::StaticAbilityId::Haste]
            ));
        }
        other => panic!("expected activated ability, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_keyword_static_combined_pregame_choose_color_routes_period_split_through_grammar_helper()
 {
    let tokens = lex_line(
        "choose a color before the game begins. this card is the chosen color.",
        0,
    )
    .expect("rewrite lexer should classify combined pregame choose-color line");

    let parsed = super::super::parse_combined_pregame_choose_color_line(&tokens)
        .expect("combined pregame choose-color line should parse")
        .expect("combined pregame choose-color line should produce abilities");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("ChooseColor") || debug.contains("chosen color"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_keyword_line_parses_simple_native_keyword_lists() {
    let keyword_tokens = lex_line("Flying and vigilance", 0)
        .expect("rewrite lexer should classify simple keyword line");
    let numeric_tokens =
        lex_line("Ward 2", 0).expect("rewrite lexer should classify numeric keyword line");

    assert!(matches!(
        super::super::clause_support::parse_ability_line_lexed(&keyword_tokens),
        Some(actions)
            if actions
                == vec![
                    crate::cards::builders::KeywordAction::Flying,
                    crate::cards::builders::KeywordAction::Vigilance,
                ]
    ));
    assert!(matches!(
        super::super::clause_support::parse_ability_line_lexed(&numeric_tokens),
        Some(actions)
            if actions
                == vec![crate::cards::builders::KeywordAction::Ward(2)]
    ));
}

#[test]
pub(super) fn rewrite_lexed_keyword_line_parses_protection_chains_without_duplicates() {
    let protection_tokens = lex_line("Protection from everything and from everything", 0)
        .expect("rewrite lexer should classify protection chain");

    assert!(matches!(
        super::super::clause_support::parse_ability_line_lexed(&protection_tokens),
        Some(actions)
            if actions
                == vec![crate::cards::builders::KeywordAction::ProtectionFromEverything]
    ));
}

#[test]
pub(super) fn rewrite_lexed_keyword_line_parses_mixed_protection_chain_targets() {
    let protection_tokens = lex_line("Protection from the chosen player and from all colors", 0)
        .expect("rewrite lexer should classify mixed protection chain");

    assert!(matches!(
        super::super::clause_support::parse_ability_line_lexed(&protection_tokens),
        Some(actions)
            if actions
                == vec![
                    crate::cards::builders::KeywordAction::ProtectionFromChosenPlayer,
                    crate::cards::builders::KeywordAction::ProtectionFromAllColors,
                ]
    ));
}

#[test]
pub(super) fn rewrite_lexed_keyword_line_parses_protection_from_permanents_with_named_counters() {
    let protection_tokens = lex_line(
        "Protection from permanents with corruption counters on them",
        0,
    )
    .expect("rewrite lexer should classify counter-filtered protection");

    let parsed = super::super::clause_support::parse_ability_line_lexed(&protection_tokens)
        .expect("counter-filtered protection should parse");
    assert_eq!(parsed.len(), 1, "{parsed:?}");

    let crate::cards::builders::KeywordAction::ProtectionFromFilter(filter) = &parsed[0] else {
        panic!("expected protection-from-filter action, got {parsed:?}");
    };
    let debug = format!("{filter:?}");
    assert!(debug.contains("with_counter: Some"), "{debug}");
    assert!(debug.to_ascii_lowercase().contains("corruption"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_keyword_line_parses_protection_from_mana_value_comparison() {
    let protection_tokens = lex_line("Protection from mana value 3 or greater", 0)
        .expect("rewrite lexer should classify mana-value protection");

    let parsed = super::super::clause_support::parse_ability_line_lexed(&protection_tokens)
        .expect("mana-value protection should parse");
    assert_eq!(parsed.len(), 1, "{parsed:?}");

    let crate::cards::builders::KeywordAction::ProtectionFromFilter(filter) = &parsed[0] else {
        panic!("expected protection-from-filter action, got {parsed:?}");
    };
    assert_eq!(
        filter.mana_value,
        Some(crate::filter::Comparison::GreaterThanOrEqual(3))
    );
}

#[test]
pub(super) fn rewrite_lexed_keyword_line_routes_separator_lists_through_grammar_primitives() {
    let keyword_tokens = lex_line("Flying, vigilance; trample and haste", 0)
        .expect("rewrite lexer should classify mixed keyword separator line");

    assert!(matches!(
        super::super::clause_support::parse_ability_line_lexed(&keyword_tokens),
        Some(actions)
            if actions
                == vec![
                    crate::cards::builders::KeywordAction::Flying,
                    crate::cards::builders::KeywordAction::Vigilance,
                    crate::cards::builders::KeywordAction::Trample,
                    crate::cards::builders::KeywordAction::Haste,
                ]
    ));
}

#[test]
pub(super) fn rewrite_lexed_keyword_line_parses_hexproof_from_color_in_keyword_list() {
    let keyword_tokens = lex_line("Reach, hexproof from blue", 0)
        .expect("rewrite lexer should classify hexproof-from keyword line");

    let parsed = super::super::clause_support::parse_ability_line_lexed(&keyword_tokens)
        .expect("hexproof-from keyword line should parse");
    assert_eq!(parsed.len(), 2, "{parsed:?}");
    assert_eq!(parsed[0], crate::cards::builders::KeywordAction::Reach);

    let crate::cards::builders::KeywordAction::HexproofFrom(filter) = &parsed[1] else {
        panic!("expected hexproof-from action, got {parsed:?}");
    };
    assert_eq!(filter.colors, Some(crate::color::ColorSet::BLUE));
}

#[test]
pub(super) fn rewrite_lexed_triggered_and_static_entrypoints_work_natively() {
    let triggered_tokens = lex_line(
        "Whenever you cast an Aura, Equipment, or Vehicle spell, draw a card.",
        0,
    )
    .expect("rewrite lexer should classify triggered probe");
    let static_tokens = lex_line(
        "Activated abilities of artifacts and creatures can't be activated.",
        0,
    )
    .expect("rewrite lexer should classify static probe");

    assert!(matches!(
        super::super::clause_support::parse_triggered_line_lexed(&triggered_tokens),
        Ok(crate::cards::builders::LineAst::Triggered { .. })
    ));
    assert!(matches!(
        super::super::clause_support::parse_static_ability_ast_line_lexed(&static_tokens),
        Ok(Some(abilities)) if !abilities.is_empty()
    ));
    assert_eq!(
        format!(
            "{:?}",
            super::super::keyword_static::parse_static_ability_ast_line_lexed(&static_tokens)
                .expect("static entrypoint should parse")
        ),
        format!(
            "{:?}",
            super::super::clause_support::parse_static_ability_ast_line_lexed(&static_tokens)
                .expect("lexed static entrypoint should parse")
        )
    );
}

#[test]
pub(super) fn rewrite_lexed_trigger_parses_possessive_keyword_ability_activation() {
    let tokens = lex_line(
        "Whenever you activate this creature's outlast ability, create a 1/1 white Warrior creature token.",
        0,
    )
    .expect("rewrite lexer should classify possessive keyword-ability trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("trigger should parse through ability-activated shape");

    let debug = format!("{parsed:?}").to_ascii_lowercase();
    assert!(debug.contains("abilityactivated"), "{debug}");
    assert!(debug.contains("outlast"), "{debug}");
}

#[test]
pub(super) fn rewrite_grammar_activated_abilities_cant_be_activated_splitter_matches_keyword_static_shape()
 {
    let tokens = lex_line(
        "Activated abilities of artifacts and creatures can't be activated unless they're mana abilities.",
        0,
    )
    .expect("rewrite lexer should classify activated-abilities restriction");

    let spec =
        super::super::grammar::abilities::parse_activated_abilities_cant_be_activated_spec_lexed(
            &tokens,
        )
        .expect("grammar-owned activated-abilities restriction splitter should match");

    assert_eq!(
        crate::runtime_backend::token_word_refs(spec.subject_tokens),
        vec!["artifacts", "and", "creatures"],
    );
    assert!(
        spec.non_mana_only,
        "splitter should preserve the unless-theyre-mana-abilities flag"
    );

    let parsed =
        super::super::keyword_static::parse_activated_abilities_cant_be_activated_line_lexed(
            &tokens,
        )
        .expect("activated-abilities restriction should parse");
    assert!(parsed.is_some(), "{parsed:?}");
}

#[test]
pub(super) fn rewrite_grammar_trigger_suppression_splitter_matches_keyword_static_shape() {
    let tokens = lex_line(
        "Creatures entering the battlefield don't cause abilities of artifacts to trigger.",
        0,
    )
    .expect("rewrite lexer should classify trigger-suppression line");

    let spec = super::super::grammar::abilities::parse_trigger_suppression_spec_lexed(&tokens)
        .expect("grammar-owned trigger-suppression splitter should match");

    assert_eq!(
        crate::runtime_backend::token_word_refs(spec.cause_tokens),
        vec!["Creatures", "entering", "the", "battlefield"],
    );
    assert_eq!(
        spec.source_filter_tokens
            .map(crate::runtime_backend::token_word_refs),
        Some(vec!["artifacts"]),
    );

    let parsed = super::super::keyword_static::parse_trigger_suppression_line_ast(&tokens)
        .expect("trigger-suppression line should parse");
    assert!(matches!(
        parsed,
        Some(crate::cards::builders::StaticAbilityAst::Static(ability))
            if ability.id()
                == crate::static_abilities::StaticAbilityId::SuppressMatchingTriggeredAbilities
    ));
}

#[test]
pub(super) fn rewrite_keyword_static_marker_line_normalizes_doctors_companion_apostrophe() {
    let tokens = lex_line("Doctor's companion", 0)
        .expect("rewrite lexer should classify doctor's companion marker line");

    let ability = super::super::keyword_static::parse_static_text_marker_line(&tokens)
        .expect("doctor's companion marker line should parse");

    assert_eq!(
        ability.id(),
        crate::static_abilities::StaticAbilityId::DoctorsCompanion
    );
}

#[test]
pub(super) fn rewrite_grammar_protection_and_ward_probes_match_static_shapes() {
    let protection_tokens = lex_line("Protection from odd mana values.", 0)
        .expect("rewrite lexer should classify protection marker line");

    assert!(
        super::super::grammar::abilities::is_protection_mana_value_marker_line_lexed(
            &protection_tokens
        ),
        "grammar-owned protection marker probe should match"
    );

    let protection =
        super::super::keyword_static::parse_static_text_marker_line(&protection_tokens)
            .expect("protection marker line should parse");
    let protection_debug = format!("{protection:?}");
    assert!(
        protection_debug.contains("Protection from odd mana values"),
        "{protection_debug}"
    );

    let ward_tokens =
        lex_line("Ward pay 3 life.", 0).expect("rewrite lexer should classify ward marker line");

    assert_eq!(
        super::super::grammar::abilities::parse_ward_pay_life_amount_lexed(&ward_tokens),
        Some(3)
    );

    let ward = super::super::keyword_static::parse_static_text_marker_line(&ward_tokens)
        .expect("ward pay-life line should parse");
    let debug = format!("{ward:?}");
    assert_eq!(ward.id(), StaticAbilityId::Ward);
    assert!(debug.contains("Life(Fixed(3))"), "{debug}");

    let mana_ward_tokens =
        lex_line("Ward {8}.", 0).expect("rewrite lexer should classify mana ward line");
    let mana_ward = super::super::keyword_static::parse_static_text_marker_line(&mana_ward_tokens)
        .expect("mana ward line should parse");
    let mana_debug = format!("{mana_ward:?}");
    assert!(mana_debug.contains("Mana("), "{mana_debug}");
    assert!(!mana_debug.contains("ManaPaymentCost"), "{mana_debug}");
}

#[test]
pub(super) fn rewrite_grammar_remaining_exact_marker_probes_match_static_shapes() {
    let odd_flash_tokens = lex_line("As long as this creature has odd power, it has flash.", 0)
        .expect("rewrite lexer should classify odd-power flash marker line");
    assert!(
        super::super::grammar::abilities::is_as_long_as_power_odd_or_even_flash_marker_line_lexed(
            &odd_flash_tokens
        ),
        "grammar-owned odd/even flash marker probe should match"
    );
    assert!(
        super::super::keyword_static::parse_static_text_marker_line(&odd_flash_tokens).is_some(),
        "odd/even flash marker line should parse"
    );

    let haste_tokens = lex_line(
        "This creature can attack as though it had haste unless it entered this turn.",
        0,
    )
    .expect("rewrite lexer should classify haste-unless-entered marker line");
    assert!(
        super::super::grammar::abilities::is_attack_as_haste_unless_entered_this_turn_marker_line_lexed(
            &haste_tokens
        ),
        "grammar-owned haste-unless-entered marker probe should match"
    );
    assert!(
        super::super::keyword_static::parse_static_text_marker_line(&haste_tokens).is_some(),
        "haste-unless-entered marker line should parse"
    );

    let sab_tokens = lex_line(
        "Sab-Sunen can't attack or block unless there are seven or more lands among cards in your graveyard.",
        0,
    )
    .expect("rewrite lexer should classify Sab-Sunen marker line");
    assert!(
        super::super::grammar::abilities::is_sab_sunen_cant_attack_or_block_unless_line_lexed(
            &sab_tokens
        ),
        "grammar-owned Sab-Sunen marker probe should match"
    );
    assert!(
        super::super::keyword_static::parse_static_text_marker_line(&sab_tokens).is_some(),
        "Sab-Sunen marker line should parse"
    );
}

#[test]
pub(super) fn rewrite_keyword_static_doesnt_untap_line_normalizes_contraction() {
    let tokens = lex_line("This creature doesn't untap during your untap step", 0)
        .expect("rewrite lexer should classify doesn't untap line");

    assert!(matches!(
        super::super::grammar::abilities::parse_doesnt_untap_during_untap_step_spec_lexed(&tokens),
        Some(super::super::grammar::abilities::DoesntUntapDuringUntapStepSpec::Source { .. })
    ));

    let parsed = super::super::keyword_static::parse_doesnt_untap_during_untap_step_line(&tokens)
        .expect("doesn't untap line should parse");

    assert!(matches!(
        parsed,
        Some(crate::cards::builders::StaticAbilityAst::Static(ref ability))
            if ability.id() == crate::static_abilities::StaticAbilityId::DoesntUntap
    ));
}

#[test]
pub(super) fn rewrite_grammar_doesnt_untap_line_matches_attached_subject_shape() {
    let tokens = lex_line(
        "Enchanted creature doesn't untap during its controller's untap step.",
        0,
    )
    .expect("rewrite lexer should classify attached doesnt-untap line");

    assert!(matches!(
        super::super::grammar::abilities::parse_doesnt_untap_during_untap_step_spec_lexed(&tokens),
        Some(super::super::grammar::abilities::DoesntUntapDuringUntapStepSpec::Attached { .. })
    ));

    let parsed = super::super::keyword_static::parse_doesnt_untap_during_untap_step_line(&tokens)
        .expect("attached doesnt-untap line should parse");

    assert!(matches!(
        parsed,
        Some(crate::cards::builders::StaticAbilityAst::AttachedStaticAbilityGrant { .. })
    ));
}

#[test]
pub(super) fn rewrite_keyword_static_reveal_first_card_probe_uses_parser_text_words() {
    let tokens = lex_line(
        "You may reveal the first card you draw on each of your turns as you draw it.",
        0,
    )
    .expect("rewrite lexer should classify reveal-first-card static line");

    let spec =
        super::super::grammar::abilities::parse_reveal_first_card_you_draw_each_turn_spec_lexed(
            &tokens,
        )
        .expect("grammar-owned reveal-first-card probe should match");
    assert!(
        spec.optional,
        "grammar probe should preserve optional prefix"
    );
    assert!(
        spec.your_turns_only,
        "grammar probe should preserve the on-each-of-your-turns variant"
    );

    let parsed = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("reveal-first-card static line should parse")
        .expect("reveal-first-card static line should produce abilities");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::StaticAbilityAst::Static(ability)]
            if ability.id() == crate::static_abilities::StaticAbilityId::RevealFirstCardYouDrawEachTurn
    ));
}
