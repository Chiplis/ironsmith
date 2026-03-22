use crate::cards::builders::{CardDefinitionBuilder, CardTextError};
use crate::ids::CardId;
use crate::mana::ManaSymbol;
use crate::object::CounterType;
use crate::static_abilities::StaticAbilityId;
use crate::types::{CardType, Subtype, Supertype};

use super::{
    LexCursor, LowercaseWordView, RewriteKeywordLineKind, RewriteSemanticItem, lex_line,
    lexed_words, lower_activation_cost_cst, parse_activate_only_timing_lexed,
    parse_activation_condition_lexed, parse_activation_cost_rewrite, parse_count_word_rewrite,
    parse_cant_effect_sentence, parse_cant_effect_sentence_lexed, parse_restriction_duration,
    parse_restriction_duration_lexed,
    parse_mana_symbol_group_rewrite, parse_mana_usage_restriction_sentence_lexed,
    parse_text_with_annotations_rewrite, parse_text_with_annotations_rewrite_lowered,
    parse_triggered_times_each_turn_lexed, parse_type_line_rewrite, split_lexed_sentences,
};

#[test]
fn rewrite_lexer_tracks_spans_for_activation_lines() {
    let tokens = lex_line("{T}, Sacrifice a creature: Add {B}{B}.", 3)
        .expect("rewrite lexer should classify activation line");
    assert_eq!(tokens[0].slice, "{T}");
    assert_eq!(tokens[0].span.line, 3);
    assert_eq!(tokens[0].span.start, 0);
    assert_eq!(tokens[0].span.end, 3);
    assert!(tokens.iter().any(|token| token.slice == ":"));
}

#[test]
fn rewrite_lexer_accepts_plus_prefixed_counter_words() {
    let tokens = lex_line("Put a +1/+1 counter on target creature.", 0)
        .expect("rewrite lexer should accept +1/+1 words");
    assert!(tokens.iter().any(|token| token.slice == "+1/+1"));
}

#[test]
fn rewrite_lex_cursor_supports_peek_and_advance() {
    let tokens = lex_line("Whenever this creature attacks, draw a card.", 2)
        .expect("rewrite lexer should classify triggered line");
    let mut cursor = LexCursor::new(&tokens);
    assert_eq!(
        cursor.peek().and_then(|token| token.as_word()),
        Some("Whenever")
    );
    assert_eq!(
        cursor.peek_n(1).and_then(|token| token.as_word()),
        Some("this")
    );
    assert_eq!(
        cursor.advance().and_then(|token| token.as_word()),
        Some("Whenever")
    );
    assert_eq!(cursor.position(), 1);
    assert_eq!(
        lexed_words(cursor.remaining()).first().copied(),
        Some("this")
    );
}

#[test]
fn rewrite_sentence_splitter_respects_quotes() {
    let tokens = lex_line("Choose one. \"Draw a card.\" Create a token.", 0)
        .expect("rewrite lexer should classify modal text");
    let sentences = split_lexed_sentences(&tokens);
    let rendered = sentences
        .into_iter()
        .map(|sentence| {
            sentence
                .iter()
                .map(|token| token.slice.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rendered,
        vec!["Choose one", "\" Draw a card . \"", "Create a token"]
    );
}

#[test]
fn rewrite_sentence_splitter_ignores_single_quotes_inside_double_quotes() {
    let tokens = lex_line(
        "\"Create a 0/0 colorless Construct artifact creature token with 'This creature gets +1/+1 for each artifact you control.'\"",
        0,
    )
    .expect("rewrite lexer should classify nested quote ability text");
    let sentences = split_lexed_sentences(&tokens);
    let rendered = sentences
        .into_iter()
        .map(|sentence| {
            sentence
                .iter()
                .map(|token| token.slice.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rendered,
        vec![
            "\" Create a 0/0 colorless Construct artifact creature token with ' This creature gets +1/+1 for each artifact you control . ' \""
        ]
    );
}

#[test]
fn rewrite_lowercase_word_view_caches_lower_words_and_word_token_indices() {
    let tokens = lex_line("Activate only during your turn.", 0)
        .expect("rewrite lexer should classify restriction text");
    let words = LowercaseWordView::new(&tokens);
    assert_eq!(words.get(0), Some("activate"));
    assert_eq!(words.get(3), Some("your"));
    assert_eq!(words.token_index_for_word_index(4), Some(4));
    assert!(words.starts_with(&["activate", "only"]));
    assert!(words.contains_sequence(&["during", "your", "turn"]));
}

#[test]
fn rewrite_lexed_restriction_parsers_match_activation_trigger_and_mana_shapes() {
    let activate_only = lex_line("Activate only during your turn.", 0)
        .expect("rewrite lexer should classify activation restriction");
    let trigger_only = lex_line("This ability triggers only once each turn.", 0)
        .expect("rewrite lexer should classify trigger restriction");
    let mana_only = lex_line(
        "Spend this mana only to cast artifact spells of the chosen type and that spell can't be countered.",
        0,
    )
    .expect("rewrite lexer should classify mana restriction");

    assert_eq!(
        parse_activate_only_timing_lexed(&activate_only),
        Some(crate::ability::ActivationTiming::DuringYourTurn)
    );
    assert_eq!(
        parse_triggered_times_each_turn_lexed(&trigger_only),
        Some(1)
    );
    assert!(matches!(
        parse_mana_usage_restriction_sentence_lexed(&mana_only),
        Some(crate::ability::ManaUsageRestriction::CastSpell {
            card_types,
            subtype_requirement: Some(
                crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource
            ),
            grant_uncounterable: true,
        }) if card_types == vec![CardType::Artifact]
    ));
}

#[test]
fn rewrite_lexed_activation_condition_parser_handles_control_and_graveyard_conditions() {
    let graveyard = lex_line(
        "Activate only if there is an artifact card in your graveyard.",
        0,
    )
    .expect("rewrite lexer should classify graveyard condition");
    let control = lex_line("Activate only if you control three or more artifacts.", 0)
        .expect("rewrite lexer should classify control condition");

    assert!(matches!(
        parse_activation_condition_lexed(&graveyard),
        Some(crate::ConditionExpr::CardInYourGraveyard { card_types, subtypes })
            if card_types == vec![CardType::Artifact] && subtypes.is_empty()
    ));
    assert!(matches!(
        parse_activation_condition_lexed(&control),
        Some(crate::ConditionExpr::PlayerControlsAtLeast {
            player: crate::target::PlayerFilter::You,
            count: 3,
            ..
        })
    ));
}

#[test]
fn rewrite_lexed_spell_filter_parser_preserves_native_shape() {
    let tokens = lex_line("face-down noncreature spells", 0)
        .expect("rewrite lexer should classify spell filter text");
    let filter = super::parse_spell_filter_lexed(&tokens);

    assert_eq!(filter.face_down, Some(true));
    assert_eq!(filter.excluded_card_types, vec![CardType::Creature]);
}

#[test]
fn rewrite_lexed_value_and_permission_helpers_match_existing_semantics() {
    let count_tokens = lex_line("equal to the number of creatures", 0)
        .expect("rewrite lexer should classify count-value clause");
    let permission_tokens = lex_line("You may cast it this turn", 0)
        .expect("rewrite lexer should classify permission clause");

    assert!(matches!(
        super::value_helpers::parse_equal_to_number_of_filter_value_lexed(&count_tokens),
        Some(crate::effect::Value::Count(filter)) if filter.card_types == vec![CardType::Creature]
    ));
    assert!(matches!(
        super::permission_helpers::parse_permission_clause_spec_lexed(&permission_tokens),
        Ok(Some(super::PermissionClauseSpec::Tagged {
            player: crate::cards::builders::PlayerAst::You,
            allow_land: false,
            as_copy: false,
            without_paying_mana_cost: false,
            lifetime: super::PermissionLifetime::ThisTurn,
        }))
    ));
}

#[test]
fn rewrite_lexed_object_filters_match_legacy_simple_shapes() {
    for text in [
        "creatures you control",
        "artifact card in your graveyard",
        "nontoken artifacts",
        "noncreature spells",
    ] {
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify filter text");
        let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);

        let native = super::object_filters::parse_object_filter_lexed(&lexed, false)
            .expect("lexed object filter should parse");
        let legacy = super::object_filters::parse_object_filter(&compat, false)
            .expect("legacy object filter should parse");

        assert_eq!(format!("{native:?}"), format!("{legacy:?}"), "{text}");
    }
}

#[test]
fn rewrite_lexed_cant_sentence_matches_legacy_output() {
    let text = "Target artifact doesn't untap during its controller's next untap step.";
    let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify restriction sentence");

    let native = parse_cant_effect_sentence_lexed(&lexed)
        .expect("lexed cant sentence should parse");
    let legacy = parse_cant_effect_sentence(&compat).expect("legacy cant sentence should parse");

    assert_eq!(
        format!("{native:?}"),
        format!("{legacy:?}"),
        "lexed cant sentence should match legacy output"
    );
}

#[test]
fn rewrite_lexed_restriction_duration_matches_legacy_shapes() {
    let text = "Target creature can't attack this turn.";
    let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify restriction duration");

    let native = parse_restriction_duration_lexed(&lexed)
        .expect("lexed restriction duration should parse")
        .expect("lexed restriction duration should be present");
    let legacy = parse_restriction_duration(&compat)
        .expect("legacy restriction duration should parse")
        .expect("legacy restriction duration should be present");

    assert_eq!(native.0, legacy.0);
    assert_eq!(
        crate::cards::builders::parse_rewrite::util::words(
            &crate::cards::builders::parse_rewrite::util::compat_tokens_from_lexed(&native.1),
        ),
        crate::cards::builders::parse_rewrite::util::words(&legacy.1),
        "lexed restriction duration remainder should match legacy words"
    );
}

#[test]
fn rewrite_lexed_value_helpers_cover_offset_and_aggregate_counts() {
    let offset_tokens = lex_line("equal to the number of creatures plus 2", 0)
        .expect("rewrite lexer should classify offset count-value clause");
    let aggregate_tokens = lex_line("equal to the greatest power among creatures you control", 0)
        .expect("rewrite lexer should classify aggregate-value clause");

    assert!(matches!(
        super::value_helpers::parse_equal_to_number_of_filter_plus_or_minus_fixed_value_lexed(
            &offset_tokens
        ),
        Some(crate::effect::Value::Add(base, offset))
            if matches!(*base, crate::effect::Value::Count(_))
                && matches!(*offset, crate::effect::Value::Fixed(2))
    ));
    assert!(matches!(
        super::value_helpers::parse_equal_to_aggregate_filter_value_lexed(&aggregate_tokens),
        Some(crate::effect::Value::GreatestPower(filter))
            if filter.card_types == vec![CardType::Creature]
                && filter.controller == Some(crate::target::PlayerFilter::You)
    ));
}

#[test]
fn rewrite_lexed_permission_helpers_cover_flash_and_free_cast_grants() {
    let flash_tokens = lex_line("You may cast creature spells as though they had flash", 0)
        .expect("rewrite lexer should classify flash permission clause");
    let free_cast_tokens = lex_line(
        "You may cast creature spells from your hand without paying their mana costs",
        0,
    )
    .expect("rewrite lexer should classify free-cast permission clause");

    assert!(matches!(
        super::permission_helpers::parse_permission_clause_spec_lexed(&flash_tokens),
        Ok(Some(super::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: super::PermissionLifetime::Static,
        })) if spec == crate::grant::GrantSpec::flash_to_spells_matching(
            crate::target::ObjectFilter {
                card_types: vec![CardType::Creature],
                ..crate::target::ObjectFilter::default()
            }
        )
    ));
    assert!(matches!(
        super::permission_helpers::parse_permission_clause_spec_lexed(&free_cast_tokens),
        Ok(Some(super::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: super::PermissionLifetime::Static,
        })) if !spec.filter.has_mana_cost
            && spec.filter.card_types == vec![CardType::Creature]
            && spec.zone == crate::zone::Zone::Hand
    ));
}

#[test]
fn rewrite_lexed_keyword_line_and_static_cost_probe_work_natively() {
    let flashback_tokens = lex_line("Flashback {2}{R}", 0)
        .expect("rewrite lexer should classify flashback keyword line");
    let cost_probe_tokens = lex_line(
        "If it is night, this spell costs {2} less to cast.",
        0,
    )
    .expect("rewrite lexer should classify this-spell cost probe");

    assert!(matches!(
        super::clause_support::rewrite_parse_ability_line_lexed(&flashback_tokens),
        Some(actions) if matches!(
            actions.as_slice(),
            [crate::cards::builders::KeywordAction::MarkerText(text)]
                if text == "Flashback {2}{R}"
        )
    ));
    assert!(matches!(
        super::keyword_static::parse_if_this_spell_costs_less_to_cast_line_lexed(
            &cost_probe_tokens
        ),
        Ok(Some(ability))
            if ability.id() == crate::static_abilities::StaticAbilityId::ThisSpellCostReduction
    ));
}

#[test]
fn rewrite_lexed_keyword_line_parses_simple_native_keyword_lists() {
    let keyword_tokens = lex_line("Flying and vigilance", 0)
        .expect("rewrite lexer should classify simple keyword line");
    let numeric_tokens =
        lex_line("Ward 2", 0).expect("rewrite lexer should classify numeric keyword line");

    assert!(matches!(
        super::clause_support::rewrite_parse_ability_line_lexed(&keyword_tokens),
        Some(actions)
            if actions
                == vec![
                    crate::cards::builders::KeywordAction::Flying,
                    crate::cards::builders::KeywordAction::Vigilance,
                ]
    ));
    assert!(matches!(
        super::clause_support::rewrite_parse_ability_line_lexed(&numeric_tokens),
        Some(actions)
            if actions
                == vec![crate::cards::builders::KeywordAction::Ward(2)]
    ));
}

#[test]
fn rewrite_lexed_triggered_and_static_wrappers_work_natively() {
    let triggered_tokens = lex_line(
        "Whenever you cast an Aura, Equipment, or Vehicle spell, draw a card.",
        0,
    )
    .expect("rewrite lexer should classify triggered wrapper probe");
    let static_tokens = lex_line(
        "Activated abilities of artifacts and creatures can't be activated.",
        0,
    )
    .expect("rewrite lexer should classify static wrapper probe");

    assert!(matches!(
        super::clause_support::rewrite_parse_triggered_line_lexed(&triggered_tokens),
        Ok(crate::cards::builders::LineAst::Triggered { .. })
    ));
    assert!(matches!(
        super::clause_support::rewrite_parse_static_ability_ast_line_lexed(&static_tokens),
        Ok(Some(abilities)) if !abilities.is_empty()
    ));
}

#[test]
fn rewrite_lexed_trigger_clause_parses_common_native_shapes() {
    let dies_tokens = lex_line("another creature dies", 0)
        .expect("rewrite lexer should classify dies trigger probe");
    let upkeep_tokens = lex_line("the beginning of your upkeep", 0)
        .expect("rewrite lexer should classify upkeep trigger probe");
    let etb_tokens = lex_line("one or more goblins enter the battlefield under your control", 0)
        .expect("rewrite lexer should classify etb trigger probe");
    let spell_tokens = lex_line("you cast an aura, equipment, or vehicle spell", 0)
        .expect("rewrite lexer should classify spell-cast trigger probe");

    assert!(matches!(
        super::activation_and_restrictions::parse_trigger_clause_lexed(&dies_tokens),
        Ok(crate::cards::builders::TriggerSpec::Dies(_))
    ));
    assert!(matches!(
        super::activation_and_restrictions::parse_trigger_clause_lexed(&upkeep_tokens),
        Ok(crate::cards::builders::TriggerSpec::BeginningOfUpkeep(
            crate::target::PlayerFilter::You
        ))
    ));
    assert!(matches!(
        super::activation_and_restrictions::parse_trigger_clause_lexed(&etb_tokens),
        Ok(
            crate::cards::builders::TriggerSpec::EntersBattlefieldOneOrMore(_)
                | crate::cards::builders::TriggerSpec::EntersBattlefield(_)
        )
    ));
    assert!(matches!(
        super::activation_and_restrictions::parse_trigger_clause_lexed(&spell_tokens),
        Ok(crate::cards::builders::TriggerSpec::SpellCast { .. })
    ));
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_legacy_multisentence_followups() {
    let text =
        "Exile the top card of that player's library. You may cast it. If you don't, create a Treasure token.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify multisentence effect");
    let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);

    let legacy = super::clause_support::rewrite_parse_effect_sentences(&compat)
        .expect("legacy effect sentence parser should succeed");
    let native = super::clause_support::rewrite_parse_effect_sentences_lexed(&lexed)
        .expect("lexed effect sentence parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{legacy:?}"));
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_legacy_comma_then_chain() {
    let text = "Discard your hand, then draw four cards.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comma-then effect");
    let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);

    let legacy = super::clause_support::rewrite_parse_effect_sentences(&compat)
        .expect("legacy effect sentence parser should succeed");
    let native = super::clause_support::rewrite_parse_effect_sentences_lexed(&lexed)
        .expect("lexed effect sentence parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{legacy:?}"));
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_legacy_and_chain_split() {
    let text = "Destroy target creature and draw a card.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify and-chain effect");
    let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);

    let legacy = super::clause_support::rewrite_parse_effect_sentences(&compat)
        .expect("legacy effect sentence parser should succeed");
    let native = super::clause_support::rewrite_parse_effect_sentences_lexed(&lexed)
        .expect("lexed effect sentence parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{legacy:?}"));
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_legacy_missing_verb_damage_chain() {
    let text = "This creature deals 4 damage to target creature and 2 damage to that creature's controller.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify missing-verb damage chain");
    let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);

    let legacy = super::clause_support::rewrite_parse_effect_sentences(&compat)
        .expect("legacy missing-verb damage parser should succeed");
    let native = super::clause_support::rewrite_parse_effect_sentences_lexed(&lexed)
        .expect("lexed missing-verb damage parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{legacy:?}"));
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_legacy_missing_verb_sacrifice_chain() {
    let text = "Target player sacrifices an artifact and a land of their choice.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify missing-verb sacrifice chain");
    let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);

    let legacy = super::clause_support::rewrite_parse_effect_sentences(&compat)
        .expect("legacy missing-verb sacrifice parser should succeed");
    let native = super::clause_support::rewrite_parse_effect_sentences_lexed(&lexed)
        .expect("lexed missing-verb sacrifice parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{legacy:?}"));
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_legacy_comma_action_chain() {
    let text = "Target player sacrifices a creature, discards a card, and loses 2 life.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comma action chain");
    let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);

    let legacy = super::clause_support::rewrite_parse_effect_sentences(&compat)
        .expect("legacy comma action parser should succeed");
    let native = super::clause_support::rewrite_parse_effect_sentences_lexed(&lexed)
        .expect("lexed comma action parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{legacy:?}"));
}

fn rewrite_lexed_effect_entrypoint_matches_legacy_simple_draw_clause() {
    let text = "Draw a card.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify simple draw effect");
    let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);

    let legacy = super::clause_support::rewrite_parse_effect_sentences(&compat)
        .expect("legacy simple draw parser should succeed");
    let native = super::clause_support::rewrite_parse_effect_sentences_lexed(&lexed)
        .expect("lexed simple draw parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{legacy:?}"));
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_legacy_simple_target_clause() {
    let text = "Destroy target creature.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify simple target effect");
    let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);

    let legacy = super::clause_support::rewrite_parse_effect_sentences(&compat)
        .expect("legacy simple target parser should succeed");
    let native = super::clause_support::rewrite_parse_effect_sentences_lexed(&lexed)
        .expect("lexed simple target parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{legacy:?}"));
}

#[test]
fn rewrite_lexed_effect_entrypoint_keeps_permission_may_as_grant_not_wrapper() {
    let text = "You may play it this turn without paying its mana cost.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify permission sentence");
    let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);

    let legacy = super::clause_support::rewrite_parse_effect_sentences(&compat)
        .expect("legacy permission sentence parser should succeed");
    let native = super::clause_support::rewrite_parse_effect_sentences_lexed(&lexed)
        .expect("lexed permission sentence parser should succeed");

    let native_debug = format!("{native:?}");
    assert_eq!(native_debug, format!("{legacy:?}"));
    assert!(
        !native_debug.contains("May"),
        "permission-granting may clause should not be wrapped as a May effect: {native_debug}"
    );
}

#[test]
fn rewrite_lexed_effect_entrypoint_keeps_additional_land_play_as_permission_not_wrapper() {
    let text = "You may play an additional land this turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify land-play permission");
    let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);

    let legacy = super::clause_support::rewrite_parse_effect_sentences(&compat)
        .expect("legacy land-play permission parser should succeed");
    let native = super::clause_support::rewrite_parse_effect_sentences_lexed(&lexed)
        .expect("lexed land-play permission parser should succeed");

    let native_debug = format!("{native:?}");
    assert_eq!(native_debug, format!("{legacy:?}"));
    assert!(
        native_debug.contains("AdditionalLandPlays"),
        "expected additional land-play effect, got {native_debug}"
    );
    assert!(
        !native_debug.contains("May"),
        "land-play permission clause should not be wrapped as a May effect: {native_debug}"
    );
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_legacy_or_action_clause() {
    let text = "Destroy target creature or draw a card.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify or-action effect");
    let compat = crate::cards::builders::parse_rewrite::util::tokenize_line(text, 0);

    let legacy = super::clause_support::rewrite_parse_effect_sentences(&compat)
        .expect("legacy or-action parser should succeed");
    let native = super::clause_support::rewrite_parse_effect_sentences_lexed(&lexed)
        .expect("lexed or-action parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{legacy:?}"));
}

#[test]
fn rewrite_count_word_parser_handles_digits_and_words() {
    assert_eq!(parse_count_word_rewrite("2").expect("digit count"), 2);
    assert_eq!(parse_count_word_rewrite("three").expect("word count"), 3);
}

#[test]
fn rewrite_mana_symbol_group_parser_handles_hybrid_symbols() {
    let symbols = parse_mana_symbol_group_rewrite("{W/U}")
        .expect("rewrite parser should parse hybrid mana group");
    assert_eq!(symbols, vec![ManaSymbol::White, ManaSymbol::Blue]);
}

#[test]
fn rewrite_type_line_parser_handles_supertypes_types_and_subtypes() {
    let parsed = parse_type_line_rewrite("Legendary Creature — Elf Druid")
        .expect("rewrite type-line parser should succeed");
    assert_eq!(parsed.supertypes, vec![Supertype::Legendary]);
    assert_eq!(parsed.card_types, vec![CardType::Creature]);
    assert_eq!(parsed.subtypes, vec![Subtype::Elf, Subtype::Druid]);
}

#[test]
fn rewrite_activation_cost_parses_sacrifice_segments() {
    let cst = parse_activation_cost_rewrite("Sacrifice a creature")
        .expect("rewrite activation-cost parser should parse sacrifice segments");
    let lowered = lower_activation_cost_cst(&cst)
        .expect("rewrite sacrifice segment should lower to TotalCost");
    assert!(!lowered.is_free());

    let another = parse_activation_cost_rewrite("Sacrifice another creature")
        .expect("rewrite activation-cost parser should preserve 'another creature'");
    let rendered = another
        .segments
        .iter()
        .map(|segment| format!("{segment:?}"))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        rendered.contains("other: true"),
        "expected rewrite sacrifice CST to preserve 'another', got {rendered}"
    );
}

#[test]
fn rewrite_activation_cost_parses_energy_and_counter_variants() {
    let energy = parse_activation_cost_rewrite("Pay {E}{E}")
        .expect("rewrite activation-cost parser should parse energy payment");
    let bare_energy = parse_activation_cost_rewrite("{E}{E}")
        .expect("rewrite activation-cost parser should parse bare energy payment");
    let counter_add = parse_activation_cost_rewrite("Put a +1/+1 counter on this creature")
        .expect("rewrite parser should parse add-counter cost");
    let counter_remove = parse_activation_cost_rewrite("Remove a +1/+1 counter from this creature")
        .expect("rewrite parser should parse remove-counter cost");
    let exile_hand = parse_activation_cost_rewrite("Exile a blue card from your hand")
        .expect("rewrite parser should parse exile-from-hand cost");

    assert!(matches!(
        energy.segments.as_slice(),
        [super::ActivationCostSegmentCst::Energy(2)]
    ));
    assert!(matches!(
        bare_energy.segments.as_slice(),
        [super::ActivationCostSegmentCst::Energy(2)]
    ));
    assert!(matches!(
        counter_add.segments.as_slice(),
        [super::ActivationCostSegmentCst::PutCounters {
            counter_type: CounterType::PlusOnePlusOne,
            count: 1
        }]
    ));
    assert!(matches!(
        counter_remove.segments.as_slice(),
        [super::ActivationCostSegmentCst::RemoveCounters {
            counter_type: CounterType::PlusOnePlusOne,
            count: 1
        }]
    ));
    assert!(matches!(
        exile_hand.segments.as_slice(),
        [super::ActivationCostSegmentCst::ExileFromHand {
            count: 1,
            color_filter: Some(colors)
        }] if *colors == crate::color::ColorSet::BLUE
    ));
}

#[test]
fn rewrite_activation_cost_parses_loyalty_shorthand_without_legacy_escape_hatch() {
    let plus = parse_activation_cost_rewrite("+1")
        .expect("rewrite activation-cost parser should parse +1 loyalty shorthand");
    let minus = parse_activation_cost_rewrite("-2")
        .expect("rewrite activation-cost parser should parse -2 loyalty shorthand");
    let minus_x = parse_activation_cost_rewrite("-X")
        .expect("rewrite activation-cost parser should parse -X loyalty shorthand");
    let zero = parse_activation_cost_rewrite("0")
        .expect("rewrite activation-cost parser should parse zero loyalty shorthand");

    assert!(matches!(
        plus.segments.as_slice(),
        [super::ActivationCostSegmentCst::PutCounters {
            counter_type: CounterType::Loyalty,
            count: 1
        }]
    ));
    assert!(matches!(
        minus.segments.as_slice(),
        [super::ActivationCostSegmentCst::RemoveCounters {
            counter_type: CounterType::Loyalty,
            count: 2
        }]
    ));
    assert!(matches!(
        minus_x.segments.as_slice(),
        [super::ActivationCostSegmentCst::RemoveCountersDynamic {
            counter_type: Some(CounterType::Loyalty),
            display_x: true
        }]
    ));
    assert!(
        zero.segments.is_empty(),
        "zero loyalty shorthand should lower as a free cost"
    );
    assert!(
        lower_activation_cost_cst(&zero)
            .expect("zero loyalty shorthand should lower")
            .is_free()
    );
}

#[test]
fn rewrite_lowered_simple_card_parses() -> Result<(), CardTextError> {
    let text = "Type: Creature — Spirit\n{1}: This creature gets +1/+1 until end of turn.";
    let builder = CardDefinitionBuilder::new(CardId::new(), "Shared Spirit");
    let (definition, _) =
        parse_text_with_annotations_rewrite_lowered(builder, text.to_string(), false)?;
    assert_eq!(definition.abilities.len(), 1);
    Ok(())
}

#[test]
fn rewrite_lowered_mana_ability_preserves_fixed_mana_groups() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Shared Ring")
        .card_types(vec![CardType::Artifact]);
    let (definition, _) = parse_text_with_annotations_rewrite_lowered(
        builder,
        "{T}: Add {C}{C}.".to_string(),
        false,
    )?;
    let ability = definition
        .abilities
        .first()
        .expect("rewrite lowering should produce one ability");

    match &ability.kind {
        crate::ability::AbilityKind::Activated(activated) => {
            assert!(activated.is_mana_ability());
            assert_eq!(
                activated.mana_symbols(),
                &[ManaSymbol::Colorless, ManaSymbol::Colorless]
            );
        }
        other => panic!("expected activated mana ability, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_semantic_parse_merges_multiline_spell_when_you_do_followup() -> Result<(), CardTextError>
{
    let builder = CardDefinitionBuilder::new(CardId::new(), "Followup Variant")
        .card_types(vec![CardType::Instant]);
    let (doc, _) = parse_text_with_annotations_rewrite(
        builder,
        "Sacrifice a creature.\nWhen you do, draw two cards.".to_string(),
        false,
    )?;

    assert!(matches!(
        doc.items.as_slice(),
        [RewriteSemanticItem::Statement(_)]
    ));
    Ok(())
}

#[test]
fn rewrite_semantic_parse_marks_plumb_additional_cost_as_non_choice() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Plumb Variant")
        .card_types(vec![CardType::Instant]);
    let (doc, _) = parse_text_with_annotations_rewrite(
        builder,
        "As an additional cost to cast this spell, you may sacrifice one or more creatures. When you do, copy this spell for each creature sacrificed this way.\nYou draw a card and you lose 1 life.".to_string(),
        false,
    )?;

    assert!(matches!(
        doc.items.first(),
        Some(RewriteSemanticItem::Keyword(keyword))
            if keyword.kind == RewriteKeywordLineKind::AdditionalCost
    ));
    Ok(())
}

#[test]
fn rewrite_lowered_former_section9_cases_parse_without_fallback_text() -> Result<(), CardTextError>
{
    let cases = vec![
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Poison")
                .card_types(vec![CardType::Creature]),
            "Whenever this creature deals damage to a player, that player gets a poison counter. The player gets another poison counter at the beginning of their next upkeep unless they pay {2} before that step. (A player with ten or more poison counters loses the game.)",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Unearth")
                .card_types(vec![CardType::Artifact, CardType::Creature]),
            "Permanents you control have \"Ward—Sacrifice a permanent.\"\nEach artifact card in your graveyard has unearth {1}{B}{R}. ({1}{B}{R}: Return the card from your graveyard to the battlefield. It gains haste. Exile it at the beginning of the next end step or if it would leave the battlefield. Unearth only as a sorcery.)",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Sticker")
                .card_types(vec![CardType::Sorcery]),
            "Put an art sticker on a nonland permanent you own. Then ask a person outside the game to rate its new art on a scale from 1 to 5, where 5 is the best. When they rate the art, up to that many target creatures can't block this turn.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Can’t Block")
                .card_types(vec![CardType::Creature]),
            "This creature can't be blocked by more than one creature.\nEach creature you control with a +1/+1 counter on it can't be blocked by more than one creature.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 White Destroy")
                .card_types(vec![CardType::Sorcery]),
            "Destroy target creature if it's white. A creature destroyed this way can't be regenerated.\nDraw a card at the beginning of the next turn's upkeep.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Spent")
                .card_types(vec![CardType::Instant]),
            "Create two 1/1 white Kithkin Soldier creature tokens if {W} was spent to cast this spell. Counter up to one target creature spell if {U} was spent to cast this spell. (Do both if {W}{U} was spent.)",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Goats")
                .card_types(vec![CardType::Artifact]),
            "{T}: Add {C}.\n{4}, {T}: Create a 0/1 white Goat creature token.\n{T}, Sacrifice X Goats: Add X mana of any one color. You gain X life.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Exile Top")
                .card_types(vec![CardType::Sorcery]),
            "Shuffle your library, then exile the top four cards. You may cast any number of spells with mana value 5 or less from among them without paying their mana costs. Lands you control don't untap during your next untap step.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Cloak")
                .card_types(vec![CardType::Sorcery]),
            "Exile target nontoken creature you own and the top two cards of your library in a face-down pile, shuffle that pile, then cloak those cards. They enter tapped. (To cloak a card, put it onto the battlefield face down as a 2/2 creature with ward {2}. Turn it face up any time for its mana cost if it's a creature card.)",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Toughness")
                .card_types(vec![CardType::Instant]),
            "Destroy target creature unless its controller pays life equal to its toughness. A creature destroyed this way can't be regenerated.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Or")
                .card_types(vec![CardType::Sorcery]),
            "Destroy all lands or all creatures. Creatures destroyed this way can't be regenerated.",
        ),
        (
            CardDefinitionBuilder::new(CardId::new(), "Section 9 Nonblack")
                .card_types(vec![CardType::Sorcery]),
            "Destroy two target nonblack creatures unless either one is a color the other isn't. They can't be regenerated.",
        ),
    ];

    let mut failures = Vec::new();

    for (builder, text) in cases {
        let (definition, _) = match parse_text_with_annotations_rewrite_lowered(
            builder,
            text.to_string(),
            false,
        ) {
            Ok(parsed) => parsed,
            Err(err) => {
                failures.push(format!(
                    "former section-9 case failed to parse: {text}\n{err:?}"
                ));
                continue;
            }
        };
        let has_fallback_text = crate::ability::extract_static_abilities(&definition.abilities)
            .iter()
            .any(|ability| {
                matches!(
                    ability.id(),
                    StaticAbilityId::RuleFallbackText | StaticAbilityId::KeywordFallbackText
                )
            });
        assert!(
            !has_fallback_text,
            "former section-9 case should lower without fallback text: {text}"
        );
    }

    assert!(
        failures.is_empty(),
        "{}",
        failures.join("\n\n")
    );

    Ok(())
}
