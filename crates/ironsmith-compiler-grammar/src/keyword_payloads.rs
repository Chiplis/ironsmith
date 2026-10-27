use crate::cards::builders::{
    CardTextError, EffectAst, LineAst, PlayerAst, PredicateAst, StaticAbilityAst, TargetAst,
    TriggerSpec,
};
use crate::effect::Value;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

use super::activation_and_restrictions::{
    parse_channel_line_lexed, parse_craft_line_lexed, parse_cycling_line_lexed,
    parse_equip_line_lexed, parse_fortify_line_lexed, parse_reconfigure_line_lexed,
};
use super::clause_support::parse_effect_sentences_lexed;
use super::grammar::abilities::{
    additional_cost_tail_tokens_lexed, is_additional_cost_choice_line_lexed,
    is_standard_gift_keyword_tokens_lexed,
};
use super::grammar::keyword_dispatch::{
    KeywordPrefixShape, KeywordSpecialFormShape, parse_keyword_prefix_shape_tokens,
    parse_keyword_special_form_shape_tokens,
};
use super::grammar::keyword_special_lines::parse_behold_and_exile_additional_cost_tokens;
use super::grammar::splice_keyword_lines::parse_splice_keyword_line_tokens;
use super::ir::RewriteKeywordLine;
use super::keyword_static::{
    parse_if_this_spell_costs_less_to_cast_line_lexed, parse_static_ability_ast_line_lexed,
};
use super::lexer::{
    OwnedLexToken, TokenKind, TokenWordView, split_lexed_sentences, token_slice_at_is,
    token_slice_first_is, trim_lexed_commas,
};
use super::preprocess::PreprocessedLine;
use super::recognized_document::{KeywordLineKind, KeywordLinePayload};
use super::semantic_line_parsing::{
    parse_exert_attack_keyword_line, parse_gift_keyword_line, parse_keyword_special_cases,
};
use super::token_primitives::locate_index as locate_token_index;
use super::util::{
    leading_mana_cost_from_tokens, parse_additional_cost_choice_options_lexed,
    parse_bargain_line_lexed, parse_bestow_line_lexed, parse_blitz_line_lexed,
    parse_buyback_line_lexed, parse_cast_this_spell_only_line_lexed, parse_entwine_line_lexed,
    parse_epic_line_lexed, parse_escalate_line_lexed, parse_escape_line_lexed,
    parse_eternalize_line_lexed, parse_evoke_line_lexed,
    parse_flash_with_additional_cost_line_lexed, parse_flashback_line_lexed,
    parse_harmonize_line_lexed, parse_if_conditional_alternative_cost_line_lexed,
    parse_jump_start_line_lexed, parse_kicker_line_lexed, parse_madness_line_lexed,
    parse_morph_keyword_line_lexed, parse_multikicker_line_lexed, parse_offspring_line_lexed,
    parse_prowl_line_lexed, parse_reinforce_line_lexed, parse_replicate_line_lexed,
    parse_retrace_line_lexed, parse_self_free_cast_alternative_cost_line_lexed,
    parse_squad_line_lexed, parse_transfigure_line_lexed, parse_transmute_line_lexed,
    parse_warp_line_lexed, parse_you_may_rather_than_spell_cost_line_lexed,
};

type KeywordParseResult = Result<Option<KeywordLinePayload>, CardTextError>;

fn ast(ast: LineAst) -> Option<KeywordLinePayload> {
    Some(KeywordLinePayload::ast(ast))
}

fn rewrite_context(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
    kind: KeywordLineKind,
) -> RewriteKeywordLine {
    RewriteKeywordLine {
        info: line.info.clone(),
        kind,
        parse_tokens: tokens.to_vec(),
        full_parse_tokens: full_tokens.to_vec(),
        payload: KeywordLinePayload::ast(LineAst::Abilities(Vec::new())),
    }
}

fn optional_cost_tail_effect_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let comma_idx = locate_token_index(tokens, |token| token.kind == TokenKind::Comma)?;
    let effect_tokens = trim_lexed_commas(tokens.get(comma_idx + 1..).unwrap_or_default());
    (!effect_tokens.is_empty()).then_some(effect_tokens)
}

fn keyword_tokens_for_shape<'a>(
    tokens: &'a [OwnedLexToken],
    full_tokens: &'a [OwnedLexToken],
    shape: KeywordPrefixShape,
) -> Option<&'a [OwnedLexToken]> {
    if parse_keyword_prefix_shape_tokens(tokens) == Some(shape) {
        return Some(tokens);
    }
    if parse_keyword_prefix_shape_tokens(full_tokens) == Some(shape) {
        return Some(full_tokens);
    }
    None
}

fn is_supported_sneak_line(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        parse_keyword_special_form_shape_tokens(tokens),
        Some(KeywordSpecialFormShape::SpellSneak | KeywordSpecialFormShape::PermanentSneak)
    )
}

pub(super) fn parse_additional_cost_choice(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if !is_additional_cost_choice_line_lexed(tokens) {
        return Ok(None);
    }
    let Some(effect_tokens) = additional_cost_tail_tokens_lexed(tokens) else {
        return Ok(None);
    };
    let Some(options) = parse_additional_cost_choice_options_lexed(effect_tokens)? else {
        return Ok(None);
    };
    let mut options = options
        .into_iter()
        .map(
            |option| crate::model::compiler_semantic::AdditionalCostChoiceOptionAst {
                description: option.description,
                effects: option.effects,
            },
        )
        .collect::<Vec<_>>();
    unify_additional_cost_choice_object_tags(&mut options);
    Ok(ast(LineAst::AdditionalCostChoice { options }))
}

/// "choose a creature you control or reveal a creature card from your hand":
/// whichever option is paid, later text names its object with one noun ("the
/// creature you chose or the card you revealed"), so every object-choosing
/// option exports the same tag.
fn unify_additional_cost_choice_object_tags(
    options: &mut [crate::model::compiler_semantic::AdditionalCostChoiceOptionAst],
) {
    let mut branches = options
        .iter_mut()
        .map(|option| &mut option.effects)
        .collect::<Vec<_>>();
    unify_object_choice_branch_tags(&mut branches);
}

/// Every branch that starts by choosing an object exports one shared tag.
fn unify_object_choice_branch_tags(branches: &mut [&mut Vec<EffectAst>]) {
    use crate::cards::builders::ObjectChoiceEffectAst;
    use ironsmith_core::tag::TagKeyWalk as _;
    let chosen_tags = branches
        .iter()
        .map(|effects| match effects.first() {
            Some(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                tag, ..
            })) => Some(tag.key.clone()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>();
    let Some(chosen_tags) = chosen_tags else {
        return;
    };
    let shared: crate::cards::builders::TagKey =
        crate::tag::declared_key("additional_cost_chosen_object").into();
    for (effects, old) in branches.iter_mut().zip(chosen_tags) {
        for effect in effects.iter_mut() {
            effect.map_tag_keys(&mut |key| {
                if *key == old {
                    *key = shared.clone();
                }
            });
        }
    }
}

pub(super) fn parse_additional_cost(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    let prose = crate::lexer::render_token_slice(full_tokens).to_ascii_lowercase();
    if let Some(rest) = prose.strip_prefix("as an additional cost to cast this spell, you may choose an opponent. if this is a permanent, when it enters, if you paid that cost, ")
        && let Some((entry, spell)) = rest.split_once(". if this is an instant or sorcery spell, if you paid that cost, ")
        && let Some(spell) = spell.trim_end_matches('.').strip_suffix(" before its other effects")
        && entry == spell {
        let effect_tokens = crate::lexer::lex_line(entry, 0)?;
        let effects = parse_effect_sentences_lexed(&effect_tokens)?;
        let cost = crate::model::CompilerOptionalCost::custom(
            format!("Chosen opponent benefit: {entry}"),
            ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::Effect(Box::new(
                EffectAst::subject_verb_choose_player(PlayerAst::You, PlayerFilter::Opponent,
                    crate::tag::CompilerReferenceTag::GiftedPlayer.bind(), false, 0),
            ))),
        );
        return Ok(ast(LineAst::GiftKeyword { cost, effects, timing: crate::cards::builders::GiftTimingAst::SourceCardType }));
    }
    // A payment and its following trigger form one definition. Bind the
    // trigger's payment count/condition to this exact optional price.
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    if sentences.len() >= 2 {
        let tail_start = tokens.iter().position(|token| std::ptr::eq(token, &sentences[1][0])).unwrap();
        let tail = crate::util::trim_edge_punctuation_tokens(&tokens[tail_start..]);
        let tail_words = crate::lexer::parser_token_word_refs(tail);
        let gated = tail_words.starts_with(&["if", "you", "do", "when"]);
        let starts_trigger = tail_words.first().is_some_and(|word| matches!(*word, "when" | "whenever" | "at"));
        if starts_trigger || gated {
            if let Some(KeywordLinePayload::Ast(parsed)) = parse_additional_cost(line, sentences[0], full_tokens)?
                && let LineAst::OptionalCost(cost) = *parsed {
                let trigger_tokens = if gated {
                    &tail[tail.iter().position(|token| token.is_word("when")).unwrap()..]
                } else { tail };
                let mut trigger = crate::clause_support::parse_triggered_line_lexed(trigger_tokens)?;
                if let LineAst::Triggered { effects, .. } = &mut trigger {
                    bind_optional_payment_copy_count(effects, &cost.reference);
                    if gated {
                        *effects = vec![EffectAst::Conditionals(crate::cards::builders::ConditionalEffectAst::Conditional {
                            predicate: crate::cards::builders::PredicateAst::ThisSpellPaidLabel(cost.reference.clone()),
                            if_true: std::mem::take(effects), if_false: Vec::new(),
                        })];
                    }
                    return Ok(ast(LineAst::Multiple(vec![LineAst::OptionalCost(cost), trigger])));
                }
            }
        }
    }
    // A payment defined in prose is an optional additional cost. Its
    // behavior does not imply the Kicker identity or the "was kicked" flag.
    let clean = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::parser_token_word_refs(clean);
    if words.starts_with(&["you", "may"])
        && matches!(words.get(2), Some(&"sacrifice" | &"discard" | &"exile" | &"return" | &"tap" | &"remove"))
        && words.ends_with(&["as", "you", "cast", "this", "spell"])
    {
        let payment = &clean[2..clean.len() - 5];
        let cost = crate::activation_and_restrictions::activated_line_core::parse_compiler_activation_cost(payment)?;
        return Ok(ast(LineAst::OptionalCost(crate::model::CompilerOptionalCost::custom("Additional", cost))));
    }
    if let Some(body) = crate::grammar::primitives::match_word_prefix(clean, &["you", "may", "pay", "an", "additional"])
        && body.len() > 5
    {
        let suffix = ["as", "you", "cast", "this", "spell"];
        let split = body.len() - suffix.len();
        if body[split..].iter().zip(suffix).all(|(token, word)| token.is_word(word)) {
            let mut cost_tokens = &body[..split];
            let repeatable = cost_tokens.len() > 4
                && cost_tokens[cost_tokens.len() - 4..].iter()
                    .zip(["any", "number", "of", "times"]).all(|(token, word)| token.is_word(word));
            if repeatable { cost_tokens = &cost_tokens[..cost_tokens.len() - 4]; }
            let mut costs = Vec::new();
            for branch in cost_tokens.split(|token| token.is_word("and/or")) {
                if branch.is_empty() {
                    return Err(CardTextError::ParseError("optional additional payment is missing a cost".into()));
                }
                let cost = crate::activation_and_restrictions::activated_line_core::parse_compiler_activation_cost(branch)?;
                let mut optional = crate::model::CompilerOptionalCost::custom("Additional", cost);
                optional.repeatable = repeatable;
                if let Some(mana) = optional.cost.mana_cost() {
                    optional.reference = crate::util::additional_mana_payment_reference(mana);
                }
                costs.push(LineAst::OptionalCost(optional));
            }
            return Ok(ast(if costs.len() == 1 { costs.remove(0) } else { LineAst::Multiple(costs) }));
        }
    }
    // "As an additional cost to cast this spell, blight X. X can't be greater
    // than the greatest toughness among creatures you control." (Soul
    // Immolation): the trailing X bound is a rule of the spell (CR 601.2b),
    // not part of the cost. Read the cost without it and add the bound.
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    if let [first, .., last] = sentences.as_slice()
        && let Some(bound) = crate::keyword_static::read_aggregate_x_maximum(last)
    {
        let cost_len = tokens
            .iter()
            .position(|token| std::ptr::eq(token, &last[0]))
            .unwrap_or(tokens.len());
        let _ = first;
        return match parse_additional_cost(line, &tokens[..cost_len], full_tokens)? {
            Some(KeywordLinePayload::Ast(cost)) => Ok(ast(LineAst::Multiple(vec![
                *cost,
                LineAst::StaticAbility(bound),
            ]))),
            _ => Ok(None),
        };
    }
    let context = rewrite_context(line, tokens, full_tokens, KeywordLineKind::AdditionalCost);
    if let Some(parsed) = parse_keyword_special_cases(&context, tokens)? {
        return Ok(ast(parsed));
    }
    if let Some(shape) = parse_behold_and_exile_additional_cost_tokens(tokens) {
        let tag = crate::tag::CompilerReferenceTag::BeheldCost0.bind();
        let effects = vec![
            EffectAst::TagAffected {
                effect: Box::new(EffectAst::subject_verb_behold(shape.subtype, 1)),
                tag: tag.clone(),
            },
            EffectAst::subject_verb_exile(TargetAst::Tagged(tag, None), false),
        ];
        return Ok(ast(LineAst::AdditionalCost { effects }));
    }
    let Some(effect_tokens) = additional_cost_tail_tokens_lexed(tokens) else {
        return Ok(None);
    };
    if let Some(price) = crate::grammar::activation_costs::parse_artifact_creature_tap_price(effect_tokens)? {
        return Ok(ast(LineAst::AdditionalCost { effects: vec![EffectAst::subject_verb_pay_mana(PlayerAst::You, price)] }));
    }
    let words = crate::lexer::parser_token_word_refs(effect_tokens);
    // "As an additional cost to cast this spell, choose a number between 0
    // and 5." (Liquid Fire): the number is announced while the spell is cast
    // (CR 601.2b), exactly an X announcement bounded by the printed range.
    // The body's "the chosen number" reads that announced X.
    if let ["choose", "a", "number", "between", low, "and", high] = words.as_slice()
        && let (Ok(low), Ok(high)) = (low.parse::<i32>(), high.parse::<i32>())
        && low <= high
    {
        let display = format!(
            "{}.",
            crate::lexer::render_token_slice(crate::util::trim_edge_punctuation_tokens(tokens))
                .trim_end_matches('.')
        );
        return Ok(ast(LineAst::Multiple(vec![
            LineAst::StaticAbility(
                crate::model::CompilerStaticAbilityCore::this_spell_x_maximum(
                    Value::Fixed(high),
                    display.clone(),
                )
                .into(),
            ),
            LineAst::StaticAbility(
                crate::model::CompilerStaticAbilityCore::this_spell_x_minimum(
                    Value::Fixed(low),
                    display,
                )
                .into(),
            ),
        ])));
    }
    if matches!(words.as_slice(), ["you", "may", "collect", "evidence", _]) {
        let collect = effect_tokens
            .iter()
            .position(|token| token.is_word("collect"))
            .unwrap();
        let cost = crate::activation_and_restrictions::activated_line_core::parse_compiler_activation_cost(&effect_tokens[collect..])?;
        let mut optional = crate::model::CompilerOptionalCost::custom("Collect evidence", cost);
        optional.reference = "Evidence".into();
        return Ok(ast(LineAst::OptionalCost(optional)));
    }
    let trimmed = crate::util::trim_edge_punctuation_tokens(effect_tokens);
    // "you may pay {1}{R} and/or {1}{G} any number of times" (Primitive
    // Justice): two independent repeatable additional payments, each counted
    // by its own "for each additional <cost> you paid".
    if trimmed.len() >= 9
        && trimmed[0].is_word("you") && trimmed[1].is_word("may") && trimmed[2].is_word("pay")
        && crate::lexer::parser_token_word_refs(&trimmed[trimmed.len() - 4..]) == ["any", "number", "of", "times"]
        && let Some(split) = trimmed[3..trimmed.len() - 4]
            .iter()
            .position(|token| token.is_word("and/or"))
            .map(|offset| offset + 3)
    {
        let mut optional_costs = Vec::new();
        for cost_tokens in [&trimmed[3..split], &trimmed[split + 1..trimmed.len() - 4]] {
            if cost_tokens.is_empty() {
                optional_costs.clear();
                break;
            }
            let cost = crate::activation_and_restrictions::activated_line_core::parse_compiler_activation_cost(cost_tokens)?;
            let Some(reference) = (!cost.has_non_mana_costs())
                .then(|| cost.mana_cost().map(crate::util::additional_mana_payment_reference))
                .flatten()
            else {
                optional_costs.clear();
                break;
            };
            let mut optional = crate::model::CompilerOptionalCost::custom("Additional", cost).repeatable();
            optional.reference = reference;
            optional_costs.push(LineAst::OptionalCost(optional));
        }
        if optional_costs.len() == 2 {
            return Ok(ast(LineAst::Multiple(optional_costs)));
        }
    }
    if trimmed.len() >= 8
        && trimmed[0].is_word("you") && trimmed[1].is_word("may")
        && crate::lexer::parser_token_word_refs(&trimmed[trimmed.len() - 4..]) == ["any", "number", "of", "times"]
    {
        let mut price = &trimmed[2..trimmed.len() - 4];
        if price.first().is_some_and(|token| token.is_word("pay"))
            && price.get(1).is_some_and(|token| matches!(token.as_word(), Some("exile" | "discard" | "sacrifice" | "return" | "tap" | "remove" | "pay"))) { price = &price[1..]; }
        let cost = crate::activation_and_restrictions::activated_line_core::parse_compiler_activation_cost(price)?;
        let reference = if !cost.has_non_mana_costs() {
            cost.mana_cost().map(crate::util::additional_mana_payment_reference)
        } else { None };
        let mut optional = crate::model::CompilerOptionalCost::custom("Additional", cost).repeatable();
        if let Some(reference) = reference { optional.reference = reference; }
        return Ok(ast(LineAst::OptionalCost(optional)));
    }
    // "you may sacrifice any number of Spirits" (Devouring Greed): sacrificing
    // zero is already a legal payment of the ordinary additional cost, whose
    // sacrificed set the body then counts ("for each Spirit sacrificed this
    // way").
    let effect_tokens = if words.starts_with(&["you", "may", "sacrifice", "any", "number", "of"])
        && words.len() > 6
        && effect_tokens.get(2).is_some_and(|token| token.is_word("sacrifice"))
    {
        &effect_tokens[2..]
    } else {
        effect_tokens
    };
    // Optional additional costs are announced and recorded as optional costs.
    // A MayEffect in the mandatory cost program cannot publish the paid label
    // queried by a later "if this spell's additional cost was paid" clause.
    if effect_tokens.get(0).is_some_and(|token| token.is_word("you"))
        && effect_tokens.get(1).is_some_and(|token| token.is_word("may"))
    {
        let cost = crate::activation_and_restrictions::activated_line_core::parse_compiler_activation_cost(&effect_tokens[2..])?;
        return Ok(ast(LineAst::OptionalCost(crate::model::CompilerOptionalCost::custom("Additional", cost))));
    }
    let evidence_minimum = match words.as_slice() {
        [
            "collect",
            "evidence",
            "x",
            "where",
            "x",
            "is",
            "the",
            "total",
            "mana",
            "value",
            "of",
            "the",
            "permanents",
            "this",
            "spell",
            "targets",
        ] => Some(Value::AnnouncedTargetTotal(
            ironsmith_core::ChoiceAggregateMetric::ManaValue,
        )),
        ["collect", "evidence", amount] => {
            crate::util::parse_number_word_u32(amount).map(|amount| Value::Fixed(amount as i32))
        }
        _ => None,
    };
    if let Some(minimum) = evidence_minimum {
        return Ok(ast(LineAst::AdditionalCost {
            effects: vec![EffectAst::subject_verb_collect_evidence(minimum)],
        }));
    }
    if is_additional_cost_choice_line_lexed(tokens)
        && parse_additional_cost_choice_options_lexed(effect_tokens)?.is_some()
    {
        return Ok(None);
    }
    let cost_segments = super::grammar::primitives::split_lexed_slices_on_and(effect_tokens);
    let has_heterogeneous_cost_heads = cost_segments.len() > 1
        && cost_segments.iter().all(|segment| {
            super::grammar::primitives::parse_prefix(
                segment,
                super::grammar::leaf::parse_leaf_activation_cost_head_lexed,
            )
            .is_some()
        });
    let effects = if has_heterogeneous_cost_heads {
        let mut effects = Vec::new();
        for segment in cost_segments {
            effects.extend(parse_effect_sentences_lexed(segment)?);
        }
        effects
    } else {
        parse_effect_sentences_lexed(effect_tokens)?
    };
    let mut effects = effects;
    if let [
        EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseOneOf {
            modes,
            ..
        }),
    ] = effects.as_mut_slice()
    {
        let mut branches = modes
            .iter_mut()
            .map(|mode| &mut mode.effects)
            .collect::<Vec<_>>();
        unify_object_choice_branch_tags(&mut branches);
    }
    Ok(ast(LineAst::AdditionalCost { effects }))
}

#[path = "keyword_payloads/alternative_cast_readings.rs"]
mod alternative_cast_readings;

pub(super) fn parse_alternative_cast(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if parse_keyword_special_form_shape_tokens(tokens) == Some(KeywordSpecialFormShape::ExertAttack)
    {
        return Ok(None);
    }
    let input = alternative_cast_readings::AlternativeCastLine {
        tokens,
        line,
        full_tokens,
        read_by_cache: Default::default(),
    };
    match alternative_cast_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }

    Ok(None)
}

macro_rules! alternative_method_parser {
    ($name:ident, $parser:ident) => {
        pub(super) fn $name(
            _line: &PreprocessedLine,
            tokens: &[OwnedLexToken],
            _full_tokens: &[OwnedLexToken],
        ) -> KeywordParseResult {
            Ok($parser(tokens)?.map(|parsed| {
                KeywordLinePayload::ast(LineAst::AlternativeCastingMethod(parsed.into()))
            }))
        }
    };
}

macro_rules! optional_cost_parser {
    ($name:ident, $parser:ident) => {
        pub(super) fn $name(
            _line: &PreprocessedLine,
            tokens: &[OwnedLexToken],
            _full_tokens: &[OwnedLexToken],
        ) -> KeywordParseResult {
            Ok($parser(tokens)?
                .map(|parsed| KeywordLinePayload::ast(LineAst::OptionalCost(parsed.into()))))
        }
    };
}

macro_rules! ability_parser {
    ($name:ident, $parser:ident) => {
        pub(super) fn $name(
            _line: &PreprocessedLine,
            tokens: &[OwnedLexToken],
            _full_tokens: &[OwnedLexToken],
        ) -> KeywordParseResult {
            Ok($parser(tokens)?.map(|parsed| KeywordLinePayload::ast(LineAst::Ability(parsed))))
        }
    };
}

alternative_method_parser!(parse_bestow, parse_bestow_line_lexed);
alternative_method_parser!(parse_escape, parse_escape_line_lexed);
alternative_method_parser!(parse_harmonize, parse_harmonize_line_lexed);
alternative_method_parser!(parse_retrace, parse_retrace_line_lexed);
alternative_method_parser!(parse_madness, parse_madness_line_lexed);
alternative_method_parser!(parse_warp, parse_warp_line_lexed);

pub(super) fn parse_beam_me_up(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    use winnow::prelude::*;
    use crate::grammar::{leaf, primitives};
    let Some(cost) = primitives::probe_all(tokens, (
        primitives::phrase(&["beam", "me", "up"]),
        leaf::parse_leaf_mana_cost_prefix_lexed,
        primitives::sentence_end(),
    ).map(|(_, mana, _)| mana.cost), "beam-me-up") else {
        return Ok(None);
    };
    let mut returned = ObjectFilter::creature().controlled_by(PlayerFilter::You);
    returned.excluded_rules.push(ironsmith_core::ObjectRule::CantBeBeamedUp);
    let total_cost = ironsmith_core::TotalCost::from_costs(vec![
        crate::model::CompilerCost::Mana(cost),
        crate::model::CompilerCost::ReturnChosenToHand {
            count: 1,
            filter: returned,
        },
    ]);
    Ok(ast(LineAst::AlternativeCastingMethod(
        crate::model::CompilerAlternativeCastingMethod::BeamMeUp { total_cost },
    )))
}

pub(super) fn parse_flashback(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    // A modifier of flashback costs is a static rule, not a flashback cost.
    if token_slice_at_is(tokens, 1, "costs") {
        return Ok(None);
    }
    // Semantic preprocessing may present the keyword registry with only the
    // leading keyword sentence while retaining the complete source line in
    // `full_tokens`.  The Commander 2021 "Visions of" cycle uses the second
    // sentence to qualify its flashback cost reduction, so parsing only the
    // selected sentence silently loses the alternative casting method.
    let selected_sentences = split_lexed_sentences(tokens);
    let full_sentences = split_lexed_sentences(full_tokens);
    let sentences = if selected_sentences.len() >= 2 {
        selected_sentences
    } else if token_slice_first_is(full_tokens, "flashback") && full_sentences.len() >= 2 {
        full_sentences[..2].to_vec()
    } else {
        selected_sentences
    };
    let complete_tokens = if token_slice_first_is(full_tokens, "flashback") {
        full_tokens
    } else { tokens };
    if crate::util::split_flashback_x_cant_be_zero(complete_tokens).1 > 0 {
        return Ok(parse_flashback_line_lexed(complete_tokens)?
            .map(|method| KeywordLinePayload::ast(LineAst::AlternativeCastingMethod(method))));
    }
    let Some(flashback_tokens) = sentences.first().copied() else {
        return Ok(None);
    };
    let Some(method) = parse_flashback_line_lexed(flashback_tokens)? else {
        return Ok(None);
    };

    if sentences.len() == 1 {
        return Ok(ast(LineAst::AlternativeCastingMethod(method)));
    }
    if sentences.len() != 2 {
        return Ok(None);
    }

    let reduction_tokens = sentences[1];
    let reduction_words = TokenWordView::new(reduction_tokens).word_refs();
    if !crate::word_primitives::parse_sequence_prefix(&reduction_words, &["this", "spell", "costs"])
        || !crate::word_primitives::sequence_occurs(
            &reduction_words,
            &["to", "cast", "this", "way"],
        )
    {
        return Ok(None);
    }

    let Some(mut abilities) = parse_static_ability_ast_line_lexed(reduction_tokens)? else {
        return Ok(None);
    };
    if abilities.len() != 1 {
        return Ok(None);
    }
    let StaticAbilityAst::Static(mut ability) = abilities.pop().expect("checked one ability")
    else {
        return Ok(None);
    };
    let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction) =
        &mut ability.payload
    else {
        return Ok(None);
    };
    reduction.alternative_cast = Some(crate::filter::AlternativeCastKind::Flashback);

    Ok(ast(LineAst::Multiple(vec![
        LineAst::AlternativeCastingMethod(method),
        LineAst::StaticAbility(StaticAbilityAst::Static(ability)),
    ])))
}

optional_cost_parser!(parse_bargain, parse_bargain_line_lexed);
optional_cost_parser!(parse_buyback, parse_buyback_line_lexed);
optional_cost_parser!(parse_multikicker, parse_multikicker_line_lexed);
optional_cost_parser!(parse_replicate, parse_replicate_line_lexed);
optional_cost_parser!(parse_offspring, parse_offspring_line_lexed);
optional_cost_parser!(parse_entwine, parse_entwine_line_lexed);

ability_parser!(parse_channel, parse_channel_line_lexed);
ability_parser!(parse_cycling, parse_cycling_line_lexed);
ability_parser!(parse_craft, parse_craft_line_lexed);
ability_parser!(parse_reinforce, parse_reinforce_line_lexed);
ability_parser!(parse_equip, parse_equip_line_lexed);
ability_parser!(parse_fortify, parse_fortify_line_lexed);
pub(super) fn parse_reconfigure(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    Ok(parse_reconfigure_line_lexed(tokens)?.map(|abilities| {
        KeywordLinePayload::ast(LineAst::Multiple(
            abilities.into_iter().map(LineAst::Ability).collect(),
        ))
    }))
}
ability_parser!(parse_morph, parse_morph_keyword_line_lexed);
ability_parser!(parse_transmute, parse_transmute_line_lexed);
ability_parser!(parse_transfigure, parse_transfigure_line_lexed);

pub(super) fn parse_blitz(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if token_slice_at_is(tokens, 1, "costs") {
        return Ok(None);
    }
    Ok(parse_blitz_line_lexed(tokens)?
        .map(|parsed| KeywordLinePayload::ast(LineAst::AlternativeCastingMethod(parsed))))
}

pub(super) fn parse_kicker(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    Ok(parse_kicker_line_lexed(tokens)?.map(|parsed| KeywordLinePayload::kicker(parsed.cost)))
}

pub(super) fn parse_mutate(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if !token_slice_first_is(tokens, "mutate") {
        return Ok(None);
    }
    let Some((cost, _)) = leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default()) else {
        return Ok(None);
    };
    Ok(ast(LineAst::AlternativeCastingMethod(
        crate::model::CompilerAlternativeCastingMethod::Mutate { cost },
    )))
}

/// "Teamwork N (As an additional cost to cast this spell, you may tap any
/// number of creatures you control with total power N or more.)" — an
/// optional crew-style additional cost (We Say Thee Nay!).
pub(super) fn parse_teamwork(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    let words = crate::lexer::token_word_refs(tokens);
    let ["teamwork", amount] = words.as_slice() else {
        return Ok(None);
    };
    let Some(amount) = crate::util::parse_number_word_u32(amount) else {
        return Ok(None);
    };
    let cost = ironsmith_core::TotalCost::from_costs(vec![crate::model::CompilerCost::Teamwork {
        amount,
    }]);
    Ok(ast(LineAst::OptionalCost(
        crate::model::CompilerOptionalCost::teamwork(cost),
    )))
}

pub(super) fn parse_squad(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if let Some(cost) = parse_squad_line_lexed(tokens)? {
        return Ok(ast(LineAst::OptionalCost(cost)));
    }
    if let Some(effect_tokens) = optional_cost_tail_effect_tokens(tokens)
        && let Ok(effects) = parse_effect_sentences_lexed(effect_tokens)
        && !effects.is_empty()
    {
        return Ok(ast(LineAst::Statement { effects }));
    }
    Ok(None)
}

pub(super) fn parse_splice(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    let Some(parsed) = parse_splice_keyword_line_tokens(tokens)? else {
        return Ok(None);
    };
    let is_edge_punctuation = |token: &crate::lexer::OwnedLexToken| {
        matches!(
            token.kind,
            crate::lexer::TokenKind::Dash
                | crate::lexer::TokenKind::EmDash
                | crate::lexer::TokenKind::Period
        )
    };
    let start = crate::slice_primitives::select_position(parsed.cost_tokens, |token| {
        !is_edge_punctuation(token)
    })
    .unwrap_or(parsed.cost_tokens.len());
    let end = crate::slice_primitives::select_last_position(parsed.cost_tokens, |token| {
        !is_edge_punctuation(token)
    })
    .map_or(start, |index| index + 1);
    let cost_tokens = &parsed.cost_tokens[start..end];
    let cost_surface = cost_tokens
        .first()
        .zip(cost_tokens.last())
        .and_then(|(first, last)| line.info.raw_line.get(first.span.start..last.span.end))
        .map(str::trim)
        .filter(|surface| !surface.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            crate::lexer::render_token_slice(cost_tokens)
                .trim()
                .to_string()
        });
    let cost = crate::activation_and_restrictions::keyword_action_costs::parse_payment_clause_as_total_cost(
        cost_tokens,
    )?
        .ok_or_else(|| {
            crate::cards::builders::CardTextError::ParseError(format!(
                "unsupported splice cost clause: {}",
                crate::lexer::render_token_slice(cost_tokens).trim()
            ))
        })?;
    let quality = match parsed.subject {
        super::grammar::splice_keyword_lines::SpliceSubject::Arcane => {
            crate::static_abilities::SpliceQuality::Arcane
        }
        super::grammar::splice_keyword_lines::SpliceSubject::InstantOrSorcery => {
            crate::static_abilities::SpliceQuality::InstantOrSorcery
        }
    };
    Ok(ast(LineAst::StaticAbility(
        crate::model::CompilerStaticAbilityCore::splice_with_cost_surface(
            quality,
            cost,
            Some(cost_surface),
        )
        .into(),
    )))
}

#[cfg(test)]
#[path = "keyword_payloads_inline_tests.rs"]
mod tests;

#[path = "keyword_payloads/core.rs"]
mod core_programs;
pub(super) use core_programs::{
    parse_epic, parse_escalate, parse_eternalize, parse_evoke, parse_exploit, parse_paradigm,
};
#[path = "keyword_payloads/combat.rs"]
mod combat_programs;
pub(super) use combat_programs::parse_exert_attack;
#[path = "keyword_payloads/condition.rs"]
mod condition_programs;
pub(super) use condition_programs::parse_gift;
#[path = "keyword_payloads/permission.rs"]
mod permission_programs;
pub(super) use permission_programs::parse_cast_this_spell_only;

/// "That many" in a following copy instruction counts this declared payment,
/// not the number of objects discarded/exiled to pay it.
fn bind_optional_payment_copy_count(effects: &mut [EffectAst], reference: &ironsmith_core::OptionalCostRef) {
    fn bind(value: &mut Value, reference: &ironsmith_core::OptionalCostRef) {
        match value {
            Value::SurfaceHinted { value, .. } => bind(value, reference),
            Value::EventValue(crate::effect::EventValueSpec::Amount) => *value = Value::TimesPaidLabel(reference.clone()),
            Value::TimesPaidLabel(label) if *label == ironsmith_core::OptionalCostRef::from("Additional") => *label = reference.clone(),
            _ => {},
        }
    }
    for effect in effects {
        if let EffectAst::SubjectVerb(subject) = effect {
            match &mut subject.action {
                crate::cards::builders::SubjectVerbActionAst::Stack(crate::cards::builders::StackActionAst::CopySpell { count, .. }) => bind(count, reference),
                crate::cards::builders::SubjectVerbActionAst::Tokens(crate::cards::builders::TokenActionAst::CreateTokenCopy { count, .. } | crate::cards::builders::TokenActionAst::CreateTokenCopyFromSource { count, .. }) => bind(count, reference),
                _ => {},
            }
        }
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| bind_optional_payment_copy_count(nested, reference));
    }
}
