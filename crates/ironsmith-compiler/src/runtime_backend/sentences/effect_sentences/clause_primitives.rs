use super::super::clause_support::parse_triggered_line_lexed;
use super::super::grammar::effects::{
    split_change_target_clause_lexed, split_change_target_unless_clause_lexed,
    split_choose_new_targets_clause_lexed,
};
use super::super::grammar::primitives as grammar;
use super::super::lexer::{LexedClause, contains_token_word};
use super::super::lowering_support::rewrite_parsed_triggered_ability as parsed_triggered_ability;
use super::super::object_filters::parse_object_filter;
use super::super::permission_helpers::{
    parse_additional_land_plays_clause, parse_cast_or_play_tagged_clause,
    parse_cast_spells_as_though_they_had_flash_clause,
    parse_unsupported_play_cast_permission_clause, parse_until_end_of_turn_may_play_tagged_clause,
    parse_until_your_next_turn_may_play_tagged_clause,
};
use super::super::util::{is_article, parse_subject, parse_target_phrase, span_from_tokens};
use super::parse_restriction_duration;
use super::sentence_helpers::*;
#[allow(unused_imports)]
use crate::cards::builders::{
    COPIED_STACK_OBJECT_TAG, CardTextError, ClashOpponentAst, EffectAst, GrantedAbilityAst, IT_TAG,
    LineAst, OwnedLexToken, PlayerAst, PredicateAst, ReferenceImports, RetargetModeAst, SubjectAst,
    TagKey, TargetAst, TextSpan, TriggerSpec,
};
use crate::effect::ChoiceCount;
use crate::mana::ManaSymbol;
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

pub(crate) type ClausePrimitiveParser =
    fn(&[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError>;

pub(crate) struct ClausePrimitive {
    pub(crate) parser: ClausePrimitiveParser,
}

const CHOSEN_NAME_TAG: &str = "__chosen_name__";
const CHOOSE_CARD_NAME_PREFIXES: &[&[&str]] = &[
    &["choose"],
    &["you", "choose"],
    &["that", "player", "chooses"],
];
const CARD_NAME_SUFFIX: &[&str] = &["card", "name"];
const REPEAT_THIS_PROCESS_ANY_NUMBER_OF_TIMES_PATTERNS: &[&[&str]] = &[
    &["repeat", "this", "process", "any", "number", "of", "times"],
    &[
        "and", "repeat", "this", "process", "any", "number", "of", "times",
    ],
    &[
        "you", "may", "repeat", "this", "process", "any", "number", "of", "times",
    ],
    &[
        "and", "you", "may", "repeat", "this", "process", "any", "number", "of", "times",
    ],
];
const REPEAT_THIS_PROCESS_PATTERNS: &[&[&str]] = &[
    &["repeat", "this", "process"],
    &["and", "repeat", "this", "process"],
];
const REPEAT_THIS_PROCESS_ONCE_PATTERNS: &[&[&str]] = &[
    &["repeat", "this", "process", "once"],
    &["and", "repeat", "this", "process", "once"],
];
const DONT_LOSE_THIS_MANA_PATTERNS: &[&[&str]] = &[
    &[
        "you", "dont", "lose", "this", "mana", "as", "steps", "and", "phases", "end",
    ],
    &[
        "you", "don't", "lose", "this", "mana", "as", "steps", "and", "phases", "end",
    ],
];

#[derive(Clone, Copy)]
enum RetargetConstraintKind {
    SingleTarget,
    SingleCreatureTarget,
    SourceOnlyTarget,
    YouOnlyTarget,
    AnyPlayerTarget,
}

const RETARGET_CONSTRAINT_PHRASES: &[(&[&str], RetargetConstraintKind)] = &[
    (
        &["with", "a", "single", "target"],
        RetargetConstraintKind::SingleTarget,
    ),
    (
        &["targets", "only", "a", "single", "creature"],
        RetargetConstraintKind::SingleCreatureTarget,
    ),
    (
        &["targets", "only", "this", "creature"],
        RetargetConstraintKind::SourceOnlyTarget,
    ),
    (
        &["targets", "only", "this", "permanent"],
        RetargetConstraintKind::SourceOnlyTarget,
    ),
    (
        &["targets", "only", "you"],
        RetargetConstraintKind::YouOnlyTarget,
    ),
    (
        &["targets", "only", "a", "player"],
        RetargetConstraintKind::AnyPlayerTarget,
    ),
    (
        &["if", "that", "target", "is", "you"],
        RetargetConstraintKind::YouOnlyTarget,
    ),
];
const ALL_CREATURES_ABLE_TO_BLOCK_PREFIXES: &[&[&str]] =
    &[&["all", "creatures", "able", "to", "block"]];
const ATTACK_OR_BLOCK_IF_ABLE_SUFFIXES: &[&[&str]] = &[
    &["attack", "or", "block", "this", "turn", "if", "able"],
    &["attacks", "or", "blocks", "this", "turn", "if", "able"],
    &["attacks", "or", "block", "this", "turn", "if", "able"],
    &["attack", "or", "blocks", "this", "turn", "if", "able"],
];
const ATTACK_IF_ABLE_SUFFIXES: &[&[&str]] = &[
    &["attack", "this", "turn", "if", "able"],
    &["attacks", "this", "turn", "if", "able"],
];
const MUST_BE_BLOCKED_IF_ABLE_SUFFIXES: &[&[&str]] = &[
    &["must", "be", "blocked", "if", "able"],
    &["must", "be", "blocked", "this", "turn", "if", "able"],
    &[
        "must", "be", "blocked", "each", "combat", "this", "turn", "if", "able",
    ],
];
const BLOCK_THIS_TURN_IF_ABLE_SUFFIXES: &[&[&str]] = &[
    &["block", "this", "turn", "if", "able"],
    &["blocks", "this", "turn", "if", "able"],
];
const UNTIL_DURATION_TRIGGER_PREFIXES: &[&[&str]] = &[
    &["until", "your", "next", "turn"],
    &["until", "your", "next", "upkeep"],
    &["until", "your", "next", "untap", "step"],
    &["during", "your", "next", "untap", "step"],
];
const AT_THE_PREFIXES: &[&[&str]] = &[&["at", "the"]];
const EACH_OF_PREFIXES: &[&[&str]] = &[&["each", "of"]];
const DAMAGE_TO_PREFIXES: &[&[&str]] = &[&["damage", "to"]];
const COPY_TARGETS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["the", "copy", "targets"],
            &["that", "copy", "targets"],
            &["copy", "targets"],
        ]
);
const PAYS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["pays"]);
const ABILITY_OR_ABILITIES_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["ability", "abilities"]]);
const SPELL_OR_SPELLS_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["spell", "spells"]]);
const ANY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["any"]);
const POWER_REF_TWO_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["its", "power"], &["that", "power"]]);
const POWER_REF_THREE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "source", "power"],
            &["this", "creature", "power"],
            &["that", "creature", "power"],
            &["that", "objects", "power"],
        ]
);
const DAMAGE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["damage"]);
const TO_PREFIX: &[&str] = &["to"];
const WITH_PREFIX: &[&str] = &["with"];
const EQUAL_TO_PHRASE: &[&str] = &["equal", "to"];
const EACH_PLAYER_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["each", "player"], &["each", "players"]]);
const EACH_OPPONENT_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["each", "opponent"],
            &["each", "opponents"],
            &["each", "other", "player"],
            &["each", "other", "players"],
        ]
);
const ITSELF_OR_IT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["itself"], &["it"]]);
const FIGHT_TAGGED_OTHER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["each", "other"], &["one", "another"]]);
const THEN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["then"]);
const TRIGGER_INTRO_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["when"], &["whenever"]]);
const CLASH_OR_CLASHES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["clash"], &["clashes"]]);
const CLASH_OPPONENT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["opponent"]);
const CLASH_TARGET_OPPONENT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["target", "opponent"]);
const CLASH_DEFENDING_PLAYER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["defending", "player"]);

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

pub(crate) fn parse_copy_targets_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    let Some(targets_prefix_len) = COPY_TARGETS_PREFIX_PATTERN.matched_prefix_len(&words) else {
        return Ok(None);
    };
    let targets_idx = targets_prefix_len - 1;
    let fixed_clause = clause.from_word(targets_idx + 1).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing targets keyword in copy-target clause (clause: '{}')",
            clause.text()
        ))
    })?;
    let fixed_clause = fixed_clause.trimmed();
    if fixed_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target after copy-target clause (clause: '{}')",
            clause.text()
        )));
    }
    let fixed_filter = parse_object_filter(fixed_clause.tokens(), false)?;
    Ok(Some(EffectAst::subject_verb_retarget_stack_object(
        PlayerAst::Implicit,
        TargetAst::Tagged(TagKey::from(COPIED_STACK_OBJECT_TAG), clause.span()),
        RetargetModeAst::OneToFixed {
            target: TargetAst::Object(fixed_filter, None, None),
        },
        false,
    )))
}

pub(crate) fn parse_choose_new_targets_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(split) = split_choose_new_targets_clause_lexed(tokens) else {
        return Ok(None);
    };
    if split.reference_target {
        let reference_clause = LexedClause::new(split.target_tokens);
        let reference_words = reference_clause.word_refs();
        let reference_tag = if matches!(
            reference_words.as_slice(),
            ["the", "copy", ..]
                | ["the", "copies", ..]
                | ["that", "copy", ..]
                | ["those", "copies", ..]
        ) {
            COPIED_STACK_OBJECT_TAG
        } else {
            IT_TAG
        };
        let target = TargetAst::Tagged(
            TagKey::from(reference_tag),
            span_from_tokens(split.target_tokens),
        );
        return Ok(Some(EffectAst::subject_verb_retarget_stack_object(
            PlayerAst::Implicit,
            target,
            RetargetModeAst::All,
            false,
        )));
    }
    let tail_tokens = split.target_tokens;
    if tail_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing choose-new-targets target".to_string(),
        ));
    }

    let mut filter = parse_stack_retarget_filter(tail_tokens)?;
    if contains_token_word(tail_tokens, "other") {
        filter.other = true;
    }

    let mut target = TargetAst::Object(
        filter,
        if split.explicit_target {
            span_from_tokens(tail_tokens)
        } else {
            None
        },
        None,
    );
    if let Some(count) = split.count {
        target = TargetAst::WithCount(Box::new(target), count);
    }

    Ok(Some(EffectAst::subject_verb_retarget_stack_object(
        PlayerAst::Implicit,
        target,
        RetargetModeAst::All,
        false,
    )))
}

pub(crate) fn parse_change_target_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if clause.first_word() != Some("change") {
        return Ok(None);
    }

    if let Some((main_tokens, unless_tokens)) = split_change_target_unless_clause_lexed(tokens) {
        let Some(inner) = parse_change_target_clause_inner(&main_tokens)? else {
            return Ok(None);
        };
        let (player, cost) = parse_unless_pays_clause(&unless_tokens)?;
        return Ok(Some(EffectAst::UnlessPays {
            effects: vec![inner],
            player,
            cost,
        }));
    }

    parse_change_target_clause_inner(tokens)
}

pub(crate) fn parse_change_target_clause_inner(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(split) = split_change_target_clause_lexed(tokens) else {
        return Ok(None);
    };
    if split.target_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing target after change-the-target clause".to_string(),
        ));
    }

    let tail_tokens = split.target_tokens;
    let mut filter = parse_stack_retarget_filter(&tail_tokens)?;

    let tail_clause = LexedClause::new(&tail_tokens);
    for (_, constraint) in RETARGET_CONSTRAINT_PHRASES
        .iter()
        .filter(|(phrase, _)| tail_clause.contains_phrase(phrase))
    {
        filter = apply_retarget_constraint(filter, *constraint);
    }

    let target = TargetAst::Object(filter, span_from_tokens(tokens), None);

    let mode = if split.fixed_to_source {
        RetargetModeAst::OneToFixed {
            target: TargetAst::Source(span_from_tokens(tokens)),
        }
    } else {
        RetargetModeAst::All
    };

    Ok(Some(EffectAst::subject_verb_retarget_stack_object(
        PlayerAst::Implicit,
        target,
        mode,
        true,
    )))
}

fn apply_retarget_constraint(
    filter: ObjectFilter,
    constraint: RetargetConstraintKind,
) -> ObjectFilter {
    match constraint {
        RetargetConstraintKind::SingleTarget => filter.target_count_exact(1),
        RetargetConstraintKind::SingleCreatureTarget => filter
            .targeting_only_object(ObjectFilter::creature())
            .target_count_exact(1),
        RetargetConstraintKind::SourceOnlyTarget => filter
            .targeting_only_object(ObjectFilter::source())
            .target_count_exact(1),
        RetargetConstraintKind::YouOnlyTarget => filter
            .targeting_only_player(PlayerFilter::You)
            .target_count_exact(1),
        RetargetConstraintKind::AnyPlayerTarget => filter
            .targeting_only_player(PlayerFilter::Any)
            .target_count_exact(1),
    }
}

pub(crate) fn parse_unless_pays_clause(
    tokens: &[OwnedLexToken],
) -> Result<(PlayerAst, crate::cost::TotalCost), CardTextError> {
    let clause = LexedClause::new(tokens);
    if clause.is_empty() {
        return Err(CardTextError::ParseError(
            "missing unless clause".to_string(),
        ));
    }
    let (player_clause, pays_clause) = clause.split_once_before_word("pays").ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing pays keyword (clause: '{}')",
            clause.text()
        ))
    })?;

    let player_clause = player_clause.trimmed();
    let player = match parse_subject(player_clause.tokens()) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };

    let mut payment_tokens = pays_clause.tokens().to_vec();
    if let Some(first) = payment_tokens.first_mut()
        && PAYS_WORD_PATTERN.matches_token(first)
    {
        first.replace_word("pay");
    }

    let cost = crate::runtime_backend::families::activation_and_restrictions::parse_payment_clause_as_total_cost(&payment_tokens)?
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported unless-payment clause (clause: '{}')",
                clause.text()
            ))
        })?;

    Ok((player, cost))
}

pub(crate) fn parse_stack_retarget_filter(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    let has_ability = ABILITY_OR_ABILITIES_MARKER_PATTERN.matches_words(&words);
    let has_spell = SPELL_OR_SPELLS_MARKER_PATTERN.matches_words(&words);
    let has_activated = clause.contains_word("activated");
    let has_instant = clause.contains_word("instant");
    let has_sorcery = clause.contains_word("sorcery");

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
            clause.text()
        )));
    };

    if clause.contains_word("other") {
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
            parser: parse_copy_targets_clause,
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
    let clause = LexedClause::new(tokens);
    if clause.word_len() < 3 {
        return Ok(None);
    }

    let (player, tail_clause) = if let Some((prefix, tail_clause)) =
        clause.strip_any_prefix_clause(CHOOSE_CARD_NAME_PREFIXES)
    {
        let player = if prefix == &["that", "player", "chooses"] {
            PlayerAst::That
        } else {
            PlayerAst::You
        };
        (player, tail_clause.trimmed())
    } else {
        return Ok(None);
    };

    let Some(filter_clause) = tail_clause.strip_suffix_clause(CARD_NAME_SUFFIX) else {
        return Ok(None);
    };
    let filter_clause = filter_clause.trimmed();

    let filter_words =
        crate::runtime_backend::util::non_article_token_word_refs(filter_clause.tokens());
    let filter = if filter_words.is_empty() || ANY_WORD_PATTERN.matches_words(&filter_words) {
        None
    } else {
        let normalized_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&filter_words);
        Some(parse_object_filter(&normalized_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported choose-card-name filter (clause: '{}')",
                clause.text()
            ))
        })?)
    };

    Ok(Some(EffectAst::subject_verb_choose_card_name(
        player,
        filter,
        TagKey::from(CHOSEN_NAME_TAG),
    )))
}

pub(crate) fn parse_repeat_this_process_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if clause.matches_any_words(REPEAT_THIS_PROCESS_ANY_NUMBER_OF_TIMES_PATTERNS) {
        return Ok(Some(EffectAst::RepeatThisProcessMay));
    }
    if clause.matches_any_words(REPEAT_THIS_PROCESS_PATTERNS) {
        return Ok(Some(EffectAst::RepeatThisProcess));
    }
    if clause.matches_any_words(REPEAT_THIS_PROCESS_ONCE_PATTERNS) {
        return Ok(Some(EffectAst::RepeatThisProcessOnce));
    }
    Ok(None)
}

pub(crate) fn parse_dont_lose_this_mana_as_steps_and_phases_end_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if LexedClause::new(tokens).matches_any_words(DONT_LOSE_THIS_MANA_PATTERNS) {
        return Ok(Some(
            EffectAst::subject_verb_dont_lose_this_mana_as_steps_and_phases_end_this_turn(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_attack_or_block_this_turn_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause = LexedClause::new(tokens);
    let Some((_matched, subject_clause)) =
        clause.strip_any_suffix_clause(ATTACK_OR_BLOCK_IF_ABLE_SUFFIXES)
    else {
        return Ok(None);
    };

    let subject_clause = subject_clause.trimmed();
    let target = if subject_clause.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span())
    } else {
        parse_target_phrase(subject_clause.tokens())?
    };
    let abilities = vec![GrantedAbilityAst::MustAttack, GrantedAbilityAst::MustBlock];

    if subject_clause.is_empty() || starts_with_target_indicator(subject_clause.tokens()) {
        return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
            target,
            abilities,
            Until::EndOfTurn,
        )));
    }

    let filter = target_ast_to_object_filter(target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker/blocker subject in attacks-or-blocks-if-able clause (clause: '{}')",
            clause.text()
        ))
    })?;

    Ok(Some(EffectAst::subject_verb_grant_abilities_all(
        filter,
        abilities,
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_attack_this_turn_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause = LexedClause::new(tokens);
    let Some((_matched, subject_clause)) = clause.strip_any_suffix_clause(ATTACK_IF_ABLE_SUFFIXES)
    else {
        return Ok(None);
    };

    let subject_clause = subject_clause.trimmed();
    let target = if subject_clause.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span())
    } else {
        parse_target_phrase(subject_clause.tokens())?
    };
    let ability = GrantedAbilityAst::MustAttack;

    if subject_clause.is_empty() || starts_with_target_indicator(subject_clause.tokens()) {
        return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
            target,
            vec![ability],
            Until::EndOfTurn,
        )));
    }

    let filter = target_ast_to_object_filter(target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker subject in attacks-if-able clause (clause: '{}')",
            clause.text()
        ))
    })?;

    Ok(Some(EffectAst::subject_verb_grant_abilities_all(
        filter,
        vec![ability],
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_must_be_blocked_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause = LexedClause::new(tokens);
    let Some((_matched, subject_clause)) =
        clause.strip_any_suffix_clause(MUST_BE_BLOCKED_IF_ABLE_SUFFIXES)
    else {
        return Ok(None);
    };

    let subject_clause = subject_clause.trimmed();
    if subject_clause.is_empty() {
        return Ok(None);
    }
    if starts_with_target_indicator(subject_clause.tokens()) {
        let attacker_target = parse_target_phrase(subject_clause.tokens())?;
        return Ok(Some(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_target_only(attacker_target),
                EffectAst::subject_verb_cant(
                    crate::effect::Restriction::must_be_blocked(ObjectFilter::tagged(IT_TAG)),
                    Until::EndOfTurn,
                    None,
                ),
            ],
        }));
    }

    let attacker_target = parse_target_phrase(subject_clause.tokens())?;
    let attacker_filter = target_ast_to_object_filter(attacker_target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker subject in must-be-blocked clause (clause: '{}')",
            clause.text()
        ))
    })?;

    Ok(Some(EffectAst::subject_verb_cant(
        crate::effect::Restriction::must_be_blocked(attacker_filter),
        Until::EndOfTurn,
        None,
    )))
}

pub(crate) fn parse_must_block_if_able_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::effect::Until;

    let clause = LexedClause::new(tokens);
    let clause_text = clause.text();

    // "<subject> blocks this turn if able."
    let Some(block_idx) = clause.find_token_word_any(&["block", "blocks"]) else {
        return Ok(None);
    };
    if block_idx == 0 || block_idx + 1 >= tokens.len() {
        return Ok(None);
    }
    if clause
        .from(block_idx)
        .starts_with_any(BLOCK_THIS_TURN_IF_ABLE_SUFFIXES)
    {
        let subject_clause = clause.before(block_idx).trimmed();
        if subject_clause.is_empty() {
            return Ok(None);
        }
        let target = parse_target_phrase(subject_clause.tokens())?;
        let ability = GrantedAbilityAst::MustBlock;

        if starts_with_target_indicator(subject_clause.tokens()) {
            return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
                target,
                vec![ability],
                Until::EndOfTurn,
            )));
        }

        let filter = target_ast_to_object_filter(target).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported blocker subject in blocks-if-able clause (clause: '{}')",
                clause_text
            ))
        })?;
        return Ok(Some(EffectAst::subject_verb_grant_abilities_all(
            filter,
            vec![ability],
            Until::EndOfTurn,
        )));
    }

    // "All creatures able to block target creature this turn do so."
    if let Some((_, tail_clause)) =
        clause.strip_any_prefix_clause(ALL_CREATURES_ABLE_TO_BLOCK_PREFIXES)
    {
        let Some(tail_clause) = tail_clause.trimmed().strip_suffix_clause(&["do", "so"]) else {
            return Ok(None);
        };
        let tail_clause = tail_clause.trimmed();

        let (duration, attacker_tokens) = if let Some((duration, remainder)) =
            parse_restriction_duration(tail_clause.tokens())?
        {
            (duration, remainder)
        } else {
            (Until::EndOfTurn, tail_clause.tokens().to_vec())
        };
        let attacker_clause = LexedClause::new(&attacker_tokens).trimmed();
        if attacker_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing attacker in must-block clause (clause: '{}')",
                clause_text
            )));
        }

        let attacker_target = parse_target_phrase(attacker_clause.tokens())?;
        let attacker_filter = target_ast_to_object_filter(attacker_target).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported attacker target in must-block clause (clause: '{}')",
                clause_text
            ))
        })?;

        return Ok(Some(EffectAst::subject_verb_cant(
            crate::effect::Restriction::must_block_specific_attacker(
                ObjectFilter::creature(),
                attacker_filter,
            ),
            duration,
            None,
        )));
    }

    // "<subject> blocks <attacker> this turn if able."
    let subject_clause = clause.before(block_idx).trimmed();
    if subject_clause.is_empty() {
        return Ok(None);
    }
    let blockers_filter =
        parse_subject_object_filter(subject_clause.tokens())?.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported blocker subject in must-block clause (clause: '{}')",
                clause_text
            ))
        })?;

    let Some(tail_clause) = clause
        .from(block_idx + 1)
        .trimmed()
        .strip_suffix_clause(&["if", "able"])
    else {
        return Ok(None);
    };
    let tail_clause = tail_clause.trimmed();

    let (duration, attacker_tokens) =
        if let Some((duration, remainder)) = parse_restriction_duration(tail_clause.tokens())? {
            (duration, remainder)
        } else {
            (Until::EndOfTurn, tail_clause.tokens().to_vec())
        };
    let attacker_clause = LexedClause::new(&attacker_tokens).trimmed();
    if attacker_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing attacker in must-block clause (clause: '{}')",
            clause_text
        )));
    }

    let attacker_target = parse_target_phrase(attacker_clause.tokens())?;
    let attacker_filter = target_ast_to_object_filter(attacker_target).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attacker target in must-block clause (clause: '{}')",
            clause_text
        ))
    })?;

    Ok(Some(EffectAst::subject_verb_cant(
        crate::effect::Restriction::must_block_specific_attacker(blockers_filter, attacker_filter),
        duration,
        None,
    )))
}

pub(crate) fn parse_until_duration_triggered_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_words = clause.word_refs();
    let has_leading_duration = starts_with_until_end_of_turn(&clause_words)
        || clause
            .strip_any_prefix_clause(UNTIL_DURATION_TRIGGER_PREFIXES)
            .is_some();
    if !has_leading_duration {
        return Ok(None);
    }

    let Some((duration, trigger_tokens)) = parse_restriction_duration(tokens)? else {
        return Ok(None);
    };
    if trigger_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing trigger after duration clause (clause: '{}')",
            clause.text()
        )));
    }

    let trigger_clause = LexedClause::new(&trigger_tokens);
    let trigger_words = trigger_clause.word_refs();
    let looks_like_trigger = trigger_clause
        .first_word()
        .is_some_and(|word| TRIGGER_INTRO_WORD_PATTERN.matches_word(word))
        || trigger_clause
            .strip_any_prefix_clause(AT_THE_PREFIXES)
            .is_some();
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
                    clause.text()
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
            crate::runtime_backend::trigger_frequency_condition(
                Some(trigger_text.as_str()),
                max_triggers_per_turn,
            ),
            None,
            ReferenceImports::default(),
        ),
        display: trigger_text,
    };

    Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
        TargetAst::Source(span_from_tokens(tokens)),
        vec![granted],
        duration,
    )))
}

pub(crate) fn parse_power_reference_word_count(words: &[&str]) -> Option<usize> {
    if POWER_REF_TWO_WORD_PATTERN.matches_words(words) {
        return Some(2);
    }
    if POWER_REF_THREE_WORD_PATTERN.matches_words(words) {
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
    let clause = LexedClause::new(tokens);
    let clause_text = clause.text();
    let Some((source_clause, rest_clause)) = clause.split_once_on_word_any(&["deal", "deals"])
    else {
        return Ok(None);
    };
    if source_clause.is_empty() {
        return Ok(None);
    }

    let source_clause = source_clause.trimmed();

    let rest_clause = rest_clause.trimmed();
    if rest_clause.is_empty() || !DAMAGE_WORD_PATTERN.matches_first_word(&rest_clause.word_refs()) {
        return Ok(None);
    }

    let Some((pre_equal_clause, after_equal_clause)) =
        rest_clause.split_once_on_phrase(EQUAL_TO_PHRASE)
    else {
        return Ok(None);
    };

    let power_ref_clause = after_equal_clause.trimmed();
    let power_ref_words = power_ref_clause.word_refs();
    let Some(power_ref_len) = parse_power_reference_word_count(&power_ref_words) else {
        return Ok(None);
    };

    let source_words = source_clause.word_refs();
    let source = if matches!(
        source_words.as_slice(),
        ["it"] | ["that", "creature"] | ["that", "permanent"] | ["that", "card"]
    ) {
        TargetAst::Tagged(TagKey::from(IT_TAG), source_clause.span())
    } else {
        parse_target_phrase(source_clause.tokens())?
    };
    if !is_damage_source_target(&source) {
        return Err(CardTextError::ParseError(format!(
            "unsupported damage source target phrase (clause: '{}')",
            clause_text
        )));
    }

    let tail_after_power_clause = power_ref_clause
        .after_words(power_ref_len)
        .unwrap_or_else(|| power_ref_clause.from(power_ref_clause.tokens().len()))
        .trimmed();
    let pre_equal_words = pre_equal_clause.word_refs();

    let target = if DAMAGE_WORD_PATTERN.matches_words(&pre_equal_words) {
        let target_clause = tail_after_power_clause
            .strip_prefix_clause(TO_PREFIX)
            .unwrap_or(tail_after_power_clause)
            .trimmed();
        if target_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing damage target after power reference (clause: '{}')",
                clause_text
            )));
        }
        let mut normalized_target_clause = target_clause;
        if let Some((_, each_of_clause)) = target_clause.strip_any_prefix_clause(EACH_OF_PREFIXES) {
            if each_of_clause.contains_word("target") {
                normalized_target_clause = each_of_clause;
            }
        }
        let normalized_target_words = normalized_target_clause.word_refs();
        if EACH_PLAYER_TARGET_PATTERN.matches_words(&normalized_target_words) {
            return Ok(Some(EffectAst::ForEachPlayer {
                effects: vec![EffectAst::subject_verb_damage_equal_to_power(
                    source.clone(),
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
            }));
        }
        if EACH_OPPONENT_TARGET_PATTERN.matches_words(&normalized_target_words) {
            return Ok(Some(EffectAst::ForEachOpponent {
                effects: vec![EffectAst::subject_verb_damage_equal_to_power(
                    source.clone(),
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
            }));
        }
        parse_target_phrase(normalized_target_clause.tokens())?
    } else if pre_equal_clause
        .strip_any_prefix_clause(DAMAGE_TO_PREFIXES)
        .is_some()
    {
        let target_clause = pre_equal_clause
            .strip_any_prefix_clause(DAMAGE_TO_PREFIXES)
            .map(|(_, target_clause)| target_clause)
            .unwrap_or(pre_equal_clause)
            .trimmed();
        let target_words = target_clause.word_refs();
        if EACH_PLAYER_TARGET_PATTERN.matches_words(&target_words) {
            return Ok(Some(EffectAst::ForEachPlayer {
                effects: vec![EffectAst::subject_verb_damage_equal_to_power(
                    source.clone(),
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
            }));
        }
        if EACH_OPPONENT_TARGET_PATTERN.matches_words(&target_words) {
            return Ok(Some(EffectAst::ForEachOpponent {
                effects: vec![EffectAst::subject_verb_damage_equal_to_power(
                    source.clone(),
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
            }));
        }
        if ITSELF_OR_IT_PATTERN.matches_words(&target_words) {
            if !tail_after_power_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing target after self-damage power clause (clause: '{}')",
                    clause_text
                )));
            }
            source.clone()
        } else {
            if !tail_after_power_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing target after explicit power-damage target (clause: '{}')",
                    clause_text
                )));
            }
            parse_target_phrase(target_clause.tokens())?
        }
    } else {
        return Ok(None);
    };

    Ok(Some(EffectAst::subject_verb_damage_equal_to_power(
        source, target,
    )))
}

pub(crate) fn parse_fight_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_text = clause.text();
    let Some((left_clause, right_clause)) = clause.split_once_on_word_any(&["fight", "fights"])
    else {
        return Ok(None);
    };

    let right_clause = right_clause.trimmed();
    if right_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "fight clause requires two creatures (clause: '{}')",
            clause_text
        )));
    }

    let creature1 = if left_clause.is_empty() {
        TargetAst::Source(None)
    } else {
        let left_clause = left_clause.trimmed();
        if left_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "fight clause requires two creatures (clause: '{}')",
                clause_text
            )));
        }
        if let Some(filter) = parse_for_each_object_subject(left_clause.tokens())? {
            let creature2 = parse_target_phrase(right_clause.tokens())?;
            if matches!(
                creature2,
                TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
            ) {
                return Err(CardTextError::ParseError(format!(
                    "fight target must be a creature (clause: '{}')",
                    clause_text
                )));
            }
            return Ok(Some(EffectAst::ForEachObject {
                filter,
                effects: vec![EffectAst::subject_verb_fight_iterated(creature2)],
            }));
        }
        parse_target_phrase(left_clause.tokens())?
    };
    let right_words = right_clause.word_refs();
    let creature2 = if FIGHT_TAGGED_OTHER_PATTERN.matches_words(&right_words) {
        TargetAst::Tagged(TagKey::from(IT_TAG), right_clause.span())
    } else {
        parse_target_phrase(right_clause.tokens())?
    };

    for target in [&creature1, &creature2] {
        if matches!(
            target,
            TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
        ) {
            return Err(CardTextError::ParseError(format!(
                "fight target must be a creature (clause: '{}')",
                clause_text
            )));
        }
    }

    Ok(Some(EffectAst::subject_verb_fight(creature1, creature2)))
}

pub(crate) fn parse_clash_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if !clause
        .first_word()
        .is_some_and(|word| CLASH_OR_CLASHES_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let tail_clause = clause
        .from(1)
        .trimmed()
        .strip_prefix_clause(WITH_PREFIX)
        .unwrap_or_else(|| clause.from(1).trimmed())
        .trimmed()
        .take_until_token_matching(|token| {
            THEN_WORD_PATTERN.matches_token(token) || token.is_comma()
        })
        .trimmed();
    if tail_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing opponent in clash clause (clause: '{}')",
            clause.text()
        )));
    }

    let tail_words =
        crate::runtime_backend::util::non_article_token_word_refs(tail_clause.tokens());
    let opponent = if CLASH_OPPONENT_PATTERN.matches_words(&tail_words) {
        ClashOpponentAst::Opponent
    } else if CLASH_TARGET_OPPONENT_PATTERN.matches_words(&tail_words) {
        ClashOpponentAst::TargetOpponent
    } else if CLASH_DEFENDING_PLAYER_PATTERN.matches_words(&tail_words) {
        ClashOpponentAst::DefendingPlayer
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported clash target (clause: '{}')",
            clause.text()
        )));
    };

    Ok(Some(EffectAst::subject_verb_clash(opponent)))
}
