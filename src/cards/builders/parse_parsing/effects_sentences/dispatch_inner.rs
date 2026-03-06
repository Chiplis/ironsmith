use super::*;

type SentenceRejectPredicate = fn(&[&str], &[Token]) -> bool;

#[derive(Clone, Copy)]
struct SentenceRejectRule {
    id: &'static str,
    heads: &'static [&'static str],
    message: &'static str,
    predicate: SentenceRejectPredicate,
}

const SENTENCE_REJECT_RULES: [SentenceRejectRule; 31] = [
    SentenceRejectRule {
        id: "each-player-lose-discard-sacrifice-chain",
        heads: &["each"],
        message: "unsupported each-player lose/discard/sacrifice chain clause",
        predicate: sentence_has_each_player_lose_discard_sacrifice_chain,
    },
    SentenceRejectRule {
        id: "each-player-exile-sacrifice-return-this-way",
        heads: &["each"],
        message: "unsupported each-player exile/sacrifice/return-this-way clause",
        predicate: sentence_has_each_player_exile_sacrifice_return_exiled_clause,
    },
    SentenceRejectRule {
        id: "lose-all-abilities-with-becomes",
        heads: &["target", "that", "it", "this", "creatures"],
        message: "unsupported loses-all-abilities with becomes clause",
        predicate: sentence_has_loses_all_abilities_with_becomes_clause,
    },
    SentenceRejectRule {
        id: "spent-to-cast-conditional",
        heads: &["if", "unless", "when", "as"],
        message: "unsupported spent-to-cast conditional clause",
        predicate: sentence_has_spent_to_cast_this_spell_without_condition,
    },
    SentenceRejectRule {
        id: "would-enter-instead",
        heads: &["if", "that", "it", "this"],
        message: "unsupported would-enter replacement clause",
        predicate: sentence_has_would_enter_instead_replacement_clause,
    },
    SentenceRejectRule {
        id: "different-mana-value-constraint",
        heads: &[],
        message: "unsupported different-mana-value constraint clause",
        predicate: sentence_has_different_mana_value_constraint,
    },
    SentenceRejectRule {
        id: "most-common-color-constraint",
        heads: &["choose", "destroy", "exile", "return"],
        message: "unsupported most-common-color constraint clause",
        predicate: sentence_has_most_common_color_constraint,
    },
    SentenceRejectRule {
        id: "power-vs-count-constraint",
        heads: &["if", "target", "destroy", "exile", "return"],
        message: "unsupported power-vs-count conditional clause",
        predicate: sentence_has_power_vs_count_constraint,
    },
    SentenceRejectRule {
        id: "put-into-graveyards-from-battlefield-this-turn",
        heads: &["for", "choose", "target", "destroy"],
        message: "unsupported put-into-graveyards-from-battlefield count clause",
        predicate: sentence_has_put_into_graveyards_from_battlefield_this_turn,
    },
    SentenceRejectRule {
        id: "phase-out-until-leaves",
        heads: &["phase", "target", "it", "that"],
        message: "unsupported phase-out-until-leaves clause",
        predicate: sentence_has_phase_out_until_leaves_clause,
    },
    SentenceRejectRule {
        id: "investigate-for-each",
        heads: &["investigate", "for"],
        message: "unsupported investigate-for-each clause",
        predicate: sentence_has_unsupported_investigate_for_each_clause,
    },
    SentenceRejectRule {
        id: "same-name-as-another-in-hand",
        heads: &["target", "choose", "discard"],
        message: "unsupported same-name-as-another-in-hand discard clause",
        predicate: sentence_has_same_name_as_another_in_hand_clause,
    },
    SentenceRejectRule {
        id: "for-each-mana-from-spent",
        heads: &["for"],
        message: "unsupported for-each-mana-from-spent clause",
        predicate: sentence_has_for_each_mana_from_spent_to_cast_clause,
    },
    SentenceRejectRule {
        id: "when-you-sacrifice-this-way",
        heads: &["when"],
        message: "unsupported when-you-sacrifice-this-way clause",
        predicate: sentence_has_when_you_sacrifice_this_way_clause,
    },
    SentenceRejectRule {
        id: "sacrifice-any-number-then-draw-that-many",
        heads: &["sacrifice", "each", "target", "you"],
        message: "unsupported sacrifice-any-number-then-draw-that-many clause",
        predicate: sentence_has_sacrifice_any_number_then_draw_that_many_clause,
    },
    SentenceRejectRule {
        id: "greatest-mana-value",
        heads: &["choose", "destroy", "exile", "return"],
        message: "unsupported greatest-mana-value selection clause",
        predicate: sentence_has_greatest_mana_value_clause,
    },
    SentenceRejectRule {
        id: "least-power-among-creatures",
        heads: &["choose", "destroy", "exile", "return"],
        message: "unsupported least-power-among-creatures selection clause",
        predicate: sentence_has_least_power_among_creatures_clause,
    },
    SentenceRejectRule {
        id: "villainous-choice",
        heads: &["villainous"],
        message: "unsupported villainous-choice clause",
        predicate: sentence_has_villainous_choice_clause,
    },
    SentenceRejectRule {
        id: "divided-evenly",
        heads: &["divide", "deals", "deal", "distribute"],
        message: "unsupported divided-evenly damage clause",
        predicate: sentence_has_divided_evenly_clause,
    },
    SentenceRejectRule {
        id: "different-names",
        heads: &["choose", "target", "destroy", "exile"],
        message: "unsupported different-names selection clause",
        predicate: sentence_has_different_names_clause,
    },
    SentenceRejectRule {
        id: "chosen-at-random",
        heads: &["choose", "target", "discard", "exile"],
        message: "unsupported chosen-at-random clause",
        predicate: sentence_has_chosen_at_random_clause,
    },
    SentenceRejectRule {
        id: "draw-for-each-card-exiled-from-hand-this-way",
        heads: &["draw", "for"],
        message: "unsupported draw-for-each-card-exiled-from-hand clause",
        predicate: sentence_has_for_each_card_exiled_from_hand_this_way_clause,
    },
    SentenceRejectRule {
        id: "defending-players-choice",
        heads: &["defending", "target", "of"],
        message: "unsupported defending-players-choice clause",
        predicate: sentence_has_defending_players_choice_clause,
    },
    SentenceRejectRule {
        id: "creature-token-player-planeswalker-target",
        heads: &["target"],
        message: "unsupported creature-token/player/planeswalker target clause",
        predicate: sentence_has_target_creature_token_player_planeswalker_clause,
    },
    SentenceRejectRule {
        id: "if-you-sacrifice-an-island-this-way",
        heads: &["if"],
        message: "unsupported if-you-sacrifice-an-island-this-way clause",
        predicate: sentence_has_if_you_sacrifice_an_island_this_way_clause,
    },
    SentenceRejectRule {
        id: "commander-cast-count",
        heads: &["for"],
        message: "unsupported commander-cast-count clause",
        predicate: sentence_has_commander_cast_count_clause,
    },
    SentenceRejectRule {
        id: "spent-to-cast-condition",
        heads: &["if", "unless", "when", "as"],
        message: "unsupported spent-to-cast condition clause",
        predicate: sentence_has_spent_to_cast_clause,
    },
    SentenceRejectRule {
        id: "face-down",
        heads: &["face", "turn", "cast", "exile", "manifest"],
        message: "unsupported face-down clause",
        predicate: sentence_has_face_down_clause,
    },
    SentenceRejectRule {
        id: "copy-spell-legendary-exception",
        heads: &["copy"],
        message: "unsupported copy-spell legendary-exception clause",
        predicate: sentence_has_copy_spell_legendary_exception_clause,
    },
    SentenceRejectRule {
        id: "return-each-creature-that-isnt-list",
        heads: &["return"],
        message: "unsupported return-each-creature-that-isnt-list clause",
        predicate: sentence_has_return_each_creature_that_isnt_list_clause,
    },
    SentenceRejectRule {
        id: "negated-untap",
        heads: &["this", "that", "target", "it", "creatures", "players"],
        message: "unsupported negated untap clause",
        predicate: sentence_has_unsupported_negated_untap_clause,
    },
];

fn sentence_head<'a>(words: &[&'a str]) -> &'a str {
    words.first().copied().unwrap_or("")
}

fn sentence_reject_rule_head_matches(head: &str, candidates: &[&str]) -> bool {
    candidates.is_empty() || candidates.iter().any(|candidate| *candidate == head)
}

fn find_sentence_reject_rule(
    sentence_words: &[&str],
    tokens: &[Token],
) -> Option<&'static SentenceRejectRule> {
    let head = sentence_head(sentence_words);
    SENTENCE_REJECT_RULES.iter().find(|rule| {
        sentence_reject_rule_head_matches(head, rule.heads)
            && (rule.predicate)(sentence_words, tokens)
    })
}

fn diagnose_sentence_unsupported(tokens: &[Token], sentence_words: &[&str]) -> Option<CardTextError> {
    let clause = ClauseView::from_tokens(tokens).display_text();

    if is_ring_tempts_sentence(tokens) {
        return Some(CardTextError::ParseError(format!(
            "unsupported ring tempts clause (clause: '{}') [rule=ring-tempts]",
            clause
        )));
    }

    if is_enters_as_copy_clause(sentence_words) {
        return Some(CardTextError::ParseError(format!(
            "unsupported enters-as-copy replacement clause (clause: '{}') [rule=enters-as-copy]",
            clause
        )));
    }

    find_sentence_reject_rule(sentence_words, tokens).map(|rule| {
        CardTextError::ParseError(format!(
            "{} (clause: '{}') [rule={}]",
            rule.message, clause, rule.id
        ))
    })
}

fn sentence_has_each_player_lose_discard_sacrifice_chain(words: &[&str], _: &[Token]) -> bool {
    words.starts_with(&["each", "player"])
        && words.contains(&"then")
        && (words.contains(&"lose") || words.contains(&"loses"))
        && (words.contains(&"discard") || words.contains(&"discards"))
        && (words.contains(&"sacrifice") || words.contains(&"sacrifices"))
}

fn sentence_has_each_player_exile_sacrifice_return_exiled_clause(
    words: &[&str],
    _: &[Token],
) -> bool {
    words.starts_with(&["each", "player", "exiles", "all"])
        && words.contains(&"sacrifices")
        && words.contains(&"puts")
        && words.contains(&"exiled")
        && words.contains(&"this")
        && words.contains(&"way")
}

fn sentence_has_loses_all_abilities_with_becomes_clause(words: &[&str], _: &[Token]) -> bool {
    let has_loses_all_abilities = (words.contains(&"lose") || words.contains(&"loses"))
        && words
            .windows(2)
            .any(|window| window == ["all", "abilities"]);
    has_loses_all_abilities && words.contains(&"becomes")
}

fn sentence_has_spent_to_cast_this_spell_without_condition(words: &[&str], _: &[Token]) -> bool {
    let has_spent_to_cast_this_spell = words
        .windows(6)
        .any(|window| window == ["was", "spent", "to", "cast", "this", "spell"]);
    has_spent_to_cast_this_spell && !words.iter().any(|word| matches!(*word, "if" | "unless"))
}

fn sentence_has_would_enter_instead_replacement_clause(words: &[&str], _: &[Token]) -> bool {
    words.iter().any(|word| *word == "would")
        && words
            .iter()
            .any(|word| *word == "enter" || *word == "enters")
        && words.iter().any(|word| *word == "instead")
}

fn sentence_has_different_mana_value_constraint(words: &[&str], _: &[Token]) -> bool {
    words
        .windows(3)
        .any(|window| window == ["different", "mana", "value"])
}

fn sentence_has_most_common_color_constraint(words: &[&str], _: &[Token]) -> bool {
    words
        .windows(5)
        .any(|window| window == ["most", "common", "color", "among", "all"])
        && words.contains(&"permanents")
}

fn sentence_has_power_vs_count_constraint(words: &[&str], _: &[Token]) -> bool {
    words.contains(&"power")
        && words
            .windows(8)
            .any(|window| window == ["less", "than", "or", "equal", "to", "the", "number", "of"])
}

fn sentence_has_put_into_graveyards_from_battlefield_this_turn(
    words: &[&str],
    _: &[Token],
) -> bool {
    words.windows(8).any(|window| {
        window
            == [
                "put",
                "into",
                "graveyards",
                "from",
                "the",
                "battlefield",
                "this",
                "turn",
            ]
    })
}

fn sentence_has_phase_out_until_leaves_clause(words: &[&str], _: &[Token]) -> bool {
    words
        .iter()
        .any(|word| matches!(*word, "phase" | "phases" | "phased"))
        && words.contains(&"until")
        && words
            .windows(3)
            .any(|window| window == ["leaves", "the", "battlefield"])
}

fn sentence_has_unsupported_investigate_for_each_clause(words: &[&str], _: &[Token]) -> bool {
    let is_for_each_vote_investigate = words.starts_with(&["for", "each"])
        && words.iter().any(|word| *word == "vote" || *word == "votes")
        && words
            .iter()
            .any(|word| *word == "investigate" || *word == "investigates");
    !is_for_each_vote_investigate
        && words
            .iter()
            .any(|word| *word == "investigate" || *word == "investigates")
        && words.windows(2).any(|window| window == ["for", "each"])
}

fn sentence_has_same_name_as_another_in_hand_clause(words: &[&str], _: &[Token]) -> bool {
    words
        .windows(6)
        .any(|window| window == ["same", "name", "as", "another", "card", "in"])
        && words.contains(&"hand")
}

fn sentence_has_for_each_mana_from_spent_to_cast_clause(words: &[&str], _: &[Token]) -> bool {
    words
        .windows(4)
        .any(|window| window == ["for", "each", "mana", "from"])
        && words.contains(&"spent")
        && words
            .windows(4)
            .any(|window| window == ["cast", "this", "spell", "create"])
}

fn sentence_has_when_you_sacrifice_this_way_clause(words: &[&str], _: &[Token]) -> bool {
    words
        .windows(3)
        .any(|window| window == ["when", "you", "sacrifice"])
        && words.windows(2).any(|window| window == ["this", "way"])
}

fn sentence_has_sacrifice_any_number_then_draw_that_many_clause(
    words: &[&str],
    _: &[Token],
) -> bool {
    words
        .iter()
        .any(|word| *word == "sacrifice" || *word == "sacrifices")
        && words
            .windows(3)
            .any(|window| window == ["any", "number", "of"])
        && words.iter().any(|word| *word == "draw" || *word == "draws")
        && words.windows(2).any(|window| window == ["that", "many"])
}

fn sentence_has_greatest_mana_value_clause(words: &[&str], _: &[Token]) -> bool {
    words
        .windows(3)
        .any(|window| window == ["greatest", "mana", "value"])
}

fn sentence_has_least_power_among_creatures_clause(words: &[&str], _: &[Token]) -> bool {
    words
        .windows(4)
        .any(|window| window == ["least", "power", "among", "creatures"])
}

fn sentence_has_villainous_choice_clause(words: &[&str], _: &[Token]) -> bool {
    words.contains(&"villainous") && words.contains(&"choice")
}

fn sentence_has_divided_evenly_clause(words: &[&str], _: &[Token]) -> bool {
    words
        .windows(2)
        .any(|window| window == ["divided", "evenly"])
}

fn sentence_has_different_names_clause(words: &[&str], _: &[Token]) -> bool {
    words
        .windows(2)
        .any(|window| window == ["different", "names"])
}

fn sentence_has_chosen_at_random_clause(words: &[&str], _: &[Token]) -> bool {
    words
        .windows(3)
        .any(|window| window == ["chosen", "at", "random"])
}

fn sentence_has_for_each_card_exiled_from_hand_this_way_clause(
    words: &[&str],
    _: &[Token],
) -> bool {
    words
        .windows(4)
        .any(|window| window == ["for", "each", "card", "exiled"])
        && words
            .windows(3)
            .any(|window| window == ["hand", "this", "way"])
}

fn sentence_has_defending_players_choice_clause(words: &[&str], _: &[Token]) -> bool {
    words.contains(&"defending")
        && words
            .windows(3)
            .any(|window| window == ["player's", "choice", "target"])
        || words
            .windows(3)
            .any(|window| window == ["defending", "player's", "choice"])
}

fn sentence_has_target_creature_token_player_planeswalker_clause(
    words: &[&str],
    _: &[Token],
) -> bool {
    words.contains(&"target")
        && words.contains(&"creature")
        && words.contains(&"token")
        && words.contains(&"player")
        && words.contains(&"planeswalker")
}

fn sentence_has_if_you_sacrifice_an_island_this_way_clause(words: &[&str], _: &[Token]) -> bool {
    words
        .windows(5)
        .any(|window| window == ["if", "you", "sacrifice", "an", "island"])
        && words.windows(2).any(|window| window == ["this", "way"])
}

fn sentence_has_commander_cast_count_clause(words: &[&str], _: &[Token]) -> bool {
    words
        .windows(3)
        .any(|window| window == ["for", "each", "time"])
        && words.contains(&"cast")
        && words.contains(&"commander")
        && words
            .windows(4)
            .any(|window| window == ["from", "the", "command", "zone"])
}

fn sentence_has_spent_to_cast_clause(words: &[&str], _: &[Token]) -> bool {
    words
        .windows(3)
        .any(|window| window == ["spent", "to", "cast"])
}

fn sentence_has_face_down_clause(words: &[&str], _: &[Token]) -> bool {
    words.windows(2).any(|window| window == ["face", "down"])
}

fn sentence_has_copy_spell_legendary_exception_clause(words: &[&str], _: &[Token]) -> bool {
    words.contains(&"copy")
        && words.contains(&"spell")
        && words.contains(&"legendary")
        && (words.contains(&"except") || words.contains(&"isnt"))
}

fn sentence_has_return_each_creature_that_isnt_list_clause(words: &[&str], _: &[Token]) -> bool {
    words.starts_with(&["return", "each", "creature", "that", "isnt"])
        && words.iter().filter(|word| **word == "or").count() >= 1
}

fn sentence_has_unsupported_negated_untap_clause(words: &[&str], _: &[Token]) -> bool {
    is_negated_untap_clause(words) && !words.contains(&"and")
}

pub(crate) fn parse_effect_sentence(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    // Generic support for trailing "where X is ..." clauses.
    //
    // Many Oracle texts define a computed X (not cost-derived X) using:
    //   "... X ..., where X is <expression>."
    //
    // We parse the where-X value, strip the suffix clause for normal parsing,
    // then substitute the parsed `Value::X` occurrences with that value.
    let clause_words = words(tokens);
    let Some(where_idx) = clause_words
        .windows(3)
        .position(|window| window == ["where", "x", "is"])
    else {
        return parse_effect_sentence_inner(tokens);
    };
    let Some(where_token_idx) = token_index_for_word_index(tokens, where_idx) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported where-x clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let where_tokens = &tokens[where_token_idx..];

    let stripped = trim_edge_punctuation(&tokens[..where_token_idx]);
    let stripped_words = words(&stripped);
    let where_words = words(where_tokens);

    // Special-case common "where X is its power/toughness/mana value" patterns, because
    // resolving "its" depends on whether the main clause is targeting something.
    let where_value = match where_words.get(3..) {
        Some(["its", "power"]) => {
            if stripped_words.iter().any(|w| *w == "target") {
                Value::PowerOf(Box::new(crate::target::ChooseSpec::target(
                    crate::target::ChooseSpec::Object(ObjectFilter::default()),
                )))
            } else {
                Value::SourcePower
            }
        }
        Some(["its", "toughness"]) => {
            if stripped_words.iter().any(|w| *w == "target") {
                Value::ToughnessOf(Box::new(crate::target::ChooseSpec::target(
                    crate::target::ChooseSpec::Object(ObjectFilter::default()),
                )))
            } else {
                Value::SourceToughness
            }
        }
        Some(["its", "mana", "value"]) => {
            Value::ManaValueOf(Box::new(if stripped_words.iter().any(|w| *w == "target") {
                crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
                    ObjectFilter::default(),
                ))
            } else {
                crate::target::ChooseSpec::Source
            }))
        }
        Some(["this", "creatures", "power"]) => Value::SourcePower,
        Some(["this", "creatures", "toughness"]) => Value::SourceToughness,
        Some(["this", "creatures", "mana", "value"]) => {
            Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Source))
        }
        Some(["that", "creatures", "power"]) => {
            Value::PowerOf(Box::new(crate::target::ChooseSpec::target(
                crate::target::ChooseSpec::Object(ObjectFilter::default()),
            )))
        }
        Some(["that", "creatures", "toughness"]) => {
            Value::ToughnessOf(Box::new(crate::target::ChooseSpec::target(
                crate::target::ChooseSpec::Object(ObjectFilter::default()),
            )))
        }
        Some(["that", "creatures", "mana", "value"]) => {
            Value::ManaValueOf(Box::new(crate::target::ChooseSpec::target(
                crate::target::ChooseSpec::Object(ObjectFilter::default()),
            )))
        }
        _ => parse_where_x_value_clause(where_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-x clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?,
    };

    let mut effects = parse_effect_sentence_inner(&stripped)?;
    replace_unbound_x_in_effects_anywhere(&mut effects, &where_value, &clause_words.join(" "))?;
    Ok(effects)
}

pub(crate) fn parse_effect_sentence_inner(
    tokens: &[Token],
) -> Result<Vec<EffectAst>, CardTextError> {
    parser_trace("parse_effect_sentence:entry", tokens);
    let sentence_words = words(tokens);
    if let Some(effects) = parse_redirect_next_damage_sentence(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_prevent_next_time_damage_sentence(tokens)? {
        return Ok(effects);
    }
    if is_activate_only_restriction_sentence(tokens) {
        return Ok(Vec::new());
    }
    if is_trigger_only_restriction_sentence(tokens) {
        return Ok(Vec::new());
    }
    if sentence_words.starts_with(&["round", "up", "each", "time"]) {
        // "Round up each time." is reminder text for half P/T copy effects.
        // The semantic behavior is represented by the underlying token-copy primitive.
        parser_trace("parse_effect_sentence:round-up-reminder", tokens);
        return Ok(Vec::new());
    }
    if let Some(stripped) = strip_labeled_conditional_prefix(tokens) {
        parser_trace("parse_effect_sentence:conditional-labeled", stripped);
        return parse_conditional_sentence(stripped);
    }
    if tokens.first().is_some_and(|token| token.is_word("then"))
        && tokens.get(1).is_some_and(|token| token.is_word("if"))
    {
        parser_trace("parse_effect_sentence:conditional-then", &tokens[1..]);
        return parse_conditional_sentence(&tokens[1..]);
    }
    if tokens.first().is_some_and(|token| token.is_word("then")) && tokens.len() > 1 {
        parser_trace("parse_effect_sentence:leading-then", &tokens[1..]);
        return parse_effect_sentence(&tokens[1..]);
    }
    if let Some(effects) = parse_double_target_power_sentence(tokens)? {
        parser_trace("parse_effect_sentence:double-target-power", tokens);
        return Ok(effects);
    }
    if let Some(effects) = run_sentence_primitives(tokens, PRE_CONDITIONAL_SENTENCE_PRIMITIVES)? {
        return Ok(effects);
    }
    if sentence_words.starts_with(&["if", "you", "cast", "a", "spell", "this", "way"])
        && sentence_words.contains(&"rather")
        && sentence_words.contains(&"mana")
        && sentence_words.contains(&"cost")
    {
        // "If you cast a spell this way, pay life equal to its mana value
        // rather than paying its mana cost."
        return Ok(vec![
            EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                tag: TagKey::from(IT_TAG),
                player: PlayerAst::You,
            },
        ]);
    }
    if tokens.first().is_some_and(|token| token.is_word("if")) {
        parser_trace("parse_effect_sentence:conditional", tokens);
        return parse_conditional_sentence(tokens);
    }
    if let Some(effects) = run_sentence_primitives(tokens, POST_CONDITIONAL_SENTENCE_PRIMITIVES)? {
        return Ok(effects);
    }

    let mut effects = match parse_effect_chain(tokens) {
        Ok(effects) => effects,
        Err(parse_err) => {
            if let Some(diag) = diagnose_sentence_unsupported(tokens, &sentence_words) {
                return Err(diag);
            }
            return Err(parse_err);
        }
    };
    apply_where_x_to_damage_amounts(tokens, &mut effects)?;
    Ok(effects)
}

pub(crate) fn is_enters_as_copy_clause(words: &[&str]) -> bool {
    let has_enter_before_as_copy = words
        .windows(3)
        .position(|window| window == ["as", "a", "copy"] || window == ["as", "an", "copy"])
        .is_some_and(|idx| {
            words[..idx]
                .iter()
                .any(|word| *word == "enter" || *word == "enters")
        });
    let has_enter_before_as_copy_no_article = words
        .windows(2)
        .position(|window| window == ["as", "copy"])
        .is_some_and(|idx| {
            words[..idx]
                .iter()
                .any(|word| *word == "enter" || *word == "enters")
        });
    has_enter_before_as_copy || has_enter_before_as_copy_no_article
}

pub(crate) fn strip_labeled_conditional_prefix(tokens: &[Token]) -> Option<&[Token]> {
    let if_idx = tokens.iter().position(|token| token.is_word("if"))?;
    if !(1..=3).contains(&if_idx) {
        return None;
    }
    if !tokens[..if_idx]
        .iter()
        .all(|token| matches!(token, Token::Word(_, _)))
    {
        return None;
    }

    let prefix_words = words(&tokens[..if_idx]);
    if prefix_words.is_empty() {
        return None;
    }
    let is_known_label = matches!(
        prefix_words[0],
        "adamant"
            | "addendum"
            | "ascend"
            | "battalion"
            | "delirium"
            | "domain"
            | "ferocious"
            | "formidable"
            | "hellbent"
            | "metalcraft"
            | "morbid"
            | "raid"
            | "revolt"
            | "spectacle"
            | "spell"
            | "surge"
            | "threshold"
            | "undergrowth"
    );
    if !is_known_label {
        return None;
    }

    Some(&tokens[if_idx..])
}

pub(crate) fn is_negated_untap_clause(words: &[&str]) -> bool {
    if words.len() < 3 {
        return false;
    }
    let has_untap = words.contains(&"untap") || words.contains(&"untaps");
    let has_negation = words.contains(&"doesnt")
        || words.contains(&"dont")
        || words.windows(2).any(|pair| pair == ["does", "not"])
        || words.windows(2).any(|pair| pair == ["do", "not"])
        || words.contains(&"cant")
        || words.windows(2).any(|pair| pair == ["can", "not"]);
    has_untap && has_negation
}

pub(crate) fn parse_token_copy_modifier_sentence(tokens: &[Token]) -> Option<TokenCopyFollowup> {
    let filtered: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let is_gain_haste_until_eot = matches!(
        filtered.as_slice(),
        ["it", "gains", "haste", "until", "end", "of", "turn"]
            | ["they", "gain", "haste", "until", "end", "of", "turn"]
    );
    if is_gain_haste_until_eot {
        return Some(TokenCopyFollowup::GainHasteUntilEndOfTurn);
    }

    let is_has_haste = matches!(
        filtered.as_slice(),
        ["it", "has", "haste"] | ["they", "have", "haste"]
    );
    if is_has_haste {
        return Some(TokenCopyFollowup::HasHaste);
    }

    if filtered.starts_with(&["sacrifice", "it"]) || filtered.starts_with(&["sacrifice", "them"]) {
        let has_next_end_step = filtered
            .windows(6)
            .any(|window| window == ["at", "beginning", "of", "next", "end", "step"]);
        if has_next_end_step {
            return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
        }
    }
    if filtered.starts_with(&["exile", "it"]) || filtered.starts_with(&["exile", "them"]) {
        let has_next_end_step = filtered
            .windows(6)
            .any(|window| window == ["at", "beginning", "of", "next", "end", "step"]);
        if has_next_end_step {
            return Some(TokenCopyFollowup::ExileAtNextEndStep);
        }
    }

    let starts_delayed_end_step_sacrifice = filtered.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "end",
        "step",
        "sacrifice",
    ]) || filtered.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "next",
        "end",
        "step",
        "sacrifice",
    ]) || filtered.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "next",
        "end",
        "step",
        "sacrifice",
    ]);
    if starts_delayed_end_step_sacrifice {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }
    let starts_delayed_end_step_exile = filtered.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "end",
        "step",
        "exile",
    ]) || filtered.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "next",
        "end",
        "step",
        "exile",
    ]) || filtered.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "next",
        "end",
        "step",
        "exile",
    ]);
    if starts_delayed_end_step_exile {
        return Some(TokenCopyFollowup::ExileAtNextEndStep);
    }

    None
}

pub(crate) fn parse_delayed_until_next_end_step_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut idx = 0usize;
    if !tokens.get(idx).is_some_and(|token| token.is_word("at")) {
        return Ok(None);
    }
    idx += 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
        idx += 1;
    }
    if !tokens
        .get(idx)
        .is_some_and(|token| token.is_word("beginning"))
    {
        return Ok(None);
    }
    idx += 1;
    if !tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        return Ok(None);
    }
    idx += 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
        idx += 1;
    }

    let mut player = if tokens.get(idx).is_some_and(|token| token.is_word("your")) {
        idx += 1;
        PlayerFilter::You
    } else {
        PlayerFilter::Any
    };
    let mut start_next_turn = false;

    if tokens.get(idx).is_some_and(|token| token.is_word("next")) {
        if !tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("end"))
            || !tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("step"))
        {
            return Ok(None);
        }
        idx += 3;
    } else {
        if !tokens.get(idx).is_some_and(|token| token.is_word("end"))
            || !tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("step"))
        {
            return Ok(None);
        }
        idx += 2;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        idx += 1;
        if tokens.get(idx).is_some_and(|token| token.is_word("that"))
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("player") || token.is_word("players"))
        {
            player = PlayerFilter::IteratedPlayer;
            idx += 2;
        } else if tokens.get(idx).is_some_and(|token| token.is_word("your")) {
            player = PlayerFilter::You;
            idx += 1;
        } else if tokens.get(idx).is_some_and(|token| token.is_word("target"))
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("player"))
        {
            player = PlayerFilter::Target(Box::new(PlayerFilter::Any));
            idx += 2;
        } else {
            return Ok(None);
        }

        if !tokens.get(idx).is_some_and(|token| token.is_word("next"))
            || !tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("turn"))
        {
            return Ok(None);
        }
        idx += 2;
        start_next_turn = true;
    }

    if matches!(tokens.get(idx), Some(Token::Comma(_))) {
        idx += 1;
    }
    let remainder = trim_commas(&tokens[idx..]);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(
            "missing delayed end-step effect clause".to_string(),
        ));
    }

    let delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed end-step effect clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    if start_next_turn {
        let player_ast = match player {
            PlayerFilter::You => PlayerAst::You,
            PlayerFilter::IteratedPlayer => PlayerAst::That,
            PlayerFilter::Target(_) => PlayerAst::Target,
            PlayerFilter::Opponent => PlayerAst::Opponent,
            _ => PlayerAst::Any,
        };
        Ok(Some(vec![EffectAst::DelayedUntilEndStepOfExtraTurn {
            player: player_ast,
            effects: delayed_effects,
        }]))
    } else {
        Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
            player,
            effects: delayed_effects,
        }]))
    }
}

pub(crate) fn parse_sentence_delayed_trigger_this_turn(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
    {
        return Ok(None);
    }

    let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    else {
        return Ok(None);
    };

    let mut trigger_tokens = trim_commas(&tokens[..comma_idx]);
    if trigger_tokens
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
    {
        trigger_tokens = trigger_tokens[1..].to_vec();
    }
    if trigger_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger clause before comma (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let trigger_words = words(&trigger_tokens);
    if trigger_words.len() < 3 || !trigger_words.ends_with(&["this", "turn"]) {
        return Ok(None);
    }

    let trim_start = token_index_for_word_index(&trigger_tokens, trigger_words.len() - 2)
        .unwrap_or(trigger_tokens.len());
    let trigger_core_tokens = trim_commas(&trigger_tokens[..trim_start]);
    if trigger_core_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger clause before 'this turn' (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let trigger_core_words = words(&trigger_core_tokens);
    let trigger = if matches!(
        trigger_core_words.as_slice(),
        ["that", "creature", "is", "dealt", "damage"]
            | ["that", "permanent", "is", "dealt", "damage"]
    ) {
        let mut filter = if trigger_core_words[1] == "creature" {
            ObjectFilter::creature()
        } else {
            ObjectFilter::permanent()
        };
        filter = filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
        TriggerSpec::IsDealtDamage(filter)
    } else {
        parse_trigger_clause(&trigger_core_tokens)?
    };
    if matches!(trigger, TriggerSpec::Custom(_)) {
        return Err(CardTextError::ParseError(format!(
            "unsupported delayed trigger clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let remainder = trim_commas(&tokens[comma_idx + 1..]);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger effect clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger effect clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
        trigger,
        effects: delayed_effects,
    }]))
}

pub(crate) fn parse_delayed_when_that_dies_this_turn_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 6 {
        return Ok(None);
    }
    if !matches!(
        clause_words.first().copied(),
        Some("when" | "whenever" | "if")
    ) {
        return Ok(None);
    }
    let mut delayed_filter: Option<ObjectFilter> = None;
    let split_after_word_idx = if clause_words.get(1) == Some(&"that") {
        let Some(dies_idx) = clause_words.iter().position(|word| *word == "dies") else {
            return Ok(None);
        };
        if clause_words.get(dies_idx + 1) != Some(&"this")
            || clause_words.get(dies_idx + 2) != Some(&"turn")
        {
            return Ok(None);
        }
        dies_idx + 2
    } else if let Some(dealt_idx) = clause_words
        .windows(7)
        .position(|window| window == ["dealt", "damage", "this", "way", "dies", "this", "turn"])
    {
        if dealt_idx <= 1 {
            return Ok(None);
        }
        let subject_start = token_index_for_word_index(tokens, 1).unwrap_or(tokens.len());
        let subject_end = token_index_for_word_index(tokens, dealt_idx).unwrap_or(tokens.len());
        if subject_start >= subject_end {
            return Ok(None);
        }
        let mut subject_tokens = trim_edge_punctuation(&tokens[subject_start..subject_end]);
        if subject_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object filter in delayed dies-this-way clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let stripped_subject = strip_leading_articles(&subject_tokens);
        if !stripped_subject.is_empty() {
            subject_tokens = stripped_subject;
        }
        delayed_filter = Some(parse_object_filter(&subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported object filter in delayed dies-this-way clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?);
        dealt_idx + 6
    } else {
        return Ok(None);
    };
    let split_idx =
        token_index_for_word_index(tokens, split_after_word_idx + 1).unwrap_or(tokens.len());
    let mut remainder = &tokens[split_idx..];
    if matches!(remainder.first(), Some(Token::Comma(_))) {
        remainder = &remainder[1..];
    }
    let remainder = trim_commas(remainder);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed dies-this-turn effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed dies-this-turn effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::DelayedWhenLastObjectDiesThisTurn {
        filter: delayed_filter,
        effects: delayed_effects,
    }]))
}

pub(crate) fn parse_each_player_choose_and_sacrifice_rest(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let all_words = words(tokens);
    if all_words.len() < 6 {
        return Ok(None);
    }

    if !all_words.starts_with(&["each", "player", "chooses"])
        && !all_words.starts_with(&["each", "player", "choose"])
    {
        return Ok(None);
    }

    let then_idx = tokens.iter().position(|token| token.is_word("then"));
    let Some(then_idx) = then_idx else {
        return Ok(None);
    };

    let after_then = &tokens[then_idx + 1..];
    let after_words = words(after_then);
    if !(after_words.starts_with(&["sacrifice", "the", "rest"])
        || after_words.starts_with(&["sacrifices", "the", "rest"]))
    {
        return Ok(None);
    }

    let choose_tokens = &tokens[3..then_idx];
    if choose_tokens.is_empty() {
        return Ok(None);
    }

    let from_idx = find_from_among(choose_tokens);
    let Some(from_idx) = from_idx else {
        return Ok(None);
    };

    let (list_tokens, base_tokens) = if from_idx == 0 {
        let list_start = find_list_start(&choose_tokens[2..])
            .map(|idx| idx + 2)
            .ok_or_else(|| {
                CardTextError::ParseError("missing choice list after 'from among'".to_string())
            })?;
        (
            choose_tokens.get(list_start..).unwrap_or_default(),
            choose_tokens.get(2..list_start).unwrap_or_default(),
        )
    } else {
        (
            choose_tokens.get(..from_idx).unwrap_or_default(),
            choose_tokens.get(from_idx + 2..).unwrap_or_default(),
        )
    };

    let list_tokens = trim_commas(list_tokens);
    let base_tokens = trim_commas(base_tokens);
    if list_tokens.is_empty() || base_tokens.is_empty() {
        return Ok(None);
    }

    let mut base_filter = parse_object_filter(&base_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported base filter in choose-and-sacrifice clause (clause: '{}')",
            all_words.join(" ")
        ))
    })?;
    if base_filter.controller.is_none() {
        base_filter.controller = Some(PlayerFilter::IteratedPlayer);
    }

    let mut effects = Vec::new();
    let keep_tag: TagKey = "keep".into();

    for segment in split_choose_list(&list_tokens) {
        let segment = strip_leading_articles(&segment);
        if segment.is_empty() {
            continue;
        }
        let segment_filter = parse_object_filter(&segment, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported choice filter in choose-and-sacrifice clause (clause: '{}')",
                all_words.join(" ")
            ))
        })?;
        let mut combined = merge_filters(&base_filter, &segment_filter);
        combined = combined.not_tagged(keep_tag.clone());
        effects.push(EffectAst::ChooseObjects {
            filter: combined,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::Implicit,
            tag: keep_tag.clone(),
        });
    }

    if effects.is_empty() {
        return Ok(None);
    }

    let sacrifice_filter = base_filter.clone().not_tagged(keep_tag.clone());
    effects.push(EffectAst::SacrificeAll {
        filter: sacrifice_filter,
        player: PlayerAst::Implicit,
    });

    Ok(Some(EffectAst::ForEachPlayer { effects }))
}

pub(crate) fn find_from_among(tokens: &[Token]) -> Option<usize> {
    tokens.iter().enumerate().find_map(|(idx, token)| {
        if token.is_word("from") && tokens.get(idx + 1).is_some_and(|t| t.is_word("among")) {
            Some(idx)
        } else {
            None
        }
    })
}

pub(crate) fn find_list_start(tokens: &[Token]) -> Option<usize> {
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        if is_article(word) {
            if tokens
                .get(idx + 1)
                .and_then(Token::as_word)
                .and_then(parse_card_type)
                .is_some()
            {
                return Some(idx);
            }
        } else if parse_card_type(word).is_some() {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn trim_commas(tokens: &[Token]) -> Vec<Token> {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end && matches!(tokens[start], Token::Comma(_)) {
        start += 1;
    }
    while end > start && matches!(tokens[end - 1], Token::Comma(_)) {
        end -= 1;
    }
    tokens[start..end].to_vec()
}

pub(crate) fn trim_edge_punctuation(tokens: &[Token]) -> Vec<Token> {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end
        && matches!(
            tokens[start],
            Token::Comma(_) | Token::Period(_) | Token::Semicolon(_)
        )
    {
        start += 1;
    }
    while end > start
        && matches!(
            tokens[end - 1],
            Token::Comma(_) | Token::Period(_) | Token::Semicolon(_)
        )
    {
        end -= 1;
    }
    tokens[start..end].to_vec()
}

pub(crate) fn strip_leading_articles(tokens: &[Token]) -> Vec<Token> {
    let mut start = 0usize;
    while start < tokens.len() {
        if let Some(word) = tokens[start].as_word()
            && is_article(word)
        {
            start += 1;
            continue;
        }
        break;
    }
    tokens[start..].to_vec()
}

pub(crate) fn split_choose_list(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    for segment in split_on_and(tokens) {
        for sub in split_on_comma(&segment) {
            let trimmed = trim_commas(&sub);
            if !trimmed.is_empty() {
                segments.push(trimmed);
            }
        }
    }
    segments
}

pub(crate) fn merge_filters(base: &ObjectFilter, specific: &ObjectFilter) -> ObjectFilter {
    let mut merged = base.clone();

    if !specific.card_types.is_empty() {
        merged.card_types = specific.card_types.clone();
    }
    if !specific.all_card_types.is_empty() {
        merged.all_card_types = specific.all_card_types.clone();
    }
    if !specific.subtypes.is_empty() {
        merged.subtypes.extend(specific.subtypes.clone());
    }
    if !specific.excluded_card_types.is_empty() {
        merged
            .excluded_card_types
            .extend(specific.excluded_card_types.clone());
    }
    if !specific.excluded_colors.is_empty() {
        merged.excluded_colors = merged.excluded_colors.union(specific.excluded_colors);
    }
    if let Some(colors) = specific.colors {
        merged.colors = Some(
            merged
                .colors
                .map_or(colors, |existing| existing.union(colors)),
        );
    }
    if merged.zone.is_none() {
        merged.zone = specific.zone;
    }
    if merged.controller.is_none() {
        merged.controller = specific.controller.clone();
    }
    if merged
        .attacking_player_or_planeswalker_controlled_by
        .is_none()
    {
        merged.attacking_player_or_planeswalker_controlled_by = specific
            .attacking_player_or_planeswalker_controlled_by
            .clone();
    }
    if merged.owner.is_none() {
        merged.owner = specific.owner.clone();
    }
    merged.other |= specific.other;
    merged.token |= specific.token;
    merged.nontoken |= specific.nontoken;
    merged.tapped |= specific.tapped;
    merged.untapped |= specific.untapped;
    merged.attacking |= specific.attacking;
    merged.nonattacking |= specific.nonattacking;
    merged.blocking |= specific.blocking;
    merged.nonblocking |= specific.nonblocking;
    merged.blocked |= specific.blocked;
    merged.unblocked |= specific.unblocked;
    merged.is_commander |= specific.is_commander;
    merged.noncommander |= specific.noncommander;
    merged.colorless |= specific.colorless;
    merged.multicolored |= specific.multicolored;
    merged.monocolored |= specific.monocolored;

    if let Some(mv) = &specific.mana_value {
        merged.mana_value = Some(mv.clone());
    }
    if let Some(power) = &specific.power {
        merged.power = Some(power.clone());
        merged.power_reference = specific.power_reference;
    }
    if let Some(toughness) = &specific.toughness {
        merged.toughness = Some(toughness.clone());
        merged.toughness_reference = specific.toughness_reference;
    }
    if specific.has_mana_cost {
        merged.has_mana_cost = true;
    }
    if specific.no_x_in_cost {
        merged.no_x_in_cost = true;
    }
    if merged.with_counter.is_none() {
        merged.with_counter = specific.with_counter;
    }
    if merged.without_counter.is_none() {
        merged.without_counter = specific.without_counter;
    }
    if merged.alternative_cast.is_none() {
        merged.alternative_cast = specific.alternative_cast;
    }
    for ability_id in &specific.static_abilities {
        if !merged.static_abilities.contains(ability_id) {
            merged.static_abilities.push(*ability_id);
        }
    }
    for ability_id in &specific.excluded_static_abilities {
        if !merged.excluded_static_abilities.contains(ability_id) {
            merged.excluded_static_abilities.push(*ability_id);
        }
    }
    for marker in &specific.ability_markers {
        if !merged
            .ability_markers
            .iter()
            .any(|value| value.eq_ignore_ascii_case(marker))
        {
            merged.ability_markers.push(marker.clone());
        }
    }
    for marker in &specific.excluded_ability_markers {
        if !merged
            .excluded_ability_markers
            .iter()
            .any(|value| value.eq_ignore_ascii_case(marker))
        {
            merged.excluded_ability_markers.push(marker.clone());
        }
    }

    merged
}

pub(crate) fn parse_monstrosity_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.first().copied() != Some("monstrosity") {
        return Ok(None);
    }

    let amount_tokens = &tokens[1..];
    let (amount, _) = parse_value(amount_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing monstrosity amount (clause: '{}')",
            words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::Monstrosity { amount }))
}

pub(crate) fn parse_for_each_counter_removed_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let words_all = words(tokens);
    if words_all.len() < 6 {
        return Ok(None);
    }
    if !words_all.starts_with(&["for", "each", "counter", "removed", "this", "way"]) {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)));
    let remainder = if let Some(idx) = comma_idx {
        &tokens[idx + 1..]
    } else {
        &tokens[6..]
    };

    let remainder_words = words(remainder);
    if remainder_words.is_empty() {
        return Ok(None);
    }

    let gets_idx = remainder_words
        .iter()
        .position(|word| *word == "gets" || *word == "get");
    let Some(gets_idx) = gets_idx else {
        return Ok(None);
    };

    let subject_tokens = &remainder[..gets_idx];
    let subject = parse_subject(subject_tokens);
    let target = match subject {
        SubjectAst::This => TargetAst::Source(None),
        _ => return Ok(None),
    };

    let after_gets = &remainder[gets_idx + 1..];
    let modifier_token = after_gets.first().and_then(Token::as_word).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing power/toughness modifier (clause: '{}')",
            remainder_words.join(" ")
        ))
    })?;
    let (power, toughness) = parse_pt_modifier(modifier_token)?;

    let duration = if remainder_words.contains(&"until")
        && remainder_words.contains(&"end")
        && remainder_words.contains(&"turn")
    {
        Until::EndOfTurn
    } else {
        Until::EndOfTurn
    };

    Ok(Some(EffectAst::PumpByLastEffect {
        power,
        toughness,
        target,
        duration,
    }))
}

pub(crate) fn is_exile_that_token_at_end_of_combat(tokens: &[Token]) -> bool {
    let words = words(tokens);
    if words.first().copied() != Some("exile") {
        return false;
    }

    let at_idx = if matches!(words.get(1).copied(), Some("that" | "the" | "those"))
        && matches!(words.get(2).copied(), Some("token" | "tokens"))
    {
        3
    } else if words.get(1).copied() == Some("it") {
        2
    } else {
        return false;
    };
    if words.get(at_idx).copied() != Some("at") {
        return false;
    }
    words[at_idx + 1..] == ["end", "of", "combat"]
        || words[at_idx + 1..] == ["the", "end", "of", "combat"]
}

pub(crate) fn is_sacrifice_that_token_at_end_of_combat(tokens: &[Token]) -> bool {
    let words = words(tokens);
    if words.first().copied() != Some("sacrifice") {
        return false;
    }

    let at_idx = if matches!(words.get(1).copied(), Some("that" | "the" | "those"))
        && matches!(words.get(2).copied(), Some("token" | "tokens"))
    {
        3
    } else if words.get(1).copied() == Some("it") {
        2
    } else {
        return false;
    };
    if words.get(at_idx).copied() != Some("at") {
        return false;
    }
    words[at_idx + 1..] == ["end", "of", "combat"]
        || words[at_idx + 1..] == ["the", "end", "of", "combat"]
}

pub(crate) fn parse_take_extra_turn_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["take", "an", "extra", "turn", "after", "this", "one"] {
        return Ok(Some(EffectAst::ExtraTurnAfterTurn {
            player: PlayerAst::You,
        }));
    }
    Ok(None)
}

pub(crate) fn is_ring_tempts_sentence(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.as_slice() == ["the", "ring", "tempts", "you"]
}

pub(crate) fn find_same_name_reference_span(
    tokens: &[Token],
) -> Result<Option<(usize, usize)>, CardTextError> {
    for idx in 0..tokens.len() {
        if !tokens[idx].is_word("with") {
            continue;
        }
        if idx + 6 < tokens.len()
            && tokens[idx + 1].is_word("the")
            && tokens[idx + 2].is_word("same")
            && tokens[idx + 3].is_word("name")
            && tokens[idx + 4].is_word("as")
            && tokens[idx + 5].is_word("that")
        {
            return Ok(Some((idx, idx + 7)));
        }
        if idx + 5 < tokens.len()
            && tokens[idx + 1].is_word("same")
            && tokens[idx + 2].is_word("name")
            && tokens[idx + 3].is_word("as")
            && tokens[idx + 4].is_word("that")
        {
            return Ok(Some((idx, idx + 6)));
        }
        if idx + 4 < tokens.len()
            && tokens[idx + 1].is_word("the")
            && tokens[idx + 2].is_word("same")
            && tokens[idx + 3].is_word("name")
            && tokens[idx + 4].is_word("as")
        {
            return Err(CardTextError::ParseError(format!(
                "missing 'that <object>' in same-name clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        if idx + 3 < tokens.len()
            && tokens[idx + 1].is_word("same")
            && tokens[idx + 2].is_word("name")
            && tokens[idx + 3].is_word("as")
        {
            return Err(CardTextError::ParseError(format!(
                "missing 'that <object>' in same-name clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    }
    Ok(None)
}

pub(crate) fn strip_same_controller_reference(tokens: &[Token]) -> (Vec<Token>, bool) {
    let mut cleaned = Vec::with_capacity(tokens.len());
    let mut idx = 0usize;
    let mut same_controller = false;
    while idx < tokens.len() {
        if idx + 2 < tokens.len()
            && tokens[idx].is_word("that")
            && tokens[idx + 1].is_word("player")
            && (tokens[idx + 2].is_word("control") || tokens[idx + 2].is_word("controls"))
        {
            same_controller = true;
            idx += 3;
            continue;
        }
        if idx + 2 < tokens.len()
            && tokens[idx].is_word("its")
            && tokens[idx + 1].is_word("controller")
            && (tokens[idx + 2].is_word("control") || tokens[idx + 2].is_word("controls"))
        {
            same_controller = true;
            idx += 3;
            continue;
        }
        if idx + 3 < tokens.len()
            && tokens[idx].is_word("that")
            && (tokens[idx + 1].is_word("creature")
                || tokens[idx + 1].is_word("permanent")
                || tokens[idx + 1].is_word("card"))
            && tokens[idx + 2].is_word("controller")
            && (tokens[idx + 3].is_word("control") || tokens[idx + 3].is_word("controls"))
        {
            same_controller = true;
            idx += 4;
            continue;
        }

        cleaned.push(tokens[idx].clone());
        idx += 1;
    }

    (cleaned, same_controller)
}

pub(crate) fn parse_same_name_fanout_filter(
    tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some((same_start, same_end)) = find_same_name_reference_span(tokens)? else {
        return Ok(None);
    };

    let mut filter_tokens = Vec::with_capacity(tokens.len());
    filter_tokens.extend_from_slice(&tokens[..same_start]);
    filter_tokens.extend_from_slice(&tokens[same_end..]);
    let filter_tokens = trim_commas(&filter_tokens);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in same-name fanout clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let (cleaned_tokens, same_controller) = strip_same_controller_reference(&filter_tokens);
    let cleaned_tokens = trim_commas(&cleaned_tokens);
    if cleaned_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing base object filter in same-name fanout clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&cleaned_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported same-name fanout filter (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    if same_controller {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::SameControllerAsTagged,
        });
    }
    Ok(Some(filter))
}

pub(crate) fn parse_same_name_target_fanout_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (tokens, until_source_leaves) = split_until_source_leaves_tail(tokens);
    let words_all = words(tokens);
    let Some(first_word) = words_all.first().copied() else {
        return Ok(None);
    };

    let deal_tokens: Option<&[Token]> = if first_word == "deal" {
        Some(tokens)
    } else if let Some((Verb::Deal, verb_idx)) = find_verb(tokens) {
        let subject_words: Vec<&str> = words(&tokens[..verb_idx])
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
        if is_source_reference_words(&subject_words) {
            Some(&tokens[verb_idx..])
        } else {
            None
        }
    } else {
        None
    };

    if let Some(deal_tokens) = deal_tokens {
        let deal_words = words(deal_tokens);
        let (amount, used) =
            if deal_words.get(1) == Some(&"that") && deal_words.get(2) == Some(&"much") {
                (Value::EventValue(EventValueSpec::Amount), 2usize)
            } else if let Some((value, used)) = parse_value(&deal_tokens[1..]) {
                (value, used)
            } else {
                return Ok(None);
            };

        let after_amount = &deal_tokens[1 + used..];
        if !after_amount
            .first()
            .is_some_and(|token| token.is_word("damage"))
        {
            return Ok(None);
        }

        let mut target_tokens = &after_amount[1..];
        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("to"))
        {
            target_tokens = &target_tokens[1..];
        }
        if target_tokens.is_empty() {
            return Ok(None);
        }

        let split_idx = (0..target_tokens.len().saturating_sub(2)).find(|idx| {
            target_tokens[*idx].is_word("and")
                && target_tokens[*idx + 1].is_word("each")
                && target_tokens[*idx + 2].is_word("other")
        });
        let Some(split_idx) = split_idx else {
            return Ok(None);
        };
        let first_target_tokens = trim_commas(&target_tokens[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }

        let second_clause_tokens = target_tokens[split_idx + 3..].to_vec();
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_same_name_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;
        return Ok(Some(vec![
            EffectAst::DealDamage {
                amount: amount.clone(),
                target: first_target,
            },
            EffectAst::DealDamageEach { amount, filter },
        ]));
    }

    let verb = first_word;
    if verb != "destroy" && verb != "exile" && verb != "return" {
        return Ok(None);
    }

    let and_idx = (0..tokens.len().saturating_sub(2)).find(|idx| {
        tokens[*idx].is_word("and")
            && tokens[*idx + 1].is_word("all")
            && tokens[*idx + 2].is_word("other")
    });
    let Some(and_idx) = and_idx else {
        return Ok(None);
    };
    if and_idx <= 1 {
        return Ok(None);
    }

    let first_target_tokens = trim_commas(&tokens[1..and_idx]);
    if first_target_tokens.is_empty()
        || !first_target_tokens
            .iter()
            .any(|token| token.is_word("target"))
    {
        return Ok(None);
    }

    let second_clause_tokens = if verb == "return" {
        let to_idx = tokens
            .iter()
            .rposition(|token| token.is_word("to"))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing return destination in same-name fanout clause (clause: '{}')",
                    words_all.join(" ")
                ))
            })?;
        if to_idx <= and_idx + 3 {
            return Err(CardTextError::ParseError(format!(
                "missing same-name filter before return destination (clause: '{}')",
                words_all.join(" ")
            )));
        }
        let destination_words = words(&tokens[to_idx + 1..]);
        if !destination_words.contains(&"hand") && !destination_words.contains(&"hands") {
            return Ok(None);
        }
        tokens[and_idx + 3..to_idx].to_vec()
    } else {
        tokens[and_idx + 3..].to_vec()
    };

    if second_clause_tokens.is_empty() {
        return Ok(None);
    }

    let Some(filter) = parse_same_name_fanout_filter(&second_clause_tokens)? else {
        return Ok(None);
    };

    let mut first_target = parse_target_phrase(&first_target_tokens)?;
    if verb == "return"
        && let Some(first_filter) = target_object_filter_mut(&mut first_target)
    {
        if first_filter.zone.is_none() {
            first_filter.zone = filter.zone;
            if first_filter.zone.is_none() && words_all.contains(&"graveyard") {
                first_filter.zone = Some(Zone::Graveyard);
            }
        }
        if first_filter.owner.is_none() {
            first_filter.owner = filter.owner.clone();
            if first_filter.owner.is_none()
                && words_all
                    .windows(2)
                    .any(|window| window == ["your", "graveyard"])
            {
                first_filter.owner = Some(PlayerFilter::You);
            }
        }
    }
    let first_effect = match verb {
        "destroy" => EffectAst::Destroy {
            target: first_target,
        },
        "exile" => {
            if until_source_leaves {
                EffectAst::ExileUntilSourceLeaves {
                    target: first_target,
                    face_down: false,
                }
            } else {
                EffectAst::Exile {
                    target: first_target,
                    face_down: false,
                }
            }
        }
        "return" => EffectAst::ReturnToHand {
            target: first_target,
            random: false,
        },
        _ => unreachable!("verb already filtered"),
    };
    let second_effect = match verb {
        "destroy" => EffectAst::DestroyAll { filter },
        "exile" => {
            if until_source_leaves {
                EffectAst::ExileUntilSourceLeaves {
                    target: TargetAst::Object(filter, None, None),
                    face_down: false,
                }
            } else {
                EffectAst::ExileAll {
                    filter,
                    face_down: false,
                }
            }
        }
        "return" => EffectAst::ReturnAllToHand { filter },
        _ => unreachable!("verb already filtered"),
    };

    Ok(Some(vec![first_effect, second_effect]))
}

pub(crate) fn find_shares_color_reference_span(
    tokens: &[Token],
) -> Result<Option<(usize, usize)>, CardTextError> {
    for idx in 0..tokens.len() {
        if !tokens[idx].is_word("that") {
            continue;
        }
        if idx + 5 < tokens.len()
            && (tokens[idx + 1].is_word("shares") || tokens[idx + 1].is_word("share"))
            && tokens[idx + 2].is_word("a")
            && tokens[idx + 3].is_word("color")
            && tokens[idx + 4].is_word("with")
            && tokens[idx + 5].is_word("it")
        {
            return Ok(Some((idx, idx + 6)));
        }
        if idx + 4 < tokens.len()
            && (tokens[idx + 1].is_word("shares") || tokens[idx + 1].is_word("share"))
            && tokens[idx + 2].is_word("a")
            && tokens[idx + 3].is_word("color")
            && tokens[idx + 4].is_word("with")
        {
            return Err(CardTextError::ParseError(format!(
                "missing 'it' in shares-color clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    }
    Ok(None)
}

pub(crate) fn parse_shared_color_fanout_filter(
    tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some((share_start, share_end)) = find_shares_color_reference_span(tokens)? else {
        return Ok(None);
    };

    let mut filter_tokens = Vec::with_capacity(tokens.len());
    filter_tokens.extend_from_slice(&tokens[..share_start]);
    filter_tokens.extend_from_slice(&tokens[share_end..]);
    let filter_tokens = trim_commas(&filter_tokens);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in shared-color fanout clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported shared-color fanout filter (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SharesColorWithTagged,
    });
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    Ok(Some(filter))
}

pub(crate) fn parse_shared_color_target_fanout_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = words(tokens);
    let Some((verb, verb_idx)) = find_verb(tokens) else {
        return Ok(None);
    };
    let Some(verb_token_idx) = token_index_for_word_index(tokens, verb_idx) else {
        return Ok(None);
    };

    let find_and_each_other = |scope: &[Token]| {
        (0..scope.len().saturating_sub(2)).find(|idx| {
            scope[*idx].is_word("and")
                && scope[*idx + 1].is_word("each")
                && scope[*idx + 2].is_word("other")
        })
    };

    if matches!(verb, Verb::Destroy | Verb::Exile | Verb::Untap) {
        let after_verb = &tokens[verb_token_idx + 1..];
        let Some(split_idx) = find_and_each_other(after_verb) else {
            return Ok(None);
        };
        let first_target_tokens = trim_commas(&after_verb[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = after_verb[split_idx + 3..].to_vec();
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;
        let mut effects = Vec::with_capacity(2);
        match verb {
            Verb::Destroy => {
                effects.push(EffectAst::Destroy {
                    target: first_target,
                });
                effects.push(EffectAst::DestroyAll { filter });
            }
            Verb::Exile => {
                effects.push(EffectAst::Exile {
                    target: first_target,
                    face_down: false,
                });
                effects.push(EffectAst::ExileAll {
                    filter,
                    face_down: false,
                });
            }
            Verb::Untap => {
                effects.push(EffectAst::Untap {
                    target: first_target,
                });
                effects.push(EffectAst::UntapAll { filter });
            }
            _ => return Ok(None),
        }
        return Ok(Some(effects));
    }

    if verb == Verb::Deal {
        let after_verb = &tokens[verb_token_idx + 1..];
        let after_words = words(after_verb);
        let (amount, used) = if after_words.starts_with(&["that", "much"]) {
            (Value::EventValue(EventValueSpec::Amount), 2usize)
        } else if let Some((value, used)) = parse_value(after_verb) {
            (value, used)
        } else {
            return Ok(None);
        };

        let after_amount = &after_verb[used..];
        if !after_amount
            .first()
            .is_some_and(|token| token.is_word("damage"))
        {
            return Ok(None);
        }
        let mut target_tokens = &after_amount[1..];
        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("to"))
        {
            target_tokens = &target_tokens[1..];
        }
        if target_tokens.is_empty() {
            return Ok(None);
        }
        let Some(split_idx) = find_and_each_other(target_tokens) else {
            return Ok(None);
        };
        let first_target_tokens = trim_commas(&target_tokens[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = target_tokens[split_idx + 3..].to_vec();
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;
        return Ok(Some(vec![
            EffectAst::DealDamage {
                amount: amount.clone(),
                target: first_target,
            },
            EffectAst::DealDamageEach { amount, filter },
        ]));
    }

    if words_all.first().copied() == Some("prevent") {
        let mut idx = verb_token_idx + 1;
        if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
            idx += 1;
        }
        if !tokens.get(idx).is_some_and(|token| token.is_word("next")) {
            return Ok(None);
        }
        idx += 1;
        let amount_token = tokens.get(idx).cloned().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing prevent damage amount (clause: '{}')",
                words_all.join(" ")
            ))
        })?;
        let Some((amount, _)) = parse_value(&[amount_token]) else {
            return Ok(None);
        };
        idx += 1;
        if !tokens.get(idx).is_some_and(|token| token.is_word("damage")) {
            return Ok(None);
        }
        idx += 1;
        if tokens.get(idx..idx + 4).is_none_or(|window| {
            !window[0].is_word("that")
                || !window[1].is_word("would")
                || !window[2].is_word("be")
                || !window[3].is_word("dealt")
        }) {
            return Ok(None);
        }
        idx += 4;
        if !tokens.get(idx).is_some_and(|token| token.is_word("to")) {
            return Ok(None);
        }
        idx += 1;

        let this_turn_rel = words(&tokens[idx..])
            .windows(2)
            .position(|window| window == ["this", "turn"]);
        let Some(this_turn_rel) = this_turn_rel else {
            return Ok(None);
        };
        let this_turn_abs = idx + this_turn_rel;
        if this_turn_abs + 2 != tokens.len() {
            return Ok(None);
        }

        let scope_tokens = &tokens[idx..this_turn_abs];
        let Some(split_idx) = find_and_each_other(scope_tokens) else {
            return Ok(None);
        };

        let first_target_tokens = trim_commas(&scope_tokens[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = scope_tokens[split_idx + 3..].to_vec();
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;

        return Ok(Some(vec![
            EffectAst::PreventDamage {
                amount: amount.clone(),
                target: first_target,
                duration: Until::EndOfTurn,
            },
            EffectAst::PreventDamageEach {
                amount,
                filter,
                duration: Until::EndOfTurn,
            },
        ]));
    }

    if matches!(verb, Verb::Get | Verb::Gain) {
        if verb_idx == 0 || verb_token_idx + 1 >= tokens.len() {
            return Ok(None);
        }

        let subject_tokens = &tokens[..verb_token_idx];
        let Some(and_idx) = find_and_each_other(subject_tokens) else {
            return Ok(None);
        };
        if and_idx == 0 {
            return Ok(None);
        }

        let first_target_tokens = trim_commas(&subject_tokens[..and_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = trim_commas(&subject_tokens[and_idx + 3..]);
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;

        if verb == Verb::Get {
            let modifier_tokens = &tokens[verb_token_idx + 1..];
            let modifier_word = modifier_tokens
                .first()
                .and_then(Token::as_word)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "missing modifier in shared-color gets clause (clause: '{}')",
                        words_all.join(" ")
                    ))
                })?;
            let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
                CardTextError::ParseError(format!(
                    "invalid power/toughness modifier in shared-color gets clause (clause: '{}')",
                    words_all.join(" ")
                ))
            })?;

            return Ok(Some(vec![
                EffectAst::Pump {
                    power: Value::Fixed(power),
                    toughness: Value::Fixed(toughness),
                    target: first_target,
                    duration: Until::EndOfTurn,
                    condition: None,
                },
                EffectAst::PumpAll {
                    filter,
                    power: Value::Fixed(power),
                    toughness: Value::Fixed(toughness),
                    duration: Until::EndOfTurn,
                },
            ]));
        }

        let mut first_clause = first_target_tokens.clone();
        first_clause.extend_from_slice(&tokens[verb_token_idx..]);
        let Some(first_effect) = parse_simple_gain_ability_clause(&first_clause)? else {
            return Ok(None);
        };
        if let EffectAst::GrantAbilitiesToTarget {
            abilities,
            duration,
            ..
        } = first_effect
        {
            return Ok(Some(vec![
                EffectAst::GrantAbilitiesToTarget {
                    target: first_target,
                    abilities: abilities.clone(),
                    duration: duration.clone(),
                },
                EffectAst::GrantAbilitiesAll {
                    filter,
                    abilities,
                    duration,
                },
            ]));
        }
    }

    Ok(None)
}

pub(crate) fn parse_same_name_gets_fanout_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((verb, verb_idx)) = find_verb(tokens) else {
        return Ok(None);
    };
    if verb != Verb::Get || verb_idx == 0 || verb_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let subject_tokens = &tokens[..verb_idx];
    let and_idx = (0..subject_tokens.len().saturating_sub(2)).find(|idx| {
        subject_tokens[*idx].is_word("and")
            && subject_tokens[*idx + 1].is_word("all")
            && subject_tokens[*idx + 2].is_word("other")
    });
    let Some(and_idx) = and_idx else {
        return Ok(None);
    };
    if and_idx == 0 {
        return Ok(None);
    }

    let first_target_tokens = trim_commas(&subject_tokens[..and_idx]);
    if first_target_tokens.is_empty()
        || !first_target_tokens
            .iter()
            .any(|token| token.is_word("target"))
    {
        return Ok(None);
    }
    let second_clause_tokens = trim_commas(&subject_tokens[and_idx + 3..]);
    if second_clause_tokens.is_empty() {
        return Ok(None);
    }
    let Some(filter) = parse_same_name_fanout_filter(&second_clause_tokens)? else {
        return Ok(None);
    };

    let modifier_tokens = &tokens[verb_idx + 1..];
    let modifier_word = modifier_tokens
        .first()
        .and_then(Token::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing modifier in same-name gets clause (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;
    let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in same-name gets clause (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    let modifier_words = words(modifier_tokens);
    let duration = if modifier_words.contains(&"until")
        && modifier_words.contains(&"end")
        && modifier_words.contains(&"turn")
    {
        Until::EndOfTurn
    } else {
        Until::EndOfTurn
    };

    let target = parse_target_phrase(&first_target_tokens)?;
    Ok(Some(vec![
        EffectAst::Pump {
            power: Value::Fixed(power),
            toughness: Value::Fixed(toughness),
            target,
            duration: duration.clone(),
            condition: None,
        },
        EffectAst::PumpAll {
            filter,
            power: Value::Fixed(power),
            toughness: Value::Fixed(toughness),
            duration,
        },
    ]))
}

pub(crate) fn parse_destroy_or_exile_all_split_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    let verb = if words[0] == "destroy" {
        Some(Verb::Destroy)
    } else if words[0] == "exile" {
        Some(Verb::Exile)
    } else {
        None
    };
    let Some(verb) = verb else {
        return Ok(None);
    };
    if words[1] != "all" || !words.contains(&"and") || words.contains(&"except") {
        return Ok(None);
    }

    let mut raw_segments = Vec::new();
    let mut current = Vec::new();
    for token in &tokens[2..] {
        if token.is_word("and") || matches!(token, Token::Comma(_)) {
            if !current.is_empty() {
                raw_segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }
    if !current.is_empty() {
        raw_segments.push(current);
    }

    let mut effects = Vec::new();
    for mut segment in raw_segments {
        if segment.is_empty() {
            continue;
        }
        if segment.first().is_some_and(|token| token.is_word("all")) {
            segment.remove(0);
        }
        if segment.is_empty() {
            continue;
        }
        let filter = parse_object_filter(&segment, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in split all clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        let effect = match verb {
            Verb::Destroy => EffectAst::DestroyAll { filter },
            Verb::Exile => EffectAst::ExileAll {
                filter,
                face_down: false,
            },
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported split all clause verb".to_string(),
                ));
            }
        };
        effects.push(effect);
    }

    if effects.len() >= 2 {
        return Ok(Some(effects));
    }
    Ok(None)
}

pub(crate) fn parse_exile_then_return_same_object_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn target_references_it_tag(target: &TargetAst) -> bool {
        match target {
            TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
            TargetAst::Object(filter, _, _) => filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
            }),
            _ => false,
        }
    }

    let mut clause_tokens = tokens;
    if clause_tokens
        .first()
        .is_some_and(|token| token.is_word("you"))
        && clause_tokens
            .get(1)
            .is_some_and(|token| token.is_word("exile"))
    {
        clause_tokens = &clause_tokens[1..];
    }

    let words_all = words(clause_tokens);
    if words_all.first().copied() != Some("exile")
        || !words_all.contains(&"then")
        || !words_all.contains(&"return")
    {
        return Ok(None);
    }

    let split_idx = (0..clause_tokens.len().saturating_sub(2)).find(|idx| {
        matches!(clause_tokens[*idx], Token::Comma(_))
            && clause_tokens[*idx + 1].is_word("then")
            && clause_tokens[*idx + 2].is_word("return")
    });
    let Some(split_idx) = split_idx else {
        return Ok(None);
    };

    let first_clause = &clause_tokens[..split_idx];
    let second_clause = &clause_tokens[split_idx + 2..];
    if first_clause.is_empty() || second_clause.is_empty() {
        return Ok(None);
    }

    let mut first_effects = parse_effect_chain_inner(first_clause)?;
    if !first_effects
        .iter()
        .any(|effect| matches!(effect, EffectAst::Exile { .. }))
    {
        return Ok(None);
    }

    // Preserve return follow-up clauses (for example "with a +1/+1 counter on it")
    // while still rewriting the "it" return target to the tagged exiled object.
    let mut second_effects = parse_effect_chain_inner(second_clause)?;
    let mut rewrote_return = false;
    for effect in &mut second_effects {
        match effect {
            EffectAst::ReturnToBattlefield {
                target,
                tapped: _,
                controller: _,
            } if target_references_it_tag(target) => {
                *target = TargetAst::Tagged(TagKey::from("exiled_0"), None);
                rewrote_return = true;
            }
            EffectAst::ReturnToHand { target, random: _ } if target_references_it_tag(target) => {
                *target = TargetAst::Tagged(TagKey::from("exiled_0"), None);
                rewrote_return = true;
            }
            _ => {}
        }
    }
    if !rewrote_return {
        return Ok(None);
    }

    first_effects.extend(second_effects);
    Ok(Some(first_effects))
}

pub(crate) fn parse_exile_up_to_one_each_target_type_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    if words.len() < 6 || words[0] != "exile" {
        return Ok(None);
    }
    if !words.starts_with(&["exile", "up", "to", "one", "target"]) {
        return Ok(None);
    }
    // This primitive is for repeated clauses like:
    // "Exile up to one target artifact, up to one target creature, ..."
    // Not for a single disjunctive target like:
    // "Exile up to one target artifact, creature, or enchantment ..."
    let target_positions: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| token.is_word("target").then_some(idx))
        .collect();
    if target_positions.len() < 2 {
        return Ok(None);
    }
    for pos in target_positions.iter().skip(1) {
        if *pos < 3
            || !tokens[*pos - 3].is_word("up")
            || !tokens[*pos - 2].is_word("to")
            || !tokens[*pos - 1].is_word("one")
        {
            return Ok(None);
        }
    }

    let mut raw_segments: Vec<Vec<Token>> = Vec::new();
    let mut current: Vec<Token> = Vec::new();
    for token in &tokens[1..] {
        if matches!(token, Token::Comma(_)) || token.is_word("and") || token.is_word("or") {
            if !current.is_empty() {
                raw_segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }
    if !current.is_empty() {
        raw_segments.push(current);
    }

    let mut filters = Vec::new();
    for segment in raw_segments {
        let mut slice: &[Token] = &segment;
        if slice.len() >= 3
            && slice[0].is_word("up")
            && slice[1].is_word("to")
            && slice[2].is_word("one")
        {
            slice = &slice[3..];
        }
        if slice.first().is_some_and(|token| token.is_word("target")) {
            slice = &slice[1..];
        }
        if slice.is_empty() {
            continue;
        }

        let mut filter = parse_object_filter(slice, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in 'exile up to one each target type' clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        if filter.controller.is_none() {
            // Keep this unrestricted to avoid implicit "you control" defaulting in ChooseObjects compilation.
            filter.controller = Some(PlayerFilter::Any);
        }
        filters.push(filter);
    }

    if filters.len() < 2 {
        return Ok(None);
    }

    let tag = TagKey::from("exiled_0");
    let mut effects: Vec<EffectAst> = filters
        .into_iter()
        .map(|filter| EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::up_to(1),
            player: PlayerAst::You,
            tag: tag.clone(),
        })
        .collect();
    effects.push(EffectAst::Exile {
        target: TargetAst::Tagged(tag, None),
        face_down: false,
    });

    Ok(Some(effects))
}

pub(crate) fn parse_look_at_hand_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["look", "at", "target", "players", "hand"]
        || words.as_slice() == ["look", "at", "target", "player", "hand"]
    {
        let target = TargetAst::Player(PlayerFilter::target_player(), Some(TextSpan::synthetic()));
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    if words.as_slice() == ["look", "at", "target", "opponent", "hand"]
        || words.as_slice() == ["look", "at", "target", "opponents", "hand"]
    {
        let target =
            TargetAst::Player(PlayerFilter::target_opponent(), Some(TextSpan::synthetic()));
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    Ok(None)
}

pub(crate) fn parse_look_at_top_then_exile_one_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    let starts_with_look_top = clause_words.starts_with(&["look", "at", "the", "top"])
        || clause_words.starts_with(&["look", "at", "top"]);
    if !starts_with_look_top {
        return Ok(None);
    }

    let Some(top_idx) = tokens.iter().position(|token| token.is_word("top")) else {
        return Ok(None);
    };
    let Some((count, used_count)) = parse_number(&tokens[top_idx + 1..]) else {
        return Ok(None);
    };
    let mut idx = top_idx + 1 + used_count;
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
    {
        idx += 1;
    }
    if !tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        return Ok(None);
    }
    idx += 1;

    let Some(library_idx) = tokens[idx..]
        .iter()
        .position(|token| token.is_word("library"))
        .map(|offset| idx + offset)
    else {
        return Ok(None);
    };
    let owner_tokens = trim_commas(&tokens[idx..library_idx]);
    if owner_tokens.is_empty() {
        return Ok(None);
    }
    let player = match parse_subject(&owner_tokens) {
        SubjectAst::Player(player) => player,
        _ => return Ok(None),
    };

    let mut tail_tokens = trim_commas(&tokens[library_idx + 1..]).to_vec();
    while tail_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        tail_tokens.remove(0);
    }
    let tail_words = words(&tail_tokens);
    let looks_like_exile_one_of_looked = tail_words.starts_with(&["exile", "one", "of", "them"])
        || tail_words.starts_with(&["exile", "one", "of", "those"])
        || tail_words.starts_with(&["exile", "one", "of", "those", "cards"]);
    if !looks_like_exile_one_of_looked {
        return Ok(None);
    }

    let looked_tag = TagKey::from("looked_0");
    let chosen_tag = TagKey::from("chosen_0");
    let mut looked_filter = ObjectFilter::tagged(looked_tag.clone());
    looked_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::LookAtTopCards {
            player,
            count: Value::Fixed(count as i32),
            tag: looked_tag,
        },
        EffectAst::ChooseObjects {
            filter: looked_filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: chosen_tag.clone(),
        },
        EffectAst::Exile {
            target: TargetAst::Tagged(chosen_tag, None),
            face_down: false,
        },
    ]))
}

pub(crate) fn parse_gain_life_equal_to_age_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Legacy fallback previously returned a hardcoded 0-life effect for age-counter clauses.
    // Let generic life parsing handle these so counter-scaled amounts compile correctly.
    let _ = tokens;
    Ok(None)
}

pub(crate) fn parse_you_and_each_opponent_voted_with_you_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    let pattern = [
        "you", "and", "each", "opponent", "who", "voted", "for", "a", "choice", "you", "voted",
        "for", "may", "scry",
    ];

    if words.len() < pattern.len() {
        return Ok(None);
    }

    if !words.starts_with(&pattern) {
        return Ok(None);
    }

    let scry_index = pattern.len() - 1;
    let value_tokens = &tokens[(scry_index + 1)..];
    let Some((count, _)) = parse_value(value_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "missing scry count in vote-with-you clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let you_effect = EffectAst::May {
        effects: vec![EffectAst::Scry {
            count: count.clone(),
            player: PlayerAst::You,
        }],
    };

    let opponent_effect = EffectAst::ForEachTaggedPlayer {
        tag: TagKey::from("voted_with_you"),
        effects: vec![EffectAst::May {
            effects: vec![EffectAst::Scry {
                count,
                player: PlayerAst::Implicit,
            }],
        }],
    };

    Ok(Some(vec![you_effect, opponent_effect]))
}

pub(crate) fn parse_gain_life_equal_to_power_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    let Some(gain_idx) = words
        .iter()
        .position(|word| *word == "gain" || *word == "gains")
    else {
        return Ok(None);
    };

    if words.get(gain_idx + 1) != Some(&"life")
        || words.get(gain_idx + 2) != Some(&"equal")
        || words.get(gain_idx + 3) != Some(&"to")
    {
        return Ok(None);
    }

    let tail = &words[gain_idx + 4..];
    let has_its_power = tail.windows(2).any(|pair| pair == ["its", "power"]);
    if !has_its_power {
        return Ok(None);
    }

    let subject = if gain_idx > 0 {
        Some(parse_subject(&tokens[..gain_idx]))
    } else {
        None
    };
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let amount = Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))));
    Ok(Some(vec![EffectAst::GainLife { amount, player }]))
}

pub(crate) fn parse_double_target_power_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    if !words.starts_with(&["double", "target"]) {
        return Ok(None);
    }

    let Some(power_idx) = words.iter().position(|word| *word == "power") else {
        return Ok(None);
    };
    if power_idx <= 1 {
        return Ok(None);
    }

    let tail_words = &words[power_idx + 1..];
    if !tail_words.is_empty() && !is_until_end_of_turn(tail_words) {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..power_idx]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in double-power clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let target = parse_target_phrase(&target_tokens)?;
    let amount_source_filter = target_ast_to_object_filter(target.clone()).unwrap_or_else(|| {
        let mut fallback = ObjectFilter::default();
        fallback.card_types.push(CardType::Creature);
        fallback
    });
    Ok(Some(vec![EffectAst::Pump {
        power: Value::PowerOf(Box::new(ChooseSpec::target(ChooseSpec::Object(
            amount_source_filter,
        )))),
        toughness: Value::Fixed(0),
        target,
        duration: crate::effect::Until::EndOfTurn,
        condition: None,
    }]))
}

pub(crate) fn parse_prevent_damage_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    let prefix = ["prevent", "all", "combat", "damage"];
    if !words.starts_with(&prefix) {
        return Ok(None);
    }

    let this_turn_positions: Vec<usize> = words
        .windows(2)
        .enumerate()
        .filter_map(|(idx, pair)| (pair == ["this", "turn"]).then_some(idx))
        .collect();
    if this_turn_positions.len() != 1 {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    }
    let this_turn_idx = this_turn_positions[0];
    if this_turn_idx < prefix.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    }

    let mut core_words = Vec::with_capacity(words.len() - prefix.len() - 2);
    core_words.extend_from_slice(&words[prefix.len()..this_turn_idx]);
    core_words.extend_from_slice(&words[this_turn_idx + 2..]);
    let mut core_tokens = Vec::with_capacity(tokens.len() - prefix.len() - 2);
    core_tokens.extend_from_slice(&tokens[prefix.len()..this_turn_idx]);
    core_tokens.extend_from_slice(&tokens[this_turn_idx + 2..]);
    let core_words = core_words;
    let core_tokens = core_tokens;

    if core_words == ["that", "would", "be", "dealt"] {
        return Ok(Some(EffectAst::PreventAllCombatDamage {
            duration: Until::EndOfTurn,
        }));
    }

    if core_words.starts_with(&["that", "would", "be", "dealt", "by"]) {
        let source_tokens = &core_tokens[5..];
        let source = parse_prevent_damage_source_target(source_tokens, &words)?;
        return Ok(Some(EffectAst::PreventAllCombatDamageFromSource {
            duration: Until::EndOfTurn,
            source,
        }));
    }

    if core_words.starts_with(&["that", "would", "be", "dealt", "to", "and", "dealt", "by"]) {
        let source_tokens = &core_tokens[8..];
        let source = parse_prevent_damage_source_target(source_tokens, &words)?;
        return Ok(Some(EffectAst::PreventAllCombatDamageFromSource {
            duration: Until::EndOfTurn,
            source,
        }));
    }

    if core_words.starts_with(&["that", "would", "be", "dealt", "to"]) {
        return parse_prevent_damage_target_scope(&core_tokens[5..], &words);
    }

    if let Some(would_idx) = core_words.iter().position(|word| *word == "would")
        && core_words.get(would_idx + 1) == Some(&"deal")
    {
        let source_tokens = &core_tokens[..would_idx];
        let source = parse_prevent_damage_source_target(source_tokens, &words)?;
        return Ok(Some(EffectAst::PreventAllCombatDamageFromSource {
            duration: Until::EndOfTurn,
            source,
        }));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported prevent-all-combat-damage clause tail (clause: '{}')",
        words.join(" ")
    )))
}

pub(crate) fn parse_prevent_damage_source_target(
    tokens: &[Token],
    clause_words: &[&str],
) -> Result<TargetAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all source target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let source_words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let is_explicit_reference = source_words.contains(&"target")
        || source_words
            .first()
            .is_some_and(|word| matches!(*word, "this" | "that" | "it"));
    if !is_explicit_reference {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all source target '{}'",
            source_words.join(" ")
        )));
    }

    let source = parse_target_phrase(tokens)?;
    match source {
        TargetAst::Source(_) | TargetAst::Object(_, _, _) | TargetAst::Tagged(_, _) => Ok(source),
        _ => Err(CardTextError::ParseError(format!(
            "unsupported prevent-all source target '{}'",
            words(tokens).join(" ")
        ))),
    }
}

pub(crate) fn parse_prevent_damage_target_scope(
    tokens: &[Token],
    clause_words: &[&str],
) -> Result<Option<EffectAst>, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all target scope (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if target_words.as_slice() == ["player"] || target_words.as_slice() == ["players"] {
        return Ok(Some(EffectAst::PreventAllCombatDamageToPlayers {
            duration: Until::EndOfTurn,
        }));
    }
    if target_words.as_slice() == ["you"] {
        return Ok(Some(EffectAst::PreventAllCombatDamageToYou {
            duration: Until::EndOfTurn,
        }));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported prevent-all target scope '{}'",
        words(tokens).join(" ")
    )))
}

pub(crate) fn parse_gain_x_plus_life_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    let Some(gain_idx) = words
        .iter()
        .position(|word| *word == "gain" || *word == "gains")
    else {
        return Ok(None);
    };

    if words.len() <= gain_idx + 4 {
        return Ok(None);
    }

    if words[gain_idx + 1] != "x" || words[gain_idx + 2] != "plus" {
        return Ok(None);
    }

    let (bonus, number_used) = parse_number(&tokens[gain_idx + 3..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life gain amount (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let life_idx = gain_idx + 3 + number_used;
    if !tokens
        .get(life_idx)
        .is_some_and(|token| token.is_word("life"))
    {
        return Err(CardTextError::ParseError(format!(
            "missing life keyword in gain-x-plus-life clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let subject_tokens = &tokens[..gain_idx];
    let player = match parse_subject(subject_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };

    let trailing_tokens = trim_commas(&tokens[life_idx + 1..]);
    let x_value = if trailing_tokens.is_empty() {
        Value::X
    } else if let Some(where_x) = parse_where_x_value_clause(&trailing_tokens) {
        where_x
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported gain-x-plus-life trailing clause (clause: '{}')",
            words.join(" ")
        )));
    };
    let amount = Value::Add(Box::new(x_value), Box::new(Value::Fixed(bonus as i32)));
    let effects = vec![EffectAst::GainLife { amount, player }];

    Ok(Some(effects))
}
