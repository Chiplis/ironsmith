use super::super::activation_and_restrictions::parse_single_word_keyword_action;
use super::super::clause_support::{
    parse_static_ability_ast_line_lexed, parse_trigger_clause_lexed, parse_triggered_line_lexed,
};
use super::super::compile_support::compile_statement_effects;
use super::super::grammar::primitives::{
    self as grammar, TokenWordView, split_lexed_slices_on_and, split_lexed_slices_on_or,
};
use super::super::grammar::structure::parse_trailing_if_predicate_lexed;
use super::super::lexer::{OwnedLexToken, TokenKind, trim_lexed_commas};
use super::super::lowering_support::{
    rewrite_lower_static_ability_ast, rewrite_parsed_triggered_ability as parsed_triggered_ability,
};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::token_primitives::{
    find_str_by as find_word_index_by, find_window_index as find_word_sequence_index,
    slice_contains_str as word_slice_contains, slice_starts_with as word_slice_starts_with,
    str_contains as string_contains,
};
use super::super::util::{
    is_article, is_source_reference_words, parse_mana_symbol, parse_target_phrase,
    span_from_tokens, token_index_for_word_index, trim_commas,
};
use super::clause_dispatch::parse_become_clause;
use super::dispatch_inner::trim_edge_punctuation;
use super::lex_chain_helpers::find_verb_lexed;
use super::sentence_helpers::*;
#[allow(unused_imports)]
use super::{Verb, find_verb, parse_effect_chain};
use crate::ability::Ability;
use crate::cards::builders::{
    CardTextError, EffectAst, GrantedAbilityAst, IT_TAG, KeywordAction, LineAst, ParsedAbility,
    ReferenceImports, SubjectVerbActionAst, SubjectVerbEffectAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::{Until, Value};
use crate::mana::ManaCost;
use crate::static_abilities::{StaticAbility, StaticAbilityId};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

type GainAbilityWordView<'a> = TokenWordView<'a>;
type SharedSubjectPump = (
    Value,
    Value,
    usize,
    Until,
    Option<crate::ConditionExpr>,
    Option<(i32, i32, Value)>,
);
type SharedSubjectBasePt = (Value, Value, usize, Until);

const UNTIL_YOUR_NEXT_TURN_PREFIXES: &[&[&str]] = &[
    &["until", "your", "next", "turn"],
    &["until", "your", "next", "upkeep"],
];

const UNTIL_YOUR_NEXT_UNTAP_PREFIXES: &[&[&str]] = &[
    &["until", "your", "next", "untap", "step"],
    &["during", "your", "next", "untap", "step"],
];

const CHOICE_OF_ABILITY_PREFIXES: &[&[&str]] =
    &[&["your", "choice", "of"], &["your", "choice", "from"]];

fn display_text_for_tokens(tokens: &[OwnedLexToken]) -> String {
    let mut text = String::new();
    let mut needs_space = false;
    let mut in_effect_text = false;

    for token in tokens {
        if let Some(word) = token.as_word() {
            if needs_space && !text.is_empty() {
                text.push(' ');
            }
            let numeric_like = word
                .chars()
                .all(|ch| ch.is_ascii_digit() || matches!(ch, 'x' | 'X' | '+' | '-' | '/'));
            let rendered = match word {
                "t" => "{T}".to_string(),
                "q" => "{Q}".to_string(),
                _ if in_effect_text && numeric_like => word.to_string(),
                _ => parse_mana_symbol(word)
                    .map(|symbol| ManaCost::from_symbols(vec![symbol]).to_oracle())
                    .unwrap_or_else(|_| word.to_ascii_lowercase()),
            };
            text.push_str(&rendered);
            needs_space = true;
        } else if token.kind == TokenKind::ManaGroup {
            if needs_space && !text.is_empty() {
                text.push(' ');
            }
            text.push_str(token.slice.as_str());
            needs_space = true;
        } else if token.is_colon() {
            text.push(':');
            needs_space = true;
            in_effect_text = true;
        } else if token.is_comma() {
            text.push(',');
            needs_space = true;
        } else if token.is_period() {
            text.push('.');
            needs_space = true;
        } else if token.is_semicolon() {
            text.push(';');
            needs_space = true;
        }
    }

    text
}

fn grants_protection_from_everything(ability: &GrantedAbilityAst) -> bool {
    matches!(
        ability,
        GrantedAbilityAst::KeywordAction(KeywordAction::ProtectionFromEverything)
    )
}

fn append_shared_subject_pump_to_target(
    effects: &mut Vec<EffectAst>,
    target: &TargetAst,
    pump_effect: &Option<SharedSubjectPump>,
) {
    let Some((power, toughness, _, pump_duration, condition, for_each)) = pump_effect else {
        return;
    };
    if let Some((power_per, toughness_per, count)) = for_each {
        effects.push(EffectAst::subject_verb_pump_for_each(
            *power_per,
            *toughness_per,
            target.clone(),
            count.clone(),
            pump_duration.clone(),
        ));
    } else {
        effects.push(EffectAst::subject_verb_pump(
            power.clone(),
            toughness.clone(),
            target.clone(),
            pump_duration.clone(),
            condition.clone(),
        ));
    }
}

fn append_shared_subject_base_pt_to_target(
    effects: &mut Vec<EffectAst>,
    target: &TargetAst,
    base_pt_effect: &Option<SharedSubjectBasePt>,
) {
    let Some((power, toughness, _, duration)) = base_pt_effect else {
        return;
    };
    effects.push(EffectAst::subject_verb_set_base_power_toughness(
        power.clone(),
        toughness.clone(),
        target.clone(),
        duration.clone(),
    ));
}

fn parse_shared_subject_base_pt_from_has_tail(
    tokens: &[OwnedLexToken],
    has_word_idx: usize,
    duration: &Until,
) -> Result<Option<SharedSubjectBasePt>, CardTextError> {
    let Some(rest_start_token_idx) = token_index_for_word_index(tokens, has_word_idx + 1) else {
        return Ok(None);
    };
    let rest_tokens = trim_commas(&tokens[rest_start_token_idx..]);
    let rest_words = GainAbilityWordView::new(&rest_tokens).to_word_refs();
    if rest_words.len() < 5 || rest_words[..4] != ["base", "power", "and", "toughness"] {
        return Ok(None);
    }
    let (power, toughness) = parse_pt_modifier_values(rest_words[4]).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            GainAbilityWordView::new(tokens).to_word_refs().join(" ")
        ))
    })?;
    let tail = &rest_words[5..];
    if !tail.is_empty() && !is_until_end_of_turn(tail) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power/toughness clause (clause: '{}')",
            GainAbilityWordView::new(tokens).to_word_refs().join(" ")
        )));
    }
    Ok(Some((power, toughness, has_word_idx, duration.clone())))
}

fn parse_leading_subject_base_pt_before_gain(
    before_gain: &[&str],
    subject_start_word_idx: usize,
    gain_idx: usize,
) -> Result<Option<SharedSubjectBasePt>, CardTextError> {
    let Some(local_has_idx) = before_gain
        .iter()
        .position(|word| matches!(*word, "has" | "have"))
    else {
        return Ok(None);
    };
    if local_has_idx == 0 {
        return Ok(None);
    }
    let rest = &before_gain[local_has_idx + 1..];
    if rest.len() < 5 || rest[..4] != ["base", "power", "and", "toughness"] {
        return Ok(None);
    }
    let (power, toughness) = parse_pt_modifier_values(rest[4]).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            before_gain.join(" ")
        ))
    })?;
    let tail = &rest[5..];
    if !tail.is_empty()
        && !is_until_end_of_turn(tail)
        && tail != ["until", "end", "of", "turn", "and"]
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power/toughness clause (clause: '{}')",
            before_gain.join(" ")
        )));
    }
    let has_word_idx = subject_start_word_idx + local_has_idx;
    if has_word_idx >= gain_idx {
        return Ok(None);
    }
    Ok(Some((power, toughness, has_word_idx, Until::EndOfTurn)))
}

fn parse_shared_subject_pump_from_get_tail(
    tokens: &[OwnedLexToken],
    get_word_idx: usize,
    duration: &Until,
    has_explicit_duration: bool,
) -> Result<Option<SharedSubjectPump>, CardTextError> {
    let Some(modifier_start_token_idx) = token_index_for_word_index(tokens, get_word_idx + 1)
    else {
        return Ok(None);
    };
    let mut modifier_token_storage = trim_commas(&tokens[modifier_start_token_idx..]).to_vec();
    for token in &mut modifier_token_storage {
        token.lowercase_word();
    }
    let modifier_tokens = trim_commas(&modifier_token_storage);
    let Some(mod_word) = modifier_tokens.first().and_then(OwnedLexToken::as_word) else {
        return Ok(None);
    };
    let Ok((power, toughness)) = parse_pt_modifier_values(mod_word) else {
        return Ok(None);
    };
    let for_each =
        if let (Value::Fixed(power_per), Value::Fixed(toughness_per)) = (&power, &toughness) {
            parse_get_for_each_count_value(modifier_tokens.get(1..).unwrap_or_default())?
                .map(|count| (*power_per, *toughness_per, count))
        } else {
            None
        };
    let modifier_words = GainAbilityWordView::new(&modifier_tokens).to_word_refs();
    let has_local_duration = modifier_words
        .iter()
        .any(|word| matches!(*word, "until" | "during"));
    let (power, toughness, local_duration, condition) =
        parse_get_modifier_values_with_tail(&modifier_tokens, power, toughness)?;
    let pump_duration = if has_explicit_duration || !has_local_duration {
        duration.clone()
    } else {
        local_duration
    };
    Ok(Some((
        power,
        toughness,
        get_word_idx,
        pump_duration,
        condition,
        for_each,
    )))
}

fn parsed_static_granted_abilities(
    ability_tokens: &[OwnedLexToken],
    abilities: Vec<crate::cards::builders::StaticAbilityAst>,
) -> Result<Vec<GrantedAbilityAst>, CardTextError> {
    let display = display_text_for_tokens(ability_tokens);
    abilities
        .into_iter()
        .map(|ability| {
            let static_ability = rewrite_lower_static_ability_ast(ability)?;
            Ok(GrantedAbilityAst::ParsedObjectAbility {
                ability: ParsedAbility {
                    ability: Ability::static_ability(static_ability).into(),
                    text: Some(display.clone()),
                    effects_ast: None,
                    reference_imports: ReferenceImports::default(),
                    trigger_spec: None,
                },
                display: display.clone(),
            })
        })
        .collect()
}

fn player_gain_effects_for_abilities(
    abilities: &[GrantedAbilityAst],
    duration: &Until,
    subject_tokens: &[OwnedLexToken],
    player_filter: PlayerFilter,
) -> Option<Vec<EffectAst>> {
    let player_target = TargetAst::Player(
        player_filter.clone(),
        span_from_lexed_tokens(subject_tokens),
    );
    let mut effects = Vec::new();

    for ability in abilities {
        match ability {
            GrantedAbilityAst::KeywordAction(KeywordAction::Hexproof) => {
                effects.push(EffectAst::subject_verb_cant(
                    crate::effect::Restriction::be_targeted_player_from(
                        player_filter.clone(),
                        ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
                    ),
                    duration.clone(),
                    None,
                ));
            }
            GrantedAbilityAst::KeywordAction(KeywordAction::HexproofFrom(filter)) => {
                effects.push(EffectAst::subject_verb_cant(
                    crate::effect::Restriction::be_targeted_player_from(
                        player_filter.clone(),
                        filter.clone().controlled_by(PlayerFilter::Opponent),
                    ),
                    duration.clone(),
                    None,
                ));
            }
            GrantedAbilityAst::KeywordAction(KeywordAction::ProtectionFromEverything) => {
                effects.push(EffectAst::subject_verb_cant(
                    crate::effect::Restriction::be_targeted_player(player_filter.clone()),
                    duration.clone(),
                    None,
                ));
                effects.push(EffectAst::subject_verb_prevent_all_damage_to_target(
                    player_target.clone(),
                    duration.clone(),
                ));
            }
            _ => return None,
        }
    }

    Some(effects)
}

fn render_lower_words(tokens: &[OwnedLexToken]) -> String {
    let word_view = GainAbilityWordView::new(tokens);
    word_view.to_word_refs().join(" ")
}

fn push_unique_keyword_action(actions: &mut Vec<KeywordAction>, action: KeywordAction) {
    for existing in actions.iter() {
        if *existing == action {
            return;
        }
    }
    actions.push(action);
}

fn color_only_hexproof_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut filters = Vec::new();
    for token in tokens {
        if token.is_word("and") || token.is_word("from") {
            continue;
        }
        let color = crate::color::Color::from_name(token.as_word()?)?;
        let mut filter = ObjectFilter::default();
        filter.colors = Some(crate::color::ColorSet::from_color(color));
        filters.push(filter);
    }

    match filters.len() {
        0 => None,
        1 => filters.pop(),
        _ => {
            let mut filter = ObjectFilter::default();
            filter.any_of = filters;
            Some(filter)
        }
    }
}

fn parse_granted_ability_component_for_gain(
    ability_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    let ability_tokens = trim_edge_punctuation(ability_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }
    let ability_words = GainAbilityWordView::new(&ability_tokens).to_word_refs();
    const CANT_BE_BLOCKED_EXCEPT_BY_HASTE_PREFIXES: &[&[&str]] = &[
        &[
            "cant",
            "be",
            "blocked",
            "this",
            "turn",
            "except",
            "by",
            "creatures",
            "with",
            "haste",
        ],
        &[
            "cant",
            "be",
            "blocked",
            "except",
            "by",
            "creatures",
            "with",
            "haste",
        ],
    ];
    if CANT_BE_BLOCKED_EXCEPT_BY_HASTE_PREFIXES
        .iter()
        .any(|prefix| word_slice_starts_with(&ability_words, prefix))
    {
        let restriction = crate::effect::Restriction::block_specific_attacker(
            ObjectFilter::creature().without_static_ability(StaticAbilityId::Haste),
            ObjectFilter::source(),
        );
        return Ok(Some(vec![GrantedAbilityAst::StaticAbility(
            StaticAbility::restriction(
                restriction,
                "can't be blocked except by creatures with haste".to_string(),
            ),
        )]));
    }

    if grammar::words_match_any_prefix(&ability_tokens, &[&["hexproof", "from"]]).is_some() {
        if let Some(filter) = color_only_hexproof_filter(&ability_tokens[2..]) {
            return Ok(Some(vec![GrantedAbilityAst::from(
                KeywordAction::HexproofFrom(filter),
            )]));
        }
        let filter_tokens = ability_tokens[2..].to_vec();
        if !filter_tokens.is_empty()
            && let Ok(filter) = parse_object_filter_lexed(&filter_tokens, false)
        {
            return Ok(Some(vec![GrantedAbilityAst::from(
                KeywordAction::HexproofFrom(filter),
            )]));
        }
    }

    if let Some(granted) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, clause_words)?
    {
        return Ok(Some(vec![granted]));
    }

    if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        return Ok(Some(
            actions.into_iter().map(GrantedAbilityAst::from).collect(),
        ));
    }

    if let Some(abilities) = parse_static_ability_ast_line_lexed(&ability_tokens)? {
        return Ok(Some(parsed_static_granted_abilities(
            &ability_tokens,
            abilities,
        )?));
    }

    if let Some(action) = ability_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .filter(|_| ability_tokens.len() == 1)
        .and_then(parse_single_word_keyword_action)
    {
        return Ok(Some(vec![GrantedAbilityAst::from(action)]));
    }

    Ok(None)
}

pub(crate) fn parse_granted_abilities_for_gain_clause(
    ability_tokens: &[OwnedLexToken],
    clause_words: &[&str],
    allow_choice: bool,
) -> Result<(Vec<GrantedAbilityAst>, bool), CardTextError> {
    if let Some(abilities) = parse_granted_ability_component_for_gain(ability_tokens, clause_words)?
    {
        return Ok((abilities, false));
    }

    if allow_choice && let Some(actions) = parse_choice_of_abilities(ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        return Ok((
            actions.into_iter().map(GrantedAbilityAst::from).collect(),
            true,
        ));
    }

    let segments = split_lexed_slices_on_and(ability_tokens);
    if segments.len() <= 1 {
        return Ok((Vec::new(), false));
    }

    let mut abilities = Vec::new();
    for segment in segments {
        let Some(parsed) = parse_granted_ability_component_for_gain(segment, clause_words)? else {
            return Ok((Vec::new(), false));
        };
        abilities.extend(parsed);
    }

    Ok((abilities, false))
}

pub(crate) fn parse_simple_ability_duration(
    words_after_verb: &[&str],
) -> Option<(usize, usize, Until)> {
    if let Some(idx) = find_word_sequence_index(words_after_verb, &["until", "end", "of", "turn"]) {
        return Some((idx, 4, Until::EndOfTurn));
    }
    if let Some(idx) =
        find_word_sequence_index(words_after_verb, &["until", "your", "next", "turn"]).or_else(
            || find_word_sequence_index(words_after_verb, &["until", "your", "next", "upkeep"]),
        )
    {
        return Some((idx, 4, Until::YourNextTurn));
    }
    if let Some(idx) = find_word_sequence_index(
        words_after_verb,
        &["until", "your", "next", "untap", "step"],
    )
    .or_else(|| {
        find_word_sequence_index(
            words_after_verb,
            &["during", "your", "next", "untap", "step"],
        )
    }) {
        return Some((idx, 5, Until::YourNextTurn));
    }
    if let Some(idx) = find_word_sequence_index(
        words_after_verb,
        &["for", "as", "long", "as", "you", "control"],
    ) {
        return Some((
            idx,
            words_after_verb.len().saturating_sub(idx),
            Until::YouStopControllingThis,
        ));
    }
    None
}

fn words_start_nested_triggered_ability(words_after_verb: &[&str]) -> bool {
    matches!(
        words_after_verb,
        ["when", ..] | ["whenever", ..] | ["at", "the", ..]
    )
}

fn parse_leading_simple_ability_duration(tokens: &[OwnedLexToken]) -> Option<(usize, Until)> {
    let clause_word_view = GainAbilityWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if starts_with_until_end_of_turn(&clause_words) {
        return Some((4, Until::EndOfTurn));
    }
    if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, UNTIL_YOUR_NEXT_TURN_PREFIXES)
    {
        return Some((prefix.len(), Until::YourNextTurn));
    }
    if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, UNTIL_YOUR_NEXT_UNTAP_PREFIXES)
    {
        return Some((prefix.len(), Until::YourNextTurn));
    }
    None
}

pub(crate) fn parse_simple_gain_ability_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause_lexed(tokens, false)
}

pub(crate) fn parse_simple_lose_ability_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause_lexed(tokens, true)
}

fn lexed_token_index_for_word_index(tokens: &[OwnedLexToken], word_idx: usize) -> Option<usize> {
    GainAbilityWordView::new(tokens).token_index_for_word_index(word_idx)
}

fn span_from_lexed_tokens(tokens: &[OwnedLexToken]) -> Option<TextSpan> {
    match (tokens.first(), tokens.last()) {
        (Some(first), Some(last)) => Some(TextSpan {
            line: first.span.line,
            start: first.span.start,
            end: last.span.end,
        }),
        _ => None,
    }
}

fn trim_trailing_also(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut end = tokens.len();
    while end > 0 && tokens[end - 1].is_word("also") {
        end -= 1;
    }
    &tokens[..end]
}

fn source_target_from_subject_tokens(tokens: &[OwnedLexToken]) -> Option<TargetAst> {
    let subject_words = GainAbilityWordView::new(tokens).to_word_refs();
    for prefix_len in (1..=subject_words.len()).rev() {
        if !is_source_reference_words(&subject_words[..prefix_len]) {
            continue;
        }

        if prefix_len == subject_words.len()
            || find_verb_lexed(&tokens[prefix_len..]).is_some_and(|(_, verb_idx)| verb_idx == 0)
        {
            return Some(TargetAst::Source(span_from_lexed_tokens(
                &tokens[..prefix_len],
            )));
        }
    }

    None
}

fn parse_simple_ability_modifier_clause_lexed(
    tokens: &[OwnedLexToken],
    losing: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_word_view = GainAbilityWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let verb_idx = find_word_index_by(&clause_words, |word| {
        if losing {
            matches!(word, "lose" | "loses")
        } else {
            matches!(word, "gain" | "gains")
        }
    });
    let Some(verb_idx) = verb_idx else {
        return Ok(None);
    };
    let implied_it_subject = verb_idx == 0;
    let Some(verb_token_idx) = lexed_token_index_for_word_index(tokens, verb_idx) else {
        return Ok(None);
    };

    if !losing && matches!(clause_words[verb_idx], "gain" | "gains") {
        let starts_with_life = clause_words
            .get(verb_idx + 1)
            .is_some_and(|word| *word == "life");
        let starts_with_control = clause_words
            .get(verb_idx + 1)
            .is_some_and(|word| *word == "control");
        if starts_with_life || starts_with_control {
            return Ok(None);
        }
    }

    let leading_duration_phrase = parse_leading_simple_ability_duration(tokens);
    let subject_start_token_idx = leading_duration_phrase
        .as_ref()
        .map(|(start_word_idx, _)| {
            lexed_token_index_for_word_index(tokens, *start_word_idx).unwrap_or(tokens.len())
        })
        .unwrap_or(0);
    if subject_start_token_idx > verb_token_idx {
        return Ok(None);
    }

    let subject_tokens = trim_trailing_also(trim_lexed_commas(
        &tokens[subject_start_token_idx..verb_token_idx],
    ));
    if subject_tokens.is_empty() && !implied_it_subject {
        return Ok(None);
    }

    if !losing
        && !subject_tokens.is_empty()
        && let Some((subject_verb, _)) = find_verb_lexed(subject_tokens)
        && subject_verb != Verb::Get
    {
        let subject_words = GainAbilityWordView::new(&subject_tokens);
        let subject_word_refs = subject_words.to_word_refs();
        let target_phrase_with_controller_tail = subject_word_refs.first().copied()
            == Some("target")
            && (word_slice_contains(&subject_word_refs, "control")
                || word_slice_contains(&subject_word_refs, "controls"));
        if !target_phrase_with_controller_tail {
            return Ok(None);
        }
    }

    let words_after_verb = &clause_words[verb_idx + 1..];
    if words_after_verb.is_empty() {
        return Ok(None);
    }

    let duration_phrase = if words_start_nested_triggered_ability(words_after_verb) {
        None
    } else {
        parse_simple_ability_duration(words_after_verb)
    };
    let duration = duration_phrase
        .as_ref()
        .map(|(_, _, duration)| duration.clone())
        .or_else(|| {
            leading_duration_phrase
                .as_ref()
                .map(|(_, duration)| duration.clone())
        })
        .unwrap_or(Until::Forever);

    let shared_gain_tail_word_idx = if losing {
        words_after_verb
            .windows(2)
            .position(|window| matches!(window, ["and", "gain"] | ["and", "gains"]))
    } else {
        None
    };
    let shared_get_tail_word_idx = if !losing {
        words_after_verb
            .windows(2)
            .position(|window| matches!(window, ["and", "get"] | ["and", "gets"]))
    } else {
        None
    };
    let shared_has_tail_word_idx = if losing {
        words_after_verb
            .windows(2)
            .position(|window| matches!(window, ["and", "has"] | ["and", "have"]))
    } else {
        None
    };
    let ability_end_word_idx = shared_gain_tail_word_idx
        .or(shared_get_tail_word_idx)
        .or(shared_has_tail_word_idx)
        .map(|idx| verb_idx + 1 + idx)
        .unwrap_or_else(|| {
            duration_phrase
                .as_ref()
                .map(|(start, _, _)| verb_idx + 1 + *start)
                .unwrap_or(clause_words.len())
        });
    let ability_end_token_idx =
        lexed_token_index_for_word_index(tokens, ability_end_word_idx).unwrap_or(tokens.len());
    let ability_tokens = trim_edge_punctuation(trim_lexed_commas(
        &tokens[verb_token_idx + 1..ability_end_token_idx],
    ));
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let ability_word_refs = GainAbilityWordView::new(&ability_tokens).to_word_refs();
    let (abilities, _) = if losing && matches!(ability_word_refs.as_slice(), ["this", "ability"]) {
        (vec![GrantedAbilityAst::ThisAbility], false)
    } else {
        parse_granted_abilities_for_gain_clause(&ability_tokens, &clause_words, false)?
    };
    let removes_all_abilities =
        losing && matches!(ability_word_refs.as_slice(), ["all", "abilities"]);
    if abilities.is_empty() && !removes_all_abilities {
        return Ok(None);
    }
    let abilities = abilities;
    reject_unsupported_lost_abilities(losing, &abilities)?;

    if let Some((start, len, _)) = duration_phrase {
        let tail_word_idx = verb_idx + 1 + start + len;
        if let Some(tail_token_idx) = lexed_token_index_for_word_index(tokens, tail_word_idx) {
            let trailing = trim_lexed_commas(&tokens[tail_token_idx..]);
            if !trailing.is_empty() {
                return Ok(None);
            }
        }
    } else if shared_gain_tail_word_idx.is_some() {
        // Shared-subject chains such as "loses all abilities and gains flying"
        // are accepted here for the remove-abilities half. The gain half is
        // handled by higher-level chain parsing when it can be split safely.
    } else if shared_get_tail_word_idx.is_some() {
        // Shared-subject chains such as "gains menace and gets +X/+0"
        // are accepted here for the grant-ability half.
    } else if shared_has_tail_word_idx.is_some() {
        // Shared-subject chains such as "loses all abilities and has base
        // power and toughness 1/1" are accepted here for the remove-abilities
        // half. The characteristic-setting half is handled by chain parsing.
    }

    let subject_words = GainAbilityWordView::new(subject_tokens);
    let subject_word_refs = subject_words.to_word_refs();
    let is_pronoun_subject =
        implied_it_subject || matches!(subject_word_refs.as_slice(), ["it"] | ["they"] | ["them"]);
    if is_pronoun_subject {
        let target =
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_lexed_tokens(subject_tokens));
        if losing {
            return Ok(Some(EffectAst::subject_verb_remove_abilities_from_target(
                target, abilities, duration,
            )));
        }
        return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
            target, abilities, duration,
        )));
    }

    if let Some(target) = source_target_from_subject_tokens(&subject_tokens) {
        if losing {
            return Ok(Some(EffectAst::subject_verb_remove_abilities_from_target(
                target, abilities, duration,
            )));
        }
        return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
            target, abilities, duration,
        )));
    }

    let is_demonstrative_subject = subject_word_refs
        .first()
        .is_some_and(|word| *word == "that" || *word == "those");
    if is_demonstrative_subject || word_slice_contains(&subject_word_refs, "target") {
        let target = parse_target_phrase(subject_tokens)?;
        if losing {
            return Ok(Some(EffectAst::subject_verb_remove_abilities_from_target(
                target, abilities, duration,
            )));
        }
        return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
            target, abilities, duration,
        )));
    }

    if !losing
        && (subject_word_refs.as_slice() == ["players"]
            || subject_word_refs.as_slice() == ["all", "players"])
    {
        let Some(mut player_effects) = player_gain_effects_for_abilities(
            &abilities,
            &duration,
            subject_tokens,
            PlayerFilter::Any,
        ) else {
            return Ok(None);
        };
        if player_effects.len() == 1 {
            return Ok(player_effects.pop());
        }
        return Ok(None);
    }

    let filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in {}-ability clause (clause: '{}')",
            if losing { "lose" } else { "gain" },
            clause_words.join(" ")
        ))
    })?;
    if losing {
        return Ok(Some(EffectAst::subject_verb_remove_abilities_all(
            filter, abilities, duration,
        )));
    }
    Ok(Some(EffectAst::subject_verb_grant_abilities_all(
        filter, abilities, duration,
    )))
}

pub(crate) fn parse_simple_gain_ability_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause(tokens, false)
}

pub(crate) fn parse_simple_lose_ability_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause(tokens, true)
}

pub(crate) fn parse_simple_ability_modifier_clause(
    tokens: &[OwnedLexToken],
    losing: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_word_view = GainAbilityWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let verb_idx = find_word_index_by(&clause_words, |word| {
        if losing {
            matches!(word, "lose" | "loses")
        } else {
            matches!(word, "gain" | "gains")
        }
    });
    let Some(verb_idx) = verb_idx else {
        return Ok(None);
    };
    let implied_it_subject = verb_idx == 0;
    let Some(verb_token_idx) = token_index_for_word_index(tokens, verb_idx) else {
        return Ok(None);
    };

    if !losing && matches!(clause_words[verb_idx], "gain" | "gains") {
        let starts_with_life = clause_words
            .get(verb_idx + 1)
            .is_some_and(|word| *word == "life");
        let starts_with_control = clause_words
            .get(verb_idx + 1)
            .is_some_and(|word| *word == "control");
        if starts_with_life || starts_with_control {
            return Ok(None);
        }
    }

    let leading_duration_phrase = parse_leading_simple_ability_duration(tokens);
    let subject_start_token_idx = leading_duration_phrase
        .as_ref()
        .map(|(start_word_idx, _)| {
            token_index_for_word_index(tokens, *start_word_idx).unwrap_or(tokens.len())
        })
        .unwrap_or(0);
    if subject_start_token_idx > verb_token_idx {
        return Ok(None);
    }

    let subject_token_storage = trim_commas(&tokens[subject_start_token_idx..verb_token_idx]);
    let subject_tokens = trim_trailing_also(&subject_token_storage);
    if subject_tokens.is_empty() && !implied_it_subject {
        return Ok(None);
    }

    if !losing
        && !subject_tokens.is_empty()
        && let Some((subject_verb, _)) = find_verb(&subject_tokens)
        && subject_verb != Verb::Get
    {
        let subject_words = GainAbilityWordView::new(&subject_tokens);
        let subject_word_refs = subject_words.to_word_refs();
        let target_phrase_with_controller_tail = subject_word_refs.first().copied()
            == Some("target")
            && (word_slice_contains(&subject_word_refs, "control")
                || word_slice_contains(&subject_word_refs, "controls"));
        if !target_phrase_with_controller_tail {
            return Ok(None);
        }
    }

    let words_after_verb = &clause_words[verb_idx + 1..];
    if words_after_verb.is_empty() {
        return Ok(None);
    }

    let duration_phrase = parse_simple_ability_duration(words_after_verb);
    let duration = duration_phrase
        .as_ref()
        .map(|(_, _, duration)| duration.clone())
        .or_else(|| {
            leading_duration_phrase
                .as_ref()
                .map(|(_, duration)| duration.clone())
        })
        .unwrap_or(Until::Forever);

    let ability_end_word_idx = duration_phrase
        .as_ref()
        .map(|(start, _, _)| verb_idx + 1 + *start)
        .unwrap_or(clause_words.len());
    let ability_end_token_idx =
        token_index_for_word_index(tokens, ability_end_word_idx).unwrap_or(tokens.len());
    let ability_token_storage = trim_commas(&tokens[verb_token_idx + 1..ability_end_token_idx]);
    let ability_tokens = trim_edge_punctuation(&ability_token_storage);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let ability_word_refs = GainAbilityWordView::new(&ability_tokens).to_word_refs();
    let (abilities, _) = if losing && matches!(ability_word_refs.as_slice(), ["this", "ability"]) {
        (vec![GrantedAbilityAst::ThisAbility], false)
    } else {
        parse_granted_abilities_for_gain_clause(&ability_tokens, &clause_words, false)?
    };
    if abilities.is_empty() {
        return Ok(None);
    }
    let abilities = abilities;

    if let Some((start, len, _)) = duration_phrase {
        let tail_word_idx = verb_idx + 1 + start + len;
        if let Some(tail_token_idx) = token_index_for_word_index(tokens, tail_word_idx) {
            let trailing = trim_commas(&tokens[tail_token_idx..]);
            if !trailing.is_empty() {
                return Ok(None);
            }
        }
    }

    let subject_words = GainAbilityWordView::new(&subject_tokens);
    let subject_word_refs = subject_words.to_word_refs();
    let is_pronoun_subject =
        implied_it_subject || matches!(subject_word_refs.as_slice(), ["it"] | ["they"] | ["them"]);
    if is_pronoun_subject {
        let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&subject_tokens));
        if losing {
            return Ok(Some(EffectAst::subject_verb_remove_abilities_from_target(
                target, abilities, duration,
            )));
        }
        return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
            target, abilities, duration,
        )));
    }

    if let Some(target) = source_target_from_subject_tokens(&subject_tokens) {
        if losing {
            return Ok(Some(EffectAst::subject_verb_remove_abilities_from_target(
                target, abilities, duration,
            )));
        }
        return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
            target, abilities, duration,
        )));
    }

    let is_demonstrative_subject = subject_word_refs
        .first()
        .is_some_and(|word| *word == "that" || *word == "those");
    if is_demonstrative_subject || word_slice_contains(&subject_word_refs, "target") {
        let target = parse_target_phrase(&subject_tokens)?;
        if losing {
            return Ok(Some(EffectAst::subject_verb_remove_abilities_from_target(
                target, abilities, duration,
            )));
        }
        return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
            target, abilities, duration,
        )));
    }

    if !losing
        && (subject_word_refs.as_slice() == ["players"]
            || subject_word_refs.as_slice() == ["all", "players"])
    {
        let Some(mut player_effects) = player_gain_effects_for_abilities(
            &abilities,
            &duration,
            &subject_tokens,
            PlayerFilter::Any,
        ) else {
            return Ok(None);
        };
        if player_effects.len() == 1 {
            return Ok(player_effects.pop());
        }
        return Ok(None);
    }

    let filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in {}-ability clause (clause: '{}')",
            if losing { "lose" } else { "gain" },
            clause_words.join(" ")
        ))
    })?;
    if losing {
        return Ok(Some(EffectAst::subject_verb_remove_abilities_all(
            filter, abilities, duration,
        )));
    }
    Ok(Some(EffectAst::subject_verb_grant_abilities_all(
        filter, abilities, duration,
    )))
}

pub(crate) fn parse_gain_ability_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let word_view = GainAbilityWordView::new(&tokens);
    let word_list = word_view.to_word_refs();
    let looks_like_can_attack_no_defender =
        find_word_sequence_index(&word_list, &["can", "attack"]).is_some()
            && find_word_sequence_index(&word_list, &["as", "though"]).is_some()
            && word_slice_contains(&word_list, "defender");
    if looks_like_can_attack_no_defender {
        return Ok(None);
    }
    let gain_idx = find_word_index_by(&word_list, |word| {
        matches!(word, "gain" | "gains" | "lose" | "loses")
    })
    .or_else(|| find_word_index_by(&word_list, |word| matches!(word, "has" | "have")));
    let Some(gain_idx) = gain_idx else {
        return Ok(None);
    };
    let Some(gain_token_idx) = token_index_for_word_index(tokens, gain_idx) else {
        return Ok(None);
    };
    if let Some((Verb::Create, create_idx)) = find_verb(tokens)
        && create_idx < gain_token_idx
        && word_slice_contains(&word_list, "token")
    {
        return Ok(None);
    }
    let losing = matches!(word_list[gain_idx], "lose" | "loses");

    let after_gain = &word_list[gain_idx + 1..];
    if matches!(word_list[gain_idx], "gain" | "gains") {
        let starts_with_life = after_gain.first().is_some_and(|word| *word == "life");
        let starts_with_control = after_gain.first().is_some_and(|word| *word == "control");
        if starts_with_life || starts_with_control {
            return Ok(None);
        }
    }

    let leading_duration_phrase = if starts_with_until_end_of_turn(&word_list) {
        Some((4usize, Until::EndOfTurn))
    } else if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, UNTIL_YOUR_NEXT_TURN_PREFIXES)
    {
        Some((prefix.len(), Until::YourNextTurn))
    } else if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, UNTIL_YOUR_NEXT_UNTAP_PREFIXES)
    {
        Some((prefix.len(), Until::YourNextTurn))
    } else {
        None
    };
    let subject_start_word_idx = leading_duration_phrase
        .as_ref()
        .map(|(len, _)| *len)
        .unwrap_or(0);
    let subject_start_token_idx = if subject_start_word_idx == 0 {
        0usize
    } else if let Some(idx) = token_index_for_word_index(tokens, subject_start_word_idx) {
        idx
    } else {
        return Ok(None);
    };
    if subject_start_token_idx < gain_token_idx
        && let Some((subject_verb, _)) = find_verb(&tokens[subject_start_token_idx..gain_token_idx])
        && subject_verb != Verb::Get
    {
        let subject_tokens = trim_commas(&tokens[subject_start_token_idx..gain_token_idx]);
        let subject_words = GainAbilityWordView::new(&subject_tokens);
        let subject_word_refs = subject_words.to_word_refs();
        let target_phrase_with_controller_tail = subject_word_refs.first().copied()
            == Some("target")
            && (word_slice_contains(&subject_word_refs, "control")
                || word_slice_contains(&subject_word_refs, "controls"));
        let controller_tail_subject = word_slice_contains(&subject_word_refs, "control")
            || word_slice_contains(&subject_word_refs, "controls");
        let object_filter_subject = parse_object_filter(&subject_tokens, false).is_ok();
        if !target_phrase_with_controller_tail && !controller_tail_subject && !object_filter_subject
        {
            return Ok(None);
        }
    }

    let duration_phrase = if words_start_nested_triggered_ability(after_gain) {
        None
    } else {
        parse_simple_ability_duration(after_gain)
    };
    let duration = duration_phrase
        .as_ref()
        .map(|(_, _, duration)| duration.clone())
        .or_else(|| {
            leading_duration_phrase
                .as_ref()
                .map(|(_, duration)| duration.clone())
        })
        .unwrap_or(Until::Forever);
    let has_explicit_duration =
        duration_phrase.is_some() || leading_duration_phrase.as_ref().is_some();

    let shared_get_tail_word_idx = if !losing {
        after_gain
            .windows(2)
            .position(|window| matches!(window, ["and", "get"] | ["and", "gets"]))
    } else {
        None
    };
    let shared_has_tail_word_idx = if losing {
        after_gain
            .windows(2)
            .position(|window| matches!(window, ["and", "has"] | ["and", "have"]))
    } else {
        None
    };
    let following_pump_effect = if let Some(shared_idx) = shared_get_tail_word_idx {
        let get_word_idx = gain_idx + 1 + shared_idx + 1;
        parse_shared_subject_pump_from_get_tail(
            tokens,
            get_word_idx,
            &duration,
            has_explicit_duration,
        )?
    } else {
        None
    };
    if shared_get_tail_word_idx.is_some() && following_pump_effect.is_none() {
        return Ok(None);
    }
    let following_base_pt_effect = if let Some(shared_idx) = shared_has_tail_word_idx {
        let has_word_idx = gain_idx + 1 + shared_idx + 1;
        parse_shared_subject_base_pt_from_has_tail(tokens, has_word_idx, &duration)?
    } else {
        None
    };
    if shared_has_tail_word_idx.is_some() && following_base_pt_effect.is_none() {
        return Ok(None);
    }

    let mut trailing_tail_tokens: Vec<OwnedLexToken> = Vec::new();
    if shared_get_tail_word_idx.is_none()
        && let Some((start_rel, len_words, _)) = duration_phrase
    {
        let tail_word_idx = gain_idx + 1 + start_rel + len_words;
        if let Some(tail_token_idx) = token_index_for_word_index(tokens, tail_word_idx) {
            let mut tail_tokens = trim_commas(&tokens[tail_token_idx..]).to_vec();
            while tail_tokens
                .first()
                .is_some_and(|token| token.is_word("and") || token.is_word("then"))
            {
                tail_tokens.remove(0);
            }
            if !tail_tokens.is_empty() {
                trailing_tail_tokens = tail_tokens;
            }
        }
    }
    let mut grants_must_attack = false;
    if !trailing_tail_tokens.is_empty() {
        let tail_view = GainAbilityWordView::new(&trailing_tail_tokens);
        let mut tail_words = tail_view.to_word_refs();
        if tail_words.first().is_some_and(|word| *word == "and") {
            tail_words = tail_words[1..].to_vec();
        }
        if tail_words.as_slice() == ["attacks", "this", "combat", "if", "able"]
            || tail_words.as_slice() == ["attack", "this", "combat", "if", "able"]
        {
            grants_must_attack = true;
            trailing_tail_tokens.clear();
        }
    }

    let ability_end_word_idx = duration_phrase
        .as_ref()
        .map(|(start_rel, _, _)| gain_idx + 1 + *start_rel);
    let ability_end_word_idx = shared_get_tail_word_idx
        .or(shared_has_tail_word_idx)
        .map(|idx| gain_idx + 1 + idx)
        .or(ability_end_word_idx);
    let ability_end_token_idx = if let Some(end_word_idx) = ability_end_word_idx {
        token_index_for_word_index(tokens, end_word_idx).unwrap_or(tokens.len())
    } else {
        tokens.len()
    };
    let ability_start_token_idx = gain_token_idx + 1;
    if ability_start_token_idx > ability_end_token_idx || ability_start_token_idx >= tokens.len() {
        return Ok(None);
    }
    let ability_tokens = trim_commas(&tokens[ability_start_token_idx..ability_end_token_idx]);

    let (mut abilities, grant_is_choice) =
        parse_granted_abilities_for_gain_clause(&ability_tokens, &word_list, !losing)?;
    if !trailing_tail_tokens.is_empty() {
        let mut tail_tokens = trailing_tail_tokens.as_slice();
        if tail_tokens
            .first()
            .is_some_and(|token| token.is_word("and") || token.is_word("then"))
        {
            tail_tokens = &tail_tokens[1..];
        }
        let (trailing_abilities, trailing_is_choice) =
            parse_granted_abilities_for_gain_clause(tail_tokens, &word_list, false)?;
        if !trailing_abilities.is_empty() && !trailing_is_choice {
            abilities.extend(trailing_abilities);
            trailing_tail_tokens.clear();
        }
    }
    let removes_all_abilities = losing
        && GainAbilityWordView::new(&ability_tokens)
            .to_word_refs()
            .as_slice()
            == ["all", "abilities"];
    if abilities.is_empty() && !grants_must_attack && !removes_all_abilities {
        return Ok(None);
    }
    if grants_must_attack {
        abilities.push(GrantedAbilityAst::MustAttack);
    }
    reject_unsupported_lost_abilities(losing, &abilities)?;

    // Check for "gets +X/+Y and gains/has/loses ..." patterns - if there's a pump
    // modifier before the ability verb, extract it as a separate Pump/PumpAll effect.
    let before_gain = &word_list[subject_start_word_idx..gain_idx];
    let leading_become_effect = if let Some(become_idx) =
        find_word_index_by(before_gain, |word| matches!(word, "become" | "becomes"))
    {
        let become_word_idx = subject_start_word_idx + become_idx;
        let Some(become_token_idx) = token_index_for_word_index(tokens, become_word_idx) else {
            return Ok(None);
        };
        let become_subject_tokens = trim_commas(&tokens[subject_start_token_idx..become_token_idx]);
        let mut become_tail_tokens =
            trim_commas(&tokens[become_token_idx + 1..gain_token_idx]).to_vec();
        while become_tail_tokens
            .last()
            .is_some_and(|token| token.is_word("and") || token.is_word("then"))
        {
            become_tail_tokens.pop();
        }
        let become_tail_tokens = trim_commas(&become_tail_tokens);
        if become_subject_tokens.is_empty() || become_tail_tokens.is_empty() {
            None
        } else {
            let mut become_effect =
                parse_become_clause(&become_subject_tokens, &become_tail_tokens)?;
            if has_explicit_duration {
                apply_gain_clause_duration_to_leading_effect(&mut become_effect, &duration);
            }
            Some(become_effect)
        }
    } else {
        None
    };
    let get_idx = find_word_index_by(before_gain, |word| matches!(word, "get" | "gets"));
    let leading_base_pt_effect = if !losing {
        parse_leading_subject_base_pt_before_gain(before_gain, subject_start_word_idx, gain_idx)?
    } else {
        None
    };
    let pump_effect = if let Some(gi) = get_idx {
        let modifier_start_word_idx = subject_start_word_idx + gi + 1;
        let Some(modifier_start_token_idx) =
            token_index_for_word_index(tokens, modifier_start_word_idx)
        else {
            return Ok(None);
        };
        let mut modifier_tokens =
            trim_commas(&tokens[modifier_start_token_idx..gain_token_idx]).to_vec();
        while modifier_tokens
            .last()
            .is_some_and(|token| token.is_word("and") || token.is_word("then"))
        {
            modifier_tokens.pop();
        }
        let modifier_tokens = trim_commas(&modifier_tokens);
        if let Some(mod_word) = modifier_tokens.first().and_then(OwnedLexToken::as_word) {
            if let Ok((power, toughness)) = parse_pt_modifier_values(mod_word) {
                let for_each = if let (Value::Fixed(power_per), Value::Fixed(toughness_per)) =
                    (&power, &toughness)
                {
                    parse_get_for_each_count_value(modifier_tokens.get(1..).unwrap_or_default())?
                        .map(|count| (*power_per, *toughness_per, count))
                } else {
                    None
                };
                let modifier_words = GainAbilityWordView::new(&modifier_tokens).to_word_refs();
                let has_local_duration = modifier_words
                    .iter()
                    .any(|word| matches!(*word, "until" | "during"));
                let (power, toughness, local_duration, condition) =
                    parse_get_modifier_values_with_tail(&modifier_tokens, power, toughness)?;
                let pump_duration = if has_explicit_duration || !has_local_duration {
                    duration.clone()
                } else {
                    local_duration
                };
                Some((
                    power,
                    toughness,
                    subject_start_word_idx + gi,
                    pump_duration,
                    condition,
                    for_each,
                ))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    if !losing
        && let Some((power, toughness, _gi, pump_duration, condition, for_each)) = &pump_effect
        && let Some(local_get_idx) = get_idx
        && let Some(and_idx) = before_gain
            .iter()
            .enumerate()
            .skip(local_get_idx + 1)
            .find_map(|(idx, word)| (*word == "and").then_some(idx))
        && and_idx + 1 < before_gain.len()
    {
        let source_subject_words = &before_gain[..local_get_idx];
        if matches!(
            source_subject_words,
            ["this"] | ["this", "creature"] | ["this", "permanent"]
        ) {
            let filter_tokens = before_gain[and_idx + 1..]
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if let Ok(filter) = parse_object_filter(&filter_tokens, false) {
                let mut effects = Vec::new();
                let source_target = TargetAst::Source(None);
                if let Some((power_per, toughness_per, count)) = for_each {
                    effects.push(EffectAst::subject_verb_pump_for_each(
                        *power_per,
                        *toughness_per,
                        source_target,
                        count.clone(),
                        pump_duration.clone(),
                    ));
                } else {
                    effects.push(EffectAst::subject_verb_pump(
                        power.clone(),
                        toughness.clone(),
                        source_target,
                        pump_duration.clone(),
                        condition.clone(),
                    ));
                }
                if grant_is_choice {
                    effects.push(EffectAst::subject_verb_grant_abilities_choice_all(
                        filter, abilities, duration,
                    ));
                } else {
                    effects.push(EffectAst::subject_verb_grant_abilities_all(
                        filter, abilities, duration,
                    ));
                }
                effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
                return Ok(Some(effects));
            }
        }
    }
    let has_have_verb = matches!(word_list[gain_idx], "has" | "have");
    let has_nested_granted_ability = abilities
        .iter()
        .any(|ability| matches!(ability, GrantedAbilityAst::ParsedObjectAbility { .. }));
    if has_have_verb
        && pump_effect.is_none()
        && !has_explicit_duration
        && !has_nested_granted_ability
    {
        return Ok(None);
    }

    // Determine the real subject (before "get"/"gets" if pump is present)
    let real_subject_end_word_idx = pump_effect
        .as_ref()
        .map(|(_, _, gi, _, _, _)| *gi)
        .or(leading_base_pt_effect
            .as_ref()
            .map(|(_, _, has_idx, _)| *has_idx))
        .unwrap_or(gain_idx);
    let real_subject_start_word_idx = if let Some(gi) = get_idx {
        before_gain[..gi]
            .iter()
            .enumerate()
            .rev()
            .find_map(|(idx, word)| {
                if matches!(*word, "it" | "they" | "target") {
                    let mut start_idx = idx;
                    if idx >= 3 && before_gain[idx - 3..idx] == ["up", "to", "one"] {
                        start_idx = idx - 3;
                    } else if idx >= 3 && before_gain[idx - 3..idx] == ["up", "to", "x"] {
                        start_idx = idx - 3;
                    } else if idx >= 3 && before_gain[idx - 3..idx] == ["any", "number", "of"] {
                        start_idx = idx - 3;
                    } else if idx >= 1 && before_gain[idx - 1] == "x" {
                        start_idx = idx - 1;
                    } else if idx >= 4 && before_gain[idx - 4..idx] == ["each", "of", "up", "to"] {
                        start_idx = idx - 4;
                    }
                    Some(subject_start_word_idx + start_idx)
                } else if *word == "this"
                    && before_gain.get(idx + 1).is_some_and(|next| {
                        matches!(*next, "creature" | "permanent" | "spell" | "card")
                    })
                {
                    Some(subject_start_word_idx + idx)
                } else {
                    None
                }
            })
            .unwrap_or(subject_start_word_idx)
    } else {
        subject_start_word_idx
    };
    let real_subject_start_token_idx =
        token_index_for_word_index(tokens, real_subject_start_word_idx)
            .unwrap_or(subject_start_token_idx);
    let real_subject_end_token_idx =
        token_index_for_word_index(tokens, real_subject_end_word_idx).unwrap_or(gain_token_idx);
    if real_subject_start_token_idx >= real_subject_end_token_idx {
        return Ok(None);
    }
    let real_subject_token_storage =
        trim_commas(&tokens[real_subject_start_token_idx..real_subject_end_token_idx]);
    let real_subject_tokens = trim_trailing_also(&real_subject_token_storage);

    let mut effects = Vec::new();

    // Check for pronoun subjects ("it", "they") that reference a prior tagged object.
    let real_subject_word_view = GainAbilityWordView::new(&real_subject_tokens);
    let real_subject_words = real_subject_word_view.to_word_refs();
    let is_pronoun_subject =
        real_subject_words.as_slice() == ["it"] || real_subject_words.as_slice() == ["they"];
    if is_pronoun_subject {
        let span = span_from_tokens(&real_subject_tokens);
        let target = TargetAst::Tagged(TagKey::from(IT_TAG), span);
        if let Some(become_effect) = &leading_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_base_pt_to_target(&mut effects, &target, &leading_base_pt_effect);
        append_shared_subject_pump_to_target(&mut effects, &target, &pump_effect);
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                target.clone(),
                abilities,
                duration,
            ));
        } else if grant_is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_to_target(
                target.clone(),
                abilities,
                duration,
            ));
        } else {
            effects.push(EffectAst::subject_verb_grant_abilities_to_target(
                target.clone(),
                abilities,
                duration,
            ));
        }
        append_shared_subject_pump_to_target(&mut effects, &target, &following_pump_effect);
        append_shared_subject_base_pt_to_target(&mut effects, &target, &following_base_pt_effect);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if let Some(target) = source_target_from_subject_tokens(&real_subject_tokens) {
        if let Some(become_effect) = &leading_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_base_pt_to_target(&mut effects, &target, &leading_base_pt_effect);
        append_shared_subject_pump_to_target(&mut effects, &target, &pump_effect);
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                target.clone(),
                abilities,
                duration,
            ));
        } else if grant_is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_to_target(
                target.clone(),
                abilities,
                duration,
            ));
        } else {
            effects.push(EffectAst::subject_verb_grant_abilities_to_target(
                target.clone(),
                abilities,
                duration,
            ));
        }
        append_shared_subject_pump_to_target(&mut effects, &target, &following_pump_effect);
        append_shared_subject_base_pt_to_target(&mut effects, &target, &following_base_pt_effect);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let is_demonstrative_subject = real_subject_words
        .first()
        .is_some_and(|word| *word == "that" || *word == "those");
    if is_demonstrative_subject {
        let target = parse_target_phrase(&real_subject_tokens)?;
        if let Some(become_effect) = &leading_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_base_pt_to_target(&mut effects, &target, &leading_base_pt_effect);
        append_shared_subject_pump_to_target(&mut effects, &target, &pump_effect);
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                target.clone(),
                abilities,
                duration,
            ));
        } else if grant_is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_to_target(
                target.clone(),
                abilities,
                duration,
            ));
        } else {
            effects.push(EffectAst::subject_verb_grant_abilities_to_target(
                target.clone(),
                abilities,
                duration,
            ));
        }
        append_shared_subject_pump_to_target(&mut effects, &target, &following_pump_effect);
        append_shared_subject_base_pt_to_target(&mut effects, &target, &following_base_pt_effect);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if word_slice_contains(before_gain, "target") {
        let has_preceding_target_effect = pump_effect.is_some() || leading_become_effect.is_some();
        let target = parse_target_phrase(&real_subject_tokens)?;
        if let Some(become_effect) = &leading_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_base_pt_to_target(&mut effects, &target, &leading_base_pt_effect);
        append_shared_subject_pump_to_target(&mut effects, &target, &pump_effect);
        let grant_target = if has_preceding_target_effect {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&real_subject_tokens))
        } else {
            target
        };
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                grant_target,
                abilities,
                duration,
            ));
        } else if grant_is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_to_target(
                grant_target,
                abilities,
                duration,
            ));
        } else {
            effects.push(EffectAst::subject_verb_grant_abilities_to_target(
                grant_target,
                abilities,
                duration,
            ));
        }
        let following_pump_target =
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&real_subject_tokens));
        append_shared_subject_pump_to_target(
            &mut effects,
            &following_pump_target,
            &following_pump_effect,
        );
        append_shared_subject_base_pt_to_target(
            &mut effects,
            &following_pump_target,
            &following_base_pt_effect,
        );
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if !losing && real_subject_words.as_slice() == ["you"] {
        let has_protection_from_everything =
            abilities.iter().any(grants_protection_from_everything);
        if has_protection_from_everything {
            let player_target =
                TargetAst::Player(PlayerFilter::You, span_from_tokens(&real_subject_tokens));
            effects.push(EffectAst::subject_verb_cant(
                crate::effect::Restriction::be_targeted_player(PlayerFilter::You),
                duration.clone(),
                None,
            ));
            effects.push(EffectAst::subject_verb_prevent_all_damage_to_target(
                player_target,
                duration.clone(),
            ));
            effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
            return Ok(Some(effects));
        }
    }

    if !losing && real_subject_words.as_slice() == ["you", "and", "permanents", "you", "control"] {
        let permanent_filter = crate::target::ObjectFilter::permanent().you_control();
        let Some(mut player_effects) = player_gain_effects_for_abilities(
            &abilities,
            &duration,
            &real_subject_tokens,
            PlayerFilter::You,
        ) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed player/permanent gain-ability clause (clause: '{}')",
                word_list.join(" ")
            )));
        };
        effects.append(&mut player_effects);
        effects.push(EffectAst::subject_verb_grant_abilities_all(
            permanent_filter,
            abilities,
            duration,
        ));
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if !losing
        && (real_subject_words.as_slice() == ["players"]
            || real_subject_words.as_slice() == ["all", "players"])
    {
        let Some(mut player_effects) = player_gain_effects_for_abilities(
            &abilities,
            &duration,
            &real_subject_tokens,
            PlayerFilter::Any,
        ) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported player gain-ability clause (clause: '{}')",
                word_list.join(" ")
            )));
        };
        effects.append(&mut player_effects);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let filter = parse_object_filter(&real_subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in {}-ability clause (clause: '{}')",
            if losing { "lose" } else { "gain" },
            word_list.join(" ")
        ))
    })?;

    if let Some(become_effect) = &leading_become_effect {
        effects.push(become_effect.clone());
    }
    if let Some((power, toughness, _has_idx, base_pt_duration)) = &leading_base_pt_effect {
        effects.push(EffectAst::subject_verb_set_base_power_toughness(
            power.clone(),
            toughness.clone(),
            TargetAst::Object(filter.clone(), None, None),
            base_pt_duration.clone(),
        ));
    }
    if let Some((power, toughness, _, pump_duration, _condition, _for_each)) = pump_effect {
        effects.push(EffectAst::subject_verb_pump_all(
            filter.clone(),
            power,
            toughness,
            pump_duration,
        ));
    }
    if losing {
        effects.push(EffectAst::subject_verb_remove_abilities_all(
            filter.clone(),
            abilities,
            duration,
        ));
    } else if grant_is_choice {
        effects.push(EffectAst::subject_verb_grant_abilities_choice_all(
            filter.clone(),
            abilities,
            duration,
        ));
    } else {
        effects.push(EffectAst::subject_verb_grant_abilities_all(
            filter.clone(),
            abilities,
            duration,
        ));
    }
    if let Some((power, toughness, _, pump_duration, _condition, _for_each)) = following_pump_effect
    {
        effects.push(EffectAst::subject_verb_pump_all(
            filter.clone(),
            power,
            toughness,
            pump_duration,
        ));
    }
    if let Some((power, toughness, _, base_pt_duration)) = following_base_pt_effect {
        effects.push(EffectAst::subject_verb_set_base_power_toughness(
            power,
            toughness,
            TargetAst::Object(filter.clone(), None, None),
            base_pt_duration,
        ));
    }
    effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;

    Ok(Some(effects))
}

fn reject_unsupported_lost_abilities(
    losing: bool,
    abilities: &[GrantedAbilityAst],
) -> Result<(), CardTextError> {
    if !losing {
        return Ok(());
    }
    if abilities.iter().any(|ability| {
        matches!(
            ability,
            GrantedAbilityAst::KeywordAction(KeywordAction::Soulbond)
        )
    }) {
        return Err(CardTextError::ParseError(
            "removing soulbond requires non-marker semantics".to_string(),
        ));
    }
    Ok(())
}

fn apply_gain_clause_duration_to_leading_effect(effect: &mut EffectAst, duration: &Until) {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SetBasePowerToughness {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasePtCreature {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetBasePower {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddSubtypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasicLandType {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetColors {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::MakeColorless {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasicLandTypeChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeCreatureTypeChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeColorChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeCopy {
                    duration: effect_duration,
                    ..
                },
            ..
        }) => {
            *effect_duration = duration.clone();
        }
        _ => {}
    }
}

pub(crate) fn parse_granted_activated_or_triggered_ability_for_gain(
    ability_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<GrantedAbilityAst>, CardTextError> {
    let ability_tokens = trim_edge_punctuation(ability_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let has_colon = ability_tokens.iter().any(|token| token.is_colon());
    let looks_like_trigger = ability_tokens.first().is_some_and(|token| {
        token.is_word("when")
            || token.is_word("whenever")
            || (token.is_word("at")
                && ability_tokens
                    .get(1)
                    .is_some_and(|next| next.is_word("the")))
    });
    if !has_colon && !looks_like_trigger {
        return Ok(None);
    }

    let display = display_text_for_tokens(&ability_tokens);
    let parsed_ability = if has_colon {
        let Some(parsed) = parse_activated_line(&ability_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        parsed
    } else {
        if let Some(parsed) = parse_granted_triggered_otherwise_ability(&ability_tokens, &display)?
        {
            parsed
        } else {
            match parse_triggered_line_lexed(&ability_tokens)? {
                LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn,
                } => parsed_triggered_ability(
                    trigger,
                    effects,
                    vec![Zone::Battlefield],
                    Some(display.clone()),
                    crate::runtime_backend::trigger_frequency_condition(
                        Some(display.as_str()),
                        max_triggers_per_turn,
                    ),
                    None,
                    ReferenceImports::default(),
                ),
                _ => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported granted activated/triggered ability clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
            }
        }
    };

    Ok(Some(GrantedAbilityAst::ParsedObjectAbility {
        ability: parsed_ability,
        display,
    }))
}

fn parse_granted_triggered_otherwise_ability(
    ability_tokens: &[OwnedLexToken],
    display: &str,
) -> Result<Option<ParsedAbility>, CardTextError> {
    let start_idx = if ability_tokens.first().is_some_and(|token| {
        token.is_word("when") || token.is_word("whenever") || token.is_word("at")
    }) {
        1
    } else {
        0
    };
    let Some(comma_idx) = ability_tokens.iter().position(OwnedLexToken::is_comma) else {
        return Ok(None);
    };
    let Some(otherwise_idx) = ability_tokens
        .iter()
        .position(|token| token.is_word("otherwise"))
    else {
        return Ok(None);
    };
    if otherwise_idx <= comma_idx + 1 || comma_idx <= start_idx {
        return Ok(None);
    }

    let trigger = parse_trigger_clause_lexed(&ability_tokens[start_idx..comma_idx])?;
    let true_tokens = trim_edge_punctuation(trim_lexed_commas(
        &ability_tokens[comma_idx + 1..otherwise_idx],
    ));
    let false_tokens =
        trim_edge_punctuation(trim_lexed_commas(&ability_tokens[otherwise_idx + 1..]));
    if true_tokens.is_empty() || false_tokens.is_empty() {
        return Ok(None);
    }

    let mut true_effects = parse_effect_chain(&true_tokens)?;
    if true_effects.len() != 1 {
        return Ok(None);
    }
    let mut conditional = true_effects.remove(0);
    let EffectAst::Conditional { if_false, .. } = &mut conditional else {
        return Ok(None);
    };
    if !if_false.is_empty() {
        return Ok(None);
    }
    *if_false = parse_effect_chain(&false_tokens)?;
    if if_false.is_empty() {
        return Ok(None);
    }

    Ok(Some(parsed_triggered_ability(
        trigger,
        vec![conditional],
        vec![Zone::Battlefield],
        Some(display.to_string()),
        None,
        None,
        ReferenceImports::default(),
    )))
}

pub(crate) fn append_gain_ability_trailing_effects(
    mut effects: Vec<EffectAst>,
    trailing_tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if trailing_tokens.is_empty() {
        return Ok(effects);
    }

    let trimmed = trim_commas(trailing_tokens);
    if let Some(predicate) = parse_trailing_if_predicate_lexed(&trimmed) {
        return Ok(vec![EffectAst::Conditional {
            predicate,
            if_true: effects,
            if_false: Vec::new(),
        }]);
    }

    if trimmed.first().is_some_and(|token| token.is_word("unless")) {
        if let Some(unless_effect) = try_build_unless(effects, &trimmed, 0)? {
            return Ok(vec![unless_effect]);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing unless gain-ability clause (clause: '{}')",
            render_lower_words(&trimmed)
        )));
    }

    if let Ok(parsed_tail) = parse_effect_chain(&trimmed)
        && !parsed_tail.is_empty()
    {
        effects.extend(parsed_tail);
    }
    Ok(effects)
}

pub(crate) fn parse_choice_of_abilities(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    let tokens = trim_commas(tokens);
    let word_view = GainAbilityWordView::new(&tokens);
    let word_list = word_view.to_word_refs();
    let prefix_words = if let Some((prefix, _)) =
        grammar::words_match_any_prefix(&tokens, CHOICE_OF_ABILITY_PREFIXES)
    {
        prefix.len()
    } else {
        return None;
    };
    if word_list.len() <= prefix_words + 1 {
        return None;
    }

    let start_idx = token_index_for_word_index(&tokens, prefix_words)?;
    let option_tokens = trim_commas(&tokens[start_idx..]);
    if option_tokens.is_empty() {
        return None;
    }

    let mut actions = Vec::new();
    for segment in split_lexed_slices_on_or(&option_tokens) {
        let segment = trim_commas(segment);
        if segment.is_empty() {
            continue;
        }
        let action = parse_ability_phrase(&segment)?;
        push_unique_keyword_action(&mut actions, action);
    }

    if actions.len() < 2 {
        return None;
    }
    Some(actions)
}

pub(crate) fn parse_gain_ability_to_source_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_word_view = GainAbilityWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let gain_idx = find_word_index_by(&clause_words, |word| matches!(word, "gain" | "gains"));
    let Some(gain_idx) = gain_idx else {
        return Ok(None);
    };

    let Some(gain_token_idx) = token_index_for_word_index(tokens, gain_idx) else {
        return Ok(None);
    };
    let subject_tokens = &tokens[..gain_token_idx];
    let subject_word_view = GainAbilityWordView::new(subject_tokens);
    let subject_words: Vec<&str> = subject_word_view
        .to_word_refs()
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if !is_source_reference_words(&subject_words) {
        return Ok(None);
    }

    let words_after_gain = &clause_words[gain_idx + 1..];
    let duration_phrase = parse_simple_ability_duration(words_after_gain);
    let duration = duration_phrase
        .as_ref()
        .map(|(_, _, duration)| duration.clone())
        .unwrap_or(Until::Forever);
    let ability_end_word_idx = duration_phrase
        .as_ref()
        .map(|(start_rel, _, _)| gain_idx + 1 + *start_rel)
        .unwrap_or(clause_words.len());
    let ability_end_token_idx =
        token_index_for_word_index(tokens, ability_end_word_idx).unwrap_or(tokens.len());
    let ability_tokens = trim_edge_punctuation(&tokens[gain_token_idx + 1..ability_end_token_idx]);
    if let Some(parsed) = parse_activated_line(&ability_tokens)? {
        return Ok(Some(EffectAst::subject_verb_grant_ability_to_source(
            parsed, duration,
        )));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::super::super::lexer::lex_line;
    use super::super::super::util::tokenize_line;
    use super::*;
    use crate::CardId;
    use crate::ability::AbilityKind;
    use crate::cards::builders::CardDefinitionBuilder;

    #[test]
    fn gain_ability_to_source_keeps_parsed_ability_until_lowering() {
        let tokens = tokenize_line("This creature gains {T}: Draw a card.", 0);
        let effect = parse_gain_ability_to_source_sentence(&tokens)
            .expect("gain-to-source sentence should parse")
            .expect("gain-to-source sentence should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "GrantAbilityToSource"),
            "expected source grant effect, got {debug}"
        );
        assert!(
            string_contains(&debug, "duration: Forever"),
            "source ability grants without an explicit duration should be indefinite, got {debug}"
        );
        assert!(
            string_contains(&debug, "effects_ast: Some"),
            "expected parsed ability to remain unlowered in the AST, got {debug}"
        );

        let compiled =
            compile_statement_effects(&[effect]).expect("grant-to-source effect should lower");
        let compiled_debug = format!("{compiled:?}");
        assert!(
            (string_contains(&compiled_debug, "ApplyContinuousEffect")
                && string_contains(&compiled_debug, "AddAbilityGeneric")
                && string_contains(&compiled_debug, "target_spec: Some(Source)")
                && string_contains(&compiled_debug, "until: Forever"))
                || (string_contains(&compiled_debug, "GrantObjectAbilityEffect")
                    && string_contains(&compiled_debug, "target: Source")),
            "expected source grant effect after lowering, got {compiled_debug}"
        );
    }

    #[test]
    fn gain_ability_to_source_respects_explicit_until_end_of_turn_duration() {
        let tokens = tokenize_line("This creature gains {T}: Draw a card until end of turn.", 0);
        let effect = parse_gain_ability_to_source_sentence(&tokens)
            .expect("gain-to-source sentence should parse")
            .expect("gain-to-source sentence should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "GrantAbilityToSource"),
            "expected source grant effect, got {debug}"
        );
        assert!(
            string_contains(&debug, "duration: EndOfTurn"),
            "explicit source ability grant duration should be preserved, got {debug}"
        );
    }

    #[test]
    fn target_gain_activated_ability_stays_unlowered_until_compile() {
        let tokens = tokenize_line(
            "Target creature gains {T}: Draw a card until end of turn.",
            0,
        );
        let effect = parse_simple_gain_ability_clause(&tokens)
            .expect("target gain clause should parse")
            .expect("target gain clause should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "ParsedObjectAbility"),
            "expected parsed granted ability in AST, got {debug}"
        );
        assert!(
            string_contains(&debug, "effects_ast: Some"),
            "expected granted ability to remain unlowered in AST, got {debug}"
        );

        let compiled =
            compile_statement_effects(&[effect]).expect("target gain clause should lower");
        let compiled_debug = format!("{compiled:?}");
        assert!(
            string_contains(&compiled_debug, "ApplyContinuousEffect")
                && (string_contains(&compiled_debug, "AddAbilityGeneric")
                    || string_contains(&compiled_debug, "GrantObjectAbilityForFilter")),
            "expected lowered granted ability effect, got {compiled_debug}"
        );
    }

    #[test]
    fn target_lose_activated_ability_stays_unlowered_until_compile() {
        let tokens = tokenize_line(
            "Target creature loses {T}: Draw a card until end of turn.",
            0,
        );
        let effect = parse_simple_lose_ability_clause(&tokens)
            .expect("target lose clause should parse")
            .expect("target lose clause should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "ParsedObjectAbility"),
            "expected parsed removed ability in AST, got {debug}"
        );
        assert!(
            string_contains(&debug, "effects_ast: Some"),
            "expected removed ability to remain unlowered in AST, got {debug}"
        );

        let compiled =
            compile_statement_effects(&[effect]).expect("target lose clause should lower");
        let compiled_debug = format!("{compiled:?}");
        assert!(
            string_contains(&compiled_debug, "RemoveAbility"),
            "expected lowered remove-ability effect, got {compiled_debug}"
        );
        assert!(
            string_contains(&compiled_debug, "ApplyContinuousEffect"),
            "expected removed ability to lower through a continuous effect, got {compiled_debug}"
        );
    }

    #[test]
    fn pump_and_lose_ability_sentence_keeps_shared_until_your_next_turn() {
        let tokens = tokenize_line(
            "Target creature gets -2/-0 and loses flying until your next turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("pump-and-lose sentence should parse")
            .expect("pump-and-lose sentence should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "Pump") && string_contains(&debug, "RemoveAbilitiesFromTarget"),
            "expected pump plus remove-ability effects, got {debug}"
        );
        assert!(
            debug.matches("YourNextTurn").count() >= 2,
            "expected shared duration to apply to both effects, got {debug}"
        );
    }

    #[test]
    fn base_pt_then_gains_keyword_in_single_clause_parses() {
        let tokens = tokenize_line(
            "This creature has base power and toughness 4/5 until end of turn and gains wither until end of turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("base-pt then gains clause should parse")
            .expect("base-pt then gains clause should produce effects");

        let debug = format!("{effects:?}").to_ascii_lowercase();
        assert!(
            string_contains(&debug, "setbasepowertoughness")
                && string_contains(&debug, "grantabilitiestotarget")
                && string_contains(&debug, "wither")
                && debug.matches("endofturn").count() >= 2,
            "expected shared self-targeted base P/T plus wither grant until EOT, got {debug}"
        );
    }

    #[test]
    fn gain_landwalk_until_next_upkeep_sentence_parses() {
        let tokens = tokenize_line(
            "Target non-Wall creature an opponent controls gains forestwalk until your next upkeep.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("gain-until-next-upkeep sentence should parse")
            .expect("gain-until-next-upkeep sentence should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "GrantAbilitiesToTarget"),
            "expected target ability grant, got {debug}"
        );
        assert!(
            string_contains(&debug, "Landwalk(Subtype { subtype: Forest, snow: false })")
                && string_contains(&debug, "YourNextTurn"),
            "expected forestwalk grant to keep next-upkeep duration, got {debug}"
        );
    }

    #[test]
    fn lexed_gain_landwalk_until_next_upkeep_sentence_parses() {
        let mut tokens = lex_line(
            "Target non-Wall creature an opponent controls gains forestwalk until your next upkeep.",
            0,
        )
        .expect("rewrite lexer should classify landwalk gain clause");
        for token in &mut tokens {
            token.lowercase_word();
        }
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("lexed gain-until-next-upkeep sentence should parse")
            .expect("lexed gain-until-next-upkeep sentence should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "GrantAbilitiesToTarget"),
            "expected target ability grant, got {debug}"
        );
        assert!(
            string_contains(&debug, "Landwalk(Subtype { subtype: Forest, snow: false })")
                && string_contains(&debug, "YourNextTurn"),
            "expected forestwalk grant to keep next-upkeep duration, got {debug}"
        );
    }

    #[test]
    fn gain_haste_and_except_by_haste_with_trailing_where_clause_keeps_unblockable_grant() {
        let tokens = tokenize_line(
            "Up to X target creatures you control each gain haste until end of turn and can't be blocked this turn except by creatures with haste, where X is the number of Bobbleheads you control as you activate this ability.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("agility bobblehead-style grant clause should parse")
            .expect("agility bobblehead-style grant clause should produce effects");

        let debug = format!("{effects:?}").to_ascii_lowercase();
        assert!(
            string_contains(&debug, "haste")
                && string_contains(&debug, "can't be blocked except by creatures with haste"),
            "expected haste plus except-by-haste unblockable grant, got {debug}"
        );
    }

    #[test]
    fn you_and_permanents_gain_hexproof_splits_player_and_permanent_grants() {
        let tokens = tokenize_line(
            "You and permanents you control gain hexproof until end of turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("mixed player/permanent grant should parse")
            .expect("mixed player/permanent grant should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "Cant")
                && string_contains(&debug, "BeTargetedPlayerFrom")
                && string_contains(&debug, "GrantAbilitiesAll")
                && string_contains(&debug, "Hexproof"),
            "expected player hexproof restriction plus permanent hexproof grant, got {debug}"
        );
    }

    #[test]
    fn you_and_permanents_gain_hexproof_from_keeps_player_grant_opponent_scoped() {
        let tokens = tokenize_line(
            "You and permanents you control gain hexproof from blue and from black until end of turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("mixed player/permanent hexproof-from grant should parse")
            .expect("mixed player/permanent hexproof-from grant should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "BeTargetedPlayerFrom")
                && string_contains(&debug, "Opponent")
                && string_contains(&debug, "GrantAbilitiesAll")
                && string_contains(&debug, "HexproofFrom"),
            "expected player hexproof-from restriction to apply only to opponents' sources plus permanent hexproof-from grant, got {debug}"
        );
    }

    #[test]
    fn gain_ability_subject_ignores_also_before_gain() {
        let tokens = tokenize_line(
            "Permanents you control also gain indestructible until end of turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("also-gain sentence should parse")
            .expect("also-gain sentence should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "GrantAbilitiesAll")
                && string_contains(&debug, "Indestructible"),
            "expected also to be ignored in the subject filter, got {debug}"
        );
    }

    #[test]
    fn dawns_truce_gift_line_compiles_promised_and_not_promised_branches() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dawn's Truce")
            .parse_text(
                "Gift a card (You may promise an opponent a gift as you cast this spell. If you do, they draw a card before its other effects.)\nYou and permanents you control gain hexproof until end of turn. If the gift was promised, permanents you control also gain indestructible until end of turn.",
            )
            .expect("Dawn's Truce gift text should parse");

        let debug = format!("{def:#?}");
        assert!(
            string_contains(&debug, "ThisSpellPaidLabel")
                && string_contains(&debug, "\"Gift\"")
                && string_contains(&debug, "EmitGiftGiven")
                && string_contains(&debug, "Hexproof")
                && string_contains(&debug, "Indestructible"),
            "expected Gift condition, gift event, hexproof, and indestructible effects, got {debug}"
        );
    }

    #[test]
    fn source_reference_simple_gain_clause_keeps_leading_duration_and_source_target() {
        let tokens = tokenize_line("Until end of turn, this creature gains flying.", 0);
        let effect = parse_simple_gain_ability_clause(&tokens)
            .expect("source-referenced simple gain clause should parse")
            .expect("source-referenced simple gain clause should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "GrantAbilitiesToTarget"),
            "expected a self-targeted temporary grant effect, got {debug}"
        );
        assert!(
            string_contains(&debug, "Source("),
            "expected the simple gain clause to stay targeted on the source, got {debug}"
        );
        assert!(
            string_contains(&debug, "EndOfTurn"),
            "expected the leading duration to survive lowering, got {debug}"
        );
        assert!(
            !string_contains(&debug, "GrantAbilitiesAll"),
            "expected no broad battlefield-wide grant effect, got {debug}"
        );
    }

    #[test]
    fn source_reference_simple_lose_clause_keeps_leading_duration_and_source_target() {
        let tokens = tokenize_line("Until end of turn, this creature loses defender.", 0);
        let effect = parse_simple_lose_ability_clause(&tokens)
            .expect("source-referenced simple lose clause should parse")
            .expect("source-referenced simple lose clause should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "RemoveAbilitiesFromTarget"),
            "expected a self-targeted temporary removal effect, got {debug}"
        );
        assert!(
            string_contains(&debug, "Source("),
            "expected the simple lose clause to stay targeted on the source, got {debug}"
        );
        assert!(
            string_contains(&debug, "EndOfTurn"),
            "expected the leading duration to survive lowering, got {debug}"
        );
        assert!(
            !string_contains(&debug, "RemoveAbilitiesAll"),
            "expected no broad battlefield-wide removal effect, got {debug}"
        );
    }

    #[test]
    fn quoted_granted_trigger_keeps_all_sentences_inside_the_grant() {
        let tokens = tokenize_line(
            "Until end of turn, permanents your opponents control gain \"When this permanent deals damage to the player who cast Hellish Rebuke, sacrifice this permanent. You lose 2 life.\"",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("quoted granted trigger should parse")
            .expect("quoted granted trigger should produce effects");

        assert_eq!(
            effects.len(),
            1,
            "quoted granted trigger should stay inside a single grant effect: {effects:?}"
        );

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "GrantAbilitiesAll"),
            "expected a global grant effect, got {debug}"
        );
        assert!(
            string_contains(&debug, "ParsedObjectAbility"),
            "expected parsed granted ability payload, got {debug}"
        );
        assert!(
            string_contains(&debug, "LoseLife"),
            "expected lose-life text to remain inside the granted ability payload, got {debug}"
        );
    }

    #[test]
    fn quoted_granted_trigger_keeps_trailing_if_otherwise_branch() {
        let tokens = tokenize_line(
            "Sliver creatures you control have \"When this creature enters, Slivers you control get +1/+1 until end of turn if you're the monarch. Otherwise, you become the monarch.\"",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("quoted monarch trigger should parse")
            .expect("quoted monarch trigger should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "GrantAbilitiesAll")
                && string_contains(&debug, "Conditional")
                && string_contains(&debug, "PlayerIsMonarch")
                && string_contains(&debug, "BecomeMonarch"),
            "expected granted trigger to keep monarch if/otherwise effects, got {debug}"
        );
    }

    #[test]
    fn hellish_rebuke_lowering_keeps_lose_life_inside_granted_trigger() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hellish Rebuke")
            .parse_text(
                "Until end of turn, permanents your opponents control gain \"When this permanent deals damage to the player who cast Hellish Rebuke, sacrifice this permanent. You lose 2 life.\"",
            )
            .expect("hellish rebuke grant line should parse");

        let spell_effects = def
            .spell_effect
            .as_ref()
            .expect("hellish rebuke should compile to spell effects");
        assert_eq!(
            spell_effects.len(),
            1,
            "lose life should not be hoisted to a top-level spell effect: {spell_effects:?}"
        );

        let debug = format!("{spell_effects:?}");
        assert!(
            string_contains(&debug, "AddAbilityGeneric")
                && string_contains(&debug, "TriggeredAbility")
                && string_contains(&debug, "LoseLifeEffect")
                && (string_contains(&debug, "sacrifice_source")
                    || string_contains(&debug, "SacrificeTargetEffect { target: Source }")),
            "granted trigger should keep its inline trigger effects together, got {debug}"
        );
        assert!(
            string_contains(&debug, "this_deals_damage_to_player")
                || string_contains(&debug, "ThisDealsDamageTrigger"),
            "granted trigger should constrain damage-to-player semantics: {debug}"
        );
    }

    #[test]
    fn mixed_keyword_and_quoted_trigger_grant_stays_targeted() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Strength of Will")
            .parse_text(
                "Until end of turn, target creature you control gains indestructible and \"Whenever this creature is dealt damage, put that many +1/+1 counters on it.\"",
            )
            .expect("strength of will grant line should parse");

        let debug = format!("{:?}", def.spell_effect);
        assert!(
            string_contains(&debug, "TriggeredAbility"),
            "grant should keep the quoted triggered ability payload: {debug}"
        );

        let rendered = format!("{def:#?}").to_ascii_lowercase();
        let compact_rendered = rendered.split_whitespace().collect::<String>();
        assert!(
            string_contains(&compact_rendered, "targetonlyeffect")
                && string_contains(&compact_rendered, "controller:some(you")
                && string_contains(&compact_rendered, "addability")
                && string_contains(&compact_rendered, "indestructible")
                && string_contains(&compact_rendered, "addabilitygeneric")
                && string_contains(&compact_rendered, "isdealtdamage")
                && string_contains(&compact_rendered, "putcounterseffect"),
            "grant should stay targeted in the lowered structure: {rendered}"
        );
    }

    #[test]
    fn players_gain_hexproof_clause_parses_as_player_wide_targeting_restriction() {
        let tokens = tokenize_line("Players gain hexproof until end of turn.", 0);
        let effect = parse_simple_gain_ability_clause(&tokens)
            .expect("players gain clause should parse")
            .expect("players gain clause should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "Cant")
                && string_contains(&debug, "BeTargetedPlayerFrom(Any")
                && string_contains(&debug, "EndOfTurn"),
            "expected a player-wide temporary targeting restriction, got {debug}"
        );
    }
}
