use super::*;
use crate::ZoneReplacementDurationAst;
use crate::cards::builders::GrantedAbilityAst;
use crate::grammar::abilities::{
    is_minimum_spell_total_mana_three_line_lexed, is_players_cant_pay_life_or_sacrifice_line_lexed,
};
use crate::grammar::keyword_special_lines as keyword_special_grammar;
use crate::grammar::semantic_lowering as semantic_grammar;
use crate::grammar::structure::{StatementLineFamily, classify_statement_line_family_lexed};
use crate::model::ast::{
    ChooseOneModeAst, SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbSubjectAst,
};
use crate::{KeywordAction, Value};

const STANDARD_MENACE_REMINDER: &str =
    "Menace (This creature can't be blocked except by two or more creatures.)";
const STANDARD_FLANKING_REMINDER: &str = "Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)";
const STANDARD_OPEN_ATTRACTION_REMINDER: &str =
    "(Put the top card of your Attraction deck onto the battlefield.)";

fn coalesce_adjacent_static_ability_chunks(chunks: Vec<LineAst>) -> Vec<LineAst> {
    let mut coalesced = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        match (coalesced.last_mut(), chunk) {
            (Some(LineAst::StaticAbilities(existing)), LineAst::StaticAbilities(mut following)) => {
                existing.append(&mut following)
            }
            (_, chunk) => coalesced.push(chunk),
        }
    }
    coalesced
}

fn has_standard_menace_reminder(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        crate::lexer::parser_token_word_refs(tokens).as_slice(),
        [
            "menace",
            "this",
            "creature",
            "cant" | "can't",
            "be",
            "blocked",
            "except",
            "by",
            "two",
            "or",
            "more",
            "creatures"
        ]
    )
}

fn has_standard_flanking_reminder(raw_line: &str) -> bool {
    raw_line.trim() == STANDARD_FLANKING_REMINDER
}

#[cfg(test)]
fn semantic_effects_for_test(line: &LineAst) -> Option<&[EffectAst]> {
    match line {
        LineAst::Triggered { effects, .. } => Some(effects),
        LineAst::Ability(ability) => ability.effects_ast.as_deref(),
        LineAst::Multiple(chunks) if chunks.len() == 1 => semantic_effects_for_test(&chunks[0]),
        _ => None,
    }
}

fn restore_copy_static_variant_source_display(
    abilities: &mut [crate::cards::builders::StaticAbilityAst],
    raw_line: &str,
) {
    let matching_count = abilities
        .iter()
        .filter_map(|ability| {
            let crate::cards::builders::StaticAbilityAst::Static(ability) = ability else {
                return None;
            };
            matches!(
                &ability.payload,
                ironsmith_core::StaticAbilityPayload::CopyStaticAbilityVariants(_)
            )
            .then_some(())
        })
        .count();
    if matching_count != 1 {
        return;
    }

    let display = raw_line.trim();
    if display.is_empty() {
        return;
    }
    for ability in abilities {
        let crate::cards::builders::StaticAbilityAst::Static(ability) = ability else {
            continue;
        };
        let ironsmith_core::StaticAbilityPayload::CopyStaticAbilityVariants(copy) =
            &mut ability.payload
        else {
            continue;
        };
        copy.display = display.to_string();
        ability.label = display.to_string();
    }
}

fn restore_named_characteristic_subject_surface(
    abilities: &mut [crate::cards::builders::StaticAbilityAst],
    source_tokens: &[OwnedLexToken],
) {
    if !crate::keyword_static::characteristic_pt_uses_named_subject_surface(source_tokens) {
        return;
    }
    for ability in abilities {
        let crate::cards::builders::StaticAbilityAst::Static(ability) = ability else {
            continue;
        };
        let ironsmith_core::StaticAbilityPayload::CharacteristicDefiningPt { power, toughness } =
            &mut ability.payload
        else {
            continue;
        };
        *power = power
            .clone()
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::SourceNameSubject);
        *toughness = toughness
            .clone()
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::SourceNameSubject);
    }
}

fn full_parse_tokens_have_triggered_intervening_if_clause(tokens: &[OwnedLexToken]) -> bool {
    let start_idx =
        if super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(tokens)
            .is_some()
        {
            1
        } else {
            0
        };

    super::super::grammar::structure::split_triggered_conditional_clause_lexed(tokens, start_idx)
        .is_some()
}

#[path = "lines/trigger_reconciliation.rs"]
mod trigger_reconciliation;
use trigger_reconciliation::{
    authored_dynamic_token_creation_from_trigger,
    dynamic_static_ability_count_token_creation_from_authored_trigger,
    is_gate_partition_core_word_program, is_gate_partition_word_program, is_parley_word_program,
    recognize_authored_correlated_trigger_programs,
    recognize_dynamic_zone_change_group_token_creation, recognize_serial_target_pt_modifiers,
    spell_or_activated_ability_x_cost_trigger_spec,
};
pub use trigger_reconciliation::{
    dynamic_zone_change_group_token_creation_from_authored_trigger,
    end_of_combat_destroy_then_next_end_step_counter_program,
};

fn recognize_open_attraction_reminder(line: &mut LineAst, raw_line: &str) {
    if !raw_line.contains(STANDARD_OPEN_ATTRACTION_REMINDER) {
        return;
    }
    fn mark(effects: &mut [EffectAst]) {
        for effect in effects {
            if let EffectAst::SubjectVerb(subject_verb) = effect
                && let SubjectVerbActionAst::OpenAttraction { reminder } = &mut subject_verb.action
            {
                *reminder = true;
            }
            crate::model::visit::for_each_nested_effect_vec_mut(effect, true, |nested| {
                mark(nested)
            });
        }
    }
    match line {
        LineAst::Triggered { effects, .. } => mark(effects),
        LineAst::Ability(ability) if ability.effects_ast.is_some() => {
            mark(ability.effects_ast.as_mut().expect("checked effects AST"));
        }
        _ => {}
    }
}

pub fn linked_created_token_next_turn_sacrifice_effects(
    effect_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !has_linked_created_token_next_turn_sacrifice_surface(effect_tokens) {
        return Ok(None);
    }
    let sentences = split_lexed_sentences(effect_tokens);
    let [create_sentence, delayed_sentence] = sentences.as_slice() else {
        return Ok(None);
    };
    let delayed_words = crate::lexer::parser_token_word_refs(delayed_sentence);
    let delayed_until_next_turn = crate::word_primitives::parse_sequence_complete(
        &delayed_words,
        &[
            "at",
            "the",
            "beginning",
            "of",
            "the",
            "end",
            "step",
            "on",
            "your",
            "next",
            "turn",
            "sacrifice",
            "that",
            "token",
        ],
    );
    let sacrifice_at_next_end_step = crate::word_primitives::parse_sequence_complete(
        &delayed_words,
        &[
            "sacrifice",
            "that",
            "token",
            "at",
            "the",
            "beginning",
            "of",
            "the",
            "next",
            "end",
            "step",
        ],
    );
    if !delayed_until_next_turn && !sacrifice_at_next_end_step {
        return Ok(None);
    }

    let mut created =
        match crate::effect_sentences::parse_counter_then_dynamic_token_creation_chain(
            create_sentence,
        )? {
            Some(effects) => effects,
            None => parse_effect_sentences_lexed(create_sentence)?,
        };
    if sacrifice_at_next_end_step {
        return crate::activation_and_restrictions::append_token_reminder_to_last_create_effect(
            &mut created,
            delayed_sentence,
        )
        .map(|applied| applied.then_some(created));
    }
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CreateTokenWithMods { .. },
            ..
        }),
    ] = created.as_slice()
    else {
        return Ok(None);
    };
    created.push(EffectAst::DelayedUntilEndStepOfExtraTurn {
        player: PlayerAst::You,
        effects: vec![EffectAst::subject_verb_sacrifice(
            PlayerAst::You,
            ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key()),
            1,
            None,
        )],
    });
    Ok(Some(created))
}

pub fn has_linked_created_token_next_turn_sacrifice_surface(
    effect_tokens: &[OwnedLexToken],
) -> bool {
    let sentences = split_lexed_sentences(effect_tokens);
    let [create_sentence, delayed_sentence] = sentences.as_slice() else {
        return false;
    };
    let create_words = crate::lexer::parser_token_word_refs(create_sentence);
    let delayed_words = crate::lexer::parser_token_word_refs(delayed_sentence);
    crate::word_primitives::contains_word(&create_words, "create")
        && crate::word_primitives::contains_word(&create_words, "token")
        && (crate::word_primitives::parse_sequence_complete(
            &delayed_words,
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "end",
                "step",
                "on",
                "your",
                "next",
                "turn",
                "sacrifice",
                "that",
                "token",
            ],
        ) || crate::word_primitives::parse_sequence_complete(
            &delayed_words,
            &[
                "sacrifice",
                "that",
                "token",
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "end",
                "step",
            ],
        ))
}

/// Preserve a token creation together with the reciprocal source/token
/// lifecycle established by its next two sentences. Both follow-ups contain
/// `when`, so a broad triggered-line probe can otherwise promote the final
/// embedded trigger and discard the producer plus its first delayed action.
pub fn has_created_token_reciprocal_lifecycle_surface(effect_tokens: &[OwnedLexToken]) -> bool {
    use crate::grammar::trigger_subjects::{
        TokenLifecycleSentenceKind, parse_token_lifecycle_sentence_tokens,
    };

    let sentences = split_lexed_sentences(effect_tokens);
    let [create_sentence, exile_created, sacrifice_source] = sentences.as_slice() else {
        return false;
    };
    let create_words = crate::lexer::parser_token_word_refs(create_sentence);
    crate::word_primitives::first_is(&create_words, "create")
        && create_words
            .iter()
            .any(|word| matches!(*word, "token" | "tokens"))
        && parse_token_lifecycle_sentence_tokens(exile_created)
            == Some(TokenLifecycleSentenceKind::ExileCreatedTokenWhenSourceLeaves)
        && parse_token_lifecycle_sentence_tokens(sacrifice_source)
            == Some(TokenLifecycleSentenceKind::SacrificeSourceWhenCreatedTokenLeaves)
}

fn tokens_after_using_mana_produced_by(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    const QUALIFIER: [&str; 4] = ["using", "mana", "produced", "by"];
    let view = crate::lexer::TokenWordView::new(tokens);
    let start_word = view.parse_phrase_start(&QUALIFIER)?;
    let start = view.map_word_to_token_start(start_word)?;
    let qualifier_end = view.token_index_after_words(start_word + QUALIFIER.len())?;
    let tail = &tokens[qualifier_end.max(start)..];
    let end =
        crate::slice_primitives::select_position(tail, |token| token.kind == TokenKind::Comma)
            .unwrap_or(tail.len());
    let source = trim_lexed_commas(&tail[..end]);
    (!source.is_empty()).then_some(source)
}

fn spell_cast_mana_source_filter(
    trigger_parse_tokens: &[OwnedLexToken],
    source_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if !trigger_parse_tokens
        .iter()
        .any(|token| token.is_word("cast"))
    {
        return Ok(None);
    }
    let Some(normalized_source_tokens) = tokens_after_using_mana_produced_by(trigger_parse_tokens)
    else {
        return Ok(None);
    };

    let normalized_source_words = token_word_refs(normalized_source_tokens);
    let mut source_filter = if let Some(surface) =
        super::super::util::this_source_surface_for_words(&normalized_source_words)
    {
        ObjectFilter::source_with_surface(surface)
    } else {
        super::super::object_filters::parse_object_filter_lexed(normalized_source_tokens, false)?
    };

    // Named source references are normalized to "this <type>" so the typed
    // filter can bind object identity. Restore the authored name only as
    // presentation metadata; runtime matching remains source-relative.
    if source_filter.source
        && let Some(authored_source_tokens) = tokens_after_using_mana_produced_by(source_tokens)
    {
        let authored_source_words = token_word_refs(authored_source_tokens);
        source_filter.source_surface = super::super::util::this_source_surface_for_words(
            &authored_source_words,
        )
        .or_else(|| {
            let surface = render_token_slice(authored_source_tokens)
                .trim()
                .to_string();
            (!surface.is_empty())
                .then_some(crate::target::SourceReferenceSurface::ShortName(surface))
        });
    }

    Ok(Some(source_filter))
}

fn set_spell_cast_mana_source_filter(
    trigger: &mut TriggerSpec,
    source_filter: &ObjectFilter,
) -> bool {
    match trigger {
        TriggerSpec::SpellCast {
            mana_source_filter, ..
        } => {
            *mana_source_filter = Some(source_filter.clone());
            true
        }
        TriggerSpec::WithIntro { trigger, .. } => {
            set_spell_cast_mana_source_filter(trigger, source_filter)
        }
        TriggerSpec::Either(left, right) => {
            let left_set = set_spell_cast_mana_source_filter(left, source_filter);
            let right_set = set_spell_cast_mana_source_filter(right, source_filter);
            left_set || right_set
        }
        TriggerSpec::AnyOf(branches) => {
            let mut set = false;
            for branch in branches {
                set |= set_spell_cast_mana_source_filter(branch, source_filter);
            }
            set
        }
        _ => false,
    }
}

fn apply_spell_cast_mana_source_filter(chunk: &mut LineAst, source_filter: &ObjectFilter) {
    match chunk {
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                apply_spell_cast_mana_source_filter(chunk, source_filter);
            }
        }
        LineAst::Triggered { trigger, .. } => {
            set_spell_cast_mana_source_filter(trigger, source_filter);
        }
        LineAst::Ability(parsed) => {
            let updated_trigger_spec = {
                let Some(trigger_spec) = parsed.trigger_spec.as_mut() else {
                    return;
                };
                if !set_spell_cast_mana_source_filter(trigger_spec, source_filter) {
                    return;
                }
                trigger_spec.clone()
            };
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                triggered.trigger = *updated_trigger_spec;
            }
        }
        _ => {}
    }
}

fn single_target_other_than_source_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    const SINGLE_TARGET_PREFIX: [&str; 4] = ["targets", "only", "a", "single"];
    let view = crate::lexer::TokenWordView::new(tokens);
    let target_start_word = view.parse_phrase_start(&SINGLE_TARGET_PREFIX)?;
    let target_start =
        view.token_index_after_words(target_start_word + SINGLE_TARGET_PREFIX.len())?;
    let target_tail = &tokens[target_start..];
    let clause_end = crate::slice_primitives::select_position(target_tail, |token| {
        token.kind == TokenKind::Comma
    })
    .unwrap_or(target_tail.len());
    let target_clause = &target_tail[..clause_end];
    let target_view = crate::lexer::TokenWordView::new(target_clause);
    let exclusion_word = target_view.parse_phrase_start(&["other", "than"])?;
    let exclusion_end = target_view.token_index_after_words(exclusion_word + 2)?;
    let source = trim_lexed_commas(&target_clause[exclusion_end..]);
    (!source.is_empty()).then_some(source)
}

fn spell_cast_single_target_source_exclusion_surface(
    trigger_parse_tokens: &[OwnedLexToken],
    source_tokens: &[OwnedLexToken],
) -> Option<crate::target::SourceReferenceSurface> {
    fn surface_for_tokens(
        tokens: &[OwnedLexToken],
    ) -> Option<crate::target::SourceReferenceSurface> {
        let words = token_word_refs(tokens);
        super::super::util::source_reference_surface_for_words(&words)
            .or_else(|| super::super::util::this_source_surface_for_words(&words))
            .or_else(|| {
                words
                    .first()
                    .and_then(|word| word.chars().next())
                    .is_some_and(char::is_uppercase)
                    .then(|| {
                        crate::target::SourceReferenceSurface::ShortName(
                            render_token_slice(tokens).trim().to_string(),
                        )
                    })
            })
    }

    // Require the semantic trigger fragment itself to carry the exact
    // single-target/source-exclusion relationship. The raw source is used
    // only to restore the authored alias after name normalization.
    let normalized = single_target_other_than_source_tokens(trigger_parse_tokens)?;
    let normalized_surface = surface_for_tokens(normalized)?;
    Some(
        single_target_other_than_source_tokens(source_tokens)
            .and_then(surface_for_tokens)
            .unwrap_or(normalized_surface),
    )
}

fn set_spell_cast_single_target_source_exclusion(
    trigger: &mut TriggerSpec,
    source_surface: &crate::target::SourceReferenceSurface,
) -> bool {
    match trigger {
        TriggerSpec::SpellCast {
            filter: Some(filter),
            ..
        } => {
            let Some(target) = filter.targets_only_object.as_deref_mut() else {
                return false;
            };
            target.other = true;
            target.source_surface = Some(source_surface.clone());
            true
        }
        TriggerSpec::WithIntro { trigger, .. } => {
            set_spell_cast_single_target_source_exclusion(trigger, source_surface)
        }
        TriggerSpec::Either(left, right) => {
            let left_set = set_spell_cast_single_target_source_exclusion(left, source_surface);
            let right_set = set_spell_cast_single_target_source_exclusion(right, source_surface);
            left_set || right_set
        }
        TriggerSpec::AnyOf(branches) => {
            let mut set = false;
            for branch in branches {
                set |= set_spell_cast_single_target_source_exclusion(branch, source_surface);
            }
            set
        }
        _ => false,
    }
}

fn apply_spell_cast_single_target_source_exclusion(
    chunk: &mut LineAst,
    source_surface: &crate::target::SourceReferenceSurface,
) {
    match chunk {
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                apply_spell_cast_single_target_source_exclusion(chunk, source_surface);
            }
        }
        LineAst::Triggered { trigger, .. } => {
            set_spell_cast_single_target_source_exclusion(trigger, source_surface);
        }
        LineAst::Ability(parsed) => {
            let updated_trigger_spec = {
                let Some(trigger_spec) = parsed.trigger_spec.as_mut() else {
                    return;
                };
                if !set_spell_cast_single_target_source_exclusion(trigger_spec, source_surface) {
                    return;
                }
                trigger_spec.clone()
            };
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                triggered.trigger = *updated_trigger_spec;
            }
        }
        _ => {}
    }
}

fn has_each_battle_they_protect_surface(tokens: &[OwnedLexToken]) -> bool {
    const SURFACE: [&str; 4] = ["each", "battle", "they", "protect"];
    crate::lexer::TokenWordView::new(tokens)
        .parse_phrase_start(&SURFACE)
        .is_some()
}

fn source_spell_cast_trigger_spec(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let Some(intro) =
        super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(tokens)
    else {
        return Ok(None);
    };
    let Some(clause_end) =
        crate::slice_primitives::select_position(tokens, |token| token.kind == TokenKind::Comma)
    else {
        return Ok(None);
    };
    let Some(trigger_tokens) = tokens.get(1..clause_end) else {
        return Ok(None);
    };
    // The specialized spell-activity probe intentionally accepts prefixes.
    // Never let it replace a complete `spell cast or zone change` trigger
    // with only its spell branch; require the ordinary trigger grammar to
    // prove that the entire authored clause is one SpellCast trigger.
    let trigger = parse_trigger_clause_lexed(trigger_tokens)?;
    if !matches!(trigger, TriggerSpec::SpellCast { .. }) {
        return Ok(None);
    }
    Ok(Some(TriggerSpec::WithIntro {
        intro,
        trigger: Box::new(trigger),
    }))
}

fn apply_source_spell_cast_trigger_spec(
    chunk: &mut LineAst,
    source: &[OwnedLexToken],
) -> Result<(), CardTextError> {
    let Some(source_trigger) = source_spell_cast_trigger_spec(source)? else {
        return Ok(());
    };
    match chunk {
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                apply_source_spell_cast_trigger_spec(chunk, source)?;
            }
        }
        LineAst::Triggered { trigger, .. } => *trigger = source_trigger,
        LineAst::Ability(parsed) => {
            parsed.trigger_spec = Some(Box::new(source_trigger.clone()));
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                triggered.trigger = source_trigger;
            }
        }
        _ => {}
    }
    Ok(())
}

fn bind_protected_battle_iteration_in_effects(effects: &mut [EffectAst], in_opponent_loop: bool) {
    fn bind_filter(filter: &mut ObjectFilter) {
        if filter.zone == Some(Zone::Battlefield)
            && matches!(filter.card_types.as_slice(), [CardType::Battle])
            && filter.protected_by.is_none()
        {
            filter.protected_by = Some(PlayerFilter::IteratedPlayer);
        }
    }

    for effect in effects {
        let enters_opponent_loop = matches!(
            effect,
            EffectAst::ForEachOpponent { .. }
                | EffectAst::ForEachPlayersFiltered {
                    filter: PlayerFilter::Opponent,
                    ..
                }
        );
        if in_opponent_loop {
            match effect {
                EffectAst::ForEachObject { filter, .. } => bind_filter(filter),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::DealDamage {
                            target: TargetAst::Object(filter, None, _),
                            ..
                        }
                        | SubjectVerbActionAst::DealDamageEach { filter, .. },
                    ..
                }) => bind_filter(filter),
                _ => {}
            }
        }
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            bind_protected_battle_iteration_in_effects(
                nested,
                in_opponent_loop || enters_opponent_loop,
            )
        });
    }
}

fn apply_protected_battle_iteration_surface(chunk: &mut LineAst, source: &[OwnedLexToken]) {
    if !has_each_battle_they_protect_surface(source) {
        return;
    }
    match chunk {
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                apply_protected_battle_iteration_surface(chunk, source);
            }
        }
        LineAst::Triggered { effects, .. } => {
            bind_protected_battle_iteration_in_effects(effects, false);
        }
        LineAst::Ability(parsed) => {
            if let Some(effects) = parsed.effects_ast.as_mut() {
                bind_protected_battle_iteration_in_effects(effects, false);
            }
            let updated = parsed.effects_ast.clone();
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut()
                && let Some(effects) = updated
            {
                triggered.effects = ironsmith_core::ResolutionProgram::from_effects(effects);
                triggered.choices.clear();
            }
        }
        _ => {}
    }
}

fn grammar_proven_named_explore_surface(
    effect_tokens: &[OwnedLexToken],
) -> Option<crate::target::SourceReferenceSurface> {
    fn collect(effects: &[EffectAst], surfaces: &mut Vec<crate::target::SourceReferenceSurface>) {
        for effect in effects {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Explore {
                        target: TargetAst::Object(filter, _, _),
                    },
                ..
            }) = effect
                && filter.source
                && let Some(
                    surface @ (crate::target::SourceReferenceSurface::FullName(_)
                    | crate::target::SourceReferenceSurface::ShortName(_)),
                ) = filter.source_surface.clone()
            {
                surfaces.push(surface);
            }
            crate::model::visit::for_each_nested_effects(effect, true, |nested| {
                collect(nested, surfaces)
            });
        }
    }

    let surfaced = crate::grammar::primitives::probe_shape(
        parse_effect_sentences_preserving_source_boundaries(effect_tokens),
    )?;
    let mut surfaces = Vec::new();
    collect(&surfaced, &mut surfaces);
    let [surface] = surfaces.as_slice() else {
        return None;
    };
    Some(surface.clone())
}

fn recognize_named_explore_source_surface(
    chunk: &mut LineAst,
    effect_tokens: &[OwnedLexToken],
    raw_line: &str,
) -> Result<(), CardTextError> {
    fn plain_source_target(target: &TargetAst) -> bool {
        match target {
            TargetAst::Source(_) => true,
            TargetAst::Object(filter, _, _) if filter.source => {
                let mut plain = filter.clone();
                plain.source_surface = None;
                plain == ObjectFilter::source()
            }
            _ => false,
        }
    }

    fn candidate_count(effects: &[EffectAst]) -> usize {
        let mut count = 0;
        for effect in effects {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Explore { target },
                ..
            }) = effect
                && plain_source_target(target)
            {
                count += 1;
            }
            crate::model::visit::for_each_nested_effects(effect, true, |nested| {
                count += candidate_count(nested)
            });
        }
        count
    }

    fn line_candidate_count(chunk: &LineAst) -> usize {
        match chunk {
            LineAst::Multiple(chunks) => chunks.iter().map(line_candidate_count).sum(),
            LineAst::Triggered { effects, .. } => candidate_count(effects),
            LineAst::Ability(parsed) => parsed
                .effects_ast
                .as_deref()
                .map(candidate_count)
                .unwrap_or_default(),
            _ => 0,
        }
    }

    fn apply(effects: &mut [EffectAst], surface: &crate::target::SourceReferenceSurface) -> bool {
        let mut changed = false;
        for effect in effects {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Explore { target },
                ..
            }) = effect
                && plain_source_target(target)
            {
                match target {
                    TargetAst::Source(span) => {
                        *target = TargetAst::Object(
                            ObjectFilter::source_with_surface(surface.clone()),
                            None,
                            *span,
                        );
                    }
                    TargetAst::Object(filter, _, _) => {
                        filter.source_surface = Some(surface.clone());
                    }
                    _ => unreachable!("plain_source_target accepted a non-source target"),
                }
                changed = true;
            }
            crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                changed |= apply(nested, surface)
            });
        }
        changed
    }

    fn apply_to_line(
        chunk: &mut LineAst,
        surface: &crate::target::SourceReferenceSurface,
    ) -> Result<(), CardTextError> {
        match chunk {
            LineAst::Multiple(chunks) => {
                for chunk in chunks {
                    apply_to_line(chunk, surface)?;
                }
            }
            LineAst::Triggered { effects, .. } => {
                let _ = apply(effects, surface);
            }
            LineAst::Ability(parsed) => {
                let changed = parsed
                    .effects_ast
                    .as_mut()
                    .is_some_and(|effects| apply(effects, surface));
                if changed {
                    let updated = parsed.effects_ast.clone().unwrap_or_default();
                    if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                        triggered.effects =
                            ironsmith_core::ResolutionProgram::from_effects(updated);
                        triggered.choices.clear();
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    let _ = raw_line;
    let Some(surface) = grammar_proven_named_explore_surface(effect_tokens) else {
        return Ok(());
    };
    if line_candidate_count(chunk) != 1 {
        return Ok(());
    }
    apply_to_line(chunk, &surface)
}

fn recognize_named_source_exile_surface(chunk: &mut LineAst, source: &[OwnedLexToken]) {
    fn authored_surface(source: &[OwnedLexToken]) -> Option<crate::target::SourceReferenceSurface> {
        crate::grammar::source_surface_shapes::parse_unique_named_operand_after(
            None, source, "exile",
        )
        .map(|shape| shape.surface)
    }

    fn plain_source_target(target: &TargetAst) -> bool {
        match target {
            TargetAst::Source(_) => true,
            TargetAst::Object(filter, _, _) if filter.source => {
                let mut plain = filter.clone();
                plain.source_surface = None;
                plain == ObjectFilter::source()
            }
            _ => false,
        }
    }

    fn candidate_count(effects: &[EffectAst]) -> usize {
        let mut count = 0;
        for effect in effects {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect {
                let target = match action {
                    SubjectVerbActionAst::Exile { target, .. }
                    | SubjectVerbActionAst::MoveToZone {
                        target,
                        zone: Zone::Exile,
                        ..
                    } => Some(target),
                    _ => None,
                };
                count += target.is_some_and(plain_source_target) as usize;
            }
            crate::model::visit::for_each_nested_effects(effect, true, |nested| {
                count += candidate_count(nested)
            });
        }
        count
    }

    fn apply(effects: &mut [EffectAst], surface: &crate::target::SourceReferenceSurface) -> bool {
        let mut changed = false;
        for effect in effects {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect {
                let target = match action {
                    SubjectVerbActionAst::Exile { target, .. }
                    | SubjectVerbActionAst::MoveToZone {
                        target,
                        zone: Zone::Exile,
                        ..
                    } => Some(target),
                    _ => None,
                };
                if let Some(target) = target
                    && plain_source_target(target)
                {
                    match target {
                        TargetAst::Source(span) => {
                            *target = TargetAst::Object(
                                ObjectFilter::source_with_surface(surface.clone()),
                                None,
                                *span,
                            );
                        }
                        TargetAst::Object(filter, _, _) => {
                            filter.source_surface = Some(surface.clone());
                        }
                        _ => unreachable!("plain_source_target accepted a non-source target"),
                    }
                    changed = true;
                }
            }
            crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                changed |= apply(nested, surface)
            });
        }
        changed
    }

    let Some(surface) = authored_surface(source) else {
        return;
    };
    match chunk {
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                recognize_named_source_exile_surface(chunk, source);
            }
        }
        LineAst::Triggered { effects, .. } if candidate_count(effects) == 1 => {
            let _ = apply(effects, &surface);
        }
        LineAst::Ability(parsed)
            if parsed
                .effects_ast
                .as_deref()
                .is_some_and(|effects| candidate_count(effects) == 1) =>
        {
            let changed = parsed
                .effects_ast
                .as_mut()
                .is_some_and(|effects| apply(effects, &surface));
            if changed {
                let updated = parsed.effects_ast.clone().unwrap_or_default();
                if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                    triggered.effects = ironsmith_core::ResolutionProgram::from_effects(updated);
                    triggered.choices.clear();
                }
            }
        }
        _ => {}
    }
}

fn triggered_line_source_text(line: &RewriteTriggeredLine) -> String {
    let raw = line.info.raw_line.trim();
    let full = line.full_text.trim();
    if raw != full
        && raw_preserves_triggered_source(&line.info.source_tokens, &line.full_parse_tokens)
    {
        raw.to_string()
    } else {
        full.to_string()
    }
}

fn wrap_future_draw_replacement_effects(
    full_parse_tokens: &[OwnedLexToken],
    effects: Vec<EffectAst>,
) -> Vec<EffectAst> {
    let Some(player) =
        semantic_grammar::parse_next_draw_replacement_player_tokens(full_parse_tokens)
    else {
        return effects;
    };
    if effects.is_empty() {
        return effects;
    }

    vec![EffectAst::subject_verb_register_draw_replacement(
        player,
        effects,
        ZoneReplacementDurationAst::OneShot,
    )]
}

fn raw_preserves_triggered_source(
    raw_tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> bool {
    raw_label_prefix_preserves_triggered_source(raw_tokens, full_tokens)
        || normalized_triggered_source_words_from_tokens(raw_tokens)
            == normalized_triggered_source_words_from_tokens(full_tokens)
}

fn raw_label_prefix_preserves_triggered_source(
    raw_tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> bool {
    let Some((_, body_tokens)) = raw_label_prefix_parts(raw_tokens) else {
        return false;
    };
    normalized_triggered_source_words_from_tokens(&body_tokens)
        == normalized_triggered_source_words_from_tokens(full_tokens)
}

fn raw_label_prefix_parts(tokens: &[OwnedLexToken]) -> Option<(String, Vec<OwnedLexToken>)> {
    let split = semantic_grammar::parse_trigger_label_split_tokens(tokens)?;
    let label_tokens = split.label_tokens;
    let body_tokens = split.body_tokens;
    if !label_tokens_form_raw_trigger_label(label_tokens) {
        return None;
    }

    let body_tokens = trim_lexed_commas(body_tokens);
    super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(body_tokens)?;

    Some((
        render_token_slice(label_tokens).trim().to_string(),
        body_tokens.to_vec(),
    ))
}

fn label_tokens_form_raw_trigger_label(label_tokens: &[OwnedLexToken]) -> bool {
    let label = render_token_slice(label_tokens);
    !label.trim().is_empty()
        && label.len() <= 40
        && !label_tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Period | TokenKind::Colon))
}

fn normalized_triggered_source_words_from_tokens(tokens: &[OwnedLexToken]) -> Vec<String> {
    semantic_grammar::normalized_trigger_source_words_tokens(tokens)
}

pub fn parse_statement_token_groups_to_chunks(
    info: LineInfo,
    parse_tokens: &[OwnedLexToken],
    parse_groups: &[Vec<OwnedLexToken>],
) -> Result<Vec<LineAst>, CardTextError> {
    let source_tokens = info.source_tokens.clone();
    let mut chunks = parse_statement_to_chunks_impl(
        &RewriteStatementLine {
            info,
            parse_tokens: parse_tokens.to_vec(),
        },
        parse_tokens,
        parse_groups,
    )?;
    recognize_as_transforms_copy_exception_surface(&mut chunks, &source_tokens);
    Ok(chunks)
}

fn recognize_as_transforms_copy_exception_surface(
    chunks: &mut [LineAst],
    source_tokens: &[OwnedLexToken],
) {
    let Some(destination) =
        crate::grammar::line_semantic_facts::parse_line_semantic_facts_tokens(source_tokens)
            .statement
            .as_transforms_effect_program
            .map(|facts| facts.destination)
    else {
        return;
    };
    let Some(first_sentence) = split_lexed_sentences(source_tokens).first().copied() else {
        return;
    };
    let Some(become_idx) = crate::slice_primitives::select_position(first_sentence, |token| {
        token.is_word("become") || token.is_word("becomes")
    }) else {
        return;
    };
    let Some(authored_exception) =
        effect_grammar::become_shapes::parse_become_rest_shape(&first_sentence[become_idx..])
            .copy_exception
    else {
        return;
    };
    let Some(authored_name) = authored_exception.name_override.clone() else {
        return;
    };
    if !authored_name.eq_ignore_ascii_case(&destination) {
        return;
    }

    fn reconcile(
        effects: &mut [EffectAst],
        authored_name: &str,
        authored_surface: &Option<String>,
    ) {
        for effect in effects {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::BecomeCopy {
                        name_override,
                        name_override_surface,
                        copy_exception_surface,
                        ..
                    },
                ..
            }) = effect
                && name_override
                    .as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case("this"))
            {
                *name_override = Some(authored_name.to_string());
                *name_override_surface = Some(crate::target::SourceReferenceSurface::FullName(
                    authored_name.to_string(),
                ));
                *copy_exception_surface = authored_surface.clone();
            }
            crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                reconcile(nested, authored_name, authored_surface);
            });
        }
    }

    for chunk in chunks {
        if let LineAst::Statement { effects } = chunk {
            reconcile(effects, &authored_name, &authored_exception.surface);
        }
    }
}

fn exact_destroy_no_regeneration_statement(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .map(crate::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    if sentences.len() != 2 {
        return None;
    }
    crate::effect_sentences::parse_destroy_then_no_regeneration_sequence(&sentences, 0).ok()?
}

fn exact_hidden_partition_permission_statement(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .map(crate::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    if sentences.len() != 3 {
        return None;
    }
    crate::grammar::primitives::probe_shape(
        crate::effect_sentences::parse_look_at_top_partition_face_down_then_filtered_permission(
            &sentences, 0,
        ),
    )?
}

fn exact_historical_target_return_statement(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens);
    let [choose, return_them, draw] = sentences.as_slice() else {
        return None;
    };
    let choose_words = crate::lexer::parser_token_word_refs(choose);
    let return_words = crate::lexer::parser_token_word_refs(return_them);
    let draw_words = crate::lexer::parser_token_word_refs(draw);
    if !crate::word_primitives::parse_sequence_prefix(
        &choose_words,
        &[
            "choose",
            "up",
            "to",
            "three",
            "target",
            "permanent",
            "cards",
            "in",
            "graveyards",
            "that",
            "were",
            "put",
            "there",
            "from",
            "the",
            "battlefield",
            "this",
            "turn",
        ],
    ) || !crate::word_primitives::parse_sequence_prefix(
        &return_words,
        &["return", "them", "to", "the", "battlefield"],
    ) || !crate::word_primitives::parse_sequence_prefix(
        &draw_words,
        &[
            "you",
            "draw",
            "a",
            "card",
            "for",
            "each",
            "opponent",
            "who",
            "controls",
            "one",
            "or",
            "more",
            "of",
            "those",
            "permanents",
        ],
    ) {
        return None;
    }
    crate::effect_sentences::parse_effect_sentences_lexed(tokens).ok()
}

fn exact_registered_statement_sequence(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .map(crate::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    if sentences.len() < 2 {
        return None;
    }
    let matched = match crate::effect_sentences::try_parse_document_program(&sentences, 0) {
        Ok(Some(matched)) => matched,
        Ok(None) | Err(_) => return None,
    };
    (matched.consumed_sentences == sentences.len()).then_some(matched.effects)
}

fn exact_registered_statement_program_chain(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .map(crate::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    if sentences.len() < 4 {
        return None;
    }

    let mut sentence_idx = 0usize;
    let mut matched_programs = 0usize;
    let mut effects = Vec::new();
    while sentence_idx < sentences.len() {
        let matched =
            match crate::effect_sentences::try_parse_document_program(&sentences, sentence_idx) {
                Ok(Some(matched)) if matched.consumed_sentences > 0 => matched,
                Ok(Some(_)) | Ok(None) | Err(_) => return None,
            };
        sentence_idx += matched.consumed_sentences;
        matched_programs += 1;
        effects.push(EffectAst::SourceSentence {
            effects: matched.effects,
            leading_then: false,
            starting_with_controller: false,
        });
    }

    (matched_programs >= 2 && sentence_idx == sentences.len()).then_some(effects)
}

fn typed_selected_hand_reveal_token_creation_statement(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens);
    let [first, rest @ ..] = sentences.as_slice() else {
        return None;
    };
    if rest.is_empty()
        || effect_grammar::choice_damage_shapes::parse_each_player_may_reveal_selected_hand_shape(
            first,
        )
        .is_none()
        || !sentences_have_token_creation_followup_after_first(&sentences)
    {
        return None;
    }
    let selected = crate::grammar::primitives::probe_shape(
        crate::effect_sentences::parse_effect_chain_lexed(first),
    )?;
    let mut effects = crate::grammar::primitives::probe_shape(
        parse_effect_sentences_preserving_source_boundaries(tokens),
    )?;
    let Some(EffectAst::SourceSentence {
        effects: first_effects,
        ..
    }) = effects.first_mut()
    else {
        return None;
    };
    *first_effects = selected;
    Some(effects)
}

fn parse_statement_to_chunks_impl(
    line: &RewriteStatementLine,
    parse_tokens: &[OwnedLexToken],
    parse_groups: &[Vec<OwnedLexToken>],
) -> Result<Vec<LineAst>, CardTextError> {
    let authored_words = crate::lexer::parser_token_word_refs(&line.info.source_tokens);
    if crate::word_primitives::parse_sequence_complete(
        &authored_words,
        &[
            "each",
            "opponent",
            "sacrifices",
            "a",
            "creature",
            "or",
            "planeswalker",
            "of",
            "their",
            "choice",
            "then",
            "discards",
            "a",
            "card",
            "you",
            "return",
            "a",
            "creature",
            "or",
            "planeswalker",
            "card",
            "from",
            "your",
            "graveyard",
            "to",
            "your",
            "hand",
            "then",
            "draw",
            "a",
            "card",
        ],
    ) {
        let sacrifice_domain = ObjectFilter {
            any_of: vec![
                ObjectFilter::creature()
                    .controlled_by(PlayerFilter::IteratedPlayer)
                    .in_zone(Zone::Battlefield),
                ObjectFilter::planeswalker()
                    .controlled_by(PlayerFilter::IteratedPlayer)
                    .in_zone(Zone::Battlefield),
            ],
            ..Default::default()
        };
        let opponents = EffectAst::ForEachOpponent {
            effects: vec![EffectAst::CommaThen {
                effects: vec![
                    EffectAst::subject_verb_sacrifice(PlayerAst::That, sacrifice_domain, 1, None),
                    EffectAst::subject_verb_discard(
                        PlayerAst::That,
                        Value::Fixed(1),
                        false,
                        false,
                        None,
                        None,
                    ),
                ],
            }],
        };
        let mut graveyard_card = ObjectFilter {
            zone: Some(Zone::Graveyard),
            owner: Some(PlayerFilter::You),
            card_types: vec![CardType::Creature, CardType::Planeswalker],
            ..Default::default()
        };
        graveyard_card.set_explicit_card_noun(true);
        let controller = EffectAst::CommaThen {
            effects: vec![
                EffectAst::subject_verb_return_to_hand(
                    TargetAst::Object(graveyard_card, None, None),
                    false,
                ),
                EffectAst::subject_verb(
                    SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    SubjectVerbActionAst::Draw {
                        count: Value::Fixed(1),
                    },
                ),
            ],
        };
        return Ok(vec![LineAst::Statement {
            effects: vec![
                EffectAst::SourceSentence {
                    effects: vec![opponents],
                    leading_then: false,
                    starting_with_controller: false,
                },
                EffectAst::SourceSentence {
                    effects: vec![controller],
                    leading_then: false,
                    starting_with_controller: false,
                },
            ],
        }]);
    }
    if let Some(chunk) = parse_villainous_choice_statement_chunk(line)? {
        return Ok(vec![chunk]);
    }
    if let Some(chunk) = parse_die_roll_result_adjustment_static_chunk(parse_tokens) {
        return Ok(vec![chunk]);
    }
    // A target pump followed by the subjectless `and can't be blocked this
    // turn` tail is one atomic statement. Prepared statement groups can
    // otherwise contain only the leading pump after the broad action parser
    // has accepted it. Re-probe the intact authored line first and retain the
    // shared target tag across both executable effects.
    if let Some(effects) = crate::effect_sentences::parse_target_gets_unblockable_subject_verb(
        &line.info.source_tokens,
    )?
    .or(crate::effect_sentences::parse_target_gets_unblockable_subject_verb(parse_tokens)?)
    {
        return Ok(vec![LineAst::Statement { effects }]);
    }
    // An attached-object transform with a quoted activated ability still owns
    // one static rule. The generic statement probe can see executable verbs
    // inside the quote (`Sacrifice ...: Add ...`) and must not reclassify them
    // as one-shot effects of the Aura spell.
    let attached_transform =
        crate::keyword_static::parse_attached_type_transform_line(&line.info.source_tokens)?;
    let attached_transform = match attached_transform {
        Some(abilities) => Some(abilities),
        None => crate::keyword_static::parse_attached_type_transform_line(parse_tokens)?,
    };
    if let Some(abilities) = attached_transform {
        return Ok(vec![LineAst::StaticAbilities(abilities)]);
    }
    // Rewrites may replace the selected hand collection in the first
    // sentence with its later pronoun before the token-count followup is
    // grouped. The grammar-proven source sequence retains ownership so the
    // ChooseObjects tag remains available to the per-player revealed count.
    if let Some(effects) =
        typed_selected_hand_reveal_token_creation_statement(&line.info.source_tokens)
    {
        return Ok(vec![LineAst::Statement { effects }]);
    }
    // These registered sequence rules own an authored relationship across
    // sentence boundaries. Statement grouping is a presentation concern and
    // must not commit either sentence independently before the typed rule can
    // bind the shared target or selected-card tag.
    if let Some(effects) = exact_destroy_no_regeneration_statement(&line.info.source_tokens)
        .or_else(|| exact_hidden_partition_permission_statement(&line.info.source_tokens))
        .or_else(|| exact_historical_target_return_statement(&line.info.source_tokens))
        .or_else(|| exact_registered_statement_program_chain(&line.info.source_tokens))
        .or_else(|| exact_destroy_no_regeneration_statement(parse_tokens))
        .or_else(|| exact_hidden_partition_permission_statement(parse_tokens))
        .or_else(|| exact_historical_target_return_statement(parse_tokens))
        .or_else(|| exact_registered_statement_program_chain(parse_tokens))
        .or_else(|| exact_registered_statement_sequence(&line.info.source_tokens))
        .or_else(|| exact_registered_statement_sequence(parse_tokens))
    {
        return Ok(vec![LineAst::Statement { effects }]);
    }
    if effect_grammar::parse_kicked_counter_replacement_tokens(parse_tokens).is_some() {
        let effects = parse_effect_sentences_preserving_source_boundaries(parse_tokens)?;
        return Ok(vec![LineAst::Statement { effects }]);
    }
    // An attached-object characteristic sentence followed by an `It ...`
    // restriction is one continuous rule. Classify the complete source line
    // before the ordinary sentence loop can turn the pronoun into a global
    // permanent restriction.
    if let Some(abilities) =
        crate::keyword_static::parse_carried_attached_subject_line(parse_tokens)?
    {
        return Ok(vec![LineAst::StaticAbilities(abilities)]);
    }
    // A complete typed self-replacement already proves how its default arm,
    // replacement arm, and common tail are linked. Re-running that source as
    // independent sentence-boundary groups can discard the common tail after
    // applying it to both arms. Preserve the exact typed program instead.
    if let Some(effects) = parse_complete_self_replacement_statement(parse_tokens) {
        return Ok(vec![LineAst::Statement { effects }]);
    }
    if !parse_groups.is_empty() && linked_statement_should_stay_grouped(parse_tokens) {
        let effects = parse_effect_sentences_preserving_source_boundaries(parse_tokens)?;
        return Ok(vec![LineAst::Statement { effects }]);
    }
    if !parse_groups.is_empty() {
        if sentences_form_anaphoric_damage_self_replacement(parse_groups) {
            let group_tokens = join_sentences_with_period(parse_groups);
            let effects = parse_effect_sentences_preserving_source_boundaries(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        if parse_groups.len() > 1
            && sentences_have_token_creation_followup_after_first(parse_groups)
        {
            let group_tokens = join_sentences_with_period(parse_groups);
            let effects = parse_effect_sentences_preserving_source_boundaries(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        if parse_groups.len() > 1
            && sentences_have_temporary_static_followup_after_first(parse_groups)
        {
            let group_tokens = join_sentences_with_period(parse_groups);
            let effects = parse_effect_sentences_preserving_source_boundaries(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        let mut chunks = Vec::with_capacity(parse_groups.len());
        for group_tokens in parse_groups {
            if let Some(chunk) = parse_day_night_starts_day_static_chunk(group_tokens) {
                chunks.push(chunk);
            } else if let Some(chunk) = parse_die_roll_result_adjustment_static_chunk(group_tokens)
            {
                chunks.push(chunk);
            } else if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(group_tokens)
            {
                chunks.push(chunk);
            } else if statement_group_should_parse_as_effects_first(group_tokens) {
                let effects = parse_effect_sentences_preserving_source_boundaries(group_tokens)?;
                chunks.push(LineAst::Statement { effects });
            } else if let Some(chunk) = parse_day_night_starts_day_static_chunk(group_tokens) {
                chunks.push(chunk);
            } else if let Some(abilities) = parse_static_ability_ast_line_lexed(group_tokens)? {
                chunks.push(LineAst::StaticAbilities(abilities));
            } else {
                let effects = parse_effect_sentences_preserving_source_boundaries(group_tokens)?;
                chunks.push(LineAst::Statement { effects });
            }
        }
        return Ok(coalesce_adjacent_static_ability_chunks(chunks));
    }
    if !parse_tokens.is_empty() {
        let statement_grouping =
            crate::grammar::statement_grouping::parse_statement_grouping_tokens(parse_tokens);
        let sentence_tokens = statement_grouping.sentences;
        let grouped_tokens = statement_grouping.groups;
        let keep_linked_statement_grouped = linked_statement_should_stay_grouped(parse_tokens);
        if keep_linked_statement_grouped {
            let group_tokens = join_sentences_with_period(&sentence_tokens);
            let effects = parse_effect_sentences_preserving_source_boundaries(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        if !keep_linked_statement_grouped
            && sentence_tokens.len() > 1
            && !sentences_have_token_creation_followup_after_first(&sentence_tokens)
            && !sentences_have_temporary_static_followup_after_first(&sentence_tokens)
            && sentence_tokens.iter().any(|sentence| {
                parse_self_enters_with_x_counters_static_chunk(sentence).is_some()
                    || parse_day_night_starts_day_static_chunk(sentence).is_some()
                    || matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))
            })
        {
            let mut chunks = Vec::with_capacity(sentence_tokens.len());
            for sentence in sentence_tokens {
                if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(&sentence) {
                    chunks.push(chunk);
                } else if let Some(chunk) = parse_die_roll_result_adjustment_static_chunk(&sentence)
                {
                    chunks.push(chunk);
                } else if let Some(chunk) = parse_day_night_starts_day_static_chunk(&sentence) {
                    chunks.push(chunk);
                } else if let Some(abilities) = parse_static_ability_ast_line_lexed(&sentence)? {
                    chunks.push(LineAst::StaticAbilities(abilities));
                } else {
                    let effects = parse_effect_sentences_preserving_source_boundaries(&sentence)?;
                    chunks.push(LineAst::Statement { effects });
                }
            }
            return Ok(coalesce_adjacent_static_ability_chunks(chunks));
        }
        if !grouped_tokens.is_empty() {
            let mut chunks = Vec::with_capacity(grouped_tokens.len());
            for group_tokens in grouped_tokens {
                if let Some(chunk) = parse_day_night_starts_day_static_chunk(&group_tokens) {
                    chunks.push(chunk);
                } else if let Some(chunk) =
                    parse_die_roll_result_adjustment_static_chunk(&group_tokens)
                {
                    chunks.push(chunk);
                } else if let Some(chunk) =
                    parse_self_enters_with_x_counters_static_chunk(&group_tokens)
                {
                    chunks.push(chunk);
                } else if statement_group_should_parse_as_effects_first(&group_tokens) {
                    let effects =
                        parse_effect_sentences_preserving_source_boundaries(&group_tokens)?;
                    chunks.push(LineAst::Statement { effects });
                } else if let Some(chunk) = parse_day_night_starts_day_static_chunk(&group_tokens) {
                    chunks.push(chunk);
                } else if let Some(abilities) = parse_static_ability_ast_line_lexed(&group_tokens)?
                {
                    chunks.push(LineAst::StaticAbilities(abilities));
                } else {
                    let effects =
                        parse_effect_sentences_preserving_source_boundaries(&group_tokens)?;
                    chunks.push(LineAst::Statement { effects });
                }
            }
            return Ok(coalesce_adjacent_static_ability_chunks(chunks));
        }
    }
    Err(CardTextError::ParseError(format!(
        "rewrite statement lowering expected prepared parse tokens for '{}'",
        line.info.raw_line
    )))
}

fn parse_villainous_choice_mode_program(
    program: semantic_grammar::VillainousChoiceModeProgram<'_>,
) -> Result<Vec<EffectAst>, CardTextError> {
    match program {
        semantic_grammar::VillainousChoiceModeProgram::Direct(tokens) => {
            if tokens.len() >= 2 && tokens[0].is_word("you") && tokens[1].is_word("create") {
                return crate::effect_sentences::parse_create(
                    &tokens[1..],
                    Some(
                        crate::grammar::shared_util::reference_shapes::SubjectAst::Player(
                            PlayerAst::You,
                        ),
                    ),
                )
                .map(|effect| vec![effect]);
            }
            parse_effect_sentences_lexed(tokens)
        }
        semantic_grammar::VillainousChoiceModeProgram::SharedSubjectPair(pair) => {
            let parse_action = |action_tokens: &[OwnedLexToken]| {
                let mut clause = Vec::with_capacity(
                    pair.subject_tokens
                        .len()
                        .saturating_add(action_tokens.len()),
                );
                clause.extend_from_slice(pair.subject_tokens);
                clause.extend_from_slice(action_tokens);
                parse_effect_sentences_lexed(&clause)
            };
            let mut effects = parse_action(pair.first_action_tokens)?;
            effects.extend(parse_action(pair.second_action_tokens)?);
            Ok(effects)
        }
    }
}

fn render_statement_source_tokens(
    line: &RewriteStatementLine,
    parsed_tokens: &[OwnedLexToken],
) -> String {
    let Some(first) = parsed_tokens.first() else {
        return String::new();
    };
    let Some(last) = parsed_tokens.last() else {
        return String::new();
    };
    line.info
        .raw_line
        .get(first.span.start..last.span.end)
        .map(str::to_string)
        .unwrap_or_else(|| render_token_slice(parsed_tokens))
}

fn parse_villainous_choice_statement_chunk(
    line: &RewriteStatementLine,
) -> Result<Option<LineAst>, CardTextError> {
    let source_sentences = split_lexed_sentences(&line.info.source_tokens);
    let player_statement = source_sentences
        .iter()
        .enumerate()
        .find_map(|(index, sentence)| {
            semantic_grammar::parse_villainous_choice_player_statement_tokens(sentence)
                .map(|shape| (index, shape))
        });
    let (player_statement, mut preceding_effects) = if let Some((index, shape)) = player_statement {
        let preceding_effects = if index == 0 {
            Vec::new()
        } else {
            let preceding_sentences = source_sentences[..index]
                .iter()
                .map(|sentence| sentence.to_vec())
                .collect::<Vec<_>>();
            let preceding_tokens = join_sentences_with_period(&preceding_sentences);
            let preceding = parse_effect_sentences_preserving_source_boundaries(&preceding_tokens)?;
            if preceding
                .iter()
                .all(|effect| matches!(effect, EffectAst::SourceSentence { .. }))
            {
                preceding
            } else {
                vec![EffectAst::SourceSentence {
                    effects: preceding,
                    leading_then: false,
                    starting_with_controller: false,
                }]
            }
        };
        (Some(shape), preceding_effects)
    } else {
        (
            semantic_grammar::parse_villainous_choice_player_statement_tokens(&line.parse_tokens),
            Vec::new(),
        )
    };
    if let Some(shape) = player_statement {
        let first_mode_effects = parse_villainous_choice_mode_program(shape.first_mode_program)?;
        let second_mode_effects = parse_villainous_choice_mode_program(shape.second_mode_program)?;
        let (player, player_surface) = match shape.iteration {
            semantic_grammar::VillainousChoicePlayerIteration::EachOpponent => {
                (PlayerFilter::IteratedPlayer, "that player")
            }
            semantic_grammar::VillainousChoicePlayerIteration::TargetOpponent => {
                (PlayerFilter::target_opponent(), "target opponent")
            }
        };
        let choice = EffectAst::VillainousChoice {
            player,
            player_surface: Some(player_surface.to_string()),
            modes: vec![
                ChooseOneModeAst {
                    description: render_statement_source_tokens(line, shape.first_mode_tokens),
                    effects: first_mode_effects,
                },
                ChooseOneModeAst {
                    description: render_statement_source_tokens(line, shape.second_mode_tokens),
                    effects: second_mode_effects,
                },
            ],
        };
        let effects = match shape.iteration {
            semantic_grammar::VillainousChoicePlayerIteration::EachOpponent => {
                let body = if let Some(count) = shape.minimum_life_lost_this_turn {
                    vec![EffectAst::Conditional {
                        predicate: PredicateAst::ValueComparison {
                            left: Value::LifeLostThisTurn(PlayerFilter::IteratedPlayer),
                            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                            right: Value::Fixed(count as i32),
                        },
                        if_true: vec![choice],
                        if_false: Vec::new(),
                    }]
                } else {
                    vec![choice]
                };
                vec![EffectAst::ForEachOpponent { effects: body }]
            }
            semantic_grammar::VillainousChoicePlayerIteration::TargetOpponent => vec![
                EffectAst::subject_verb_target_only(TargetAst::Player(
                    PlayerFilter::target_opponent(),
                    Some(crate::TextSpan::synthetic()),
                )),
                choice,
            ],
        };
        let effects = if shape.leading_then {
            vec![EffectAst::SourceSentence {
                effects,
                leading_then: true,
                starting_with_controller: false,
            }]
        } else {
            effects
        };
        preceding_effects.extend(effects);
        return Ok(Some(LineAst::Statement {
            effects: preceding_effects,
        }));
    }

    let Some(shape) =
        semantic_grammar::parse_villainous_choice_statement_tokens(&line.info.source_tokens)
            .or_else(|| {
                semantic_grammar::parse_villainous_choice_statement_tokens(&line.parse_tokens)
            })
    else {
        return Ok(None);
    };
    let target_tag = crate::tag::CompilerReferenceTag::It.key();
    let mut effects = match shape.target {
        semantic_grammar::VillainousChoiceTarget::CreaturesYouDontControl => {
            let target = TargetAst::WithCount(
                Box::new(TargetAst::Object(
                    ObjectFilter::creature().controlled_by(PlayerFilter::NotYou),
                    Some(crate::TextSpan::synthetic()),
                    None,
                )),
                shape.count,
            );
            vec![EffectAst::subject_verb_target_only(target)]
        }
    };
    let first_mode_effects = parse_villainous_choice_mode_program(shape.first_mode_program)?;
    let second_mode_effects = parse_villainous_choice_mode_program(shape.second_mode_program)?;
    let player = match shape.chooser {
        semantic_grammar::VillainousChoiceChooser::IteratedCreaturesController => {
            PlayerFilter::ControllerOf(crate::target::ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::It.key(),
            ))
        }
    };
    let iteration_tag = match shape.iteration {
        semantic_grammar::VillainousChoiceIteration::EachOfThem => target_tag,
    };
    effects.push(EffectAst::ForEachTagged {
        tag: iteration_tag,
        effects: vec![EffectAst::VillainousChoice {
            player,
            player_surface: Some(render_token_slice(shape.chooser_tokens)),
            modes: vec![
                ChooseOneModeAst {
                    description: render_statement_source_tokens(line, shape.first_mode_tokens),
                    effects: first_mode_effects,
                },
                ChooseOneModeAst {
                    description: render_statement_source_tokens(line, shape.second_mode_tokens),
                    effects: second_mode_effects,
                },
            ],
        }],
    });

    Ok(Some(LineAst::Statement { effects }))
}

fn parse_die_roll_result_adjustment_static_chunk(tokens: &[OwnedLexToken]) -> Option<LineAst> {
    let rendered = render_token_slice(tokens);
    let words = crate::lexer::TokenWordView::new(tokens);
    if words.parses_prefix(&["once", "each", "turn", "you", "may", "pay"])
        && crate::word_primitives::parse_sequence_suffix(
            &words.word_refs(),
            &["to", "reroll", "one", "or", "more", "dice", "you", "rolled"],
        )
    {
        let pay_word = words.parse_word_position("pay")?;
        let pay_idx = words.map_word_to_token_start(pay_word)?;
        let mana_cost = super::super::grammar::leaf::parse_leaf_mana_cost_prefix_tokens(
            &tokens[pay_idx + 1..],
        )?
        .cost;
        return Some(LineAst::StaticAbilities(vec![
            crate::cards::builders::StaticAbilityAst::Static(StaticAbility::die_roll_reroll(
                PlayerFilter::You,
                mana_cost,
                true,
                rendered,
            )),
        ]));
    }

    let spec = semantic_grammar::parse_die_roll_adjustment_tokens(tokens)?;
    let life_cost = spec.life_cost;
    let amount = spec.adjustment;
    let display = format!(
        "After you roll a die, you may pay {life_cost} life. If you do, increase or decrease the result by {amount}. Do this only once each turn."
    );
    Some(LineAst::StaticAbilities(vec![
        crate::cards::builders::StaticAbilityAst::Static(
            StaticAbility::die_roll_result_adjustment(
                PlayerFilter::You,
                life_cost,
                amount,
                true,
                display,
            ),
        ),
    ]))
}

fn sentences_have_token_copy_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        crate::effect_sentences::parse_token_copy_followup_sentence_lexed(sentence.as_ref())
            .is_some()
    })
}

fn sentences_have_token_granted_ability_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        matches!(
            crate::effect_sentences::parse_token_granted_ability_followup_sentence_lexed(
                sentence.as_ref()
            ),
            Ok(Some(_))
        )
    })
}

fn sentences_have_token_creation_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences_have_token_copy_followup_after_first(sentences)
        || sentences_have_token_granted_ability_followup_after_first(sentences)
        || sentences.iter().skip(1).any(|sentence| {
            let sentence = sentence.as_ref();
            semantic_grammar::parse_token_characteristic_followup_tokens(sentence).is_some()
                || effect_grammar::followup_shapes::parse_create_more_prior_tokens(sentence)
                    .is_some()
                || effect_grammar::parse_create_head_tokens(strip_leading_if_you_do_lexed(sentence))
                    .is_some()
        })
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
            &token_word_refs(&shape.body_tokens),
            &["copy", "of"],
        )
}

fn sentences_have_temporary_static_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        let sentence = sentence.as_ref();
        sentence_has_typed_become_copy_exception(sentence)
            || effect_grammar::followup_shapes::parse_moved_object_entry_followup_shape(sentence)
                .is_some()
            || semantic_grammar::parse_temporary_static_followup_tokens(sentence).is_some_and(
                |facts| {
                    matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))
                        || facts.has_negation
                },
            )
    })
}

/// Recognize the complete optional collect-evidence procedure before the
/// triggered-line fallback rejects all multi-sentence `if you do` bodies.
/// The effect-sentence dispatcher owns the typed lowering; this predicate
/// only proves the exact two-sentence envelope it supports, so unrelated
/// optional procedures retain the conservative path.
fn is_optional_source_exile_collect_evidence_procedure(tokens: &[OwnedLexToken]) -> bool {
    let sentences = split_lexed_sentences(tokens);
    let [optional, followup] = sentences.as_slice() else {
        return false;
    };
    let optional_words = token_word_refs(optional);
    let followup_words = token_word_refs(followup);
    optional_words.len() == 8
        && crate::word_primitives::parse_sequence_prefix(
            &optional_words,
            &["you", "may", "exile", "it", "and", "collect", "evidence"],
        )
        && crate::util::parse_number_word_u32(optional_words[7]).is_some()
        && followup_words.len() == 10
        && followup_words[0].eq_ignore_ascii_case("if")
        && crate::word_primitives::parse_sequence_complete(
            &followup_words[1..],
            &[
                "you",
                "do",
                "return",
                "this",
                "card",
                "to",
                "the",
                "battlefield",
                "tapped",
            ],
        )
}

fn sentences_form_anaphoric_damage_self_replacement(sentences: &[Vec<OwnedLexToken>]) -> bool {
    let [_, replacement] = sentences else {
        return false;
    };
    if !effect_grammar::followup_shapes::is_anaphoric_damage_self_replacement(replacement.as_ref())
    {
        return false;
    }

    let grouped = join_sentences_with_period(sentences);
    parse_effect_sentences_preserving_source_boundaries(&grouped)
        .is_ok_and(|effects| matches!(effects.as_slice(), [EffectAst::SelfReplacement { .. }]))
}

fn sentences_have_bound_characteristic_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        effect_grammar::labeled_dispatch::parse_passive_color_type_addition_shape(sentence.as_ref())
            .is_some_and(|shape| shape.tagged_subject)
    })
}

fn returned_object_static_followup_start<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> Option<usize> {
    let first_sentence = sentences.first()?;
    semantic_grammar::parse_returned_object_move_head_tokens(first_sentence.as_ref())?;

    sentences
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(idx, sentence)| {
            let sentence = sentence.as_ref();
            let facts = semantic_grammar::parse_returned_object_followup_tokens(sentence)?;
            (matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))
                || facts.has_characteristic_changes())
            .then_some(idx)
        })
}

fn filter_is_exact_tagged_it(filter: &ObjectFilter) -> bool {
    filter == &ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key())
}

fn push_returned_object_keyword_grant_effect(
    effects: &mut Vec<EffectAst>,
    action: KeywordAction,
    condition: Option<PredicateAst>,
) {
    let target = TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None);
    let ability = GrantedAbilityAst::from(action);
    let effect = if let Some(condition) = condition {
        EffectAst::subject_verb_grant_abilities_to_target_with_condition(
            target,
            vec![ability],
            Until::Forever,
            condition,
        )
    } else {
        EffectAst::subject_verb_grant_abilities_to_target(target, vec![ability], Until::Forever)
    };
    effects.push(effect);
}

fn returned_object_static_ability_effects(
    ability: crate::cards::builders::StaticAbilityAst,
    effects: &mut Vec<EffectAst>,
) -> bool {
    match ability {
        crate::cards::builders::StaticAbilityAst::KeywordAction(action) => {
            push_returned_object_keyword_grant_effect(effects, action, None);
            true
        }
        crate::cards::builders::StaticAbilityAst::ConditionalKeywordAction {
            action,
            condition,
        } => {
            push_returned_object_keyword_grant_effect(effects, action, Some(condition));
            true
        }
        crate::cards::builders::StaticAbilityAst::GrantKeywordAction {
            filter,
            action,
            condition,
        } if filter_is_exact_tagged_it(&filter) => {
            push_returned_object_keyword_grant_effect(effects, action, condition);
            true
        }
        _ => false,
    }
}

fn returned_object_static_followup_effects<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> Result<Option<(usize, Vec<EffectAst>)>, CardTextError> {
    let Some(first_followup_idx) = returned_object_static_followup_start(sentences) else {
        return Ok(None);
    };

    let mut effects = Vec::new();
    for sentence in sentences.iter().skip(first_followup_idx) {
        let sentence = sentence.as_ref();
        let grammar_facts = semantic_grammar::parse_returned_object_followup_tokens(sentence);
        let before_len = effects.len();
        let before_keyword_len = effects.len();
        if let Some(abilities) = parse_static_ability_ast_line_lexed(sentence)? {
            for ability in abilities {
                returned_object_static_ability_effects(ability, &mut effects);
            }
        }
        if effects.len() == before_keyword_len
            && let Some(keyword_tokens) = grammar_facts
                .as_ref()
                .and_then(|facts| facts.keyword_tokens)
            && let Some(actions) = parse_ability_line_lexed(keyword_tokens)
        {
            for action in actions {
                push_returned_object_keyword_grant_effect(&mut effects, action, None);
            }
        }
        if let Some(colors) = grammar_facts.as_ref().and_then(|facts| facts.colors) {
            effects.push(EffectAst::subject_verb_add_colors(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None),
                colors,
                Until::Forever,
            ));
        }
        if let Some(subtypes) = grammar_facts
            .as_ref()
            .map(|facts| facts.subtypes.clone())
            .filter(|subtypes| !subtypes.is_empty())
        {
            effects.push(EffectAst::subject_verb_add_subtypes(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None),
                subtypes,
                Until::Forever,
            ));
        }
        if effects.len() == before_len {
            return Ok(None);
        }
    }

    Ok(Some((first_followup_idx, effects)))
}

fn sentence_is_conditional_self_replacement_effect(sentence: &[OwnedLexToken]) -> bool {
    let instead_semantics =
        crate::grammar::effects::classify_instead_followup_semantics_tokens(sentence);
    if instead_semantics != crate::cards::builders::InsteadSemantics::SelfReplacement {
        return false;
    }
    if sentence.first().is_some_and(|token| token.is_word("if"))
        && sentence.get(1).is_some_and(|token| token.is_word("you"))
        && sentence
            .get(2)
            .is_some_and(|token| token.is_any_word(&["win", "won"]))
        && sentence.get(3).is_some_and(OwnedLexToken::is_comma)
    {
        return true;
    }
    if crate::word_primitives::sequence_occurs(&token_word_refs(sentence), &["instead", "if"]) {
        return true;
    }
    if crate::grammar::line_semantic_facts::parse_line_semantic_facts_tokens(sentence)
        .statement
        .trailing_instead_if_predicate
        .is_some()
    {
        return true;
    }

    crate::parse_loss::capture(|| parse_effect_sentences_lexed(sentence))
        .0
        .is_ok_and(|effects| {
            matches!(
                effects.as_slice(),
                [EffectAst::Conditional { .. } | EffectAst::TrailingIf { .. }]
            )
        })
}

fn conditional_predicate_has_implicit_object_surface(sentence: &[OwnedLexToken]) -> bool {
    if !sentence
        .first()
        .is_some_and(|token| token.is_any_word(&["if", "unless"]))
    {
        return false;
    }
    let predicate_end = crate::slice_primitives::select_position(sentence, OwnedLexToken::is_comma)
        .unwrap_or(sentence.len());
    let predicate = &sentence[1..predicate_end];
    predicate.iter().any(|token| token.is_word("it"))
        || predicate
            .first()
            .is_some_and(|token| token.is_word("target"))
        || predicate.windows(2).any(|pair| {
            pair[0].is_word("that")
                && pair[1].is_any_word(&[
                    "card",
                    "creature",
                    "object",
                    "permanent",
                    "spell",
                    "token",
                ])
        })
}

fn sentence_is_linked_anaphoric_conditional_effect(sentence: &[OwnedLexToken]) -> bool {
    sentence_is_conditional_self_replacement_effect(sentence)
        || conditional_predicate_has_implicit_object_surface(sentence)
}

fn linked_statement_should_stay_grouped(tokens: &[OwnedLexToken]) -> bool {
    let sentences = split_lexed_sentences(tokens);
    if let [first_sentence, fallback_sentence] = sentences.as_slice()
        && matches!(
            crate::grammar::primitives::probe_shape(parse_effect_sentences_lexed(
                fallback_sentence
            ))
            .as_deref(),
            Some([EffectAst::IfResult {
                predicate: crate::cards::builders::IfResultPredicate::DidNot,
                ..
            }])
        )
        && matches!(
            crate::grammar::primitives::probe_shape(parse_effect_sentences_lexed(first_sentence))
                .as_deref(),
            Some([EffectAst::CommaThen { .. }])
        )
        && (matches!(
            crate::grammar::primitives::probe_shape(parse_effect_sentences_lexed(tokens)).as_deref(),
            Some([EffectAst::CommaThen { effects }])
                if matches!(effects.last(), Some(EffectAst::IfResult {
                    predicate: crate::cards::builders::IfResultPredicate::DidNot,
                    ..
                }))
        ) || matches!(
            crate::grammar::primitives::probe_shape(parse_effect_sentences_preserving_source_boundaries(tokens))
                .as_deref(),
            Some([
                EffectAst::SourceSentence {
                    effects: first,
                    ..
                },
                EffectAst::SourceSentence {
                    effects: fallback,
                    ..
                },
            ]) if matches!(first.as_slice(), [EffectAst::CommaThen { .. }])
                && matches!(fallback.as_slice(), [EffectAst::IfResult {
                    predicate: crate::cards::builders::IfResultPredicate::DidNot,
                    ..
                }])
        ))
    {
        // The fallback consumes the outcome of the complete first-sentence
        // sequence, so statement grouping must not prepare the two sentences
        // as unrelated chunks.
        return true;
    }
    if let [default_copy, replacement_copy] = sentences.as_slice()
        && tokens.iter().any(|token| token.kind == TokenKind::Quote)
    {
        let default_words = token_word_refs(default_copy);
        let replacement_words = token_word_refs(replacement_copy);
        let is_copy_creation = |words: &[&str]| {
            crate::word_primitives::sequence_occurs(words, &["create", "a"])
                && crate::word_primitives::sequence_occurs(words, &["token", "that"])
                && crate::word_primitives::sequence_occurs(words, &["copy", "of"])
        };
        if is_copy_creation(&default_words)
            && is_copy_creation(&replacement_words)
            && crate::word_primitives::contains_word(&replacement_words, "instead")
            && crate::parse_loss::capture(|| parse_effect_sentences_lexed(tokens))
                .0
                .is_ok_and(|effects| {
                    effects
                        .iter()
                        .any(|effect| matches!(effect, EffectAst::SelfReplacement { .. }))
                })
        {
            return true;
        }
    }
    if let [replacement, delayed_return] = sentences.as_slice()
        && crate::grammar::effects::is_filtered_future_exile_return_next_end_step_shape(
            replacement,
            delayed_return,
        )
    {
        return true;
    }

    let line_family = classify_statement_line_family_lexed(tokens);
    if matches!(
        line_family,
        Some(
            StatementLineFamily::Divvy
                | StatementLineFamily::Emblem
                | StatementLineFamily::PactNextUpkeep
                | StatementLineFamily::ExilePlayCostsMore
        )
    ) {
        return true;
    }

    // A choice/event replacement can be followed by effects that are common
    // to both the default and replacement arms.  Parsing those source
    // sentences independently loses that shared tail.  Keep the statement
    // grouped only when the ordinary typed sentence parser proves that the
    // complete source is one self-replacement program; an unrelated
    // conditional or an `instead` sentence without a common tail does not
    // satisfy this shape.
    if parse_complete_self_replacement_statement(tokens).is_some() {
        return true;
    }

    // A conditional sentence whose predicate says `it` can look like a
    // battlefield static ability in isolation. When it follows a resolution
    // instruction, however, `it` is the object selected or produced by that
    // instruction, so the sentences must share one reference environment.
    if sentences.iter().skip(1).any(|sentence| {
        sentence_is_conditional_self_replacement_effect(sentence)
            || conditional_predicate_has_implicit_object_surface(sentence)
    }) {
        return true;
    }

    // A typed statement-replacement surface can depend on both authored
    // sentences (for example Whirlpool Whelm's clash result and its following
    // "instead" sentence). Keep the program intact through semantic lowering.
    crate::grammar::lowering_surfaces::parse_statement_replacement_surface_tokens(tokens).is_some()
        || semantic_grammar::parse_linked_statement_surface_tokens(tokens).is_some()
}

fn parse_complete_self_replacement_statement(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let words = token_word_refs(tokens);
    if split_lexed_sentences(tokens).len() < 3
        || (!crate::word_primitives::contains_word(&words, "instead")
            && !crate::word_primitives::sequence_occurs(&words, &["rather", "than"]))
    {
        return None;
    }
    let effects = crate::grammar::primitives::probe_shape(
        crate::parse_loss::capture(|| parse_effect_sentences_lexed(tokens)).0,
    )?;
    matches!(effects.as_slice(), [EffectAst::SelfReplacement { .. }]).then_some(effects)
}

#[cfg(test)]
#[path = "lines/statement_grouping_tests.rs"]
mod statement_grouping_tests;

#[path = "lines/lines_choice.rs"]
mod lines_choice_programs;
pub use lines_choice_programs::rewrite_modal_to_parsed_item;
use lines_choice_programs::{
    specialize_modal_common_target_suffix, try_parse_chosen_type_behold_two_additional_cost,
};
#[path = "lines/lines_resource.rs"]
mod lines_resource_programs;
use lines_resource_programs::{
    capitalize_first_equip_cost_alternative_display,
    full_text_has_non_mana_activated_ability_qualifier, mark_non_mana_activated_line,
};
pub use lines_resource_programs::{
    try_parse_optional_behold_additional_cost, try_parse_optional_waterbend_additional_cost,
};
#[path = "lines/lines_trigger.rs"]
mod lines_trigger_programs;
#[cfg(test)]
use lines_trigger_programs::{
    collect_evidence_if_do_procedure_reaches_the_public_trigger_route,
    created_token_next_turn_sacrifice_stays_inside_the_trigger,
    dynamic_death_group_token_creation_reaches_the_public_trigger_semantic_handoff,
    dynamic_exile_permission_bundle_reaches_the_public_trigger_route,
    dynamic_static_ability_token_count_survives_the_public_trigger_handoff,
    independent_trigger_sentences_reach_the_public_semantic_handoff, parse_triggered_text_for_test,
    quantified_token_rules_reach_the_public_trigger_semantic_handoff,
    semantic_trigger_root_restores_single_target_source_exclusion,
    serial_target_modifier_reconciliation_reaches_the_public_trigger_route,
    source_spell_surface_repair_does_not_erase_a_zone_change_trigger_arm,
    test_rewrite_triggered_line,
    triggered_line_source_text_keeps_labelled_raw_do_this_only_once_suffix,
    triggered_line_source_text_keeps_raw_do_this_only_once_suffix,
    triggered_semantic_split_keeps_effect_backed_static_surfaces_in_resolution,
};
use lines_trigger_programs::{
    hoist_delayed_copy_retargeting_in_line, lower_special_rewrite_triggered_divvy,
    lower_special_rewrite_triggered_head, lower_special_rewrite_triggered_oath,
    lower_special_rewrite_triggered_tail, lower_spell_or_activated_ability_x_cost_trigger,
    mark_non_mana_activated_trigger, parse_triggered_ability_line_impl, parse_triggered_line_impl,
    recognize_triggered_effect_surfaces,
};
pub use lines_trigger_programs::{
    is_exact_correlated_trigger_effect_bundle, parse_special_triggered_line, parse_triggered_line,
    try_parse_optional_cost_with_cast_trigger,
};
#[path = "lines/lines_object_action.rs"]
mod lines_object_action_programs;
#[cfg(any(test, feature = "test-support"))]
pub use lines_object_action_programs::parse_keyword_line_with_full_tokens_for_test;
#[cfg(test)]
use lines_object_action_programs::{
    additional_land_play_static_count_uses_token_words,
    graveyard_copy_cast_accepts_conditional_copy_and_one_cast_result_tail,
    graveyard_copy_cast_accepts_only_the_standard_copy_cast_reminder_suffix,
    hideaway_special_case_uses_parse_tokens,
    partner_name_and_visible_label_trim_on_lexed_reminder_tokens,
};
use lines_object_action_programs::{
    partner_with_name_from_tokens, rewrite_copy_count_to_times_paid_label_rewrite,
    standard_gift_create_token_effect, try_lower_hideaway_tokens, try_lower_partner_with_tokens,
};
#[path = "lines/lines_core.rs"]
mod lines_core_programs;
use lines_core_programs::hideaway_line_ast;
#[cfg(test)]
use lines_core_programs::test_line_info;
#[cfg(any(test, feature = "test-support"))]
pub use lines_core_programs::{parse_single_effect_lexed, strip_lexed_suffix_phrase};
#[path = "lines/lines_ability.rs"]
mod lines_ability_programs;
#[cfg(any(test, feature = "test-support"))]
pub use lines_ability_programs::parse_keyword_line_for_test;
use lines_ability_programs::{
    parse_day_night_starts_day_static_chunk, parse_static_line_impl, try_lower_hideaway_keyword,
    try_lower_partner_variant_keyword,
};
pub use lines_ability_programs::{parse_keyword_special_cases, parse_static_line};
#[cfg(test)]
use lines_ability_programs::{
    standard_flanking_reminder_is_typed_without_broad_keyword_expansion,
    standard_menace_reminder_is_typed_without_broad_keyword_expansion,
};
#[path = "lines/lines_condition.rs"]
mod lines_condition_programs;
pub use lines_condition_programs::parse_gift_keyword_line;
use lines_condition_programs::{fixed_standard_gift_creature_definition, standard_gift_effects};
#[path = "lines/lines_combat.rs"]
mod lines_combat_programs;
pub use lines_combat_programs::parse_exert_attack_keyword_line;
#[cfg(test)]
use lines_combat_programs::{
    generic_triggered_source_pump_unblockable_keeps_both_effects,
    protected_battle_surface_binds_the_pre_lowering_damage_target_inside_opponent_loop,
};
#[path = "lines/lines_reference.rs"]
mod lines_reference_programs;
use lines_reference_programs::membership_predicate_for_iterated_object;
#[cfg(test)]
use lines_reference_programs::{
    atomic_return_as_aura_bundle_preempts_returned_object_static_split,
    source_sentence_boundaries_preserve_jointly_parsed_reference_flow,
    tagged_characteristic_addition_is_a_bound_effect_followup,
    targeted_same_name_graveyard_cast_keeps_target_and_optional_normal_payment,
};
pub use lines_reference_programs::{
    exact_target_graveyard_any_type_may_cast_bundle,
    exact_target_same_name_graveyard_may_cast_bundle,
    normalize_exert_followup_source_reference_tokens,
};
#[path = "lines/lines_counter.rs"]
mod lines_counter_programs;
#[cfg(test)]
use lines_counter_programs::{
    ability_word_marker_detection_uses_token_kinds,
    exiled_last_counter_qualifier_stays_on_the_trigger_side_of_the_comma,
};
use lines_counter_programs::{
    lower_spell_cast_snow_mana_enter_counter_static_chunk, parse_exiled_last_counter_triggered_line,
};
#[path = "lines/lines_library.rs"]
mod lines_library_programs;
use lines_library_programs::starts_with_exact_graveyard_card_copy_cast_sequence;
pub use lines_library_programs::{
    exact_graveyard_card_copy_cast_sequence, exact_looked_hand_optional_cast_bundle,
    is_authored_look_hand_optional_cast_bundle,
    parse_library_origin_source_pump_unblockable_triggered_line,
};
#[cfg(test)]
use lines_library_programs::{
    library_origin_source_pump_unblockable_preemption_is_exact,
    looked_hand_optional_cast_authored_guard_keeps_possessive_and_may,
};
#[path = "lines/lines_permission.rs"]
mod lines_permission_programs;
use lines_permission_programs::exact_dynamic_exile_permission_bundle;
pub use lines_permission_programs::is_authored_dynamic_exile_permission_bundle;
#[path = "lines/lines_zone.rs"]
mod lines_zone_programs;
use lines_zone_programs::exact_atomic_return_as_aura_bundle;

#[path = "lines/spell_cast_trigger_filters.rs"]
mod spell_cast_trigger_filters;
use spell_cast_trigger_filters::spell_cast_trigger_filter;
#[path = "lines/self_counter_entry.rs"]
mod self_counter_entry;
use self_counter_entry::parse_self_enters_with_x_counters_static_chunk;
#[path = "lines/statement_routing.rs"]
mod statement_routing;
use statement_routing::statement_group_should_parse_as_effects_first;
#[cfg(test)]
#[path = "lines/selected_hand_reveal_tests.rs"]
mod selected_hand_reveal_tests;
#[cfg(test)]
use selected_hand_reveal_tests::{
    selected_hand_reveal_token_creation_uses_the_unabridged_source_program,
    whole_hand_reveal_does_not_match_selected_hand_sequence,
};
