use winnow::Parser as _;

use super::super::clause_support::parse_triggered_line_lexed;
use super::super::grammar::primitives as grammar;
use super::super::lexer::{LexStream, token_word_refs};
use super::super::lowering_support::rewrite_parsed_triggered_ability as parsed_triggered_ability;
use super::super::object_filters::parse_object_filter;
use super::super::permission_helpers::{
    parse_additional_land_plays_clause, parse_cast_or_play_tagged_clause,
    parse_cast_spells_as_though_they_had_flash_clause,
    parse_unsupported_play_cast_permission_clause, parse_until_end_of_turn_may_play_tagged_clause,
    parse_until_your_next_turn_may_play_tagged_clause,
};
use super::super::token_primitives::{
    find_index as find_token_index, slice_starts_with as word_slice_starts_with,
};
use super::super::util::{
    is_article, parse_subject, parse_target_phrase, span_from_tokens, trim_commas,
};
use super::sentence_helpers::*;
use super::{parse_mana_symbol, parse_restriction_duration};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, ClashOpponentAst, EffectAst, GrantedAbilityAst, IT_TAG, LineAst, OwnedLexToken,
    PlayerAst, PredicateAst, ReferenceImports, RetargetModeAst, SubjectAst, TagKey, TargetAst,
    TextSpan, TriggerSpec,
};
use crate::effect::ChoiceCount;
use crate::mana::ManaSymbol;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

pub(crate) type ClausePrimitiveParser =
    fn(&[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError>;

pub(crate) struct ClausePrimitive {
    pub(crate) parser: ClausePrimitiveParser,
}

const CHOSEN_NAME_TAG: &str = "__chosen_name__";
const CHOOSE_NEW_TARGET_PREFIXES: &[&[&str]] = &[
    &["choose", "new", "targets", "for"],
    &["chooses", "new", "targets", "for"],
    &["choose", "a", "new", "target", "for"],
    &["chooses", "a", "new", "target", "for"],
];
const CHOOSE_NEW_TARGET_REFERENCE_PREFIXES: &[&[&str]] = &[
    &["it"],
    &["them"],
    &["the", "copy"],
    &["that", "copy"],
    &["the", "spell"],
    &["that", "spell"],
];
const CHANGE_TARGET_PREFIXES: &[&[&str]] = &[
    &["change", "the", "target", "of"],
    &["change", "the", "targets", "of"],
    &["change", "a", "target", "of"],
];
const CHOOSE_CARD_NAME_PREFIXES: &[&[&str]] = &[
    &["choose"],
    &["you", "choose"],
    &["that", "player", "chooses"],
];
const ALL_CREATURES_ABLE_TO_BLOCK_PREFIXES: &[&[&str]] =
    &[&["all", "creatures", "able", "to", "block"]];
const UNTIL_DURATION_TRIGGER_PREFIXES: &[&[&str]] = &[
    &["until", "your", "next", "turn"],
    &["until", "your", "next", "upkeep"],
    &["until", "your", "next", "untap", "step"],
    &["during", "your", "next", "untap", "step"],
];
const AT_THE_PREFIXES: &[&[&str]] = &[&["at", "the"]];
const EACH_OF_PREFIXES: &[&[&str]] = &[&["each", "of"]];
const DAMAGE_TO_PREFIXES: &[&[&str]] = &[&["damage", "to"]];

fn render_clause_words(tokens: &[OwnedLexToken]) -> String {
    token_word_refs(tokens).join(" ")
}

pub(crate) fn parse_retarget_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if let Some(effect) = parse_choose_new_targets_clause(tokens)? {
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_change_target_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}

pub(crate) fn parse_choose_new_targets_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some((_, mut tail_tokens)) =
        grammar::strip_lexed_prefix_phrases(tokens, CHOOSE_NEW_TARGET_PREFIXES)
    else {
        return Ok(None);
    };
    if tail_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing choose-new-targets target".to_string(),
        ));
    }

    if let Some(if_idx) = find_token_index(tail_tokens, |token| token.is_word("if")) {
        tail_tokens = &tail_tokens[..if_idx];
    }

    if grammar::starts_with_any_phrase(tail_tokens, CHOOSE_NEW_TARGET_REFERENCE_PREFIXES) {
        let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tail_tokens));
        return Ok(Some(EffectAst::RetargetStackObject {
            target,
            mode: RetargetModeAst::All,
            chooser: PlayerAst::Implicit,
            require_change: false,
        }));
    }

    let (count, base_tokens, explicit_target) = if let Some((prefix, rest)) =
        grammar::strip_lexed_prefix_phrases(tail_tokens, &[&["any", "number", "of"], &["target"]])
    {
        if prefix.len() == 3 {
            (Some(ChoiceCount::any_number()), rest, false)
        } else {
            (None, rest, true)
        }
    } else {
        (None, tail_tokens, false)
    };

    let mut filter = parse_stack_retarget_filter(base_tokens)?;
    if base_tokens.iter().any(|token| token.is_word("other")) {
        filter.other = true;
    }

    let mut target = TargetAst::Object(
        filter,
        if explicit_target {
            span_from_tokens(tail_tokens)
        } else {
            None
        },
        None,
    );
    if let Some(count) = count {
        target = TargetAst::WithCount(Box::new(target), count);
    }

    Ok(Some(EffectAst::RetargetStackObject {
        target,
        mode: RetargetModeAst::All,
        chooser: PlayerAst::Implicit,
        require_change: false,
    }))
}

pub(crate) fn parse_change_target_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = token_word_refs(tokens);
    if clause_words.is_empty() || clause_words[0] != "change" {
        return Ok(None);
    }

    if let Some((main_slice, unless_slice)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("unless").void()
        })
    {
        let main_tokens = trim_commas(main_slice);
        let unless_tokens = trim_commas(unless_slice);
        let Some(inner) = parse_change_target_clause_inner(&main_tokens)? else {
            return Ok(None);
        };
        let (player, mana) = parse_unless_pays_clause(&unless_tokens)?;
        return Ok(Some(EffectAst::UnlessPays {
            effects: vec![inner],
            player,
            mana,
        }));
    }

    parse_change_target_clause_inner(tokens)
}

pub(crate) fn parse_change_target_clause_inner(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some((_, after_prefix_tokens)) =
        grammar::strip_lexed_prefix_phrases(tokens, CHANGE_TARGET_PREFIXES)
    else {
        return Ok(None);
    };

    if after_prefix_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing target after change-the-target clause".to_string(),
        ));
    }

    let mut tail_tokens = trim_commas(after_prefix_tokens).to_vec();
    let mut fixed_target: Option<TargetAst> = None;
    if let Some((before_to, to_tail)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(&tail_tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("to").void()
        })
    {
        if to_tail.first().is_some_and(|t| t.is_word("this")) {
            fixed_target = Some(TargetAst::Source(span_from_tokens(to_tail)));
            tail_tokens.truncate(before_to.len());
        }
    }

    let mut filter = parse_stack_retarget_filter(&tail_tokens)?;

    if grammar::words_find_phrase(&tail_tokens, &["with", "a", "single", "target"]).is_some() {
        filter = filter.target_count_exact(1);
    }
    if grammar::words_find_phrase(
        &tail_tokens,
        &["targets", "only", "a", "single", "creature"],
    )
    .is_some()
    {
        filter = filter
            .targeting_only_object(ObjectFilter::creature())
            .target_count_exact(1);
    }
    if grammar::words_find_phrase(&tail_tokens, &["targets", "only", "this", "creature"]).is_some()
        || grammar::words_find_phrase(&tail_tokens, &["targets", "only", "this", "permanent"])
            .is_some()
    {
        filter = filter
            .targeting_only_object(ObjectFilter::source())
            .target_count_exact(1);
    }
    if grammar::words_find_phrase(&tail_tokens, &["targets", "only", "you"]).is_some() {
        filter = filter
            .targeting_only_player(PlayerFilter::You)
            .target_count_exact(1);
    }
    if grammar::words_find_phrase(&tail_tokens, &["targets", "only", "a", "player"]).is_some() {
        filter = filter
            .targeting_only_player(PlayerFilter::Any)
            .target_count_exact(1);
    }
    if grammar::words_find_phrase(&tail_tokens, &["if", "that", "target", "is", "you"]).is_some() {
        filter = filter
            .targeting_only_player(PlayerFilter::You)
            .target_count_exact(1);
    }

    let target = TargetAst::Object(filter, span_from_tokens(tokens), None);

    let mode = if let Some(fixed) = fixed_target {
        RetargetModeAst::OneToFixed { target: fixed }
    } else {
        RetargetModeAst::All
    };

    Ok(Some(EffectAst::RetargetStackObject {
        target,
        mode,
        chooser: PlayerAst::Implicit,
        require_change: true,
    }))
}

pub(crate) fn parse_unless_pays_clause(
    tokens: &[OwnedLexToken],
) -> Result<(PlayerAst, Vec<ManaSymbol>), CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing unless clause".to_string(),
        ));
    }
    let (player_slice, pays_tail) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("pays").void()
        })
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing pays keyword (clause: '{}')",
                render_clause_words(tokens)
            ))
        })?;
    let _ = pays_tail; // used below via tokens[pays_idx + 1..]
    let pays_idx = player_slice.len();

    let player_tokens = trim_commas(player_slice);
    let player = match parse_subject(&player_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };

    let mana_slice = &tokens[pays_idx + 1..];
    let (mana, trailing_start) = {
        use winnow::combinator::repeat;
        use winnow::prelude::*;

        let mut stream = LexStream::new(mana_slice);
        let symbols: Vec<_> = repeat(0.., grammar::mana_symbol_token)
            .parse_next(&mut stream)
            .unwrap_or_default();
        let consumed = mana_slice.len() - stream.len();
        let trailing = if consumed < mana_slice.len() {
            Some(pays_idx + 1 + consumed)
        } else {
            None
        };
        (symbols, trailing)
    };

    if mana.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing mana cost (clause: '{}')",
            render_clause_words(tokens)
        )));
    }

    if let Some(start) = trailing_start {
        let trailing_tokens = trim_commas(&tokens[start..]);
        let trailing_words = token_word_refs(&trailing_tokens);
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing unless-payment clause (clause: '{}', trailing: '{}')",
                render_clause_words(tokens),
                trailing_words.join(" ")
            )));
        }
    }

    Ok((player, mana))
}

pub(crate) fn parse_stack_retarget_filter(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let words = token_word_refs(tokens);
    let has_ability = words
        .iter()
        .any(|word| matches!(*word, "ability" | "abilities"));
    let has_spell = words.iter().any(|word| matches!(*word, "spell" | "spells"));
    let has_activated = grammar::contains_word(tokens, "activated");
    let has_instant = grammar::contains_word(tokens, "instant");
    let has_sorcery = grammar::contains_word(tokens, "sorcery");

    let mut filter = if has_activated && has_ability {
        ObjectFilter::activated_ability()
    } else if has_ability && has_spell {
        ObjectFilter::spell_or_ability()
    } else if has_ability {
        ObjectFilter::ability()
    } else if (has_instant || has_sorcery) && has_spell {
        ObjectFilter::instant_or_sorcery()
    } else if has_spell {
        ObjectFilter::spell()
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported retarget target clause (clause: '{}')",
            words.join(" ")
        )));
    };

    if grammar::contains_word(tokens, "other") {
        filter.other = true;
    }

    Ok(filter)
}

pub(crate) fn run_clause_primitives(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    const PRIMITIVES: &[ClausePrimitive] = &[
        ClausePrimitive {
            parser: parse_choose_card_name_clause,
        },
        ClausePrimitive {
            parser: parse_repeat_this_process_clause,
        },
        ClausePrimitive {
            parser: parse_dont_lose_this_mana_as_steps_and_phases_end_clause,
        },
        ClausePrimitive {
            parser: parse_retarget_clause,
        },
        ClausePrimitive {
            parser: parse_copy_spell_clause,
        },
        ClausePrimitive {
            parser: parse_win_the_game_clause,
        },
        ClausePrimitive {
            parser: parse_deal_damage_equal_to_power_clause,
        },
        ClausePrimitive {
            parser: parse_fight_clause,
        },
        ClausePrimitive {
            parser: parse_clash_clause,
        },
        ClausePrimitive {
            parser: parse_for_each_target_players_clause,
        },
        ClausePrimitive {
            parser: parse_for_each_opponent_clause,
        },
        ClausePrimitive {
            parser: parse_for_each_player_clause,
        },
        ClausePrimitive {
            parser: parse_double_counters_clause,
        },
        ClausePrimitive {
            parser: parse_distribute_counters_clause,
        },
        ClausePrimitive {
            parser: parse_until_end_of_turn_may_play_tagged_clause,
        },
        ClausePrimitive {
            parser: parse_until_your_next_turn_may_play_tagged_clause,
        },
        ClausePrimitive {
            parser: parse_additional_land_plays_clause,
        },
        ClausePrimitive {
            parser: parse_cast_spells_as_though_they_had_flash_clause,
        },
        ClausePrimitive {
            parser: parse_unsupported_play_cast_permission_clause,
        },
        ClausePrimitive {
            parser: parse_cast_or_play_tagged_clause,
        },
        ClausePrimitive {
            parser: parse_prevent_next_damage_clause,
        },
        ClausePrimitive {
            parser: parse_prevent_all_damage_clause,
        },
        ClausePrimitive {
            parser: parse_can_attack_as_though_no_defender_clause,
        },
        ClausePrimitive {
            parser: parse_can_block_additional_creature_this_turn_clause,
        },
        ClausePrimitive {
            parser: parse_attack_or_block_this_turn_if_able_clause,
        },
        ClausePrimitive {
            parser: parse_attack_this_turn_if_able_clause,
        },
        ClausePrimitive {
            parser: parse_must_be_blocked_if_able_clause,
        },
        ClausePrimitive {
            parser: parse_must_block_if_able_clause,
        },
        ClausePrimitive {
            parser: parse_until_duration_triggered_clause,
        },
        ClausePrimitive {
            parser: parse_keyword_mechanic_clause,
        },
        ClausePrimitive {
            parser: parse_connive_clause,
        },
        ClausePrimitive {
            parser: parse_choose_target_and_verb_clause,
        },
        ClausePrimitive {
            parser: parse_verb_first_clause,
        },
    ];

    for primitive in PRIMITIVES {
        if let Some(effect) = (primitive.parser)(tokens)? {
            return Ok(Some(effect));
        }
    }
    Ok(None)
}

pub(crate) fn parse_choose_card_name_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = token_word_refs(tokens);
    if clause_words.len() < 3 {
        return Ok(None);
    }

    let (player, prefix_len) = if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, CHOOSE_CARD_NAME_PREFIXES)
    {
        let player = if prefix == &["that", "player", "chooses"] {
            PlayerAst::That
        } else {
            PlayerAst::You
        };
        (player, prefix.len())
    } else {
        return Ok(None);
    };

    if clause_words.len() < prefix_len + 2
        || clause_words[clause_words.len() - 2..] != ["card", "name"]
    {
        return Ok(None);
    }

    let filter_words = clause_words[prefix_len..clause_words.len() - 2]
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    let filter = if filter_words.is_empty() || filter_words.as_slice() == ["any"] {
        None
    } else {
        let normalized_tokens: Vec<OwnedLexToken> = filter_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect();
        Some(parse_object_filter(&normalized_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported choose-card-name filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?)
    };

    Ok(Some(EffectAst::ChooseCardName {
        player,
        filter,
        tag: TagKey::from(CHOSEN_NAME_TAG),
    }))
}

pub(crate) fn parse_repeat_this_process_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = token_word_refs(tokens);
    if matches!(
        clause_words.as_slice(),
        ["repeat", "this", "process", "any", "number", "of", "times"]
            | [
                "and", "repeat", "this", "process", "any", "number", "of", "times",
            ]
            | [
                "you", "may", "repeat", "this", "process", "any", "number", "of", "times"
            ]
            | [
                "and", "you", "may", "repeat", "this", "process", "any", "number", "of", "times",
            ]
    ) {
        return Ok(Some(EffectAst::RepeatThisProcessMay));
    }
    if matches!(
        clause_words.as_slice(),
        ["repeat", "this", "process"] | ["and", "repeat", "this", "process"]
    ) {
        return Ok(Some(EffectAst::RepeatThisProcess));
    }
    if matches!(
        clause_words.as_slice(),
        ["repeat", "this", "process", "once"] | ["and", "repeat", "this", "process", "once"]
    ) {
        return Ok(Some(EffectAst::RepeatThisProcessOnce));
    }
    Ok(None)
}

pub(crate) fn parse_dont_lose_this_mana_as_steps_and_phases_end_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = token_word_refs(tokens);
    if matches!(
        clause_words.as_slice(),
        [
            "you", "dont", "lose", "this", "mana", "as", "steps", "and", "phases", "end"
        ] | [
            "you", "don't", "lose", "this", "mana", "as", "steps", "and", "phases", "end",
        ]
    ) {
        return Ok(Some(EffectAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn));
    }
    Ok(None)
}

pub(crate) fn parse_attack_or_block_this_turn_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    use super::super::grammar::primitives as grammar;

    let clause_words = token_word_refs(tokens);
    let suffix = grammar::strip_lexed_suffix_phrases(
        tokens,
        &[
            &["attack", "or", "block", "this", "turn", "if", "able"],
            &["attacks", "or", "blocks", "this", "turn", "if", "able"],
            &["attacks", "or", "block", "this", "turn", "if", "able"],
            &["attack", "or", "blocks", "this", "turn", "if", "able"],
        ],
    );
    let Some((_matched, subject_part)) = suffix else {
        return Ok(None);
    };

    let subject_tokens = trim_commas(subject_part);
    let target = if subject_tokens.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens))
    } else {
        parse_target_phrase(&subject_tokens)?
    };
    let abilities = vec![GrantedAbilityAst::MustAttack, GrantedAbilityAst::MustBlock];

    if subject_tokens.is_empty() || starts_with_target_indicator(&subject_tokens) {
        return Ok(Some(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration: Until::EndOfTurn,
        }));
    }

    let filter = target_ast_to_object_filter(target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker/blocker subject in attacks-or-blocks-if-able clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::GrantAbilitiesAll {
        filter,
        abilities,
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_attack_this_turn_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use super::super::grammar::primitives as grammar;
    use crate::effect::Until;

    let clause_words = token_word_refs(tokens);
    // Try splitting on "attack(s) this turn if able" suffix
    let suffix =
        grammar::strip_lexed_suffix_phrase(tokens, &["attack", "this", "turn", "if", "able"])
            .or_else(|| {
                grammar::strip_lexed_suffix_phrase(
                    tokens,
                    &["attacks", "this", "turn", "if", "able"],
                )
            });
    let Some(subject_part) = suffix else {
        return Ok(None);
    };

    let subject_tokens = trim_commas(subject_part);
    let target = if subject_tokens.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens))
    } else {
        parse_target_phrase(&subject_tokens)?
    };
    let ability = GrantedAbilityAst::MustAttack;

    if subject_tokens.is_empty() || starts_with_target_indicator(&subject_tokens) {
        return Ok(Some(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities: vec![ability],
            duration: Until::EndOfTurn,
        }));
    }

    let filter = target_ast_to_object_filter(target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker subject in attacks-if-able clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::GrantAbilitiesAll {
        filter,
        abilities: vec![ability],
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_must_be_blocked_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    use super::super::grammar::primitives as grammar;

    let clause_words = token_word_refs(tokens);
    let suffix =
        grammar::strip_lexed_suffix_phrase(tokens, &["must", "be", "blocked", "if", "able"])
            .or_else(|| {
                grammar::strip_lexed_suffix_phrase(
                    tokens,
                    &["must", "be", "blocked", "this", "turn", "if", "able"],
                )
            });
    let Some(subject_part) = suffix else {
        return Ok(None);
    };

    let subject_tokens = trim_commas(subject_part);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if starts_with_target_indicator(&subject_tokens) {
        // We only support source/tagged subjects here; explicit "target ..." needs
        // a target+restriction sequence that this single-clause parser cannot encode.
        return Ok(None);
    }

    let attacker_target = parse_target_phrase(&subject_tokens)?;
    let attacker_filter = target_ast_to_object_filter(attacker_target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker subject in must-be-blocked clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::Cant {
        restriction: crate::effect::Restriction::must_block_specific_attacker(
            ObjectFilter::creature(),
            attacker_filter,
        ),
        duration: Until::EndOfTurn,
        condition: None,
    }))
}

pub(crate) fn parse_must_block_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause_words = token_word_refs(tokens);

    // "<subject> blocks this turn if able."
    let Some(block_idx) = find_token_index(tokens, |token| {
        token.is_word("block") || token.is_word("blocks")
    }) else {
        return Ok(None);
    };
    if block_idx == 0 || block_idx + 1 >= tokens.len() {
        return Ok(None);
    }
    let tail_words = token_word_refs(&tokens[block_idx..]);
    if tail_words == ["block", "this", "turn", "if", "able"]
        || tail_words == ["blocks", "this", "turn", "if", "able"]
    {
        let subject_tokens = trim_commas(&tokens[..block_idx]);
        if subject_tokens.is_empty() {
            return Ok(None);
        }
        let target = parse_target_phrase(&subject_tokens)?;
        let ability = GrantedAbilityAst::MustBlock;

        if starts_with_target_indicator(&subject_tokens) {
            return Ok(Some(EffectAst::GrantAbilitiesToTarget {
                target,
                abilities: vec![ability],
                duration: Until::EndOfTurn,
            }));
        }

        let filter = target_ast_to_object_filter(target).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported blocker subject in blocks-if-able clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        return Ok(Some(EffectAst::GrantAbilitiesAll {
            filter,
            abilities: vec![ability],
            duration: Until::EndOfTurn,
        }));
    }

    // "All creatures able to block target creature this turn do so."
    if grammar::words_match_any_prefix(tokens, ALL_CREATURES_ABLE_TO_BLOCK_PREFIXES).is_some() {
        let mut tail_tokens = trim_commas(&tokens[5..]);
        if grammar::words_match_suffix(&tail_tokens, &["do", "so"]).is_none() {
            return Ok(None);
        }
        tail_tokens = trim_commas(&tail_tokens[..tail_tokens.len().saturating_sub(2)]);

        let (duration, attacker_tokens) =
            if let Some((duration, remainder)) = parse_restriction_duration(&tail_tokens)? {
                (duration, remainder)
            } else {
                (Until::EndOfTurn, tail_tokens.to_vec())
            };
        let attacker_tokens = trim_commas(&attacker_tokens);
        if attacker_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing attacker in must-block clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let attacker_target = parse_target_phrase(&attacker_tokens)?;
        let attacker_filter = target_ast_to_object_filter(attacker_target).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported attacker target in must-block clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

        return Ok(Some(EffectAst::Cant {
            restriction: crate::effect::Restriction::must_block_specific_attacker(
                ObjectFilter::creature(),
                attacker_filter,
            ),
            duration,
            condition: None,
        }));
    }

    // "<subject> blocks <attacker> this turn if able."
    let subject_tokens = trim_commas(&tokens[..block_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let blockers_filter = parse_subject_object_filter(&subject_tokens)?.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported blocker subject in must-block clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    let mut tail_tokens = trim_commas(&tokens[block_idx + 1..]);
    if grammar::words_match_suffix(&tail_tokens, &["if", "able"]).is_none() {
        return Ok(None);
    }
    tail_tokens = trim_commas(&tail_tokens[..tail_tokens.len().saturating_sub(2)]);

    let (duration, attacker_tokens) =
        if let Some((duration, remainder)) = parse_restriction_duration(&tail_tokens)? {
            (duration, remainder)
        } else {
            (Until::EndOfTurn, tail_tokens.to_vec())
        };
    let attacker_tokens = trim_commas(&attacker_tokens);
    if attacker_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing attacker in must-block clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let attacker_target = parse_target_phrase(&attacker_tokens)?;
    let attacker_filter = target_ast_to_object_filter(attacker_target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker target in must-block clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::Cant {
        restriction: crate::effect::Restriction::must_block_specific_attacker(
            blockers_filter,
            attacker_filter,
        ),
        duration,
        condition: None,
    }))
}

pub(crate) fn parse_until_duration_triggered_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = token_word_refs(tokens);
    let has_leading_duration = starts_with_until_end_of_turn(&clause_words)
        || grammar::words_match_any_prefix(tokens, UNTIL_DURATION_TRIGGER_PREFIXES).is_some();
    if !has_leading_duration {
        return Ok(None);
    }

    let Some((duration, trigger_tokens)) = parse_restriction_duration(tokens)? else {
        return Ok(None);
    };
    if trigger_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing trigger after duration clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let trigger_words = token_word_refs(&trigger_tokens);
    let looks_like_trigger = trigger_words
        .first()
        .is_some_and(|word| *word == "when" || *word == "whenever")
        || grammar::words_match_any_prefix(&trigger_tokens, AT_THE_PREFIXES).is_some();
    if !looks_like_trigger {
        return Ok(None);
    }

    let (trigger, effects, max_triggers_per_turn) =
        match parse_triggered_line_lexed(&trigger_tokens)? {
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => (trigger, effects, max_triggers_per_turn),
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported duration-triggered clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        };

    let trigger_text = trigger_words.join(" ");
    let granted = GrantedAbilityAst::ParsedObjectAbility {
        ability: parsed_triggered_ability(
            trigger,
            effects,
            vec![Zone::Battlefield],
            Some(trigger_text.clone()),
            max_triggers_per_turn.map(crate::ConditionExpr::MaxTimesEachTurn),
            ReferenceImports::default(),
        ),
        display: trigger_text,
    };

    Ok(Some(EffectAst::GrantAbilitiesToTarget {
        target: TargetAst::Source(span_from_tokens(tokens)),
        abilities: vec![granted],
        duration,
    }))
}

pub(crate) fn parse_power_reference_word_count(words: &[&str]) -> Option<usize> {
    if word_slice_starts_with(words, &["its", "power"])
        || word_slice_starts_with(words, &["that", "power"])
    {
        return Some(2);
    }
    if word_slice_starts_with(words, &["this", "source", "power"])
        || word_slice_starts_with(words, &["this", "creature", "power"])
        || word_slice_starts_with(words, &["that", "creature", "power"])
        || word_slice_starts_with(words, &["that", "objects", "power"])
    {
        return Some(3);
    }
    None
}

pub(crate) fn is_damage_source_target(target: &TargetAst) -> bool {
    matches!(
        target,
        TargetAst::Source(_) | TargetAst::Object(_, _, _) | TargetAst::Tagged(_, _)
    )
}

pub(crate) fn parse_deal_damage_equal_to_power_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let clause_words = token_word_refs(tokens);
    let deal_split = grammar::split_lexed_once_on_separator(tokens, || grammar::kw("deal").void())
        .or_else(|| grammar::split_lexed_once_on_separator(tokens, || grammar::kw("deals").void()));
    let Some((source_slice, rest_slice)) = deal_split else {
        return Ok(None);
    };
    if source_slice.is_empty() {
        return Ok(None);
    }

    let source_tokens = trim_commas(source_slice);

    let rest = trim_commas(rest_slice);
    if rest.is_empty() || !rest[0].is_word("damage") {
        return Ok(None);
    }

    let Some((_before_equal, _after_equal_to)) =
        grammar::split_lexed_once_on_separator(&rest, || grammar::phrase(&["equal", "to"]).void())
    else {
        return Ok(None);
    };
    let equal_idx = _before_equal.len();

    let source = parse_target_phrase(&source_tokens)?;
    if !is_damage_source_target(&source) {
        return Err(CardTextError::ParseError(format!(
            "unsupported damage source target phrase (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let power_ref_words = token_word_refs(&rest[equal_idx + 2..]);
    let Some(power_ref_len) = parse_power_reference_word_count(&power_ref_words) else {
        return Ok(None);
    };

    let tail_after_power = trim_commas(&rest[equal_idx + 2 + power_ref_len..]);
    let pre_equal_words = token_word_refs(&rest[..equal_idx]);

    let target = if pre_equal_words == ["damage"] {
        let mut target_tokens = tail_after_power.as_slice();
        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("to"))
        {
            target_tokens = &target_tokens[1..];
        }
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing damage target after power reference (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let mut normalized_target_tokens = target_tokens;
        if grammar::words_match_any_prefix(target_tokens, EACH_OF_PREFIXES).is_some() {
            let each_of_tokens = &target_tokens[2..];
            if grammar::contains_word(each_of_tokens, "target") {
                normalized_target_tokens = each_of_tokens;
            }
        }
        let normalized_target_words = token_word_refs(normalized_target_tokens);
        if normalized_target_words.as_slice() == ["each", "player"]
            || normalized_target_words.as_slice() == ["each", "players"]
        {
            return Ok(Some(EffectAst::ForEachPlayer {
                effects: vec![EffectAst::DealDamageEqualToPower {
                    source: source.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                }],
            }));
        }
        if normalized_target_words.as_slice() == ["each", "opponent"]
            || normalized_target_words.as_slice() == ["each", "opponents"]
            || normalized_target_words.as_slice() == ["each", "other", "player"]
            || normalized_target_words.as_slice() == ["each", "other", "players"]
        {
            return Ok(Some(EffectAst::ForEachOpponent {
                effects: vec![EffectAst::DealDamageEqualToPower {
                    source: source.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                }],
            }));
        }
        parse_target_phrase(normalized_target_tokens)?
    } else if grammar::words_match_any_prefix(&rest[..equal_idx], DAMAGE_TO_PREFIXES).is_some() {
        let target_tokens = trim_commas(&rest[2..equal_idx]);
        let target_words = token_word_refs(&target_tokens);
        if target_words.as_slice() == ["each", "player"]
            || target_words.as_slice() == ["each", "players"]
        {
            return Ok(Some(EffectAst::ForEachPlayer {
                effects: vec![EffectAst::DealDamageEqualToPower {
                    source: source.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                }],
            }));
        }
        if target_words.as_slice() == ["each", "opponent"]
            || target_words.as_slice() == ["each", "opponents"]
            || target_words.as_slice() == ["each", "other", "player"]
            || target_words.as_slice() == ["each", "other", "players"]
        {
            return Ok(Some(EffectAst::ForEachOpponent {
                effects: vec![EffectAst::DealDamageEqualToPower {
                    source: source.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                }],
            }));
        }
        if target_words == ["itself"] || target_words == ["it"] {
            if !tail_after_power.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing target after self-damage power clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            source.clone()
        } else {
            if !tail_after_power.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing target after explicit power-damage target (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            parse_target_phrase(&target_tokens)?
        }
    } else {
        return Ok(None);
    };

    Ok(Some(EffectAst::DealDamageEqualToPower { source, target }))
}

pub(crate) fn parse_fight_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = token_word_refs(tokens);
    let fight_split =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("fight").void()
        })
        .or_else(|| {
            super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
                use winnow::Parser as _;
                super::super::grammar::primitives::kw("fights").void()
            })
        });
    let Some((left_of_fight, right_of_fight)) = fight_split else {
        return Ok(None);
    };
    let fight_idx = left_of_fight.len();

    if right_of_fight.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "fight clause requires two creatures (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let right_tokens = trim_commas(&tokens[fight_idx + 1..]);
    if right_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "fight clause requires two creatures (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let creature1 = if fight_idx == 0 {
        TargetAst::Source(None)
    } else {
        let left_tokens = trim_commas(&tokens[..fight_idx]);
        if left_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "fight clause requires two creatures (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if let Some(filter) = parse_for_each_object_subject(&left_tokens)? {
            let creature2 = parse_target_phrase(&right_tokens)?;
            if matches!(
                creature2,
                TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
            ) {
                return Err(CardTextError::ParseError(format!(
                    "fight target must be a creature (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(Some(EffectAst::ForEachObject {
                filter,
                effects: vec![EffectAst::FightIterated { creature2 }],
            }));
        }
        parse_target_phrase(&left_tokens)?
    };
    let right_words = token_word_refs(&right_tokens);
    let creature2 = if right_words == ["each", "other"] || right_words == ["one", "another"] {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&right_tokens))
    } else {
        parse_target_phrase(&right_tokens)?
    };

    for target in [&creature1, &creature2] {
        if matches!(
            target,
            TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
        ) {
            return Err(CardTextError::ParseError(format!(
                "fight target must be a creature (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    Ok(Some(EffectAst::Fight {
        creature1,
        creature2,
    }))
}

pub(crate) fn parse_clash_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = token_word_refs(tokens);
    let Some(first) = clause_words.first().copied() else {
        return Ok(None);
    };
    if first != "clash" && first != "clashes" {
        return Ok(None);
    }

    let mut tail = trim_commas(&tokens[1..]);
    if tail.first().is_some_and(|token| token.is_word("with")) {
        tail = trim_commas(&tail[1..]);
    }
    let tail_end = find_token_index(&tail, |token| token.is_word("then") || token.is_comma())
        .unwrap_or(tail.len());
    let tail = trim_commas(&tail[..tail_end]);
    if tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing opponent in clash clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let tail_words: Vec<&str> = token_word_refs(&tail)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let opponent = match tail_words.as_slice() {
        ["opponent"] => ClashOpponentAst::Opponent,
        ["target", "opponent"] => ClashOpponentAst::TargetOpponent,
        ["defending", "player"] => ClashOpponentAst::DefendingPlayer,
        _ => {
            return Err(CardTextError::ParseError(format!(
                "unsupported clash target (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };

    Ok(Some(EffectAst::Clash { opponent }))
}
