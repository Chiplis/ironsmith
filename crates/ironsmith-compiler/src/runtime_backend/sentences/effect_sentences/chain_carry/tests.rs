use crate::ability::AbilityKind;
use crate::cards::builders::{
    CardDefinitionBuilder, EffectAst, PlayerAst, SubjectVerbActionAst, SubjectVerbEffectAst,
};
use crate::effect::Value;
use crate::ids::CardId;
use crate::types::CardType;
use crate::zone::Zone;

use super::super::super::lexer::lex_line;
use super::{
    parse_effect_chain_lexed, parse_effect_clause_with_trailing_if_lexed,
    parse_effect_sentence_lexed, parse_leading_player_may_lexed, starts_like_create_fragment_lexed,
};

#[test]
fn absolving_lammasu_one_clause_actions_keep_coordinated_surface() {
    let tokens = lex_line(
        "You gain 3 life and suspect up to one target creature an opponent controls.",
        0,
    )
    .expect("Absolving Lammasu effect should lex");

    let effects =
        parse_effect_sentence_lexed(&tokens).expect("Absolving Lammasu effect should parse");
    let [
        EffectAst::Coordinated {
            effects: coordinated,
            leading_duration: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one coordinated Lammasu clause, got {effects:#?}");
    };
    assert_eq!(coordinated.len(), 2, "{coordinated:#?}");
    assert!(
        coordinated.iter().any(|effect| matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GainLife { .. },
                ..
            })
        )),
        "{coordinated:#?}"
    );
    assert!(
        coordinated.iter().any(|effect| matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Suspect { .. },
                ..
            })
        )),
        "{coordinated:#?}"
    );
}

#[test]
fn aatchik_separate_sentence_actions_do_not_become_coordinated() {
    let tokens = lex_line(
        "Put a +1/+1 counter on this. Each opponent loses 1 life.",
        0,
    )
    .expect("Aatchik effect sentences should lex");

    let effects = super::super::parse_effect_sentences_lexed(&tokens)
        .expect("Aatchik effect sentences should parse");
    assert_eq!(effects.len(), 2, "{effects:#?}");
    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::Coordinated { .. })),
        "separate Oracle sentences must remain ordinary siblings: {effects:#?}"
    );
}

#[test]
fn peregrination_search_partition_stays_a_specialist_bundle() {
    let tokens = lex_line(
        "Search your library for up to two basic land cards, reveal those cards, and put one onto the battlefield tapped and the other into your hand.",
        0,
    )
    .expect("Peregrination search sentence should lex");

    let effects =
        parse_effect_sentence_lexed(&tokens).expect("Peregrination search sentence should parse");
    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::Coordinated { .. })),
        "search partition program must not become display coordination: {effects:#?}"
    );
    let debug = format!("{effects:#?}");
    assert!(debug.contains("ChooseObjectsAcrossZones"), "{debug}");
    assert!(debug.contains("PutTaggedRemainderInZone"), "{debug}");
}

#[test]
fn extortion_hand_choice_stays_a_specialist_bundle() {
    let tokens = lex_line(
        "Look at target player's hand and choose up to two cards from it.",
        0,
    )
    .expect("Extortion hand-choice sentence should lex");

    let effects =
        parse_effect_sentence_lexed(&tokens).expect("Extortion hand-choice sentence should parse");
    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::Coordinated { .. })),
        "look/choose program must remain available to its discard follow-up: {effects:#?}"
    );
    assert!(format!("{effects:#?}").contains("ChooseObjects"));
}

#[test]
fn vraskas_fall_choice_and_consequences_do_not_become_coordinated() {
    let tokens = lex_line(
        "Each opponent sacrifices a creature or planeswalker of their choice and gets a poison counter.",
        0,
    )
    .expect("Vraska's Fall sentence should lex");

    let effects =
        parse_effect_sentence_lexed(&tokens).expect("Vraska's Fall sentence should parse");
    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::Coordinated { .. })),
        "choice and its consequences must remain on the specialist path: {effects:#?}"
    );
}

#[test]
fn malboro_three_action_opponent_chain_does_not_become_coordinated() {
    let tokens = lex_line(
        "Each opponent discards a card, loses 2 life, and exiles the top three cards of their library.",
        0,
    )
    .expect("Malboro sentence should lex");

    let effects = parse_effect_sentence_lexed(&tokens).expect("Malboro sentence should parse");
    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::Coordinated { .. })),
        "multi-action opponent chains must retain their existing specialist rendering: {effects:#?}"
    );
}

#[test]
fn triggered_lowering_keeps_sentences_separate_and_one_clause_coordinated() {
    let aatchik = CardDefinitionBuilder::new(CardId::from_raw(1), "Aatchik Boundary Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever another Insect you control dies, put a +1/+1 counter on this creature. Each opponent loses 1 life.",
        )
        .expect("Aatchik-style trigger should lower");
    let aatchik_triggered = aatchik
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Aatchik-style fixture should produce a triggered ability");
    assert_eq!(
        aatchik_triggered.effects.segments.len(),
        2,
        "separate Oracle sentences must lower as separate resolution segments: {aatchik_triggered:#?}"
    );

    let lammasu = CardDefinitionBuilder::new(CardId::from_raw(2), "Lammasu Boundary Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature dies, you gain 3 life and suspect up to one target creature an opponent controls.",
        )
        .expect("Lammasu-style trigger should lower");
    let lammasu_triggered = lammasu
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Lammasu-style fixture should produce a triggered ability");
    assert_eq!(
        lammasu_triggered.effects.segments.len(),
        1,
        "one coordinated Oracle clause must stay in one resolution segment: {lammasu_triggered:#?}"
    );
    let [coordinated] = lammasu_triggered.effects.flattened_default_effects() else {
        panic!("expected one typed coordinated effect: {lammasu_triggered:#?}");
    };
    let sequence = coordinated
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("Lammasu actions should retain a typed sequence");
    assert_eq!(
        sequence.surface,
        ironsmith_core::SequenceSurface::Coordinated,
        "Lammasu's single-clause conjunction must not become a sentence break"
    );
}

#[test]
fn leading_may_land_play_permission_does_not_lower_to_may_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore")
        .parse_text("You may play an additional land this turn.\nDraw a card.")
        .expect("explore-style text should parse");

    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        super::string_contains(&spell_debug, "AdditionalLandPlaysEffect")
            || super::string_contains(&spell_debug, "additional_land_plays"),
        "expected Explore-style permission text to lower to additional land plays, got {spell_debug}"
    );
}

#[test]
fn create_fragment_probe_accepts_capitalized_pt_token_clauses() {
    let tokens = lex_line("Two 1/1 white Soldier creature tokens", 0)
        .expect("rewrite lexer should classify create-fragment text");

    assert!(starts_like_create_fragment_lexed(&tokens));
}

#[test]
fn implicit_draw_then_discard_keeps_discard_on_ability_controller() {
    let tokens = lex_line("Draw an additional card, then discard a card.", 0)
        .expect("draw-discard fixture should lex");
    let effects = parse_effect_chain_lexed(&tokens).expect("draw-discard fixture should parse");

    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { .. },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::Discard { .. },
        }),
    ] = effects.as_slice()
    else {
        panic!("expected adjacent draw and discard effects, got {effects:#?}");
    };
    assert_eq!(subject.player, PlayerAst::You);
}

#[test]
fn source_damage_then_keyword_grant_keeps_coordinated_surface() {
    let tokens = lex_line(
        "This creature deals 2 damage to target player and gains indestructible until end of turn.",
        0,
    )
    .expect("source damage-and-grant fixture should lex");
    let effects =
        parse_effect_chain_lexed(&tokens).expect("source damage-and-grant fixture should parse");

    let [
        EffectAst::Coordinated {
            effects: coordinated,
            leading_duration: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected coordinated source damage-and-grant clause, got {effects:#?}");
    };
    let debug = format!("{coordinated:#?}");
    assert!(debug.contains("DealDamageEqualToPower"), "{debug}");
    assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
    assert!(debug.contains("Indestructible"), "{debug}");
}

#[test]
fn tap_then_next_untap_conjunction_keeps_coordinated_surface() {
    let tokens = lex_line(
        "Tap target creature and it doesn't untap during its controller's next untap step.",
        0,
    )
    .expect("freeze conjunction fixture should lex");
    let effects =
        parse_effect_chain_lexed(&tokens).expect("freeze conjunction fixture should parse");

    let [
        EffectAst::Coordinated {
            effects: coordinated,
            leading_duration: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected coordinated tap/freeze clause, got {effects:#?}");
    };
    assert_eq!(coordinated.len(), 2, "{coordinated:#?}");
    assert!(
        matches!(
            coordinated.first(),
            Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Tap { .. },
                ..
            }))
        ),
        "{coordinated:#?}"
    );
    assert!(
        matches!(
            coordinated.get(1),
            Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Cant {
                    restriction: crate::effect::Restriction::Untap(_),
                    duration: crate::effect::Until::ControllersNextUntapStep,
                    condition: None,
                },
                ..
            }))
        ),
        "{coordinated:#?}"
    );
}

#[test]
fn create_fragment_probe_accepts_named_token_appositive_clauses() {
    let tokens = lex_line(
        "a legendary 2/1 black Skeleton creature token with \"Jumblebones can't block\"",
        0,
    )
    .expect("rewrite lexer should classify named-token appositive text");

    assert!(starts_like_create_fragment_lexed(&tokens));
}

#[test]
fn parses_named_token_appositive_with_quoted_trigger_rules() {
    let tokens = lex_line(
        "Create Jumblebones, a legendary 2/1 black Skeleton creature token with \"Jumblebones can't block\" and \"When Jumblebones leaves the battlefield, return target card named Ozox, the Clattering King from your graveyard to your hand.\"",
        0,
    )
    .expect("named-token appositive should lex");

    parse_effect_chain_lexed(&tokens)
        .expect("named-token appositive with nested token trigger should parse");
}

#[test]
fn parses_target_card_type_list_with_lte_mana_value_reference() {
    let tokens = lex_line(
        "Exile target enchantment, instant, or sorcery card with equal or lesser mana value than that spell from an opponent's graveyard",
        0,
    )
    .expect("target list clause should lex");

    parse_effect_chain_lexed(&tokens).expect("target list clause should parse");
}

#[test]
fn coordinated_tap_set_stays_one_antecedent_for_then_them() {
    let tokens = lex_line(
        "Tap this creature and all creatures named Kobolds of Kher Keep, then an opponent gains control of them.",
        0,
    )
    .expect("coordinated tap chain should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("coordinated tap chain should parse");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TapAll { filter },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GainControl { .. },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected tap-union then gain-control effects, got {effects:#?}");
    };
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(filter.any_of[0].source, "{filter:#?}");
    assert_eq!(
        filter.any_of[1].name.as_deref(),
        Some("kobolds of kher keep")
    );
}

#[test]
fn discard_up_to_two_then_draw_binds_the_actual_discard_outcome() {
    let tokens = lex_line("Discard up to two cards, then draw that many cards.", 0)
        .expect("discard/draw chain should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("discard/draw chain should parse");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Discard {
                    count: discard_count,
                    any_number,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { count: draw_count },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected adjacent discard and draw effects, got {effects:#?}");
    };

    assert_eq!(discard_count, &Value::Fixed(2));
    assert!(*any_number, "up to two must allow choosing fewer than two");
    assert!(matches!(
        draw_count,
        Value::PendingEffectMetric {
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::Count,
        }
    ));
}

#[test]
fn gain_toughness_lose_power_then_put_keeps_all_three_actions() {
    let tokens = lex_line(
        "You gain life equal to that card's toughness, lose life equal to its power, then put it into your hand.",
        0,
    )
    .expect("life-stat chain should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("life-stat chain should parse");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GainLife {
                    amount: Value::ToughnessOf(_),
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::LoseLife {
                    amount: Value::PowerOf(_),
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::MoveToZone {
                    zone: Zone::Hand, ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected gain-toughness, lose-power, then put-into-hand, got {effects:#?}");
    };

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::GainLife {
            amount: gain_amount,
        },
        ..
    }) = &effects[0]
    else {
        unreachable!();
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LoseLife {
            amount: lose_amount,
        },
        ..
    }) = &effects[1]
    else {
        unreachable!();
    };
    let Value::ToughnessOf(gain_spec) = gain_amount.unhinted() else {
        unreachable!();
    };
    let Value::PowerOf(lose_spec) = lose_amount.unhinted() else {
        unreachable!();
    };
    assert_eq!(gain_spec.unhinted(), lose_spec.unhinted());
    assert!(matches!(
        lose_spec.unhinted(),
        crate::target::ChooseSpec::Tagged(tag) if tag.as_str() == crate::cards::builders::IT_TAG
    ));
}

#[test]
fn conditional_reveal_moves_preserve_explicit_contextual_destinations() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Reveal Destination Variant")
        .parse_text(
            "Reveal the top card of your library. If it's a creature card, put it onto the battlefield. Otherwise, put it into your graveyard.",
        )
        .expect("conditional reveal destination should parse");
    let debug = format!("{:#?}", def.spell_effect);

    assert!(debug.contains("zone: Graveyard"), "{debug}");
    assert!(
        debug.contains("destination_player_surface: Some(\n") && debug.contains("You"),
        "explicit your-graveyard surface was lost: {debug}"
    );
}

#[test]
fn return_to_hand_preserves_explicit_contextual_destination() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(2), "Return Destination Variant")
        .parse_text("Return target permanent card from your graveyard to your hand.")
        .expect("contextual return destination should parse");
    let debug = format!("{:#?}", def.spell_effect);

    assert!(debug.contains("ReturnFromGraveyardToHandEffect"), "{debug}");
    assert!(
        debug.contains("destination_player_surface: Some(") && debug.contains("You"),
        "explicit your-hand surface was lost: {debug}"
    );
}

#[test]
fn source_card_return_preserves_identity_and_explicit_graveyard() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(3), "Chandra's Phoenix")
        .parse_text("Return this card from your graveyard to your hand.")
        .expect("source-card return should parse");
    let debug = format!("{:#?}", def.spell_effect);

    assert!(debug.contains("ReturnFromGraveyardToHandEffect"), "{debug}");
    assert!(
        debug.contains("zone: Some(\n") && debug.contains("Graveyard"),
        "{debug}"
    );
    assert!(
        debug.contains("owner: Some(\n") && debug.contains("You"),
        "{debug}"
    );
    assert!(debug.contains("source: true"), "{debug}");
    assert!(debug.contains("this card"), "{debug}");
    assert!(debug.contains("graveyard_player_surface: Some("), "{debug}");
    assert!(
        debug.contains("destination_player_surface: Some("),
        "{debug}"
    );
}

#[test]
fn chain_entrypoint_accepts_nonverb_additional_phase_clause() {
    let tokens = lex_line("There's an additional combat phase after this phase.", 0)
        .expect("additional phase clause should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("additional phase should parse");
    assert!(
        matches!(
            effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::AdditionalPhases { .. },
                ..
            })]
        ),
        "{effects:#?}"
    );
}

#[test]
fn copy_then_gain_clause_keeps_the_explicit_gain_duration() {
    let tokens = lex_line(
        "Each land you control of that type becomes a copy of target creature you control until end of turn and gains haste until end of turn.",
        0,
    )
    .expect("copy-and-gain clause should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("copy-and-gain clause should parse");
    let gain = effects
        .iter()
        .find_map(|effect| match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GrantAbilitiesAll { duration, .. },
                ..
            }) => Some(duration),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected an all-lands haste grant, got {effects:#?}"));
    assert_eq!(*gain, crate::effect::Until::EndOfTurn, "{effects:#?}");
}

#[test]
fn trailing_if_keeps_relative_target_spell_controller_predicate() {
    let tokens = lex_line(
        "Counter target spell if you control more creatures than that spell's controller.",
        0,
    )
    .expect("relative counter condition should lex");

    let effect = parse_effect_clause_with_trailing_if_lexed(&tokens)
        .expect("relative counter condition should parse");
    assert!(
        matches!(
            effect,
            EffectAst::Conditional {
                predicate: crate::cards::builders::PredicateAst::YouControlMoreCreaturesThanTargetSpellController,
                ..
            }
        ),
        "{effect:#?}"
    );
}

#[test]
fn source_linked_exile_reveal_keeps_nonpermanents_face_up_and_moves_only_permanents() {
    let tokens = lex_line(
        "Each player turns face up all cards they own exiled with this artifact, then puts all permanent cards among them onto the battlefield.",
        0,
    )
    .expect("source-linked exile sequence should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("sequence should parse");
    let sentence_effects =
        parse_effect_sentence_lexed(&tokens).expect("sentence entrypoint should parse");
    assert_eq!(sentence_effects, effects);
    let [EffectAst::ForEachPlayer { effects: nested }] = effects.as_slice() else {
        panic!("expected per-player source-linked sequence, got {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TurnFaceUp { target },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ReturnAllToBattlefield { filter, .. },
            ..
        }),
    ] = nested.as_slice()
    else {
        panic!("expected reveal then permanent-return effects, got {nested:#?}");
    };
    let crate::cards::builders::TargetAst::Object(reveal_filter, None, None) = target else {
        panic!("expected non-target reveal filter, got {target:#?}");
    };
    for candidate in [reveal_filter, filter] {
        assert_eq!(candidate.zone, Some(Zone::Exile));
        assert_eq!(
            candidate.owner,
            Some(crate::target::PlayerFilter::IteratedPlayer)
        );
        assert!(
            candidate
                .tagged_constraints
                .iter()
                .any(|constraint| { constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG })
        );
    }
    assert!(reveal_filter.card_types.is_empty(), "{reveal_filter:#?}");
    assert_eq!(filter.card_types.len(), 6, "{filter:#?}");
}

#[test]
fn leading_player_may_probe_accepts_capitalized_opponent_clauses() {
    let tokens = lex_line("An opponent may cast it", 0)
        .expect("rewrite lexer should classify player-may text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::Opponent)
    );
}

#[test]
fn leading_player_may_probe_accepts_then_target_player_clauses() {
    let tokens = lex_line("Then target player may draw a card", 0)
        .expect("rewrite lexer should classify target-player may text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::Target)
    );
}

#[test]
fn leading_player_may_probe_accepts_possessive_controller_clauses() {
    let tokens = lex_line("That creature's controller may cast it", 0)
        .expect("rewrite lexer should classify possessive controller text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::ItsController)
    );
}

#[test]
fn leading_player_may_probe_accepts_that_attacking_player_clauses() {
    let tokens = lex_line("That attacking player may create a tapped Zombie token", 0)
        .expect("rewrite lexer should classify attacking-player may text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::Attacking)
    );
}

#[test]
fn leading_player_may_probe_accepts_that_player_or_target_controller_clauses() {
    let tokens = lex_line(
        "That player or that permanent's controller may draw a card",
        0,
    )
    .expect("rewrite lexer should classify split controller text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::ThatPlayerOrTargetController)
    );
}

#[test]
fn top_cards_then_put_counted_into_hand_rest_graveyard_chain_parses() {
    let tokens = lex_line(
        "Look at the top three cards of your library, then put one of them into your hand and the rest into your graveyard",
        0,
    )
    .expect("looked-cards split clause should lex");

    let effects =
        parse_effect_chain_lexed(&tokens).expect("looked-cards split clause should parse");

    match effects.as_slice() {
        [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::LookAtTopCards { .. },
                ..
            }),
            EffectAst::SnapshotLastObjectTag { .. },
            EffectAst::ChooseTaggedObjectsInZone {
                player,
                count,
                zone: Zone::Library,
                ..
            },
            EffectAst::MoveTaggedGroupToZone {
                zone: Zone::Hand, ..
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PutTaggedRemainderInZone {
                        zone: Zone::Graveyard,
                        ..
                    },
                ..
            }),
        ] => {
            assert_eq!(*player, PlayerAst::You);
            assert_eq!(*count, crate::effect::ChoiceCount::exactly(1));
        }
        other => panic!("expected composed looked-cards split effects, got {other:?}"),
    }
}

#[test]
fn exile_then_shuffle_graveyard_chain_keeps_both_effects() {
    let tokens = lex_line(
        "Exile all cards from your library face down, then shuffle all cards from your graveyard into your library.",
        0,
    )
    .expect("rewrite lexer should classify exile-then-shuffle text");
    let effects = parse_effect_chain_lexed(&tokens).expect("chain should parse");
    let debug = format!("{effects:?}");

    assert!(
        debug.contains("ExileAll")
            && debug.contains("face_down: true")
            && debug.contains("ShuffleGraveyardIntoLibrary"),
        "expected exile-all face-down and graveyard shuffle effects, got {debug}"
    );
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileAll {
                    face_down: true,
                    ..
                },
                ..
            })
        )),
        "expected a face-down exile-all effect in the parsed chain: {debug}"
    );
    assert!(
        effects.iter().any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::ShuffleGraveyardIntoLibrary,
                    ..
                })
            )
        }),
        "expected a graveyard shuffle effect in the parsed chain: {debug}"
    );
}

#[test]
fn or_action_clause_preserves_secondary_or_inside_sacrifice_filter() {
    let tokens = lex_line(
        "Discard two cards or sacrifice a creature or planeswalker of your choice",
        0,
    )
    .expect("or-action text should lex");

    let parsed = super::parse_or_action_clause_lexed(&tokens)
        .expect("or-action parse should succeed")
        .expect("or-action clause should be recognized");

    let debug = format!("{parsed:?}");
    assert!(
        debug.contains("UnlessAction"),
        "expected or-action lowering to use unless-action AST, got {debug}"
    );
    assert!(
        debug.contains("Discard"),
        "expected discard branch in or-action AST, got {debug}"
    );
    assert!(
        debug.contains("Sacrifice"),
        "expected sacrifice branch in or-action AST, got {debug}"
    );
    assert!(
        debug.contains("Planeswalker"),
        "expected sacrifice filter to keep planeswalker branch, got {debug}"
    );
}
