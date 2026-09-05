use super::*;
use crate::Subtype;

#[test]
fn as_you_cast_from_zone_this_turn_grant_preserves_origin_duration_and_keyword() {
    let tokens = crate::lexer::lex_line(
        "As you cast spells from your hand this turn, they gain cascade.",
        0,
    )
    .expect("cast-origin grant should lex");
    let effects = super::parse_effect_sentence_lexed(&tokens)
        .expect("public sentence route should retain the cast-origin grant");
    let [effect] = effects.as_slice() else {
        panic!("expected one cast-origin grant: {effects:#?}")
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
                ..
            },
        ..
    }) = effect
    else {
        panic!("expected typed grant-all effect")
    };

    assert_eq!(filter.zone, Some(Zone::Hand));
    assert_eq!(
        filter.stack_kind,
        Some(crate::filter::StackObjectKind::Spell)
    );
    assert_eq!(filter.cast_by, Some(PlayerFilter::You));
    assert!(filter.has_as_you_cast_this_turn_surface());
    assert_eq!(*duration, Until::EndOfTurn);
    assert!(matches!(
        abilities.as_slice(),
        [GrantedAbilityAst::KeywordAction(action)]
            if matches!(action.as_ref(), KeywordAction::Cascade)
    ));
}

#[test]
fn permanent_grant_does_not_enter_cast_origin_route() {
    let tokens = crate::lexer::lex_line("Creatures you control gain trample until end of turn.", 0)
        .expect("ordinary grant should lex");

    assert!(
        parse_as_you_cast_from_zone_this_turn_grant(&tokens)
            .expect("ordinary grant route should not error")
            .is_none()
    );
}

#[test]
fn top_level_cant_route_preserves_leading_end_of_turn_surface() {
    let parse_surface = |text: &str| {
        let tokens = crate::lexer::lex_line(text, 0).expect("temporary restriction should lex");
        let (_, effects) = parse_top_level_subject_verb_recognition(&tokens)
            .expect("top-level restriction route should not error")
            .expect("top-level restriction route should match");
        effects
            .iter()
            .find_map(|effect| {
                let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::Cant {
                            duration: crate::effect::Until::EndOfTurn,
                            duration_surface,
                            ..
                        },
                    ..
                }) = effect
                else {
                    return None;
                };
                Some(*duration_surface)
            })
            .expect("expected a typed end-of-turn restriction")
    };

    assert_eq!(
        parse_surface("Until end of turn, target creature can't be blocked by Walls."),
        crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
    );
    assert_eq!(
        parse_surface("Target creature can't be blocked by Walls this turn."),
        crate::effect::RestrictionDurationSurface::Default
    );
}

#[test]
fn top_cards_counted_hand_remainder_uses_captured_owners() {
    let tokens = crate::lexer::lex_line(
            "look at the top three cards of your library, then put one of those cards into that player's hand and the rest into that player's graveyard.",
            0,
        )
        .expect("rewrite lexer should classify looked-card bundle");
    let effects =
        parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb(&tokens)
            .expect("top-card hand/remainder parser should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(debug.contains("MoveTaggedGroupToZone"), "{debug}");
    assert!(debug.contains("PutTaggedRemainderInZone"), "{debug}");
    assert!(debug.contains("player: That"), "{debug}");
    assert!(!debug.contains("Unsupported"), "{debug}");
}

#[test]
fn counted_face_down_exile_keeps_target_opponents_library_owner() {
    let tokens = crate::lexer::lex_line(
            "Look at the top nine cards of target opponent's library, exile two of them face down, then put the rest on the bottom of their library in a random order.",
            0,
        )
        .expect("counted face-down exile bundle should lex");
    let effects = parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(&tokens)
        .expect("counted face-down exile bundle should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
    assert!(
        debug.matches("player: TargetOpponent").count() >= 2,
        "{debug}"
    );
}

#[test]
fn looked_cloak_partition_keeps_selected_and_remainder_tags_disjoint() {
    let tokens = crate::lexer::lex_line(
            "Look at the top five cards of your library, cloak two of them, and put the rest on the bottom of your library in a random order.",
            0,
        )
        .expect("looked-card cloak partition should lex");
    let effects = parse_generic_top_cards_cloak_counted_rest_bottom_subject_verb(&tokens)
        .expect("looked-card cloak partition should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(debug.contains("cloak: true"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
    assert!(debug.contains("Random"), "{debug}");
    assert!(!debug.contains("Unsupported"), "{debug}");

    let routed = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
        .expect("public sentence route should parse the complete cloak partition");
    let routed_debug = format!("{routed:#?}");
    assert!(routed_debug.contains("LookAtTopCards"), "{routed_debug}");
    assert!(routed_debug.contains("cloak: true"), "{routed_debug}");
    assert!(
        routed_debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{routed_debug}"
    );
    assert!(!routed_debug.contains("Unsupported"), "{routed_debug}");
}

#[test]
fn source_exiled_counted_return_keeps_original_set_for_the_remainder() {
    let tokens = crate::lexer::lex_line(
            "Return two cards exiled with this Saga to the battlefield under their owners' control and put the rest on the bottom of their owners' libraries.",
            0,
        )
        .expect("source-exiled partition should lex");
    let effects = parse_source_exiled_counted_return_remainder_to_owners_libraries(&tokens)
        .expect("typed source-exiled partition should match");
    let debug = format!("{effects:#?}");
    assert!(debug.contains("WithCount"), "{debug}");
    assert!(debug.contains("PutTaggedRemainderInZone"), "{debug}");
    assert!(debug.matches("__source_exiled__").count() >= 2, "{debug}");
    let EffectAst::TagAffected { effect, tag } = &effects[0] else {
        panic!("expected distinctly tagged counted return: {debug}");
    };
    assert_eq!(tag.as_str(), "source_exiled_returned");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::MoveToZone {
                target: TargetAst::WithCount(returned, count),
                ..
            },
        ..
    }) = effect.as_ref()
    else {
        panic!("expected counted typed return inside result tag: {debug}");
    };
    assert_eq!(count.min, 2, "{debug}");
    assert_eq!(count.max, Some(2), "{debug}");
    let TargetAst::Object(returned, _, _) = returned.as_ref() else {
        panic!("expected source-linked object filter: {debug}");
    };
    assert_eq!(
        returned,
        &ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.key())
            .in_zone(Zone::Exile),
        "the source's Saga type is provenance, not a restriction on returned cards"
    );
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::PutTaggedRemainderInZone {
                tag: original_set,
                keep_tagged,
                ..
            },
        ..
    }) = &effects[1]
    else {
        panic!("expected typed source-exiled complement: {debug}");
    };
    assert_eq!(
        original_set.as_str(),
        crate::tag::CompilerReferenceTag::SourceExiled.as_str()
    );
    assert_eq!(keep_tagged, tag);
    assert_ne!(keep_tagged, original_set);

    let near_miss = crate::lexer::lex_line(
            "Return two cards exiled with this Saga to the battlefield under their owners' control and put those cards on the bottom of their owners' libraries.",
            0,
        )
        .expect("near miss should lex");
    assert!(parse_source_exiled_counted_return_remainder_to_owners_libraries(&near_miss).is_none());
}

#[test]
fn counted_face_down_exile_accepts_implicit_looked_set() {
    let tokens = crate::lexer::lex_line(
            "Look at the top four cards of your library, exile one face down, then put the rest on the bottom of your library in any order.",
            0,
        )
        .expect("implicit looked-set face-down exile bundle should lex");
    let effects = parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(&tokens)
        .expect("implicit looked-set face-down exile bundle should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("face_down: true"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn full_sentence_dispatch_keeps_the_face_down_looked_partition_before_comma_then() {
    let tokens = crate::lexer::lex_line(
            "Look at the top four cards of your library, exile one face down, then put the rest on the bottom of your library in any order.",
            0,
        )
        .expect("face-down looked partition should lex");
    let effects = super::super::dispatch_entry::parse_effect_sentences_lexed(&tokens)
        .expect("full sentence dispatcher should preserve the partition");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("face_down: true"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn two_card_face_down_partition_accepts_the_single_other_without_order_text() {
    let tokens = crate::lexer::lex_line(
            "Look at the top two cards of target opponent's library. Exile one of them face down and put the other on the bottom of that library.",
            0,
        )
        .expect("two-card face-down partition should lex");
    let effects = parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(&tokens)
        .expect("two-card face-down partition should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
    assert!(
        debug.matches("player: TargetOpponent").count() >= 2,
        "{debug}"
    );
}

#[test]
fn face_down_exile_counter_stays_on_the_selected_card_not_the_remainder() {
    let tokens = crate::lexer::lex_line(
            "Look at the top three cards of your library. Exile one of them face down with a hatching counter on it, then put the rest on the bottom of your library in any order.",
            0,
        )
        .expect("face-down counter partition should lex");
    let effects = parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(&tokens)
        .expect("face-down counter partition should match");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::LookAtTopCards {
                    tag: looked_tag, ..
                },
            ..
        }),
        EffectAst::ChooseObjects {
            tag: selected_tag, ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Exile {
                    target: TargetAst::Tagged(exile_tag, _),
                    face_down: true,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutCounters {
                    counter_type: CounterType::Named(counter_name),
                    count: Value::Fixed(1),
                    target: TargetAst::Tagged(counter_tag, _),
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    tag: remainder_pool_tag,
                    keep_tagged: Some(remainder_keep_tag),
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("unexpected face-down counter partition AST: {effects:#?}");
    };
    assert_eq!(counter_name.as_str(), "hatching");

    assert_eq!(exile_tag, selected_tag);
    assert_eq!(counter_tag, selected_tag);
    assert_eq!(remainder_pool_tag, looked_tag);
    assert_eq!(remainder_keep_tag, selected_tag);
}

#[test]
fn consult_reveal_until_hand_uses_captured_consult_and_followup_clauses() {
    let tokens = crate::lexer::lex_line(
            "Reveal cards from the top of your library until you reveal a nonland card, then put all cards revealed this way into your hand.",
            0,
        )
        .expect("consult all-revealed-to-hand text should lex");
    let effects =
        parse_generic_consult_reveal_until_put_all_revealed_into_hand_subject_verb(&tokens)
            .expect("consult hand parser should not error")
            .expect("consult hand parser should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("MoveToZone"), "{debug}");
    assert!(debug.contains("Hand"), "{debug}");
    assert!(debug.contains("revealed"), "{debug}");
}

#[test]
fn undying_flames_exile_until_uses_typed_consult_traversal() {
    let tokens = crate::lexer::lex_line(
        "Exile cards from the top of your library until you exile a nonland card.",
        0,
    )
    .expect("Undying Flames consult text should lex");
    let effects = parse_generic_consult_reveal_until_subject_verb(&tokens)
        .expect("Undying Flames consult parser should not error")
        .expect("Undying Flames consult parser should match");

    assert!(matches!(
        effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Exile,
                filter,
                ..
            },
            ..
        })] if filter.excluded_card_types.contains(&crate::types::CardType::Land)
    ));
}

#[test]
fn consult_reveal_until_graveyard_moves_all_revealed_cards() {
    let tokens = crate::lexer::lex_line(
            "Each opponent reveals cards from the top of their library until they reveal X land cards, then puts all cards revealed this way into their graveyard.",
            0,
        )
        .expect("consult all-revealed-to-graveyard text should lex");
    let effects =
        parse_generic_consult_reveal_until_put_all_revealed_into_graveyard_subject_verb(&tokens)
            .expect("consult graveyard parser should not error")
            .expect("consult graveyard parser should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("MoveToZone"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
    assert!(debug.contains("revealed"), "{debug}");
    assert!(debug.contains("Opponent"), "{debug}");
}

#[test]
fn consult_reveal_until_battlefield_bottom_uses_captured_consult_and_followup_clauses() {
    let tokens = crate::lexer::lex_line(
            "Reveal cards from the top of your library until you reveal a creature card, put it onto the battlefield, then put the rest on the bottom of your library in any order.",
            0,
        )
        .expect("consult battlefield-bottom text should lex");
    let effects = parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(&tokens)
        .expect("consult battlefield-bottom parser should not error")
        .expect("consult battlefield-bottom parser should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("MoveToZone"), "{debug}");
    assert!(debug.contains("Battlefield"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn consult_reveal_until_battlefield_bottom_preserves_tapped_land_group() {
    let tokens = crate::lexer::lex_line(
            "Reveal cards from the top of your library until you reveal X land cards, put those land cards onto the battlefield tapped and the rest on the bottom of your library in a random order.",
            0,
        )
        .expect("consult tapped land battlefield-bottom text should lex");
    let effects = parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(&tokens)
        .expect("consult tapped land battlefield-bottom parser should not error")
        .expect("consult tapped land battlefield-bottom parser should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("MatchCount"), "{debug}");
    assert!(debug.contains("zone: Battlefield"), "{debug}");
    assert!(debug.contains("battlefield_tapped: true"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn each_player_exile_top_cast_uses_captured_exile_and_cast_clauses() {
    let tokens = crate::lexer::lex_line(
            "Exile the top card of each player's library, then you may cast any number of spells from among the nonland cards exiled this way without paying their mana costs.",
            0,
        )
        .expect("each-player exile-top cast text should lex");
    let effects = parse_generic_each_player_exile_top_then_cast_any_number_subject_verb(&tokens)
        .expect("each-player exile-top cast parser should not error")
        .expect("each-player exile-top cast parser should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("ForEachPlayer"), "{debug}");
    assert!(debug.contains("ForEachObject"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
}

#[test]
fn zone_replacement_uses_captured_condition_and_replacement_clauses() {
    for text in [
        "If that card would be put into your graveyard this turn, exile that card instead.",
        "If a card would be put into your graveyard from anywhere this turn, exile that card instead.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0)
            .expect("future graveyard exile replacement text should lex");
        let effect = parse_zone_replacement_subject_verb(&tokens)
            .expect("zone replacement parser should not error")
            .expect("zone replacement parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("AffectedPlayer"), "{debug}");
        assert!(debug.contains("You"), "{debug}");
        assert!(debug.contains("ExileInsteadOfGraveyardThisTurn"), "{debug}");

        let public = super::super::parse_effect_sentences_lexed(&tokens)
            .expect("public effect-sentence parser should accept the replacement");
        let public_debug = format!("{public:#?}");
        assert!(
            public_debug.contains("ExileInsteadOfGraveyardThisTurn"),
            "{public_debug}"
        );
    }
}

#[test]
fn play_permission_uses_captured_duration_and_permission_tail() {
    let tokens = crate::lexer::lex_line(
        "Until end of turn, you may play lands and cast spells from your graveyard.",
        0,
    )
    .expect("graveyard play permission text should lex");
    let effect = parse_play_permission_subject_verb(&tokens)
        .expect("play permission parser should not error")
        .expect("play permission parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("PlayFromGraveyardUntilEot"), "{debug}");
    assert!(debug.contains("You"), "{debug}");
}

#[test]
fn secret_number_choice_vote_uses_captured_participants_and_options() {
    let tokens = crate::lexer::lex_line(
        "You and target opponent each secretly choose 1, 2, or 3.",
        0,
    )
    .expect("secret numeric choice vote text should lex");
    let effect = parse_secret_number_choice_vote_start(&tokens)
        .expect("secret numeric choice vote parser should not error")
        .expect("secret numeric choice vote parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("SecretChoiceStart"), "{debug}");
    assert!(debug.contains("\"1\""), "{debug}");
    assert!(debug.contains("\"2\""), "{debug}");
    assert!(debug.contains("\"3\""), "{debug}");
    assert!(debug.contains("Target"), "{debug}");
}

#[test]
fn secret_number_choice_declines_secret_object_choices() {
    let tokens = crate::lexer::lex_line(
        "You and target opponent each secretly choose a creature that player controls.",
        0,
    )
    .expect("secret object choice text should lex");

    assert!(
        parse_secret_number_choice_vote_start(&tokens)
            .expect("the numeric rule should not invalidate a sibling object choice")
            .is_none(),
        "object choices belong to the typed secret-object rule"
    );
}

#[test]
fn generic_vote_start_uses_captured_voters_and_options() {
    let tokens = crate::lexer::lex_line("Each player votes for death or torture.", 0)
        .expect("generic vote-start text should lex");
    let effect = parse_generic_vote_start(&tokens)
        .expect("generic vote-start parser should not error")
        .expect("generic vote-start parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("VoteStart"), "{debug}");
    assert!(debug.contains("death"), "{debug}");
    assert!(debug.contains("torture"), "{debug}");
}

#[test]
fn generic_vote_start_prefers_named_options_over_source_name_alias() {
    let tokens = crate::lexer::lex_line(
        "Each player secretly votes for truth or consequences, then those votes are revealed.",
        0,
    )
    .expect("source-name vote text should lex");
    let effect = (|| {
        parse_generic_vote_start(&tokens)
            .expect("generic vote-start parser should not error")
            .expect("generic vote-start parser should match")
    })();
    let debug = format!("{effect:#?}");

    assert!(debug.contains("VoteStart"), "{debug}");
    assert!(debug.contains("truth"), "{debug}");
    assert!(debug.contains("consequences"), "{debug}");
    assert!(!debug.contains("VoteStartObjects"), "{debug}");
}

#[test]
fn generic_vote_option_effect_uses_captured_option_and_effect_tail() {
    let tokens = crate::lexer::lex_line("For each death vote, draw a card.", 0)
        .expect("generic vote-option effect text should lex");
    let effect = parse_generic_vote_option_effects(&tokens)
        .expect("generic vote-option parser should not error")
        .expect("generic vote-option parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("VoteOption"), "{debug}");
    assert!(debug.contains("death"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
}

#[test]
fn player_vote_received_effect_uses_captured_player_and_effect_tail() {
    let tokens = crate::lexer::lex_line("For each vote you received, draw a card.", 0)
        .expect("player vote-received effect text should lex");
    let effect = parse_generic_vote_option_effects(&tokens)
        .expect("player vote-received parser should not error")
        .expect("player vote-received parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("RepeatEffects"), "{debug}");
    assert!(debug.contains("PlayerVoteCount"), "{debug}");
    assert!(debug.contains("You"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
}

#[test]
fn extra_vote_uses_captured_optional_vote_shape() {
    let tokens = crate::lexer::lex_line("You may vote an additional time.", 0)
        .expect("optional extra vote text should lex");
    let effect =
        parse_generic_extra_vote(&tokens).expect("optional extra vote parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("VoteExtra"), "{debug}");
    assert!(debug.contains("count: 1"), "{debug}");
    assert!(debug.contains("optional: true"), "{debug}");
}

#[test]
fn extra_vote_uses_captured_required_vote_shape() {
    let tokens = crate::lexer::lex_line("You vote an additional time.", 0)
        .expect("required extra vote text should lex");
    let effect =
        parse_generic_extra_vote(&tokens).expect("required extra vote parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("VoteExtra"), "{debug}");
    assert!(debug.contains("count: 1"), "{debug}");
    assert!(debug.contains("optional: false"), "{debug}");
}

#[test]
fn extra_vote_accepts_subjectless_clause_inside_optional_wrapper() {
    let tokens = crate::lexer::lex_line("Vote an additional time.", 0)
        .expect("subjectless extra vote text should lex");
    let effect =
        parse_generic_extra_vote(&tokens).expect("subjectless extra vote parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("VoteExtra"), "{debug}");
    assert!(debug.contains("count: 1"), "{debug}");
    assert!(debug.contains("optional: false"), "{debug}");
}

#[test]
fn vote_reveal_uses_captured_choice_reveal_shape() {
    let tokens = crate::lexer::lex_line("Then those choices are revealed.", 0)
        .expect("vote reveal text should lex");
    let effect = parse_vote_reveal_sentence(&tokens).expect("vote reveal parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("SecretChoiceReveal"), "{debug}");
}

#[test]
fn control_combat_choices_uses_captured_attack_shape() {
    let tokens = crate::lexer::lex_line("You choose which creatures attack this turn.", 0)
        .expect("combat choice attack text should lex");
    let effect = parse_generic_control_combat_choices_subject_verb(&tokens)
        .expect("combat choice attack parser should not error")
        .expect("combat choice attack parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("ControlCombatChoicesThisTurn"), "{debug}");
    assert!(debug.contains("attackers: true"), "{debug}");
    assert!(debug.contains("blockers: false"), "{debug}");
}

#[test]
fn control_combat_choices_uses_captured_block_shape() {
    for (text, this_combat) in [
        (
            "You choose which creatures block this turn and how those creatures block.",
            false,
        ),
        (
            "You choose which creatures block this combat and how those creatures block.",
            true,
        ),
    ] {
        let tokens = crate::lexer::lex_line(text, 0).expect("combat choice block text should lex");
        let effect = parse_generic_control_combat_choices_subject_verb(&tokens)
            .expect("combat choice block parser should not error")
            .expect("combat choice block parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("ControlCombatChoicesThisTurn"), "{debug}");
        assert!(debug.contains("attackers: false"), "{debug}");
        assert!(debug.contains("blockers: true"), "{debug}");
        assert!(
            debug.contains(&format!("this_combat: {this_combat}")),
            "{debug}"
        );
    }
}

#[test]
fn control_combat_choices_accepts_anaphoric_block_assignment_shape() {
    let tokens = crate::lexer::lex_line("You choose how those creatures block.", 0)
        .expect("anaphoric combat-choice text should lex");
    let effect = parse_generic_control_combat_choices_subject_verb(&tokens)
        .expect("combat-choice parser should not error")
        .expect("anaphoric block-assignment parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("ControlCombatChoicesThisTurn"), "{debug}");
    assert!(debug.contains("attackers: false"), "{debug}");
    assert!(debug.contains("blockers: true"), "{debug}");
    assert!(debug.contains("this_combat: false"), "{debug}");
}

#[test]
fn where_x_value_binding_uses_captured_effect_and_definition() {
    let tokens = crate::lexer::lex_line(
            "Target creature gets +X/+X until end of turn, where X is the number of cards in your hand.",
            0,
        )
        .expect("where-x value-binding text should lex");
    let non_binding_tokens =
        crate::lexer::lex_line("Target creature gets +1/+1 until end of turn.", 0)
            .expect("non-binding pump text should lex");

    assert!(has_where_x_value_binding(&tokens));
    assert!(!has_where_x_value_binding(&non_binding_tokens));
}

#[test]
fn where_x_player_comparison_keeps_participant_cardinality() {
    for text in [
        "Search your library for up to X Plains cards, where X is the number of players who control more lands than you.",
        "Create X 1/1 white Spirit creature tokens with flying, where X is the number of opponents who control more lands than you.",
    ] {
        let tokens =
            crate::lexer::lex_line(text, 0).expect("player-comparison where-X text should lex");
        let effects = parse_effect_sentence_with_where_x_lexed(&tokens)
            .expect("player-comparison where-X text should parse");
        let debug = format!("{effects:#?}");

        assert!(
            debug.contains("PlayersWhoControlMoreThanYou"),
            "participant cardinality collapsed to an object count: {debug}"
        );
    }

    let tokens = crate::lexer::lex_line(
            "Search your library for up to X basic land cards, where X is the number of players who control at least two more lands than you.",
            0,
        )
        .expect("minimum-difference player-comparison text should lex");
    let effects = parse_effect_sentence_with_where_x_lexed(&tokens)
        .expect("minimum-difference player-comparison text should parse");
    let debug = format!("{effects:#?}");
    assert!(
        debug.contains("PlayersWhoControlAtLeastMoreThanYou")
            && debug.contains("minimum_difference: 2"),
        "minimum-difference participant cardinality collapsed: {debug}"
    );
}

#[test]
fn where_x_scry_amount_binds_the_dynamic_counter_target_count() {
    let tokens = crate::lexer::lex_line(
            "Put a +1/+1 counter on each of up to X target creatures, where X is the number of cards looked at while scrying this way.",
            0,
        )
        .expect("scry-derived target-count text should lex");

    assert!(has_where_x_value_binding(&tokens));
    let effects = parse_effect_sentence_with_where_x_lexed(&tokens)
        .expect("scry-derived target-count text should parse");
    let debug = format!("{effects:#?}");
    assert!(debug.contains("WithCountValue"), "{debug}");
    assert!(debug.contains("EventValue"), "{debug}");
    assert!(
        debug.contains("CardsLookedAtWhileScryingThisWay"),
        "{debug}"
    );
}

#[test]
fn where_x_binding_prioritizes_spell_history_aggregate_over_plain_count() {
    let tokens = crate::lexer::lex_line(
            "Create an X/X blue and red Elemental creature token with flying and haste, where X is the greatest mana value among instant and sorcery spells you've cast this turn.",
            0,
        )
        .expect("spell-history aggregate create text should lex");
    let effects = super::super::dispatch_entry::parse_effect_sentences_lexed(&tokens)
        .expect("spell-history aggregate create text should parse");
    let [EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
        panic!("expected one token-creation effect: {effects:#?}");
    };
    let SubjectVerbActionAst::CreateTokenWithMods {
        dynamic_power_toughness: Some((power, toughness)),
        ..
    } = &subject_verb.action
    else {
        panic!("expected dynamic token power and toughness: {effects:#?}");
    };
    for value in [power, toughness] {
        let Value::GreatestManaValue(filter) = value.unhinted() else {
            panic!("aggregate must not collapse into a count: {value:#?}");
        };
        assert!(filter.cast_this_turn, "{filter:#?}");
        assert_eq!(filter.cast_by, Some(PlayerFilter::You), "{filter:#?}");
    }
}

#[test]
fn shared_where_x_sum_binds_the_full_value_to_each_pump_clause() {
    let text = "Target creature you control gets +X/+0 until end of turn and up to one target creature an opponent controls gets -0/-X until end of turn, where X is the number of Elves you control plus the number of Elf cards in your graveyard.";
    let tokens = crate::lexer::lex_line(text, 0).expect("shared sum where-x pump text should lex");
    let effects = parse_effect_sentence_with_where_x_lexed(&tokens)
        .expect("shared sum where-x pump text should parse");
    let debug = format!("{effects:#?}");

    assert_eq!(effects.len(), 1, "{debug}");
    assert!(debug.contains("Coordination"), "{debug}");
    assert_eq!(debug.matches("action: Pump {").count(), 2, "{debug}");
    assert_eq!(debug.matches("Add(").count(), 2, "{debug}");
    assert!(debug.matches("Battlefield").count() >= 2, "{debug}");
    assert_eq!(debug.matches("Graveyard").count(), 2, "{debug}");
    assert_eq!(debug.matches("WhereXIs").count(), 2, "{debug}");
    assert!(debug.contains("Scaled("), "{debug}");
    assert!(debug.contains("-1,"), "{debug}");
}

#[test]
fn shared_where_x_sum_binds_the_full_value_to_damage() {
    let text = "This deals X damage to target creature, where X is the number of creatures you control plus the number of Foods you control.";
    let tokens = crate::lexer::lex_line(text, 0).expect("shared sum damage text should lex");
    let effects = parse_effect_sentence_with_where_x_lexed(&tokens)
        .expect("shared sum damage text should parse");
    let debug = format!("{effects:#?}");

    assert_eq!(effects.len(), 1, "{debug}");
    assert_eq!(debug.matches("Add(").count(), 1, "{debug}");
    assert!(debug.contains("Food"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
    assert_eq!(debug.matches("WhereXIs").count(), 1, "{debug}");
}

#[test]
fn shared_where_x_dynamic_subtraction_binds_both_hand_counts() {
    fn collect_hand_players<'a>(value: &'a Value, players: &mut Vec<&'a PlayerFilter>) {
        match value {
            Value::SurfaceHinted { value, .. } | Value::Scaled(value, _) => {
                collect_hand_players(value, players);
            }
            Value::Add(left, right) => {
                collect_hand_players(left, players);
                collect_hand_players(right, players);
            }
            Value::Count(filter) if filter.zone == Some(Zone::Hand) => {
                if let Some(owner) = filter.owner.as_ref() {
                    players.push(owner);
                }
            }
            Value::CardsInHand(player) => players.push(player),
            _ => {}
        }
    }

    for (text, expected_players) in [
        (
            "That player loses X life, where X is the number of cards in that player's hand minus the number of cards in your hand.",
            [PlayerFilter::IteratedPlayer, PlayerFilter::You],
        ),
        (
            "This enchantment deals X damage to target opponent, where X is the number of cards in your hand minus the number of cards in that player's hand.",
            [PlayerFilter::You, PlayerFilter::IteratedPlayer],
        ),
    ] {
        let tokens =
            crate::lexer::lex_line(text, 0).expect("dynamic subtraction where-x text should lex");
        let effects = parse_effect_sentence_with_where_x_lexed(&tokens)
            .expect("dynamic subtraction where-x text should parse");
        let debug = format!("{effects:#?}");

        assert_eq!(debug.matches("Add(").count(), 1, "{text}: {debug}");
        assert!(debug.contains("Scaled("), "{text}: {debug}");
        assert!(debug.contains("-1,"), "{text}: {debug}");
        assert_eq!(debug.matches("Hand").count(), 2, "{text}: {debug}");
        assert_eq!(debug.matches("WhereXIs").count(), 1, "{text}: {debug}");
        assert_eq!(
            debug.matches("ThatPlayerPossessive").count(),
            1,
            "{text}: authored that-player possessive provenance: {debug}"
        );

        let [EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
            panic!("{text}: expected one subject-verb effect: {effects:#?}");
        };
        let amount = match &subject_verb.action {
            SubjectVerbActionAst::LoseLife { amount }
            | SubjectVerbActionAst::DealDamage { amount, .. } => amount,
            action => panic!("{text}: expected life-loss or damage action: {action:#?}"),
        };
        let mut actual_players = Vec::new();
        collect_hand_players(amount, &mut actual_players);
        assert_eq!(
            actual_players,
            expected_players.iter().collect::<Vec<_>>(),
            "{text}: hand-count player references must remain typed: {amount:#?}"
        );
    }
}

#[test]
fn where_x_binding_reaches_granted_entry_counter_static_ability() {
    for (text, expected) in [
        (
            "That creature enters with X additional +1/+1 counters on it, where X is the number of ingredient counters on this enchantment.",
            &[
                "CountersOn(",
                "spec: Source",
                "this enchantment",
                "ingredient",
                "WhereXIs",
            ][..],
        ),
        (
            "That creature enters with X additional +1/+1 counters on it, where X is its mana value minus 4.",
            &["ManaValueOf(", "Fixed(", "-4", "WhereXIs"][..],
        ),
        (
            "That creature enters with X additional +1/+1 counters on it, where X is the number of colors of mana spent to cast it.",
            &["ColorsOfManaSpentToCastThisSpell", "WhereXIs"][..],
        ),
    ] {
        let tokens =
            crate::lexer::lex_line(text, 0).expect("dynamic entry-counter text should lex");
        let effects = parse_effect_sentence_with_where_x_lexed(&tokens)
            .expect("dynamic entry-counter text should parse");
        let debug = format!("{effects:#?}");

        assert!(
            debug.contains("EntersWithCountersAndSubtypesForFilter"),
            "{text}: {debug}"
        );
        for fragment in expected {
            assert!(
                debug.contains(fragment),
                "{text}: missing {fragment}: {debug}"
            );
        }
        assert!(
            !debug.contains("count: Fixed(\n                            1,"),
            "{text}: dynamic X must not freeze to one: {debug}"
        );
    }
}

#[test]
fn explicit_source_entry_where_x_keeps_source_and_dynamic_counter_value() {
    let tokens = crate::lexer::lex_line(
            "This creature enters with X +1/+1 counters on it, where X is the total mana value of all cards revealed this way.",
            0,
        )
        .expect("source entry-counter text should lex");
    let effects = parse_effect_sentence_with_where_x_lexed(&tokens)
        .expect("source entry-counter text should parse");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("target: Source"), "{debug}");
    assert!(debug.contains("EntersWithCountersValue"), "{debug}");
    assert!(!debug.contains("count: X"), "{debug}");
    assert!(debug.contains("WhereXIs"), "{debug}");
    assert!(debug.contains("ManaValue"), "{debug}");
}

#[test]
fn where_x_value_binding_accepts_quoted_token_abilities() {
    for text in [
        "Create X 1/1 black Fungus creature tokens with \"This token can't block,\" where X is the number of times you descended this turn.",
        "Create X 1/1 black Rat creature tokens with \"This token can't block,\" where X is the amount of damage dealt to it this turn.",
        "Create X 1/1 black and green Pest creature tokens with \"When this token dies, you gain 1 life,\" where X is the sacrificed creature's power.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).expect("quoted token where-x text should lex");
        assert!(has_where_x_value_binding(&tokens), "{text}");
    }
}

#[test]
fn choice_complement_uses_captured_choice_and_sacrifice_shape() {
    let tokens = crate::lexer::lex_line(
            "Each player chooses a creature from among creatures they control, then sacrifices the rest.",
            0,
        )
        .expect("choice-complement text should lex");
    let effect = parse_choice_complement_subject_verb(&tokens)
        .expect("choice-complement parser should not error")
        .expect("choice-complement parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("ForEachPlayer"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("Sacrifice"), "{debug}");
    assert!(debug.contains("keep"), "{debug}");
}

#[test]
fn counted_choice_complement_keeps_that_many_and_sacrifices_others() {
    let tokens = crate::lexer::lex_line(
        "Each player chooses five lands they control and sacrifices the rest.",
        0,
    )
    .expect("counted choice-complement text should lex");
    let effect = parse_choice_complement_subject_verb(&tokens)
        .expect("counted choice-complement parser should not error")
        .expect("counted choice-complement parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("ForEachPlayer"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("min: 5"), "{debug}");
    assert!(
        debug.contains("max: Some(\n                    5"),
        "{debug}"
    );
    assert!(debug.contains("SacrificeAll"), "{debug}");
    assert!(debug.contains("keep"), "{debug}");
}

#[test]
fn aggregate_choice_complement_keeps_the_group_power_constraint() {
    let tokens = crate::lexer::lex_line(
            "Each player chooses any number of creatures they control with total power 4 or less, then sacrifices all other creatures they control.",
            0,
        )
        .expect("aggregate choice-complement text should lex");
    let effect = parse_choice_complement_subject_verb(&tokens)
        .expect("aggregate choice-complement parser should not error")
        .expect("aggregate choice-complement parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("ForEachPlayer"), "{debug}");
    assert!(
        debug.contains("ChooseObjectsWithAggregateConstraint"),
        "{debug}"
    );
    assert!(
        debug.contains("Power") && debug.contains("maximum: Fixed(\n                    4"),
        "{debug}"
    );
    assert!(debug.contains("SacrificeAll"), "{debug}");

    let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
        .expect("aggregate choice-complement full sentence should parse");
    let full_debug = format!("{effects:#?}");
    assert!(
        full_debug.contains("ChooseObjectsWithAggregateConstraint"),
        "{full_debug}"
    );
}

#[test]
fn party_choice_complement_uses_four_optional_distinct_role_slots() {
    let tokens = crate::lexer::lex_line(
        "Each player chooses a party from among creatures they control, then sacrifices the rest.",
        0,
    )
    .expect("party choice text should lex");
    let effect = parse_choice_complement_subject_verb(&tokens)
        .expect("party choice parser should not error")
        .expect("party choice parser should match");
    let EffectAst::ForEachPlayer { effects } = effect else {
        panic!("party choice must iterate over players: {effect:#?}");
    };
    assert_eq!(effects.len(), 5, "{effects:#?}");
    let mut roles = Vec::new();
    for choice in &effects[..4] {
        let EffectAst::ChooseObjects { filter, count, .. } = choice else {
            panic!("party slot must be an object choice: {choice:#?}");
        };
        assert_eq!(*count, ChoiceCount::up_to(1));
        assert!(filter.card_types.contains(&CardType::Creature));
        assert!(filter.controller == Some(PlayerFilter::IteratedPlayer));
        roles.extend(filter.subtypes.iter().copied());
    }
    assert_eq!(
        roles,
        vec![
            Subtype::Cleric,
            Subtype::Rogue,
            Subtype::Warrior,
            Subtype::Wizard,
        ]
    );
    assert!(
        matches!(effects[4], EffectAst::SubjectVerb(_)),
        "{effects:#?}"
    );

    let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
        .expect("full effect parser should accept party complement");
    let full_debug = format!("{effects:#?}");
    assert_eq!(
        full_debug.matches("ChooseObjects").count(),
        4,
        "{full_debug}"
    );
    assert!(full_debug.contains("SacrificeAll"), "{full_debug}");
}

#[test]
fn triggering_spell_damage_uses_triggering_spell_as_source_and_fans_out() {
    let tokens = crate::lexer::lex_line(
            "That spell deals damage to each opponent equal to the number of instant and sorcery spells you've cast this turn.",
            0,
        )
        .expect("triggering-spell damage text should lex");
    let effect = parse_triggered_spell_opponent_damage_subject_verb(&tokens)
        .expect("triggering-spell damage parser should not error")
        .expect("triggering-spell damage parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("ForEachOpponent"), "{debug}");
    assert!(debug.contains("triggering"), "{debug}");
    assert!(debug.contains("SpellsCastThisTurnMatching"), "{debug}");
    assert!(
        debug.contains("Instant") && debug.contains("Sorcery"),
        "{debug}"
    );

    let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
        .expect("triggering-spell damage full sentence should parse");
    let full_debug = format!("{effects:#?}");
    assert!(full_debug.contains("ForEachOpponent"), "{full_debug}");
    assert!(full_debug.contains("triggering"), "{full_debug}");
}

#[test]
fn choice_complement_preserves_independent_keep_slots_for_type_lists() {
    let tokens = crate::lexer::lex_line(
            "Each player chooses from among the permanents they control an artifact, a creature, an enchantment, and a land, then sacrifices the rest.",
            0,
        )
        .expect("choice-complement type-list text should lex");
    let recovered = choice_complement_choice_clause_from_word_order(LexedClause::new(&tokens))
        .expect("from-among word-order helper should recover choice clause");
    assert!(
        crate::lexer::render_token_slice(recovered.tokens()).contains("from among"),
        "{}",
        crate::lexer::render_token_slice(recovered.tokens())
    );
    let recovered_tokens = recovered.tokens();
    let from_idx = find_from_among(recovered_tokens).expect("should find from among");
    assert_eq!(from_idx, 0);
    let list_start = find_list_start(&recovered_tokens[2..])
        .map(|idx| idx + 2)
        .expect("should find choice list start");
    let base_tokens = trim_commas(recovered_tokens.get(2..list_start).unwrap_or_default());
    let list_tokens = trim_commas(recovered_tokens.get(list_start..).unwrap_or_default());
    assert!(
        !base_tokens.is_empty(),
        "base was empty; recovered={}",
        crate::lexer::render_token_slice(recovered.tokens())
    );
    assert!(
        !list_tokens.is_empty(),
        "list was empty; recovered={}",
        crate::lexer::render_token_slice(recovered.tokens())
    );
    let effect = parse_choice_complement_subject_verb(&tokens)
        .expect("choice-complement parser should not error")
        .expect("choice-complement parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("ForEachPlayer"), "{debug}");
    assert_eq!(debug.matches("ChooseObjects").count(), 4, "{debug}");
    assert!(debug.contains("Artifact"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
    assert!(debug.contains("Enchantment"), "{debug}");
    assert!(debug.contains("Land"), "{debug}");
    assert!(debug.contains("Sacrifice"), "{debug}");
}

#[test]
fn choice_complement_full_effect_sentence_keeps_comma_list_together() {
    let tokens = crate::lexer::lex_line(
            "Each player chooses from among the permanents they control an artifact, a creature, an enchantment, and a land, then sacrifices the rest.",
            0,
        )
        .expect("choice-complement type-list text should lex");
    let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
        .expect("choice-complement full sentence should parse");
    let debug = format!("{effects:#?}");

    assert_eq!(debug.matches("ChooseObjects").count(), 4, "{debug}");
    assert!(debug.contains("Artifact"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
    assert!(debug.contains("Enchantment"), "{debug}");
    assert!(debug.contains("Land"), "{debug}");
    assert!(debug.contains("Sacrifice"), "{debug}");

    let statement_effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens)
        .expect("choice-complement statement should parse without splitting its list");
    let statement_debug = format!("{statement_effects:#?}");
    assert_eq!(
        statement_debug.matches("ChooseObjects").count(),
        4,
        "{statement_debug}"
    );
    assert!(statement_debug.contains("Sacrifice"), "{statement_debug}");
}

#[test]
fn each_opponent_choice_complement_uses_opponent_scope() {
    let tokens = crate::lexer::lex_line(
            "Each opponent chooses an artifact, a creature, an enchantment, and a planeswalker from among the nonland permanents they control, then sacrifices the rest.",
            0,
        )
        .expect("opponent choice-complement text should lex");
    let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
        .expect("opponent choice-complement should parse");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("ForEachOpponent"), "{debug}");
    assert_eq!(debug.matches("ChooseObjects").count(), 4, "{debug}");
    assert!(debug.contains("Sacrifice"), "{debug}");
}

#[test]
fn source_gets_unblockable_uses_captured_subject_modifier_and_tail() {
    let tokens = crate::lexer::lex_line(
        "This creature gets +1/+1 until end of turn and can't be blocked this turn.",
        0,
    )
    .expect("source pump plus unblockable text should lex");
    let effects = parse_source_gets_unblockable_subject_verb(&tokens)
        .expect("source pump plus unblockable parser should not error")
        .expect("source pump plus unblockable parser should match");
    let debug = format!("{effects:#?}");

    assert_eq!(effects.len(), 2, "{debug}");
    assert!(debug.contains("Pump"), "{debug}");
    assert!(debug.contains("Fixed") && debug.contains("1"), "{debug}");
    assert!(debug.contains("BeBlocked"), "{debug}");
    assert!(debug.contains("source: true"), "{debug}");
}

#[test]
fn attached_object_destroy_and_source_damage_keeps_one_linked_program() {
    let tokens = crate::lexer::lex_line(
        "Destroy enchanted land and this Aura deals 2 damage to that land's controller.",
        0,
    )
    .expect("attached-object damage chain should lex");
    let effects = parse_destroy_attached_object_then_source_damage_to_controller(&tokens)
        .expect("attached-object damage parser should not error")
        .expect("attached-object damage parser should match");
    let debug = format!("{effects:#?}");

    assert_eq!(effects.len(), 1, "{debug}");
    assert!(debug.contains("Destroy"), "{debug}");
    assert!(debug.contains("DealDamage"), "{debug}");
    assert!(debug.contains("enchanted"), "{debug}");
    assert!(debug.contains("ControllerOf"), "{debug}");
}

#[test]
fn attached_object_damage_rejects_a_mismatched_controller_noun() {
    let tokens = crate::lexer::lex_line(
        "Destroy enchanted land and this Aura deals 2 damage to that creature's controller.",
        0,
    )
    .expect("near-miss chain should lex");
    assert!(
        parse_destroy_attached_object_then_source_damage_to_controller(&tokens)
            .expect("near-miss parser should not error")
            .is_none()
    );
}

#[test]
fn source_gets_filter_gains_uses_captured_filter_and_ability_tail() {
    let tokens = crate::lexer::lex_line(
        "This creature gets +1/+1 and creatures you control gain trample until end of turn.",
        0,
    )
    .expect("source pump plus ability-grant text should lex");
    let effects = parse_source_gets_filter_gains_subject_verb(&tokens)
        .expect("source pump plus ability-grant parser should not error")
        .expect("source pump plus ability-grant parser should match");
    let debug = format!("{effects:#?}");

    assert_eq!(effects.len(), 2, "{debug}");
    assert!(debug.contains("Pump"), "{debug}");
    assert!(
        debug.contains("power: Fixed") && debug.contains("1"),
        "{debug}"
    );
    assert!(debug.contains("GrantAbilitiesAll"), "{debug}");
    assert!(debug.contains("Trample"), "{debug}");
}

#[test]
fn target_gains_then_gets_gate_uses_captured_ability_and_pump_tail() {
    let tokens = crate::lexer::lex_line(
        "Target creature gains trample and gets +1/+0 until end of turn.",
        0,
    )
    .expect("target gains then gets text should lex");
    let effects = parse_target_gains_then_gets_subject_verb(&tokens)
        .expect("target gains then gets parser should not error")
        .expect("target gains then gets parser should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("Trample"), "{debug}");
    assert!(debug.contains("Pump"), "{debug}");
    assert!(debug.contains("Fixed") && debug.contains("1"), "{debug}");
}

#[test]
fn target_gains_then_gets_where_x_reuses_the_exact_declared_target() {
    let tokens = crate::lexer::lex_line(
            "Target creature gains trample and gets +X/+0 until end of turn, where X is that creature's mana value.",
            0,
        )
        .expect("target mana-value pump text should lex");
    let effects = parse_target_gains_then_gets_subject_verb(&tokens)
        .expect("target gain-then-get parser should not error")
        .expect("target gain-then-get parser should match");
    let [EffectAst::Coordination(coordination)] = effects.as_slice() else {
        panic!("expected one canonical coordination, got {effects:#?}");
    };
    let effects = coordination.effects().cloned().collect::<Vec<_>>();
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TargetOnly { target, .. },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    target: grant,
                    duration: grant_duration,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Pump {
                    target: pump,
                    power,
                    duration: pump_duration,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one target declaration plus grant/pump consumers, got {effects:#?}");
    };

    assert_eq!(grant, pump, "the shared subject must reuse one target");
    assert!(matches!(target, TargetAst::Object(..)), "{target:#?}");
    assert!(
        matches!(grant, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
    );
    assert_eq!(grant_duration, &crate::effect::Until::EndOfTurn);
    assert_eq!(pump_duration, &crate::effect::Until::EndOfTurn);
    assert!(matches!(
        power.unhinted(),
        Value::ManaValueOf(spec)
            if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
    ));
}

#[test]
fn full_dispatch_keeps_gain_then_get_on_one_declared_target() {
    let tokens = crate::lexer::lex_line(
            "Target creature gains trample and gets +X/+0 until end of turn, where X is that creature's mana value.",
            0,
        )
        .expect("target mana-value pump text should lex");
    let effects = super::super::dispatch_entry::parse_effect_sentences_lexed(&tokens)
        .expect("full sentence dispatcher should preserve the gain/get compound");
    let [EffectAst::Coordination(coordination)] = effects.as_slice() else {
        panic!("expected one canonical coordination, got {effects:#?}");
    };
    let effects = coordination.effects().cloned().collect::<Vec<_>>();
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TargetOnly { target, .. },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    target: grant,
                    duration: grant_duration,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Pump {
                    target: pump,
                    duration: pump_duration,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!(
            "expected one target declaration plus shared grant/pump consumers, got {effects:#?}"
        );
    };

    assert_eq!(grant, pump, "both arms must reuse the declared target");
    assert!(matches!(target, TargetAst::Object(..)), "{target:#?}");
    assert!(
        matches!(grant, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
    );
    assert_eq!(grant_duration, &crate::effect::Until::EndOfTurn);
    assert_eq!(pump_duration, &crate::effect::Until::EndOfTurn);
}

#[test]
fn target_gets_then_gains_gate_uses_captured_pump_and_ability_tail() {
    let tokens = crate::lexer::lex_line(
        "Target creature gets +1/+1 and gains trample until end of turn.",
        0,
    )
    .expect("target gets then gains text should lex");
    let effects = parse_target_gets_then_gains_subject_verb(&tokens)
        .expect("target gets then gains parser should not error")
        .expect("target gets then gains parser should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("Pump"), "{debug}");
    assert!(debug.contains("Fixed") && debug.contains("1"), "{debug}");
    assert!(debug.contains("Trample"), "{debug}");
}

#[test]
fn target_gets_then_gains_preserves_other_than_source_filter() {
    let tokens = crate::lexer::lex_line(
        "Target creature other than this creature gets +1/+1 and gains trample until end of turn.",
        0,
    )
    .expect("other-target get-then-gain text should lex");
    let effects = parse_target_gets_then_gains_subject_verb(&tokens)
        .expect("other-target parser should not error")
        .expect("other-target parser should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("other: true"), "{debug}");
    assert!(debug.contains("source: false"), "{debug}");
    assert!(!debug.contains("target: Source"), "{debug}");
    assert!(debug.contains("Trample"), "{debug}");
}

#[test]
fn target_gets_then_gains_preserves_sticker_filter_before_reflexive_pronoun() {
    let tokens = crate::lexer::lex_line(
            "Another target creature with an art sticker on it gets +2/+0 and gains menace until end of turn.",
            0,
        )
        .expect("stickered-target get-then-gain text should lex");
    let effects = parse_target_gets_then_gains_subject_verb(&tokens)
        .expect("stickered-target parser should not error")
        .expect("stickered-target parser should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("other: true"), "{debug}");
    assert!(
        debug.contains("sticker: Some") && debug.contains("ArtSticker"),
        "{debug}"
    );
    assert!(!debug.contains("target: Source"), "{debug}");
    assert!(debug.contains("Menace"), "{debug}");
}

#[test]
fn conditional_another_target_gets_then_gains_preserves_source_exclusion() {
    let tokens = crate::lexer::lex_line(
            "If you do, another target attacking creature gets +1/+0 and gains menace until end of turn.",
            0,
        )
        .expect("conditional another-target sentence should lex");
    let effects = parse_target_gets_then_gains_subject_verb(&tokens)
        .expect("conditional another-target parser should not error")
        .expect("conditional another-target parser should match");
    let debug = format!("{effects:?}");

    assert!(debug.contains("other: true"), "{debug}");
    assert!(debug.contains("attacking: true"), "{debug}");
    assert!(debug.contains("Menace"), "{debug}");
}

#[test]
fn duration_led_another_target_gets_then_gains_preserves_source_exclusion() {
    let tokens = crate::lexer::lex_line(
            "Until end of turn, another target creature you control gets +2/+0 and gains \"When this creature dies, return it to the battlefield tapped under its owner's control.\"",
            0,
        )
        .expect("duration-led another-target sentence should lex");
    let effects = parse_target_gets_then_gains_subject_verb(&tokens)
        .expect("duration-led another-target parser should not error")
        .expect("duration-led another-target parser should match");
    let debug = format!("{effects:?}");

    assert!(debug.contains("other: true"), "{debug}");
    assert!(debug.contains("controller: Some(You)"), "{debug}");
    assert!(debug.contains("ParsedObjectAbility"), "{debug}");
}

#[test]
fn attached_and_related_creatures_keep_both_subject_branches() {
    let tokens = crate::lexer::lex_line(
            "Enchanted creature and other creatures that share a creature type with it get +1/+0 and gain first strike until end of turn.",
            0,
        )
        .expect("attached and related creature text should lex");
    let effects = parse_target_gets_then_gains_subject_verb(&tokens)
        .expect("attached and related parser should not error")
        .expect("attached and related parser should match");
    let debug = format!("{effects:#?}");

    assert_eq!(effects.len(), 2, "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
    assert!(debug.contains("SharesSubtypeWithTagged"), "{debug}");
    assert!(debug.contains("FirstStrike"), "{debug}");
}

#[test]
fn attached_and_related_stat_pump_keeps_both_subject_branches() {
    let tokens = crate::lexer::lex_line(
            "Enchanted creature and other creatures that share a creature type with it get +1/+1 until end of turn.",
            0,
        )
        .expect("attached and related creature text should lex");
    let effects = parse_attached_and_related_get_subject_verb(&tokens)
        .expect("attached and related pump parser should not error")
        .expect("attached and related pump parser should match");
    let debug = format!("{effects:#?}");

    assert_eq!(effects.len(), 1, "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
    assert!(debug.contains("SharesSubtypeWithTagged"), "{debug}");
    assert!(debug.contains("Fixed") && debug.contains('1'), "{debug}");
}

#[test]
fn target_controlled_pump_uses_captured_granted_ability_tail() {
    let tokens = crate::lexer::lex_line(
        "Creatures target player controls get +1/+1 and gain haste until end of turn.",
        0,
    )
    .expect("target-controlled pump plus grant text should lex");
    let effects = parse_target_player_controls_get_subject_verb(&tokens)
        .expect("target-controlled pump parser should not error")
        .expect("target-controlled pump parser should match");
    let debug = format!("{effects:#?}");

    assert_eq!(effects.len(), 2, "{debug}");
    assert!(debug.contains("Pump"), "{debug}");
    assert!(debug.contains("Target") && debug.contains("Any"), "{debug}");
    assert!(debug.contains("GrantAbilitiesAll"), "{debug}");
    assert!(debug.contains("Haste"), "{debug}");
}

#[test]
fn target_controlled_pump_can_grant_all_creature_types() {
    let tokens = crate::lexer::lex_line(
        "Creatures target player controls get +0/+1 and gain all creature types until end of turn.",
        0,
    )
    .expect("target-controlled pump plus creature-type text should lex");
    let effects = parse_target_player_controls_get_subject_verb(&tokens)
        .expect("target-controlled pump parser should not error")
        .expect("target-controlled pump parser should match");
    let debug = format!("{effects:#?}");

    assert_eq!(effects.len(), 2, "{debug}");
    assert!(debug.contains("PumpAll"), "{debug}");
    assert!(debug.contains("AddAllSubtypesOfFamily"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
    assert!(debug.contains("Target") && debug.contains("Any"), "{debug}");
}

#[test]
fn target_controlled_pump_can_remove_all_creature_types() {
    let tokens = crate::lexer::lex_line(
        "Creatures target player controls get -2/-0 and lose all creature types until end of turn.",
        0,
    )
    .expect("target-controlled pump plus creature-type loss should lex");
    let effects = parse_target_player_controls_get_subject_verb(&tokens)
        .expect("target-controlled pump parser should not error")
        .expect("target-controlled pump parser should match");
    let debug = format!("{effects:#?}");

    assert_eq!(effects.len(), 2, "{debug}");
    assert!(debug.contains("PumpAll"), "{debug}");
    assert!(debug.contains("RemoveAllSubtypesOfFamily"), "{debug}");
    assert!(!debug.contains("AddAllSubtypesOfFamily"), "{debug}");
}

#[test]
fn target_controlled_pump_keeps_trailing_mana_spent_condition() {
    let tokens = crate::lexer::lex_line(
            "Creatures target player controls get +2/+0 and gain haste until end of turn if {R} was spent to cast this spell.",
            0,
        )
        .expect("conditional target-controlled pump text should lex");
    let effects = parse_target_player_controls_get_subject_verb(&tokens)
        .expect("conditional target-controlled pump parser should not error")
        .expect("conditional target-controlled pump parser should match");

    let [EffectAst::TrailingIf { predicate, effects }] = effects.as_slice() else {
        panic!("expected one trailing-if program, got {effects:#?}");
    };
    assert!(matches!(
        predicate,
        crate::cards::builders::PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(crate::mana::ManaSymbol::Red),
        }
    ));
    assert_eq!(effects.len(), 2, "{effects:#?}");
    let debug = format!("{effects:#?}");
    assert!(debug.contains("Pump"), "{debug}");
    assert!(debug.contains("GrantAbilitiesAll"), "{debug}");
    assert!(debug.contains("Haste"), "{debug}");
}

#[test]
fn result_gated_sacrificed_card_type_consult_uses_typed_traversal() {
    let tokens = crate::lexer::lex_line(
            "they reveal cards from the top of their library until they reveal a permanent card that shares a card type with the sacrificed permanent, put that card onto the battlefield, then shuffle",
            0,
        )
        .expect("consult sentence should lex");
    let effects = parse_generic_consult_reveal_until_subject_verb(&tokens)
        .expect("consult parser should not error")
        .expect("consult parser should match");
    let debug = format!("{effects:#?}");
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("SharesCardType"), "{debug}");
    assert!(debug.contains("sacrificed_0"), "{debug}");
}

#[test]
fn triggering_object_counter_total_binds_create_x_without_duplicating_condition() {
    let tokens = crate::lexer::lex_line(
            "Create X tapped 2/1 white and black Inkling creature tokens with flying, where X is the number of counters it had on it.",
            0,
        )
        .expect("counter-count token sentence should lex");
    let effect = parse_triggering_object_had_counters_create_tokens(&tokens)
        .expect("counter-count parser should not error")
        .expect("counter-count parser should match");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::CreateTokenWithMods { count, .. },
        ..
    }) = effect
    else {
        panic!("expected direct token creation for the already-separated intervening-if body");
    };
    assert!(matches!(
        count.unhinted(),
        Value::CountersOn(spec, None)
            if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")
    ));
}
