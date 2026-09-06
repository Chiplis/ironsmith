#[cfg(test)]
use ironsmith_compiler::ParseCardText;
use ironsmith_core::card::CardBuilder;

use crate::PtValue;
use crate::ability::{ActivationTiming, PresentationLabel};
use crate::cards::builders::{
    CardTextError, LineAst, ParseAnnotations, ParsedLevelAbilityItemAst,
    ParsedLevelActivatedAbilityAst, PredicateAst, TextSpan, SourcePredicateAst,
};
use crate::parse_context::{ParseContext, ParseContextView, ParseScopeKind};
use crate::parse_trace;
use winnow::Parser;
use winnow::error::ModalResult as WResult;
use winnow::stream::Stream;
use winnow::token::any;

use super::activation_and_restrictions::parse_payment_clause_as_total_cost;
use super::clause_support::{
    parse_ability_line_lexed, parse_effect_sentences_lexed, parse_static_ability_ast_line_lexed,
    parse_trigger_clause_lexed, parse_triggered_line_lexed,
};
use super::grammar::abilities::{
    is_activate_only_once_each_turn_line_lexed, is_doesnt_untap_during_your_untap_step_line_lexed,
    is_land_reveal_enters_static_line_lexed, is_land_reveal_enters_tapped_followup_line_lexed,
    is_opening_hand_begin_game_static_line_lexed, is_ward_or_echo_static_prefix_line_lexed,
    split_nested_combat_whenever_clause_lexed,
};
use super::grammar::activation_costs::parse_activation_cost_tokens as parse_activation_cost_tokens_rewrite;
use super::grammar::document_facts as document_fact_grammar;
use super::grammar::document_shapes as document_grammar;
use super::grammar::effects as effect_grammar;
use super::grammar::line_families as line_family_grammar;
use super::grammar::preprocess as preprocess_grammar;
use super::grammar::primitives as grammar;
use super::grammar::semantic_lowering as semantic_grammar;
use super::grammar::structure::split_lexed_sentences;
use super::ir::{
    ChosenOptionContext, OverloadRewritePayload, RewriteSemanticDocument, RewriteSemanticItem,
};
use super::keyword_registry::{recognize_keyword_line, rewrite_keyword_dash_parse_tokens};
use super::keyword_static::{
    parse_if_this_spell_costs_less_to_cast_line_lexed,
    parse_spell_additional_life_cost_per_target_line,
    parse_spell_and_player_activated_ability_cost_modifier_line,
    parse_spell_cost_increase_per_target_beyond_first_line, parse_spells_cost_modifier_line,
};
use super::lexer::{
    LexStream, OwnedLexToken, TokenKind, TokenWordPiece, TokenWordView, lex_line,
    locate_token_word, render_token_slice, token_word_refs, trim_lexed_commas,
};
#[cfg(test)]
use super::preprocess::preprocess_document;
use super::preprocess::{
    PreprocessedDocument, PreprocessedItem, PreprocessedLine, preprocess_document_with_provenance,
    strip_parenthetical_segments,
};
#[cfg(test)]
use super::recognized_document::KeywordLineKind;
use super::recognized_document::{
    LevelItemKind, RecognizedActivatedLine, RecognizedDocument, RecognizedLevelHeader,
    RecognizedLevelItem, RecognizedLine, RecognizedMetadataLine, RecognizedModalBlock,
    RecognizedModalMode, RecognizedSagaChapterLine, RecognizedStatementLine, RecognizedStaticLine,
    RecognizedTriggerIntro, RecognizedTriggeredLine, RecognizedUnsupportedLine,
};
use super::semantic_assembly::assemble_non_metadata_line;
use super::token_primitives::{
    clone_sentence_chunk_tokens, lexed_head_words, locate_index as locate_token_index,
    strip_leading_if_you_do_lexed,
};
use super::util::{
    map_span_to_original, parse_level_header_tokens, parse_level_up_line_lexed,
    parse_power_toughness, parse_saga_chapter_prefix_tokens, parse_subtype_flexible, parser_trace,
    parser_trace_enabled, span_from_tokens,
};
const TOKEN_NAME_SUFFIX_WORD: &str = "twin";
const LESS_THAN_ONE_MANA_REDUCTION_REMINDER: &str =
    "this effect can't reduce the mana in that cost to less than one mana.";

mod block_parsing;
mod line_dispatch;
mod line_family_handlers;
mod line_recognition;
mod statement_recognition;
mod unsupported;

use block_parsing::{try_parse_level_header_block, try_parse_modal_bullet_block};
use line_dispatch::{LineDispatchResult, dispatch_standard_line};
use line_recognition::{
    recognize_level_item, recognize_modal_mode, recognize_saga_chapter_line, recognize_static_line,
    recognize_triggered_line, strict_unsupported_triggered_line_error,
};
use statement_recognition::{
    extend_activated_line_with_result_followups, extend_statement_line_with_result_followups,
    extend_statement_line_with_result_followups_in_place,
    extend_triggered_line_with_result_followups, has_effect_prefix_before_trailing_static_sentence,
    looks_like_statement_line_lexed, parse_colon_nonactivation_statement_fallback,
    recognize_source_gain_ability_statement_boxed, recognize_statement_line,
};
#[cfg(test)]
use statement_recognition::{looks_like_statement_line, normalize_statement_parse_groups_lexed};
use unsupported::diagnose_known_unsupported_rewrite_line;

fn recognized_line_kind(line: &RecognizedLine) -> &'static str {
    match line {
        RecognizedLine::Metadata(_) => "metadata",
        RecognizedLine::Keyword(_) => "keyword",
        RecognizedLine::Static(_) => "static",
        RecognizedLine::Activated(_) => "activated",
        RecognizedLine::Triggered(_) => "triggered",
        RecognizedLine::Statement(_) => "statement",
        RecognizedLine::LevelHeader(_) => "level-header",
        RecognizedLine::SagaChapter(_) => "saga-chapter",
        RecognizedLine::Modal(_) => "modal",
        RecognizedLine::Unsupported(_) => "unsupported",
    }
}

fn trace_recognized_line(line: &RecognizedLine) {
    parse_trace::event(format!("classified as {}", recognized_line_kind(line)));
}

/// Join an adjacent prior-token count replacement to the statement that
/// creates its token blueprint. The followup shape proves the authored
/// `those tokens` reference, while the joint sentence parser must prove that
/// the pair becomes one typed self-replacement before either recognized form is changed.
fn try_merge_labeled_prior_token_replacement_statement(
    lines: &mut [RecognizedLine],
    dispatch: &LineDispatchResult,
) -> bool {
    let [RecognizedLine::Statement(followup)] = dispatch.lines.as_slice() else {
        return false;
    };
    if effect_grammar::followup_shapes::parse_create_more_prior_tokens(&followup.parse_tokens)
        .is_none()
    {
        return false;
    }
    let Some(RecognizedLine::Statement(previous)) = lines.last_mut() else {
        return false;
    };

    let mut sentences = split_lexed_sentences(&previous.parse_tokens)
        .into_iter()
        .map(<[OwnedLexToken]>::to_vec)
        .collect::<Vec<_>>();
    sentences.extend(
        split_lexed_sentences(&followup.parse_tokens)
            .into_iter()
            .map(<[OwnedLexToken]>::to_vec),
    );
    let combined = crate::util::join_sentences_with_period(&sentences);
    let Ok(effects) = parse_effect_sentences_lexed(&combined) else {
        return false;
    };
    if !matches!(
        effects.as_slice(),
        [crate::cards::builders::EffectAst::SelfReplacement { .. }]
    ) {
        return false;
    }

    previous.parse_tokens = combined.clone();
    previous.parse_groups = vec![combined];
    // The preceding statement may already carry a typed standalone-create
    // result. Once the joint grammar proves a self-replacement, replace that
    // stale partial result with the complete typed program so semantic
    // assembly does not bypass the merged token group.
    previous.parsed_effects = Some(effects);
    previous.text = format!("{} {}", previous.text.trim(), followup.text.trim());
    previous.info.raw_line = format!(
        "{}\n{}",
        previous.info.raw_line.trim(),
        followup.info.raw_line.trim()
    );
    previous
        .info
        .source_tokens
        .extend(followup.info.source_tokens.iter().cloned());
    let prior_token_facts = &followup.info.semantic_facts.statement;
    previous.info.semantic_facts.statement.instead_followup = prior_token_facts.instead_followup;
    previous
        .info
        .semantic_facts
        .statement
        .trailing_instead_if_predicate = prior_token_facts.trailing_instead_if_predicate.clone();
    previous.info.semantic_facts.statement.presentation_label =
        prior_token_facts.presentation_label.clone();
    true
}

fn is_bullet_line(line: &PreprocessedLine) -> bool {
    let Some(first) = line.tokens.first() else {
        return false;
    };
    if first.kind == TokenKind::Bullet {
        return true;
    }
    if starts_with_pawprint_modal_label(&line.tokens) {
        return true;
    }
    let dash_starts_loyalty_or_numeric_prefix = first.kind == TokenKind::Dash
        && line.tokens.get(1).is_some_and(|token| {
            let text = token.parser_text();
            token.kind == TokenKind::Number
                || text.eq_ignore_ascii_case("x")
                || text.parse::<u32>().is_ok()
        });
    if dash_starts_loyalty_or_numeric_prefix {
        return false;
    }
    first.kind == TokenKind::Dash
        && !line
            .tokens
            .get(1)
            .is_some_and(|token| token.kind == TokenKind::Number)
}

fn strip_choice_bullet_prefix_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if tokens
        .first()
        .is_some_and(|token| matches!(token.kind, TokenKind::Bullet | TokenKind::Dash))
    {
        tokens.get(1..).unwrap_or_default()
    } else {
        tokens
    }
}

fn is_named_option_as_enters_choice_header(line: &PreprocessedLine) -> bool {
    document_grammar::parse_named_option_choice_header(&line.tokens).is_some()
}

fn starts_with_pawprint_modal_label(tokens: &[OwnedLexToken]) -> bool {
    super::grammar::modal::parse_modal_point_label_tokens(tokens).is_some()
}

fn parse_trigger_intro_tokens(tokens: &[OwnedLexToken]) -> Option<RecognizedTriggerIntro> {
    if let Some((intro, _)) = grammar::parse_prefix(
        tokens,
        winnow::combinator::alt((
            grammar::kw("when").value(RecognizedTriggerIntro::When),
            grammar::kw("whenever").value(RecognizedTriggerIntro::Whenever),
        )),
    ) {
        return Some(intro);
    }

    super::parser_support::is_at_trigger_intro_lexed(tokens, 0)
        .then_some(RecognizedTriggerIntro::At)
}

fn strip_trigger_frequency_suffix_tokens(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<u32>) {
    if let Some((_, rest)) = grammar::strip_lexed_suffix_phrases(
        tokens,
        &[
            &["for", "the", "first", "time", "each", "turn"][..],
            &["for", "the", "first", "time", "this", "turn"][..],
            &[
                "for", "the", "first", "time", "during", "each", "of", "your", "turns",
            ][..],
        ],
    ) {
        return (rest, Some(1));
    }

    (tokens, None)
}

fn strip_trailing_trigger_cap_suffix_tokens(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<u32>) {
    let Some(shape) = document_grammar::parse_trailing_trigger_cap_suffix_tokens(tokens) else {
        return (tokens, None);
    };
    let count = match shape.cap {
        document_grammar::TriggerCapSurface::Once => 1,
        document_grammar::TriggerCapSurface::Twice => 2,
    };
    (shape.head_tokens, Some(count))
}

fn line_starts_with_trigger_intro_tokens(tokens: &[OwnedLexToken]) -> bool {
    if super::parser_support::looks_like_reflexive_followup_intro_lexed(tokens) {
        return false;
    }
    parse_trigger_intro_tokens(tokens).is_some()
}

fn labeled_body_starts_with_trigger_intro_tokens(tokens: &[OwnedLexToken]) -> bool {
    split_label_prefix_lexed(tokens)
        .is_some_and(|(_, _, body_tokens)| line_starts_with_trigger_intro_tokens(body_tokens))
}

fn is_if_you_do_exile_followup_tokens(tokens: &[OwnedLexToken]) -> bool {
    grammar::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        (
            grammar::phrase(&["if", "you", "do"]),
            winnow::combinator::opt(grammar::comma()),
            grammar::kw("exile"),
        )
            .void()
            .parse_next(input)
    })
    .is_some()
}

fn should_try_combined_static_tokens(
    line_tokens: &[OwnedLexToken],
    next_line_tokens: &[OwnedLexToken],
) -> bool {
    ((is_land_reveal_enters_static_line_lexed(line_tokens)
        || is_pay_life_enters_static_line_lexed(line_tokens))
        && is_land_reveal_enters_tapped_followup_line_lexed(next_line_tokens))
        || (is_opening_hand_begin_game_static_line_lexed(line_tokens)
            && is_if_you_do_exile_followup_tokens(next_line_tokens))
}

fn is_pay_life_enters_static_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    grammar::parse_prefix(tokens, grammar::phrase(&["as", "this"])).is_some()
        && tokens.iter().any(|token| token.is_word("enter"))
        && tokens.iter().any(|token| token.is_word("may"))
        && tokens.iter().any(|token| token.is_word("pay"))
        && tokens.iter().any(|token| token.is_word("life"))
}

#[derive(Debug, Clone)]
struct TriggeredSplitCandidate {
    trigger_parse_tokens: Vec<OwnedLexToken>,
    effect_parse_tokens: Vec<OwnedLexToken>,
    intervening_if: Option<PredicateAst>,
    max_triggers_per_turn: Option<u32>,
}

impl TriggeredSplitCandidate {
    fn into_recognized_line(
        self,
        line: &PreprocessedLine,
        full_parse_tokens: &[OwnedLexToken],
    ) -> RecognizedTriggeredLine {
        let mut info = line.info.clone();
        if let Some(intro_surface) =
            super::grammar::trigger_surface::parse_trigger_intro_surface_tokens(full_parse_tokens)
        {
            info.semantic_facts.triggered_ability.intro_surface = Some(intro_surface);
        }
        RecognizedTriggeredLine {
            info,
            full_text: render_token_slice(full_parse_tokens).trim().to_string(),
            full_parse_tokens: full_parse_tokens.to_vec(),
            trigger_parse_tokens: self.trigger_parse_tokens,
            effect_parse_tokens: self.effect_parse_tokens,
            intervening_if: self.intervening_if,
            presentation: trigger_presentation_from_preprocessed_line(line),
            max_triggers_per_turn: self.max_triggers_per_turn,
            chosen_option: None,
        }
    }
}

#[derive(Debug, Clone)]
enum TriggeredSplitProbe {
    Empty,
    Supported(TriggeredSplitCandidate),
    Unsupported {
        candidate: TriggeredSplitCandidate,
        trigger_error: Option<CardTextError>,
        effect_error: Option<CardTextError>,
    },
}

impl TriggeredSplitProbe {
    fn supported_recognized(
        &self,
        line: &PreprocessedLine,
        full_parse_tokens: &[OwnedLexToken],
    ) -> Option<RecognizedTriggeredLine> {
        match self {
            Self::Supported(candidate) => Some(
                candidate
                    .clone()
                    .into_recognized_line(line, full_parse_tokens),
            ),
            _ => None,
        }
    }

    fn fallback_recognized(
        &self,
        line: &PreprocessedLine,
        full_parse_tokens: &[OwnedLexToken],
    ) -> Option<RecognizedTriggeredLine> {
        match self {
            Self::Supported(candidate) => Some(
                candidate
                    .clone()
                    .into_recognized_line(line, full_parse_tokens),
            ),
            Self::Unsupported { candidate, .. } => Some(
                candidate
                    .clone()
                    .into_recognized_line(line, full_parse_tokens),
            ),
            Self::Empty => None,
        }
    }

    fn preferred_error(&self) -> Option<CardTextError> {
        match self {
            Self::Unsupported {
                trigger_error,
                effect_error,
                ..
            } => effect_error.clone().or_else(|| trigger_error.clone()),
            Self::Empty | Self::Supported(_) => None,
        }
    }
}

fn render_triggered_split_candidate(
    trigger_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
    intervening_if: Option<PredicateAst>,
    trailing_cap: Option<u32>,
) -> Option<TriggeredSplitCandidate> {
    let trigger_candidate_tokens = trim_lexed_commas(trigger_tokens);
    let effect_candidate_tokens = trim_lexed_commas(effect_tokens);
    if trigger_candidate_tokens.is_empty() || effect_candidate_tokens.is_empty() {
        return None;
    }

    let (trigger_tokens, max_triggers_per_turn) =
        strip_trigger_frequency_suffix_tokens(trigger_candidate_tokens);
    if render_token_slice(trigger_tokens).trim().is_empty()
        || render_token_slice(effect_candidate_tokens)
            .trim()
            .is_empty()
    {
        return None;
    }

    Some(TriggeredSplitCandidate {
        trigger_parse_tokens: trigger_tokens.to_vec(),
        effect_parse_tokens: effect_candidate_tokens.to_vec(),
        intervening_if,
        max_triggers_per_turn: max_triggers_per_turn.or(trailing_cap),
    })
}

fn probe_triggered_split(
    trigger_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
    intervening_if: Option<PredicateAst>,
    trailing_cap: Option<u32>,
) -> TriggeredSplitProbe {
    let trigger_candidate_tokens = trim_lexed_commas(trigger_tokens);
    let effect_candidate_tokens = trim_lexed_commas(effect_tokens);
    let Some(candidate) = render_triggered_split_candidate(
        trigger_tokens,
        effect_tokens,
        intervening_if,
        trailing_cap,
    ) else {
        return TriggeredSplitProbe::Empty;
    };
    let (trigger_tokens, _) = strip_trigger_frequency_suffix_tokens(trigger_candidate_tokens);

    // Split candidates are speculative. A rejected comma boundary must not
    // leak lossy-recovery diagnostics into the ultimately committed trigger.
    let trigger_error = crate::parse_loss::capture(|| parse_trigger_clause_lexed(trigger_tokens))
        .0
        .err();
    let effect_error =
        crate::parse_loss::capture(|| parse_effect_sentences_lexed(effect_candidate_tokens))
            .0
            .err();
    if trigger_error.is_none()
        && (effect_error.is_none()
            || triggered_effect_tokens_have_trailing_static_sentences(effect_candidate_tokens))
    {
        TriggeredSplitProbe::Supported(candidate)
    } else {
        TriggeredSplitProbe::Unsupported {
            candidate,
            trigger_error,
            effect_error,
        }
    }
}

fn triggered_effect_tokens_have_trailing_static_sentences(tokens: &[OwnedLexToken]) -> bool {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() < 2 {
        return false;
    }

    if semantic_grammar::parse_returned_object_move_head_tokens(sentences[0]).is_some()
        && sentences.iter().skip(1).all(|sentence| {
            semantic_grammar::parse_returned_object_followup_tokens(sentence)
                .is_some_and(|facts| facts.has_characteristic_changes())
        })
    {
        // These static-looking sentences modify the permanent returned by
        // the first resolution instruction. Keep the complete typed program
        // together so reference resolution can bind every `it` to that
        // returned object rather than promoting the followups to source-wide
        // static abilities.
        return false;
    }

    let Some(first_static_idx) =
        sentences
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(idx, sentence)| {
                sentence_is_static_after_trigger_effect(sentence).then_some(idx)
            })
    else {
        return false;
    };

    if !sentences[first_static_idx..]
        .iter()
        .all(|sentence| sentence_is_static_after_trigger_effect(sentence))
    {
        return false;
    }

    sentences[..first_static_idx].iter().all(|sentence| {
        crate::parse_loss::capture(|| parse_effect_sentences_lexed(sentence))
            .0
            .is_ok()
    })
}

fn sentence_is_static_after_trigger_effect(tokens: &[OwnedLexToken]) -> bool {
    if effect_grammar::followup_shapes::parse_moved_object_entry_followup_shape(tokens).is_some() {
        // This sentence describes the object moved by the preceding optional
        // effect. Keep it in the trigger's resolution program so the typed
        // follow-up parser can attach its entry state and temporary grant to
        // that move; the static parser also accepting "It enters ..." must
        // not peel it off as a source ability.
        return false;
    }
    let create_tokens = strip_leading_if_you_do_lexed(tokens);
    if effect_grammar::parse_create_head_tokens(create_tokens).is_some() {
        // A conditional token creation can contain quoted static-looking
        // rules text. It remains part of the trigger's resolution program;
        // the quoted rule is not a new static ability on the source card.
        return false;
    }
    if sentence_has_typed_become_copy_exception(tokens) {
        // Copy exceptions such as "except it has flying" describe the
        // resolving copy effect. They must not be peeled off as a source
        // static ability merely because the exception itself looks static.
        return false;
    }
    if crate::parse_loss::capture(|| parse_effect_sentences_lexed(tokens))
        .0
        .is_ok_and(|effects| {
            matches!(
                effects.as_slice(),
                [crate::model::ast::EffectAst::Conditionals(crate::cards::builders::ConditionalEffectAst::Conditional { predicate, .. })]
                    if predicate.uses_implicit_object_reference()
            )
        })
    {
        // A sentence such as "If it isn't a creature, it becomes ..." can
        // also resemble a source-wide continuous rule.  Here the implicit
        // object reference proves that it consumes the object chosen by the
        // preceding resolving effect, so it belongs to that trigger's
        // resolution program rather than becoming a separate static ability.
        return false;
    }
    semantic_grammar::parse_self_counter_entry_tokens(tokens).is_some()
        || matches!(
            crate::parse_loss::capture(|| parse_static_ability_ast_line_lexed(tokens)).0,
            Ok(Some(_))
        )
}

fn sentence_has_typed_become_copy_exception(tokens: &[OwnedLexToken]) -> bool {
    let Some(become_idx) = crate::slice_primitives::select_position(tokens, |token| {
        token.is_word("become") || token.is_word("becomes")
    }) else {
        return false;
    };
    let shape = effect_grammar::become_shapes::parse_become_rest_shape(&tokens[become_idx..]);
    shape.copy_exception.is_some()
        && crate::word_primitives::sequence_occurs(
            &TokenWordView::new(&shape.body_tokens).word_refs(),
            &["copy", "of"],
        )
}

fn strip_non_keyword_label_prefix_lexed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    document_grammar::parse_statement_label_strip_tokens(tokens).body_tokens
}

fn tokens_after_non_keyword_label_prefix(line: &PreprocessedLine) -> Option<&[OwnedLexToken]> {
    let stripped = strip_non_keyword_label_prefix_lexed(&line.tokens);
    (stripped.len() != line.tokens.len()).then_some(stripped)
}

fn should_skip_keyword_action_static_probe(tokens: &[OwnedLexToken]) -> bool {
    document_grammar::parse_blocked_keyword_action_surface(tokens).is_some()
}

fn should_prefer_statement_before_static_for_nonpermanent_spell(
    preprocessed: &PreprocessedDocument,
    tokens: &[OwnedLexToken],
) -> bool {
    let builder_has_nonpermanent_spell_type =
        preprocessed.card.card_types_ref().iter().any(|card_type| {
            matches!(
                card_type,
                crate::types::CardType::Instant | crate::types::CardType::Sorcery
            )
        });
    let metadata_has_nonpermanent_spell_type = preprocessed.items.iter().any(|item| {
        matches!(
            item,
            PreprocessedItem::Metadata(metadata)
                if matches!(
                    metadata.value,
                    crate::cards::builders::MetadataLine::TypeLine(ref raw)
                        if raw
                            .split(|ch: char| !ch.is_alphabetic())
                            .any(|part| matches!(part, "Instant" | "Sorcery"))
                )
        )
    });
    let is_nonpermanent_spell =
        builder_has_nonpermanent_spell_type || metadata_has_nonpermanent_spell_type;
    if crate::grammar::abilities::is_shuffle_into_library_from_graveyard_line_lexed(tokens) {
        return false;
    }
    let is_effect_redirect =
        crate::grammar::effects::clause_pattern_shapes::parse_redirect_next_damage_tokens(tokens)
            .is_some();
    let has_statement_family =
        crate::grammar::structure::classify_statement_line_family_lexed(tokens).is_some();
    let starts_with_clash_action = tokens.first().is_some_and(|token| token.is_word("clash"));
    is_nonpermanent_spell
        && (document_grammar::parse_nonpermanent_statement_surface(tokens).is_some()
            || is_effect_redirect
            || has_statement_family
            || starts_with_clash_action)
}

fn looks_like_leading_conditional_self_replacement(tokens: &[OwnedLexToken]) -> bool {
    document_grammar::parse_conditional_replacement_surface(tokens).is_some()
}

fn parse_labeled_conditional_replacement_sentence_split(
    line: &PreprocessedLine,
    idx: usize,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    // Some replacement abilities are one semantic static line even though
    // their Oracle surface contains multiple sentences (for example Eruth
    // and Sages of the Anima). Give the typed static grammar the complete
    // token stream before considering a sentence-by-sentence split.
    if matches!(
        line_family_grammar::parse_statement_static_preference(&line.tokens),
        Some(
            line_family_grammar::StatementStaticPreference::DrawReplacement
                | line_family_grammar::StatementStaticPreference::DiscardOrRedirectReplacement
        )
    ) && let Ok(Some(static_line)) = recognize_static_line(line)
    {
        return Ok(Some(LineDispatchResult::single(
            RecognizedLine::Static(static_line),
            idx + 1,
        )));
    }

    let prevention_then_trigger =
        line_family_grammar::parse_remove_counter_prevention_then_trigger(&line.tokens);
    let parsed_prevention = prevention_then_trigger
        .as_ref()
        .map(|shape| {
            super::keyword_static::lower_remove_counter_prevention_spec(shape.prevention)
                .map(LineAst::StaticAbility)
        })
        .transpose()?;
    let sentences = split_lexed_sentences(&line.tokens);
    if sentences.len() < 2 {
        return Ok(None);
    }
    // This is one linked spell instruction: the delayed return consumes the
    // objects exported by the future replacement. Splitting it here produces
    // two independent statement programs before sequence lowering can assign
    // the shared replacement tag.
    if let [replacement, delayed_return] = sentences.as_slice()
        && effect_grammar::is_filtered_future_exile_return_next_end_step_shape(
            replacement,
            delayed_return,
        )
    {
        return Ok(None);
    }
    if sentences.first().is_some_and(|sentence| {
        super::grammar::effects::followup_shapes::is_skip_tapped_source_turn_replacement(sentence)
    }) && sentences.get(1).is_some_and(|sentence| {
        super::grammar::effects::followup_shapes::is_if_did_untap_source_followup(sentence)
    }) {
        return Ok(None);
    }
    let leading_conditional_replacement = sentences
        .first()
        .is_some_and(|sentence| looks_like_leading_conditional_self_replacement(sentence));
    let suffix_starts_with_trigger = prevention_then_trigger.is_some()
        || sentences
            .get(1)
            .is_some_and(|sentence| line_starts_with_trigger_intro_tokens(sentence));

    let first_sentence_tokens = if let Some(shape) = prevention_then_trigger.as_ref() {
        let mut tokens = shape.prevention_tokens.to_vec();
        tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
        tokens
    } else {
        let Some(tokens) = clone_sentence_chunk_tokens(&line.tokens, &sentences[..1]) else {
            return Ok(None);
        };
        tokens
    };
    let first_sentence_line = rewrite_line_tokens(line, &first_sentence_tokens);
    let first_static = if prevention_then_trigger.is_some() {
        None
    } else {
        match recognize_static_line(&first_sentence_line) {
            Ok(parsed) => parsed,
            Err(err) if leading_conditional_replacement => return Err(err),
            Err(_) => return Ok(None),
        }
    };
    if !leading_conditional_replacement
        && !(suffix_starts_with_trigger
            && (first_static.is_some() || prevention_then_trigger.is_some()))
    {
        return Ok(None);
    }

    let mut lines = if prevention_then_trigger.is_some() {
        vec![RecognizedLine::Static(RecognizedStaticLine {
            info: first_sentence_line.info.clone(),
            parse_tokens: first_sentence_line.tokens.clone(),
            chosen_option: None,
            parsed: parsed_prevention.map(Box::new),
        })]
    } else if let Some(static_line) = first_static {
        vec![RecognizedLine::Static(static_line)]
    } else if let Some(statement_line) = recognize_statement_line(&first_sentence_line)? {
        vec![RecognizedLine::Statement(statement_line)]
    } else {
        return Ok(None);
    };
    for (sentence_idx, sentence_tokens) in sentences.iter().enumerate().skip(1) {
        let sentence_line = rewrite_line_tokens(line, sentence_tokens);
        let is_typed_prevention_trigger = prevention_then_trigger.is_some() && sentence_idx == 1;
        let is_delayed_schedule =
            effect_grammar::delayed_sentence_shapes::parse_delayed_schedule_sentence_shape(
                &sentence_line.tokens,
            )
            .is_some();
        if is_typed_prevention_trigger
            || (line_starts_with_trigger_intro_tokens(&sentence_line.tokens)
                && !is_delayed_schedule)
        {
            let triggered = recognize_triggered_line(&sentence_line)?;
            lines.push(RecognizedLine::Triggered(triggered));
        } else if let Some(statement_line) = recognize_statement_line(&sentence_line)? {
            lines.push(RecognizedLine::Statement(statement_line));
        } else if let Some(static_line) = recognize_static_line(&sentence_line)? {
            lines.push(RecognizedLine::Static(static_line));
        } else {
            return Err(CardTextError::ParseError(format!(
                "parser could not lower a sentence following a conditional replacement: '{}'",
                render_token_slice(&sentence_line.tokens)
            )));
        }
    }

    Ok(Some(LineDispatchResult {
        lines,
        next_idx: idx + 1,
    }))
}

fn should_parse_delayed_trigger_line_as_spell_effect(
    preprocessed: &PreprocessedDocument,
    tokens: &[OwnedLexToken],
) -> bool {
    let builder_has_nonpermanent_spell_type =
        preprocessed.card.card_types_ref().iter().any(|card_type| {
            matches!(
                card_type,
                crate::types::CardType::Instant | crate::types::CardType::Sorcery
            )
        });
    let metadata_has_nonpermanent_spell_type = preprocessed.items.iter().any(|item| {
        matches!(
            item,
            PreprocessedItem::Metadata(metadata)
                if matches!(
                    metadata.value,
                    crate::cards::builders::MetadataLine::TypeLine(ref raw)
                        if raw
                            .split(|ch: char| !ch.is_alphabetic())
                            .any(|part| matches!(part, "Instant" | "Sorcery"))
                )
        )
    });

    let is_delayed_effect = document_grammar::parse_next_cast_trigger_surface(tokens).is_some()
        || effect_grammar::delayed_sentence_shapes::parse_delayed_this_turn_shape(tokens).is_some()
        || effect_grammar::delayed_sentence_shapes::parse_delayed_schedule_sentence_shape(tokens)
            .is_some();
    let is_source_spell_cast_trigger = grammar::parse_prefix(
        tokens,
        winnow::combinator::alt((
            grammar::phrase(&["when", "you", "cast", "this", "spell"]),
            grammar::phrase(&["whenever", "you", "cast", "this", "spell"]),
        )),
    )
    .is_some();
    (builder_has_nonpermanent_spell_type || metadata_has_nonpermanent_spell_type)
        && is_delayed_effect
        && !is_source_spell_cast_trigger
}

fn looks_like_activation_cost_prefix(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        document_grammar::recognize_activation_cost_head(tokens),
        crate::recognition::ParseOutcome::Match(_) | crate::recognition::ParseOutcome::Error(_)
    )
}

#[cfg(test)]
fn looks_like_static_line(normalized: &str) -> bool {
    lex_line(normalized, 0)
        .ok()
        .is_some_and(|tokens| looks_like_static_line_tokens(&tokens))
}

fn looks_like_static_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        super::grammar::structure::classify_static_line_family_lexed(tokens),
        Some(
            super::grammar::structure::StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep
                | super::grammar::structure::StaticLineFamily::Generic
        )
    )
}

fn looks_like_static_line_lexed(line: &PreprocessedLine) -> bool {
    if let Some(tokens) = tokens_after_non_keyword_label_prefix(line) {
        return looks_like_static_line_tokens(tokens);
    }
    looks_like_static_line_tokens(&line.tokens)
}

fn parse_segment_len_until_colon_outside_quotes<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    let initial_len = input.len();
    let mut inside_quotes = false;

    while let Some(token) = input.peek_token() {
        if token.kind == TokenKind::Quote {
            grammar::quote().parse_next(input)?;
            inside_quotes = !inside_quotes;
            continue;
        }
        if token.kind == TokenKind::Colon && !inside_quotes {
            return Ok(initial_len - input.len());
        }

        any.parse_next(input)?;
    }

    Err(grammar::backtrack_err("colon", "colon outside quotes"))
}

pub fn split_lexed_once_on_colon_outside_quotes(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let (left_len, rest) =
        grammar::parse_prefix(tokens, parse_segment_len_until_colon_outside_quotes)?;
    let (_, right_tokens) = grammar::parse_prefix(rest, grammar::colon())?;
    Some((&tokens[..left_len], right_tokens))
}

#[cfg(test)]
fn split_label_prefix(text: &str) -> Option<(&str, &str)> {
    let trimmed = text.trim();
    let (label, body) = trimmed.split_once('—')?;
    let label = label.trim();
    let body = body.trim();
    (!label.is_empty() && !body.is_empty() && !label.contains('.')).then_some((label, body))
}

fn split_label_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(String, &[OwnedLexToken], &[OwnedLexToken])> {
    let split = document_grammar::parse_statement_label_split_tokens(tokens)?;
    let label = render_label_prefix_tokens(split.label_tokens);
    (!label.is_empty()).then_some((label, split.label_tokens, split.body_tokens))
}

fn render_label_prefix_tokens(tokens: &[OwnedLexToken]) -> String {
    let rendered = render_token_slice(tokens);
    let trimmed = rendered.trim();
    if tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::Period)
    {
        let rest = trimmed.trim_start_matches('.').trim_start();
        return format!("... {rest}").trim().to_string();
    }
    if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Period)
    {
        let rest = trimmed.trim_end_matches('.').trim_end();
        return format!("{rest} ...").trim().to_string();
    }
    trimmed.to_string()
}

fn trigger_presentation_from_line_tokens(tokens: &[OwnedLexToken]) -> Option<PresentationLabel> {
    let (label, label_tokens, body_tokens) = split_label_prefix_lexed(tokens)?;
    if !looks_like_ability_word_label(label_tokens, false) {
        return None;
    }
    line_starts_with_trigger_intro_tokens(body_tokens)
        .then(|| trigger_presentation(label_tokens, &label))
}

fn trigger_presentation_from_preprocessed_line(
    line: &PreprocessedLine,
) -> Option<PresentationLabel> {
    trigger_presentation_from_line_tokens(&line.info.source_tokens)
        .or_else(|| trigger_presentation_from_line_tokens(&line.tokens))
}

pub(super) fn activated_presentation_from_preprocessed_line(
    line: &PreprocessedLine,
) -> Option<PresentationLabel> {
    let (label, _, _) = split_label_prefix_lexed(&line.info.source_tokens)
        .or_else(|| split_label_prefix_lexed(&line.tokens))?;
    Some(PresentationLabel::AbilityWord(label))
}

fn is_nonkeyword_choice_labeled_line(line: &PreprocessedLine) -> bool {
    split_label_prefix_lexed(strip_choice_bullet_prefix_tokens(&line.tokens)).is_some_and(
        |(label, label_tokens, _)| {
            document_grammar::parse_preserved_keyword_label_tokens(label_tokens).is_none()
                && !is_named_ability_label(label.as_str())
        },
    )
}

fn trigger_presentation(label_tokens: &[OwnedLexToken], label: &str) -> PresentationLabel {
    match document_grammar::parse_case_label_tokens(label_tokens) {
        Some(document_grammar::CaseLabelKind::ToSolve) => PresentationLabel::CaseToSolve,
        Some(document_grammar::CaseLabelKind::Solved) => PresentationLabel::CaseSolved,
        None => PresentationLabel::from_ability_word(label.trim()),
    }
}

fn is_case_ability_label(label_tokens: &[OwnedLexToken]) -> bool {
    document_grammar::parse_case_label_tokens(label_tokens).is_some()
}

fn recognize_case_to_solve_line(
    line: &PreprocessedLine,
    label_tokens: &[OwnedLexToken],
    body_tokens: &[OwnedLexToken],
) -> Result<Option<RecognizedTriggeredLine>, CardTextError> {
    if document_grammar::parse_case_label_tokens(label_tokens)
        != Some(document_grammar::CaseLabelKind::ToSolve)
    {
        return Ok(None);
    }

    // The trailing period is presentation, not part of the condition.
    let mut condition_tokens = body_tokens;
    while condition_tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Period)
    {
        condition_tokens = &condition_tokens[..condition_tokens.len() - 1];
    }
    if condition_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "case solve line is missing a condition: '{}'",
            line.info.raw_line
        )));
    }

    // A Case to solve is a triggered ability in disguise: the trigger is
    // built around the authored condition tokens from token constants.
    let mut tokens = crate::lexer::synthetic_word_tokens([
        "At",
        "the",
        "beginning",
        "of",
        "your",
        "end",
        "step",
    ]);
    tokens.push(OwnedLexToken::synthetic_comma());
    tokens.extend(crate::lexer::synthetic_word_tokens(["if"]));
    tokens.extend_from_slice(condition_tokens);
    tokens.push(OwnedLexToken::synthetic_comma());
    tokens.extend(crate::lexer::synthetic_word_tokens([
        "put", "a", "level", "counter", "on", "this",
    ]));
    tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    let rewritten = rewrite_line_tokens(line, &tokens);
    let mut triggered = recognize_triggered_line(&rewritten)?;
    triggered.presentation = Some(PresentationLabel::CaseToSolve);
    Ok(Some(triggered))
}

fn labeled_choice_block_has_peer(items: &[PreprocessedItem], idx: usize) -> bool {
    let mut probe = idx;
    while probe > 0 {
        probe -= 1;
        match items.get(probe) {
            Some(PreprocessedItem::Line(line)) if is_nonkeyword_choice_labeled_line(line) => {
                return true;
            }
            Some(PreprocessedItem::Line(_)) => break,
            Some(PreprocessedItem::Metadata(_)) => continue,
            None => break,
        }
    }

    let mut probe = idx + 1;
    while let Some(item) = items.get(probe) {
        match item {
            PreprocessedItem::Line(line) if is_nonkeyword_choice_labeled_line(line) => {
                return true;
            }
            PreprocessedItem::Line(_) => break,
            PreprocessedItem::Metadata(_) => {
                probe += 1;
                continue;
            }
        }
    }

    false
}

fn labeled_choice_block_has_named_option_header(items: &[PreprocessedItem], idx: usize) -> bool {
    let mut probe = idx;
    while probe > 0 {
        probe -= 1;
        match items.get(probe) {
            Some(PreprocessedItem::Line(line)) if is_named_option_as_enters_choice_header(line) => {
                return true;
            }
            Some(PreprocessedItem::Line(line)) if is_nonkeyword_choice_labeled_line(line) => {
                continue;
            }
            Some(PreprocessedItem::Metadata(_)) => continue,
            Some(PreprocessedItem::Line(_)) | None => break,
        }
    }

    false
}

fn normalize_trailing_keyword_activation_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentences.len() <= 1 {
        return None;
    }

    for split_idx in 1..sentences.len() {
        let prefix = clone_sentence_chunk_tokens(tokens, &sentences[..split_idx])?;
        let suffix = clone_sentence_chunk_tokens(tokens, &sentences[split_idx..])?;
        let Some((_, label_tokens, body_tokens)) = split_label_prefix_lexed(&suffix) else {
            continue;
        };
        if document_grammar::parse_preserved_keyword_label_tokens(label_tokens).is_none()
            || split_lexed_once_on_colon_outside_quotes(body_tokens).is_none()
        {
            continue;
        }
        return Some((prefix, suffix));
    }

    None
}

fn preflight_known_strict_unsupported(lines: &[&[OwnedLexToken]]) -> Option<CardTextError> {
    for tokens in lines {
        if document_grammar::parse_half_starting_life_plus_one_surface(tokens).is_some() {
            return Some(CardTextError::ParseError(
                "unsupported predicate".to_string(),
            ));
        }
    }
    None
}

fn preflight_invalid_payment_keyword_lines(lines: &[&[OwnedLexToken]]) -> Option<CardTextError> {
    for tokens in lines {
        for segment in grammar::split_lexed_slices_on_commas_or_semicolons(tokens) {
            let (keyword, cost_start, is_echo) =
                if document_grammar::parse_cumulative_upkeep_surface(segment).is_some() {
                    ("cumulative upkeep", 2, false)
                } else if segment.first().is_some_and(|token| token.is_word("echo")) {
                    ("echo", 1, true)
                } else {
                    continue;
                };

            let reminder_start = locate_token_index(segment, |token| {
                token.is_period() || token.kind == TokenKind::LParen
            })
            .or_else(|| {
                if is_echo {
                    locate_token_word(&segment[1..], "at").map(|idx| idx + 1)
                } else {
                    None
                }
            })
            .unwrap_or(segment.len());
            let mut cost_tokens = trim_lexed_commas(&segment[cost_start..reminder_start]);
            while cost_tokens
                .first()
                .is_some_and(|token| matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))
            {
                cost_tokens = &cost_tokens[1..];
            }

            if cost_tokens.is_empty() {
                continue;
            }

            let rendered_cost = render_token_slice(cost_tokens);

            match parse_payment_clause_as_total_cost(cost_tokens) {
                Ok(Some(_)) => {}
                Ok(None) => {
                    return Some(CardTextError::ParseError(format!(
                        "unsupported {keyword} payment cost (clause: '{}')",
                        rendered_cost.trim()
                    )));
                }
                Err(err) => {
                    return Some(CardTextError::ParseError(format!(
                        "unsupported {keyword} payment cost (clause: '{}'): {err}",
                        rendered_cost.trim()
                    )));
                }
            }
        }
    }

    None
}

/// The token-level form of [`normalize_explicit_named_source_references_for_builder`].
///
/// Recognition already holds the tokens, so the card's own name is replaced in
/// them directly: every alias occurrence the string form would rewrite becomes
/// the typed self-reference as synthesized word tokens, and the "enter" that
/// follows a rewritten subject becomes "enters" the same way. Nothing is
/// rendered back to text and lexed again. Kept tokens keep their spans; a
/// synthesized subject takes the span of the name it stands in for, which is
/// where the reference was authored.
///
/// Returns `None` when nothing changed, like the string form.
pub(crate) fn normalize_named_source_tokens_for_builder(
    card: &crate::card::CardBuilder,
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let subject = named_source_subject_for_builder(card);
    let aliases = aliases_for_builder(card);
    let all_words = alias_word_lists(&aliases);
    let mut rewritten = tokens.to_vec();
    let mut changed = false;
    for alias in &aliases {
        if let Some(next) =
            replace_named_source_alias_tokens(&rewritten, &alias.words, subject, &all_words, false)
        {
            rewritten = next;
            changed = true;
        }
    }
    if normalize_named_source_enter_agreement_tokens(&mut rewritten, subject) {
        changed = true;
    }
    changed.then_some(rewritten)
}

pub(crate) fn normalize_named_source_tokens_with_context(
    context: ParseContextView<'_>,
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let mut card = CardBuilder::new(
        crate::ids::CardId::new(),
        context.source().card_name.as_str(),
    )
    .card_types(context.card().card_types.clone());
    if !context.card().subtypes.is_empty() {
        card = card.subtypes(context.card().subtypes.clone());
    }
    normalize_named_source_tokens_for_builder(&card, tokens)
}

/// One pass of alias replacement over tokens — the token-level twin of
/// [`replace_named_source_aliases_with_options`].
///
/// Word pieces are the unit of matching, as in the string form; a match that
/// does not begin and end on token boundaries is left alone, since there is no
/// token to stand in for part of a token.
fn replace_named_source_alias_tokens(
    tokens: &[OwnedLexToken],
    alias_words: &[String],
    replacement: &str,
    all_alias_words: &[Vec<String>],
    preserve_surface_hints: bool,
) -> Option<Vec<OwnedLexToken>> {
    if alias_words.is_empty() {
        return None;
    }
    let pieces = source_alias_word_pieces(tokens);
    if pieces.is_empty() {
        return None;
    }
    // Which token each piece came from, so a piece range maps back to tokens.
    let mut piece_tokens = Vec::with_capacity(pieces.len());
    for (token_index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Period {
            continue;
        }
        piece_tokens.extend(std::iter::repeat_n(
            token_index,
            token.parser_word_pieces().len(),
        ));
    }
    debug_assert_eq!(piece_tokens.len(), pieces.len());

    let mut out: Vec<OwnedLexToken> = Vec::with_capacity(tokens.len());
    let mut next_token = 0usize;
    let mut word_idx = 0usize;
    let mut changed = false;
    while word_idx + alias_words.len() <= pieces.len() {
        if !source_alias_word_span_matches(&pieces, word_idx, &alias_words) {
            word_idx += 1;
            continue;
        }
        let overlaps_preserved_longer_alias = all_alias_words.iter().any(|longer_words| {
            longer_words.len() > alias_words.len()
                && source_alias_word_span_matches(&pieces, word_idx, longer_words)
        });
        if overlaps_preserved_longer_alias {
            word_idx += 1;
            continue;
        }
        let end_word = word_idx + alias_words.len();
        let remaining_words = pieces[word_idx..]
            .iter()
            .map(|piece| piece.text)
            .collect::<Vec<_>>();
        let alias_is_strict_prefix_of_compound_subtype =
            crate::grammar::filters::reference_tag_stage::compound_filter_subtype_prefix_word_len(
                &remaining_words,
            )
            .is_some_and(|compound_len| compound_len > alias_words.len());
        let preserve_surface = alias_is_strict_prefix_of_compound_subtype
            || source_alias_occurrence_looks_like_effect_verb_lexed(&pieces, word_idx, end_word)
            || source_alias_occurrence_is_name_override_surface_lexed(&pieces, word_idx, end_word)
            || source_alias_occurrence_is_created_token_name_lexed(&pieces, word_idx, end_word)
            || source_alias_occurrence_is_typed_subtype_noun_lexed(&pieces, word_idx, end_word)
            || source_alias_occurrence_is_rules_term_lexed(&pieces, word_idx, end_word)
            || (!pieces[end_word - 1].possessive
                && matches!(
                    pieces.get(end_word).map(|piece| piece.text),
                    Some("counter" | "counters")
                ))
            || (preserve_surface_hints
                && source_alias_occurrence_should_preserve_surface_lexed(
                    &pieces, word_idx, end_word,
                ));
        if preserve_surface {
            word_idx += 1;
            continue;
        }
        let first_token = piece_tokens[word_idx];
        let last_token = piece_tokens[end_word - 1];
        let starts_token = word_idx == 0 || piece_tokens[word_idx - 1] != first_token;
        let ends_token = end_word == pieces.len() || piece_tokens[end_word] != last_token;
        if !starts_token || !ends_token || first_token < next_token {
            word_idx += 1;
            continue;
        }
        out.extend_from_slice(&tokens[next_token..first_token]);
        let span = TextSpan {
            line: tokens[first_token].span.line,
            start: tokens[first_token].span.start,
            end: tokens[last_token].span.end,
        };
        let mut words: Vec<String> = replacement.split_whitespace().map(str::to_string).collect();
        // The matched name span includes an authored possessive; the typed
        // subject keeps that grammar.
        if pieces[end_word - 1].possessive
            && let Some(last) = words.last_mut()
        {
            last.push_str("'s");
        }
        out.extend(
            words
                .into_iter()
                .map(|word| OwnedLexToken::word(word, span)),
        );
        next_token = last_token + 1;
        word_idx = end_word;
        changed = true;
    }
    if !changed {
        return None;
    }
    out.extend_from_slice(&tokens[next_token..]);
    Some(out)
}

/// "this creature enter" → "this creature enters", on tokens.
fn normalize_named_source_enter_agreement_tokens(
    tokens: &mut [OwnedLexToken],
    subject: &str,
) -> bool {
    let subject_words: Vec<&str> = subject.split_whitespace().collect();
    let mut changed = false;
    let mut index = 0;
    while index + subject_words.len() < tokens.len() {
        let subject_here = subject_words
            .iter()
            .enumerate()
            .all(|(offset, word)| tokens[index + offset].is_word(word));
        let enter_index = index + subject_words.len();
        // The string form only rewrote "enter" at the end of the text or
        // before a space, never before punctuation.
        let followed_by_word_or_end = tokens
            .get(enter_index + 1)
            .is_none_or(|next| matches!(next.kind, TokenKind::Word | TokenKind::Number));
        if subject_here && tokens[enter_index].is_word("enter") && followed_by_word_or_end {
            let span = tokens[enter_index].span;
            tokens[enter_index] = OwnedLexToken::word("enters", span);
            changed = true;
        }
        index += 1;
    }
    changed
}

fn named_source_subject_for_builder(card: &crate::card::CardBuilder) -> &'static str {
    if card
        .card_types_ref()
        .contains(&crate::types::CardType::Creature)
    {
        "this creature"
    } else if card
        .card_types_ref()
        .contains(&crate::types::CardType::Land)
    {
        "this land"
    } else if card
        .card_types_ref()
        .contains(&crate::types::CardType::Artifact)
    {
        "this artifact"
    } else if card
        .card_types_ref()
        .contains(&crate::types::CardType::Enchantment)
    {
        "this enchantment"
    } else if card
        .card_types_ref()
        .contains(&crate::types::CardType::Planeswalker)
    {
        "this planeswalker"
    } else if card
        .card_types_ref()
        .contains(&crate::types::CardType::Battle)
    {
        "this battle"
    } else {
        "this permanent"
    }
}

#[derive(Debug, Clone, Copy)]
struct SourceAliasWordPiece<'a> {
    text: &'a str,
    span: TextSpan,
    possessive: bool,
    sentence: usize,
}

fn source_alias_word_pieces(tokens: &[OwnedLexToken]) -> Vec<SourceAliasWordPiece<'_>> {
    let mut sentence = 0usize;
    let mut pieces = Vec::new();
    for token in tokens {
        if token.kind == TokenKind::Period {
            sentence += 1;
            continue;
        }
        let possessive = crate::string_primitives::contains_char(token.slice.as_str(), '\'')
            || crate::string_primitives::contains_char(token.slice.as_str(), '’');
        pieces.extend(
            token
                .parser_word_pieces()
                .iter()
                .map(|piece: &TokenWordPiece| SourceAliasWordPiece {
                    text: piece.text.as_str(),
                    span: piece.span,
                    possessive,
                    sentence,
                }),
        );
    }
    pieces
}

fn source_alias_word_span_matches(
    pieces: &[SourceAliasWordPiece<'_>],
    start_word: usize,
    alias_words: &[String],
) -> bool {
    pieces
        .get(start_word..start_word + alias_words.len())
        .is_some_and(|window| {
            window
                .iter()
                .zip(alias_words.iter().map(String::as_str))
                .enumerate()
                .all(|(offset, (piece, expected))| {
                    piece.text == expected
                        || (offset + 1 == alias_words.len()
                            && piece.possessive
                            && piece.text.strip_suffix('s') == Some(expected))
                })
        })
}

fn source_alias_occurrence_is_rules_term_lexed(
    pieces: &[SourceAliasWordPiece<'_>],
    start_word: usize,
    end_word: usize,
) -> bool {
    let previous_word = start_word
        .checked_sub(1)
        .and_then(|idx| pieces.get(idx))
        .map(|piece| piece.text);
    let next_word = pieces.get(end_word).map(|piece| piece.text);
    let matched_word = (end_word == start_word + 1)
        .then(|| pieces.get(start_word).map(|piece| piece.text))
        .flatten();

    (matched_word == Some("control")
        && matches!(previous_word, Some("gain" | "gains" | "lose" | "loses"))
        && next_word == Some("of"))
        || (matched_word == Some("combat") && next_word == Some("damage"))
}

fn source_alias_occurrence_is_typed_subtype_noun_lexed(
    pieces: &[SourceAliasWordPiece<'_>],
    start_word: usize,
    end_word: usize,
) -> bool {
    let alias = pieces
        .get(start_word..end_word)
        .unwrap_or_default()
        .iter()
        .map(|piece| piece.text)
        .collect::<Vec<_>>()
        .join(" ");
    if !parse_subtype_flexible(&alias).is_some_and(|subtype| subtype.is_creature_type()) {
        return false;
    }

    let previous_word = start_word
        .checked_sub(1)
        .and_then(|idx| pieces.get(idx))
        .map(|piece| piece.text);
    let next_word = pieces.get(end_word).map(|piece| piece.text);

    previous_word == Some("target")
        || (matches!(previous_word, Some("a" | "an"))
            && matches!(
                next_word,
                Some("card" | "cards" | "creature" | "creatures" | "token" | "tokens")
            ))
}

fn source_alias_occurrence_looks_like_effect_verb_lexed(
    pieces: &[SourceAliasWordPiece<'_>],
    start_word: usize,
    end_word: usize,
) -> bool {
    // Word pieces are already normalized words; they become word tokens for
    // the shape parser without a round trip through text.
    let alias_tokens = crate::lexer::synthetic_word_tokens(
        pieces
            .get(start_word..end_word)
            .unwrap_or_default()
            .iter()
            .map(|piece| piece.text),
    );
    let remainder_tokens = crate::lexer::synthetic_word_tokens(
        pieces
            .get(end_word..)
            .unwrap_or_default()
            .iter()
            .map(|piece| piece.text),
    );
    !remainder_tokens.is_empty()
        && document_grammar::source_alias_effect_verb_surface_tokens(
            &alias_tokens,
            &remainder_tokens,
        )
        .is_some()
}

fn source_alias_occurrence_should_preserve_surface_lexed(
    pieces: &[SourceAliasWordPiece<'_>],
    start_word: usize,
    end_word: usize,
) -> bool {
    let previous_word = start_word
        .checked_sub(1)
        .and_then(|idx| pieces.get(idx))
        .map(|piece| piece.text);
    let previous_previous_word = start_word
        .checked_sub(2)
        .and_then(|idx| pieces.get(idx))
        .map(|piece| piece.text);
    let next_word = pieces.get(end_word).map(|piece| piece.text);

    // A source alias can be identical to a registered multiword creature
    // subtype. Without explicit source identity, a sentence-leading surface
    // such as "Time Lord gains vigilance" is ambiguous and must remain
    // authored so the typed filter grammar can own it. Builder/context-aware
    // callers use the non-preserving rewrite when they know the occurrence is
    // the source card itself.
    let begins_sentence = start_word == 0
        || pieces
            .get(start_word - 1)
            .zip(pieces.get(start_word))
            .is_some_and(|(previous, current)| previous.sentence != current.sentence);
    let alias = pieces
        .get(start_word..end_word)
        .unwrap_or_default()
        .iter()
        .map(|piece| piece.text)
        .collect::<Vec<_>>()
        .join(" ");
    if end_word > start_word + 1
        && begins_sentence
        && parse_subtype_flexible(&alias).is_some_and(|subtype| subtype.is_creature_type())
    {
        return true;
    }

    let is_modal_may = end_word == start_word + 1
        && pieces
            .get(start_word)
            .is_some_and(|piece| piece.text == "may")
        && previous_word.is_some_and(|word| {
            matches!(
                word,
                "you"
                    | "they"
                    | "player"
                    | "players"
                    | "opponent"
                    | "opponents"
                    | "controller"
                    | "owner"
            )
        });
    if is_modal_may {
        return true;
    }

    if previous_word == Some("as") {
        return false;
    }

    if previous_word == Some("is") && previous_previous_word == Some("name") {
        return true;
    }

    if previous_word == Some("for") && matches!(previous_previous_word, Some("vote" | "votes")) {
        return true;
    }

    // An alias can also be ordinary characteristic data. In
    // "becomes a Coward", the one-word front-face name happens to equal the
    // creature subtype; rewriting that predicate noun to "this sorcery"
    // produces the nonsensical "becomes a this sorcery" before the typed
    // subtype grammar gets a chance to consume it.
    let is_indefinite_become_descriptor = matches!(previous_word, Some("a" | "an"))
        && matches!(
            previous_previous_word,
            Some("become" | "becomes" | "became" | "becoming")
        );
    if is_indefinite_become_descriptor {
        return true;
    }

    next_word == Some(TOKEN_NAME_SUFFIX_WORD)
        // A named vote option remains option data in later references such as
        // "cards equal to the number of truth votes". It cannot denote the
        // source here because only players vote.
        || matches!(next_word, Some("vote" | "votes"))
        || next_word == Some("s")
        || previous_word.is_some_and(|word| {
            matches!(
                word,
                "attach"
                    | "destroy"
                    | "exile"
                    | "transform"
                    | "convert"
                    | "regenerate"
                    | "return"
                    | "tap"
                    | "untap"
                    | "control"
                    | "of"
                    | "to"
                    | "on"
            )
        })
        || next_word.is_some_and(|word| {
            matches!(
                word,
                "attack"
                    | "attacks"
                    | "become"
                    | "becomes"
                    | "becoming"
                    | "get"
                    | "gets"
                    | "deal"
                    | "deals"
                    | "counter"
                    | "counters"
                    | "enter"
                    | "enters"
                    | "remain"
                    | "remains"
                    | "power"
                    | "toughness"
            )
        })
}

fn source_alias_occurrence_is_name_override_surface_lexed(
    pieces: &[SourceAliasWordPiece<'_>],
    start_word: usize,
    _end_word: usize,
) -> bool {
    let previous_word = start_word
        .checked_sub(1)
        .and_then(|idx| pieces.get(idx))
        .map(|piece| piece.text);
    let previous_previous_word = start_word
        .checked_sub(2)
        .and_then(|idx| pieces.get(idx))
        .map(|piece| piece.text);

    previous_word == Some("named")
        || (previous_word == Some("is") && previous_previous_word == Some("name"))
}

fn source_alias_occurrence_is_created_token_name_lexed(
    pieces: &[SourceAliasWordPiece<'_>],
    start_word: usize,
    end_word: usize,
) -> bool {
    let Some(sentence) = pieces.get(start_word).map(|piece| piece.sentence) else {
        return false;
    };
    let previous_word = start_word
        .checked_sub(1)
        .and_then(|idx| pieces.get(idx))
        .map(|piece| piece.text);
    let follows_create = matches!(previous_word, Some("create" | "creates"))
        || pieces[..start_word]
            .iter()
            .rev()
            .take_while(|piece| piece.sentence == sentence && !matches!(piece.text, "and" | "then"))
            .take(12)
            .any(|piece| matches!(piece.text, "create" | "creates"));
    follows_create
        && pieces
            .get(end_word..)
            .unwrap_or_default()
            .iter()
            .take_while(|piece| piece.sentence == sentence)
            .any(|piece| matches!(piece.text, "token" | "tokens"))
}

fn normalize_named_source_enter_agreement(text: &str, subject: &str) -> String {
    let singular = format!("{subject} enter");
    let plural = format!("{subject} enters");
    if text.ends_with(&singular) {
        return format!("{}{}", &text[..text.len() - singular.len()], plural);
    }
    text.replace(&format!("{singular} "), &format!("{plural} "))
}

/// The card's full names: the printed name, its front face, and each of those
/// without a digital-variant marker or trailing numeral.
fn source_full_names_for_builder(card: &crate::card::CardBuilder) -> Vec<String> {
    let name = card.name_ref().trim();
    if name.is_empty() {
        return Vec::new();
    }

    let mut full_names = Vec::new();
    push_unique_source_name_alias(&mut full_names, name);
    if let Some((front_face, _)) = name.split_once(" // ") {
        push_unique_source_name_alias(&mut full_names, front_face);
    }
    let existing_full_names = full_names.clone();
    for full_name in existing_full_names {
        if let Some(stripped) = strip_leading_digital_variant_marker(full_name.as_str()) {
            push_unique_source_name_alias(&mut full_names, stripped);
        }
        if let Some(stripped) = strip_trailing_roman_numeral(full_name.as_str()) {
            push_unique_source_name_alias(&mut full_names, stripped);
        }
    }
    full_names
}

/// The alias surfaces one full name contributes, given the short alias rules
/// text uses for it: lowercased, deduplicated, with "&" spelled every way
/// rules text spells it.
fn push_source_name_alias_surfaces(aliases: &mut Vec<String>, full_name: &str, short_name: &str) {
    let mut push_alias = |alias: &str| {
        let alias = alias.trim().to_ascii_lowercase();
        if !alias.is_empty() && !aliases.contains(&alias) {
            let ampersandless = alias.replace(" & ", " ");
            let and_alias = alias.replace(" & ", " and ");
            aliases.push(alias);
            if !ampersandless.is_empty() && !aliases.contains(&ampersandless) {
                aliases.push(ampersandless);
            }
            if !and_alias.is_empty() && !aliases.contains(&and_alias) {
                aliases.push(and_alias);
            }
        }
    };

    push_alias(full_name);
    if short_name != full_name {
        push_alias(short_name);
    }
    if let Some((short_name, _)) = full_name.split_once(',') {
        push_alias(short_name);
        for part in short_name
            .split(" & ")
            .flat_map(|piece| piece.split(" and "))
        {
            push_alias(part);
        }
        if let Some(stripped) = strip_leading_digital_variant_marker(short_name) {
            push_alias(stripped);
        }
    }
}

/// Every alias surface as text, longest first.
#[cfg(test)]
fn source_name_aliases_for_builder(card: &crate::card::CardBuilder) -> Vec<String> {
    named_source_tokens::aliases_for_builder(card)
        .into_iter()
        .map(|alias| alias.text)
        .collect()
}

fn push_unique_source_name_alias(aliases: &mut Vec<String>, raw: &str) {
    let raw = raw.trim();
    if !raw.is_empty() && !aliases.iter().any(|existing| existing == raw) {
        aliases.push(raw.to_string());
    }
}

fn strip_leading_digital_variant_marker(name: &str) -> Option<&str> {
    let trimmed = name.trim();
    let bytes = trimmed.as_bytes();
    if bytes.len() > 2 && bytes[1] == b'-' && bytes[0].is_ascii_alphabetic() {
        let rest = trimmed[2..].trim();
        if !rest.is_empty() {
            return Some(rest);
        }
    }
    None
}

fn strip_trailing_roman_numeral(name: &str) -> Option<&str> {
    let trimmed = name.trim();
    let (prefix, suffix) = trimmed.rsplit_once(char::is_whitespace)?;
    let suffix = suffix.trim_matches(|ch: char| !ch.is_ascii_alphabetic());
    if suffix.len() < 2
        || !suffix.bytes().all(|byte| {
            matches!(
                byte.to_ascii_uppercase(),
                b'I' | b'V' | b'X' | b'L' | b'C' | b'D' | b'M'
            )
        })
    {
        return None;
    }
    let prefix = prefix.trim();
    (!prefix.is_empty()).then_some(prefix)
}

#[cfg(test)]
fn strip_non_keyword_label_prefix(text: &str) -> &str {
    let mut current = text.trim();
    if looks_like_numeric_result_prefix_text(current) {
        return current;
    }
    while let Some((label, body)) = split_label_prefix(current) {
        if lex_line(label, 0)
            .ok()
            .and_then(|tokens| document_grammar::parse_preserved_keyword_label_tokens(&tokens))
            .is_some()
        {
            break;
        }
        current = body.trim();
    }
    current
}

#[cfg(test)]
fn looks_like_numeric_result_prefix_text(text: &str) -> bool {
    lex_line(text.trim_start(), 0)
        .is_ok_and(|tokens| document_grammar::parse_numeric_result_prefix_tokens(&tokens).is_some())
}

fn is_named_ability_label(label: &str) -> bool {
    matches!(
        label.to_ascii_lowercase().as_str(),
        "alliance"
            | "astral projection"
            | "bigby's hand"
            | "body-print"
            | "boast"
            | "catch"
            | "cohort"
            | "devouring monster"
            | "diana"
            | "exhaust"
            | "gooooaaaalll!"
            | "hero's sundering"
            | "hunt for heresy"
            | "machina"
            | "mage hand"
            | "megamorph"
            | "morph"
            | "psychic blades"
            | "raid"
            | "renew"
            | "rope dart"
            | "scorching ray"
            | "share"
            | "shieldwall"
            | "sleight of hand"
            | "smear campaign"
            | "stunning strike"
            | "teleport"
            | "trance"
            | "throw"
            | "throw ..."
            | "valiant"
            | "waterbend"
            | "... catch"
    )
}

fn looks_like_ability_word_label(
    label_tokens: &[OwnedLexToken],
    preserve_as_choice_label: bool,
) -> bool {
    if preserve_as_choice_label {
        return false;
    }
    !label_tokens.is_empty()
        && !label_tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Colon))
        && token_word_refs(label_tokens).len() <= 4
}

/// Build the labeled public-route recognized form for the source-qualified
/// ability-trigger event directly from its two independently typed halves.
///
/// The generic triggered-line probe also speculates across every comma in a
/// line.  In a clause such as "a creature entering ... causes a triggered
/// ability ... to trigger", those probes can let the complete effect body be
/// claimed as a statement after the label is removed even though both the
/// trigger and resolution have already parsed exactly.  Restrict this fast
/// path to the qualified `AbilityTriggered` model so ordinary labeled
/// triggers retain the existing ambiguity handling.
fn recognize_labeled_qualified_ability_trigger(
    line: &PreprocessedLine,
) -> Option<RecognizedTriggeredLine> {
    let (trigger_with_intro, effect_tokens) = grammar::split_lexed_once_on_comma(&line.tokens)?;
    let trigger_tokens = trigger_with_intro.get(1..)?;
    // This is a deliberately speculative fast-path probe. A diagnostic from
    // either half means only that this narrow qualified-trigger shape does
    // not own the line; the committing triggered-line parser runs below.
    let trigger = match crate::parse_loss::capture(|| parse_trigger_clause_lexed(trigger_tokens)).0
    {
        Ok(trigger) => trigger,
        Err(_) => return None,
    };
    if !matches!(
        trigger,
        crate::model::ast::TriggerSpec::AbilityTriggered {
            source_filter: Some(_),
            caused_by_source_entering: true,
            ..
        }
    ) {
        return None;
    }
    let effects = match crate::parse_loss::capture(|| parse_effect_sentences_lexed(effect_tokens)).0
    {
        Ok(effects) => effects,
        Err(_) => return None,
    };
    if effects.is_empty() {
        return None;
    }
    let candidate = render_triggered_split_candidate(trigger_tokens, effect_tokens, None, None)?;
    Some(candidate.into_recognized_line(line, &line.tokens))
}

fn probe_triggered_line(line: &PreprocessedLine) -> Option<RecognizedTriggeredLine> {
    #[allow(clippy::manual_ok_err)]
    match recognize_triggered_line(line) {
        Ok(triggered) => Some(triggered),
        Err(_) => None,
    }
}

fn normalize_activation_cost_tokens_for_builder(
    card: &crate::card::CardBuilder,
    cost_tokens: Vec<OwnedLexToken>,
) -> Result<Vec<OwnedLexToken>, CardTextError> {
    if !tokens_mention_source_alias(card, &cost_tokens) {
        return Ok(cost_tokens);
    }
    // Keep a directly parseable named-source cost intact so the typed cost
    // model can retain its Oracle-facing source-reference surface.
    if parse_activation_cost_tokens_rewrite(&cost_tokens).is_ok() {
        return Ok(cost_tokens);
    }
    Ok(normalize_named_source_sentence_tokens(card, &cost_tokens).unwrap_or(cost_tokens))
}

fn normalize_activation_effect_tokens_for_builder(
    card: &crate::card::CardBuilder,
    effect_tokens: &[OwnedLexToken],
) -> Result<Vec<OwnedLexToken>, CardTextError> {
    Ok(
        normalize_named_source_tokens_for_builder(card, effect_tokens)
            .unwrap_or_else(|| effect_tokens.to_vec()),
    )
}

fn render_original_text_for_token_slice(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Option<String> {
    let span = span_from_tokens(tokens)?;
    let original_span = map_span_to_original(
        span,
        line.info.normalized.normalized.as_str(),
        line.info.normalized.original.as_str(),
        &line.info.normalized.char_map,
    );
    line.info
        .normalized
        .original
        .get(original_span.start..original_span.end)
        .map(str::to_string)
}

/// A line whose text is the rendering of `tokens`.
///
/// The tokens keep the spans they came with. Recognized lines built from a
/// rewrite carry the source line's info, and consumers map effect spans back
/// to the authored text through it, so a span must stay relative to the line
/// it was authored in — not to the rendering.
fn rewrite_line_tokens(line: &PreprocessedLine, tokens: &[OwnedLexToken]) -> PreprocessedLine {
    let normalized = render_token_slice(tokens);
    let mut rewritten = line.clone();
    rewritten.info.normalized.original = normalized.clone();
    rewritten.info.normalized.normalized = normalized.clone();
    rewritten.info.normalized.char_map = (0..normalized.len()).collect();
    rewritten.tokens = tokens.to_vec();
    rewritten
}

/// The authored tokens that sit under a normalized token slice.
///
/// `source_tokens` were lexed from the trimmed authored line while the source
/// map speaks in untrimmed offsets, so the mapped span is shifted by the
/// leading whitespace before it is compared with token spans.
pub(super) fn authored_tokens_for_normalized_slice(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let span = span_from_tokens(tokens)?;
    let original = line.info.normalized.original.as_str();
    let original_span = map_span_to_original(
        span,
        line.info.normalized.normalized.as_str(),
        original,
        &line.info.normalized.char_map,
    );
    let offset = original.len() - original.trim_start().len();
    let start = original_span.start.saturating_sub(offset);
    let end = original_span.end.saturating_sub(offset);
    // Every authored token the mapped span touches: the source map is
    // approximate at the edges, and a token it clips is still under the slice.
    let selected: Vec<OwnedLexToken> = line
        .info
        .source_tokens
        .iter()
        .filter(|token| token.span.start < end && token.span.end > start)
        .cloned()
        .collect();
    (!selected.is_empty()).then_some(selected)
}

#[path = "named_source_tokens.rs"]
mod named_source_tokens;
use named_source_tokens::*;

fn try_parse_triggered_line_with_named_source_rewrite(
    card: &crate::card::CardBuilder,
    line: &PreprocessedLine,
    authored: &[OwnedLexToken],
) -> Result<Option<RecognizedTriggeredLine>, CardTextError> {
    // This fallback is reached with the authored token stream so a
    // comma-bearing source name is still available. Reminder text is
    // presentation, not a second executable trigger body, so parentheticals
    // are dropped the way `line.tokens` dropped them.
    let semantic = strip_parenthetical_tokens(authored);
    let Some(rewritten) = normalize_named_source_trigger_tokens(card, &semantic) else {
        return Ok(None);
    };

    for candidate in this_permanent_candidates(rewritten) {
        let rewritten_line = rewrite_line_tokens(line, &candidate);
        if let Ok(mut triggered) = recognize_triggered_line(&rewritten_line) {
            restore_authored_named_source_trigger_subject(card, authored, &mut triggered);
            return Ok(Some(triggered));
        }
    }

    Ok(None)
}

fn restore_authored_named_source_trigger_subject(
    card: &crate::card::CardBuilder,
    authored: &[OwnedLexToken],
    triggered: &mut RecognizedTriggeredLine,
) {
    let Some(mut authored_tokens) = leading_authored_trigger_subject(card, authored) else {
        return;
    };

    let generic_subject = named_source_subject_for_builder(card);
    let tokens = &triggered.trigger_parse_tokens;
    // Preprocessing and some typed trigger shapes canonicalize a named source
    // to the source-only subject "this". Either form is restored to its
    // authored provenance.
    let Some((subject_len, possessive)) = leading_generic_subject_tokens(tokens, generic_subject)
        .or_else(|| leading_generic_subject_tokens(tokens, "this"))
    else {
        return;
    };
    if possessive && let Some(last) = authored_tokens.last_mut() {
        let span = last.span;
        let slice = format!("{}'s", last.slice);
        *last = OwnedLexToken::word(slice, span);
    }
    authored_tokens.extend_from_slice(&tokens[subject_len..]);
    triggered.trigger_parse_tokens = authored_tokens;
}

/// The words of a typed self-reference subject such as "this creature".
fn generic_subject_words(subject: &str) -> &'static [&'static str] {
    match subject {
        "this creature" => &["this", "creature"],
        "this land" => &["this", "land"],
        "this artifact" => &["this", "artifact"],
        "this enchantment" => &["this", "enchantment"],
        "this planeswalker" => &["this", "planeswalker"],
        "this battle" => &["this", "battle"],
        "this permanent" => &["this", "permanent"],
        _ => &["this"],
    }
}

/// How many leading tokens spell `subject`, and whether the last of them
/// carries a possessive ("this creature's"). `None` when the tokens do not
/// begin with the subject as whole words.
fn leading_generic_subject_tokens(
    tokens: &[OwnedLexToken],
    subject: &str,
) -> Option<(usize, bool)> {
    let words = generic_subject_words(subject);
    if tokens.len() < words.len() {
        return None;
    }
    for (offset, word) in words.iter().enumerate() {
        let token = &tokens[offset];
        if token.is_word(word) {
            continue;
        }
        let is_last = offset + 1 == words.len();
        let possessive = format!("{word}'s");
        if is_last && token.is_word(&possessive) {
            return Some((words.len(), true));
        }
        return None;
    }
    Some((words.len(), false))
}

fn line_starts_with_lparen_token(line: &PreprocessedLine) -> bool {
    line.tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::LParen)
}

fn is_fully_parenthetical_line(line: &PreprocessedLine) -> bool {
    line_starts_with_lparen_token(line)
        && line
            .tokens
            .last()
            .is_some_and(|token| token.kind == TokenKind::RParen)
}

fn is_delayed_when_that_dies_this_turn_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    document_grammar::parse_delayed_prior_object_dies_surface(tokens).is_some()
}

fn is_delayed_when_that_leaves_battlefield_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::delayed_sentence_shapes::parse_delayed_tagged_leaves_shape(tokens).is_some()
}

fn is_delayed_next_end_step_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::delayed_sentence_shapes::parse_delayed_schedule_sentence_shape(tokens).is_some()
}

fn is_attack_group_combat_damage_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    grammar::has_phrase(tokens, &["either", "of", "those", "creatures"])
        && grammar::has_phrase(tokens, &["deals", "combat", "damage"])
        && grammar::has_phrase(tokens, &["this", "combat"])
}

fn split_trigger_sentence_chunks_rewrite_lexed(
    tokens: &[OwnedLexToken],
) -> Vec<Vec<OwnedLexToken>> {
    let sentence_tokens = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentence_tokens.len() <= 1 {
        return sentence_tokens
            .into_iter()
            .map(|sentence| sentence.to_vec())
            .collect();
    }

    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut current_starts_with_trigger = false;

    for sentence_tokens in sentence_tokens {
        let sentence_starts_with_trigger = line_starts_with_trigger_intro_tokens(sentence_tokens);
        let sentence_is_delayed_followup =
            is_delayed_when_that_dies_this_turn_followup_sentence(sentence_tokens)
                || is_delayed_when_that_leaves_battlefield_followup_sentence(sentence_tokens)
                || is_delayed_next_end_step_followup_sentence(sentence_tokens);
        let sentence_is_attack_group_followup =
            is_attack_group_combat_damage_followup_sentence(sentence_tokens);
        if !current.is_empty()
            && current_starts_with_trigger
            && sentence_starts_with_trigger
            && !sentence_is_delayed_followup
            && !sentence_is_attack_group_followup
        {
            if let Some(chunk) = clone_sentence_chunk_tokens(tokens, &current) {
                chunks.push(chunk);
            }
            current.clear();
            current_starts_with_trigger = false;
        }
        if current.is_empty() {
            current_starts_with_trigger = sentence_starts_with_trigger;
        }
        current.push(sentence_tokens);
    }

    if !current.is_empty()
        && let Some(chunk) = clone_sentence_chunk_tokens(tokens, &current)
    {
        chunks.push(chunk);
    }

    chunks
}

fn starts_with_when_one_or_more_this_way_clause(tokens: &[OwnedLexToken]) -> bool {
    let this_way_in_prefix = grammar::split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
        .map(|(before, _after)| grammar::has_phrase(before, &["this", "way"]))
        .unwrap_or(false);
    document_grammar::parse_when_one_or_more_surface(tokens).is_some() && this_way_in_prefix
}

fn rewrite_when_one_or_more_this_way_line(line: &PreprocessedLine) -> PreprocessedLine {
    let rewritten_tokens =
        crate::effect_sentences::rewrite_when_one_or_more_this_way_clause_prefix(&line.tokens);
    rewrite_line_tokens(line, &rewritten_tokens)
}

fn split_reveal_first_draw_line_rewrite_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<Vec<OwnedLexToken>>> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentences.len() <= 1 {
        return None;
    }

    let first_tokens = *sentences.first()?;
    document_grammar::parse_reveal_first_draw_surface(first_tokens)?;

    let tail_tokens = clone_sentence_chunk_tokens(tokens, &sentences[1..])?;
    document_grammar::parse_reveal_first_draw_followup_surface(&tail_tokens)?;

    Some(vec![first_tokens.to_vec(), tail_tokens])
}

fn classify_unsupported_line_reason(line: &PreprocessedLine) -> &'static str {
    let classification_tokens = tokens_after_non_keyword_label_prefix(line).unwrap_or(&line.tokens);

    if is_bullet_line(line) {
        return "bullet-line-without-modal-header";
    }
    if line_starts_with_trigger_intro_tokens(&line.tokens) {
        return "triggered-line-not-yet-supported";
    }
    if split_lexed_once_on_colon_outside_quotes(&line.tokens).is_some() {
        return "activated-line-not-yet-supported";
    }
    if matches!(
        document_grammar::parse_unsupported_line_head(classification_tokens),
        Some(document_grammar::UnsupportedLineHeadSurface::ModalChoice)
    ) {
        return "modal-header-not-yet-supported";
    }
    if matches!(
        super::grammar::structure::classify_statement_line_family_lexed(&line.tokens),
        Some(super::grammar::structure::StatementLineFamily::ArtRating)
    ) {
        return "outside-the-game-rating-not-supported";
    }
    if looks_like_statement_line_lexed(line) {
        return "statement-line-not-yet-supported";
    }
    if looks_like_static_line_lexed(line) {
        return "static-line-not-yet-supported";
    }
    "unclassified-line-family"
}

fn try_parse_labeled_line_dispatch(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    // Prefer the authored token stream for the narrow labeled trigger whose
    // event is itself a source-qualified triggered-ability event.  Generic
    // preprocessing is allowed to strip presentation labels and rewrite
    // source nouns before line-family dispatch; for this event both the
    // trigger and its complete resolution already have exact typed parsers,
    // so retain the label and build the Triggered recognized form before either half can
    // be claimed as a standalone statement.
    if let Some((label, label_tokens, body_tokens)) =
        split_label_prefix_lexed(&line.info.source_tokens)
    {
        let body_line = rewrite_line_tokens(line, body_tokens);
        let is_eminence = label.eq_ignore_ascii_case("eminence");
        let mut authored_trigger = recognize_labeled_qualified_ability_trigger(&body_line);
        if authored_trigger.is_none() && is_eminence {
            authored_trigger = probe_triggered_line(&body_line);
        }
        if authored_trigger.is_none() && looks_like_ability_word_label(label_tokens, false) {
            let authored_body = authored_tokens_for_normalized_slice(line, body_tokens)
                .unwrap_or_else(|| body_tokens.to_vec());
            authored_trigger = try_parse_triggered_line_with_named_source_rewrite(
                &preprocessed.card,
                line,
                &authored_body,
            )?;
        }
        if authored_trigger.is_none()
            && is_eminence
            && let Some(spec) = super::grammar::structure::split_triggered_conditional_clause_lexed(
                &body_line.tokens,
                1,
            )
        {
            authored_trigger = Some(RecognizedTriggeredLine {
                info: line.info.clone(),
                full_text: render_token_slice(&body_line.tokens),
                full_parse_tokens: body_line.tokens.clone(),
                trigger_parse_tokens: spec.trigger_tokens.to_vec(),
                effect_parse_tokens: spec.effects_tokens.to_vec(),
                intervening_if: Some(spec.predicate),
                max_triggers_per_turn: None,
                chosen_option: None,
                presentation: Some(PresentationLabel::AbilityWord("Eminence".to_string())),
            });
        }
        if line_starts_with_trigger_intro_tokens(&body_line.tokens)
            && let Some(mut triggered) = authored_trigger
        {
            if looks_like_ability_word_label(label_tokens, false) {
                triggered.presentation = Some(trigger_presentation(label_tokens, &label));
            }
            let (triggered, next_idx) =
                extend_triggered_line_with_result_followups(&preprocessed.items, idx, triggered);
            return Ok(Some(LineDispatchResult::single(
                RecognizedLine::Triggered(triggered),
                next_idx,
            )));
        }

        // Normalization removes ordinary ability-word prefixes before recognized form
        // dispatch. Triggered lines above recover that presentation from the
        // authored token stream, but static lines previously fell through to
        // the already-stripped stream and silently lost labels such as
        // `Hellbent`. When the authored body names its source, build the parse
        // view with card metadata first so a creature does not degrade to the
        // untyped subject `this` before the static grammar sees it.
        if looks_like_ability_word_label(label_tokens, false)
            && !looks_like_leading_conditional_self_replacement(&body_line.tokens)
        {
            let builder_aware_static = authored_tokens_for_normalized_slice(line, body_tokens)
                .and_then(|body| normalize_named_source_sentence_tokens(&preprocessed.card, &body))
                .map(|body| rewrite_line_tokens(line, &body))
                .map(|body_line| recognize_static_line(&body_line))
                .transpose()?
                .flatten();
            let mut labeled_static = builder_aware_static;
            if labeled_static.is_none() {
                labeled_static = recognize_static_line(line)?;
            }
            if let Some(mut static_line) = labeled_static {
                static_line
                    .info
                    .semantic_facts
                    .static_ability
                    .presentation_label = Some(PresentationLabel::from_ability_word(label));
                return Ok(Some(LineDispatchResult::single(
                    RecognizedLine::Static(static_line),
                    idx + 1,
                )));
            }
        }
    }

    let Some((label, label_tokens, body_tokens)) = split_label_prefix_lexed(&line.tokens) else {
        return Ok(None);
    };

    let is_named_label = is_named_ability_label(label.as_str());
    let max_speed_chosen_option = label
        .eq_ignore_ascii_case("max speed")
        .then_some(ChosenOptionContext::MaxSpeed);
    let preserve_as_choice_label = labeled_choice_block_has_peer(&preprocessed.items, idx)
        && labeled_choice_block_has_named_option_header(&preprocessed.items, idx);
    let presentation = (!preserve_as_choice_label).then(|| {
        activated_presentation_from_preprocessed_line(line)
            .unwrap_or_else(|| PresentationLabel::AbilityWord(label.clone()))
    });
    if document_grammar::parse_preserved_keyword_label_tokens(label_tokens).is_some() {
        return Ok(None);
    }

    if let Some(triggered) = recognize_case_to_solve_line(line, label_tokens, body_tokens)? {
        return Ok(Some(LineDispatchResult::single(
            RecognizedLine::Triggered(triggered),
            idx + 1,
        )));
    }

    let authored_body_tokens = authored_tokens_for_normalized_slice(line, body_tokens);
    let body_line = rewrite_line_tokens(line, body_tokens);
    if label.eq_ignore_ascii_case("eminence")
        && let Some((trigger_with_intro, after_trigger)) =
            grammar::split_lexed_once_on_comma(&body_line.tokens)
        && let Some((_source_zone_condition, effect_tokens)) =
            grammar::split_lexed_once_on_comma(after_trigger)
        && trigger_with_intro.len() > 1
    {
        let source_zone_condition = PredicateAst::Source(SourcePredicateAst::SourceMatches(crate::ObjectFilter {
            any_of: vec![
                crate::ObjectFilter {
                    zone: Some(crate::zone::Zone::Command),
                    ..Default::default()
                },
                crate::ObjectFilter {
                    zone: Some(crate::zone::Zone::Battlefield),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }));
        let triggered = RecognizedTriggeredLine {
            info: line.info.clone(),
            full_text: render_token_slice(&body_line.tokens),
            full_parse_tokens: body_line.tokens.clone(),
            trigger_parse_tokens: trigger_with_intro[1..].to_vec(),
            effect_parse_tokens: effect_tokens.to_vec(),
            intervening_if: Some(source_zone_condition),
            max_triggers_per_turn: None,
            chosen_option: None,
            presentation: Some(PresentationLabel::AbilityWord("Eminence".to_string())),
        };
        let (triggered, next_idx) =
            extend_triggered_line_with_result_followups(&preprocessed.items, idx, triggered);
        return Ok(Some(LineDispatchResult::single(
            RecognizedLine::Triggered(triggered),
            next_idx,
        )));
    }
    let labeled_activation = if (!line_starts_with_lparen_token(line)
        || is_fully_parenthetical_line(line))
        && let Some((cost_tokens, effect_parse_tokens)) =
            split_activation_text_tokens_lexed(&body_line.tokens)
    {
        Some((cost_tokens, effect_parse_tokens))
    } else {
        None
    };
    let prefer_activation = labeled_activation
        .as_ref()
        .is_some_and(|(cost_tokens, _)| looks_like_activation_cost_prefix(cost_tokens));

    if line_starts_with_trigger_intro_tokens(&body_line.tokens) {
        let triggered = recognize_labeled_qualified_ability_trigger(&body_line)
            .or_else(|| probe_triggered_line(&body_line));
        if let Some(mut triggered) = triggered {
            restore_authored_named_source_trigger_subject(
                &preprocessed.card,
                authored_body_tokens.as_deref().unwrap_or(&body_line.tokens),
                &mut triggered,
            );
            if preserve_as_choice_label && !is_case_ability_label(label_tokens) {
                triggered.chosen_option =
                    document_grammar::parse_chosen_option_context_tokens(label_tokens);
            }
            if looks_like_ability_word_label(label_tokens, preserve_as_choice_label) {
                triggered.presentation = trigger_presentation_from_preprocessed_line(line)
                    .or_else(|| Some(trigger_presentation(label_tokens, &label)));
            }
            let (triggered, next_idx) =
                extend_triggered_line_with_result_followups(&preprocessed.items, idx, triggered);
            return Ok(Some(LineDispatchResult::single(
                RecognizedLine::Triggered(triggered),
                next_idx,
            )));
        }
        if let Some(mut triggered) = try_parse_triggered_line_with_named_source_rewrite(
            &preprocessed.card,
            line,
            authored_body_tokens.as_deref().unwrap_or(&body_line.tokens),
        )? {
            if preserve_as_choice_label && !is_case_ability_label(label_tokens) {
                triggered.chosen_option =
                    document_grammar::parse_chosen_option_context_tokens(label_tokens);
            }
            if looks_like_ability_word_label(label_tokens, preserve_as_choice_label) {
                triggered.presentation = trigger_presentation_from_preprocessed_line(line)
                    .or_else(|| Some(trigger_presentation(label_tokens, &label)));
            }
            let (triggered, next_idx) =
                extend_triggered_line_with_result_followups(&preprocessed.items, idx, triggered);
            return Ok(Some(LineDispatchResult::single(
                RecognizedLine::Triggered(triggered),
                next_idx,
            )));
        }
        if allow_unsupported && is_named_label {
            return Ok(Some(LineDispatchResult::single(
                RecognizedLine::Unsupported(RecognizedUnsupportedLine {
                    info: line.info.clone(),
                    reason_code: "triggered-line-not-yet-supported",
                }),
                idx + 1,
            )));
        }
        if is_named_label {
            return Err(recognize_triggered_line(&body_line)
                .err()
                .unwrap_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported triggered line: '{}'",
                        body_line.info.normalized.normalized.as_str()
                    ))
                }));
        }
    }

    if prefer_activation
        && let Some((cost_tokens, effect_parse_tokens)) = labeled_activation.clone()
    {
        let normalized_cost_tokens =
            normalize_activation_cost_tokens_for_builder(&preprocessed.card, cost_tokens.clone())?;
        match parse_activation_cost_tokens_rewrite(&normalized_cost_tokens) {
            Ok(cost) => {
                let effect_parse_tokens = normalize_activation_effect_tokens_for_builder(
                    &preprocessed.card,
                    &effect_parse_tokens,
                )?;
                return Ok(Some(LineDispatchResult::single(
                    RecognizedLine::Activated(RecognizedActivatedLine {
                        info: line.info.clone(),
                        cost,
                        cost_parse_tokens: normalized_cost_tokens,
                        effect_parse_tokens,
                        presentation: presentation.clone(),
                        chosen_option: max_speed_chosen_option.clone().or_else(|| {
                            preserve_as_choice_label
                                .then(|| {
                                    document_grammar::parse_chosen_option_context_tokens(
                                        label_tokens,
                                    )
                                })
                                .flatten()
                        }),
                    }),
                    idx + 1,
                )));
            }
            Err(err) if looks_like_activation_cost_prefix(&cost_tokens) => {
                return Err(err);
            }
            Err(_) => {}
        }
    }

    if is_named_label && let Some(keyword_line) = recognize_keyword_line(&body_line)? {
        return Ok(Some(LineDispatchResult::single(
            RecognizedLine::Keyword(keyword_line),
            idx + 1,
        )));
    }

    if should_prefer_statement_before_static_for_nonpermanent_spell(preprocessed, &body_line.tokens)
    {
        match recognize_statement_line(&body_line) {
            Ok(Some(mut statement_line)) => {
                apply_labeled_statement_surface_facts(
                    &mut statement_line,
                    line,
                    presentation.clone(),
                );
                let (statement_line, next_idx) = extend_statement_line_with_result_followups(
                    &preprocessed.items,
                    idx,
                    statement_line,
                );
                return Ok(Some(LineDispatchResult::single(
                    RecognizedLine::Statement(statement_line),
                    next_idx,
                )));
            }
            Ok(None) => {}
            Err(_) if tokens_mention_source_alias(&preprocessed.card, &body_line.tokens) => {
                // The authored proper-name subject is normalized by the
                // builder-aware branch below. Do not let a context-free
                // statement probe commit to a suffix of that name first.
            }
            Err(error) => return Err(error),
        }
    }
    if let Some(split_result) =
        parse_labeled_conditional_replacement_sentence_split(&body_line, idx)?
    {
        return Ok(Some(split_result));
    }

    if tokens_mention_source_alias(&preprocessed.card, &body_line.tokens)
        && let Some(rewritten_body) =
            normalize_named_source_sentence_tokens(&preprocessed.card, &body_line.tokens)
    {
        let rewritten_body_line = rewrite_line_tokens(line, &rewritten_body);
        if let Some(mut static_line) = recognize_static_line(&rewritten_body_line)? {
            if let Some(chosen_option) = max_speed_chosen_option.clone() {
                static_line.chosen_option = Some(chosen_option);
            } else if preserve_as_choice_label {
                static_line.chosen_option =
                    document_grammar::parse_chosen_option_context_tokens(label_tokens);
            }
            static_line
                .info
                .semantic_facts
                .static_ability
                .presentation_label = presentation.clone();
            return Ok(Some(LineDispatchResult::single(
                RecognizedLine::Static(static_line),
                idx + 1,
            )));
        }
        if let Some(mut statement_line) = recognize_statement_line(&rewritten_body_line)? {
            apply_labeled_statement_surface_facts(&mut statement_line, line, presentation.clone());
            let (statement_line, next_idx) = extend_statement_line_with_result_followups(
                &preprocessed.items,
                idx,
                statement_line,
            );
            return Ok(Some(LineDispatchResult::single(
                RecognizedLine::Statement(statement_line),
                next_idx,
            )));
        }
    }

    if let Some(mut static_line) = recognize_static_line(&body_line)? {
        if let Some(chosen_option) = max_speed_chosen_option.clone() {
            static_line.chosen_option = Some(chosen_option);
        } else if preserve_as_choice_label {
            static_line.chosen_option =
                document_grammar::parse_chosen_option_context_tokens(label_tokens);
        }
        if std::env::var("IRONSMITH_CHOICE_TRACE").is_ok() {
            eprintln!("labeled-static-dispatch: presentation={presentation:?}");
        }
        static_line
            .info
            .semantic_facts
            .static_ability
            .presentation_label = presentation.clone();
        return Ok(Some(LineDispatchResult::single(
            RecognizedLine::Static(static_line),
            idx + 1,
        )));
    }

    if tokens_mention_source_alias(&preprocessed.card, &body_line.tokens)
        && let Some(rewritten_body) =
            normalize_named_source_sentence_tokens(&preprocessed.card, &body_line.tokens)
    {
        let rewritten_body_line = rewrite_line_tokens(line, &rewritten_body);
        if let Some(mut static_line) = recognize_static_line(&rewritten_body_line)? {
            if let Some(chosen_option) = max_speed_chosen_option.clone() {
                static_line.chosen_option = Some(chosen_option);
            } else if preserve_as_choice_label {
                static_line.chosen_option =
                    document_grammar::parse_chosen_option_context_tokens(label_tokens);
            }
            static_line
                .info
                .semantic_facts
                .static_ability
                .presentation_label = presentation.clone();
            return Ok(Some(LineDispatchResult::single(
                RecognizedLine::Static(static_line),
                idx + 1,
            )));
        }
    }

    if let Some((cost_tokens, effect_parse_tokens)) = labeled_activation {
        let normalized_cost_tokens =
            normalize_activation_cost_tokens_for_builder(&preprocessed.card, cost_tokens.clone())?;
        match parse_activation_cost_tokens_rewrite(&normalized_cost_tokens) {
            Ok(cost) => {
                let effect_parse_tokens = normalize_activation_effect_tokens_for_builder(
                    &preprocessed.card,
                    &effect_parse_tokens,
                )?;
                return Ok(Some(LineDispatchResult::single(
                    RecognizedLine::Activated(RecognizedActivatedLine {
                        info: line.info.clone(),
                        cost,
                        cost_parse_tokens: normalized_cost_tokens,
                        effect_parse_tokens,
                        presentation,
                        chosen_option: max_speed_chosen_option.or_else(|| {
                            preserve_as_choice_label
                                .then(|| {
                                    document_grammar::parse_chosen_option_context_tokens(
                                        label_tokens,
                                    )
                                })
                                .flatten()
                        }),
                    }),
                    idx + 1,
                )));
            }
            Err(err) if looks_like_activation_cost_prefix(&cost_tokens) => {
                return Err(err);
            }
            Err(_) => {}
        }
    }

    if let Some(mut statement_line) = recognize_statement_line(&body_line)? {
        apply_labeled_statement_surface_facts(&mut statement_line, line, presentation);
        let (statement_line, next_idx) =
            extend_statement_line_with_result_followups(&preprocessed.items, idx, statement_line);
        return Ok(Some(LineDispatchResult::single(
            RecognizedLine::Statement(statement_line),
            next_idx,
        )));
    }

    Ok(None)
}

fn apply_labeled_statement_surface_facts(
    statement: &mut RecognizedStatementLine,
    source_line: &PreprocessedLine,
    presentation: Option<PresentationLabel>,
) {
    // The statement body is parsed after the label is stripped and named card
    // references have been rewritten to "this". Keep the original label and
    // transform destination as presentation metadata while the lowered effect
    // program continues to use the normalized self reference.
    let source_facts = super::grammar::line_semantic_facts::parse_line_semantic_facts_tokens(
        &source_line.info.source_tokens,
    );
    if let Some(as_transforms) = source_facts.statement.as_transforms_effect_program {
        statement
            .info
            .semantic_facts
            .statement
            .as_transforms_effect_program = Some(as_transforms);
    }
    statement.info.semantic_facts.statement.presentation_label = presentation.clone();
    // Static lines carry the same authored ability word ("Threshold — This
    // creature has flying as long as ..."), and the static-ability compile
    // reads its own facts slot (Mystic Visionary family).
    statement
        .info
        .semantic_facts
        .static_ability
        .presentation_label = presentation;
}

fn try_parse_triggered_line_dispatch(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if !line_starts_with_trigger_intro_tokens(&line.tokens) {
        return Ok(None);
    }

    if should_parse_delayed_trigger_line_as_spell_effect(preprocessed, &line.tokens)
        && let Some(statement_line) = recognize_statement_line(line)?
    {
        return Ok(Some(LineDispatchResult::single(
            RecognizedLine::Statement(statement_line),
            idx + 1,
        )));
    }

    // Keep a token created by the first instruction correlated with the
    // delayed sacrifice in the second sentence. The generic trigger-sentence
    // splitter otherwise separates them before the semantic linked-token rule
    // can inspect the complete authored effect tail.
    let (preserve_linked_created_token, preserve_reciprocal_token_lifecycle) =
        grammar::split_lexed_once_on_comma(&line.tokens)
            .map(|(_, effect_tokens)| {
                (
                    super::semantic_line_parsing::has_linked_created_token_next_turn_sacrifice_surface(
                        effect_tokens,
                    ),
                    super::semantic_line_parsing::has_created_token_reciprocal_lifecycle_surface(
                        effect_tokens,
                    ),
                )
            })
            .unwrap_or_default();
    if preserve_linked_created_token {
        return try_parse_linked_created_token_triggered_line(
            preprocessed,
            idx,
            line,
            allow_unsupported,
        );
    }
    if preserve_reciprocal_token_lifecycle {
        return try_parse_unsplit_triggered_line_dispatch(
            preprocessed,
            idx,
            line,
            allow_unsupported,
            preserve_reciprocal_token_lifecycle,
        );
    }
    let trigger_chunks = split_trigger_sentence_chunks_rewrite_lexed(&line.tokens);
    if trigger_chunks.len() <= 1 {
        return try_parse_unsplit_triggered_line_dispatch(
            preprocessed,
            idx,
            line,
            allow_unsupported,
            false,
        );
    }
    try_parse_triggered_line_dispatch_general(
        preprocessed,
        idx,
        line,
        allow_unsupported,
        trigger_chunks,
    )
}

fn try_parse_linked_created_token_triggered_line(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let mut triggered = match recognize_triggered_line(line) {
        Ok(triggered) => triggered,
        Err(_) if allow_unsupported => {
            return Ok(Some(LineDispatchResult::single(
                RecognizedLine::Unsupported(RecognizedUnsupportedLine {
                    info: line.info.clone(),
                    reason_code: "triggered-line-not-yet-supported",
                }),
                idx + 1,
            )));
        }
        Err(error) => {
            return Err(strict_unsupported_triggered_line_error(
                &line.info.raw_line,
                Some(error),
            ));
        }
    };
    restore_authored_named_source_trigger_subject(
        &preprocessed.card,
        &line.info.source_tokens,
        &mut triggered,
    );
    let (triggered, next_idx) =
        extend_triggered_line_with_result_followups(&preprocessed.items, idx, triggered);
    Ok(Some(LineDispatchResult::single(
        RecognizedLine::Triggered(triggered),
        next_idx,
    )))
}

fn try_parse_triggered_line_dispatch_general(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
    trigger_chunks: Vec<Vec<OwnedLexToken>>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if trigger_chunks.len() > 1 {
        let mut lines = Vec::with_capacity(trigger_chunks.len());
        for chunk_tokens in trigger_chunks {
            let authored_chunk_tokens = authored_tokens_for_normalized_slice(line, &chunk_tokens);
            let chunk_line = rewrite_line_tokens(line, &chunk_tokens);
            match recognize_triggered_line(&chunk_line) {
                Ok(mut triggered) => {
                    restore_authored_named_source_trigger_subject(
                        &preprocessed.card,
                        authored_chunk_tokens
                            .as_deref()
                            .unwrap_or(&chunk_line.tokens),
                        &mut triggered,
                    );
                    lines.push(RecognizedLine::Triggered(triggered));
                }
                Err(_) => {
                    if starts_with_when_one_or_more_this_way_clause(&chunk_line.tokens)
                        && let Some(statement) = recognize_statement_line(
                            &rewrite_when_one_or_more_this_way_line(&chunk_line),
                        )?
                    {
                        lines.push(RecognizedLine::Statement(statement));
                        continue;
                    }
                    if let Some(statement) = recognize_statement_line(&chunk_line)? {
                        lines.push(RecognizedLine::Statement(statement));
                        continue;
                    }
                    if let Some(triggered) = try_parse_triggered_line_with_named_source_rewrite(
                        &preprocessed.card,
                        line,
                        authored_chunk_tokens
                            .as_deref()
                            .unwrap_or(&chunk_line.tokens),
                    )? {
                        lines.push(RecognizedLine::Triggered(triggered));
                        continue;
                    }
                    if allow_unsupported {
                        lines.push(RecognizedLine::Unsupported(RecognizedUnsupportedLine {
                            info: line.info.clone(),
                            reason_code: "triggered-line-not-yet-supported",
                        }));
                    } else {
                        return Err(strict_unsupported_triggered_line_error(
                            chunk_line.info.normalized.normalized.as_str(),
                            recognize_triggered_line(&chunk_line).err(),
                        ));
                    }
                }
            }
        }
        return Ok(Some(LineDispatchResult {
            lines,
            next_idx: idx + 1,
        }));
    }

    unreachable!("split triggered-line dispatch requires multiple chunks")
}

fn try_parse_unsplit_triggered_line_dispatch(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
    preserve_reciprocal_token_lifecycle: bool,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    // The generic recognized form splitter has no card-name context. Give an
    // exact comma-bearing full source name its builder-aware rewrite before a
    // syntactically valid but lossy split can claim the comma inside the name
    // as the trigger/effect boundary.
    if (trigger_names_source(&preprocessed.card, &line.info.source_tokens)
        || preserve_reciprocal_token_lifecycle
        || normalize_named_source_tokens_for_builder(&preprocessed.card, &line.tokens).is_some())
        && let Some(triggered) = try_parse_triggered_line_with_named_source_rewrite(
            &preprocessed.card,
            line,
            &line.info.source_tokens,
        )?
    {
        let (triggered, next_idx) =
            extend_triggered_line_with_result_followups(&preprocessed.items, idx, triggered);
        return Ok(Some(LineDispatchResult::single(
            RecognizedLine::Triggered(triggered),
            next_idx,
        )));
    }

    match recognize_triggered_line(line) {
        Ok(mut triggered) => {
            restore_authored_named_source_trigger_subject(
                &preprocessed.card,
                &line.info.source_tokens,
                &mut triggered,
            );
            let (triggered, next_idx) =
                extend_triggered_line_with_result_followups(&preprocessed.items, idx, triggered);
            Ok(Some(LineDispatchResult::single(
                RecognizedLine::Triggered(triggered),
                next_idx,
            )))
        }
        Err(_) => {
            if let Some(triggered) = try_parse_triggered_line_with_named_source_rewrite(
                &preprocessed.card,
                line,
                &line.info.source_tokens,
            )? {
                let (triggered, next_idx) = extend_triggered_line_with_result_followups(
                    &preprocessed.items,
                    idx,
                    triggered,
                );
                Ok(Some(LineDispatchResult::single(
                    RecognizedLine::Triggered(triggered),
                    next_idx,
                )))
            } else if allow_unsupported {
                Ok(Some(LineDispatchResult::single(
                    RecognizedLine::Unsupported(RecognizedUnsupportedLine {
                        info: line.info.clone(),
                        reason_code: "triggered-line-not-yet-supported",
                    }),
                    idx + 1,
                )))
            } else {
                Err(strict_unsupported_triggered_line_error(
                    &line.info.raw_line,
                    recognize_triggered_line(line).err(),
                ))
            }
        }
    }
}

#[cfg(test)]
fn split_activation_text_parts_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, String)> {
    let (cost_tokens, effect_tokens) = split_activation_text_tokens_lexed(tokens)?;
    Some((
        cost_tokens,
        render_token_slice(&effect_tokens).trim().to_string(),
    ))
}

fn split_activation_text_tokens_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let (cost_tokens, effect_tokens) = split_lexed_once_on_colon_outside_quotes(tokens)?;

    let cost_tokens = trim_lexed_commas(cost_tokens);
    let effect_tokens = trim_lexed_commas(effect_tokens);
    if cost_tokens.is_empty() || effect_tokens.is_empty() {
        return None;
    }

    Some((cost_tokens.to_vec(), effect_tokens.to_vec()))
}

fn rewrite_cleave_bracket_document(
    document: &PreprocessedDocument,
    remove_bracketed_text: bool,
) -> Result<PreprocessedDocument, CardTextError> {
    let mut rewritten = document.clone();
    let mut items = Vec::with_capacity(rewritten.items.len());

    for item in rewritten.items {
        let PreprocessedItem::Line(line) = item else {
            items.push(item);
            continue;
        };
        if !line
            .tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::LBracket | TokenKind::RBracket))
        {
            items.push(PreprocessedItem::Line(line));
            continue;
        }

        let mut depth = 0usize;
        let mut tokens = Vec::with_capacity(line.tokens.len());
        for token in &line.tokens {
            match token.kind {
                TokenKind::LBracket => {
                    depth += 1;
                }
                TokenKind::RBracket => {
                    if depth == 0 {
                        return Err(CardTextError::ParseError(format!(
                            "cleave line has an unmatched closing bracket: '{}'",
                            line.info.raw_line
                        )));
                    }
                    depth -= 1;
                }
                _ if depth == 0 || !remove_bracketed_text => tokens.push(token.clone()),
                _ => {}
            }
        }
        if depth != 0 {
            return Err(CardTextError::ParseError(format!(
                "cleave line has an unmatched opening bracket: '{}'",
                line.info.raw_line
            )));
        }
        if tokens.is_empty() {
            continue;
        }
        items.push(PreprocessedItem::Line(rewrite_line_tokens(&line, &tokens)));
    }

    rewritten.items = items;
    Ok(rewritten)
}

pub fn parse_text_to_semantic_document_with_context(
    context: &mut ParseContext,
    card: CardBuilder,
    text: String,
) -> Result<(RewriteSemanticDocument, ParseAnnotations), CardTextError> {
    // A card is the unit of recognition: nothing parsed for the previous card
    // is relevant to this one, and the memo must not grow across cards.
    crate::sentence_memo::reset();
    let allow_unsupported = context.features().allow_unsupported;
    let card_name = card.name_ref().to_string();
    let _trace_scope = parse_trace::scope(format!(
        "card parse: \"{}\" allow_unsupported={} source_lines={}",
        card_name,
        allow_unsupported,
        text.lines().count()
    ));
    if parser_trace_enabled() {
        eprintln!(
            "[parser-flow] stage=parse_text_to_semantic_document:start card={:?} allow_unsupported={} lines={}",
            card.name_ref(),
            allow_unsupported,
            text.lines().count()
        );
    }
    let mut preprocessed =
        preprocess_document_with_provenance(card, text.as_str(), context.provenance().clone())?;
    // Preflight checks read the authored token stream of every line the
    // document phase produced — the same words, tokenized once.
    let authored_lines: Vec<&[OwnedLexToken]> = preprocessed
        .items
        .iter()
        .map(|item| match item {
            PreprocessedItem::Metadata(line) => line.info.source_tokens.as_slice(),
            PreprocessedItem::Line(line) => line.info.source_tokens.as_slice(),
        })
        .collect();
    if let Some(err) = preflight_invalid_payment_keyword_lines(&authored_lines) {
        return Err(err);
    }
    if !allow_unsupported && let Some(err) = preflight_known_strict_unsupported(&authored_lines) {
        return Err(err);
    }
    context.replace_provenance(preprocessed.provenance.clone());
    let semantic_facts = document_fact_grammar::parse_document_semantic_facts(
        preprocessed.items.iter().filter_map(|item| match item {
            PreprocessedItem::Metadata(_) => None,
            PreprocessedItem::Line(line) => {
                Some((line.info.display_line_index, line.tokens.as_slice()))
            }
        }),
    );
    let cleave_preprocessed = if semantic_facts.cleave_rewrite.is_some() {
        let cleaved = rewrite_cleave_bracket_document(&preprocessed, true)?;
        preprocessed = rewrite_cleave_bracket_document(&preprocessed, false)?;
        Some(cleaved)
    } else {
        None
    };
    parse_trace::event(format!(
        "preprocessed document: {} item(s)",
        preprocessed.items.len()
    ));
    for item in &preprocessed.items {
        match item {
            PreprocessedItem::Metadata(meta) => parse_trace::event(format!(
                "line {} metadata: {:?}",
                meta.info.display_line_index + 1,
                meta.value
            )),
            PreprocessedItem::Line(line) => {
                let raw = line.info.raw_line.trim();
                let normalized = line.info.normalized.normalized.trim();
                if raw == normalized {
                    parse_trace::event(format!(
                        "line {} text: \"{}\"",
                        line.info.display_line_index + 1,
                        normalized
                    ));
                } else {
                    parse_trace::event(format!(
                        "line {} text: \"{}\" -> \"{}\"",
                        line.info.display_line_index + 1,
                        raw,
                        normalized
                    ));
                }
            }
        }
    }
    if parser_trace_enabled() {
        eprintln!(
            "[parser-flow] stage=parse_text_to_semantic_document:preprocessed items={}",
            preprocessed.items.len()
        );
    }
    let document_context = context.view().child(ParseScopeKind::Document);
    document_context.bind_well_known_keys();
    let recognized = recognize_document_with_context(document_context, &preprocessed)?;
    let cleave_recognized = cleave_preprocessed
        .as_ref()
        .map(|document| {
            recognize_document_with_context(
                document_context.child(ParseScopeKind::CleaveBranch),
                document,
            )
        })
        .transpose()?;
    parse_trace::event(format!("recognized lines: {}", recognized.lines.len()));
    if parser_trace_enabled() {
        eprintln!(
            "[parser-flow] stage=parse_text_to_semantic_document:recognized lines={}",
            recognized.lines.len()
        );
    }
    let semantic = assemble_document_with_symbols(
        preprocessed,
        recognized,
        cleave_recognized,
        semantic_facts,
        allow_unsupported,
        context.symbols().clone(),
    )?;
    let annotations = semantic.annotations.clone();
    parse_trace::event(format!("semantic items: {}", semantic.items.len()));
    if parser_trace_enabled() {
        eprintln!(
            "[parser-flow] stage=parse_text_to_semantic_document:done items={}",
            semantic.items.len()
        );
    }
    Ok((semantic, annotations))
}

#[inline(never)]
pub fn recognize_document_with_context(
    context: ParseContextView<'_>,
    preprocessed: &PreprocessedDocument,
) -> Result<RecognizedDocument, CardTextError> {
    let allow_unsupported = context.features().allow_unsupported;
    let mut lines = Vec::with_capacity(preprocessed.items.len());
    let mut idx = 0usize;
    while idx < preprocessed.items.len() {
        let item = &preprocessed.items[idx];
        match item {
            PreprocessedItem::Metadata(meta) => {
                let recognized = RecognizedLine::Metadata(recognize_metadata_line(
                    meta.info.clone(),
                    meta.value.clone(),
                )?);
                trace_recognized_line(&recognized);
                lines.push(recognized);
                idx += 1;
            }
            PreprocessedItem::Line(line) => {
                // Every recognizer of this line, and the later phases that parse
                // its effects, bind the keys they mint in the line's symbol scope.
                let line_context = context.child(ParseScopeKind::Line {
                    source_line: line.info.display_line_index,
                });
                let _line_references = line_context.reference_scope();
                if try_push_complete_typed_static_line(line, &mut lines)? {
                    idx += 1;
                    continue;
                }
                if let Some(next_idx) =
                    try_push_named_source_gain_statement(preprocessed, idx, line, &mut lines)?
                {
                    idx = next_idx;
                    continue;
                }
                if try_push_complete_typed_quoted_gain_statement(line, &mut lines)? {
                    idx += 1;
                    continue;
                }
                idx = dispatch_remaining_preprocessed_line(
                    line_context,
                    preprocessed,
                    idx,
                    line,
                    allow_unsupported,
                    &mut lines,
                )?;
            }
        }
    }

    Ok(RecognizedDocument { lines })
}

#[inline(never)]
fn dispatch_remaining_preprocessed_line(
    line_context: ParseContextView<'_>,
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
    lines: &mut Vec<RecognizedLine>,
) -> Result<usize, CardTextError> {
    let _line_scope = parse_trace::scope(format!(
        "line {} parse: \"{}\"",
        line.info.display_line_index + 1,
        line.info.raw_line
    ));
    parser_trace("recognize_document:line", &line.tokens);
    if let Some(next_idx) =
        try_push_level_header_block(preprocessed, idx, line, allow_unsupported, lines)?
    {
        return Ok(next_idx);
    }
    if try_push_saga_chapter(preprocessed, line, lines)? {
        return Ok(idx + 1);
    }
    if let Some(next_idx) = try_push_modal_bullet_block(preprocessed, idx, line, lines)? {
        return Ok(next_idx);
    }
    if let Some(next_idx) = try_push_sticker_sheet_ticket_marker_result(
        line_context,
        preprocessed,
        idx,
        line,
        allow_unsupported,
        lines,
    ) {
        return Ok(next_idx);
    }
    if try_push_reveal_first_draw_line(line, lines)? {
        return Ok(idx + 1);
    }
    if try_push_trailing_keyword_activation(preprocessed, line, lines)? {
        return Ok(idx + 1);
    }
    let normalized = line.info.normalized.normalized.as_str();
    if normalized == LESS_THAN_ONE_MANA_REDUCTION_REMINDER {
        // A few inputs preserve this rules sentence on its own physical line.
        // Keep it attached to the preceding cost reducer so lowering can
        // distinguish an explicit minimum from an unbounded reduction.
        if let Some(RecognizedLine::Static(previous)) = lines.last_mut()
            && previous.parsed.is_none()
        {
            previous.parse_tokens.extend(line.tokens.clone());
        }
        return Ok(idx + 1);
    }
    if let Some(next_idx) = try_push_labeled_line_dispatch(
        line_context,
        preprocessed,
        idx,
        line,
        allow_unsupported,
        lines,
    )? {
        return Ok(next_idx);
    }
    if try_push_complete_typed_statement(line, lines)? {
        return Ok(idx + 1);
    }
    if let Some(next_idx) = try_push_named_source_dispatch(
        line_context,
        preprocessed,
        idx,
        line,
        allow_unsupported,
        lines,
    )? {
        return Ok(next_idx);
    }
    if !allow_unsupported && let Some(err) = diagnose_known_unsupported_rewrite_line(&line.tokens) {
        return Err(err);
    }
    push_standard_dispatch(
        line_context,
        preprocessed,
        idx,
        line,
        allow_unsupported,
        lines,
    )
}

fn try_push_level_header_block(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
    lines: &mut Vec<RecognizedLine>,
) -> Result<Option<usize>, CardTextError> {
    let Some((level_block, next_idx)) =
        try_parse_level_header_block(preprocessed, idx, line, allow_unsupported)?
    else {
        return Ok(None);
    };
    trace_recognized_line(&level_block);
    lines.push(level_block);
    Ok(Some(next_idx))
}

fn try_push_modal_bullet_block(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    lines: &mut Vec<RecognizedLine>,
) -> Result<Option<usize>, CardTextError> {
    let Some((modal_block, next_idx)) = try_parse_modal_bullet_block(preprocessed, idx, line)?
    else {
        return Ok(None);
    };
    trace_recognized_line(&modal_block);
    lines.push(modal_block);
    Ok(Some(next_idx))
}

fn try_push_saga_chapter(
    preprocessed: &PreprocessedDocument,
    line: &PreprocessedLine,
    lines: &mut Vec<RecognizedLine>,
) -> Result<bool, CardTextError> {
    let Some(authored) = parse_saga_chapter_prefix_tokens(&line.info.source_tokens) else {
        return Ok(false);
    };
    let chapters = authored.chapters.clone();
    let presentation_label = authored.presentation_label.clone();
    let text = render_token_slice(authored.body_tokens).trim().to_string();
    // Retain the authored casing and chapter label from the authored stream,
    // but compile the preprocessed body. The latter has reminder text removed,
    // so token reminder abilities cannot leak into the executable program.
    let parse_tokens = parse_saga_chapter_prefix_tokens(&line.tokens)
        .filter(|normalized| normalized.chapters == chapters)
        .map(|normalized| normalized.body_tokens.to_vec())
        .unwrap_or_else(|| authored.body_tokens.to_vec());
    // Saga chapters bypass ordinary source normalization, so normalize only
    // their parse view while preserving the authored display text.
    let parse_tokens = normalize_named_source_sentence_tokens(&preprocessed.card, &parse_tokens)
        .unwrap_or(parse_tokens);
    let recognized = RecognizedLine::SagaChapter(recognize_saga_chapter_line(
        line,
        chapters,
        presentation_label,
        text.as_str(),
        &parse_tokens,
    )?);
    trace_recognized_line(&recognized);
    lines.push(recognized);
    Ok(true)
}

fn try_push_reveal_first_draw_line(
    line: &PreprocessedLine,
    lines: &mut Vec<RecognizedLine>,
) -> Result<bool, CardTextError> {
    let Some(chunks) = split_reveal_first_draw_line_rewrite_lexed(&line.tokens) else {
        return Ok(false);
    };
    for chunk_tokens in chunks {
        let chunk_line = rewrite_line_tokens(line, &chunk_tokens);
        if line_starts_with_trigger_intro_tokens(&chunk_line.tokens) {
            for trigger_chunk in split_trigger_sentence_chunks_rewrite_lexed(&chunk_line.tokens) {
                let trigger_line = rewrite_line_tokens(&chunk_line, &trigger_chunk);
                let recognized =
                    RecognizedLine::Triggered(recognize_triggered_line(&trigger_line)?);
                trace_recognized_line(&recognized);
                lines.push(recognized);
            }
        } else if let Some(static_line) = recognize_static_line(&chunk_line)? {
            let recognized = RecognizedLine::Static(static_line);
            trace_recognized_line(&recognized);
            lines.push(recognized);
        } else {
            return Err(CardTextError::ParseError(format!(
                "parser could not split reveal-first-draw line family: '{}'",
                line.info.raw_line
            )));
        }
    }
    Ok(true)
}

fn try_push_trailing_keyword_activation(
    preprocessed: &PreprocessedDocument,
    line: &PreprocessedLine,
    lines: &mut Vec<RecognizedLine>,
) -> Result<bool, CardTextError> {
    let Some((prefix_tokens, suffix_tokens)) =
        normalize_trailing_keyword_activation_sentence_lexed(&line.tokens)
    else {
        return Ok(false);
    };
    let prefix_line = rewrite_line_tokens(line, &prefix_tokens);
    let (prefix_statement, prefix_statement_error) = match recognize_statement_line(&prefix_line) {
        Ok(statement) => (statement, None),
        Err(err) => (None, Some(err)),
    };
    if let Some(statement_line) = prefix_statement {
        let recognized = RecognizedLine::Statement(statement_line);
        trace_recognized_line(&recognized);
        lines.push(recognized);
    } else {
        let mut parsed_raw_static_prefix = false;
        let prefix_static_error = match recognize_static_line(&prefix_line) {
            Ok(Some(static_line)) => {
                let recognized = RecognizedLine::Static(static_line);
                trace_recognized_line(&recognized);
                lines.push(recognized);
                parsed_raw_static_prefix = true;
                None
            }
            Ok(None) => None,
            Err(err) => Some(err),
        };
        if parsed_raw_static_prefix {
            // Handled by the raw static parse above.
        } else if let Some(rewritten_prefix) =
            normalize_named_source_sentence_tokens(&preprocessed.card, &prefix_line.tokens)
        {
            let rewritten_prefix_line = rewrite_line_tokens(line, &rewritten_prefix);
            if let Some(statement_line) = recognize_statement_line(&rewritten_prefix_line)? {
                let recognized = RecognizedLine::Statement(statement_line);
                trace_recognized_line(&recognized);
                lines.push(recognized);
            } else if let Some(static_line) = recognize_static_line(&rewritten_prefix_line)? {
                let recognized = RecognizedLine::Static(static_line);
                trace_recognized_line(&recognized);
                lines.push(recognized);
            } else {
                return Err(CardTextError::ParseError(format!(
                    "parser could not split leading sentence before keyword ability: '{}'",
                    line.info.raw_line
                )));
            }
        } else if let Some(err) = prefix_statement_error {
            return Err(err);
        } else if let Some(err) = prefix_static_error {
            return Err(err);
        } else {
            return Err(CardTextError::ParseError(format!(
                "parser could not split leading sentence before keyword ability: '{}'",
                line.info.raw_line
            )));
        }
    }

    let suffix_line = rewrite_line_tokens(line, &suffix_tokens);
    let Some((_, _, body_tokens)) = split_label_prefix_lexed(&suffix_line.tokens) else {
        return Err(CardTextError::ParseError(format!(
            "parser could not recover keyword activation suffix: '{}'",
            line.info.raw_line
        )));
    };
    let Some((cost_tokens, effect_parse_tokens)) = split_activation_text_tokens_lexed(body_tokens)
    else {
        return Err(CardTextError::ParseError(format!(
            "parser could not recover activation suffix: '{}'",
            line.info.raw_line
        )));
    };
    let normalized_cost_tokens =
        normalize_activation_cost_tokens_for_builder(&preprocessed.card, cost_tokens.clone())?;
    let cost = parse_activation_cost_tokens_rewrite(&normalized_cost_tokens)?;
    let effect_parse_tokens =
        normalize_activation_effect_tokens_for_builder(&preprocessed.card, &effect_parse_tokens)?;
    let recognized = RecognizedLine::Activated(RecognizedActivatedLine {
        info: suffix_line.info.clone(),
        cost,
        cost_parse_tokens: normalized_cost_tokens,
        effect_parse_tokens,
        presentation: None,
        chosen_option: None,
    });
    trace_recognized_line(&recognized);
    lines.push(recognized);
    Ok(true)
}

fn try_push_sticker_sheet_ticket_marker_result(
    line_context: ParseContextView<'_>,
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
    lines: &mut Vec<RecognizedLine>,
) -> Option<usize> {
    let ticket_ctx = line_dispatch::LineDispatchContext {
        parse: line_context,
        preprocessed,
        idx,
        line,
        allow_unsupported,
    };
    let dispatch = line_family_handlers::sticker_sheet_ticket_marker_result(&ticket_ctx)?;
    for recognized in &dispatch.lines {
        trace_recognized_line(recognized);
    }
    lines.extend(dispatch.lines);
    Some(dispatch.next_idx)
}

fn try_push_labeled_line_dispatch(
    line_context: ParseContextView<'_>,
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
    lines: &mut Vec<RecognizedLine>,
) -> Result<Option<usize>, CardTextError> {
    if split_label_prefix_lexed(&line.info.source_tokens).is_none()
        && split_label_prefix_lexed(&line.tokens).is_none()
    {
        return Ok(None);
    }
    let Some(mut dispatch) =
        try_parse_labeled_line_dispatch(preprocessed, idx, line, allow_unsupported)?
    else {
        return Ok(None);
    };
    if try_merge_labeled_prior_token_replacement_statement(lines, &dispatch) {
        parse_trace::event("joined labeled prior-token replacement to preceding statement");
        return Ok(Some(dispatch.next_idx));
    }
    line_dispatch::attach_compiler_trigger_facts(line_context, &mut dispatch)?;
    for recognized in &dispatch.lines {
        trace_recognized_line(recognized);
    }
    let next_idx = dispatch.next_idx;
    lines.extend(dispatch.lines);
    Ok(Some(next_idx))
}

#[inline(never)]
fn try_push_complete_typed_static_line(
    line: &PreprocessedLine,
    lines: &mut Vec<RecognizedLine>,
) -> Result<bool, CardTextError> {
    if split_label_prefix_lexed(&line.info.source_tokens).is_some()
        || split_label_prefix_lexed(&line.tokens).is_some()
    {
        // The labeled-line family owns both the typed body and its authored
        // presentation metadata. Claiming the normalized body here would
        // compile the right static abilities while silently discarding the
        // label before semantic assembly.
        return Ok(false);
    }
    if super::grammar::structure::classify_static_line_family_lexed(&line.tokens)
        != Some(super::grammar::structure::StaticLineFamily::GrantedQuotedAbility)
    {
        return Ok(false);
    }
    if split_lexed_sentences(&line.tokens).len() != 1 {
        // This fast path owns one complete quoted-grant sentence. Earlier
        // sentences on the same line (a search, a token creation) are
        // resolution steps that sentence dispatch must keep.
        return Ok(false);
    }
    let Some(parsed) = crate::keyword_static::parse_filter_has_granted_ability_line(&line.tokens)?
    else {
        return Ok(false);
    };
    if parsed.is_empty() {
        return Ok(false);
    }
    let recognized = RecognizedLine::Static(RecognizedStaticLine {
        info: line.info.clone(),
        parse_tokens: line.tokens.clone(),
        chosen_option: None,
        parsed: Some(Box::new(LineAst::StaticAbilities(parsed))),
    });
    trace_recognized_line(&recognized);
    lines.push(recognized);
    Ok(true)
}

/// The lines another family owns even though they carry a quoted ability
/// gain; the quoted-gain fast path declines them all alike, whatever the
/// order.
const QUOTED_GAIN_DECLINES: &[fn(&PreprocessedLine) -> Result<bool, CardTextError>] = &[
    // a labeled body reaches labeled-line dispatch first
    |line| {
        Ok(split_label_prefix_lexed(&line.info.source_tokens).is_some()
            || split_label_prefix_lexed(&line.tokens).is_some())
    },
    // one complete quoted ability-gain sentence only
    |line| Ok(split_lexed_sentences(&line.tokens).len() != 1),
    // a permanent anthem owns its trailing quoted grants
    |line| {
        Ok(
            crate::keyword_static::parse_anthem_with_trailing_segments_line(&line.tokens)?
                .is_some(),
        )
    },
    // an enter-as-copy replacement owns its quoted exception
    |line| {
        Ok(
            crate::grammar::keyword_static_lines::parse_enter_as_copy_tokens(&line.tokens)
                .is_some(),
        )
    },
    // an attachment-subject `has` line is a continuous grant
    |line| {
        Ok(matches!(
            crate::keyword_static::parse_enchanted_creature_has_line(&line.tokens),
            Ok(Some(_))
        ))
    },
    // a conditioned grant list belongs to the conditional static family
    |line| {
        Ok(crate::word_primitives::sequence_occurs(
            &crate::lexer::parser_token_word_refs(&line.tokens),
            &["as", "long", "as"],
        ))
    },
];

#[inline(never)]
fn try_push_complete_typed_quoted_gain_statement(
    line: &PreprocessedLine,
    lines: &mut Vec<RecognizedLine>,
) -> Result<bool, CardTextError> {
    for declines in QUOTED_GAIN_DECLINES {
        if declines(line)? {
            return Ok(false);
        }
    }
    let first_quote = crate::slice_primitives::select_position(&line.tokens, |token| {
        token.kind == TokenKind::Quote
    });
    let has_outer_activation_colon = first_quote.is_some_and(|quote| {
        line.tokens[..quote]
            .iter()
            .any(|token| token.kind == TokenKind::Colon)
    });
    if super::grammar::effects::sentence_predicate_shapes::parse_quoted_ability_sentence_tokens(
        &line.tokens,
    )
    .is_none()
        || super::grammar::effects::emblem_shapes::parse_emblem_payload_tokens(&line.tokens)
            .is_some()
        || has_outer_activation_colon
        || line_starts_with_trigger_intro_tokens(&line.tokens)
        || labeled_body_starts_with_trigger_intro_tokens(&line.tokens)
    {
        return Ok(false);
    }
    let Some(parsed_effects) = crate::effect_sentences::parse_gain_ability_sentence(&line.tokens)?
    else {
        return Ok(false);
    };
    if parsed_effects.is_empty() {
        return Ok(false);
    }
    let recognized = RecognizedLine::Statement(RecognizedStatementLine {
        info: line.info.clone(),
        text: line.info.normalized.normalized.clone(),
        parse_tokens: line.tokens.clone(),
        parse_groups: vec![line.tokens.clone()],
        parsed_effects: Some(parsed_effects),
    });
    trace_recognized_line(&recognized);
    lines.push(recognized);
    Ok(true)
}

fn try_push_complete_typed_statement(
    line: &PreprocessedLine,
    lines: &mut Vec<RecognizedLine>,
) -> Result<bool, CardTextError> {
    if line_starts_with_trigger_intro_tokens(&line.tokens) {
        // Typed effect leaves may recognize verbs inside a trigger's result
        // clause. The complete line still belongs to triggered-line dispatch;
        // feeding the trigger header to a damage source parser turns it into
        // a bogus target phrase and prevents the proven comma split.
        return Ok(false);
    }
    if split_activation_text_tokens_lexed(&line.tokens)
        .is_some_and(|(cost_tokens, _)| looks_like_activation_cost_prefix(&cost_tokens))
    {
        return Ok(false);
    }
    // The early typed-statement front door runs before the ordinary
    // static-vs-statement registry.  A composable effect parser can also
    // understand permanent anthem text, but that must remain a battlefield
    // static ability rather than a one-shot resolution program.
    let typed_persistent_anthem =
        crate::keyword_static::parse_anthem_with_trailing_segments_line(&line.tokens)?.is_some()
            || super::grammar::anthem_grants::parse_anthem_modifier_head(&line.tokens)
                .is_some_and(|head| !head.has_target && !head.temporary);
    if typed_persistent_anthem && matches!(recognize_static_line(line), Ok(Some(_))) {
        return Ok(false);
    }
    let authored_sentence_count = split_lexed_sentences(&line.tokens).len();
    let parsed_effects = if authored_sentence_count == 1
        && let Some(effects) =
            crate::effect_sentences::parse_complete_each_player_return_with_additional_counter(
                &line.tokens,
            )? {
        Some(effects)
    } else if authored_sentence_count == 1
        && let Some(effects) =
            crate::effect_sentences::parse_complete_each_player_reveal_partition(&line.tokens)?
    {
        Some(effects)
    } else if let Some(effects) =
        crate::effect_sentences::parse_choose_target_prelude_sentence(&line.tokens)?
    {
        Some(effects)
    } else if authored_sentence_count == 1
        && let Some(effect) =
            crate::effect_sentences::parse_deal_damage_equal_to_power_clause(&line.tokens)?
    {
        Some(vec![effect])
    } else if let Some(effects) =
        crate::effect_sentences::parse_complete_create_statement(&line.tokens)?
    {
        Some(effects)
    } else if let Some(effects) =
        crate::effect_sentences::parse_complete_investigate_statement(&line.tokens)?
    {
        Some(effects)
    } else if let Some(effects) =
        crate::effect_sentences::parse_complete_kicked_search_replacement_bundle(&line.tokens)?
    {
        Some(effects)
    } else if let Some(effects) =
        crate::effect_sentences::parse_complete_delegated_partition_program(&line.tokens)
    {
        Some(effects)
    } else {
        crate::effect_sentences::parse_complete_composable_fight_program(&line.tokens)?
    };
    let Some(parsed_effects) = parsed_effects else {
        return Ok(false);
    };
    let recognized = RecognizedLine::Statement(RecognizedStatementLine {
        info: line.info.clone(),
        text: line.info.normalized.normalized.clone(),
        parse_tokens: line.tokens.clone(),
        parse_groups: vec![line.tokens.clone()],
        parsed_effects: Some(parsed_effects),
    });
    trace_recognized_line(&recognized);
    lines.push(recognized);
    Ok(true)
}

fn try_push_named_source_gain_statement(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    lines: &mut Vec<RecognizedLine>,
) -> Result<Option<usize>, CardTextError> {
    let Some(rewritten_line) = rewrite_named_source_gain_line(preprocessed, line)? else {
        return Ok(None);
    };
    let Some(mut statement) = recognize_source_gain_ability_statement_boxed(&rewritten_line)?
    else {
        return Ok(None);
    };
    let next_idx = extend_statement_line_with_result_followups_in_place(
        &preprocessed.items,
        idx,
        &mut statement,
    );
    push_boxed_recognized_statement(lines, statement);
    Ok(Some(next_idx))
}

fn push_boxed_recognized_statement(
    lines: &mut Vec<RecognizedLine>,
    statement: Box<RecognizedStatementLine>,
) {
    let recognized = RecognizedLine::Statement(*statement);
    trace_recognized_line(&recognized);
    lines.push(recognized);
}

fn rewrite_named_source_gain_line(
    preprocessed: &PreprocessedDocument,
    line: &PreprocessedLine,
) -> Result<Option<Box<PreprocessedLine>>, CardTextError> {
    if line_starts_with_trigger_intro_tokens(&line.tokens)
        || labeled_body_starts_with_trigger_intro_tokens(&line.tokens)
        || line_family_grammar::parse_champion_line(&line.tokens).is_some()
        || !tokens_mention_source_alias(&preprocessed.card, &line.tokens)
    {
        return Ok(None);
    }
    let Some(rewritten) = normalize_named_source_sentence_tokens(&preprocessed.card, &line.tokens)
    else {
        return Ok(None);
    };
    let rewritten_line = rewrite_line_tokens(line, &rewritten);
    if super::grammar::effects::gain_ability_shapes::parse_source_gain_ability_shape(
        &rewritten_line.tokens,
    )
    .is_none()
    {
        return Ok(None);
    }
    Ok(Some(Box::new(rewritten_line)))
}

fn try_push_named_source_dispatch(
    line_context: ParseContextView<'_>,
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
    lines: &mut Vec<RecognizedLine>,
) -> Result<Option<usize>, CardTextError> {
    if line_starts_with_trigger_intro_tokens(&line.tokens)
        || labeled_body_starts_with_trigger_intro_tokens(&line.tokens)
        || line_family_grammar::parse_champion_line(&line.tokens).is_some()
        || !tokens_mention_source_alias(&preprocessed.card, &line.tokens)
    {
        return Ok(None);
    }
    let Some(rewritten) = normalize_named_source_sentence_tokens(&preprocessed.card, &line.tokens)
    else {
        return Ok(None);
    };
    let rewritten_line = rewrite_line_tokens(line, &rewritten);
    let Ok(dispatch) = dispatch_standard_line(
        line_context,
        preprocessed,
        idx,
        &rewritten_line,
        allow_unsupported,
    ) else {
        return Ok(None);
    };
    for recognized in &dispatch.lines {
        trace_recognized_line(recognized);
    }
    let next_idx = dispatch.next_idx;
    lines.extend(dispatch.lines);
    Ok(Some(next_idx))
}

fn push_standard_dispatch(
    line_context: ParseContextView<'_>,
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
    lines: &mut Vec<RecognizedLine>,
) -> Result<usize, CardTextError> {
    let dispatch = if is_bullet_line(line)
        && split_label_prefix_lexed(strip_choice_bullet_prefix_tokens(&line.tokens)).is_some()
    {
        let stripped_line =
            rewrite_line_tokens(line, strip_choice_bullet_prefix_tokens(&line.tokens));
        dispatch_standard_line(
            line_context,
            preprocessed,
            idx,
            &stripped_line,
            allow_unsupported,
        )?
    } else {
        dispatch_standard_line(line_context, preprocessed, idx, line, allow_unsupported)?
    };
    if try_merge_labeled_prior_token_replacement_statement(lines, &dispatch) {
        parse_trace::event("joined labeled prior-token replacement to preceding statement");
        return Ok(dispatch.next_idx);
    }
    for recognized in &dispatch.lines {
        trace_recognized_line(recognized);
    }
    let next_idx = dispatch.next_idx;
    lines.extend(dispatch.lines);
    Ok(next_idx)
}

/// Parse a prepared document at the public grammar boundary with fresh,
/// request-scoped compiler state.
#[inline(never)]
pub fn recognize_document(
    preprocessed: &PreprocessedDocument,
    allow_unsupported: bool,
) -> Result<RecognizedDocument, CardTextError> {
    // A card is the unit of recognition; nothing parsed for the last one is
    // relevant to this one.
    crate::sentence_memo::reset();
    let source_text = preprocessed
        .items
        .iter()
        .map(|item| match item {
            PreprocessedItem::Metadata(line) => line.info.raw_line.as_str(),
            PreprocessedItem::Line(line) => line.info.raw_line.as_str(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let context =
        crate::parse_context_for_builder(&preprocessed.card, &source_text, allow_unsupported);
    recognize_document_with_context(context.view().child(ParseScopeKind::Document), preprocessed)
}

fn assemble_document(
    preprocessed: PreprocessedDocument,
    recognized: RecognizedDocument,
    cleave_recognized: Option<RecognizedDocument>,
    semantic_facts: super::ir::DocumentSemanticFacts,
    allow_unsupported: bool,
) -> Result<RewriteSemanticDocument, CardTextError> {
    assemble_document_with_symbols(
        preprocessed,
        recognized,
        cleave_recognized,
        semantic_facts,
        allow_unsupported,
        crate::model::symbols::SymbolTable::default(),
    )
}

fn assemble_document_with_symbols(
    preprocessed: PreprocessedDocument,
    recognized: RecognizedDocument,
    cleave_recognized: Option<RecognizedDocument>,
    semantic_facts: super::ir::DocumentSemanticFacts,
    allow_unsupported: bool,
    symbols: crate::model::symbols::SymbolTable,
) -> Result<RewriteSemanticDocument, CardTextError> {
    // Assembly parses the recognized lines' effects: keys minted for a line bind
    // in the scope recognition opened for it.
    let symbols = std::cell::RefCell::new(symbols);
    let overload_items = semantic_facts
        .overload_rewrite
        .as_ref()
        .map(|payload| {
            let mut items = Vec::new();
            for line in &recognized.lines {
                let Some(line) = rewrite_overload_recognized_line(line, payload) else {
                    continue;
                };
                let _references = line_reference_scope(&symbols, &line);
                items.push(assemble_non_metadata_line(line, allow_unsupported)?);
            }
            Ok::<_, CardTextError>(items)
        })
        .transpose()?;
    let cleave_items = semantic_facts
        .cleave_rewrite
        .as_ref()
        .zip(cleave_recognized.as_ref())
        .map(|(payload, cleave_recognized)| {
            let mut items = Vec::new();
            for line in &cleave_recognized.lines {
                if recognized_line_source_index(line) == Some(payload.keyword_line_index)
                    || matches!(line, RecognizedLine::Metadata(_))
                {
                    continue;
                }
                let _references = line_reference_scope(&symbols, line);
                items.push(assemble_non_metadata_line(line.clone(), allow_unsupported)?);
            }
            Ok::<_, CardTextError>(items)
        })
        .transpose()?;

    let mut card = preprocessed.card;
    let mut annotations = preprocessed.annotations;
    let mut items = Vec::with_capacity(recognized.lines.len());

    for line in recognized.lines {
        if let RecognizedLine::Statement(statement) = &line {
            let normalized = statement.info.normalized.normalized.as_str();
            if annotations
                .normalized_lines
                .get(&statement.info.line_index)
                .is_none_or(|recorded| recorded != normalized)
            {
                annotations
                    .normalized_lines
                    .insert(statement.info.line_index, normalized.to_string());
                annotations.normalized_char_maps.insert(
                    statement.info.line_index,
                    statement.info.normalized.char_map.clone(),
                );
            }
        }
        match line {
            RecognizedLine::Metadata(RecognizedMetadataLine { value }) => {
                card = crate::card_metadata::apply_compiler_metadata_line(card, value)?;
                items.push(RewriteSemanticItem::Metadata);
            }
            other => {
                let _references = line_reference_scope(&symbols, &other);
                items.push(assemble_non_metadata_line(other, allow_unsupported)?);
            }
        }
    }

    Ok(RewriteSemanticDocument {
        card,
        annotations,
        provenance: preprocessed.provenance,
        symbols: symbols.into_inner(),
        items,
        overload_items,
        cleave_items,
        allow_unsupported,
    })
}

fn rewrite_overload_target_tokens(
    tokens: &[OwnedLexToken],
    payload: &OverloadRewritePayload,
) -> Vec<OwnedLexToken> {
    tokens
        .iter()
        .map(|token| {
            if payload
                .target_spans
                .iter()
                .any(|target_span| target_span == &token.span)
            {
                OwnedLexToken::word("each", token.span)
            } else {
                token.clone()
            }
        })
        .collect()
}

fn rewrite_overload_recognized_line(
    line: &RecognizedLine,
    payload: &OverloadRewritePayload,
) -> Option<RecognizedLine> {
    if recognized_line_source_index(line) == Some(payload.keyword_line_index) {
        return None;
    }
    match line {
        RecognizedLine::Metadata(_) => None,
        RecognizedLine::Statement(statement) => {
            let mut statement = statement.clone();
            statement.parse_tokens =
                rewrite_overload_target_tokens(&statement.parse_tokens, payload);
            statement.parse_groups = statement
                .parse_groups
                .iter()
                .map(|group| rewrite_overload_target_tokens(group, payload))
                .collect();
            statement.text = render_token_slice(&statement.parse_tokens)
                .trim()
                .to_string();
            statement.info.normalized.normalized = statement.text.clone();
            statement.info.semantic_facts =
                super::grammar::line_semantic_facts::parse_line_semantic_facts_tokens(
                    &statement.parse_tokens,
                );
            // `parsed_effects` belongs to the original target-bearing token
            // stream. Force semantic assembly to lower the rewritten `each`
            // clauses instead of reusing that stale pre-rewrite AST.
            statement.parsed_effects = None;
            Some(RecognizedLine::Statement(statement))
        }
        other => Some(other.clone()),
    }
}

/// The reference scope of the line `line` was recognized on, when recognition
/// opened one: keys minted while the guard lives bind there.
fn line_reference_scope<'a>(
    symbols: &'a std::cell::RefCell<crate::model::symbols::SymbolTable>,
    line: &RecognizedLine,
) -> Option<ironsmith_compiler_ast::reference_ledger::ReferenceScopeGuard<'a>> {
    let scope = symbols
        .borrow()
        .line_scope(recognized_line_source_index(line)?)?;
    Some(ironsmith_compiler_ast::reference_ledger::ReferenceScopeGuard::enter(symbols, scope))
}

fn recognized_line_source_index(line: &RecognizedLine) -> Option<usize> {
    match line {
        RecognizedLine::Metadata(_) => None,
        RecognizedLine::Keyword(line) => Some(line.info.display_line_index),
        RecognizedLine::Activated(line) => Some(line.info.display_line_index),
        RecognizedLine::Triggered(line) => Some(line.info.display_line_index),
        RecognizedLine::Static(line) => Some(line.info.display_line_index),
        RecognizedLine::Statement(line) => Some(line.info.display_line_index),
        RecognizedLine::Modal(line) => Some(line.header.display_line_index),
        RecognizedLine::LevelHeader(line) => {
            line.items.first().map(|item| item.info.display_line_index)
        }
        RecognizedLine::SagaChapter(line) => Some(line.info.display_line_index),
        RecognizedLine::Unsupported(line) => Some(line.info.display_line_index),
    }
}

pub fn recognize_metadata_line(
    info: crate::cards::builders::LineInfo,
    value: crate::cards::builders::MetadataLine,
) -> Result<RecognizedMetadataLine, CardTextError> {
    let _ = info;
    Ok(RecognizedMetadataLine { value })
}

// ---- test-only adapters: the string normalizers these tests were written
// against are gone; the token twins run over lexed text and render lowercase,
// which is what the string forms returned.

#[cfg(test)]
fn normalize_named_source_trigger_for_builder(
    card: &crate::card::CardBuilder,
    text: &str,
) -> Option<String> {
    let tokens = lex_line(text.trim(), 0).ok()?;
    normalize_named_source_trigger_tokens(card, &tokens)
        .map(|tokens| render_token_slice(&tokens).to_ascii_lowercase())
}

#[cfg(test)]
fn normalize_named_source_sentence_for_builder(
    card: &crate::card::CardBuilder,
    text: &str,
) -> Option<String> {
    let tokens = lex_line(text.trim(), 0).ok()?;
    normalize_named_source_sentence_tokens(card, &tokens)
        .map(|tokens| render_token_slice(&tokens).to_ascii_lowercase())
}

#[cfg(test)]
fn test_alias_words(alias: &str) -> Vec<String> {
    lex_line(alias.trim(), 0)
        .map(|tokens| TokenWordView::new(&tokens).owned_words())
        .unwrap_or_default()
}

#[cfg(test)]
fn replace_named_source_aliases(text: &str, alias: &str, replacement: &str) -> String {
    replace_named_source_aliases_from_set(text, alias, replacement, &[], true)
}

#[cfg(test)]
fn replace_named_source_aliases_from_set(
    text: &str,
    alias: &str,
    replacement: &str,
    all_aliases: &[String],
    preserve_surface_hints: bool,
) -> String {
    let lower = text.to_ascii_lowercase();
    let Ok(tokens) = lex_line(lower.as_str(), 0) else {
        return lower;
    };
    let all_words: Vec<Vec<String>> = all_aliases.iter().map(|a| test_alias_words(a)).collect();
    replace_named_source_alias_tokens(
        &tokens,
        &test_alias_words(alias),
        replacement,
        &all_words,
        preserve_surface_hints,
    )
    .map(|tokens| render_token_slice(&tokens).to_ascii_lowercase())
    .unwrap_or(lower)
}

#[cfg(test)]
fn rewrite_line_normalized(
    line: &PreprocessedLine,
    normalized: &str,
) -> Result<PreprocessedLine, CardTextError> {
    let tokens = lex_line(normalized, line.info.line_index)?;
    Ok(rewrite_line_tokens(line, &tokens))
}

#[cfg(test)]
mod tests {
    use crate::cards::builders::TurnEventPredicateAst;
    use crate::cards::builders::SourcePredicateAst;
    use crate::cards::builders::KeywordActionAst;
    use crate::ability::PresentationLabel;
    use crate::cards::builders::CardTextError;
    use crate::cards::builders::document_parser::KeywordLineKind;
    use crate::ids::CardId;
    use crate::types::{CardType, Subtype};
    use ironsmith_compiler::ParseCardText;
    use ironsmith_compiler_lowering::CardDefinitionBuilder;
    use ironsmith_core::card::CardBuilder;

    use super::super::grammar::structure::{
        StatementLineFamily, StaticLineFamily, classify_statement_line_family_lexed,
        classify_static_line_family_lexed,
    };
    use super::{
        PreprocessedItem, RecognizedLine, TriggeredSplitProbe, classify_unsupported_line_reason,
        diagnose_known_unsupported_rewrite_line, is_bullet_line,
        is_doesnt_untap_during_your_untap_step_line_lexed, is_if_you_do_exile_followup_tokens,
        is_land_reveal_enters_static_line_lexed, is_land_reveal_enters_tapped_followup_line_lexed,
        is_opening_hand_begin_game_static_line_lexed, is_ward_or_echo_static_prefix_line_lexed,
        lex_line, looks_like_statement_line, looks_like_statement_line_lexed,
        looks_like_static_line, looks_like_static_line_lexed,
        normalize_named_source_sentence_for_builder, normalize_named_source_trigger_for_builder,
        normalize_statement_parse_groups_lexed,
        normalize_trailing_keyword_activation_sentence_lexed,
        parse_colon_nonactivation_statement_fallback, preprocess_document, probe_triggered_split,
        recognize_keyword_line, recognize_labeled_qualified_ability_trigger, recognize_level_item,
        recognize_statement_line, recognize_static_line, recognize_triggered_line,
        render_token_slice, replace_named_source_aliases, replace_named_source_aliases_from_set,
        rewrite_keyword_dash_parse_tokens, rewrite_when_one_or_more_this_way_line,
        sentence_is_static_after_trigger_effect, should_parse_delayed_trigger_line_as_spell_effect,
        source_name_aliases_for_builder, split_activation_text_parts_lexed, split_label_prefix,
        split_label_prefix_lexed, split_reveal_first_draw_line_rewrite_lexed,
        split_trigger_sentence_chunks_rewrite_lexed, strip_non_keyword_label_prefix,
        strip_trailing_trigger_cap_suffix_tokens, tokens_after_non_keyword_label_prefix,
        trigger_presentation_from_line_tokens,
        triggered_effect_tokens_have_trailing_static_sentences,
        try_parse_triggered_line_with_named_source_rewrite,
    };

    fn parse_text_to_semantic_document(
        card: CardBuilder,
        text: String,
        allow_unsupported: bool,
    ) -> Result<
        (
            crate::ir::RewriteSemanticDocument,
            crate::cards::ParseAnnotations,
        ),
        CardTextError,
    > {
        let mut context = crate::parse_context_for_builder(&card, &text, allow_unsupported);
        super::parse_text_to_semantic_document_with_context(&mut context, card, text)
    }

    fn single_preprocessed_line(text: &str) -> super::PreprocessedLine {
        let document = preprocess_document(
            CardBuilder::new(CardId::new(), "Document Parser Test")
                .card_types(vec![CardType::Creature]),
            text,
        )
        .expect("expected preprocess_document to keep test line");
        match document
            .items
            .into_iter()
            .next()
            .expect("expected one preprocessed item")
        {
            PreprocessedItem::Line(line) => line,
            other => panic!("expected preprocessed line, got {other:?}"),
        }
    }

    #[test]
    fn document_statement_retains_complete_correlated_fight_program() -> Result<(), CardTextError> {
        let text = "Choose two target creatures that share no creature types. Those creatures fight each other.";
        let card =
            CardBuilder::new(CardId::new(), "Correlated Fight").card_types(vec![CardType::Sorcery]);
        let preprocessed = preprocess_document(card, text)?;
        let recognized = super::recognize_document(&preprocessed, false)?;
        let [RecognizedLine::Statement(statement)] = recognized.lines.as_slice() else {
            panic!("expected one statement: {recognized:#?}");
        };
        let effects = statement
            .parsed_effects
            .as_deref()
            .expect("typed statement effects");

        let [
            crate::cards::builders::EffectAst::SourceSentence {
                effects: target_effects,
                ..
            },
            crate::cards::builders::EffectAst::SourceSentence {
                effects: fight_effects,
                ..
            },
        ] = effects
        else {
            panic!("expected two source-sentence effects: {effects:#?}");
        };
        assert_eq!(target_effects.len(), 1, "{target_effects:#?}");
        assert!(
            matches!(
                fight_effects.as_slice(),
                [crate::cards::builders::EffectAst::SubjectVerb(
                    crate::cards::builders::SubjectVerbEffectAst {
                        action: crate::cards::builders::SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fight { .. }),
                        ..
                    }
                )]
            ),
            "{fight_effects:#?}"
        );
        Ok(())
    }

    #[test]
    fn carried_conditional_equipment_anthem_survives_document_preprocessing() {
        let text = "Equipped creature gets +2/+0. It gets an additional +0/+2 and has first strike as long as an Equipment named Groom's Finery is attached to a creature you control.";
        let document = preprocess_document(
            CardBuilder::new(CardId::new(), "Bride's Gown").card_types(vec![CardType::Artifact]),
            text,
        )
        .expect("preprocess carried conditional anthem");
        let Some(PreprocessedItem::Line(line)) = document.items.first() else {
            panic!("expected carried conditional anthem line");
        };
        let parsed = recognize_static_line(line)
            .unwrap_or_else(|error| panic!("tokens={:#?}; error={error:?}", line.tokens));
        assert!(
            parsed.is_some(),
            "expected static line after preprocessing; tokens={:#?}",
            line.tokens
        );
    }

    #[test]
    fn bullet_line_detection_uses_tokens_and_rejects_negative_numbers() {
        assert!(is_bullet_line(&single_preprocessed_line("- choose one")));
        assert!(is_bullet_line(&single_preprocessed_line("  • choose one")));
        assert!(is_bullet_line(&single_preprocessed_line("* choose one")));
        assert!(!is_bullet_line(&single_preprocessed_line(
            "-1/-1 until end of turn"
        )));
    }

    #[test]
    fn activation_cost_prefix_detection_uses_lexed_tokens() {
        for text in [
            "{T}, Tap an untapped Ally you control",
            "+1",
            "-2",
            "Tap this creature",
        ] {
            let tokens = lex_line(text, 0).expect("expected activation cost probe to lex");
            assert!(
                super::looks_like_activation_cost_prefix(&tokens),
                "expected {text:?} to look like an activation cost prefix"
            );
        }
        let tokens =
            lex_line("target opponent loses 2 life", 0).expect("expected non-cost probe to lex");
        assert!(!super::looks_like_activation_cost_prefix(&tokens));
    }

    #[test]
    fn strip_non_keyword_label_prefix_removes_chained_mode_name_and_cost() {
        assert_eq!(
            strip_non_keyword_label_prefix(
                "Meteor Strikes — {2} — Double target creature's power and toughness until end of turn."
            ),
            "Double target creature's power and toughness until end of turn."
        );
        assert_eq!(
            strip_non_keyword_label_prefix(
                "Final Heaven — {6}{G} — Triple target creature's power and toughness until end of turn."
            ),
            "Triple target creature's power and toughness until end of turn."
        );
    }

    #[test]
    fn split_label_prefix_lexed_reuses_existing_body_tokens() {
        let tokens = lex_line(
            "Secret Council — Each player votes for death or torture.",
            0,
        )
        .expect("rewrite lexer should classify labeled line");

        let (label, label_tokens, body_tokens) =
            split_label_prefix_lexed(&tokens).expect("expected token label prefix split");

        assert_eq!(label, "Secret Council");
        assert_eq!(render_token_slice(label_tokens), "Secret Council");
        assert_eq!(
            render_token_slice(body_tokens),
            "Each player votes for death or torture."
        );
    }

    #[test]
    fn split_label_prefix_lexed_handles_councils_dilemma_possessive_label() {
        let tokens = lex_line(
            "Council's dilemma — Whenever Tivit enters, each player votes for evidence or bribery.",
            0,
        )
        .expect("rewrite lexer should classify possessive labeled line");

        let (label, _, body_tokens) =
            split_label_prefix_lexed(&tokens).expect("expected token label prefix split");

        assert_eq!(label, "Council's dilemma");
        assert_eq!(
            render_token_slice(body_tokens),
            "Whenever Tivit enters, each player votes for evidence or bribery."
        );
    }

    #[test]
    fn tokens_after_non_keyword_label_prefix_reuses_chained_body_tokens() {
        let line = single_preprocessed_line(
            "Meteor Strikes — {2} — Double target creature's power and toughness until end of turn.",
        );

        let tokens = tokens_after_non_keyword_label_prefix(&line)
            .expect("expected chained non-keyword label prefix to strip");

        assert_eq!(
            render_token_slice(tokens),
            "double target creature's power and toughness until end of turn."
        );
    }

    #[test]
    fn rewrite_keyword_dash_parse_tokens_drops_council_label_body_only() {
        let tokens = lex_line(
            "secret council — each player votes for death or torture.",
            0,
        )
        .expect("rewrite lexer should classify council label line");

        let rewritten = rewrite_keyword_dash_parse_tokens(&tokens);

        assert_eq!(
            render_token_slice(&rewritten),
            "each player votes for death or torture."
        );
    }

    #[test]
    fn rewrite_keyword_dash_parse_tokens_keeps_keyword_label_without_dash() {
        let tokens = lex_line("cycling — {2}, discard this card: draw a card.", 0)
            .expect("rewrite lexer should classify keyword label line");

        let rewritten = rewrite_keyword_dash_parse_tokens(&tokens);

        assert_eq!(
            render_token_slice(&rewritten),
            "cycling {2}, discard this card: draw a card."
        );
    }

    #[test]
    fn strip_trailing_trigger_cap_suffix_tokens_supports_do_this_only_once_each_turn() {
        let tokens = lex_line(
            "Whenever one or more lands enter under an opponent's control without being played, you may search your library for a Plains card, put it onto the battlefield tapped, then shuffle. Do this only once each turn.",
            0,
        )
        .expect("rewrite lexer should classify capped trigger line");

        let (stripped, max_triggers_per_turn) = strip_trailing_trigger_cap_suffix_tokens(&tokens);

        assert_eq!(max_triggers_per_turn, Some(1));
        assert_eq!(
            render_token_slice(stripped),
            "Whenever one or more lands enter under an opponent's control without being played, you may search your library for a Plains card, put it onto the battlefield tapped, then shuffle"
        );

        let tokens = lex_line(
            "Whenever you commit a crime, create a 1/1 red Mercenary creature token with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\" This ability triggers only once each turn.",
            0,
        )
        .expect("lex quoted token trigger cap");
        let (stripped, max_triggers_per_turn) = strip_trailing_trigger_cap_suffix_tokens(&tokens);
        assert_eq!(max_triggers_per_turn, Some(1));
        assert!(render_token_slice(stripped).contains("as a sorcery"));
    }

    #[test]
    fn keyword_line_recognized_stores_rewritten_parse_tokens() -> Result<(), CardTextError> {
        let line = single_preprocessed_line("Cycling — {2}, Discard this card: Draw a card.");

        let parsed =
            recognize_keyword_line(&line)?.expect("expected cycling line to parse as keyword");

        assert_eq!(
            render_token_slice(&parsed.parse_tokens),
            "cycling {2}, discard this card: draw a card."
        );

        Ok(())
    }

    #[test]
    fn morph_life_keyword_line_with_reminder_parses_as_keyword() -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "Morph—Pay 5 life. (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)",
        );

        let parsed = recognize_keyword_line(&line)?
            .expect("expected Zombie Cutthroat line to parse as keyword");

        assert_eq!(parsed.kind, KeywordLineKind::Morph);
        Ok(())
    }

    #[test]
    fn parse_document_recognized_keeps_morph_dash_keyword_out_of_labeled_line_fallback()
    -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Zombie Cutthroat")
                .card_types(vec![CardType::Creature]),
            "Morph—Pay 5 life. (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        match recognized.lines.as_slice() {
            [super::RecognizedLine::Keyword(keyword)] => {
                assert_eq!(keyword.kind, KeywordLineKind::Morph);
                assert_eq!(
                    render_token_slice(&keyword.parse_tokens),
                    "morph pay 5 life."
                );
            }
            other => panic!("expected one morph keyword line, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn ward_and_echo_static_prefixes_are_token_classified() {
        let ward = lex_line("Ward — Pay 3 life.", 0)
            .expect("rewrite lexer should classify ward static prefix");
        let echo =
            lex_line("Echo {2}{R}", 0).expect("rewrite lexer should classify echo static prefix");

        assert!(is_ward_or_echo_static_prefix_line_lexed(&ward));
        assert!(is_ward_or_echo_static_prefix_line_lexed(&echo));
    }

    #[test]
    fn land_reveal_combined_static_pair_is_token_classified() {
        let first = lex_line(
            "As this land enters, you may reveal an Island card from your hand.",
            0,
        )
        .expect("rewrite lexer should classify first static line");
        let second = lex_line("If you don't, it enters tapped.", 0)
            .expect("rewrite lexer should classify followup static line");

        assert!(is_land_reveal_enters_static_line_lexed(&first));
        assert!(is_land_reveal_enters_tapped_followup_line_lexed(&second));
    }

    #[test]
    fn opening_hand_begin_game_combined_static_pair_is_token_classified() {
        let first = lex_line(
            "If this card is in your opening hand, you may begin the game with it on the battlefield.",
            0,
        )
        .expect("rewrite lexer should classify opening-hand static line");
        let second = lex_line("If you do exile a card from your hand.", 0)
            .expect("rewrite lexer should classify if-you-do followup line");

        assert!(is_opening_hand_begin_game_static_line_lexed(&first));
        assert!(is_if_you_do_exile_followup_tokens(&second));
    }

    #[test]
    fn parse_document_recognized_merges_numeric_result_followups_into_triggered_line()
    -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Aberrant Mind Sorcerer")
                .card_types(vec![CardType::Creature]),
            "Psionic Spells — When this creature enters, choose target instant or sorcery card in your graveyard, then roll a d20.\n1—9 | You may put that card on top of your library.\n10—20 | Return that card to your hand.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        match recognized.lines.as_slice() {
            [super::RecognizedLine::Triggered(triggered)] => {
                let effect_text = render_token_slice(&triggered.effect_parse_tokens);
                assert!(
                    effect_text.contains("roll a d20"),
                    "expected initial roll clause in triggered effect text, got {:?}",
                    effect_text
                );
                assert!(
                    effect_text.contains("1—9") && effect_text.contains("10—20"),
                    "expected numeric result followups to merge into triggered line, got {:?}",
                    effect_text
                );
            }
            other => panic!("expected one merged triggered line, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn parse_document_recognized_merges_exact_numeric_result_with_inner_label_into_activation()
    -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Result Table Probe")
                .card_types(vec![CardType::Artifact]),
            "{4}, Sacrifice this artifact: Roll a d20.\n1 | Trapped! — You lose 3 life.\n2—9 | Create five Treasure tokens.\n10—20 | Draw a card.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        let [super::RecognizedLine::Activated(activated)] = recognized.lines.as_slice() else {
            panic!(
                "expected one merged activated result table, got {:#?}",
                recognized.lines
            );
        };
        let effect_text = render_token_slice(&activated.effect_parse_tokens);
        assert!(effect_text.contains("roll a d20"), "{effect_text}");
        assert!(effect_text.contains("1 | trapped!"), "{effect_text}");
        assert!(effect_text.contains("2—9"), "{effect_text}");
        assert!(effect_text.contains("10—20"), "{effect_text}");

        Ok(())
    }

    #[test]
    fn parse_document_recognized_merges_plural_animation_result_into_statement()
    -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Plural Return Probe"),
            "Return up to one target artifact card and up to one target land card from your graveyard to the battlefield.\nThey are 5/5 Elemental creatures in addition to their other types.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        let [super::RecognizedLine::Statement(statement)] = recognized.lines.as_slice() else {
            panic!(
                "expected one merged statement line, got {:#?}",
                recognized.lines
            );
        };
        let parse_text = render_token_slice(&statement.parse_tokens);
        assert!(
            parse_text.contains("they are 5/5 elemental creatures"),
            "{parse_text}"
        );
        assert_eq!(
            statement.parse_groups.len(),
            1,
            "{:#?}",
            statement.parse_groups
        );

        Ok(())
    }

    #[test]
    fn keyword_line_recognized_recognizes_gift_family_from_tokens() -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "Gift a card (You may promise an opponent a gift as you cast this spell. If you do, they draw a card before its other effects.)",
        );

        let parsed =
            recognize_keyword_line(&line)?.expect("expected gift line to parse as keyword");

        assert!(matches!(parsed.kind, super::KeywordLineKind::Gift));

        Ok(())
    }

    #[test]
    fn keyword_line_recognized_recognizes_exert_attack_from_tokens() -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "You may exert this creature as it attacks. (An exerted creature won't untap during your next untap step.)",
        );

        let parsed =
            recognize_keyword_line(&line)?.expect("expected exert line to parse as keyword");

        assert!(matches!(parsed.kind, super::KeywordLineKind::ExertAttack));

        Ok(())
    }

    #[test]
    fn parse_document_recognized_rewrites_surge_keyword_line_to_alternative_cost()
    -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Surge Parse Test").card_types(vec![CardType::Sorcery]),
            "Surge {3}{U}{U} (You may cast this spell for its surge cost if you or a teammate has cast another spell this turn.)\nReturn all nonland permanents to their owners' hands.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        match recognized.lines.as_slice() {
            [
                super::RecognizedLine::Keyword(keyword),
                super::RecognizedLine::Statement(_),
            ] => {
                assert_eq!(keyword.kind, KeywordLineKind::AlternativeCast);
                assert_eq!(
                    render_token_slice(&keyword.parse_tokens),
                    "If you've cast another spell this turn, you may pay {3}{U}{U} rather than pay this spell's mana cost."
                );
            }
            other => panic!("expected surge keyword plus statement line, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn parse_document_recognized_rewrites_freerunning_keyword_line_to_alternative_cost()
    -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Freerunning Parse Test")
                .card_types(vec![CardType::Sorcery]),
            "Freerunning {2}{R} (You may cast this spell for its freerunning cost if you dealt combat damage to a player this turn with an Assassin or commander.)\nUntap all creatures you control that attacked this turn.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        match recognized.lines.as_slice() {
            [
                super::RecognizedLine::Keyword(keyword),
                super::RecognizedLine::Statement(_),
            ] => {
                assert_eq!(keyword.kind, KeywordLineKind::AlternativeCast);
                assert_eq!(
                    render_token_slice(&keyword.parse_tokens),
                    "If you dealt combat damage to a player this turn with an Assassin or commander, you may pay {2}{R} rather than pay this spell's mana cost."
                );
            }
            other => panic!("expected freerunning keyword plus statement line, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn parse_document_recognized_recognizes_sneak_keyword_line() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Sneak Parse Test").card_types(vec![CardType::Sorcery]),
            "Sneak {1}{B} (You may cast this spell for {1}{B} if you also return an unblocked attacker you control to hand during the declare blockers step.)\nSearch your library for a card, put that card into your hand, then shuffle.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        match recognized.lines.as_slice() {
            [
                super::RecognizedLine::Keyword(keyword),
                super::RecognizedLine::Statement(_),
            ] => {
                assert_eq!(keyword.kind, KeywordLineKind::AlternativeCast);
                assert_eq!(render_token_slice(&keyword.parse_tokens), "sneak {1}{b}");
                assert!(
                    render_token_slice(&keyword.full_parse_tokens)
                        .to_ascii_lowercase()
                        .contains("you may cast this spell for"),
                    "full Sneak parse tokens should retain reminder text"
                );
            }
            other => panic!("expected sneak keyword plus statement line, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn static_line_recognized_recognizes_compound_unblockable_from_tokens()
    -> Result<(), CardTextError> {
        let line = single_preprocessed_line("Enchanted creature gets +2/+2 and can't be blocked.");

        assert!(recognize_static_line(&line)?.is_some());

        Ok(())
    }

    #[test]
    fn document_recognizes_quoted_equipped_activation_as_static() -> Result<(), CardTextError> {
        let text = "Equipped creature has \"{2}: This creature gets +1/+0 until end of turn.\"";
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Quoted Equipment Test")
                .card_types(vec![CardType::Artifact])
                .subtypes(vec![Subtype::Equipment]),
            text,
        )?;
        let Some(PreprocessedItem::Line(line)) = preprocessed.items.first() else {
            panic!("expected one preprocessed line: {preprocessed:#?}");
        };
        assert!(
            recognize_static_line(line)?.is_some(),
            "preprocessed static recognition failed: {line:#?}"
        );
        let (recognized, trace) =
            crate::parse_trace::capture(|| super::recognize_document(&preprocessed, false));
        let recognized = recognized.unwrap_or_else(|error| {
            panic!(
                "quoted static document recognition failed: {error:?}\n{}",
                trace.render()
            )
        });
        assert!(
            matches!(
                recognized.lines.as_slice(),
                [super::RecognizedLine::Static(_)]
            ),
            "expected one static document line: {recognized:#?}"
        );
        Ok(())
    }

    #[test]
    fn document_keeps_quoted_token_creation_inside_loyalty_activation() -> Result<(), CardTextError>
    {
        let text = "+1: Create a colorless artifact token named Etherium Cell with \"{T}, Sacrifice this token: Add one mana of any color.\"";
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Quoted Loyalty Activation Test")
                .card_types(vec![CardType::Planeswalker]),
            text,
        )?;

        let recognized = super::recognize_document(&preprocessed, false)?;
        match recognized.lines.as_slice() {
            [super::RecognizedLine::Activated(activated)] => assert!(
                render_token_slice(&activated.effect_parse_tokens)
                    .to_ascii_lowercase()
                    .starts_with("create a colorless artifact token"),
                "unexpected activation effect: {activated:#?}"
            ),
            other => panic!("expected one loyalty activation, got {other:#?}"),
        }

        Ok(())
    }

    #[test]
    fn statement_line_recognized_recognizes_exile_then_play_costs_more_from_tokens()
    -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "Exile target nonland permanent. For as long as that card remains exiled, its owner may play it. A spell cast by an opponent this way costs {2} more to cast.",
        );

        assert!(recognize_statement_line(&line)?.is_some());

        Ok(())
    }

    #[test]
    fn statement_line_recognized_recognizes_each_player_choose_then_bounce_unchosen()
    -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "Each player chooses a nonland permanent they control. Return all nonland permanents not chosen this way to their owners' hands. Then you draw a card for each opponent who has more cards in their hand than you.",
        );

        assert!(looks_like_statement_line_lexed(&line));
        assert!(recognize_statement_line(&line)?.is_some());

        Ok(())
    }

    #[test]
    fn statement_line_recognized_recognizes_each_opponent_gets_poison_counter()
    -> Result<(), CardTextError> {
        let line = single_preprocessed_line("Each opponent gets a poison counter.");

        assert!(looks_like_statement_line_lexed(&line));
        assert!(recognize_statement_line(&line)?.is_some());

        Ok(())
    }

    #[test]
    fn statement_line_recognized_recognizes_player_counter_clauses_in_compound_effects()
    -> Result<(), CardTextError> {
        for text in [
            "You draw two cards and you lose 2 life. Each opponent gets a poison counter.",
            "Each opponent sacrifices a creature or planeswalker of their choice and gets a poison counter.",
        ] {
            let line = single_preprocessed_line(text);
            assert!(
                recognize_statement_line(&line)?.is_some(),
                "expected compound player-counter effect to be a statement: {text}"
            );
        }

        Ok(())
    }

    #[test]
    fn unsupported_line_reason_recognizes_modal_header_from_tokens() {
        let line = single_preprocessed_line("Choose one —");

        assert_eq!(
            classify_unsupported_line_reason(&line),
            "modal-header-not-yet-supported"
        );
    }

    #[test]
    fn art_rating_statement_routes_to_unsupported_reason_from_tokens() -> Result<(), CardTextError>
    {
        let line = single_preprocessed_line(
            "Ask a person outside the game to rate its new art on a scale from 1 to 5.",
        );

        assert!(recognize_statement_line(&line)?.is_none());
        assert_eq!(
            classify_unsupported_line_reason(&line),
            "outside-the-game-rating-not-supported"
        );

        Ok(())
    }

    #[test]
    fn level_item_recognized_stores_parsed_payload() -> Result<(), CardTextError> {
        let card = CardBuilder::new(CardId::new(), "Document Parser Test")
            .card_types(vec![CardType::Creature]);
        let line = single_preprocessed_line("Flying");

        let parsed =
            recognize_level_item(&card, &line)?.expect("expected flying to parse as level item");

        assert_eq!(parsed.text, "flying");
        match &parsed.parsed {
            crate::cards::builders::ParsedLevelAbilityItemAst::KeywordActions(actions) => {
                assert!(!actions.is_empty());
            }
            other => panic!("expected keyword-actions payload, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn saga_chapter_recognized_stores_effects_ast() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Saga Parse Tokens Test")
                .card_types(vec![CardType::Enchantment]),
            "I, II — Mega Flare — Draw a card.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        match recognized.lines.as_slice() {
            [super::RecognizedLine::SagaChapter(saga)] => {
                assert_eq!(saga.text, "Draw a card.");
                assert_eq!(saga.presentation_label.as_deref(), Some("Mega Flare"));
                assert!(!saga.effects_ast.is_empty());
            }
            other => panic!("expected one saga chapter line, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn level_header_lowering_keeps_parsed_level_items() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Level Lowering Parse Tokens Test")
                .card_types(vec![CardType::Creature]),
            "Level up {1}\nLEVEL 1-2\nFlying\n3/3",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;
        let semantic = super::assemble_document(
            preprocessed,
            recognized,
            None,
            super::super::ir::DocumentSemanticFacts::default(),
            false,
        )?;

        let level = semantic
            .items
            .iter()
            .find_map(|item| match item {
                super::RewriteSemanticItem::LevelHeader(level) => Some(level),
                _ => None,
            })
            .expect("expected lowered semantic document to contain a level header");

        match level.items.as_slice() {
            [item] => match &item.parsed {
                crate::cards::builders::ParsedLevelAbilityItemAst::KeywordActions(actions) => {
                    assert!(!actions.is_empty());
                }
                other => panic!("expected keyword-actions level item, got {other:?}"),
            },
            other => panic!("expected one lowered level item, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn saga_chapter_lowering_keeps_effects_ast() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Saga Lowering Parse Tokens Test")
                .card_types(vec![CardType::Enchantment]),
            "I, II — Mega Flare — Draw a card.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;
        let semantic = super::assemble_document(
            preprocessed,
            recognized,
            None,
            super::super::ir::DocumentSemanticFacts::default(),
            false,
        )?;

        let saga = semantic
            .items
            .iter()
            .find_map(|item| match item {
                super::RewriteSemanticItem::SagaChapter(saga) => Some(saga),
                _ => None,
            })
            .expect("expected lowered semantic document to contain a saga chapter");

        assert_eq!(saga.text, "Draw a card.");
        assert_eq!(
            saga.presentation_label
                .as_ref()
                .and_then(PresentationLabel::display_prefix)
                .as_deref(),
            Some("Mega Flare")
        );
        assert!(!saga.effects_ast.is_empty());

        Ok(())
    }

    #[test]
    fn statement_parse_groups_lexed_strip_labels_and_rewrite_followups() {
        let line = single_preprocessed_line(
            "Meteor Strikes — Exile target artifact. When you do, draw a card.",
        );

        let groups = normalize_statement_parse_groups_lexed(&line.tokens)
            .into_iter()
            .map(|group| render_token_slice(&group))
            .collect::<Vec<_>>();

        assert_eq!(
            groups,
            vec!["exile target artifact. when you do, draw a card.".to_string()]
        );
    }

    #[test]
    fn statement_parse_groups_keep_typed_energy_payment_threshold_bundle_together()
    -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "You get X {E} (energy counters), then you may pay any amount of {E}. Destroy each artifact, creature, and enchantment with mana value less than or equal to the amount of {E} paid this way.",
        );

        let groups = normalize_statement_parse_groups_lexed(&line.tokens);
        assert_eq!(groups.len(), 1, "typed bundle was split: {groups:#?}");
        let effects = super::parse_effect_sentences_lexed(&groups[0])
            .expect("typed energy payment threshold bundle should lower");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("EventValue"), "{debug}");
        assert!(debug.contains("LessThanOrEqualExpr"), "{debug}");
        assert!(debug.contains("PayAnyEnergy"), "{debug}");
        assert!(
            recognize_statement_line(&line)?.is_some(),
            "typed bundle should route as a statement line"
        );
        Ok(())
    }

    #[test]
    fn historical_target_return_preempts_recognized_put_clause_probe() -> Result<(), CardTextError>
    {
        let line = single_preprocessed_line(
            "Choose up to three target permanent cards in graveyards that were put there from the battlefield this turn. Return them to the battlefield tapped under their owners' control. You draw a card for each opponent who controls one or more of those permanents.",
        );
        let statement = recognize_statement_line(&line)?
            .expect("historical target return should remain one statement");
        assert_eq!(statement.parse_groups.len(), 1);
        let effects = super::parse_effect_sentences_lexed(&statement.parse_groups[0])?;
        let debug = format!("{effects:#?}");
        assert!(
            debug.contains("entered_graveyard_from_battlefield_this_turn: true"),
            "{debug}"
        );
        assert!(debug.contains("PlayerControls"), "{debug}");
        Ok(())
    }

    #[test]
    fn counter_linked_land_subtype_followup_routes_as_typed_statement() -> Result<(), CardTextError>
    {
        let line = single_preprocessed_line(
            "That land is an Island in addition to its other types for as long as it has a flood counter on it.",
        );
        let statement = recognize_statement_line(&line)?
            .expect("typed counter-linked subtype followup should route as a statement");
        let effects = super::parse_effect_sentences_lexed(&statement.parse_groups[0])?;
        let debug = format!("{effects:#?}");
        assert!(debug.contains("AddSubtypes"), "{debug}");
        assert!(debug.contains("Island"), "{debug}");
        Ok(())
    }

    #[test]
    fn statement_parse_groups_lexed_split_instead_followup_into_separate_chunks() {
        let line = single_preprocessed_line(
            "Exile target creature. Return that card to the battlefield under its owner's control instead, then scry 1.",
        );

        let groups = normalize_statement_parse_groups_lexed(&line.tokens)
            .into_iter()
            .map(|group| render_token_slice(&group))
            .collect::<Vec<_>>();

        assert_eq!(
            groups,
            vec![
                "exile target creature.".to_string(),
                "return that card to the battlefield under its owner's control instead, then scry 1."
                    .to_string(),
            ]
        );
    }

    #[test]
    fn statement_parse_groups_lexed_rewrites_copy_exception_without_relex() {
        let line = single_preprocessed_line(
            "Target artifact becomes a copy of target enchantment, except it's an artifact and it loses all other card types.",
        );

        let groups = normalize_statement_parse_groups_lexed(&line.tokens)
            .into_iter()
            .map(|group| render_token_slice(&group))
            .collect::<Vec<_>>();

        assert_eq!(
            groups,
            vec![
                "target artifact becomes a copy of target enchantment, except it's an artifact."
                    .to_string()
            ]
        );
    }

    #[test]
    fn statement_parse_groups_lexed_keep_broken_visage_followups_in_one_effect_group() {
        let line = single_preprocessed_line(
            "Destroy target nonartifact attacking creature. It can't be regenerated. Create a black Spirit creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness. Sacrifice the token at the beginning of the next end step.",
        );

        let groups = normalize_statement_parse_groups_lexed(&line.tokens)
            .into_iter()
            .map(|group| render_token_slice(&group))
            .collect::<Vec<_>>();

        assert_eq!(
            groups,
            vec![
                "destroy target nonartifact attacking creature. it can't be regenerated. create a black spirit creature token. its power is equal to that creature's power and its toughness is equal to that creature's toughness. sacrifice the token at the beginning of the next end step.".to_string()
            ]
        );
    }

    #[test]
    fn statement_parse_groups_keep_regeneration_followup_with_conditional_destroy() {
        let line = single_preprocessed_line(
            "Destroy two target nonblack creatures unless either one is a color the other isn't. They can't be regenerated.",
        );

        let groups = normalize_statement_parse_groups_lexed(&line.tokens)
            .into_iter()
            .map(|group| render_token_slice(&group))
            .collect::<Vec<_>>();

        assert_eq!(groups.len(), 1, "dependent followup was split: {groups:#?}");
        assert!(groups[0].contains("they can't be regenerated"));
    }

    #[test]
    fn parse_statement_line_recognized_does_not_abort_on_broken_visage_static_probe_error()
    -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "Destroy target nonartifact attacking creature. It can't be regenerated. Create a black Spirit creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness. Sacrifice the token at the beginning of the next end step.",
        );

        assert!(recognize_statement_line(&line)?.is_some());

        Ok(())
    }

    #[test]
    fn parse_statement_line_recognized_keeps_typed_create_token_rules_out_of_static_probe()
    -> Result<(), CardTextError> {
        for text in [
            "Create a 1/1 green Wolf creature token. It has \"This token gets +1/+1 for each card named Sound the Call in each graveyard.\"",
            "Create a 0/0 colorless Construct artifact creature token with 'This token gets +1/+1 for each artifact you control.'",
        ] {
            let line = single_preprocessed_line(text);
            assert!(
                recognize_statement_line(&line)?.is_some(),
                "typed create-token statement was diverted to the static probe: {text}"
            );
        }

        Ok(())
    }

    #[test]
    fn statement_static_probes_do_not_report_loss_for_typed_token_rules()
    -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "Create a 1/1 green Wolf creature token. It has \"This token gets +1/+1 for each card named Sound the Call in each graveyard.\"",
        );
        let (parsed, loss) = crate::parse_loss::capture(|| recognize_statement_line(&line));

        assert!(parsed?.is_some());
        assert!(
            !loss.is_lossy(),
            "rejected static probes must not taint the committed statement parse: {}",
            loss.reasons_text()
        );
        Ok(())
    }

    #[test]
    fn triggered_static_tail_probe_does_not_split_conditional_token_rules() {
        let token_creation = lex_line(
            "You may pay {2}. If you do, create a 0/0 colorless Construct artifact creature token with \"This token gets +1/+1 for each artifact you control.\"",
            0,
        )
        .expect("conditional token creation should lex");
        assert!(
            !triggered_effect_tokens_have_trailing_static_sentences(&token_creation),
            "quoted token rules must stay in the trigger resolution program"
        );

        for copy_effect in [
            "If you reveal a creature card this way, this creature becomes a copy of that card until end of turn, except it has flying.",
            "Until end of turn, target token you control becomes a copy of it, except it has flying.",
        ] {
            let copy_tokens = lex_line(copy_effect, 0).expect("copy effect should lex");
            assert!(
                !triggered_effect_tokens_have_trailing_static_sentences(&copy_tokens),
                "typed copy exceptions must stay in the trigger resolution program: {copy_effect}"
            );
        }

        let moved_object_followup = lex_line(
            "Draw a card. Then you may put a creature card with mana value 3 or less from your hand onto the battlefield. It enters tapped and attacking and gains indestructible until end of turn.",
            0,
        )
        .expect("moved-object follow-up should lex");
        assert!(
            !triggered_effect_tokens_have_trailing_static_sentences(&moved_object_followup),
            "an entry-state follow-up belongs to the preceding optional move"
        );

        let returned_object_characteristics = lex_line(
            "You may return target Pirate creature card from your graveyard to the battlefield with a finality counter on it. It has base power and toughness 4/4. It gains haste until end of turn.",
            0,
        )
        .expect("returned-object characteristic followups should lex");
        assert!(
            !triggered_effect_tokens_have_trailing_static_sentences(
                &returned_object_characteristics
            ),
            "returned-object characteristics belong to the preceding move"
        );

        let true_static_tail = lex_line("Draw a card. Creatures you control have flying.", 0)
            .expect("static-tail control should lex");
        assert!(
            triggered_effect_tokens_have_trailing_static_sentences(&true_static_tail),
            "an actual trailing source static ability remains distinguishable"
        );
    }

    #[test]
    fn source_spell_cast_trigger_is_not_misclassified_as_delayed_spell_effect() {
        let instant = preprocess_document(
            CardBuilder::new(CardId::new(), "Malicious Affliction Variant")
                .card_types(vec![CardType::Instant]),
            "When you cast this spell, if a creature died this turn, you may copy this spell and may choose a new target for the copy.",
        )
        .expect("source-cast trigger should preprocess");
        let PreprocessedItem::Line(source_cast) = instant.items.last().unwrap() else {
            panic!("expected source-cast line");
        };
        assert!(
            !should_parse_delayed_trigger_line_as_spell_effect(&instant, &source_cast.tokens),
            "a trigger caused by casting this same spell belongs on the spell"
        );

        let delayed = preprocess_document(
            CardBuilder::new(CardId::new(), "Delayed Cast Variant")
                .card_types(vec![CardType::Instant]),
            "This turn, whenever you cast a creature spell, draw a card.",
        )
        .expect("delayed cast trigger should preprocess");
        let PreprocessedItem::Line(delayed_line) = delayed.items.last().unwrap() else {
            panic!("expected delayed line");
        };
        assert!(
            should_parse_delayed_trigger_line_as_spell_effect(&delayed, &delayed_line.tokens),
            "a genuine this-turn future trigger remains a resolution effect"
        );
    }

    #[test]
    fn looks_like_statement_line_recognizes_vote_leads() {
        for text in [
            "Starting with you, each player votes for death or torture. If death gets more votes, each opponent sacrifices a creature of their choice. If torture gets more votes or the vote is tied, each opponent loses 4 life.",
            "Secret council — Each player secretly votes for truth or consequences, then those votes are revealed. For each truth vote, draw a card. Then choose an opponent at random. For each consequences vote, Truth or Consequences deals 3 damage to that player.",
        ] {
            let helper_text = split_label_prefix(text)
                .map(|(_, body)| body.trim())
                .unwrap_or(text);
            let tokens =
                lex_line(helper_text, 0).expect("rewrite lexer should classify vote line body");
            assert_eq!(
                classify_statement_line_family_lexed(&tokens),
                Some(StatementLineFamily::Vote),
                "{text}"
            );
            assert!(
                looks_like_statement_line(text.to_ascii_lowercase().as_str()),
                "expected vote line to classify as a statement: {text}"
            );
        }
    }

    #[test]
    fn council_choice_label_routes_vote_sequence_as_statement() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Council Vote Test")
                .card_types(vec![CardType::Sorcery]),
            "Will of the council — Starting with you, each player votes for death or torture. If death gets more votes, each opponent sacrifices a creature of their choice. If torture gets more votes or the vote is tied, each opponent loses 4 life.",
        )?;
        let PreprocessedItem::Line(line) = &preprocessed.items[0] else {
            panic!("expected council vote to preprocess as one line");
        };
        assert_eq!(
            render_token_slice(&line.tokens),
            "starting with you, each player votes for death or torture. if death gets more votes, each opponent sacrifices a creature of their choice. if torture gets more votes or the vote is tied, each opponent loses 4 life."
        );
        assert_eq!(
            classify_statement_line_family_lexed(&line.tokens),
            Some(StatementLineFamily::Vote)
        );
        let parse_groups = normalize_statement_parse_groups_lexed(&line.tokens);
        assert_eq!(
            parse_groups.len(),
            1,
            "unexpected vote parse groups: {parse_groups:#?}"
        );
        let effects = super::parse_effect_sentences_lexed(&parse_groups[0])?;
        assert!(!effects.is_empty(), "vote group parsed to no effects");
        assert!(
            recognize_statement_line(line)?.is_some(),
            "expected council vote body to parse as a statement"
        );
        let recognized = super::recognize_document(&preprocessed, false)?;

        assert!(
            matches!(
                recognized.lines.as_slice(),
                [super::RecognizedLine::Statement(_)]
            ),
            "expected labeled council vote to route as one statement, got {:#?}",
            recognized.lines
        );

        Ok(())
    }

    #[test]
    fn looks_like_statement_line_recognizes_next_turn_cast_lock() {
        let text =
            "Each opponent can't cast instant or sorcery spells during that player's next turn.";
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify next-turn cast-lock line");
        assert_eq!(
            classify_statement_line_family_lexed(&tokens),
            Some(StatementLineFamily::NextTurnCantCast)
        );
        assert!(looks_like_statement_line(
            text.to_ascii_lowercase().as_str()
        ));
    }

    #[test]
    fn looks_like_statement_line_recognizes_generic_heads() {
        for text in [
            "Draw a card.",
            "Each player discards a card.",
            "Each other player sacrifices a creature.",
            "That target player sacrifices a creature.",
            "This spell deals 3 damage to any target.",
            "Target creature gets +2/+2 until end of turn.",
        ] {
            let tokens =
                lex_line(text, 0).expect("rewrite lexer should classify generic statement head");
            assert_eq!(
                classify_statement_line_family_lexed(&tokens),
                Some(StatementLineFamily::Generic)
            );
            assert!(looks_like_statement_line(
                text.to_ascii_lowercase().as_str()
            ));
        }
    }

    #[test]
    fn emblem_payload_statement_routes_through_document_recognized() -> Result<(), CardTextError> {
        for text in [
            r#"You get an emblem with "Whenever you cast a spell, draw a card.""#,
            r#"You get an emblem with "You have no maximum hand size.""#,
            r#"You get an emblem with "{T}: Draw a card.""#,
            r#"You get an emblem with "You have no maximum hand size." and "{T}: Draw a card.""#,
        ] {
            let preprocessed = preprocess_document(
                CardBuilder::new(CardId::new(), "Emblem Document Test")
                    .card_types(vec![CardType::Sorcery]),
                text,
            )?;
            let PreprocessedItem::Line(line) = &preprocessed.items[0] else {
                panic!("expected emblem payload to preprocess as one line: {text}");
            };
            assert_eq!(
                classify_statement_line_family_lexed(&line.tokens),
                Some(StatementLineFamily::Emblem),
                "{text}"
            );
            assert!(looks_like_statement_line_lexed(line), "{text}");
            let parse_groups = normalize_statement_parse_groups_lexed(&line.tokens);
            assert_eq!(parse_groups.len(), 1, "{text}: {parse_groups:#?}");
            let effects = super::parse_effect_sentences_lexed(&parse_groups[0])?;
            assert!(!effects.is_empty(), "{text}: {parse_groups:#?}");
            assert!(recognize_statement_line(line)?.is_some(), "{text}");

            let recognized = super::recognize_document(&preprocessed, false)?;
            assert!(
                matches!(
                    recognized.lines.as_slice(),
                    [super::RecognizedLine::Statement(_)]
                ),
                "expected emblem payload to route as one statement: {text}; got {:#?}",
                recognized.lines
            );
        }

        Ok(())
    }

    #[test]
    fn looks_like_lexed_line_family_helpers_handle_nonkeyword_labels() {
        let statement = single_preprocessed_line("Battle Plan — Each player discards a card.");
        let static_line = single_preprocessed_line("Mystic Aura — Enchanted creature gets +1/+1.");

        assert!(looks_like_statement_line_lexed(&statement));
        assert!(looks_like_static_line_lexed(&static_line));
    }

    #[test]
    fn looks_like_static_line_recognizes_generic_heads() {
        for text in [
            "This creature has flying.",
            "Enchanted creature gets +1/+1.",
            "As long as you control an artifact, this creature has hexproof.",
            "Your maximum hand size is reduced by four.",
            "Power Tester's power is equal to the number of creatures you control.",
        ] {
            let tokens =
                lex_line(text, 0).expect("rewrite lexer should classify generic static head");
            assert_eq!(
                classify_static_line_family_lexed(&tokens),
                Some(StaticLineFamily::Generic)
            );
            assert!(looks_like_static_line(text.to_ascii_lowercase().as_str()));
        }
    }

    #[test]
    fn triggered_split_probe_preserves_failed_effect_parse_details() {
        let line = single_preprocessed_line(
            "Whenever this creature attacks, search your library for artifact card named.",
        );
        let comma_idx =
            crate::lexer::locate_token_kind(&line.tokens, crate::lexer::TokenKind::Comma)
                .expect("expected triggered probe line to contain a comma");
        let probe = probe_triggered_split(
            &line.tokens[1..comma_idx],
            &line.tokens[comma_idx + 1..],
            None,
            None,
        );
        let fallback = probe.fallback_recognized(&line, &line.tokens);

        match probe {
            TriggeredSplitProbe::Unsupported {
                trigger_error,
                effect_error,
                ..
            } => {
                assert!(trigger_error.is_none());
                assert!(effect_error.is_some());
                assert!(fallback.is_some());
            }
            other => panic!("expected unsupported triggered split probe, got {other:?}"),
        }
    }

    #[test]
    fn triggered_conditional_split_preserves_full_text_for_lowering() {
        let line = single_preprocessed_line(
            "At the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the rest on the bottom of your library in a random order.",
        );

        let parsed =
            recognize_triggered_line(&line).expect("expected triggered conditional line to parse");

        assert_eq!(
            parsed.full_text,
            "at the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. put that card into your hand and the rest on the bottom of your library in a random order."
        );
        assert_eq!(
            render_token_slice(&parsed.trigger_parse_tokens),
            "the beginning of your second main phase"
        );
        assert_eq!(
            render_token_slice(&parsed.effect_parse_tokens),
            "reveal cards from the top of your library until you reveal a land card. put that card into your hand and the rest on the bottom of your library in a random order."
        );
        assert_eq!(
            render_token_slice(&parsed.full_parse_tokens),
            parsed.full_text
        );
    }

    #[test]
    fn triggered_split_keeps_producer_before_reflexive_conditional_followup() {
        let line = single_preprocessed_line(
            "At the beginning of combat on your turn, mill a card. When you do, if there are four or more creature cards in your graveyard, put a +1/+1 counter on target creature you control and it gains deathtouch until end of turn.",
        );

        let parsed = recognize_triggered_line(&line)
            .expect("producer and reflexive conditional should remain one trigger body");

        assert_eq!(
            render_token_slice(&parsed.trigger_parse_tokens),
            "the beginning of combat on your turn"
        );
        assert_eq!(
            render_token_slice(&parsed.effect_parse_tokens),
            "mill a card. when you do, if there are four or more creature cards in your graveyard, put a +1/+1 counter on target creature you control and it gains deathtouch until end of turn."
        );
        let effects = super::parse_effect_sentences_lexed(&parsed.effect_parse_tokens)
            .expect("producer and reflexive conditional effects should parse");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("ValueComparison"), "{debug}");
        assert!(debug.contains("PutCounters"), "{debug}");
        assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
    }

    #[test]
    fn ordinary_reflexive_followup_does_not_claim_conditional_split_override() {
        let line = single_preprocessed_line(
            "When this land enters, sacrifice it. When you do, search your library for a basic Forest, Plains, or Island card, put it onto the battlefield tapped, then shuffle and you gain 1 life.",
        );
        let (_, effect_tokens) = super::grammar::split_lexed_once_on_comma(&line.tokens)
            .expect("triggered land line has a trigger/effect comma");

        assert!(
            !super::line_recognition::contains_reflexive_conditional_followup_sentence(
                effect_tokens
            )
        );
        let effects = super::parse_effect_sentences_lexed(effect_tokens)
            .expect("ordinary reflexive trigger body parses");
        let normalized = crate::effect_ast_normalization::normalize_effects_ast(&effects);
        let annotated = crate::reference_resolution::annotate_effect_sequence(
            &normalized,
            &crate::model::reference_state::ReferenceImports::default(),
            crate::reference_resolution::EffectReferenceResolutionConfig::default(),
            crate::cards::builders::IdGenContext::default(),
        )
        .expect("ordinary reflexive producer and result annotate together");
        assert!(annotated.effects[0].assigned_effect_id.is_some());
        assert!(matches!(
            annotated.effects.get(1).map(|effect| &effect.effect),
            Some(crate::cards::builders::EffectAst::ControlFlow(_))
        ));
    }

    #[test]
    fn triggered_recognized_preserves_linked_token_next_turn_sacrifice_for_semantic_lowering() {
        let line = single_preprocessed_line(
            "When this creature enters, create a Lander token. At the beginning of the end step on your next turn, sacrifice that token.",
        );
        let parsed = recognize_triggered_line(&line)
            .expect("linked delayed sacrifice should survive the recognized form probe");

        assert_eq!(
            render_token_slice(&parsed.trigger_parse_tokens),
            "this creature enters"
        );
        assert_eq!(
            render_token_slice(&parsed.effect_parse_tokens),
            "create a Lander token. At the beginning of the end step on your next turn, sacrifice that token."
        );
    }

    #[test]
    fn triggered_recognized_preserves_reciprocal_created_token_lifecycle() {
        let line = single_preprocessed_line(
            "When this creature enters, create a 1/1 colorless Construct artifact creature token. Exile that token when this creature leaves the battlefield. Sacrifice this creature when that token leaves the battlefield.",
        );
        let parsed = recognize_triggered_line(&line)
            .expect("reciprocal created-token lifecycle should survive the recognized form probe");

        assert_eq!(
            render_token_slice(&parsed.trigger_parse_tokens),
            "this creature enters"
        );
        let effects = render_token_slice(&parsed.effect_parse_tokens);
        let lower_effects = effects.to_ascii_lowercase();
        assert!(
            lower_effects.starts_with("create a 1/1 colorless construct"),
            "{effects}"
        );
        assert!(
            lower_effects.contains("exile that token when this creature leaves the battlefield"),
            "{effects}"
        );
        assert!(
            lower_effects
                .contains("sacrifice this creature when that token leaves the battlefield"),
            "{effects}"
        );
    }

    #[test]
    fn triggered_split_keeps_anaphoric_animation_in_the_resolution_program() {
        let followup = lex_line(
            "If it isn't a creature, it becomes a 0/0 Mutant creature in addition to its other types.",
            0,
        )
        .expect("anaphoric animation followup should lex");
        let parsed_followup = super::parse_effect_sentences_lexed(&followup)
            .expect("anaphoric animation followup should parse as a resolving effect");
        assert!(
            !sentence_is_static_after_trigger_effect(&followup),
            "a conditional that consumes the prior chosen object must not become a source static ability: {parsed_followup:#?}"
        );

        let line = single_preprocessed_line(
            "At the beginning of your end step, if a permanent left the battlefield under your control this turn, put three +1/+1 counters on up to one other target artifact or creature. If it isn't a creature, it becomes a 0/0 Mutant creature in addition to its other types.",
        );
        let parsed = recognize_triggered_line(&line)
            .expect("the complete trigger and its linked animation should parse");
        let effect_text = render_token_slice(&parsed.effect_parse_tokens);
        assert!(
            effect_text.contains("put three +1/+1 counters"),
            "{effect_text}"
        );
        assert!(
            effect_text.contains("if it isn't a creature, it becomes a 0/0 mutant creature"),
            "{effect_text}"
        );
    }

    #[test]
    fn labeled_qualified_ability_trigger_uses_the_typed_trigger_and_complete_body() {
        let line = single_preprocessed_line(
            "Whenever a creature entering under an opponent's control causes a triggered ability of that creature to trigger, you may copy that ability. You may choose new targets for the copy.",
        );
        let parsed = recognize_labeled_qualified_ability_trigger(&line)
            .expect("the qualified ability trigger should have a direct typed recognized form");
        assert_eq!(
            render_token_slice(&parsed.trigger_parse_tokens),
            "a creature entering under an opponent's control causes a triggered ability of that creature to trigger"
        );
        assert_eq!(
            render_token_slice(&parsed.effect_parse_tokens),
            "you may copy that ability. you may choose new targets for the copy."
        );
    }

    #[test]
    fn triggered_conditional_split_accepts_creature_died_under_your_control() {
        let line = single_preprocessed_line(
            "At the beginning of your end step, if a creature died under your control this turn, each opponent sacrifices a creature of their choice",
        );

        let parsed =
            recognize_triggered_line(&line).expect("Barrensteppe Siege Mardu trigger should parse");

        assert_eq!(
            render_token_slice(&parsed.trigger_parse_tokens),
            "the beginning of your end step"
        );
        let effect_text = render_token_slice(&parsed.effect_parse_tokens);
        assert!(
            effect_text.contains("each opponent sacrifices a creature of their choice"),
            "expected sacrifice-choice effect text, got {}",
            effect_text
        );
        assert!(
            matches!(
                parsed.intervening_if,
                Some(crate::cards::builders::PredicateAst::ValueComparison { .. })
            ),
            "expected controller-qualified death predicate, got {:?}",
            parsed.intervening_if
        );
    }

    #[test]
    fn triggered_conditional_split_preserves_negative_attack_history_gates() {
        for (text, expected) in [
            (
                "At the beginning of your end step, if this creature didn't attack this turn, put a +1/+1 counter on it.",
                crate::cards::builders::PredicateAst::Not(Box::new(
                    crate::cards::builders::PredicateAst::Source(SourcePredicateAst::SourceAttackedThisTurn),
                )),
            ),
            (
                "At the beginning of your end step, if you didn't attack with a creature this turn, sacrifice this Aura.",
                crate::cards::builders::PredicateAst::Not(Box::new(
                    crate::cards::builders::PredicateAst::TurnEvents(TurnEventPredicateAst::YouAttackedThisTurn),
                )),
            ),
        ] {
            let line = single_preprocessed_line(text);
            let parsed = recognize_triggered_line(&line)
                .unwrap_or_else(|err| panic!("negative attack gate should parse: {err}"));
            assert_eq!(parsed.intervening_if, Some(expected), "{text}");
        }
    }

    #[test]
    fn triggered_conditional_split_preserves_named_label_unpaid_gate() {
        let line = single_preprocessed_line(
            "When this creature enters, if tribute wasn't paid, it gains haste until end of turn.",
        );

        let parsed =
            recognize_triggered_line(&line).expect("Thunder Brute tribute trigger should parse");

        assert_eq!(
            render_token_slice(&parsed.trigger_parse_tokens),
            "this creature enters"
        );
        assert_eq!(
            render_token_slice(&parsed.effect_parse_tokens),
            "it gains haste until end of turn."
        );
        assert_eq!(
            parsed.intervening_if,
            Some(crate::cards::builders::PredicateAst::Not(Box::new(
                crate::cards::builders::PredicateAst::ThisSpellPaidLabel("Tribute".into()),
            ))),
        );
    }

    #[test]
    fn triggered_end_step_puts_counters_on_each_creature_you_control() {
        let line = single_preprocessed_line(
            "At the beginning of your end step, put a +1/+1 counter on each creature you control.",
        );

        let parsed =
            recognize_triggered_line(&line).expect("Barrensteppe Siege Abzan trigger should parse");

        assert_eq!(
            render_token_slice(&parsed.trigger_parse_tokens),
            "the beginning of your end step"
        );
        let effect_text = render_token_slice(&parsed.effect_parse_tokens);
        assert!(
            effect_text.contains("put a +1/+1 counter on each creature you control"),
            "expected counter effect text, got {}",
            effect_text
        );
    }

    #[test]
    fn bullet_choice_labels_are_detected_as_choice_peers() {
        let line = single_preprocessed_line(
            "• Mardu — At the beginning of your end step, if a creature died under your control this turn, each opponent sacrifices a creature of their choice.",
        );

        assert!(
            super::is_nonkeyword_choice_labeled_line(&line),
            "expected bullet-prefixed option label to be detected as a choice peer"
        );
    }

    #[test]
    fn barrensteppe_siege_choice_block_parses_both_bullet_options() {
        let (semantic, _) = parse_text_to_semantic_document(
            CardBuilder::new(CardId::new(), "Barrensteppe Siege")
                .card_types(vec![CardType::Enchantment]),
            "As this enchantment enters, choose Abzan or Mardu.\n• Abzan — At the beginning of your end step, put a +1/+1 counter on each creature you control.\n• Mardu — At the beginning of your end step, if a creature died under your control this turn, each opponent sacrifices a creature of their choice.".to_string(),
            false,
        )
        .expect("Barrensteppe Siege choice block should parse");

        assert_eq!(semantic.items.len(), 3);
    }

    #[test]
    fn eminence_labeled_trigger_remains_one_triggered_semantic_item() {
        let (semantic, _) = parse_text_to_semantic_document(
            CardBuilder::new(CardId::new(), "Eminence Fixture")
                .card_types(vec![CardType::Creature]),
            "Eminence — At the beginning of combat on your turn, if this is in the command zone or on the battlefield, another target Cat you control gets +3/+3 until end of turn."
                .to_string(),
            false,
        )
        .expect("Eminence should remain a typed triggered line");

        assert_eq!(semantic.items.len(), 1, "{semantic:#?}");
        let [crate::ir::RewriteSemanticItem::ParsedLine(line)] = semantic.items.as_slice() else {
            panic!("expected one parsed Eminence line: {semantic:#?}");
        };
        let [crate::cards::builders::LineAst::Ability(ability)] = line.chunks.as_slice() else {
            panic!("expected one typed Eminence ability: {:#?}", line.chunks);
        };
        assert!(matches!(
            ability.trigger_spec.as_deref(),
            Some(crate::cards::builders::TriggerSpec::BeginningOfCombat(
                crate::target::PlayerFilter::You
            ))
        ));
        assert!(matches!(
            ability.effects_ast.as_deref(),
            Some([crate::cards::builders::EffectAst::Conditionals(crate::cards::builders::ConditionalEffectAst::Conditional {
                predicate: crate::cards::builders::PredicateAst::Source(SourcePredicateAst::SourceMatches(filter)),
                if_true,
                ..
            })]) if filter.any_of.len() == 2 && !if_true.is_empty()
        ));
    }

    #[test]
    fn triggered_presentation_label_is_derived_from_lexed_line_tokens() {
        let tokens = lex_line(
            "Mold Earth — Whenever one or more lands enter under an opponent's control without being played, draw a card.",
            0,
        )
        .expect("trigger label fixture should lex");

        assert_eq!(
            trigger_presentation_from_line_tokens(&tokens),
            Some(PresentationLabel::AbilityWord("Mold Earth".to_string()))
        );
    }

    #[test]
    fn triggered_presentation_label_keeps_source_acronym_casing_after_dispatch()
    -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "ED-E").card_types(vec![CardType::Creature]),
            "ED-E My Love — Whenever you attack, draw a card.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        match recognized.lines.as_slice() {
            [super::RecognizedLine::Triggered(triggered)] => assert_eq!(
                triggered.presentation,
                Some(PresentationLabel::AbilityWord("ED-E My Love".to_string()))
            ),
            other => panic!("expected one labeled triggered line, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn statement_parse_handles_when_one_or_more_this_way_target_card_type_list() {
        let line = single_preprocessed_line(
            "When one or more cards are milled this way, exile target enchantment, instant, or sorcery card with equal or lesser mana value than that spell from an opponent's graveyard.",
        );
        let rewritten = rewrite_when_one_or_more_this_way_line(&line);

        assert!(
            recognize_statement_line(&rewritten)
                .expect("result follow-up should not error")
                .is_some(),
            "result follow-up should parse as an effect statement"
        );
    }

    #[test]
    fn colon_nonactivation_statement_fallback_reuses_split_token_slice() {
        let line = single_preprocessed_line("Reveal this card from your hand: Draw a card.");

        let parsed = parse_colon_nonactivation_statement_fallback(&line)
            .expect("expected fallback parse to succeed")
            .expect("expected reveal-prefix fallback to produce a statement");

        assert_eq!(parsed.text, "reveal this card from your hand");
        assert_eq!(
            parsed.info.normalized.normalized,
            "reveal this card from your hand"
        );
    }

    #[test]
    fn trigger_sentence_chunk_splitter_reuses_token_ranges() {
        let tokens = lex_line(
            "Whenever this creature attacks, draw a card. Whenever it deals combat damage to a player, create a Treasure token.",
            0,
        )
        .expect("rewrite lexer should classify trigger chunk line");

        let chunks = split_trigger_sentence_chunks_rewrite_lexed(&tokens)
            .into_iter()
            .map(|chunk| render_token_slice(&chunk))
            .collect::<Vec<_>>();

        assert_eq!(
            chunks,
            vec![
                "Whenever this creature attacks, draw a card".to_string(),
                "Whenever it deals combat damage to a player, create a Treasure token".to_string(),
            ]
        );
    }

    #[test]
    fn trigger_sentence_chunk_splitter_keeps_delayed_dies_followup_with_trigger() {
        let tokens = lex_line(
            "Whenever you attack, target attacking creature gets +1/+0 until end of turn. When that creature dies this turn, surveil 1.",
            0,
        )
        .expect("rewrite lexer should classify delayed dies followup line");

        let chunks = split_trigger_sentence_chunks_rewrite_lexed(&tokens)
            .into_iter()
            .map(|chunk| render_token_slice(&chunk))
            .collect::<Vec<_>>();

        assert_eq!(
            chunks,
            vec![
                "Whenever you attack, target attacking creature gets +1/+0 until end of turn. When that creature dies this turn, surveil 1"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn trigger_sentence_chunk_splitter_keeps_delayed_leaves_followup_with_trigger() {
        let tokens = lex_line(
            "When this creature dies, exile it and choose target creature an opponent controls. When that creature leaves the battlefield, return this card from exile to the battlefield under its owner's control.",
            0,
        )
        .expect("rewrite lexer should classify delayed leaves followup line");

        let chunks = split_trigger_sentence_chunks_rewrite_lexed(&tokens)
            .into_iter()
            .map(|chunk| render_token_slice(&chunk))
            .collect::<Vec<_>>();

        assert_eq!(
            chunks,
            vec![
                "When this creature dies, exile it and choose target creature an opponent controls. When that creature leaves the battlefield, return this card from exile to the battlefield under its owner's control"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn trigger_sentence_chunk_splitter_keeps_next_end_step_schedule_with_trigger() {
        let tokens = lex_line(
            "Whenever this creature enters, gain control of target creature an opponent controls until end of turn. Untap that creature. At the beginning of the next end step, target land deals 3 damage to that creature.",
            0,
        )
        .expect("rewrite lexer should classify next-end-step followup line");

        let chunks = split_trigger_sentence_chunks_rewrite_lexed(&tokens)
            .into_iter()
            .map(|chunk| render_token_slice(&chunk))
            .collect::<Vec<_>>();

        assert_eq!(
            chunks,
            vec![
                "Whenever this creature enters, gain control of target creature an opponent controls until end of turn. Untap that creature. At the beginning of the next end step, target land deals 3 damage to that creature"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn trigger_sentence_chunk_splitter_keeps_one_or_more_this_way_followup_with_trigger() {
        let tokens = lex_line(
            "When this creature enters, you may sacrifice up to three Zombies. When you sacrifice one or more Zombies this way, each opponent sacrifices that many creatures of their choice.",
            0,
        )
        .expect("rewrite lexer should classify one-or-more this-way followup line");

        let chunks = split_trigger_sentence_chunks_rewrite_lexed(&tokens)
            .into_iter()
            .map(|chunk| render_token_slice(&chunk))
            .collect::<Vec<_>>();

        assert_eq!(
            chunks,
            vec![
                "When this creature enters, you may sacrifice up to three Zombies. When you sacrifice one or more Zombies this way, each opponent sacrifices that many creatures of their choice"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn named_source_rewrite_covers_trigger_bodies_as_well_as_heads() {
        let card = CardBuilder::new(CardId::from_raw(1), "Kain, Traitorous Dragoon")
            .card_types(vec![CardType::Creature]);

        let rewritten = normalize_named_source_trigger_for_builder(
            &card,
            "Whenever Kain deals combat damage to a player, that player gains control of Kain. If they do, you draw that many cards, create that many tapped Treasure tokens, then lose that much life.",
        )
        .expect("expected named source rewrite to apply");

        assert!(
            rewritten.contains("whenever this creature deals combat damage to a player")
                && rewritten.contains("that player gains control of this creature")
                && rewritten.contains("if they do, you draw that many cards")
                && rewritten.contains("lose that much life"),
            "expected source-name rewrite to normalize the whole triggered line, got {rewritten}"
        );
    }

    #[test]
    fn named_source_rewrite_preserves_name_override_but_rewrites_later_subject() {
        let card = CardBuilder::new(CardId::from_raw(1), "Gogo, Mysterious Mime")
            .card_types(vec![CardType::Creature]);
        let text = "At the beginning of combat on your turn, you may have Gogo become a copy of another target creature you control until end of turn, except its name is Gogo, Mysterious Mime. If you do, Gogo and that creature each get +2/+0 and gain haste until end of turn and attack this turn if able.";

        let rewritten = normalize_named_source_trigger_for_builder(&card, text)
            .expect("the named source trigger body should be rewritten");
        assert!(
            rewritten.contains("except its name is gogo, mysterious mime")
                && rewritten.contains("if you do, this creature and that creature each get"),
            "{rewritten}"
        );
        let preprocessed = preprocess_document(card, text).expect("preprocess Gogo trigger");
        let Some(PreprocessedItem::Line(line)) = preprocessed.items.first() else {
            panic!("expected one Gogo trigger line");
        };
        let rewritten_line =
            super::rewrite_line_normalized(line, &rewritten).expect("rewrite normalized Gogo line");
        let (_, effect_tokens) = super::grammar::split_lexed_once_on_comma(&rewritten_line.tokens)
            .expect("Gogo trigger has an outer trigger comma");
        super::parse_effect_sentences_lexed(effect_tokens)
            .expect("Gogo's complete resolution program should parse");
        super::recognize_triggered_line(&rewritten_line)
            .expect("rewritten Gogo trigger should parse as one ability");
    }

    #[test]
    fn comma_bearing_full_source_name_is_normalized_before_trigger_split() {
        let card = CardBuilder::new(CardId::from_raw(1), "Example, Grim Manipulator")
            .card_types(vec![CardType::Creature]);
        let text = "When Example, Grim Manipulator enters, you and target opponent each secretly choose a creature that player controls. Then those choices are revealed, and that player sacrifices those creatures.";

        let rewritten = normalize_named_source_trigger_for_builder(&card, text)
            .expect("the leading full source name should be normalized");
        assert_eq!(
            rewritten,
            "when this creature enters, you and target opponent each secretly choose a creature that player controls. then those choices are revealed, and that player sacrifices those creatures."
        );

        let preprocessed =
            preprocess_document(card, text).expect("the trigger fixture should preprocess");
        let Some(PreprocessedItem::Line(line)) = preprocessed.items.first() else {
            panic!("expected one preprocessed trigger line");
        };
        let rewritten_line =
            super::rewrite_line_normalized(line, &rewritten).expect("rewrite normalized trigger");
        let (direct, direct_trace) =
            crate::parse_trace::capture(|| super::recognize_triggered_line(&rewritten_line));
        direct.unwrap_or_else(|error| {
            panic!(
                "the normalized trigger should parse directly: {error}\n{}",
                direct_trace.render()
            )
        });
        let (dispatch, trace) = crate::parse_trace::capture(|| {
            super::try_parse_triggered_line_dispatch(&preprocessed, 0, line, false)
        });
        let dispatch = dispatch
            .unwrap_or_else(|error| {
                panic!(
                    "the trigger dispatch should not fail: {error}\n{}",
                    trace.render()
                )
            })
            .expect("the trigger family should claim the line");
        let Some(RecognizedLine::Triggered(triggered)) = dispatch.lines.first() else {
            panic!("expected a triggered recognized form");
        };
        assert_eq!(
            render_token_slice(&triggered.trigger_parse_tokens),
            "Example, Grim Manipulator enters"
        );
        assert_eq!(
            render_token_slice(&triggered.effect_parse_tokens),
            "you and target opponent each secretly choose a creature that player controls. then those choices are revealed, and that player sacrifices those creatures."
        );
    }

    #[test]
    fn comma_bearing_named_trigger_rewrite_does_not_parse_reminder_text_as_effects() {
        let card = CardBuilder::new(CardId::from_raw(1), "Sophina, Spearsage Deserter")
            .card_types(vec![CardType::Creature]);
        let text = "Whenever Sophina, Spearsage Deserter attacks, investigate once for each nontoken attacking creature. (To investigate, create a Clue token. It's an artifact with \"{2}, Sacrifice this artifact: Draw a card.\")";

        let preprocessed =
            preprocess_document(card, text).expect("the Sophina trigger should preprocess");
        let Some(PreprocessedItem::Line(line)) = preprocessed.items.first() else {
            panic!("expected one preprocessed trigger line");
        };
        let dispatch = super::try_parse_triggered_line_dispatch(&preprocessed, 0, line, false)
            .expect("the trigger dispatch should not fail")
            .expect("the trigger family should claim the line");
        let Some(RecognizedLine::Triggered(triggered)) = dispatch.lines.first() else {
            panic!("expected a triggered recognized form");
        };

        assert_eq!(
            render_token_slice(&triggered.effect_parse_tokens),
            "investigate once for each nontoken attacking creature."
        );
    }

    #[test]
    fn named_source_fallback_restores_authored_trigger_subject() {
        let card = CardBuilder::new(CardId::from_raw(1), "God-Eternal Rhonas")
            .card_types(vec![CardType::Creature]);
        let text = "When God-Eternal Rhonas dies or is put into exile from the battlefield, you may put it into its owner's library third from the top.";

        let preprocessed =
            preprocess_document(card, text).expect("the trigger fixture should preprocess");
        let Some(PreprocessedItem::Line(line)) = preprocessed.items.first() else {
            panic!("expected one preprocessed trigger line");
        };
        let dispatch = super::try_parse_triggered_line_dispatch(&preprocessed, 0, line, false)
            .expect("the trigger dispatch should not fail")
            .expect("the trigger family should claim the line");
        let Some(RecognizedLine::Triggered(triggered)) = dispatch.lines.first() else {
            panic!("expected a triggered recognized form");
        };

        assert_eq!(
            render_token_slice(&triggered.trigger_parse_tokens),
            "God-Eternal Rhonas dies or is put into exile from the battlefield"
        );
    }

    #[test]
    fn named_source_rewrite_covers_trigger_body_when_head_needs_no_rewrite() {
        let card = CardBuilder::new(CardId::from_raw(1), "Rayne, Academy Chancellor")
            .card_types(vec![CardType::Creature]);

        let rewritten = normalize_named_source_trigger_for_builder(
            &card,
            "Whenever a permanent you control becomes the target of a spell or ability an opponent controls, you may draw a card. You may draw an additional card if Rayne is enchanted.",
        )
        .expect("expected body-only named source rewrite to apply");

        assert!(
            rewritten.contains("you may draw an additional card if this creature is enchanted"),
            "expected source name in trigger body condition to normalize, got {rewritten}"
        );
    }

    #[test]
    fn named_source_rewrite_keeps_compound_and_name_as_one_effect_subject() {
        let card = CardBuilder::new(CardId::from_raw(1), "Firesong and Sunspeaker")
            .card_types(vec![CardType::Creature]);

        let rewritten = normalize_named_source_trigger_for_builder(
            &card,
            "Whenever a white instant or sorcery spell causes you to gain life, Firesong and Sunspeaker deals 3 damage to target creature or player.",
        )
        .expect("expected compound named source in the trigger body to normalize");

        assert_eq!(
            rewritten,
            "whenever a white instant or sorcery spell causes you to gain life, this creature deals 3 damage to target creature or player."
        );
    }

    #[test]
    fn named_source_rewrite_normalizes_short_name_in_labeled_tivit_trigger_head() {
        let card = CardBuilder::new(CardId::from_raw(1), "Tivit, Seller of Secrets");

        let rewritten = normalize_named_source_trigger_for_builder(
            &card,
            "Whenever Tivit enters the battlefield or deals combat damage to a player, starting with you, each player votes for evidence or bribery.",
        )
        .expect("expected named source rewrite to apply");

        assert!(
            rewritten.starts_with(
                "whenever this permanent enters the battlefield or deals combat damage to a player,"
            ),
            "expected short source name to normalize, got {rewritten}"
        );
    }

    #[test]
    fn labeled_ability_word_dispatch_normalizes_a_named_source_trigger() {
        let card = CardBuilder::new(CardId::from_raw(1), "Rose Tyler")
            .card_types(vec![CardType::Creature]);
        let text = "Bad Wolf — Whenever Rose Tyler attacks, put a time counter on it for each suspended card you own and each other permanent you control with a time counter on it.";
        let preprocessed = preprocess_document(card, text).expect("preprocess labeled trigger");
        let Some(PreprocessedItem::Line(line)) = preprocessed.items.first() else {
            panic!("expected one labeled trigger line");
        };
        let (label, _, body) = super::split_label_prefix_lexed(&line.info.source_tokens)
            .unwrap_or_else(|| {
                panic!(
                    "expected source label split for '{}'",
                    render_token_slice(&line.info.source_tokens)
                )
            });
        assert_eq!(label, "Bad Wolf");
        assert!(
            render_token_slice(body).starts_with("Whenever Rose Tyler attacks"),
            "{}",
            render_token_slice(body)
        );
        let rewritten = normalize_named_source_trigger_for_builder(
            &preprocessed.card,
            render_token_slice(body).as_str(),
        )
        .expect("named source should normalize inside the labeled body");
        let rewritten_tokens =
            crate::lexer::lex_line(&rewritten, 0).expect("rewritten trigger should lex");
        let (_, rewritten_effect) = super::grammar::split_lexed_once_on_comma(&rewritten_tokens)
            .expect("rewritten trigger should have an effect comma");
        crate::effect_sentences::parse_effect_sentence_lexed(rewritten_effect)
            .expect("compound suspended-card count effect should parse atomically");
        let rewritten_line = super::rewrite_line_normalized(line, &rewritten)
            .expect("rewritten labeled body should lex");
        super::recognize_triggered_line(&rewritten_line)
            .expect("rewritten labeled body should parse as a trigger");

        let dispatch = super::try_parse_labeled_line_dispatch(&preprocessed, 0, line, false)
            .expect("labeled trigger dispatch should not fail")
            .expect("ability-word route should claim the labeled trigger");
        let Some(RecognizedLine::Triggered(triggered)) = dispatch.lines.first() else {
            panic!("expected a triggered recognized form");
        };
        assert_eq!(
            render_token_slice(&triggered.trigger_parse_tokens),
            "Rose Tyler attacks"
        );
        assert_eq!(
            triggered.presentation,
            Some(PresentationLabel::AbilityWord("Bad Wolf".to_string()))
        );
        assert!(
            render_token_slice(&triggered.effect_parse_tokens)
                .starts_with("put a time counter on it for each suspended card"),
            "{}",
            render_token_slice(&triggered.effect_parse_tokens)
        );
    }

    #[test]
    fn labeled_static_dispatch_normalizes_a_named_source_with_its_card_type() {
        let card =
            CardBuilder::new(CardId::from_raw(1), "Shao Jun").card_types(vec![CardType::Creature]);
        let text = "Leap Strike — During your turn, Shao Jun has flying and first strike.";
        let preprocessed = preprocess_document(card, text).expect("preprocess labeled static");
        let Some(PreprocessedItem::Line(line)) = preprocessed.items.first() else {
            panic!("expected one labeled static line");
        };

        let dispatch = super::try_parse_labeled_line_dispatch(&preprocessed, 0, line, false)
            .expect("labeled static dispatch should not fail")
            .expect("ability-word route should claim the labeled static line");
        let Some(RecognizedLine::Static(static_line)) = dispatch.lines.first() else {
            panic!("expected a static recognized form");
        };
        assert_eq!(
            render_token_slice(&static_line.parse_tokens),
            "during your turn, this creature has flying and first strike."
        );
        assert_eq!(
            static_line
                .info
                .semantic_facts
                .static_ability
                .presentation_label,
            Some(PresentationLabel::AbilityWord("Leap Strike".to_string()))
        );

        let debug = format!(
            "{:#?}",
            crate::keyword_static::parse_static_ability_ast_line_lexed(&static_line.parse_tokens)
                .expect("typed static parse")
                .expect("labeled static body should remain typed")
        );
        assert!(debug.contains("DuringYourTurn"), "{debug}");
    }

    #[test]
    fn labeled_multi_static_dispatch_retains_its_authored_ability_word() {
        let card =
            CardBuilder::new(CardId::from_raw(1), "Toxicrene").card_types(vec![CardType::Creature]);
        let text = "Hypertoxic Miasma — All lands have \"{T}: Add one mana of any color\" and lose all other abilities.";
        let preprocessed = preprocess_document(card, text).expect("preprocess labeled static");
        let Some(PreprocessedItem::Line(line)) = preprocessed.items.first() else {
            panic!("expected one labeled static line");
        };

        let dispatch = super::try_parse_labeled_line_dispatch(&preprocessed, 0, line, false)
            .expect("labeled static dispatch should not fail")
            .expect("ability-word route should claim the labeled static line");
        let Some(RecognizedLine::Static(static_line)) = dispatch.lines.first() else {
            panic!("expected a static recognized form");
        };
        assert_eq!(
            static_line
                .info
                .semantic_facts
                .static_ability
                .presentation_label,
            Some(PresentationLabel::AbilityWord(
                "Hypertoxic Miasma".to_string()
            ))
        );

        let recognized = super::recognize_document(&preprocessed, false)
            .expect("full document recognition should succeed");
        let [RecognizedLine::Static(recognized_static)] = recognized.lines.as_slice() else {
            panic!("expected one static line from full recognition");
        };
        assert_eq!(
            recognized_static
                .info
                .semantic_facts
                .static_ability
                .presentation_label,
            Some(PresentationLabel::AbilityWord(
                "Hypertoxic Miasma".to_string()
            ))
        );
    }

    #[test]
    fn quoted_token_reminder_does_not_claim_a_multi_sentence_spell_line() {
        let card = CardBuilder::new(CardId::from_raw(1), "Life And Token Probe")
            .card_types(vec![CardType::Sorcery]);
        let text = "Target player loses 3 life. You gain 3 life and create three 0/1 colorless Eldrazi Spawn creature tokens. They have \"Sacrifice this token: Add {C}.\"";
        let preprocessed = preprocess_document(card, text).expect("preprocess spell line");
        let Some(PreprocessedItem::Line(line)) = preprocessed.items.first() else {
            panic!("expected one spell line");
        };
        let mut lines = Vec::new();
        assert!(
            !super::try_push_complete_typed_quoted_gain_statement(line, &mut lines)
                .expect("quoted-gain fast path should not error"),
            "the token reminder belongs to the complete spell program"
        );
        assert!(lines.is_empty());
    }

    #[test]
    fn power_damage_leaf_does_not_claim_a_multi_sentence_target_program() {
        let card = CardBuilder::new(CardId::from_raw(1), "Power Fanout Probe")
            .card_types(vec![CardType::Sorcery]);
        let text = "Choose target creature you control. It deals damage equal to its power to each other creature. If this spell was cast from a graveyard, discard your hand and draw four cards.";
        let preprocessed = preprocess_document(card, text).expect("preprocess spell line");
        let Some(PreprocessedItem::Line(line)) = preprocessed.items.first() else {
            panic!("expected one spell line");
        };
        let mut lines = Vec::new();
        assert!(
            !super::try_push_complete_typed_statement(line, &mut lines)
                .expect("typed statement front door should not error"),
            "the sentence compositor owns the complete target/fanout/conditional program"
        );
        assert!(lines.is_empty());
    }

    #[test]
    fn paid_label_create_followup_reaches_the_complete_statement_parser() {
        let card = CardBuilder::new(CardId::from_raw(1), "Perch Protection")
            .card_types(vec![CardType::Instant]);
        let text = "Create four 2/2 blue Bird creature tokens with flying. If the gift was promised, all permanents you control phase out, and until your next turn, your life total can't change and you gain protection from everything.";
        let preprocessed = preprocess_document(card, text).expect("preprocess spell line");
        let Some(PreprocessedItem::Line(line)) = preprocessed.items.first() else {
            panic!("expected one spell line");
        };
        let mut early_lines = Vec::new();
        assert!(
            !super::try_push_complete_typed_statement(line, &mut early_lines)
                .expect("typed statement front door should not error"),
            "the sentence compositor owns the create plus paid-label followup"
        );
        assert!(early_lines.is_empty());

        let statement = recognize_statement_line(line)
            .expect("statement recognition should not error")
            .expect("statement should be recognized");
        let effects = statement
            .parsed_effects
            .expect("multi-sentence statement should retain typed effects");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("PhaseOutAll"), "{debug}");
        assert!(debug.contains("ChangeLifeTotal"), "{debug}");
        assert!(debug.contains("PreventAllDamageToTarget"), "{debug}");
    }

    #[test]
    fn named_source_sentence_rewrite_normalizes_short_legendary_name_in_as_enters_line() {
        let card = CardBuilder::new(CardId::from_raw(1), "Shimatsu the Bloodcloaked")
            .card_types(vec![CardType::Creature]);

        let rewritten = normalize_named_source_sentence_for_builder(
            &card,
            "As Shimatsu enters, sacrifice any number of permanents. Shimatsu enters with that many +1/+1 counters on it.",
        )
        .expect("expected short legendary source name to normalize");

        assert_eq!(
            rewritten,
            "as this creature enters, sacrifice any number of permanents. this creature enters with that many +1/+1 counters on it."
        );
    }

    #[test]
    fn saga_chapter_normalizes_a_named_source_subject_before_effect_parsing() {
        let card = CardBuilder::new(CardId::from_raw(1), "Ifrit, Warden of Inferno")
            .card_types(vec![CardType::Enchantment, CardType::Creature])
            .subtypes(vec![Subtype::Saga, Subtype::Demon]);

        let (document, _) = parse_text_to_semantic_document(
            card,
            "I — Lunge — Ifrit fights up to one other target creature.".to_string(),
            false,
        )
        .expect("named Saga source should compile through the chapter route");
        let debug = format!("{document:#?}");
        assert!(debug.contains("Fight"), "{debug}");
        assert!(debug.contains("Source"), "{debug}");
    }

    #[test]
    fn named_source_sentence_rewrite_uses_preprocessed_untyped_as_enters_subject() {
        let card = CardBuilder::new(CardId::from_raw(1), "Shimatsu the Bloodcloaked");

        let rewritten = normalize_named_source_sentence_for_builder(
            &card,
            "as this enters, sacrifice any number of permanents. shimatsu enters with that many +1/+1 counters on it.",
        )
        .expect("expected the preserved follow-up short name to normalize");

        assert_eq!(
            rewritten,
            "as this enters, sacrifice any number of permanents. this enters with that many +1/+1 counters on it."
        );
    }

    #[test]
    fn named_source_sentence_rewrite_keeps_filtered_entry_subject_with_named_value_reference() {
        let card = CardBuilder::new(CardId::from_raw(1), "Arwen, Weaver of Hope")
            .card_types(vec![CardType::Creature]);

        assert_eq!(
            normalize_named_source_sentence_for_builder(
                &card,
                "Each other creature you control enters with a number of additional +1/+1 counters on it equal to Arwen's toughness.",
            ),
            None,
            "a later named value reference must not turn a filtered entry rule into a self-entry rule"
        );
    }

    #[test]
    fn named_source_sentence_rewrite_preserves_named_characteristic_subject() {
        let card = CardBuilder::new(CardId::from_raw(1), "Tidewalker")
            .card_types(vec![CardType::Creature]);

        assert_eq!(
            normalize_named_source_sentence_for_builder(
                &card,
                "Tidewalker's power and toughness are each equal to the number of time counters on it.",
            ),
            None,
            "the characteristic parser needs the authored proper-name subject"
        );
    }

    #[test]
    fn named_source_sentence_rewrite_still_normalizes_leading_short_alias_with_named_reference() {
        let card = CardBuilder::new(CardId::from_raw(1), "Brago, King Eternal")
            .card_types(vec![CardType::Creature]);

        let rewritten = normalize_named_source_sentence_for_builder(
            &card,
            "Brago enters with a number of +1/+1 counters equal to Brago's power.",
        )
        .expect("a leading short source alias should normalize");

        assert_eq!(
            rewritten,
            "this creature enters with a number of +1/+1 counters equal to brago's power."
        );
    }

    #[test]
    fn named_source_rewrite_preserves_short_alias_used_as_effect_verb() {
        assert_eq!(
            replace_named_source_aliases(
                "Search your library for a card, reveal it, then shuffle.",
                "search",
                "this permanent",
            ),
            "search your library for a card, reveal it, then shuffle."
        );
    }

    #[test]
    fn named_source_rewrite_preserves_combat_damage_rules_term() {
        assert_eq!(
            replace_named_source_aliases(
                "Whenever this creature deals combat damage to a player, draw a card.",
                "combat",
                "this enchantment",
            ),
            "whenever this creature deals combat damage to a player, draw a card."
        );
    }

    #[test]
    fn named_source_rewrite_preserves_short_alias_used_as_created_token_name() {
        assert_eq!(
            replace_named_source_aliases(
                "Create Mechtitan, a legendary 10/10 Construct artifact creature token with flying.",
                "mechtitan",
                "this artifact",
            ),
            "create mechtitan, a legendary 10/10 construct artifact creature token with flying."
        );
        assert_eq!(
            replace_named_source_aliases("Mechtitan has flying.", "mechtitan", "this artifact",),
            "this artifact has flying.",
            "ordinary source-name subjects must still normalize"
        );
    }

    #[test]
    fn created_token_name_does_not_hide_source_references_in_later_sentences() {
        let card =
            CardBuilder::new(CardId::from_raw(1), "Stangg").card_types(vec![CardType::Creature]);
        let rewritten = normalize_named_source_trigger_for_builder(
            &card,
            "When Stangg enters, create Stangg Twin, a legendary 3/4 red and green Human Warrior creature token. Exile that token when Stangg leaves the battlefield. Sacrifice Stangg when that token leaves the battlefield.",
        )
        .expect("named source trigger should normalize");

        assert_eq!(
            rewritten,
            "when this creature enters, create stangg twin, a legendary 3/4 red and green human warrior creature token. exile that token when this creature leaves the battlefield. sacrifice this creature when that token leaves the battlefield."
        );
    }

    #[test]
    fn named_source_rewrite_preserves_registered_subtype_in_token_wording() {
        assert_eq!(
            replace_named_source_aliases(
                "Create a 0/1 white Caribou creature token.",
                "caribou",
                "this enchantment",
            ),
            "create a 0/1 white caribou creature token."
        );
        assert_eq!(
            replace_named_source_aliases(
                "Sacrifice a Caribou token: You gain 1 life.",
                "caribou",
                "this enchantment",
            ),
            "sacrifice a caribou token: you gain 1 life."
        );
        assert_eq!(
            replace_named_source_aliases("Caribou has flying.", "caribou", "this enchantment",),
            "this enchantment has flying.",
            "ordinary source references must still normalize",
        );
    }

    #[test]
    fn named_source_rewrite_preserves_short_alias_used_as_counter_type() {
        assert_eq!(
            replace_named_source_aliases(
                "Put up to three lore counters on target Saga.",
                "lore",
                "this permanent",
            ),
            "put up to three lore counters on target saga."
        );
    }

    #[test]
    fn named_source_rewrite_preserves_short_alias_used_as_modal_may() {
        assert_eq!(
            replace_named_source_aliases(
                "Reveal the top five cards of your library. You may put one of them into your hand.",
                "may",
                "this permanent",
            ),
            "reveal the top five cards of your library. you may put one of them into your hand."
        );
    }

    #[test]
    fn named_source_rewrite_preserves_authored_source_subject_before_gets() {
        assert_eq!(
            replace_named_source_aliases(
                "Choose one and Glorfindel gets +1/+1 until end of turn.",
                "glorfindel",
                "this creature",
            ),
            "choose one and glorfindel gets +1/+1 until end of turn."
        );
        assert_eq!(
            replace_named_source_aliases(
                "Glorfindel has vigilance.",
                "glorfindel",
                "this creature",
            ),
            "this creature has vigilance.",
            "the authored-name exception is restricted to get/gets effects"
        );
    }

    #[test]
    fn named_source_rewrite_preserves_alias_used_as_indefinite_become_descriptor() {
        assert_eq!(
            replace_named_source_aliases(
                "Target creature becomes a Coward in addition to its other types until end of turn.",
                "coward",
                "this sorcery",
            ),
            "target creature becomes a coward in addition to its other types until end of turn."
        );
    }

    #[test]
    fn named_source_rewrite_preserves_alias_used_as_typed_creature_subtype_noun() {
        assert_eq!(
            replace_named_source_aliases(
                "Until end of turn, target Time Lord you control gains vigilance.",
                "time lord",
                "this permanent",
            ),
            "until end of turn, target time lord you control gains vigilance."
        );
        assert_eq!(
            replace_named_source_aliases(
                "Reveal cards until you reveal a Time Lord creature card.",
                "time lord",
                "this permanent",
            ),
            "reveal cards until you reveal a time lord creature card."
        );
        assert_eq!(
            replace_named_source_aliases(
                "Time Lord gains vigilance.",
                "time lord",
                "this permanent",
            ),
            "time lord gains vigilance.",
            "the existing authored-name effect surface remains independent of subtype grammar"
        );
        assert_eq!(
            replace_named_source_aliases(
                "Until end of turn, target Time Lord you control gains vigilance and reveal until you reveal a Time Lord creature card.",
                "time",
                "this permanent",
            ),
            "until end of turn, target time lord you control gains vigilance and reveal until you reveal a time lord creature card.",
            "a one-word source alias must not consume the first word of a compound creature subtype"
        );
    }

    #[test]
    fn named_source_rewrite_preserves_alias_after_cards_named() {
        assert_eq!(
            replace_named_source_aliases(
                "You removed a creature card with flying from the draft with cards named Draft Mimic.",
                "draft mimic",
                "this creature",
            ),
            "you removed a creature card with flying from the draft with cards named draft mimic."
        );
        assert_eq!(
            replace_named_source_aliases("Draft Mimic has flying.", "draft mimic", "this creature",),
            "this creature has flying.",
            "ordinary source references must still normalize",
        );
    }

    #[test]
    fn named_source_rewrite_preserves_control_as_the_gain_control_action_noun() {
        assert_eq!(
            replace_named_source_aliases(
                "The player to your right gains control of this artifact.",
                "control",
                "this permanent",
            ),
            "the player to your right gains control of this artifact."
        );
        assert_eq!(
            replace_named_source_aliases_from_set(
                "The player to your right gains control of this artifact.",
                "control",
                "this permanent",
                &["control".to_string()],
                false,
            ),
            "the player to your right gains control of this artifact.",
            "explicit source normalization must also preserve the gain-control rules term",
        );
    }

    #[test]
    fn named_source_rewrite_preserves_possessive_suffix() {
        assert_eq!(
            replace_named_source_aliases(
                "Hold for Ransom's controller sacrifices it.",
                "hold for ransom",
                "this enchantment",
            ),
            "this enchantment's controller sacrifices it."
        );
    }

    #[test]
    fn named_source_rewrite_does_not_rewrite_prefix_of_preserved_full_name() {
        let card = CardBuilder::new(CardId::from_raw(1), "Vivi Ornitier")
            .card_types(vec![CardType::Creature]);
        let aliases = source_name_aliases_for_builder(&card);

        let mut full_name_surface = "where x is vivi ornitier's power".to_string();
        for alias in &aliases {
            full_name_surface = replace_named_source_aliases_from_set(
                &full_name_surface,
                alias,
                "this creature",
                &aliases,
                true,
            );
        }
        assert_eq!(
            full_name_surface, "where x is vivi ornitier's power",
            "a preserved full-name surface must not be partially rewritten by its short alias"
        );

        let mut short_name_surface = "copy vivi".to_string();
        for alias in &aliases {
            short_name_surface = replace_named_source_aliases_from_set(
                &short_name_surface,
                alias,
                "this creature",
                &aliases,
                true,
            );
        }
        assert_eq!(
            short_name_surface, "copy this creature",
            "a standalone short source alias should still normalize"
        );
    }

    #[test]
    fn named_source_rewrite_preserves_vote_option_alias() {
        let card = CardBuilder::new(CardId::from_raw(1), "Truth or Consequences")
            .card_types(vec![CardType::Sorcery]);

        let rewritten = normalize_named_source_sentence_for_builder(
            &card,
            "Each player secretly votes for truth or consequences, then those votes are revealed. You draw cards equal to the number of truth votes. Truth or Consequences can't be countered.",
        )
        .expect("expected source name to normalize outside the vote option");

        assert!(
            rewritten.contains("votes for truth or consequences"),
            "expected vote option alias to remain named, got {rewritten}"
        );
        assert!(
            rewritten.contains("number of truth votes"),
            "expected later vote-count references to remain named, got {rewritten}"
        );
        assert!(
            rewritten.contains("this permanent can't be countered"),
            "expected non-option source alias to normalize, got {rewritten}"
        );
    }

    #[test]
    fn rewritten_tivit_vote_trigger_candidate_parses_as_triggered_recognized() {
        let line = single_preprocessed_line(
            "Whenever this creature enters the battlefield or deals combat damage to a player, starting with you, each player votes for evidence or bribery. For each evidence vote, investigate. For each bribery vote, create a Treasure token. You may vote an additional time.",
        );

        recognize_triggered_line(&line)
            .expect("rewritten Tivit trigger should parse as recognized form");
    }

    #[test]
    fn named_source_leaves_trigger_keeps_original_head_for_surface_rendering() {
        let document = preprocess_document(
            CardBuilder::new(CardId::from_raw(1), "Emrakul, the World Anew")
                .card_types(vec![CardType::Creature]),
            "When Emrakul leaves the battlefield, sacrifice all creatures you control.",
        )
        .expect("named source leaves line should preprocess");
        let line = match document.items.first().expect("expected one line") {
            PreprocessedItem::Line(line) => line,
            other => panic!("expected preprocessed line, got {other:?}"),
        };

        let parsed = try_parse_triggered_line_with_named_source_rewrite(
            &document.card,
            line,
            &line.info.source_tokens,
        )
        .expect("named source leaves rewrite should not fail")
        .expect("named source leaves trigger should parse as triggered recognized form");

        assert_eq!(
            render_token_slice(&parsed.trigger_parse_tokens),
            "Emrakul leaves the battlefield"
        );
    }

    #[test]
    fn reveal_first_draw_splitter_reuses_token_ranges() {
        let tokens = lex_line(
            "Reveal the first card you draw each turn. Whenever you reveal an instant card this way, draw a card.",
            0,
        )
        .expect("rewrite lexer should classify reveal-first-draw line");

        let chunks = split_reveal_first_draw_line_rewrite_lexed(&tokens)
            .expect("expected reveal-first-draw splitter to match")
            .into_iter()
            .map(|chunk| render_token_slice(&chunk))
            .collect::<Vec<_>>();

        assert_eq!(
            chunks,
            vec![
                "Reveal the first card you draw each turn".to_string(),
                "Whenever you reveal an instant card this way, draw a card".to_string(),
            ]
        );
    }

    #[test]
    fn trailing_keyword_activation_splitter_reuses_token_ranges() {
        let tokens = lex_line(
            "draw a card. cycling — {2}, discard this card: draw a card.",
            0,
        )
        .expect("rewrite lexer should classify trailing keyword activation line");

        let (prefix, suffix) = normalize_trailing_keyword_activation_sentence_lexed(&tokens)
            .expect("expected trailing keyword activation split");

        assert_eq!(render_token_slice(&prefix), "draw a card");
        assert_eq!(
            render_token_slice(&suffix),
            "cycling — {2}, discard this card: draw a card"
        );
    }

    #[test]
    fn activation_text_parts_lexed_reuse_existing_token_split() {
        let tokens = lex_line("{2}, discard this card: draw a card.", 0)
            .expect("rewrite lexer should classify activation text");

        let (cost_tokens, effect_text) =
            split_activation_text_parts_lexed(&tokens).expect("expected activation text split");

        assert_eq!(render_token_slice(&cost_tokens), "{2}, discard this card");
        assert_eq!(effect_text, "draw a card.");
    }

    #[test]
    fn activated_line_recognized_stores_cost_and_effect_parse_tokens() -> Result<(), CardTextError>
    {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Activated Parse Tokens Test")
                .card_types(vec![CardType::Artifact]),
            "{T}: Draw a card.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        match recognized.lines.as_slice() {
            [super::RecognizedLine::Activated(activated)] => {
                assert_eq!(render_token_slice(&activated.cost_parse_tokens), "{t}");
                assert_eq!(
                    render_token_slice(&activated.effect_parse_tokens),
                    "draw a card."
                );
            }
            other => panic!("expected one activated line, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn saga_can_gain_a_quoted_filtered_mana_ability() -> Result<(), CardTextError> {
        CardDefinitionBuilder::new(CardId::new(), "Quoted Filtered Grant Saga")
            .card_types(vec![CardType::Enchantment])
            .parse_text(
                "II — This Saga gains \"Creatures you control have '{T}: Add {R}, {G}, or {W}.'\"",
            )?;
        Ok(())
    }

    #[test]
    fn reveal_first_draw_line_family_parses_through_document_recognized()
    -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Reveal First Draw Split Test")
                .card_types(vec![CardType::Enchantment]),
            "Reveal the first card you draw each turn. Whenever you reveal an instant card this way, draw a card.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        match recognized.lines.as_slice() {
            [
                super::RecognizedLine::Static(static_line),
                super::RecognizedLine::Triggered(triggered),
            ] => {
                assert_eq!(
                    render_token_slice(&static_line.parse_tokens),
                    "reveal the first card you draw each turn"
                );
                assert_eq!(
                    render_token_slice(&triggered.trigger_parse_tokens),
                    "you reveal an instant card this way"
                );
                assert_eq!(
                    render_token_slice(&triggered.effect_parse_tokens),
                    "draw a card"
                );
            }
            other => {
                panic!("expected static plus triggered reveal-first-draw split, got {other:?}")
            }
        }

        Ok(())
    }

    #[test]
    fn championed_with_this_trigger_rewrite_parses_through_document_recognized()
    -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Champion Trigger Rewrite Test")
                .card_types(vec![CardType::Creature]),
            "When a creature is championed with this creature, draw a card.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        match recognized.lines.as_slice() {
            [super::RecognizedLine::Triggered(triggered)] => {
                assert_eq!(
                    triggered.full_text,
                    "When this creature enters, draw a card"
                );
                assert_eq!(
                    render_token_slice(&triggered.trigger_parse_tokens),
                    "this creature enters"
                );
                assert_eq!(
                    render_token_slice(&triggered.effect_parse_tokens),
                    "draw a card"
                );
            }
            other => panic!("expected rewritten championed-with-this trigger, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn modal_mode_recognized_stores_parsed_effects_ast() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Modal Parse Tokens Test")
                .card_types(vec![CardType::Instant]),
            "Choose one —\n• Meteor Strikes — Draw a card.\n• Final Heaven — Gain 3 life.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        match recognized.lines.as_slice() {
            [super::RecognizedLine::Modal(modal)] => {
                assert_eq!(modal.modes.len(), 2);
                assert_eq!(modal.modes[0].text, "Draw a card.");
                assert_eq!(modal.modes[1].text, "Gain 3 life.");
                assert!(!modal.modes[0].effects_ast.is_empty());
                assert!(!modal.modes[1].effects_ast.is_empty());
            }
            other => panic!("expected one modal block, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn modal_mode_recognized_groups_season_of_the_burrow_pawprint_modes()
    -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardBuilder::new(CardId::new(), "Season of the Burrow")
                .card_types(vec![CardType::Sorcery]),
            "Choose up to five {P} worth of modes. You may choose the same mode more than once.\n{P} — Create a 1/1 white Rabbit creature token.\n{P}{P} — Exile target nonland permanent. Its controller draws a card.\n{P}{P}{P} — Return target permanent card with mana value 3 or less from your graveyard to the battlefield with an indestructible counter on it.",
        )?;
        let recognized = super::recognize_document(&preprocessed, false)?;

        match recognized.lines.as_slice() {
            [super::RecognizedLine::Modal(modal)] => {
                assert_eq!(modal.modes.len(), 3);
                assert_eq!(
                    modal.modes[0].text,
                    "Create a 1/1 white Rabbit creature token."
                );
                assert!(
                    modal.modes[1]
                        .text
                        .starts_with("Exile target nonland permanent")
                );
                assert!(
                    modal.modes[2]
                        .text
                        .starts_with("Return target permanent card")
                );
            }
            other => panic!(
                "expected Season of the Burrow pawprint modes to form one modal block, got {other:?}"
            ),
        }

        Ok(())
    }

    #[test]
    fn unsupported_line_diagnostics_use_token_normalization() {
        let landwalk = single_preprocessed_line(
            "Creatures with islandwalk can be blocked as though they didn’t have islandwalk.",
        );
        let aura_copy = single_preprocessed_line(
            "Create a token that’s a copy of that Aura attached to that creature.",
        );

        let landwalk_error = diagnose_known_unsupported_rewrite_line(&landwalk.tokens)
            .expect("expected landwalk override diagnostic");
        let aura_copy_error = diagnose_known_unsupported_rewrite_line(&aura_copy.tokens)
            .expect("expected aura-copy diagnostic");

        assert_eq!(
            landwalk_error.to_string(),
            "unsupported landwalk override clause"
        );
        assert_eq!(
            aura_copy_error.to_string(),
            "unsupported aura-copy attachment fanout clause"
        );
    }

    #[test]
    fn looks_like_divvy_statement_probe_recognizes_pile_lines() {
        let text = "Separate all creatures target player controls into two piles. Destroy all creatures in the pile of your choice.";
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify divvy pile line");
        assert_eq!(
            classify_statement_line_family_lexed(&tokens),
            Some(StatementLineFamily::Divvy)
        );
    }

    #[test]
    fn untap_shape_probes_recognize_expected_token_patterns() {
        let your_step = lex_line("Lands you control don't untap during your untap step.", 0)
            .expect("rewrite lexer should classify your-untap-step probe");
        assert!(is_doesnt_untap_during_your_untap_step_line_lexed(
            &your_step
        ));

        let your_step_do_not = lex_line(
            "Artifacts you control do not untap during your untap step.",
            0,
        )
        .expect("rewrite lexer should classify do-not untap-step probe");
        assert!(is_doesnt_untap_during_your_untap_step_line_lexed(
            &your_step_do_not
        ));

        let other_players_text =
            "Untap all permanents you control during each other player's untap step.";
        let other_players = lex_line(other_players_text, 0)
            .expect("rewrite lexer should classify other-players untap-step probe");
        assert_eq!(
            classify_static_line_family_lexed(&other_players),
            Some(StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep)
        );
        assert!(!looks_like_statement_line(
            other_players_text.to_ascii_lowercase().as_str()
        ));
        assert!(looks_like_static_line(
            other_players_text.to_ascii_lowercase().as_str()
        ));

        let singular_other_players_text =
            "Untap this artifact during each other player's untap step.";
        let singular_other_players = lex_line(singular_other_players_text, 0)
            .expect("rewrite lexer should classify singular other-players untap-step probe");
        assert_eq!(
            classify_static_line_family_lexed(&singular_other_players),
            Some(StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep)
        );
        assert!(!looks_like_statement_line(
            singular_other_players_text.to_ascii_lowercase().as_str()
        ));
        assert!(looks_like_static_line(
            singular_other_players_text.to_ascii_lowercase().as_str()
        ));
    }

    #[test]
    fn timeless_phase_in_prohibitions_prefer_the_static_line_path() {
        for text in ["Permanents can't phase in.", "Permanents cannot phase in."] {
            let normalized = text.to_ascii_lowercase();
            assert!(
                !looks_like_statement_line(normalized.as_str()),
                "{text} must not become a resolving phase-in effect"
            );
            assert!(
                looks_like_static_line(normalized.as_str()),
                "{text} must remain a continuous rule restriction"
            );
            let line = single_preprocessed_line(text);
            let parsed_static_ast =
                crate::keyword_static::parse_static_ability_ast_line_lexed(&line.tokens);
            assert!(
                matches!(parsed_static_ast, Ok(Some(_))),
                "{text} must reach the typed static AST parser: {parsed_static_ast:#?}; words={:?}",
                crate::lexer::token_word_refs(&line.tokens)
            );
            let repeated_static_ast =
                crate::keyword_static::parse_static_ability_ast_line_lexed(&line.tokens);
            assert!(
                matches!(repeated_static_ast, Ok(Some(_))),
                "{text} must remain typed on a repeated static AST parse: \
                 {repeated_static_ast:#?}"
            );
            assert!(
                recognize_static_line(&line)
                    .expect("the static parser must not error")
                    .is_some(),
                "{text} must be claimed by the complete static parser"
            );
        }
    }

    #[test]
    fn pact_shape_probe_recognizes_next_upkeep_lose_game_line() {
        let tokens = lex_line(
            "At the beginning of your next upkeep, pay {2}{U}{U}. If you don't, you lose the game.",
            0,
        )
        .expect("rewrite lexer should classify pact next-upkeep statement line");
        assert_eq!(
            classify_statement_line_family_lexed(&tokens),
            Some(StatementLineFamily::PactNextUpkeep)
        );
    }

    #[test]
    fn labeled_prior_token_replacement_joins_adjacent_spell_lines() {
        let text = "Create two 1/1 white Human creature tokens.\nFateful hour — If you have 5 or less life, create five of those tokens instead.";
        let builder = CardDefinitionBuilder::new(CardId::new(), "Prior Token Replacement")
            .card_types(vec![CardType::Sorcery]);
        let preprocessed = preprocess_document(builder.card_builder.clone(), text)
            .expect("prior-token replacement should preprocess");
        let (recognized, recognition_trace) =
            crate::parse_trace::capture(|| super::recognize_document(&preprocessed, false));
        let recognized = recognized.unwrap_or_else(|error| {
            panic!(
                "prior-token replacement should form a document recognized form: {error}\n{}",
                recognition_trace.render()
            )
        });
        assert_eq!(recognized.lines.len(), 1, "{:#?}", recognized.lines);
        let [RecognizedLine::Statement(statement)] = recognized.lines.as_slice() else {
            panic!("expected the merged line to remain a statement: {recognized:#?}");
        };
        assert!(matches!(
            statement.parsed_effects.as_deref(),
            Some([crate::cards::builders::EffectAst::SelfReplacement { .. }])
        ));

        let (parsed, compile_trace) = crate::parse_trace::capture(|| builder.parse_text(text));
        let parsed = parsed.unwrap_or_else(|error| {
            panic!(
                "adjacent labeled prior-token replacement should compile: {error}\n{}",
                compile_trace.render()
            )
        });
        let program = parsed
            .spell_effect
            .as_ref()
            .expect("sorcery should retain one resolution program");
        let [segment] = program.segments.as_slice() else {
            panic!("expected one replacement-bearing segment: {program:#?}");
        };
        let [branch] = segment.self_replacements.as_slice() else {
            panic!("expected one typed self-replacement: {segment:#?}");
        };
        assert_eq!(
            branch
                .presentation_label
                .as_ref()
                .and_then(crate::ability::PresentationLabel::display_prefix)
                .as_deref(),
            Some("Fateful hour")
        );

        let changed_reference =
            CardDefinitionBuilder::new(CardId::new(), "Changed Prior Token Reference")
                .card_types(vec![CardType::Sorcery])
                .parse_text(
                    "Create two 1/1 white Human creature tokens.\nFateful hour — If you have 5 or less life, create five of those cards instead.",
                );
        assert!(
            changed_reference.is_err(),
            "a changed antecedent noun must not bind to the token creation"
        );
    }

    #[test]
    fn public_statement_route_keeps_each_player_return_entry_counter() {
        let text = "Each player returns each creature card from their graveyard to the battlefield with an additional -1/-1 counter on it.";
        let card = CardBuilder::new(CardId::new(), "Return Counter Probe")
            .card_types(vec![CardType::Sorcery]);
        let preprocessed = preprocess_document(card, text).expect("statement should preprocess");
        let (recognized, trace) =
            crate::parse_trace::capture(|| super::recognize_document(&preprocessed, false));
        let recognized = recognized.unwrap_or_else(|error| {
            panic!("statement recognition failed: {error}\n{}", trace.render())
        });
        let [RecognizedLine::Statement(statement)] = recognized.lines.as_slice() else {
            panic!("expected one statement line: {recognized:#?}");
        };
        let debug = format!("{:#?}", statement.parsed_effects);
        assert!(
            debug.contains("ReturnAllToBattlefield"),
            "{debug}\n{}",
            trace.render()
        );
        assert!(debug.contains("PutCounters"), "{debug}\n{}", trace.render());
        assert!(
            debug.contains("MinusOneMinusOne"),
            "{debug}\n{}",
            trace.render()
        );
    }

    #[test]
    fn public_statement_route_keeps_each_player_revealed_partition() {
        let text = "Each player reveals the top five cards of their library, puts all land cards revealed this way onto the battlefield tapped, and exiles the rest.";
        let card = CardBuilder::new(CardId::new(), "Reveal Partition Probe")
            .card_types(vec![CardType::Sorcery]);
        let preprocessed = preprocess_document(card, text).expect("statement should preprocess");
        let (recognized, trace) =
            crate::parse_trace::capture(|| super::recognize_document(&preprocessed, false));
        let recognized = recognized.unwrap_or_else(|error| {
            panic!("statement recognition failed: {error}\n{}", trace.render())
        });
        let [RecognizedLine::Statement(statement)] = recognized.lines.as_slice() else {
            panic!("expected one statement line: {recognized:#?}");
        };
        let debug = format!("{:#?}", statement.parsed_effects);
        assert!(
            debug.contains("ForEachPlayer"),
            "{debug}\n{}",
            trace.render()
        );
        assert!(
            debug.contains("ForEachTagged"),
            "{debug}\n{}",
            trace.render()
        );
        assert!(debug.contains("Land"), "{debug}\n{}", trace.render());
        assert!(debug.contains("Exile"), "{debug}\n{}", trace.render());
    }
}
