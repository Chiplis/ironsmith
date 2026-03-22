use super::super::activation_and_restrictions::parse_triggered_line;
use super::super::lowering_support::rewrite_parsed_triggered_ability as parsed_triggered_ability;
use super::super::object_filters::parse_object_filter;
use super::super::permission_helpers::{
    parse_additional_land_plays_clause, parse_cast_or_play_tagged_clause,
    parse_cast_spells_as_though_they_had_flash_clause,
    parse_unsupported_play_cast_permission_clause, parse_until_end_of_turn_may_play_tagged_clause,
    parse_until_your_next_turn_may_play_tagged_clause,
};
use super::super::util::{
    is_article, parse_subject, parse_target_phrase, span_from_tokens, trim_commas, words,
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
    let clause_words = words(tokens);
    let is_choose = clause_words.starts_with(&["choose", "new", "targets", "for"])
        || clause_words.starts_with(&["chooses", "new", "targets", "for"]);
    let is_choose_single_target = clause_words
        .starts_with(&["choose", "a", "new", "target", "for"])
        || clause_words.starts_with(&["chooses", "a", "new", "target", "for"]);
    if !is_choose && !is_choose_single_target {
        return Ok(None);
    }

    let mut tail_tokens = if is_choose_single_target {
        &tokens[5..]
    } else {
        &tokens[4..]
    };
    if tail_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing choose-new-targets target".to_string(),
        ));
    }

    if let Some(if_idx) = tail_tokens.iter().position(|token| token.is_word("if")) {
        tail_tokens = &tail_tokens[..if_idx];
    }

    let tail_words = words(tail_tokens);
    if tail_words.starts_with(&["it"])
        || tail_words.starts_with(&["them"])
        || tail_words.starts_with(&["the", "copy"])
        || tail_words.starts_with(&["that", "copy"])
        || tail_words.starts_with(&["the", "spell"])
        || tail_words.starts_with(&["that", "spell"])
    {
        let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tail_tokens));
        return Ok(Some(EffectAst::RetargetStackObject {
            target,
            mode: RetargetModeAst::All,
            chooser: PlayerAst::Implicit,
            require_change: false,
        }));
    }

    let (count, base_tokens, explicit_target) = if tail_words.starts_with(&["any", "number", "of"])
    {
        (Some(ChoiceCount::any_number()), &tail_tokens[3..], false)
    } else if tail_words.starts_with(&["target"]) {
        (None, &tail_tokens[1..], true)
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
    let clause_words = words(tokens);
    if clause_words.is_empty() || clause_words[0] != "change" {
        return Ok(None);
    }

    if let Some(unless_idx) = tokens.iter().position(|token| token.is_word("unless")) {
        let main_tokens = trim_commas(&tokens[..unless_idx]);
        let unless_tokens = trim_commas(&tokens[unless_idx + 1..]);
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
    let clause_words = words(tokens);
    let (mode, after_of_idx) = if clause_words.starts_with(&["change", "the", "target", "of"]) {
        (RetargetModeAst::All, 4)
    } else if clause_words.starts_with(&["change", "the", "targets", "of"]) {
        (RetargetModeAst::All, 4)
    } else if clause_words.starts_with(&["change", "a", "target", "of"]) {
        (RetargetModeAst::All, 4)
    } else {
        return Ok(None);
    };

    if tokens.len() <= after_of_idx {
        return Err(CardTextError::ParseError(
            "missing target after change-the-target clause".to_string(),
        ));
    }

    let mut tail_tokens = trim_commas(&tokens[after_of_idx..]).to_vec();
    let mut fixed_target: Option<TargetAst> = None;
    if let Some(to_idx) = tail_tokens.iter().position(|token| token.is_word("to")) {
        let to_tail = &tail_tokens[to_idx + 1..];
        let to_words = words(to_tail);
        if to_words.starts_with(&["this"]) {
            fixed_target = Some(TargetAst::Source(span_from_tokens(to_tail)));
            tail_tokens.truncate(to_idx);
        }
    }

    let mut filter = parse_stack_retarget_filter(&tail_tokens)?;
    let tail_words = words(&tail_tokens);

    if tail_words
        .windows(4)
        .any(|w| w == ["with", "a", "single", "target"])
    {
        filter = filter.target_count_exact(1);
    }
    if tail_words
        .windows(5)
        .any(|w| w == ["targets", "only", "a", "single", "creature"])
    {
        filter = filter
            .targeting_only_object(ObjectFilter::creature())
            .target_count_exact(1);
    }
    if tail_words
        .windows(4)
        .any(|w| w == ["targets", "only", "this", "creature"])
        || tail_words
            .windows(4)
            .any(|w| w == ["targets", "only", "this", "permanent"])
    {
        filter = filter
            .targeting_only_object(ObjectFilter::source())
            .target_count_exact(1);
    }
    if tail_words
        .windows(3)
        .any(|w| w == ["targets", "only", "you"])
    {
        filter = filter
            .targeting_only_player(PlayerFilter::You)
            .target_count_exact(1);
    }
    if tail_words
        .windows(4)
        .any(|w| w == ["targets", "only", "a", "player"])
    {
        filter = filter
            .targeting_only_player(PlayerFilter::Any)
            .target_count_exact(1);
    }
    if tail_words
        .windows(5)
        .any(|w| w == ["if", "that", "target", "is", "you"])
    {
        filter = filter
            .targeting_only_player(PlayerFilter::You)
            .target_count_exact(1);
    }

    let target = TargetAst::Object(filter, span_from_tokens(tokens), None);

    let mode = if let Some(fixed) = fixed_target {
        RetargetModeAst::OneToFixed { target: fixed }
    } else {
        mode
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
    let pays_idx = tokens
        .iter()
        .position(|token| token.is_word("pays"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing pays keyword (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;

    let player_tokens = trim_commas(&tokens[..pays_idx]);
    let player = match parse_subject(&player_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };

    let mut mana = Vec::new();
    let mut trailing_start: Option<usize> = None;
    for (offset, token) in tokens[pays_idx + 1..].iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        match parse_mana_symbol(word) {
            Ok(symbol) => mana.push(symbol),
            Err(_) => {
                trailing_start = Some(pays_idx + 1 + offset);
                break;
            }
        }
    }

    if mana.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing mana cost (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    if let Some(start) = trailing_start {
        let trailing_tokens = trim_commas(&tokens[start..]);
        let trailing_words = words(&trailing_tokens);
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing unless-payment clause (clause: '{}', trailing: '{}')",
                words(tokens).join(" "),
                trailing_words.join(" ")
            )));
        }
    }

    Ok((player, mana))
}

pub(crate) fn parse_stack_retarget_filter(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let words = words(tokens);
    let has_ability = words
        .iter()
        .any(|word| *word == "ability" || *word == "abilities");
    let has_spell = words
        .iter()
        .any(|word| *word == "spell" || *word == "spells");
    let has_activated = words.iter().any(|word| *word == "activated");
    let has_instant = words.iter().any(|word| *word == "instant");
    let has_sorcery = words.iter().any(|word| *word == "sorcery");

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

    if words.iter().any(|word| *word == "other") {
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
    let clause_words = words(tokens);
    if clause_words.len() < 3 {
        return Ok(None);
    }

    let (player, prefix_len) = if clause_words.first().is_some_and(|word| *word == "choose") {
        (PlayerAst::You, 1usize)
    } else if clause_words.starts_with(&["you", "choose"]) {
        (PlayerAst::You, 2usize)
    } else if clause_words.starts_with(&["that", "player", "chooses"]) {
        (PlayerAst::That, 3usize)
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
        tag: TagKey::from(IT_TAG),
    }))
}

pub(crate) fn parse_repeat_this_process_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
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

pub(crate) fn parse_attack_or_block_this_turn_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause_words = words(tokens);
    let Some(attack_idx) = tokens
        .iter()
        .position(|token| token.is_word("attack") || token.is_word("attacks"))
    else {
        return Ok(None);
    };
    let tail_words = words(&tokens[attack_idx..]);
    let has_supported_tail = tail_words == ["attack", "or", "block", "this", "turn", "if", "able"]
        || tail_words == ["attacks", "or", "blocks", "this", "turn", "if", "able"]
        || tail_words == ["attacks", "or", "block", "this", "turn", "if", "able"]
        || tail_words == ["attack", "or", "blocks", "this", "turn", "if", "able"];
    if !has_supported_tail {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..attack_idx]);
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
    use crate::effect::Until;

    let clause_words = words(tokens);
    let Some(attack_idx) = tokens
        .iter()
        .position(|token| token.is_word("attack") || token.is_word("attacks"))
    else {
        return Ok(None);
    };
    let tail_words = words(&tokens[attack_idx..]);
    if tail_words != ["attack", "this", "turn", "if", "able"]
        && tail_words != ["attacks", "this", "turn", "if", "able"]
    {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..attack_idx]);
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

    let clause_words = words(tokens);
    let Some(must_idx) = tokens.iter().position(|token| token.is_word("must")) else {
        return Ok(None);
    };
    if must_idx == 0 {
        return Ok(None);
    }

    let tail_words = words(&tokens[must_idx..]);
    let has_supported_tail = tail_words == ["must", "be", "blocked", "if", "able"]
        || tail_words == ["must", "be", "blocked", "this", "turn", "if", "able"];
    if !has_supported_tail {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..must_idx]);
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

    let clause_words = words(tokens);

    // "<subject> blocks this turn if able."
    let Some(block_idx) = tokens
        .iter()
        .position(|token| token.is_word("block") || token.is_word("blocks"))
    else {
        return Ok(None);
    };
    if block_idx == 0 || block_idx + 1 >= tokens.len() {
        return Ok(None);
    }
    let tail_words = words(&tokens[block_idx..]);
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
    if clause_words.starts_with(&["all", "creatures", "able", "to", "block"]) {
        let mut tail_tokens = trim_commas(&tokens[5..]);
        let tail_words = words(&tail_tokens);
        if !tail_words.ends_with(&["do", "so"]) {
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
    let tail_words = words(&tail_tokens);
    if !tail_words.ends_with(&["if", "able"]) {
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
    let clause_words = words(tokens);
    let has_leading_duration = starts_with_until_end_of_turn(&clause_words)
        || clause_words.starts_with(&["until", "your", "next", "turn"])
        || clause_words.starts_with(&["until", "your", "next", "upkeep"])
        || clause_words.starts_with(&["until", "your", "next", "untap", "step"])
        || clause_words.starts_with(&["during", "your", "next", "untap", "step"]);
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

    let trigger_words = words(&trigger_tokens);
    let looks_like_trigger = trigger_words
        .first()
        .is_some_and(|word| *word == "when" || *word == "whenever")
        || trigger_words.starts_with(&["at", "the"]);
    if !looks_like_trigger {
        return Ok(None);
    }

    let (trigger, effects, max_triggers_per_turn) = match parse_triggered_line(&trigger_tokens)? {
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
    if words.starts_with(&["its", "power"]) || words.starts_with(&["that", "power"]) {
        return Some(2);
    }
    if words.starts_with(&["this", "source", "power"])
        || words.starts_with(&["this", "creature", "power"])
        || words.starts_with(&["that", "creature", "power"])
        || words.starts_with(&["that", "objects", "power"])
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
    let clause_words = words(tokens);
    let Some(deal_idx) = tokens
        .iter()
        .position(|token| token.is_word("deal") || token.is_word("deals"))
    else {
        return Ok(None);
    };
    if deal_idx == 0 {
        return Ok(None);
    }

    let source_tokens = trim_commas(&tokens[..deal_idx]);

    let rest = trim_commas(&tokens[deal_idx + 1..]);
    if rest.is_empty() || !rest[0].is_word("damage") {
        return Ok(None);
    }

    let Some(equal_idx) = rest
        .windows(2)
        .position(|window| window[0].is_word("equal") && window[1].is_word("to"))
    else {
        return Ok(None);
    };

    let source = parse_target_phrase(&source_tokens)?;
    if !is_damage_source_target(&source) {
        return Err(CardTextError::ParseError(format!(
            "unsupported damage source target phrase (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let power_ref_words = words(&rest[equal_idx + 2..]);
    let Some(power_ref_len) = parse_power_reference_word_count(&power_ref_words) else {
        return Ok(None);
    };

    let tail_after_power = trim_commas(&rest[equal_idx + 2 + power_ref_len..]);
    let pre_equal_words = words(&rest[..equal_idx]);

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
        let target_words = words(target_tokens);
        if target_words.starts_with(&["each", "of"]) {
            let each_of_tokens = &target_tokens[2..];
            let each_of_words = words(each_of_tokens);
            if each_of_words.iter().any(|word| *word == "target") {
                normalized_target_tokens = each_of_tokens;
            }
        }
        let normalized_target_words = words(normalized_target_tokens);
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
    } else if pre_equal_words.starts_with(&["damage", "to"]) {
        let target_tokens = trim_commas(&rest[2..equal_idx]);
        let target_words = words(&target_tokens);
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
    let clause_words = words(tokens);
    let Some(fight_idx) = tokens
        .iter()
        .position(|token| token.is_word("fight") || token.is_word("fights"))
    else {
        return Ok(None);
    };

    if fight_idx + 1 >= tokens.len() {
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
    let right_words = words(&right_tokens);
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
    let clause_words = words(tokens);
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
    let tail_end = tail
        .iter()
        .position(|token| token.is_word("then") || token.is_comma())
        .unwrap_or(tail.len());
    let tail = trim_commas(&tail[..tail_end]);
    if tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing opponent in clash clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let tail_words: Vec<&str> = words(&tail)
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
