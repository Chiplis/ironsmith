use crate::cards::builders::{CardDefinitionBuilder, CardTextError};
use crate::ids::CardId;
use crate::mana::ManaSymbol;
use crate::object::CounterType;
use crate::static_abilities::StaticAbilityId;
use crate::types::{CardType, Subtype, Supertype};
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    LexCursor, LowercaseWordView, RewriteKeywordLineKind, RewriteSemanticItem, lex_line,
    lexed_words, lower_activation_cost_cst, parse_activate_only_timing_lexed,
    parse_activation_condition_lexed, parse_activation_cost_rewrite, parse_cant_effect_sentence,
    parse_cant_effect_sentence_lexed, parse_cost_reduction_line, parse_count_word_rewrite,
    parse_effect_sentence_lexed, parse_mana_symbol_group_rewrite,
    parse_mana_usage_restriction_sentence_lexed, parse_restriction_duration,
    parse_restriction_duration_lexed, parse_text_to_semantic_document,
    parse_text_with_annotations_lowered, parse_triggered_times_each_turn_lexed,
    parse_type_line_rewrite, split_lexed_sentences,
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
fn rewrite_lowercase_word_view_normalizes_compat_style_word_shapes() {
    let tokens = lex_line("Its controller's face-down creature gets {W/U}.", 0)
        .expect("rewrite lexer should classify mixed word shapes");
    let words = LowercaseWordView::new(&tokens);

    assert_eq!(
        words.to_word_refs(),
        vec![
            "its",
            "controllers",
            "face",
            "down",
            "creature",
            "gets",
            "w/u"
        ]
    );
    assert_eq!(words.token_index_for_word_index(2), Some(2));
    assert_eq!(words.token_index_after_words(4), Some(3));
    assert_eq!(words.token_index_after_words(5), Some(4));
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
fn rewrite_lexed_object_filter_tracks_spell_caster_and_origin_zone() {
    let tokens = lex_line("enchantment spells you cast from your hand", 0)
        .expect("rewrite lexer should classify spell grant filter text");
    let filter =
        super::parse_object_filter_lexed(&tokens, false).expect("spell grant filter should parse");

    assert_eq!(filter.zone, Some(crate::zone::Zone::Hand));
    assert_eq!(filter.cast_by, Some(crate::target::PlayerFilter::You));
    assert_eq!(filter.owner, None);
    assert_eq!(filter.card_types, vec![CardType::Enchantment]);
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
fn rewrite_lexed_object_filters_match_wrapper_simple_shapes() {
    for text in [
        "creatures you control",
        "artifact card in your graveyard",
        "nontoken artifacts",
        "noncreature spells",
    ] {
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify filter text");
        let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

        let native = super::object_filters::parse_object_filter_lexed(&lexed, false)
            .expect("lexed object filter should parse");
        let wrapper = super::object_filters::parse_object_filter(&compat, false)
            .expect("wrapper object filter should parse");

        assert_eq!(format!("{native:?}"), format!("{wrapper:?}"), "{text}");
    }
}

#[test]
fn rewrite_lexed_object_filters_match_wrapper_complex_phrase_shapes() {
    let text =
        "creature card with mana value equal to the number of charge counters on this artifact";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify complex filter text");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let native = super::object_filters::parse_object_filter_lexed(&lexed, false)
        .expect("lexed object filter should parse");
    let wrapper = super::object_filters::parse_object_filter(&compat, false)
        .expect("wrapper object filter should parse");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_cant_sentence_matches_wrapper_output() {
    for text in [
        "Target artifact doesn't untap during its controller's next untap step.",
        "Target artifact does not untap during its controller's next untap step.",
    ] {
        let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify restriction sentence");

        let native =
            parse_cant_effect_sentence_lexed(&lexed).expect("lexed cant sentence should parse");
        let wrapper =
            parse_cant_effect_sentence(&compat).expect("wrapper cant sentence should parse");

        assert_eq!(format!("{native:?}"), format!("{wrapper:?}"), "{text}");
    }
}

#[test]
fn rewrite_lexed_cant_sentence_supports_next_turn_silence() {
    let text = "Each opponent can't cast instant or sorcery spells during that player's next turn.";
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify next-turn silence");

    let parsed =
        parse_cant_effect_sentence_lexed(&lexed).expect("lexed next-turn silence should parse");
    let wrapper =
        parse_cant_effect_sentence(&compat).expect("wrapper next-turn silence should parse");
    let sentence =
        super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sentence parser");

    assert!(
        parsed.is_some(),
        "expected next-turn silence helper to match"
    );
    assert!(
        wrapper.is_some(),
        "expected next-turn silence wrapper to match"
    );
    assert!(
        !sentence.is_empty(),
        "expected sentence parser to produce next-turn silence effects"
    );
}

#[test]
fn rewrite_lexed_cant_sentence_preserves_hyphenated_spell_filter_for_next_turn_silence() {
    let text = "Each opponent can't cast non-Creature spells during that player's next turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify hyphenated next-turn silence");
    let parsed =
        parse_cant_effect_sentence_lexed(&lexed).expect("lexed next-turn silence should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("excluded_card_types: [Creature]"),
        "expected non-Creature spell filter to survive parsing, got {debug}"
    );
}

#[test]
fn rewrite_parse_target_phrase_preserves_hyphenated_filter_before_random_suffix() {
    let text = "target non-Vampire creature chosen at random";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify hyphenated random target phrase");
    let target =
        super::util::parse_target_phrase(&tokens).expect("hyphenated random target should parse");
    let debug = format!("{target:?}");

    assert!(
        debug.contains("random: true"),
        "expected target to remain random, got {debug}"
    );
    assert!(
        debug.contains("card_types: [Creature]"),
        "expected creature filter in parsed target, got {debug}"
    );
    assert!(
        debug.contains("excluded_subtypes: [Vampire]"),
        "expected excluded Vampire subtype in parsed target, got {debug}"
    );
}

#[test]
fn semantic_document_supports_next_turn_silence() {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Sphinx's Decree")
        .card_types(vec![CardType::Sorcery]);

    let parsed = parse_text_to_semantic_document(
        builder,
        "Each opponent can't cast instant or sorcery spells during that player's next turn."
            .to_string(),
        false,
    );

    parsed.expect("expected semantic document parse to succeed");
}

#[test]
fn rewrite_lexed_restriction_duration_matches_wrapper_shapes() {
    let text = "Target creature can't attack this turn.";
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify restriction duration");

    let native = parse_restriction_duration_lexed(&lexed)
        .expect("lexed restriction duration should parse")
        .expect("lexed restriction duration should be present");
    let wrapper = parse_restriction_duration(&compat)
        .expect("wrapper restriction duration should parse")
        .expect("wrapper restriction duration should be present");

    assert_eq!(native.0, wrapper.0);
    let native_word_view = LowercaseWordView::new(&native.1);
    let native_words = native_word_view.to_word_refs();
    assert_eq!(
        native_words,
        crate::cards::builders::parser::util::words(&wrapper.1),
        "lexed restriction duration remainder should match wrapper words"
    );
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).expect("rewrite audit should read source directory");
    for entry in entries {
        let entry = entry.expect("rewrite audit should read directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn rewrite_runtime_sources_do_not_reintroduce_compat_token_bridges() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards/builders/parser");
    let removed_helper_names = [
        format!("{}_{}", "compat_tokens_from", "lexed"),
        format!("{}_{}", "lexed_tokens_from", "compat"),
    ];
    let mut files = Vec::new();
    collect_rust_files(&root, &mut files);

    let mut offenders = Vec::new();
    for path in files {
        if path.ends_with("tests.rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("rewrite audit should read source file");
        let relative = path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .expect("rewrite audit should relativize source path")
            .display()
            .to_string();

        if removed_helper_names
            .iter()
            .any(|helper_name| source.contains(helper_name))
        {
            offenders.push(relative);
        }
    }

    assert!(
        offenders.is_empty(),
        "compat token bridge helpers should stay removed: {}",
        offenders.join(", ")
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
    let cost_probe_tokens = lex_line("If it is night, this spell costs {2} less to cast.", 0)
        .expect("rewrite lexer should classify this-spell cost probe");

    assert!(matches!(
        super::clause_support::parse_ability_line_lexed(&flashback_tokens),
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
fn rewrite_lexed_next_spell_cascade_grants_parse_natively() {
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

    let single_effects = super::effect_sentences::parse_effect_sentence_lexed(&single_tokens)
        .expect("single next-spell cascade grant should parse");
    let dual_effects = super::effect_sentences::parse_effect_sentence_lexed(&dual_tokens)
        .expect("dual next-spell cascade grant should parse");

    assert!(matches!(
        single_effects.as_slice(),
        [crate::cards::builders::EffectAst::GrantNextSpellAbilityThisTurn { .. }]
    ));
    assert_eq!(
        dual_effects.len(),
        2,
        "expected one grant per next-spell lane"
    );
    assert!(dual_effects.iter().all(|effect| matches!(
        effect,
        crate::cards::builders::EffectAst::GrantNextSpellAbilityThisTurn { .. }
    )));
}

#[test]
fn rewrite_lexed_simple_restriction_and_free_cast_wrappers_match_wrapper_output() {
    let free_cast_text = "You may cast this spell without paying its mana cost";
    let free_cast_lexed =
        lex_line(free_cast_text, 0).expect("rewrite lexer should classify free-cast clause");
    let free_cast_compat = crate::cards::builders::parser::util::tokenize_line(free_cast_text, 0);

    let restriction_text = "Cast this spell only during combat.";
    let restriction_lexed =
        lex_line(restriction_text, 0).expect("rewrite lexer should classify cast restriction");
    let restriction_compat =
        crate::cards::builders::parser::util::tokenize_line(restriction_text, 0);

    assert_eq!(
        format!(
            "{:?}",
            super::util::parse_self_free_cast_alternative_cost_line_lexed(&free_cast_lexed)
        ),
        format!(
            "{:?}",
            super::util::parse_self_free_cast_alternative_cost_line(&free_cast_compat)
        )
    );
    assert_eq!(
        format!(
            "{:?}",
            super::util::parse_cast_this_spell_only_line_lexed(&restriction_lexed)
                .expect("lexed cast restriction should parse")
        ),
        format!(
            "{:?}",
            super::util::parse_cast_this_spell_only_line(&restriction_compat)
                .expect("wrapper cast restriction should parse")
        )
    );
}

#[test]
fn rewrite_lexed_activation_keyword_wrappers_match_wrapper_output() {
    let cycling_text = "cycling {2}";
    let cycling_lexed =
        lex_line(cycling_text, 0).expect("rewrite lexer should classify cycling line");
    let cycling_compat = crate::cards::builders::parser::util::tokenize_line(cycling_text, 0);

    let channel_text = "channel {1}{g}, discard this card: draw a card.";
    let channel_lexed =
        lex_line(channel_text, 0).expect("rewrite lexer should classify channel line");
    let channel_compat = crate::cards::builders::parser::util::tokenize_line(channel_text, 0);

    let equip_text = "equip {1}";
    let equip_lexed = lex_line(equip_text, 0).expect("rewrite lexer should classify equip line");
    let equip_compat = crate::cards::builders::parser::util::tokenize_line(equip_text, 0);

    assert_eq!(
        format!(
            "{:?}",
            super::parse_cycling_line_lexed(&cycling_lexed).expect("lexed cycling should parse")
        ),
        format!(
            "{:?}",
            super::parse_cycling_line(&cycling_compat).expect("wrapper cycling should parse")
        )
    );
    assert_eq!(
        format!(
            "{:?}",
            super::parse_channel_line_lexed(&channel_lexed).expect("lexed channel should parse")
        ),
        format!(
            "{:?}",
            super::parse_channel_line(&channel_compat).expect("wrapper channel should parse")
        )
    );
    assert_eq!(
        format!(
            "{:?}",
            super::parse_equip_line_lexed(&equip_lexed).expect("lexed equip should parse")
        ),
        format!(
            "{:?}",
            super::parse_equip_line(&equip_compat).expect("wrapper equip should parse")
        )
    );
}

#[test]
fn rewrite_lexed_optional_cost_keyword_wrappers_match_wrapper_output() {
    for text in [
        "Buyback {3}",
        "Kicker {2}",
        "Multikicker {1}",
        "Squad {2}",
        "Offspring {2}",
        "Entwine {2}",
    ] {
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify optional-cost keyword");
        let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

        let native = match text {
            t if t.starts_with("Buyback") => format!(
                "{:?}",
                super::util::parse_buyback_line_lexed(&lexed).expect("lexed buyback should parse")
            ),
            t if t.starts_with("Kicker") => format!(
                "{:?}",
                super::util::parse_kicker_line_lexed(&lexed).expect("lexed kicker should parse")
            ),
            t if t.starts_with("Multikicker") => format!(
                "{:?}",
                super::util::parse_multikicker_line_lexed(&lexed)
                    .expect("lexed multikicker should parse")
            ),
            t if t.starts_with("Squad") => format!(
                "{:?}",
                super::util::parse_squad_line_lexed(&lexed).expect("lexed squad should parse")
            ),
            t if t.starts_with("Offspring") => format!(
                "{:?}",
                super::util::parse_offspring_line_lexed(&lexed)
                    .expect("lexed offspring should parse")
            ),
            _ => format!(
                "{:?}",
                super::util::parse_entwine_line_lexed(&lexed).expect("lexed entwine should parse")
            ),
        };
        let wrapper = match text {
            t if t.starts_with("Buyback") => format!(
                "{:?}",
                super::util::parse_buyback_line(&compat).expect("wrapper buyback should parse")
            ),
            t if t.starts_with("Kicker") => format!(
                "{:?}",
                super::util::parse_kicker_line(&compat).expect("wrapper kicker should parse")
            ),
            t if t.starts_with("Multikicker") => format!(
                "{:?}",
                super::util::parse_multikicker_line(&compat)
                    .expect("wrapper multikicker should parse")
            ),
            t if t.starts_with("Squad") => format!(
                "{:?}",
                super::util::parse_squad_line(&compat).expect("wrapper squad should parse")
            ),
            t if t.starts_with("Offspring") => format!(
                "{:?}",
                super::util::parse_offspring_line(&compat).expect("wrapper offspring should parse")
            ),
            _ => format!(
                "{:?}",
                super::util::parse_entwine_line(&compat).expect("wrapper entwine should parse")
            ),
        };

        assert_eq!(native, wrapper, "{text}");
    }
}

#[test]
fn rewrite_lexed_alternative_cost_wrappers_match_wrapper_output() {
    for text in [
        "Madness {2}{R}",
        "Flashback {2}{R}",
        "Warp {1}{U}",
        "Bestow {3}{W}",
    ] {
        let lexed =
            lex_line(text, 0).expect("rewrite lexer should classify alternative-cost keyword");
        let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

        let native = match text {
            t if t.starts_with("Madness") => format!(
                "{:?}",
                super::util::parse_madness_line_lexed(&lexed).expect("lexed madness should parse")
            ),
            t if t.starts_with("Flashback") => format!(
                "{:?}",
                super::util::parse_flashback_line_lexed(&lexed)
                    .expect("lexed flashback should parse")
            ),
            t if t.starts_with("Warp") => format!(
                "{:?}",
                super::util::parse_warp_line_lexed(&lexed).expect("lexed warp should parse")
            ),
            _ => format!(
                "{:?}",
                super::util::parse_bestow_line_lexed(&lexed).expect("lexed bestow should parse")
            ),
        };
        let wrapper = match text {
            t if t.starts_with("Madness") => format!(
                "{:?}",
                super::util::parse_madness_line(&compat).expect("wrapper madness should parse")
            ),
            t if t.starts_with("Flashback") => format!(
                "{:?}",
                super::util::parse_flashback_line(&compat).expect("wrapper flashback should parse")
            ),
            t if t.starts_with("Warp") => format!(
                "{:?}",
                super::util::parse_warp_line(&compat).expect("wrapper warp should parse")
            ),
            _ => format!(
                "{:?}",
                super::util::parse_bestow_line(&compat).expect("wrapper bestow should parse")
            ),
        };

        assert_eq!(native, wrapper, "{text}");
    }
}

#[test]
fn rewrite_lexed_spell_cost_alternative_wrappers_match_wrapper_output() {
    let direct_text = "You may pay {1}{R} rather than pay this spell's mana cost.";
    let direct_lexed =
        lex_line(direct_text, 0).expect("rewrite lexer should classify direct alt-cost line");
    let direct_compat = crate::cards::builders::parser::util::tokenize_line(direct_text, 0);

    let conditional_text = "If an opponent cast two or more spells this turn, you may pay {1}{R} rather than pay this spell's mana cost.";
    let conditional_lexed = lex_line(conditional_text, 0)
        .expect("rewrite lexer should classify conditional alt-cost line");
    let conditional_compat =
        crate::cards::builders::parser::util::tokenize_line(conditional_text, 0);

    assert_eq!(
        format!(
            "{:?}",
            super::util::parse_you_may_rather_than_spell_cost_line_lexed(
                &direct_lexed,
                direct_text
            )
            .expect("lexed direct alt-cost should parse")
        ),
        format!(
            "{:?}",
            super::util::parse_you_may_rather_than_spell_cost_line(&direct_compat, direct_text)
                .expect("wrapper direct alt-cost should parse")
        )
    );
    assert_eq!(
        format!(
            "{:?}",
            super::util::parse_if_conditional_alternative_cost_line_lexed(
                &conditional_lexed,
                conditional_text
            )
            .expect("lexed conditional alt-cost should parse")
        ),
        format!(
            "{:?}",
            super::util::parse_if_conditional_alternative_cost_line(
                &conditional_compat,
                conditional_text
            )
            .expect("wrapper conditional alt-cost should parse")
        )
    );
}

#[test]
fn rewrite_lexed_remaining_keyword_wrappers_match_wrapper_output() {
    for text in [
        "Level up {2}{U}",
        "Morph {3}",
        "Megamorph {5}{G}",
        "Transmute {1}{U}{B}",
        "Reinforce 1 {2}{G}",
    ] {
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify keyword wrapper");
        let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

        let native = match text {
            t if t.starts_with("Level up") => format!(
                "{:?}",
                super::util::parse_level_up_line_lexed(&lexed)
                    .expect("lexed level up should parse")
            ),
            t if t.starts_with("Morph") || t.starts_with("Megamorph") => format!(
                "{:?}",
                super::util::parse_morph_keyword_line_lexed(&lexed)
                    .expect("lexed morph should parse")
            ),
            t if t.starts_with("Transmute") => format!(
                "{:?}",
                super::util::parse_transmute_line_lexed(&lexed)
                    .expect("lexed transmute should parse")
            ),
            _ => format!(
                "{:?}",
                super::util::parse_reinforce_line_lexed(&lexed)
                    .expect("lexed reinforce should parse")
            ),
        };
        let wrapper = match text {
            t if t.starts_with("Level up") => format!(
                "{:?}",
                super::util::parse_level_up_line(&compat).expect("wrapper level up should parse")
            ),
            t if t.starts_with("Morph") || t.starts_with("Megamorph") => format!(
                "{:?}",
                super::util::parse_morph_keyword_line(&compat).expect("wrapper morph should parse")
            ),
            t if t.starts_with("Transmute") => format!(
                "{:?}",
                super::util::parse_transmute_line(&compat).expect("wrapper transmute should parse")
            ),
            _ => format!(
                "{:?}",
                super::util::parse_reinforce_line(&compat).expect("wrapper reinforce should parse")
            ),
        };

        assert_eq!(native, wrapper, "{text}");
    }
}

#[test]
fn rewrite_lexed_number_and_additional_cost_wrappers_match_wrapper_output() {
    for text in ["X", "Three"] {
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify numeric value");
        let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

        assert_eq!(
            super::util::parse_number_or_x_value_lexed(&lexed),
            super::util::parse_number_or_x_value(&compat),
            "{text}"
        );
    }

    let text = "Sacrifice a creature or discard a card";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify additional cost options");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    assert_eq!(
        format!(
            "{:?}",
            super::util::parse_additional_cost_choice_options_lexed(&lexed)
                .expect("lexed additional cost options should parse")
        ),
        format!(
            "{:?}",
            super::util::parse_additional_cost_choice_options(&compat)
                .expect("wrapper additional cost options should parse")
        )
    );
}

#[test]
fn rewrite_lexed_search_library_sentence_matches_wrapper_output() {
    for text in [
        "Search target player's library for an artifact card, reveal it, put it into your hand, then shuffle.",
        "Discard a card, then search your library for a creature card, reveal it, put it into your hand, then shuffle.",
    ] {
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify search-library text");
        let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

        let native = super::parse_search_library_sentence_lexed(&lexed)
            .expect("lexed search-library sentence should parse");
        let wrapper = super::parse_search_library_sentence(&compat)
            .expect("wrapper search-library sentence should parse");

        assert_eq!(format!("{native:?}"), format!("{wrapper:?}"), "{text}");
    }
}

#[test]
fn rewrite_lexed_clause_and_sentence_wrapper_slices_match_wrapper_output() {
    let gain_text = "Target creature gains flying until end of turn";
    let gain_lexed =
        lex_line(gain_text, 0).expect("rewrite lexer should classify gain-ability clause");
    let gain_compat = crate::cards::builders::parser::util::tokenize_line(gain_text, 0);

    assert_eq!(
        format!(
            "{:?}",
            super::parse_simple_gain_ability_clause_lexed(&gain_lexed)
                .expect("lexed gain-ability clause should parse")
        ),
        format!(
            "{:?}",
            super::parse_simple_gain_ability_clause(&gain_compat)
                .expect("wrapper gain-ability clause should parse")
        )
    );

    for text in [
        "Return target creature card to the battlefield with a +1/+1 counter on it",
        "Put target creature card onto the battlefield with a flying counter on it",
        "Exile this creature with three time counters on it",
    ] {
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify sentence wrapper");
        let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

        let native = if text.starts_with("Return") {
            format!(
                "{:?}",
                super::parse_sentence_return_with_counters_on_it_lexed(&lexed)
                    .expect("lexed return sentence should parse")
            )
        } else if text.starts_with("Put") {
            format!(
                "{:?}",
                super::parse_sentence_put_onto_battlefield_with_counters_on_it_lexed(&lexed)
                    .expect("lexed put sentence should parse")
            )
        } else {
            format!(
                "{:?}",
                super::parse_sentence_exile_source_with_counters_lexed(&lexed)
                    .expect("lexed exile sentence should parse")
            )
        };

        let wrapper = if text.starts_with("Return") {
            format!(
                "{:?}",
                super::parse_sentence_return_with_counters_on_it(&compat)
                    .expect("wrapper return sentence should parse")
            )
        } else if text.starts_with("Put") {
            format!(
                "{:?}",
                super::parse_sentence_put_onto_battlefield_with_counters_on_it(&compat)
                    .expect("wrapper put sentence should parse")
            )
        } else {
            format!(
                "{:?}",
                super::parse_sentence_exile_source_with_counters(&compat)
                    .expect("wrapper exile sentence should parse")
            )
        };

        assert_eq!(native, wrapper, "{text}");
    }
}

#[test]
fn rewrite_lexed_keyword_line_parses_simple_native_keyword_lists() {
    let keyword_tokens = lex_line("Flying and vigilance", 0)
        .expect("rewrite lexer should classify simple keyword line");
    let numeric_tokens =
        lex_line("Ward 2", 0).expect("rewrite lexer should classify numeric keyword line");

    assert!(matches!(
        super::clause_support::parse_ability_line_lexed(&keyword_tokens),
        Some(actions)
            if actions
                == vec![
                    crate::cards::builders::KeywordAction::Flying,
                    crate::cards::builders::KeywordAction::Vigilance,
                ]
    ));
    assert!(matches!(
        super::clause_support::parse_ability_line_lexed(&numeric_tokens),
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
        super::clause_support::parse_triggered_line_lexed(&triggered_tokens),
        Ok(crate::cards::builders::LineAst::Triggered { .. })
    ));
    assert_eq!(
        format!(
            "{:?}",
            super::clause_support::parse_triggered_line_lexed(&triggered_tokens)
                .expect("triggered entrypoint should parse")
        ),
        format!(
            "{:?}",
            super::clause_support::parse_triggered_line_lexed(&triggered_tokens)
                .expect("lexed triggered wrapper should parse")
        )
    );
    assert!(matches!(
        super::clause_support::parse_static_ability_ast_line_lexed(&static_tokens),
        Ok(Some(abilities)) if !abilities.is_empty()
    ));
    assert_eq!(
        format!(
            "{:?}",
            super::keyword_static::parse_static_ability_ast_line_lexed(&static_tokens)
                .expect("static entrypoint should parse")
        ),
        format!(
            "{:?}",
            super::clause_support::parse_static_ability_ast_line_lexed(&static_tokens)
                .expect("lexed static wrapper should parse")
        )
    );
}

#[test]
fn rewrite_lexed_triggered_line_supports_tivit_vote_trigger_body() {
    let triggered_tokens = lex_line(
        "Whenever this creature enters the battlefield or deals combat damage to a player, starting with you, each player votes for evidence or bribery. For each evidence vote, investigate. For each bribery vote, create a Treasure token. You may vote an additional time.",
        0,
    )
    .expect("rewrite lexer should classify tivit trigger probe");

    let parsed = super::clause_support::parse_triggered_line_lexed(&triggered_tokens);
    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::LineAst::Triggered { .. })
        ),
        "{parsed:?}"
    );
}

#[test]
fn rewrite_lexed_trigger_clause_parses_common_native_shapes() {
    let dies_tokens = lex_line("another creature dies", 0)
        .expect("rewrite lexer should classify dies trigger probe");
    let upkeep_tokens = lex_line("the beginning of your upkeep", 0)
        .expect("rewrite lexer should classify upkeep trigger probe");
    let etb_tokens = lex_line(
        "one or more goblins enter the battlefield under your control",
        0,
    )
    .expect("rewrite lexer should classify etb trigger probe");
    let spell_tokens = lex_line("you cast an aura, equipment, or vehicle spell", 0)
        .expect("rewrite lexer should classify spell-cast trigger probe");
    let counter_tokens = lex_line("you put one or more -1/-1 counters on a creature", 0)
        .expect("rewrite lexer should classify counter trigger probe");
    let graveyard_tokens = lex_line(
        "a nontoken creature is put into your graveyard from the battlefield",
        0,
    )
    .expect("rewrite lexer should classify graveyard trigger probe");
    let second_main_tokens = lex_line("the beginning of your second main phase", 0)
        .expect("rewrite lexer should classify second-main trigger probe");

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
    let counter = super::activation_and_restrictions::parse_trigger_clause_lexed(&counter_tokens);
    assert!(
        matches!(
            counter,
            Ok(crate::cards::builders::TriggerSpec::CounterPutOn {
                one_or_more: true,
                ..
            })
        ),
        "{counter:?}"
    );
    let graveyard =
        super::activation_and_restrictions::parse_trigger_clause_lexed(&graveyard_tokens);
    assert!(
        matches!(
            graveyard,
            Ok(crate::cards::builders::TriggerSpec::PutIntoGraveyardFromZone { .. })
        ),
        "{graveyard:?}"
    );
    assert!(matches!(
        super::activation_and_restrictions::parse_trigger_clause_lexed(&second_main_tokens),
        Ok(
            crate::cards::builders::TriggerSpec::BeginningOfPostcombatMain(
                crate::target::PlayerFilter::You
            )
        )
    ));
}

#[test]
fn rewrite_lexed_trigger_clause_tail_branches_match_wrapper_output() {
    for text in [
        "one or more Goblins attack",
        "another creature blocks",
        "this or another creature dies",
        "the creature it haunts dies",
    ] {
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify trigger tail probe");
        let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

        let native = super::activation_and_restrictions::parse_trigger_clause_lexed(&lexed);
        let wrapper = super::activation_and_restrictions::parse_trigger_clause_lexed(&compat);

        assert_eq!(format!("{native:?}"), format!("{wrapper:?}"), "{text}");
    }
}

#[test]
fn rewrite_lexed_trigger_clause_unsupported_tail_matches_wrapper_error() {
    let text = "this weird thing dies";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify unsupported trigger");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let native = super::activation_and_restrictions::parse_trigger_clause_lexed(&lexed)
        .expect_err("lexed trigger clause should reject unsupported tail");
    let wrapper = super::activation_and_restrictions::parse_trigger_clause_lexed(&compat)
        .expect_err("wrapper trigger clause should reject unsupported tail");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_trigger_clause_etb_edge_cases_match_wrapper_output() {
    for text in [
        "another creature enters the battlefield and whenever it attacks",
        "another creature enters the battlefield and attacks",
        "enters tapped",
        "another creature enters the battlefield and blocks",
    ] {
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify etb edge-case trigger");
        let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

        let native = super::activation_and_restrictions::parse_trigger_clause_lexed(&lexed);
        let wrapper = super::activation_and_restrictions::parse_trigger_clause_lexed(&compat);

        assert_eq!(format!("{native:?}"), format!("{wrapper:?}"), "{text}");
    }
}

#[test]
fn rewrite_lexed_triggered_line_handles_punctuation_before_enter_verb() {
    let text = "Whenever one or more noncreature, nonland permanents you control enter, put a +1/+1 counter on target creature you control.";
    let tokens = lex_line(text, 0)
        .expect("rewrite lexer should classify comma-separated enter trigger line");

    let parsed = super::clause_support::parse_triggered_line_lexed(&tokens);

    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::LineAst::Triggered { .. })
        ),
        "{parsed:?}"
    );
}

#[test]
fn rewrite_lexed_triggered_line_parses_state_trigger_condition() {
    let text = "When you control no Swamps, sacrifice this creature.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify state trigger line");

    let parsed = super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("state-triggered line should parse");

    match parsed {
        crate::cards::builders::LineAst::Triggered { trigger, .. } => {
            assert!(
                matches!(
                    trigger,
                    crate::cards::builders::TriggerSpec::StateBased { .. }
                ),
                "expected state trigger, got {trigger:?}"
            );
        }
        other => panic!("expected triggered line, got {other:?}"),
    }
}

#[test]
fn rewrite_lexed_triggered_line_lifts_intervening_if_with_multisentence_body() {
    let text = "At the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the rest on the bottom of your library in a random order.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify postcombat intervening-if trigger");

    let parsed = super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("intervening-if trigger should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("BeginningOfPostcombatMain"), "{debug}");
    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_wrapper_multisentence_followups() {
    let text = "Exile the top card of that player's library. You may cast it. If you don't, create a Treasure token.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify multisentence effect");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper effect sentence parser should succeed");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed effect sentence parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_wrapper_comma_then_chain() {
    let text = "Discard your hand, then draw four cards.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comma-then effect");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper effect sentence parser should succeed");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed effect sentence parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_effect_sentence_matches_wrapper_conditional_dispatch() {
    let text = "If you control an artifact, draw a card.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify conditional sentence");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper conditional sentence should parse");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed conditional sentence should parse");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_predicate_parser_matches_wrapper_output() {
    let text = "it's your turn";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify predicate text");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let native = super::parse_predicate_lexed(&lexed).expect("lexed predicate should parse");
    let wrapper =
        super::conditionals::parse_predicate(&compat).expect("wrapper predicate should parse");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_predicate_parser_matches_wrapper_for_this_spell_cast_from_zone() {
    let text = "this spell was cast from a graveyard";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify graveyard-cast predicate");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let native = super::parse_predicate_lexed(&lexed).expect("lexed predicate should parse");
    let wrapper =
        super::conditionals::parse_predicate(&compat).expect("wrapper predicate should parse");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
    assert!(
        format!("{native:?}").contains("ThisSpellWasCastFromZone(Graveyard)"),
        "expected graveyard-cast predicate AST, got {native:?}"
    );
}

#[test]
fn rewrite_lexed_effect_sentence_matches_wrapper_pre_diagnostic_clause_helpers() {
    for text in [
        "The next time a red source of your choice would deal damage to you this turn, prevent that damage.",
        "Double target creature's power until end of turn.",
    ] {
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify clause helper probe");
        let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

        let wrapper = parse_effect_sentence_lexed(&compat)
            .expect("wrapper clause helper sentence should parse");
        let native =
            parse_effect_sentence_lexed(&lexed).expect("lexed clause helper sentence should parse");

        assert_eq!(format!("{native:?}"), format!("{wrapper:?}"), "{text}");
    }
}

#[test]
fn rewrite_lexed_effect_sentence_unsupported_diagnostic_matches_wrapper_error() {
    let text = "The Ring tempts you.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify ring sentence");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let native =
        parse_effect_sentence_lexed(&lexed).expect("lexed sentence should parse ring clause");
    let wrapper =
        parse_effect_sentence_lexed(&compat).expect("wrapper sentence should parse ring clause");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_effect_sentence_matches_wrapper_where_x_clause() {
    let text = "Target creature gets +X/+0 until end of turn, where X is its power.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify where-x sentence");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let native = parse_effect_sentence_lexed(&lexed).expect("lexed where-x sentence should parse");
    let wrapper =
        parse_effect_sentence_lexed(&compat).expect("wrapper where-x sentence should parse");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_effect_sentence_matches_wrapper_sacrifice_land_clause() {
    let text = "Sacrifice a land.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify sacrifice sentence");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper sacrifice sentence should parse");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed sacrifice sentence should parse");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_effect_sentence_matches_wrapper_sacrifice_all_non_ogres_clause() {
    let text = "Sacrifice all non-Ogre creatures you control.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify sacrifice-all sentence");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper sacrifice-all sentence should parse");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed sacrifice-all sentence should parse");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_trigger_clause_supports_this_creature_leaves_battlefield() {
    let tokens = lex_line("this creature leaves the battlefield", 0)
        .expect("rewrite lexer should classify leaves-the-battlefield trigger");

    let parsed = super::activation_and_restrictions::parse_trigger_clause_lexed(&tokens)
        .expect("lexed leaves-the-battlefield trigger should parse");

    assert_eq!(
        format!("{parsed:?}"),
        format!(
            "{:?}",
            crate::cards::builders::TriggerSpec::ThisLeavesBattlefield
        )
    );
}

#[test]
fn rewrite_lexed_triggered_line_supports_leave_battlefield_sacrifice_land() {
    let tokens = lex_line("When this leaves the battlefield, sacrifice a land.", 0)
        .expect("rewrite lexer should classify leave-battlefield sacrifice line");

    let parsed = super::clause_support::parse_triggered_line_lexed(&tokens);

    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::LineAst::Triggered { .. })
        ),
        "{parsed:?}"
    );
}

#[test]
fn rewrite_lexed_triggered_line_supports_leave_battlefield_sacrifice_all_non_ogres() {
    let tokens = lex_line(
        "When this creature leaves the battlefield, sacrifice all non-Ogre creatures you control.",
        0,
    )
    .expect("rewrite lexer should classify leave-battlefield sacrifice-all line");

    let parsed = super::clause_support::parse_triggered_line_lexed(&tokens);

    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::LineAst::Triggered { .. })
        ),
        "{parsed:?}"
    );
}

#[test]
fn rewrite_lexed_effect_sentence_supports_labeled_spent_to_cast_conditional() {
    let text =
        "Adamant — If at least three blue mana was spent to cast this spell, create a Food token.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify labeled spent-to-cast sentence");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed);

    assert!(parsed.is_ok(), "{parsed:?}");
}

#[test]
fn rewrite_lexed_effect_sentence_supports_unlabeled_spent_to_cast_conditional() {
    let text = "If at least three blue mana was spent to cast this spell, create a Food token.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify unlabeled spent-to-cast sentence");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed);

    assert!(parsed.is_ok(), "{parsed:?}");
}

#[test]
fn rewrite_lexed_conditional_parser_supports_spent_to_cast_conditional_directly() {
    let text = "If at least three blue mana was spent to cast this spell, create a Food token.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify unlabeled spent-to-cast sentence");

    let parsed = super::effect_sentences::parse_conditional_sentence_lexed(&lexed);

    assert!(parsed.is_ok(), "{parsed:?}");
}

#[test]
fn rewrite_lexed_effect_sentence_supports_delayed_this_turn_trigger_via_lexed_trigger_parser() {
    let lexed = lex_line("When this creature dies this turn, exile this creature.", 0)
        .expect("rewrite lexer should classify delayed this-turn trigger sentence");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("lexed delayed this-turn trigger sentence should parse");
    let wrapper = parse_effect_sentence_lexed(&lexed)
        .expect("wrapper delayed this-turn trigger sentence should parse");
    let debug = format!("{parsed:?}");

    assert_eq!(format!("{wrapper:?}"), debug);
    assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
    assert!(debug.contains("ThisDies"), "{debug}");
    assert!(debug.contains("Exile"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sentence_preserves_conditional_for_leading_instead_followup() {
    let text = "If it's a Human, instead it gets +3/+3 and gains indestructible until end of turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify leading-instead conditional");

    let parsed = parse_effect_sentence_lexed(&lexed).expect("leading-instead conditional");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Conditional"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_preserves_for_each_player_doesnt_predicate() {
    let text = "Each player discards a card. Then each player who didn't discard a creature card this way loses 4 life.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify for-each-player-doesnt sequence");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ForEachPlayerDoesNot"), "{debug}");
    assert!(debug.contains("PlayerTaggedObjectMatches"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_builds_self_replacement_for_return_followup() {
    let text = "Return target creature card from your graveyard to your hand. If you gained 7 or more life this turn, return that card to the battlefield instead.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify return followup");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_builds_self_replacement_for_damage_followup() {
    let text = "This creature deals 1 damage to any target. If that land is a Mountain, this creature deals 2 damage instead.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify damage followup");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_builds_self_replacement_for_toxic_followup() {
    let text = "Target creature you control gets +1/+1 until end of turn. If that creature has toxic, instead it gets +2/+2 until end of turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify toxic followup");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_builds_self_replacement_for_creatures_died_count_followup() {
    let text = "If a creature died this turn, you draw a card and you lose 1 life. If seven or more creatures died this turn, instead you draw seven cards and you lose 7 life.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify died-count followup");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_builds_self_replacement_for_full_party_followup() {
    let text = "Creatures you control get +1/+0 until end of turn. If you have a full party, creatures you control get +3/+0 until end of turn instead.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify full-party followup");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("SelfReplacement"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_parses_consult_hand_bottom_family() {
    let text = "Reveal cards from the top of your library until you reveal an artifact card. Put that card into your hand and the rest on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify consult-hand-bottom text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("MoveToZone"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_effect_sequence_parses_prefixed_consult_sequence() {
    let text = "Draw a card. Reveal cards from the top of your library until you reveal an artifact card. Put that card into your hand and the rest on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify prefixed consult text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Draw"), "{debug}");
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_effect_sequence_keeps_consult_cast_bottom_family_parseable() {
    let text = "Exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost. Put all cards exiled this way that weren't cast this way on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify consult-cast-bottom text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_effect_sequence_keeps_reveal_consult_cast_bottom_family_parseable() {
    let text = "Reveal cards from the top of your library until you reveal a nonland card. You may cast that card without paying its mana cost. Then put all revealed cards not cast this way on the bottom of your library in a random order.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify reveal consult-cast-bottom text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_effect_sequence_parses_target_opponent_consult_until_eot_cast() {
    let text = "Target opponent exiles cards from the top of their library until they exile a nonland card. Until end of turn, you may cast that card without paying its mana cost.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify target-opponent consult text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("TargetOpponent"), "{debug}");
    assert!(debug.contains("GrantPlayTaggedUntilEndOfTurn"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_parses_target_opponent_consult_cast_bottom_family() {
    let text = "Target opponent exiles cards from the top of their library until they exile an instant or sorcery card. You may cast that card without paying its mana cost. Then put the exiled cards that weren't cast this way on the bottom of that library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify chaos-wand consult text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn rewrite_lexed_effect_sequence_parses_consult_dynamic_mana_value_gate() {
    let text = "Exile cards from the top of your library until you exile a nonland card. You may cast the exiled card without paying its mana cost if it's a spell with mana value less than or equal to this's power. Put the exiled cards not cast this way on the bottom of your library in a random order.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify dynamic consult gate");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("mana_value: Some(LessThanOrEqualExpr"),
        "{debug}"
    );
    assert!(debug.contains("SourcePower"), "{debug}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("MoveToZone"), "{debug}");
}

#[test]
fn rewrite_lexed_effect_sequence_parses_tainted_pact_loop() {
    let text = "Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify tainted pact text");

    let parsed = super::clause_support::parse_effect_sentences_lexed(&lexed).expect("sequence");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("RepeatProcess"), "{debug}");
    assert!(debug.contains("ExileTopOfLibrary"), "{debug}");
    assert!(debug.contains("MayMoveToZone"), "{debug}");
}

#[test]
fn rewrite_semantic_parse_supports_adamant_spent_to_cast_statement_line()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Adamant Variant")
        .card_types(vec![CardType::Sorcery]);
    let (doc, _) = parse_text_to_semantic_document(
        builder,
        "Adamant — If at least three blue mana was spent to cast this spell, create a Food token."
            .to_string(),
        false,
    )?;

    assert!(matches!(
        doc.items.as_slice(),
        [RewriteSemanticItem::Statement(_)]
    ));
    Ok(())
}

#[test]
fn rewrite_lowered_supports_adamant_spent_to_cast_statement_line() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Adamant Variant")
        .card_types(vec![CardType::Sorcery]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "Adamant — If at least three blue mana was spent to cast this spell, create a Food token."
            .to_string(),
        false,
    )?;

    let debug = format!("{definition:#?}");
    assert!(debug.contains("ManaSpentToCastThisSpellAtLeast"), "{debug}");
    assert!(debug.contains("CreateTokenEffect"), "{debug}");
    Ok(())
}

#[test]
fn rewrite_lexed_effect_sentence_supports_radiance_shared_color_fanout() {
    let text = "Radiance — Target creature and each other creature that shares a color with it gain haste until end of turn.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify labeled radiance fanout sentence");

    let stripped = crate::cards::builders::parse_effect_sentence_lexed(
        lexed
            .split(|token| {
                matches!(
                    token.kind,
                    super::lexer::TokenKind::Dash | super::lexer::TokenKind::EmDash
                )
            })
            .nth(1)
            .expect("labeled sentence should contain body after dash"),
    )
    .expect("rewrite effect sentence parser should support radiance fanout");

    let parsed = parse_effect_sentence_lexed(&lexed)
        .expect("rewrite effect sentence parser should support radiance fanout");
    let direct = crate::cards::builders::parse_shared_color_target_fanout_sentence(
        lexed
            .split(|token| {
                matches!(
                    token.kind,
                    super::lexer::TokenKind::Dash | super::lexer::TokenKind::EmDash
                )
            })
            .nth(1)
            .expect("labeled sentence should contain body after dash"),
    )
    .expect("shared-color primitive should not error");
    let mut lowered_body = lexed
        .split(|token| {
            matches!(
                token.kind,
                super::lexer::TokenKind::Dash | super::lexer::TokenKind::EmDash
            )
        })
        .nth(1)
        .expect("labeled sentence should contain body after dash")
        .to_vec();
    for token in &mut lowered_body {
        if let Some(word) = token.word_mut() {
            *word = word.to_ascii_lowercase();
        }
    }
    let lowered_direct =
        crate::cards::builders::parse_shared_color_target_fanout_sentence(&lowered_body)
            .expect("lowered shared-color primitive should not error");
    let debug = format!("{parsed:?}");
    let direct_debug = format!("{direct:?}");
    let lowered_direct_debug = format!("{lowered_direct:?}");
    let stripped_debug = format!("{stripped:?}");

    assert!(
        direct_debug.contains("GrantAbilitiesAll"),
        "expected direct shared-color primitive to build fanout grant effect, got {direct_debug}"
    );
    assert!(
        direct_debug.contains("SharesColorWithTagged"),
        "expected direct shared-color primitive to keep shared-color tagged constraint, got {direct_debug}"
    );
    assert!(
        lowered_direct_debug.contains("GrantAbilitiesAll"),
        "expected lowered shared-color primitive to build fanout grant effect, got {lowered_direct_debug}"
    );
    assert!(
        stripped_debug.contains("GrantAbilitiesAll"),
        "expected stripped sentence parser to preserve fanout grant effect, got {stripped_debug}"
    );
    assert!(
        debug.contains("GrantAbilitiesAll"),
        "expected labeled sentence parser to preserve fanout grant effect, got {debug}"
    );
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_wrapper_and_chain_split() {
    let text = "Destroy target creature and draw a card.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify and-chain effect");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper effect sentence parser should succeed");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed effect sentence parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_wrapper_missing_verb_damage_chain() {
    let text = "This creature deals 4 damage to target creature and 2 damage to that creature's controller.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify missing-verb damage chain");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper missing-verb damage parser should succeed");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed missing-verb damage parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_wrapper_missing_verb_sacrifice_chain() {
    let text = "Target player sacrifices an artifact and a land of their choice.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify missing-verb sacrifice chain");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper missing-verb sacrifice parser should succeed");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed missing-verb sacrifice parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_effect_sentence_preserves_non_vampire_sacrifice_filter() {
    let text = "Each player sacrifices a non-Vampire creature of their choice.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify non-Vampire sacrifice sentence");
    let effects = parse_effect_sentence_lexed(&lexed)
        .expect("lexed non-Vampire sacrifice sentence should parse");
    let debug = format!("{effects:?}");

    assert!(
        debug.contains("card_types: [Creature]"),
        "expected creature filter in parsed effect, got {debug}"
    );
    assert!(
        debug.contains("excluded_subtypes: [Vampire]"),
        "expected excluded Vampire subtype in parsed effect, got {debug}"
    );
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_wrapper_comma_action_chain() {
    let text = "Target player sacrifices a creature, discards a card, and loses 2 life.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comma action chain");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper comma action parser should succeed");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed comma action parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_wrapper_simple_draw_clause() {
    let text = "Draw a card.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify simple draw effect");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper simple draw parser should succeed");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed simple draw parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_cost_reduction_line_rejects_unmodeled_activate_if_condition() {
    let tokens = lex_line(
        "this ability costs 1 less to activate if you control an artifact.",
        0,
    )
    .expect("rewrite lexer should classify activated cost reduction");
    let err = parse_cost_reduction_line(&tokens)
        .expect_err("unmodeled activated cost reduction condition should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported activated-ability cost reduction condition"),
        "expected explicit unsupported cost reduction condition, got {message}"
    );
}

#[test]
fn rewrite_lexed_effect_entrypoint_matches_wrapper_simple_target_clause() {
    let text = "Destroy target creature.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify simple target effect");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper simple target parser should succeed");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed simple target parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_lexed_effect_entrypoint_keeps_permission_may_as_grant_not_wrapper() {
    let text = "You may play it this turn without paying its mana cost.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify permission sentence");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper permission sentence parser should succeed");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed permission sentence parser should succeed");

    let native_debug = format!("{native:?}");
    assert_eq!(native_debug, format!("{wrapper:?}"));
    assert!(
        !native_debug.contains("May"),
        "permission-granting may clause should not be wrapped as a May effect: {native_debug}"
    );
}

#[test]
fn rewrite_lexed_effect_entrypoint_keeps_additional_land_play_as_permission_not_wrapper() {
    let text = "You may play an additional land this turn.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify land-play permission");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper land-play permission parser should succeed");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed land-play permission parser should succeed");

    let native_debug = format!("{native:?}");
    assert_eq!(native_debug, format!("{wrapper:?}"));
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
fn rewrite_lexed_effect_entrypoint_matches_wrapper_or_action_clause() {
    let text = "Destroy target creature or draw a card.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify or-action effect");
    let compat = crate::cards::builders::parser::util::tokenize_line(text, 0);

    let wrapper = super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper or-action parser should succeed");
    let native = super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed or-action parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
fn rewrite_count_word_parser_handles_digits_and_words() {
    assert_eq!(parse_count_word_rewrite("2").expect("digit count"), 2);
    assert_eq!(parse_count_word_rewrite("three").expect("word count"), 3);
}

#[test]
fn rewrite_mana_symbol_group_parser_handles_hybrid_symbols() {
    let symbols =
        parse_mana_symbol_group_rewrite("{W/U}").expect("parser should parse hybrid mana group");
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
        .expect("parser should parse add-counter cost");
    let counter_remove = parse_activation_cost_rewrite("Remove a +1/+1 counter from this creature")
        .expect("parser should parse remove-counter cost");
    let exile_hand = parse_activation_cost_rewrite("Exile a blue card from your hand")
        .expect("parser should parse exile-from-hand cost");

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
fn rewrite_activation_cost_parses_loyalty_shorthand_without_wrapper_escape_hatch() {
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
    let (definition, _) = parse_text_with_annotations_lowered(builder, text.to_string(), false)?;
    assert_eq!(definition.abilities.len(), 1);
    Ok(())
}

#[test]
fn rewrite_lowered_mana_ability_preserves_fixed_mana_groups() -> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Shared Ring")
        .card_types(vec![CardType::Artifact]);
    let (definition, _) =
        parse_text_with_annotations_lowered(builder, "{T}: Add {C}{C}.".to_string(), false)?;
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
    let (doc, _) = parse_text_to_semantic_document(
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
    let (doc, _) = parse_text_to_semantic_document(
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
        let (definition, _) =
            match parse_text_with_annotations_lowered(builder, text.to_string(), false) {
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

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));

    Ok(())
}

#[test]
fn rewrite_preprocess_expands_same_is_true_trigger_chain() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Thunderous Orator Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature attacks, it gains flying until end of turn if you control a creature with flying. The same is true for first strike and vigilance.",
        )
        .expect("same-is-true trigger chain should parse");

    let rendered = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("whenever this creature attacks, it gains flying until end of turn"),
        "expected flying branch to remain, got {rendered}"
    );
    assert!(
        rendered
            .contains("whenever this creature attacks, it gains first strike until end of turn")
            && rendered
                .contains("whenever this creature attacks, it gains vigilance until end of turn"),
        "expected remaining borrowed keyword branches, got {rendered}"
    );
}

#[test]
fn rewrite_preprocess_expands_same_is_true_static_graveyard_chain() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cairn Wanderer Variant")
        .parse_text(
            "As long as a creature card with flying is in a graveyard, this creature has flying. The same is true for first strike and vigilance.",
        )
        .expect("same-is-true static graveyard chain should parse");

    let rendered = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("as long as there is a creature card with flying in a graveyard")
            && rendered.contains("this creature has flying"),
        "expected flying graveyard condition to be normalized, got {rendered}"
    );
    assert!(
        rendered.contains("as long as there is a creature card with first strike in a graveyard")
            && rendered.contains("this creature has first strike")
            && rendered
                .contains("as long as there is a creature card with vigilance in a graveyard")
            && rendered.contains("this creature has vigilance"),
        "expected same-is-true graveyard branches to expand, got {rendered}"
    );
}

#[test]
fn rewrite_preprocess_expands_same_is_true_static_exile_chain() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Urborg Scavengers Variant")
        .parse_text(
            "This creature has flying as long as a card exiled with it has flying. The same is true for trample and vigilance.",
        )
        .expect("same-is-true exile chain should parse");

    let rendered = crate::compiled_text::compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "this creature has flying as long as there is a card exiled with it with flying"
        ),
        "expected exile-linked flying condition to be normalized, got {rendered}"
    );
    assert!(
        rendered.contains(
            "this creature has trample as long as there is a card exiled with it with trample"
        ) && rendered.contains(
            "this creature has vigilance as long as there is a card exiled with it with vigilance"
        ),
        "expected same-is-true exile branches to expand, got {rendered}"
    );
}
