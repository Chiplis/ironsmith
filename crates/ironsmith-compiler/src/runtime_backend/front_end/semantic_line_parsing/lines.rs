use super::*;
use crate::ZoneReplacementDurationAst;
use crate::runtime_backend::GrantedAbilityAst;
use crate::runtime_backend::ast::{
    ChooseOneModeAst, SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbSubjectAst,
};
use crate::runtime_backend::grammar::abilities::{
    is_minimum_spell_total_mana_three_line_lexed, is_players_cant_pay_life_or_sacrifice_line_lexed,
};
use crate::runtime_backend::grammar::keyword_special_lines as keyword_special_grammar;
use crate::runtime_backend::grammar::semantic_lowering as semantic_grammar;
use crate::runtime_backend::grammar::structure::{
    StatementLineFamily, classify_statement_line_family_lexed,
};
use crate::{KeywordAction, Value};

const STANDARD_MENACE_REMINDER: &str =
    "Menace (This creature can't be blocked except by two or more creatures.)";
const STANDARD_FLANKING_REMINDER: &str = "Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)";
const STANDARD_OPEN_ATTRACTION_REMINDER: &str =
    "(Put the top card of your Attraction deck onto the battlefield.)";

fn has_standard_menace_reminder(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        token_word_refs(tokens).as_slice(),
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
                crate::static_abilities::StaticAbilityPayload::CopyStaticAbilityVariants(_)
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
        let crate::static_abilities::StaticAbilityPayload::CopyStaticAbilityVariants(copy) =
            &mut ability.payload
        else {
            continue;
        };
        copy.display = display.to_string();
        ability.label = display.to_string();
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

pub(crate) fn dynamic_zone_change_group_token_creation_from_authored_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // The document route supplies the intact trigger line, while the
    // prepared triggered-chunk route supplies only the effect tail. Both are
    // grammar-proven source slices; accept either shell before validating the
    // exact dynamic zone-change-group payload below.
    let effect_tokens = semantic_grammar::parse_comma_split_tokens(tokens)
        .map(|split| split.after)
        .unwrap_or(tokens);
    let Ok(effect) = crate::runtime_backend::effect_sentences::parse_create(effect_tokens, None)
    else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(subject_verb) = &effect else {
        return Ok(None);
    };
    let SubjectVerbActionAst::CreateTokenWithMods {
        dynamic_power_toughness: Some((power, toughness)),
        ..
    } = &subject_verb.action
    else {
        return Ok(None);
    };
    let is_zone_change_group_total_power = |value: &Value| {
        matches!(
            value.unhinted(),
            Value::TotalPower(filter)
                if filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == ironsmith_core::ZONE_CHANGE_GROUP_TAG
                        && constraint.relation
                            == crate::target::TaggedOpbjectRelation::IsTaggedObject
                })
        )
    };
    if !is_zone_change_group_total_power(power) || !is_zone_change_group_total_power(toughness) {
        return Ok(None);
    }
    Ok(Some(effect))
}

fn dynamic_static_ability_count_token_creation_from_authored_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // The authored aggregate itself contains commas (both after `tokens` and
    // throughout the ability list), so a generic comma split can start the
    // reparse at `where X is ...` and silently miss the create action. Try
    // actual create-verb token boundaries, from the last one backwards; the
    // typed value guard below keeps quoted or trigger-side creates from being
    // claimed.
    let mut starts = crate::runtime_backend::lexer::parser_token_word_positions(tokens)
        .into_iter()
        .filter_map(|(index, word)| matches!(word, "create" | "creates").then_some(index))
        .collect::<Vec<_>>();
    starts.dedup();
    for start in starts.into_iter().rev() {
        let Ok(effects) = crate::runtime_backend::effect_sentences::parse_effect_sentences_lexed(
            &tokens[start..],
        ) else {
            continue;
        };
        let [effect] = effects.as_slice() else {
            continue;
        };
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            continue;
        };
        let SubjectVerbActionAst::CreateTokenWithMods { count, .. } = &subject_verb.action else {
            continue;
        };
        if matches!(count.unhinted(), Value::StaticAbilitiesAmong { .. }) {
            return Ok(Some(effect.clone()));
        }
    }
    Ok(None)
}

fn authored_dynamic_token_creation_from_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if let Some(effect) = dynamic_zone_change_group_token_creation_from_authored_trigger(tokens)? {
        return Ok(Some(effect));
    }
    dynamic_static_ability_count_token_creation_from_authored_trigger(tokens)
}

fn reconcile_dynamic_zone_change_group_token_creation(
    line: &mut LineAst,
    source_tokens: &[OwnedLexToken],
) -> Result<(), CardTextError> {
    let Some(effect) = authored_dynamic_token_creation_from_trigger(source_tokens)? else {
        return Ok(());
    };
    match line {
        LineAst::Triggered { effects, .. } => *effects = vec![effect],
        LineAst::Ability(ability) => {
            ability.effects_ast = Some(vec![effect]);
            let recompiled = super::super::compile_support::compile_trigger_effects(
                ability.trigger_spec.as_ref(),
                ability.effects_ast.as_deref().unwrap_or_default(),
            )
            .ok();
            if let AbilityKind::Triggered(triggered) = ability.kind_mut()
                && let Some((effects, choices)) = recompiled
            {
                triggered.effects = crate::resolution::ResolutionProgram::from_effects(effects);
                triggered.choices = choices;
            }
        }
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                reconcile_dynamic_zone_change_group_token_creation(chunk, source_tokens)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn replace_triggered_effects(
    line: &mut LineAst,
    replacement: &[EffectAst],
) -> Result<(), CardTextError> {
    match line {
        LineAst::Triggered { effects, .. } => *effects = replacement.to_vec(),
        LineAst::Ability(ability) => {
            ability.effects_ast = Some(replacement.to_vec());
            let (effects, choices) = super::super::compile_support::compile_trigger_effects(
                ability.trigger_spec.as_ref(),
                replacement,
            )?;
            if let AbilityKind::Triggered(triggered) = ability.kind_mut() {
                triggered.effects = crate::resolution::ResolutionProgram::from_effects(effects);
                triggered.choices = choices;
            }
        }
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                replace_triggered_effects(chunk, replacement)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn replace_trigger_spec(line: &mut LineAst, replacement: &TriggerSpec) {
    match line {
        LineAst::Triggered { trigger, .. } => *trigger = replacement.clone(),
        LineAst::Ability(ability) => {
            ability.trigger_spec = Some(replacement.clone());
            if let AbilityKind::Triggered(triggered) = ability.kind_mut() {
                triggered.trigger =
                    super::super::compile_support::compile_trigger_spec(replacement.clone());
            }
        }
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                replace_trigger_spec(chunk, replacement);
            }
        }
        _ => {}
    }
}

fn spell_or_activated_ability_x_cost_trigger_spec() -> TriggerSpec {
    let mut spell_filter = ObjectFilter::instant_or_sorcery();
    spell_filter.has_x_in_cost = true;
    let mut ability_filter = ObjectFilter::default();
    ability_filter.has_x_in_cost = true;
    TriggerSpec::Either(
        Box::new(TriggerSpec::SpellCast {
            filter: Some(spell_filter),
            mana_source_filter: None,
            caster: PlayerFilter::You,
            timing: None,
            during_turn: None,
            min_spells_this_turn: None,
            exact_spells_this_turn: None,
            from_not_hand: false,
        }),
        Box::new(TriggerSpec::AbilityActivated {
            activator: PlayerFilter::You,
            filter: ability_filter,
            non_mana_only: false,
            loyalty_only: false,
            activation_cost_has_tap: None,
        }),
    )
}

fn reconcile_authored_correlated_trigger_programs(
    line: &mut LineAst,
    source_tokens: &[OwnedLexToken],
) -> Result<(), CardTextError> {
    let Some(split) = semantic_grammar::parse_comma_split_tokens(source_tokens) else {
        return Ok(());
    };
    let words = crate::runtime_backend::lexer::parser_token_word_refs(split.after);

    let parley = words
        .windows(7)
        .any(|w| w == ["each", "player", "reveals", "the", "top", "card", "of"])
        && words
            .windows(7)
            .any(|w| w == ["for", "each", "nonland", "card", "revealed", "this", "way"])
        && words
            .windows(5)
            .any(|w| w == ["each", "player", "draws", "a", "card"]);
    if parley {
        let sentence_tokens = split_lexed_sentences(split.after);
        let terminal_draw = sentence_tokens.iter().position(|sentence| {
            crate::runtime_backend::lexer::parser_token_word_refs(sentence)
                .windows(5)
                .any(|words| words == ["each", "player", "draws", "a", "card"])
        });
        if let Some(terminal_draw) = terminal_draw {
            // Named-token lexing appends the token's reminder definition to
            // the source token stream. The grammar-proven terminal draw owns
            // the end of the authored Parley procedure; do not let a reminder
            // after it become another resolution instruction. Some Parley
            // programs have a token and a pump between reveal and draw, so a
            // fixed sentence count would discard a real final effect.
            let authored = sentence_tokens[..=terminal_draw]
                .iter()
                .map(|tokens| tokens.to_vec())
                .collect::<Vec<_>>();
            let authored =
                crate::runtime_backend::front_end::shared::util::join_sentences_with_period(
                    &authored,
                );
            let effects = parse_effect_sentences_lexed(&authored)?;
            replace_triggered_effects(line, &effects)?;
            return Ok(());
        }
    }

    let gate_partition = words
        .windows(8)
        .any(|w| w == ["look", "at", "the", "top", "nine", "cards", "of", "your"])
        && words
            .windows(7)
            .any(|w| w == ["put", "a", "gate", "card", "from", "among", "them"])
        && words
            .windows(7)
            .any(|w| w == ["if", "you", "control", "nine", "or", "more", "gates"])
        && words
            .windows(5)
            .any(|w| w == ["otherwise", "put", "the", "rest", "on"]);
    if gate_partition {
        let sentence_tokens = split_lexed_sentences(split.after);
        let sentences = sentence_tokens
            .iter()
            .map(|tokens| {
                crate::runtime_backend::effect_sentences::SentenceInput::from_lexed(tokens)
            })
            .collect::<Vec<_>>();
        if sentences.len() == 4
            && let Some(effects) = crate::runtime_backend::effect_sentences::
                parse_look_at_top_optional_battlefield_then_conditional_remainder(&sentences, 0)?
        {
            replace_triggered_effects(line, &effects)?;
            return Ok(());
        }
    }

    let full_words = crate::runtime_backend::lexer::parser_token_word_refs(source_tokens);
    let spell_or_ability_x_cost =
        semantic_grammar::parse_spell_or_activated_ability_x_cost_trigger_tokens(
            source_tokens,
            split.before,
            split.after,
        )
        .is_some()
            || (full_words.windows(11).any(|w| {
                w == [
                    "you", "cast", "an", "instant", "or", "sorcery", "spell", "or", "activate",
                    "an", "ability",
                ]
            }) && full_words.windows(9).any(|w| {
                w == [
                    "copy", "that", "spell", "or", "ability", "you", "may", "choose", "new",
                ]
            }));
    if spell_or_ability_x_cost {
        replace_trigger_spec(line, &spell_or_activated_ability_x_cost_trigger_spec());
    }
    Ok(())
}

fn reconcile_open_attraction_reminder(line: &mut LineAst, raw_line: &str) {
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
            crate::runtime_backend::model::effect_ast_traversal::for_each_nested_effect_vec_mut(
                effect,
                true,
                |nested| mark(nested),
            );
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

pub(crate) fn linked_created_token_next_turn_sacrifice_effects(
    effect_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !has_linked_created_token_next_turn_sacrifice_surface(effect_tokens) {
        return Ok(None);
    }
    let sentences = split_lexed_sentences(effect_tokens);
    let [create_sentence, delayed_sentence] = sentences.as_slice() else {
        return Ok(None);
    };
    let delayed_words = crate::runtime_backend::lexer::parser_token_word_refs(delayed_sentence);
    if delayed_words.as_slice()
        != [
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
        ]
    {
        return Ok(None);
    }

    let mut created = parse_effect_sentences_lexed(create_sentence)?;
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
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
            1,
            None,
        )],
    });
    Ok(Some(created))
}

pub(crate) fn has_linked_created_token_next_turn_sacrifice_surface(
    effect_tokens: &[OwnedLexToken],
) -> bool {
    let sentences = split_lexed_sentences(effect_tokens);
    let [create_sentence, delayed_sentence] = sentences.as_slice() else {
        return false;
    };
    let create_words = crate::runtime_backend::lexer::parser_token_word_refs(create_sentence);
    let delayed_words = crate::runtime_backend::lexer::parser_token_word_refs(delayed_sentence);
    create_words.first() == Some(&"create")
        && create_words.iter().any(|word| *word == "token")
        && delayed_words.as_slice()
            == [
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
            ]
}

fn tokens_after_using_mana_produced_by(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    const QUALIFIER: [&str; 4] = ["using", "mana", "produced", "by"];
    let start = tokens.windows(QUALIFIER.len()).position(|window| {
        window
            .iter()
            .zip(QUALIFIER)
            .all(|(token, word)| token.is_word(word))
    })?;
    let tail = &tokens[start + QUALIFIER.len()..];
    let end = tail
        .iter()
        .position(|token| token.kind == TokenKind::Comma)
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
                triggered.trigger =
                    super::super::compile_support::compile_trigger_spec(updated_trigger_spec);
            }
        }
        _ => {}
    }
}

fn single_target_other_than_source_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    const SINGLE_TARGET_PREFIX: [&str; 4] = ["targets", "only", "a", "single"];
    let target_start = tokens
        .windows(SINGLE_TARGET_PREFIX.len())
        .position(|window| {
            window
                .iter()
                .zip(SINGLE_TARGET_PREFIX)
                .all(|(token, word)| token.is_word(word))
        })?;
    let target_tail = &tokens[target_start + SINGLE_TARGET_PREFIX.len()..];
    let clause_end = target_tail
        .iter()
        .position(|token| token.kind == TokenKind::Comma)
        .unwrap_or(target_tail.len());
    let target_clause = &target_tail[..clause_end];
    let exclusion_start = target_clause
        .windows(2)
        .position(|window| window[0].is_word("other") && window[1].is_word("than"))?;
    let source = trim_lexed_commas(&target_clause[exclusion_start + 2..]);
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
        } if filter.target_count == Some(ChoiceCount::exactly(1)) => {
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
                triggered.trigger =
                    super::super::compile_support::compile_trigger_spec(updated_trigger_spec);
            }
        }
        _ => {}
    }
}

fn has_each_battle_they_protect_surface(tokens: &[OwnedLexToken]) -> bool {
    const SURFACE: [&str; 4] = ["each", "battle", "they", "protect"];
    tokens.windows(SURFACE.len()).any(|window| {
        window
            .iter()
            .zip(SURFACE)
            .all(|(token, word)| token.is_word(word))
    })
}

fn source_spell_cast_trigger_spec(tokens: &[OwnedLexToken]) -> Option<TriggerSpec> {
    let intro = super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(tokens)?;
    let clause_end = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Comma)?;
    let trigger_tokens = tokens.get(1..clause_end)?;
    // The specialized spell-activity probe intentionally accepts prefixes.
    // Never let it replace a complete `spell cast or zone change` trigger
    // with only its spell branch; require the ordinary trigger grammar to
    // prove that the entire authored clause is one SpellCast trigger.
    let trigger = parse_trigger_clause_lexed(trigger_tokens).ok()?;
    if !matches!(trigger, TriggerSpec::SpellCast { .. }) {
        return None;
    }
    Some(TriggerSpec::WithIntro {
        intro,
        trigger: Box::new(trigger),
    })
}

fn apply_source_spell_cast_trigger_spec(chunk: &mut LineAst, source: &[OwnedLexToken]) {
    let Some(source_trigger) = source_spell_cast_trigger_spec(source) else {
        return;
    };
    match chunk {
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                apply_source_spell_cast_trigger_spec(chunk, source);
            }
        }
        LineAst::Triggered { trigger, .. } => *trigger = source_trigger,
        LineAst::Ability(parsed) => {
            parsed.trigger_spec = Some(source_trigger.clone());
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                triggered.trigger =
                    super::super::compile_support::compile_trigger_spec(source_trigger);
            }
        }
        _ => {}
    }
}

fn bind_protected_battle_iteration_in_effects(effects: &mut [EffectAst], in_opponent_loop: bool) {
    fn bind_filter(filter: &mut ObjectFilter) {
        if filter.zone == Some(Zone::Battlefield)
            && filter.card_types == [CardType::Battle]
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
        crate::runtime_backend::model::effect_ast_traversal::for_each_nested_effects_mut(
            effect,
            true,
            |nested| {
                bind_protected_battle_iteration_in_effects(
                    nested,
                    in_opponent_loop || enters_opponent_loop,
                )
            },
        );
    }
}

fn bind_protected_battle_iteration_in_effect(
    effect: &crate::effect::Effect,
    in_opponent_loop: bool,
) -> crate::effect::Effect {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        let mut rewritten = sequence.clone();
        rewritten.effects = rewritten
            .effects
            .iter()
            .map(|nested| bind_protected_battle_iteration_in_effect(nested, in_opponent_loop))
            .collect();
        return crate::effect::Effect::new(rewritten);
    }
    if let Some(players) =
        effect.downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
    {
        let mut rewritten = players.clone();
        let enters_opponent_loop = players.filter == PlayerFilter::Opponent;
        rewritten.effects = rewritten
            .effects
            .iter()
            .map(|nested| {
                bind_protected_battle_iteration_in_effect(
                    nested,
                    in_opponent_loop || enters_opponent_loop,
                )
            })
            .collect();
        return crate::effect::Effect::new(rewritten);
    }
    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        let mut rewritten = for_each.clone();
        if in_opponent_loop
            && rewritten.filter.zone == Some(Zone::Battlefield)
            && rewritten.filter.card_types == [CardType::Battle]
            && rewritten.filter.protected_by.is_none()
        {
            rewritten.filter.protected_by = Some(PlayerFilter::IteratedPlayer);
        }
        rewritten.effects = rewritten
            .effects
            .iter()
            .map(|nested| bind_protected_battle_iteration_in_effect(nested, in_opponent_loop))
            .collect();
        return crate::effect::Effect::new(rewritten);
    }
    effect.clone()
}

fn bind_protected_battle_iteration_in_runtime(triggered: &mut crate::ability::TriggeredAbility) {
    let mut segments = triggered.effects.segments.clone();
    for segment in &mut segments {
        segment.default_effects = segment
            .default_effects
            .iter()
            .map(|effect| bind_protected_battle_iteration_in_effect(effect, false))
            .collect();
        for replacement in &mut segment.self_replacements {
            replacement.replacement_effects = replacement
                .replacement_effects
                .iter()
                .map(|effect| bind_protected_battle_iteration_in_effect(effect, false))
                .collect();
        }
    }
    triggered.effects = crate::resolution::ResolutionProgram::new(segments);
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
            let recompiled = parsed.effects_ast.as_ref().and_then(|effects| {
                super::super::compile_support::compile_trigger_effects(
                    parsed.trigger_spec.as_ref(),
                    effects,
                )
                .ok()
            });
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                if let Some((effects, choices)) = recompiled {
                    triggered.effects = crate::resolution::ResolutionProgram::from_effects(effects);
                    triggered.choices = choices;
                } else {
                    bind_protected_battle_iteration_in_runtime(triggered);
                }
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
            crate::runtime_backend::model::effect_ast_traversal::for_each_nested_effects(
                effect,
                true,
                |nested| collect(nested, surfaces),
            );
        }
    }

    let surfaced = parse_effect_sentences_preserving_source_boundaries(effect_tokens).ok()?;
    let mut surfaces = Vec::new();
    collect(&surfaced, &mut surfaces);
    let [surface] = surfaces.as_slice() else {
        return None;
    };
    Some(surface.clone())
}

fn reconcile_named_explore_source_surface(
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
            crate::runtime_backend::model::effect_ast_traversal::for_each_nested_effects(
                effect,
                true,
                |nested| count += candidate_count(nested),
            );
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
            crate::runtime_backend::model::effect_ast_traversal::for_each_nested_effects_mut(
                effect,
                true,
                |nested| changed |= apply(nested, surface),
            );
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
                    let (effects, choices) =
                        super::super::compile_support::compile_trigger_effects(
                            parsed.trigger_spec.as_ref(),
                            parsed.effects_ast.as_deref().unwrap_or_default(),
                        )?;
                    if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                        triggered.effects =
                            crate::resolution::ResolutionProgram::from_effects(effects);
                        triggered.choices = choices;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    let authored_surface = || {
        let source_name =
            crate::runtime_backend::front_end::shared::util::current_source_reference_name()?;
        let lowercase_line = raw_line.to_ascii_lowercase();
        let full_name_phrase = format!("{} explores", source_name.to_ascii_lowercase());
        if lowercase_line.contains(&full_name_phrase) {
            return Some(crate::target::SourceReferenceSurface::FullName(source_name));
        }
        let short_name = source_name.split(',').next()?.trim();
        let short_name_phrase = format!("{} explores", short_name.to_ascii_lowercase());
        lowercase_line
            .contains(&short_name_phrase)
            .then(|| crate::target::SourceReferenceSurface::ShortName(short_name.to_string()))
    };
    let Some(surface) =
        authored_surface().or_else(|| grammar_proven_named_explore_surface(effect_tokens))
    else {
        return Ok(());
    };
    if line_candidate_count(chunk) != 1 {
        return Ok(());
    }
    apply_to_line(chunk, &surface)
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
    if super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(body_tokens)
        .is_none()
    {
        return None;
    }

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

pub(crate) fn parse_statement_token_groups_to_chunks(
    info: LineInfo,
    parse_tokens: &[OwnedLexToken],
    parse_groups: &[Vec<OwnedLexToken>],
) -> Result<Vec<LineAst>, CardTextError> {
    parse_statement_to_chunks_impl(
        &RewriteStatementLine {
            info,
            parse_tokens: parse_tokens.to_vec(),
        },
        parse_tokens,
        parse_groups,
    )
}

fn exact_destroy_no_regeneration_statement(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .map(crate::runtime_backend::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    if sentences.len() != 2 {
        return None;
    }
    crate::runtime_backend::effect_sentences::parse_destroy_then_no_regeneration_sequence(
        &sentences, 0,
    )
    .ok()?
}

fn exact_hidden_partition_permission_statement(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .map(crate::runtime_backend::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    if sentences.len() != 3 {
        return None;
    }
    crate::runtime_backend::effect_sentences::parse_look_at_top_partition_face_down_then_filtered_permission(
        &sentences,
        0,
    )
    .ok()?
}

fn exact_historical_target_return_statement(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens);
    let [choose, return_them, draw] = sentences.as_slice() else {
        return None;
    };
    let choose_words = crate::runtime_backend::lexer::parser_token_word_refs(choose);
    let return_words = crate::runtime_backend::lexer::parser_token_word_refs(return_them);
    let draw_words = crate::runtime_backend::lexer::parser_token_word_refs(draw);
    if !choose_words.starts_with(&[
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
    ]) || !return_words.starts_with(&["return", "them", "to", "the", "battlefield"])
        || !draw_words.starts_with(&[
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
        ])
    {
        return None;
    }
    crate::runtime_backend::effect_sentences::parse_effect_sentences_lexed(tokens).ok()
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
    let selected =
        crate::runtime_backend::effect_sentences::parse_effect_chain_lexed(first).ok()?;
    let mut effects = parse_effect_sentences_preserving_source_boundaries(tokens).ok()?;
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
    if let Some(effects) =
        crate::runtime_backend::effect_sentences::parse_target_gets_unblockable_subject_verb(
            &line.info.source_tokens,
        )?
        .or(
            crate::runtime_backend::effect_sentences::parse_target_gets_unblockable_subject_verb(
                parse_tokens,
            )?,
        )
    {
        return Ok(vec![LineAst::Statement { effects }]);
    }
    // A graveyard card is not a spell on the stack. The exact reusable
    // graveyard-copy/cast sequence lowers the authored card copy directly to
    // CastTagged(as_copy); splitting its sentences for statement-surface
    // preservation would instead reparse `copy it` as CopySpell. Give the
    // grammar-proven sequence the same first refusal here that it receives in
    // triggered lines.
    if let Some(effects) = exact_graveyard_card_copy_cast_sequence(parse_tokens) {
        return Ok(vec![LineAst::Statement { effects }]);
    }
    // Rewrites may replace the selected hand collection in the first
    // sentence with its later pronoun before the token-count followup is
    // grouped. Give the grammar-proven source sequence first refusal so the
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
        .or_else(|| exact_destroy_no_regeneration_statement(parse_tokens))
        .or_else(|| exact_hidden_partition_permission_statement(parse_tokens))
        .or_else(|| exact_historical_target_return_statement(parse_tokens))
    {
        return Ok(vec![LineAst::Statement { effects }]);
    }
    // The second sentence's `in it` refers to the exact hand revealed by the
    // first. Parsing statement groups independently loses both that player
    // and the hand domain before the registered pair rule can bind them.
    if let Some(effects) = exact_revealed_hand_union_count_statement(parse_tokens) {
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
        crate::runtime_backend::families::keyword_static::parse_carried_attached_subject_line(
            parse_tokens,
        )?
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
        return Ok(chunks);
    }
    if !parse_tokens.is_empty() {
        let statement_grouping =
            crate::runtime_backend::grammar::statement_grouping::parse_statement_grouping_tokens(
                parse_tokens,
            );
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
            return Ok(chunks);
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
            return Ok(chunks);
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
    let Some(shape) =
        semantic_grammar::parse_villainous_choice_statement_tokens(&line.parse_tokens)
    else {
        return Ok(None);
    };
    let target_tag = TagKey::from(IT_TAG);
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
            PlayerFilter::ControllerOf(crate::target::ObjectRef::tagged(IT_TAG))
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
    let normalized = rendered
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if normalized.starts_with("once each turn, you may pay ")
        && normalized.ends_with(" to reroll one or more dice you rolled.")
    {
        let pay_idx = tokens.iter().position(|token| {
            token
                .as_word()
                .is_some_and(|word| word.eq_ignore_ascii_case("pay"))
        })?;
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
        crate::runtime_backend::sentences::effect_sentences::parse_token_copy_followup_sentence_lexed(
            sentence.as_ref(),
        )
        .is_some()
    })
}

fn sentences_have_token_granted_ability_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        matches!(
            crate::runtime_backend::sentences::effect_sentences::parse_token_granted_ability_followup_sentence_lexed(sentence.as_ref()),
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
                || effect_grammar::parse_create_head_tokens(strip_leading_if_you_do_lexed(sentence))
                    .is_some()
        })
}

fn sentence_has_typed_become_copy_exception(tokens: &[OwnedLexToken]) -> bool {
    let Some(become_idx) = tokens
        .iter()
        .position(|token| token.is_word("become") || token.is_word("becomes"))
    else {
        return false;
    };
    let shape = effect_grammar::become_shapes::parse_become_rest_shape(&tokens[become_idx..]);
    shape.copy_exception.is_some()
        && token_word_refs(&shape.body_tokens)
            .windows(2)
            .any(|window| window == ["copy", "of"])
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
        && optional_words[..7] == ["you", "may", "exile", "it", "and", "collect", "evidence"]
        && crate::runtime_backend::front_end::shared::util::parse_number_word_u32(optional_words[7])
            .is_some()
        && followup_words.len() == 10
        && followup_words[0].eq_ignore_ascii_case("if")
        && followup_words[1..]
            == [
                "you",
                "do",
                "return",
                "this",
                "card",
                "to",
                "the",
                "battlefield",
                "tapped",
            ]
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
    if semantic_grammar::parse_returned_object_move_head_tokens(first_sentence.as_ref()).is_none() {
        return None;
    }

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
    filter == &ObjectFilter::tagged(TagKey::from(IT_TAG))
}

fn push_returned_object_keyword_grant_effect(
    effects: &mut Vec<EffectAst>,
    action: KeywordAction,
    condition: Option<crate::ConditionExpr>,
) {
    let target = TargetAst::Tagged(TagKey::from(IT_TAG), None);
    let ability = GrantedAbilityAst::KeywordAction(action);
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
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
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
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
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
    let instead_semantics = crate::runtime_backend::front_end::grammar::effects::classify_instead_followup_semantics_tokens(
        sentence,
    );
    if instead_semantics != crate::cards::builders::InsteadSemantics::SelfReplacement {
        return false;
    }
    if token_word_refs(sentence)
        .windows(2)
        .any(|words| words == ["instead", "if"])
    {
        return true;
    }
    if crate::runtime_backend::front_end::grammar::line_semantic_facts::parse_line_semantic_facts_tokens(
        sentence,
    )
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

fn sentence_is_linked_anaphoric_conditional_effect(sentence: &[OwnedLexToken]) -> bool {
    sentence_is_conditional_self_replacement_effect(sentence)
        || crate::parse_loss::capture(|| parse_effect_sentences_lexed(sentence))
            .0
            .is_ok_and(|effects| {
                matches!(
                    effects.as_slice(),
                    [EffectAst::Conditional { predicate, .. }]
                        if predicate.uses_implicit_object_reference()
                )
            })
}

fn linked_statement_should_stay_grouped(tokens: &[OwnedLexToken]) -> bool {
    let sentences = split_lexed_sentences(tokens);
    if let [default_copy, replacement_copy] = sentences.as_slice()
        && tokens.iter().any(|token| token.kind == TokenKind::Quote)
    {
        let default_words = token_word_refs(default_copy);
        let replacement_words = token_word_refs(replacement_copy);
        let is_copy_creation = |words: &[&str]| {
            words.windows(2).any(|window| window == ["create", "a"])
                && words.windows(2).any(|window| window == ["token", "that"])
                && words.windows(2).any(|window| window == ["copy", "of"])
        };
        if is_copy_creation(&default_words)
            && is_copy_creation(&replacement_words)
            && replacement_words.contains(&"instead")
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
        && crate::runtime_backend::front_end::grammar::effects::is_filtered_future_exile_return_next_end_step_shape(
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
            || crate::parse_loss::capture(|| parse_effect_sentences_lexed(sentence))
                .0
                .is_ok_and(|effects| {
                    matches!(
                        effects.as_slice(),
                        [EffectAst::Conditional { predicate, .. }]
                            if predicate.uses_implicit_object_reference()
                    )
                })
    }) {
        return true;
    }

    // A typed statement-replacement surface can depend on both authored
    // sentences (for example Whirlpool Whelm's clash result and its following
    // "instead" sentence). Keep the program intact through semantic lowering.
    crate::runtime_backend::front_end::grammar::lowering_surfaces::parse_statement_replacement_surface_tokens(
        tokens,
    )
    .is_some()
        || semantic_grammar::parse_linked_statement_surface_tokens(tokens).is_some()
}

fn parse_complete_self_replacement_statement(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    if split_lexed_sentences(tokens).len() < 3 {
        return None;
    }
    let effects = crate::parse_loss::capture(|| parse_effect_sentences_lexed(tokens))
        .0
        .ok()?;
    matches!(effects.as_slice(), [EffectAst::SelfReplacement { .. }]).then_some(effects)
}

#[cfg(test)]
#[test]
fn typed_statement_replacement_surface_stays_grouped() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Clash with an opponent, then return target creature to its owner's hand. If you win, you may put that creature on top of its owner's library instead.",
        0,
    )
    .expect("lex Whirlpool Whelm");

    assert!(linked_statement_should_stay_grouped(&tokens));
}

#[cfg(test)]
#[test]
fn trailing_conditional_self_replacement_stays_grouped() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Target creature an opponent controls gets -1/-1 until end of turn. That creature gets -4/-4 instead if you control a creature named Bogbrew Witch.",
        0,
    )
    .expect("lex conditional pump replacement");

    assert!(linked_statement_should_stay_grouped(&tokens));
}

#[cfg(test)]
#[test]
fn self_replacement_with_a_common_resolution_tail_stays_grouped() {
    let text = "Choose target creature with mana value 3 or less. If this spell was kicked, instead choose target creature. Exile the chosen creature, then its controller gains life equal to its mana value.";
    let tokens = crate::runtime_backend::lexer::lex_line(text, 0)
        .expect("lex choice replacement with common tail");
    assert!(linked_statement_should_stay_grouped(&tokens));
    let groups = split_lexed_sentences(&tokens)
        .into_iter()
        .map(|group| group.to_vec())
        .collect::<Vec<_>>();
    let parsed = parse_statement_token_groups_to_chunks(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: text.to_string(),
            source_tokens: tokens.clone(),
            normalized: NormalizedLine {
                original: text.to_string(),
                normalized: text.to_string(),
                char_map: Vec::new(),
            },
            semantic_facts: Default::default(),
        },
        &tokens,
        &groups,
    )
    .expect("public statement lowering should preserve the typed program");
    let [LineAst::Statement { effects }] = parsed.as_slice() else {
        panic!("expected one statement program: {parsed:#?}");
    };
    assert!(
        matches!(effects.as_slice(), [EffectAst::SelfReplacement { .. }]),
        "{effects:#?}"
    );
    assert!(format!("{effects:#?}").contains("GainLife"), "{effects:#?}");

    let parsed_without_precomputed_groups = parse_statement_token_groups_to_chunks(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: text.to_string(),
            source_tokens: tokens.clone(),
            normalized: NormalizedLine {
                original: text.to_string(),
                normalized: text.to_string(),
                char_map: Vec::new(),
            },
            semantic_facts: Default::default(),
        },
        &tokens,
        &[],
    )
    .expect("ungrouped public statement lowering should preserve the typed program");
    let [LineAst::Statement { effects }] = parsed_without_precomputed_groups.as_slice() else {
        panic!("expected one ungrouped statement program: {parsed_without_precomputed_groups:#?}");
    };
    assert!(
        matches!(effects.as_slice(), [EffectAst::SelfReplacement { .. }])
            && format!("{effects:#?}").contains("GainLife"),
        "{effects:#?}"
    );

    let unrelated = crate::runtime_backend::lexer::lex_line(
        "Choose target creature with mana value 3 or less. If this spell was kicked, choose target creature. Exile the chosen creature.",
        0,
    )
    .expect("lex nonreplacement near miss");
    assert!(!linked_statement_should_stay_grouped(&unrelated));
}

#[cfg(test)]
fn parse_public_statement_groups_for_test(text: &str) -> Vec<LineAst> {
    let tokens = crate::runtime_backend::lexer::lex_line(text, 0).expect("statement should lex");
    let groups = split_lexed_sentences(&tokens)
        .into_iter()
        .map(|group| group.to_vec())
        .collect::<Vec<_>>();
    parse_statement_token_groups_to_chunks(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: text.to_string(),
            source_tokens: tokens.clone(),
            normalized: NormalizedLine {
                original: text.to_string(),
                normalized: text.to_string(),
                char_map: Vec::new(),
            },
            semantic_facts: Default::default(),
        },
        &tokens,
        &groups,
    )
    .expect("public statement route should parse")
}

#[cfg(test)]
#[test]
fn destroy_no_regeneration_pair_preempts_statement_group_splitting() {
    let text = "Destroy target creature that isn't enchanted. It can't be regenerated.";
    let parsed = parse_public_statement_groups_for_test(text);
    let [LineAst::Statement { effects }] = parsed.as_slice() else {
        panic!("expected one linked statement: {parsed:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Destroy {
                    target: TargetAst::Object(filter, _, _),
                    no_regeneration: true,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one typed no-regeneration destroy: {effects:#?}");
    };
    let aura = filter
        .without_attached_object
        .as_deref()
        .expect("negative enchanted state should survive statement grouping");
    assert_eq!(aura.subtypes, [crate::types::Subtype::Aura]);
}

#[cfg(test)]
#[test]
fn hidden_partition_permission_preempts_statement_group_splitting() {
    let text = "Look at the top three cards of your library. Exile one face down and put the rest on the bottom of your library in any order. For as long as it remains exiled, you may cast it if it's a creature spell.";
    let parsed = parse_public_statement_groups_for_test(text);
    let [LineAst::Statement { effects }] = parsed.as_slice() else {
        panic!("expected one linked statement: {parsed:#?}");
    };
    let debug = format!("{effects:#?}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(debug.contains("face_down: true"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnLibraryBottom"),
        "{debug}"
    );
    assert!(
        debug.contains("GrantPlayTaggedForAsLongAsExiled"),
        "{debug}"
    );
    assert!(debug.contains("Creature"), "{debug}");
}

#[cfg(test)]
#[test]
fn historical_target_return_preempts_statement_group_splitting() {
    let text = "Choose up to three target permanent cards in graveyards that were put there from the battlefield this turn. Return them to the battlefield tapped under their owners' control. You draw a card for each opponent who controls one or more of those permanents.";
    let parsed = parse_public_statement_groups_for_test(text);
    let [LineAst::Statement { effects }] = parsed.as_slice() else {
        panic!("expected one linked statement: {parsed:#?}");
    };
    let debug = format!("{effects:#?}");
    assert!(
        debug.contains("entered_graveyard_from_battlefield_this_turn: true"),
        "{debug}"
    );
    assert!(debug.contains("ReturnToBattlefield"), "{debug}");
    assert!(debug.contains("PlayerControls"), "{debug}");
}

#[cfg(test)]
#[test]
fn quoted_token_copy_replacement_stays_grouped_with_its_granted_ability() {
    let text = "Create a token that's a copy of target permanent. If {R}{G} was spent to cast this spell, instead create a token that's a copy of that permanent, except the token has \"When this token enters, if it's a creature, it fights up to one target creature you don't control.\"";
    let tokens = crate::runtime_backend::lexer::lex_line(text, 0)
        .expect("lex quoted token-copy replacement");
    assert!(linked_statement_should_stay_grouped(&tokens));
    let mut direct = parse_effect_sentences_lexed(&tokens).expect("parse direct replacement");
    assert!(
        crate::runtime_backend::effect_sentences::attach_inline_token_granted_abilities_to_last_create(
            &mut direct,
            &tokens,
        ),
        "direct attachment failed: {direct:#?}"
    );
    let effects = parse_effect_sentences_preserving_source_boundaries(&tokens)
        .expect("parse grouped token-copy replacement");
    let [EffectAst::SelfReplacement { if_true, .. }] = effects.as_slice() else {
        panic!("expected one self-replacement: {effects:#?}");
    };
    let [EffectAst::SubjectVerb(subject_verb)] = if_true.as_slice() else {
        panic!("expected one replacement copy: {if_true:#?}");
    };
    let SubjectVerbActionAst::CreateTokenCopyFromSource {
        granted_abilities, ..
    } = &subject_verb.action
    else {
        panic!("expected a source-relative token copy: {subject_verb:#?}");
    };
    assert_eq!(granted_abilities.len(), 1, "{subject_verb:#?}");
}

#[cfg(test)]
#[test]
fn revealed_hand_union_count_stays_linked_through_the_public_statement_route() {
    let text =
        "Target opponent reveals their hand. You draw a card for each Forest and green card in it.";
    let tokens = crate::runtime_backend::lexer::lex_line(text, 0)
        .expect("revealed-hand union statement should lex");
    let groups = split_lexed_sentences(&tokens)
        .into_iter()
        .map(|group| group.to_vec())
        .collect::<Vec<_>>();
    let parsed = parse_statement_token_groups_to_chunks(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: text.to_string(),
            source_tokens: tokens.clone(),
            normalized: NormalizedLine {
                original: text.to_string(),
                normalized: text.to_string(),
                char_map: Vec::new(),
            },
            semantic_facts: Default::default(),
        },
        &tokens,
        &groups,
    )
    .expect("public statement route should preserve the revealed-hand pair");
    let [LineAst::Statement { effects }] = parsed.as_slice() else {
        panic!("expected one statement program: {parsed:#?}");
    };
    let [
        _,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { count },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected reveal plus typed draw: {effects:#?}");
    };
    let Value::Count(filter) = count.unhinted() else {
        panic!("expected a revealed-hand object count: {count:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert_eq!(
        filter.owner,
        Some(PlayerFilter::AliasedTarget(Box::new(
            PlayerFilter::Opponent
        )))
    );
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
}

#[cfg(test)]
#[test]
fn selected_hand_reveal_token_creation_uses_the_unabridged_source_program() {
    let text = "Each player may reveal any number of creature cards from their hand. Then each player creates a 2/2 green Bear creature token for each card they revealed this way.";
    let tokens = crate::runtime_backend::lexer::lex_line(text, 0)
        .expect("selected hand reveal statement should lex");
    let effects = typed_selected_hand_reveal_token_creation_statement(&tokens)
        .expect("typed source sequence should be recognized");
    let debug = format!("{effects:#?}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("RevealTagged"), "{debug}");
    assert!(debug.contains("CardsRevealedThisWay"), "{debug}");
}

#[cfg(test)]
#[test]
fn whole_hand_reveal_does_not_match_selected_hand_sequence() {
    let text = "Each player may reveal their hand. Then each player creates a 1/1 green Saproling creature token.";
    let tokens = crate::runtime_backend::lexer::lex_line(text, 0)
        .expect("whole hand reveal near miss should lex");
    assert!(typed_selected_hand_reveal_token_creation_statement(&tokens).is_none());
}

fn statement_group_should_parse_as_effects_first(tokens: &[OwnedLexToken]) -> bool {
    if matches!(
        crate::runtime_backend::families::keyword_static::parse_double_counters_replacement_line(
            tokens,
        ),
        Ok(Some(_))
    ) {
        return false;
    }
    if linked_statement_should_stay_grouped(tokens) {
        return true;
    }
    if crate::runtime_backend::front_end::grammar::effects::parse_persistent_no_maximum_hand_size_player_lexed(
        tokens,
    )
    .is_some()
    {
        return true;
    }
    if matches!(
        classify_statement_line_family_lexed(tokens),
        Some(StatementLineFamily::Vote)
    ) {
        return true;
    }

    if crate::runtime_backend::front_end::grammar::effects::clause_pattern_shapes::parse_keyword_mechanic_tokens(tokens)
        .is_some()
    {
        return true;
    }

    semantic_grammar::parse_statement_effect_preference_tokens(tokens).is_some()
}

fn parse_self_enters_with_x_counters_static_chunk(tokens: &[OwnedLexToken]) -> Option<LineAst> {
    match semantic_grammar::parse_self_counter_entry_tokens(tokens)? {
        semantic_grammar::SelfCounterEntrySpec::Adamant {
            condition,
            predicate_body,
        } => Some(LineAst::StaticAbilities(vec![
            crate::cards::builders::StaticAbilityAst::Static(
                StaticAbility::enters_with_counters_if_condition(
                    crate::object::CounterType::PlusOnePlusOne,
                    crate::effect::Value::Fixed(1),
                    condition,
                    predicate_body,
                ),
            ),
        ])),
        semantic_grammar::SelfCounterEntrySpec::Unconditional { count } => {
            Some(LineAst::StaticAbilities(vec![
                crate::cards::builders::StaticAbilityAst::Static(
                    StaticAbility::enters_with_counters_value(
                        crate::object::CounterType::PlusOnePlusOne,
                        count,
                    ),
                ),
            ]))
        }
    }
}

fn spell_cast_trigger_filter(trigger: &TriggerSpec) -> Option<(ObjectFilter, PlayerFilter)> {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => spell_cast_trigger_filter(trigger),
        TriggerSpec::SpellCast {
            filter: Some(filter),
            mana_source_filter: None,
            caster,
            timing: None,
            during_turn: None,
            min_spells_this_turn: None,
            exact_spells_this_turn: None,
            from_not_hand: false,
        } => Some((filter.clone(), caster.clone())),
        _ => None,
    }
}

fn lower_spell_cast_snow_mana_enter_counter_static_chunk(
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
    intervening_if: Option<&PredicateAst>,
) -> Result<Option<LineAst>, CardTextError> {
    let Some(spec) = semantic_grammar::parse_snow_mana_counter_entry_tokens(
        effect_parse_tokens,
        matches!(
            intervening_if,
            Some(PredicateAst::SnowManaOfAnySpellColorSpentToCastThisSpell)
        ),
    ) else {
        return Ok(None);
    };

    let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
    let Some((mut filter, caster)) = spell_cast_trigger_filter(&trigger) else {
        return Ok(None);
    };
    if !matches!(filter.zone, Some(Zone::Stack))
        || filter.card_types.len() != 1
        || filter.card_types.first().copied() != Some(CardType::Creature)
    {
        return Ok(None);
    }

    filter.zone = Some(Zone::Battlefield);
    filter.stack_kind = None;
    filter.has_mana_cost = false;
    filter.controller = Some(caster);

    let ability = StaticAbility::enters_with_counters_and_subtypes_for_filter(
        filter,
        spec.counter_type,
        spec.count,
        Vec::new(),
    )
    .with_condition(spec.condition);

    Ok(Some(LineAst::StaticAbilities(vec![
        crate::cards::builders::StaticAbilityAst::Static(ability),
    ])))
}

fn parse_day_night_starts_day_static_chunk(tokens: &[OwnedLexToken]) -> Option<LineAst> {
    let rendered = render_token_slice(tokens);
    semantic_grammar::parse_day_night_starts_day_tokens(tokens).map(|_| {
        LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
            StaticAbility::rule_fallback_text(rendered.trim().trim_end_matches('.').to_string()),
        )])
    })
}

fn membership_predicate_for_iterated_object(tag: &str) -> PredicateAst {
    PredicateAst::TaggedMatches(
        TagKey::from(tag),
        ObjectFilter::default().same_stable_id_as_tagged(TagKey::from(IT_TAG)),
    )
}

#[cfg(test)]
pub(crate) fn parse_single_effect_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    parse_effect_sentences_lexed(tokens)?
        .into_iter()
        .next()
        .ok_or_else(|| CardTextError::ParseError("missing effect in lexed sentence".to_string()))
}

#[cfg(test)]
pub(crate) fn strip_lexed_suffix_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &[&str],
) -> Option<&'a [OwnedLexToken]> {
    let words = TokenWordView::new(tokens);
    if words.len() < phrase.len() {
        return None;
    }
    let start_word_idx = words.len() - phrase.len();
    if !words.slice_eq(start_word_idx, phrase) {
        return None;
    }
    let token_idx = words.token_boundary_for_word(start_word_idx)?;
    Some(&tokens[..token_idx])
}

pub(crate) fn parse_triggered_line(
    info: LineInfo,
    full_text: &str,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
    intervening_if: Option<PredicateAst>,
    presentation: Option<&PresentationLabel>,
    max_triggers_per_turn: Option<u32>,
    chosen_option: Option<&ChosenOptionContext>,
) -> Result<LineAst, CardTextError> {
    parse_triggered_line_impl(
        &RewriteTriggeredLine {
            info,
            full_text: full_text.to_string(),
            full_parse_tokens: full_parse_tokens.to_vec(),
            intervening_if,
            max_triggers_per_turn,
            chosen_option: chosen_option.cloned(),
            presentation: presentation.cloned(),
        },
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )
}

fn parse_triggered_line_impl(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    use crate::runtime_backend::grammar::effects::delayed_sentence_shapes::{
        DelayedScheduleStep, parse_delayed_schedule_sentence_shape,
    };

    let delayed_schedule = parse_delayed_schedule_sentence_shape(full_parse_tokens);
    let nested_combat_payment = (|| {
        let (_, after_intro) = crate::runtime_backend::grammar::primitives::parse_prefix(
            full_parse_tokens,
            crate::runtime_backend::grammar::primitives::phrase(&[
                "at",
                "the",
                "beginning",
                "of",
                "each",
                "combat",
            ]),
        )?;
        let after_intro = trim_lexed_commas(after_intro);
        let (_, after_pay) = crate::runtime_backend::grammar::primitives::parse_prefix(
            after_intro,
            crate::runtime_backend::grammar::primitives::phrase(&["unless", "you", "pay"]),
        )?;
        let (cost_tokens, nested_trigger_tokens) =
            crate::runtime_backend::grammar::primitives::split_lexed_once_on_comma(after_pay)?;
        if !nested_trigger_tokens
            .first()
            .is_some_and(|token| token.is_word("whenever"))
        {
            return None;
        }
        let cost = crate::runtime_backend::grammar::leaf::parse_leaf_mana_cost_tokens(
            trim_lexed_commas(cost_tokens),
        )
        .ok()?;
        Some(crate::cost::TotalCost::mana(cost))
    })();
    let mut parsed = parse_triggered_ability_line_impl(
        line,
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )?;
    transport_delayed_copy_retarget_in_line(&mut parsed);
    apply_source_spell_cast_trigger_spec(&mut parsed, line.info.source_tokens.as_slice());
    apply_protected_battle_iteration_surface(&mut parsed, line.info.source_tokens.as_slice());
    if let Some(source_filter) =
        spell_cast_mana_source_filter(trigger_parse_tokens, line.info.source_tokens.as_slice())?
    {
        apply_spell_cast_mana_source_filter(&mut parsed, &source_filter);
    }
    if let Some(source_surface) = spell_cast_single_target_source_exclusion_surface(
        trigger_parse_tokens,
        line.info.source_tokens.as_slice(),
    ) {
        apply_spell_cast_single_target_source_exclusion(&mut parsed, &source_surface);
    }
    // The graveyard-card copy/cast sequence owns executable semantics across
    // its first two sentences. Re-splitting those sentences for presentation
    // would turn the exiled card into an invalid stack CopySpellEffect. Keep
    // the semantic parse intact, including any later `If you do` follow-up.
    let mut parsed =
        if let Some(effects) = exact_graveyard_card_copy_cast_sequence(effect_parse_tokens) {
            match &mut parsed {
                LineAst::Triggered {
                    effects: parsed_effects,
                    ..
                } => *parsed_effects = effects,
                LineAst::Ability(ability) if ability.effects_ast.is_some() => {
                    ability.effects_ast = Some(effects)
                }
                _ => {}
            }
            parsed
        } else if starts_with_exact_graveyard_card_copy_cast_sequence(effect_parse_tokens) {
            parsed
        } else {
            preserve_triggered_effect_surfaces(parsed, effect_parse_tokens, full_parse_tokens)
        };
    // Surface preservation reparses the effect body and can replace the raw
    // effect vector. Reapply the idempotent typed transport so neither public
    // trigger route can leave a copied-object retarget on the outer program.
    reconcile_named_explore_source_surface(
        &mut parsed,
        effect_parse_tokens,
        line.info.raw_line.as_str(),
    )?;
    // The generic surface-preservation pass above reparses triggered bodies
    // sentence-by-sentence. For an authored dynamic death-group token this
    // can collapse the already-typed aggregate P/T back to the token
    // definition's 0/0. Reconcile from the intact source only after that
    // lossy pass, retaining the exact TotalPower + zone-change-group proof.
    let authored_source_tokens =
        crate::runtime_backend::lexer::lex_line(line.info.raw_line.as_str(), line.info.line_index)
            .unwrap_or_else(|_| line.info.source_tokens.clone());
    reconcile_authored_correlated_trigger_programs(&mut parsed, &authored_source_tokens)?;
    reconcile_dynamic_zone_change_group_token_creation(&mut parsed, &authored_source_tokens)?;
    reconcile_open_attraction_reminder(&mut parsed, line.info.raw_line.as_str());
    transport_delayed_copy_retarget_in_line(&mut parsed);
    apply_source_spell_cast_trigger_spec(&mut parsed, line.info.source_tokens.as_slice());
    apply_protected_battle_iteration_surface(&mut parsed, line.info.source_tokens.as_slice());
    // Source-trigger restoration above is intentionally broad for ordinary
    // spell-cast triggers. Reapply the stricter coordinated spell-or-ability
    // proof last so it cannot be simplified back to only its spell arm.
    reconcile_authored_correlated_trigger_programs(&mut parsed, &authored_source_tokens)?;
    if let Some(cost) = nested_combat_payment {
        let (trigger, effects, nested_cap) = match parsed {
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => (trigger, effects, max_triggers_per_turn),
            LineAst::Ability(parsed)
                if matches!(parsed.kind(), crate::ability::AbilityKind::Triggered(_)) =>
            {
                let trigger = parsed.trigger_spec.ok_or_else(|| {
                    CardTextError::InvariantViolation(format!(
                        "nested beginning-of-combat payment lost its trigger spec: '{}'",
                        line.info.raw_line
                    ))
                })?;
                let effects = parsed.effects_ast.ok_or_else(|| {
                    CardTextError::InvariantViolation(format!(
                        "nested beginning-of-combat payment lost its effect AST: '{}'",
                        line.info.raw_line
                    ))
                })?;
                (trigger, effects, None)
            }
            _ => {
                return Err(CardTextError::InvariantViolation(format!(
                    "nested beginning-of-combat payment did not preserve its typed trigger: '{}'",
                    line.info.raw_line
                )));
            }
        };
        if nested_cap.is_some() {
            return Err(CardTextError::ParseError(format!(
                "nested beginning-of-combat payment trigger cannot carry a frequency cap: '{}'",
                line.info.raw_line
            )));
        }
        return Ok(LineAst::Triggered {
            trigger: TriggerSpec::BeginningOfCombat(PlayerFilter::Any),
            effects: vec![EffectAst::UnlessPays {
                effects: vec![EffectAst::DelayedTriggerForDuration {
                    trigger,
                    effects,
                    one_shot: false,
                    duration: Until::EndOfCombat,
                    either_of_watched_objects: false,
                    while_any_tagged_object_in_zone: None,
                }],
                player: PlayerAst::You,
                cost,
                before_delayed_step: false,
            }],
            max_triggers_per_turn: None,
        });
    }
    let Some(schedule) = delayed_schedule else {
        return Ok(parsed);
    };
    let effects = match parsed {
        LineAst::Triggered { effects, .. } => effects,
        LineAst::Ability(parsed)
            if matches!(parsed.kind(), crate::ability::AbilityKind::Triggered(_)) =>
        {
            parsed.effects_ast.ok_or_else(|| {
                CardTextError::InvariantViolation(format!(
                    "delayed schedule ability did not preserve semantic effects: '{}'",
                    line.info.raw_line
                ))
            })?
        }
        _ => {
            return Err(CardTextError::InvariantViolation(format!(
                "delayed schedule sentence did not produce triggered effects: '{}'",
                line.info.raw_line
            )));
        }
    };

    let delayed = match schedule.step {
        DelayedScheduleStep::UntapStep => EffectAst::DelayedUntilNextUntapStep {
            player: schedule.player,
            effects,
        },
        DelayedScheduleStep::Upkeep => EffectAst::DelayedUntilNextUpkeep {
            player: schedule.player,
            effects,
        },
        DelayedScheduleStep::DrawStep => EffectAst::DelayedUntilNextDrawStep {
            player: schedule.player,
            effects,
        },
        DelayedScheduleStep::MainPhase => EffectAst::DelayedUntilNextMainPhase {
            player: match schedule.player {
                PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                PlayerAst::That => PlayerFilter::IteratedPlayer,
                PlayerAst::Target => PlayerFilter::target_player(),
                PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                _ => PlayerFilter::Any,
            },
            effects,
        },
        DelayedScheduleStep::FirstMainPhase => EffectAst::DelayedUntilNextFirstMainPhase {
            player: match schedule.player {
                PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                PlayerAst::That => PlayerFilter::IteratedPlayer,
                PlayerAst::Target => PlayerFilter::target_player(),
                PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                _ => PlayerFilter::Any,
            },
            effects,
        },
        DelayedScheduleStep::EndStep if schedule.start_next_turn => {
            EffectAst::DelayedUntilEndStepOfExtraTurn {
                player: schedule.player,
                effects,
            }
        }
        DelayedScheduleStep::EndStep => EffectAst::DelayedUntilNextEndStep {
            player: match schedule.player {
                PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                PlayerAst::That => PlayerFilter::IteratedPlayer,
                PlayerAst::Target => PlayerFilter::target_player(),
                PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                _ => PlayerFilter::Any,
            },
            effects,
        },
    };
    Ok(LineAst::Statement {
        effects: vec![delayed],
    })
}

fn transport_delayed_copy_retarget_in_line(parsed: &mut LineAst) {
    match parsed {
        LineAst::Triggered { effects, .. } => {
            crate::runtime_backend::effect_sentences::transport_copy_retarget_into_trailing_delayed_trigger(
                effects,
            );
        }
        LineAst::Ability(parsed) => {
            if let Some(effects) = parsed.effects_ast.as_mut() {
                crate::runtime_backend::effect_sentences::transport_copy_retarget_into_trailing_delayed_trigger(
                    effects,
                );
            }
        }
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                transport_delayed_copy_retarget_in_line(chunk);
            }
        }
        _ => {}
    }
}

fn starts_with_exact_graveyard_card_copy_cast_sequence(
    effect_parse_tokens: &[OwnedLexToken],
) -> bool {
    let sentences = split_lexed_sentences(effect_parse_tokens)
        .into_iter()
        .map(crate::runtime_backend::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    let Ok(Some(matched)) =
        crate::runtime_backend::effect_sentences::try_parse_subject_verb_sequence_rule(
            &sentences, 0,
        )
    else {
        return false;
    };
    matched.feature_tag == Some("graveyard-card-copy-cast")
        && matched.consumed_sentences <= sentences.len()
}

pub(crate) fn exact_graveyard_card_copy_cast_sequence(
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(effect_parse_tokens)
        .into_iter()
        .map(crate::runtime_backend::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    let Ok(Some(matched)) =
        crate::runtime_backend::effect_sentences::try_parse_subject_verb_sequence_rule(
            &sentences, 0,
        )
    else {
        return None;
    };
    if !matches!(
        matched.feature_tag,
        Some("graveyard-card-copy-cast" | "conditional-graveyard-card-copy-cast")
    ) {
        return None;
    }
    let trailing = &sentences[matched.consumed_sentences..];
    let has_standard_copy_cast_reminder = matches!(
        trailing,
        [costs, permanent_copy]
            if matches!(
                crate::runtime_backend::lexer::parser_token_word_refs(costs.lowered()).as_slice(),
                ["you", "still", "pay", "its", "costs"]
            ) && matches!(
                crate::runtime_backend::lexer::parser_token_word_refs(permanent_copy.lowered()).as_slice(),
                ["a", "copy", "of", "a", "permanent", "spell", "becomes", "a", "token"]
            )
    );
    let trailing_cast_result = match trailing {
        [] => None,
        [sentence] => {
            let Ok(effects) = crate::runtime_backend::effect_sentences::parse_effect_sentence_lexed(
                sentence.lowered(),
            ) else {
                return None;
            };
            let [
                effect @ EffectAst::IfResult {
                    predicate: crate::cards::builders::IfResultPredicate::Did,
                    effects: result_effects,
                },
            ] = effects.as_slice()
            else {
                return None;
            };
            if result_effects.is_empty() {
                return None;
            }
            Some(effect.clone())
        }
        [_, _] if has_standard_copy_cast_reminder => None,
        _ => return None,
    };

    let mut effects = matched.effects;
    if has_standard_copy_cast_reminder {
        fn mark_cast_copy(effects: &mut [EffectAst]) {
            for effect in effects {
                if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CastTagged {
                            as_copy: true,
                            copy_cast_reminder_surface,
                            ..
                        },
                    ..
                }) = effect
                {
                    *copy_cast_reminder_surface = true;
                }
                crate::runtime_backend::model::effect_ast_traversal::for_each_nested_effects_mut(
                    effect,
                    true,
                    mark_cast_copy,
                );
            }
        }
        mark_cast_copy(&mut effects);
    }
    if let Some(trailing_cast_result) = trailing_cast_result {
        effects.push(trailing_cast_result);
    }
    Some(effects)
}

fn exact_dynamic_exile_permission_bundle(
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    // Do not call the aggregate typed-bundle dispatcher from this CST proof.
    // The public triggered-line router asks this predicate before choosing a
    // split candidate, while that dispatcher can in turn enter the same
    // public routing path.  Parse only the reusable two-sentence rule whose
    // shape this guard is proving.
    let sentences = split_lexed_sentences(effect_parse_tokens)
        .into_iter()
        .map(crate::runtime_backend::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    if sentences.len() != 2 {
        return None;
    }
    let effects = crate::runtime_backend::effect_sentences::parse_dynamic_exile_top_then_play_for_as_long_as_exiled(
        &sentences,
        0,
    )
    .ok()??;
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: PlayerAst::ItsOwner,
                    ..
                },
            action:
                SubjectVerbActionAst::ExileTopOfLibrary {
                    count,
                    tags,
                    face_down: false,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    tag,
                    allow_land: false,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        return None;
    };
    if tags != std::slice::from_ref(tag)
        || !matches!(
            count.unhinted(),
            Value::PowerOf(spec)
                if matches!(spec.as_ref(), ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")
        )
    {
        return None;
    }
    Some(effects)
}

pub(crate) fn exact_looked_hand_optional_cast_bundle(
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(effect_parse_tokens)
        .into_iter()
        .map(crate::runtime_backend::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    if sentences.len() != 2 {
        return None;
    }
    let effects = crate::runtime_backend::effect_sentences::parse_look_at_players_hand_then_may_cast_from_those_cards(
        &sentences,
        0,
    )
    .ok()??;
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::LookAtHand {
                    target:
                        TargetAst::Player(
                            PlayerFilter::DamagedPlayer
                            | PlayerFilter::IteratedPlayer
                            | PlayerFilter::Target(_)
                            | PlayerFilter::AliasedTarget(_),
                            _,
                        ),
                },
            ..
        }),
        EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
            player: PlayerAst::You,
            zone_owner: PlayerAst::That,
            filter,
            zone: Zone::Hand,
            payment: ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost,
        },
    ] = effects.as_slice()
    else {
        return None;
    };
    if filter != &ObjectFilter::nonland().in_zone(Zone::Hand) {
        return None;
    }
    Some(effects)
}

/// Preserve a targeted graveyard card and the optional normal-cost cast in a
/// single typed trigger body. The broad grant-ability sentence parser can
/// otherwise read `you may cast target card ...` as an ability granted to the
/// triggering spell, losing both targeting and execution.
pub(crate) fn exact_target_same_name_graveyard_may_cast_bundle(
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(effect_parse_tokens);
    const BODY: &[&str] = &[
        "you",
        "may",
        "cast",
        "target",
        "card",
        "with",
        "the",
        "same",
        "name",
        "as",
        "that",
        "spell",
        "from",
        "your",
        "graveyard",
    ];
    if !words.starts_with(BODY)
        || !matches!(
            &words[BODY.len()..],
            [] | ["you", "still", "pay", "its", "costs"]
        )
    {
        return None;
    }

    let target_tag = crate::runtime_backend::util::helper_tag_for_tokens(
        effect_parse_tokens,
        "targeted_same_name_spell",
    );
    let mut filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    filter
        .tagged_constraints
        .push(crate::target::TaggedObjectConstraint {
            tag: TagKey::from("triggering"),
            relation: crate::target::TaggedOpbjectRelation::SameNameAsTagged,
        });
    filter.set_same_name_antecedent_surface(Some(ironsmith_core::SameNameAntecedentSurface::Spell));
    let target = TargetAst::Object(filter, Some(TextSpan::synthetic()), None);
    Some(vec![
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_explicit_target_only(target)),
            tag: target_tag.clone(),
        },
        EffectAst::May {
            effects: vec![EffectAst::subject_verb_cast_tagged(
                target_tag,
                PlayerAst::You,
                false,
                false,
                false,
                None,
            )],
        },
    ])
}

fn exact_atomic_return_as_aura_bundle(
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(effect_parse_tokens);
    let [return_sentence, aura_sentence] = sentences.as_slice() else {
        return None;
    };
    let mut effects =
        crate::runtime_backend::effect_sentences::parse_effect_sentence_lexed(return_sentence)
            .ok()?;
    // The ordinary complete-sentence dispatcher may claim the trailing
    // outside-quote ability loss before the preceding Aura animation. Split
    // only the exact authored conjunction after the balanced quoted grant,
    // then feed both typed leaves to the normal AST fusion pass.
    let mut in_quote = false;
    let loss_start = aura_sentence.iter().enumerate().find_map(|(idx, token)| {
        if token.kind == TokenKind::Quote {
            in_quote = !in_quote;
            return None;
        }
        (!in_quote
            && token.is_word("and")
            && matches!(
                token_word_refs(&aura_sentence[idx + 1..]).as_slice(),
                ["it", "loses", "all", "other", "abilities"]
            ))
        .then_some(idx)
    })?;
    let aura_prefix = trim_lexed_commas(&aura_sentence[..loss_start]);
    let loss_suffix = trim_lexed_commas(&aura_sentence[loss_start + 1..]);
    let quote_positions = aura_prefix
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| (token.kind == TokenKind::Quote).then_some(idx))
        .collect::<Vec<_>>();
    let [open_quote, close_quote] = quote_positions.as_slice() else {
        return None;
    };
    let quoted_ability_tokens = &aura_prefix[*open_quote + 1..*close_quote];
    let quoted_words = token_word_refs(quoted_ability_tokens);
    let granted_ability = crate::runtime_backend::effect_sentences::parse_granted_activated_or_triggered_ability_for_gain(
        quoted_ability_tokens,
        &quoted_words,
    )
    .ok()??;
    // Parse the Aura animation without the quoted rule, then put the rule on
    // the typed Aura payload. This avoids letting the colon inside the quoted
    // activation turn the entire authored sentence into an activated line.
    let mut aura_base = aura_prefix[..*open_quote].to_vec();
    while aura_base
        .last()
        .is_some_and(|token| token.kind == TokenKind::Comma || token.is_word("and"))
    {
        aura_base.pop();
    }
    let mut aura_effects =
        crate::runtime_backend::effect_sentences::parse_effect_sentence_lexed(&aura_base).ok()?;
    let [EffectAst::SubjectVerb(aura_subject_verb)] = aura_effects.as_mut_slice() else {
        return None;
    };
    let SubjectVerbActionAst::BecomeAuraEnchantment {
        granted_abilities, ..
    } = &mut aura_subject_verb.action
    else {
        return None;
    };
    if !granted_abilities.is_empty() {
        return None;
    }
    granted_abilities.push(granted_ability);
    let loss_effects =
        crate::runtime_backend::effect_sentences::parse_effect_sentence_lexed(&loss_suffix).ok()?;
    aura_effects.extend(loss_effects);
    match aura_effects.as_slice() {
        [
            EffectAst::Coordinated {
                effects: coordinated,
                leading_duration: false,
                result_conjunction: false,
            },
        ] => effects.extend(coordinated.iter().cloned()),
        _ => effects.extend(aura_effects),
    }
    let effects = crate::runtime_backend::effect_ast_normalization::normalize_effects_ast(&effects);
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ReturnToBattlefield {
                    as_aura: Some(as_aura),
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        return None;
    };
    if !as_aura.remove_all_abilities || as_aura.granted_abilities.is_empty() {
        return None;
    }
    Some(effects)
}

/// Public CST routing must preserve the complete authored token stream for
/// these correlated multi-sentence trigger bodies. Broad split probes can
/// successfully parse each sentence while losing the value/object-set bridge,
/// so expose one exact predicate for the CST layer to claim them before those
/// probes run.
pub(crate) fn is_exact_correlated_trigger_effect_bundle(
    effect_parse_tokens: &[OwnedLexToken],
) -> bool {
    exact_dynamic_exile_permission_bundle(effect_parse_tokens).is_some()
        || exact_atomic_return_as_aura_bundle(effect_parse_tokens).is_some()
        || exact_looked_hand_optional_cast_bundle(effect_parse_tokens).is_some()
}

/// Lexical CST proof for the dynamic two-sentence exile permission. At this
/// boundary, contextual `its` references have not yet been bound to the
/// triggering object, so the stronger typed proof above is intentionally too
/// early. The semantic lowering pass still has to produce the exact typed
/// PowerOf/ItsOwner/shared-tag bundle before this route has any effect.
pub(crate) fn is_authored_dynamic_exile_permission_bundle(
    effect_parse_tokens: &[OwnedLexToken],
) -> bool {
    let sentences = split_lexed_sentences(effect_parse_tokens);
    let [exile, permission] = sentences.as_slice() else {
        return false;
    };
    matches!(
        crate::runtime_backend::lexer::parser_token_word_refs(exile).as_slice(),
        [
            "exile", "cards", "equal", "to", "its", "power", "from", "the", "top", "of", "its",
            "owners", "library"
        ]
    ) && matches!(
        crate::runtime_backend::lexer::parser_token_word_refs(permission).as_slice(),
        [
            "you", "may", "cast", "spells", "from", "among", "those", "cards", "for", "as", "long",
            "as", "they", "remain", "exiled", "and", "mana", "of", "any", "type", "can", "be",
            "spent", "to", "cast", "them"
        ]
    )
}

/// Preserve the authored optional collection cast long enough for the
/// two-sentence looked-hand rule to bind its zone owner. Generic pronoun
/// normalization would otherwise reduce the second sentence to `Cast it`.
pub(crate) fn is_authored_look_hand_optional_cast_bundle(
    effect_parse_tokens: &[OwnedLexToken],
) -> bool {
    let sentences = split_lexed_sentences(effect_parse_tokens);
    let [look, cast] = sentences.as_slice() else {
        return false;
    };
    matches!(
        crate::runtime_backend::lexer::parser_token_word_refs(look).as_slice(),
        ["look", "at", "that", "players", "hand"]
    ) && matches!(
        crate::runtime_backend::lexer::parser_token_word_refs(cast).as_slice(),
        [
            "you", "may", "cast", "a", "spell", "from", "among", "those", "cards", "without",
            "paying", "its", "mana", "cost"
        ]
    )
}

fn exact_revealed_hand_union_count_statement(
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(effect_parse_tokens)
        .into_iter()
        .map(crate::runtime_backend::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    let Ok(Some(matched)) =
        crate::runtime_backend::effect_sentences::try_parse_subject_verb_sequence_rule(
            &sentences, 0,
        )
    else {
        return None;
    };
    (matched.feature_tag == Some("revealed-hand-union-count")
        && matched.consumed_sentences == sentences.len())
    .then_some(matched.effects)
}

fn preserve_triggered_effect_surfaces(
    mut parsed: LineAst,
    effect_parse_tokens: &[OwnedLexToken],
    full_parse_tokens: &[OwnedLexToken],
) -> LineAst {
    let Ok(mut surfaced) = parse_effect_sentences_preserving_source_boundaries(effect_parse_tokens)
    else {
        return parsed;
    };
    let full_words = crate::runtime_backend::token_word_refs(full_parse_tokens);
    let explicit_participant_order = full_words.windows(5).any(|window| {
        window[0].eq_ignore_ascii_case("starting")
            && window[1].eq_ignore_ascii_case("with")
            && window[2].eq_ignore_ascii_case("you")
            && window[3].eq_ignore_ascii_case("each")
            && window[4].eq_ignore_ascii_case("player")
    });
    if explicit_participant_order
        && let Some(EffectAst::SourceSentence {
            starting_with_controller,
            ..
        }) = surfaced.first_mut()
    {
        *starting_with_controller = true;
    } else if explicit_participant_order {
        surfaced = vec![EffectAst::SourceSentence {
            effects: surfaced,
            leading_then: false,
            starting_with_controller: true,
        }];
    }
    fn without_source_sentence_markers(effects: &[EffectAst]) -> Vec<EffectAst> {
        let mut flattened = Vec::new();
        for effect in effects {
            match effect {
                EffectAst::SourceSentence { effects, .. } => {
                    flattened.extend(without_source_sentence_markers(effects));
                }
                effect => flattened.push(effect.clone()),
            }
        }
        flattened
    }
    fn without_surface_markers(effects: &[EffectAst]) -> Vec<EffectAst> {
        let mut flattened = Vec::new();
        for effect in effects {
            match effect {
                EffectAst::SourceSentence { effects, .. }
                | EffectAst::CommaThen { effects }
                | EffectAst::Coordinated { effects, .. } => {
                    flattened.extend(without_surface_markers(effects));
                }
                effect => {
                    let mut effect = effect.clone();
                    // Surface provenance can sit inside semantic owners such
                    // as `May`, `IfResult`, or a conditional branch. Compare
                    // those owners after recursively erasing only the nested
                    // presentation wrappers; a shallow comparison otherwise
                    // rejects a valid resurfacing merely because the authored
                    // `, then` was inside an optional program.
                    crate::runtime_backend::model::effect_ast_traversal::for_each_nested_effect_vec_mut(
                        &mut effect,
                        true,
                        |nested| {
                            *nested = without_surface_markers(nested);
                        },
                    );
                    flattened.push(effect);
                }
            }
        }
        flattened
    }
    let sentence_flattened = without_source_sentence_markers(&surfaced);
    let flattened = without_surface_markers(&surfaced);
    if surfaced == flattened {
        return parsed;
    }

    fn matches_surfaced_effects(
        effects: &[EffectAst],
        sentence_flattened: &[EffectAst],
        flattened: &[EffectAst],
    ) -> bool {
        effects == sentence_flattened
            || effects == flattened
            || without_surface_markers(effects) == flattened
    }

    fn replace_matching_effects(
        parsed: &mut LineAst,
        sentence_flattened: &[EffectAst],
        flattened: &[EffectAst],
        surfaced: &[EffectAst],
    ) -> bool {
        match parsed {
            LineAst::Triggered { effects, .. }
                if matches_surfaced_effects(effects, sentence_flattened, flattened) =>
            {
                *effects = surfaced.to_vec();
                true
            }
            LineAst::Ability(parsed)
                if parsed.effects_ast.as_deref().is_some_and(|effects| {
                    matches_surfaced_effects(effects, sentence_flattened, flattened)
                }) =>
            {
                parsed.effects_ast = Some(surfaced.to_vec());
                true
            }
            LineAst::Multiple(chunks) => chunks.iter_mut().any(|chunk| {
                replace_matching_effects(chunk, sentence_flattened, flattened, surfaced)
            }),
            _ => false,
        }
    }

    let _ = replace_matching_effects(&mut parsed, &sentence_flattened, &flattened, &surfaced);
    parsed
}

fn full_text_has_non_mana_activated_ability_qualifier(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    words.windows(6).any(|window| {
        window[0] == "if"
            && window[1] == "it"
            && matches!(window[2], "isnt" | "isn't" | "not")
            && window[3] == "a"
            && window[4] == "mana"
            && window[5] == "ability"
    })
}

fn mark_non_mana_activated_trigger(trigger: &mut TriggerSpec) {
    match trigger {
        TriggerSpec::AbilityActivated { non_mana_only, .. } => *non_mana_only = true,
        TriggerSpec::WithIntro { trigger, .. } => mark_non_mana_activated_trigger(trigger),
        TriggerSpec::Either(left, right) => {
            mark_non_mana_activated_trigger(left);
            mark_non_mana_activated_trigger(right);
        }
        _ => {}
    }
}

fn mark_non_mana_activated_line(line: &mut LineAst) {
    match line {
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                mark_non_mana_activated_line(chunk);
            }
        }
        LineAst::Triggered { trigger, .. } => mark_non_mana_activated_trigger(trigger),
        _ => {}
    }
}

pub(crate) fn parse_library_origin_source_pump_unblockable_triggered_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    fn exact_owned_card_filter(filter: &ObjectFilter) -> bool {
        filter.owner == Some(PlayerFilter::You)
            && filter.nontoken
            && filter.card_types.is_empty()
            && filter.zone.is_none()
            && filter.controller.is_none()
    }

    fn recover_exact_library_origin(trigger: &TriggerSpec) -> Option<TriggerSpec> {
        match trigger {
            TriggerSpec::WithIntro { intro, trigger } => Some(TriggerSpec::WithIntro {
                intro: *intro,
                trigger: Box::new(recover_exact_library_origin(trigger)?),
            }),
            TriggerSpec::PutIntoGraveyardFromZone {
                filter,
                from: Zone::Library,
                one_or_more: true,
            } if exact_owned_card_filter(filter) => Some(trigger.clone()),
            TriggerSpec::PutIntoGraveyardOneOrMore(filter) if exact_owned_card_filter(filter) => {
                Some(TriggerSpec::PutIntoGraveyardFromZone {
                    filter: filter.clone(),
                    from: Zone::Library,
                    one_or_more: true,
                })
            }
            _ => None,
        }
    }

    let Some(split) = semantic_grammar::parse_comma_split_tokens(tokens) else {
        return Ok(None);
    };
    if !crate::runtime_backend::lexer::parser_token_word_refs(split.before)
        .windows(3)
        .any(|window| window == ["from", "your", "library"])
    {
        return Ok(None);
    }
    let authored_intro =
        super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(split.before);
    let trigger_tokens = if split
        .before
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
    {
        &split.before[1..]
    } else {
        split.before
    };
    let trigger = match parse_trigger_clause_lexed(trigger_tokens) {
        Ok(trigger) => trigger,
        Err(_) => {
            let Some(origin_idx) = trigger_tokens.windows(3).position(|window| {
                window[0].is_word("from")
                    && window[1].is_word("your")
                    && window[2].is_word("library")
            }) else {
                return Ok(None);
            };
            let without_origin = trigger_tokens[..origin_idx]
                .iter()
                .chain(trigger_tokens[origin_idx + 3..].iter())
                .cloned()
                .collect::<Vec<_>>();
            let Ok(trigger) = parse_trigger_clause_lexed(&without_origin) else {
                return Ok(None);
            };
            trigger
        }
    };
    let Some(mut trigger) = recover_exact_library_origin(&trigger) else {
        return Ok(None);
    };
    if let Some(intro) = authored_intro {
        trigger = TriggerSpec::WithIntro {
            intro,
            trigger: Box::new(trigger),
        };
    }
    // Parse the pump independently from the shared-subject restriction. The
    // broad `can't` sentence family otherwise claims the full conjunction and
    // mistakes the trigger's `creature card in your library` for its subject.
    let Some(and_idx) = split.after.iter().enumerate().find_map(|(idx, token)| {
        (token.is_word("and")
            && matches!(
                crate::runtime_backend::lexer::parser_token_word_refs(&split.after[idx + 1..])
                    .as_slice(),
                ["cant" | "can't", "be", "blocked", "this", "turn"]
                    | ["can", "t", "be", "blocked", "this", "turn"]
            ))
        .then_some(idx)
    }) else {
        return Ok(None);
    };
    let pump_words = crate::runtime_backend::lexer::parser_token_word_refs(trim_lexed_commas(
        &split.after[..and_idx],
    ));
    if !pump_words.starts_with(&["this", "creature", "gets"])
        || !pump_words.ends_with(&["until", "end", "of", "turn"])
        || !(pump_words.contains(&"+1/+1")
            || pump_words.iter().filter(|word| **word == "1").count() == 2)
    {
        return Ok(None);
    }
    let mut effects = vec![EffectAst::subject_verb_pump(
        Value::Fixed(1),
        Value::Fixed(1),
        TargetAst::Source(None),
        Until::EndOfTurn,
        None,
    )];
    effects.push(EffectAst::subject_verb_cant(
        crate::effect::Restriction::BeBlocked(ObjectFilter::source()),
        Until::EndOfTurn,
        None,
    ));
    Ok(Some(LineAst::Triggered {
        trigger,
        effects,
        max_triggers_per_turn: None,
    }))
}

fn parse_exiled_last_counter_triggered_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let Some(split) = semantic_grammar::parse_comma_split_tokens(tokens) else {
        return Ok(None);
    };
    if !split
        .before
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
    {
        return Ok(None);
    }
    let Some(while_idx) = split.before.iter().position(|token| token.is_word("while")) else {
        return Ok(None);
    };
    let qualifier_words =
        crate::runtime_backend::lexer::parser_token_word_refs(&split.before[while_idx..]);
    let is_exiled_qualifier = matches!(
        qualifier_words.as_slice(),
        ["while", "it", "s", "exiled"]
            | ["while", "it", "is", "exiled"]
            | ["while", "its", "exiled"]
            | ["while", "it's", "exiled"]
    );
    if !is_exiled_qualifier || while_idx <= 1 {
        return Ok(None);
    }

    let trigger = parse_trigger_clause_lexed(&split.before[1..while_idx])?;
    if !matches!(
        &trigger,
        TriggerSpec::CounterRemovedFrom {
            filter,
            last: true,
            ..
        } if filter.source
    ) {
        return Ok(None);
    }
    let effects = parse_effect_sentences_preserving_source_boundaries(split.after)?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(LineAst::Triggered {
        trigger,
        effects,
        max_triggers_per_turn: None,
    }))
}

fn parse_triggered_ability_line_impl(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let source_text = triggered_line_source_text(line);
    let source_text_tokens = if source_text.trim() == line.info.raw_line.trim() {
        line.info.source_tokens.as_slice()
    } else {
        full_parse_tokens
    };
    let authored_raw_tokens =
        crate::runtime_backend::lexer::lex_line(&line.info.raw_line, line.info.line_index)
            .unwrap_or_else(|_| line.info.source_tokens.clone());
    let source_intro = super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(
        &source_text_tokens,
    );
    let full_intro = super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(
        full_parse_tokens,
    );
    let trigger_surface_text = if source_intro.is_some() || full_intro.is_none() {
        source_text.as_str()
    } else {
        line.full_text.trim()
    };
    let mut trigger_facts = line.info.semantic_facts.triggered_ability.clone();
    if let Some(intro_surface) =
        super::super::grammar::trigger_surface::parse_trigger_intro_surface_tokens(
            full_parse_tokens,
        )
    {
        // A physical Oracle line can contain more than one triggered sentence.
        // Each prepared chunk owns its own introduction; do not inherit the
        // first sentence's `When`/`Whenever` surface from line-level facts.
        trigger_facts.intro_surface = Some(intro_surface);
    }
    let trigger_facts = &trigger_facts;
    let chosen_option = line.chosen_option.as_ref();
    let presentation_label = line.presentation.as_ref();
    let inferred_max_triggers_per_turn = line.max_triggers_per_turn;
    let full_text_facts = semantic_grammar::parse_triggered_text_facts_tokens(full_parse_tokens);
    let effect_text_facts =
        semantic_grammar::parse_triggered_text_facts_tokens(effect_parse_tokens);

    // Eminence abilities are live in two functional zones. The document
    // splitter already proves the trigger/effect boundary and intervening
    // condition, but its ordinary fallback defaults the ability to the
    // battlefield and can detach the resolution body as a spell instruction.
    // Rebuild only the typed ability-word shell that explicitly names the
    // command-zone-or-battlefield source condition.
    let authored_words =
        crate::runtime_backend::lexer::parser_token_word_refs(&authored_raw_tokens);
    let has_eminence_label = authored_words.first() == Some(&"eminence");
    let names_command_or_battlefield = authored_words.windows(8).any(|window| {
        window
            == [
                "in",
                "the",
                "command",
                "zone",
                "or",
                "on",
                "the",
                "battlefield",
            ]
    });
    if has_eminence_label && names_command_or_battlefield {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        let effects = parse_effect_sentences_lexed(effect_parse_tokens)?;
        if !effects.is_empty() {
            let label = PresentationLabel::from_ability_word("Eminence");
            return Ok(LineAst::Ability(rewrite_parsed_triggered_ability(
                trigger,
                effects,
                vec![Zone::Command, Zone::Battlefield],
                Some(line.info.raw_line.clone()),
                line.intervening_if
                    .as_ref()
                    .map(|predicate| {
                        crate::runtime_backend::lowering::compile_support::compile_condition_from_predicate_ast_with_env(
                            predicate,
                            &crate::runtime_backend::reference_model::ReferenceEnv::default(),
                            None,
                        )
                    })
                    .transpose()?,
                Some(&label),
                ReferenceImports::default(),
            )));
        }
    }

    // `while it's exiled` qualifies the source of the counter-removal event;
    // it is not part of the restriction after the comma. Prepared trigger
    // rewrites can otherwise split at `this` and feed `card while ...` into
    // the effect parser, producing an unrelated object-filter union.
    let exiled_last_counter = parse_exiled_last_counter_triggered_line(&authored_raw_tokens)?.or(
        parse_exiled_last_counter_triggered_line(source_text_tokens)?,
    );
    if let Some(chunk) = exiled_last_counter {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(chunk, line.intervening_if.clone())?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    // The public document splitter can simplify the coordinated
    // "cast ... or activate ..." trigger head to its spell arm before this
    // semantic handoff.  The authored line still carries the exact grammar
    // proof for both trigger domains and the shared X-cost qualification, so
    // give that intact head first refusal while retaining the already-parsed
    // effect slice (which owns the copy/retarget reference flow).
    if let Some(split) = semantic_grammar::parse_comma_split_tokens(&authored_raw_tokens)
        && let Some(chunk) = lower_spell_or_activated_ability_x_cost_trigger(
            &authored_raw_tokens,
            split.before,
            effect_parse_tokens,
            inferred_max_triggers_per_turn,
        )?
    {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(chunk, line.intervening_if.clone())?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    // Parley-style reveal programs carry one revealed-card set across three
    // authored sentences. Prepared trigger-body normalization can split that
    // set before the `revealed this way` iterator is resolved, leaving only a
    // bare reveal, token creation, and draw. Give the intact authored tail
    // first refusal when all three grammar facts are present.
    if let Some(split) = semantic_grammar::parse_comma_split_tokens(&authored_raw_tokens) {
        let words = crate::runtime_backend::lexer::parser_token_word_refs(split.after);
        let conditional_gate_remainder_program = words
            .windows(8)
            .any(|w| w == ["look", "at", "the", "top", "nine", "cards", "of", "your"])
            && words
                .windows(7)
                .any(|w| w == ["put", "a", "gate", "card", "from", "among", "them"])
            && words
                .windows(7)
                .any(|w| w == ["if", "you", "control", "nine", "or", "more", "gates"])
            && words
                .windows(5)
                .any(|w| w == ["otherwise", "put", "the", "rest", "on"]);
        if conditional_gate_remainder_program {
            let effects = parse_effect_sentences_lexed(split.after)?;
            if !effects.is_empty() {
                let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
                return apply_chosen_option_to_triggered_chunk(
                    apply_explicit_intervening_if_to_triggered_chunk(
                        LineAst::Triggered {
                            trigger,
                            effects,
                            max_triggers_per_turn: inferred_max_triggers_per_turn,
                        },
                        line.intervening_if.clone(),
                    )?,
                    trigger_surface_text,
                    trigger_facts,
                    inferred_max_triggers_per_turn,
                    chosen_option,
                    presentation_label,
                );
            }
        }
        let parley_reveal_program = words
            .windows(7)
            .any(|w| w == ["each", "player", "reveals", "the", "top", "card", "of"])
            && words
                .windows(7)
                .any(|w| w == ["for", "each", "nonland", "card", "revealed", "this", "way"])
            && words
                .windows(5)
                .any(|w| w == ["each", "player", "draws", "a", "card"]);
        if parley_reveal_program {
            let effects = parse_effect_sentences_lexed(split.after)?;
            if !effects.is_empty() {
                let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
                return apply_chosen_option_to_triggered_chunk(
                    apply_explicit_intervening_if_to_triggered_chunk(
                        LineAst::Triggered {
                            trigger,
                            effects,
                            max_triggers_per_turn: inferred_max_triggers_per_turn,
                        },
                        line.intervening_if.clone(),
                    )?,
                    trigger_surface_text,
                    trigger_facts,
                    inferred_max_triggers_per_turn,
                    chosen_option,
                    presentation_label,
                );
            }
        }
    }

    // Some document routes retain the full authored tail only in the effect
    // or source-token view rather than `raw_line`. Repeat the same narrow
    // proof over those views so the public runtime-backed path cannot bypass
    // the collection semantics merely because its raw chunk was shortened.
    for candidate in [effect_parse_tokens, source_text_tokens, full_parse_tokens] {
        let candidate_words = crate::runtime_backend::lexer::parser_token_word_refs(candidate);
        let tail = if candidate_words.contains(&"whenever") || candidate_words.contains(&"when") {
            semantic_grammar::parse_comma_split_tokens(candidate)
                .map(|split| split.after)
                .unwrap_or(candidate)
        } else {
            candidate
        };
        let words = crate::runtime_backend::lexer::parser_token_word_refs(tail);
        let is_parley = words
            .windows(7)
            .any(|w| w == ["each", "player", "reveals", "the", "top", "card", "of"])
            && words
                .windows(7)
                .any(|w| w == ["for", "each", "nonland", "card", "revealed", "this", "way"])
            && words
                .windows(5)
                .any(|w| w == ["each", "player", "draws", "a", "card"]);
        let is_gate_partition = words
            .windows(8)
            .any(|w| w == ["look", "at", "the", "top", "nine", "cards", "of", "your"])
            && words
                .windows(7)
                .any(|w| w == ["put", "a", "gate", "card", "from", "among", "them"])
            && words
                .windows(7)
                .any(|w| w == ["if", "you", "control", "nine", "or", "more", "gates"]);
        if is_parley || is_gate_partition {
            let effects = parse_effect_sentences_lexed(tail)?;
            if !effects.is_empty() {
                let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
                return apply_chosen_option_to_triggered_chunk(
                    apply_explicit_intervening_if_to_triggered_chunk(
                        LineAst::Triggered {
                            trigger,
                            effects,
                            max_triggers_per_turn: inferred_max_triggers_per_turn,
                        },
                        line.intervening_if.clone(),
                    )?,
                    trigger_surface_text,
                    trigger_facts,
                    inferred_max_triggers_per_turn,
                    chosen_option,
                    presentation_label,
                );
            }
        }
    }

    // This exact two-sentence procedure deliberately links the token created
    // by the first sentence to a delayed sacrifice on the controller's next
    // turn. Claim it before the broad sentence probes, which otherwise try to
    // parse `end step on your next turn` as an ordinary `end` action.
    if let Some(effects) = linked_created_token_next_turn_sacrifice_effects(effect_parse_tokens)? {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(
                LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn: inferred_max_triggers_per_turn,
                },
                line.intervening_if.clone(),
            )?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    // A serial list of independently targeted P/T modifiers owns its shared
    // leading duration. The ordinary triggered-body splitter can otherwise
    // treat the first required target as setup for the two optional `other`
    // targets, dropping its modifier and normalizing the surviving duration.
    // Claim the already-typed generic sequence before those split probes.
    let serial_target_modifiers =
        crate::runtime_backend::effect_sentences::parse_serial_target_pt_modifiers_sentence(
            effect_parse_tokens,
        )?
        .or_else(|| {
            crate::runtime_backend::grammar::semantic_lowering::parse_comma_split_tokens(
                &authored_raw_tokens,
            )
            .and_then(|split| {
                crate::runtime_backend::effect_sentences::parse_serial_target_pt_modifiers_sentence(
                    split.after,
                )
                .ok()
                .flatten()
            })
        });
    if let Some(effects) = serial_target_modifiers {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(
                LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn: inferred_max_triggers_per_turn,
                },
                line.intervening_if.clone(),
            )?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    // A prepared triggered-body slice can already have reduced an authored
    // dynamic token to its 0/0 definition. Reparse only the grammar-proven
    // aggregate death-group creation from the intact source tail before that
    // lossy slice reaches ordinary sentence parsing.
    if let Some(effect) = authored_dynamic_token_creation_from_trigger(&authored_raw_tokens)? {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(
                LineAst::Triggered {
                    trigger,
                    effects: vec![effect],
                    max_triggers_per_turn: inferred_max_triggers_per_turn,
                },
                line.intervening_if.clone(),
            )?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    // The CST's prepared effect slice can already have lost the quote
    // boundaries that distinguish token rules from the outer instruction.
    // At this semantic handoff, the source-token stream is still intact.
    // Give only a fully parsed quantified create-with-embedded-rules tail
    // first refusal, then lower it with the ordinary trigger and presentation
    // wrappers. This prevents a quoted `can't block` rule from becoming the
    // resolution's outer restriction.
    if let Some(source_split) = semantic_grammar::parse_comma_split_tokens(source_text_tokens)
        && let Some(effect) = crate::runtime_backend::effect_sentences::
            parse_quantified_token_creation_with_embedded_rules(source_split.after)?
    {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(
                LineAst::Triggered {
                    trigger,
                    effects: vec![effect],
                    max_triggers_per_turn: inferred_max_triggers_per_turn,
                },
                line.intervening_if.clone(),
            )?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    if let Some(chunk) = parse_linked_attack_group_combat_triggered_line_lexed(full_parse_tokens)? {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(chunk, line.intervening_if.clone())?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    if let Some(chunk) =
        parse_library_origin_source_pump_unblockable_triggered_line(full_parse_tokens)?
    {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(chunk, line.intervening_if.clone())?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    // The prepared trigger body can let the broad `can't` family claim this
    // exact shared-source conjunction before the generic subject/verb parser
    // sees its leading P/T modifier. Reparse only the intact authored tail
    // through the strict two-effect grammar, then keep the already-prepared
    // trigger and presentation wrappers.
    let authored_source_pump_unblockable =
        crate::runtime_backend::effect_sentences::parse_source_gets_unblockable_subject_verb(
            effect_parse_tokens,
        )?
        .or(
            semantic_grammar::parse_comma_split_tokens(&authored_raw_tokens)
                .or_else(|| semantic_grammar::parse_comma_split_tokens(source_text_tokens))
                .and_then(|split| {
                    crate::runtime_backend::effect_sentences::
                    parse_source_gets_unblockable_subject_verb(split.after)
                    .transpose()
                })
                .transpose()?,
        );
    if let Some(effects) = authored_source_pump_unblockable {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(
                LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn: inferred_max_triggers_per_turn,
                },
                line.intervening_if.clone(),
            )?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    // These two-sentence programs own typed provenance across the sentence
    // boundary. Route the already-proved bundle before the generic
    // returned-object/static splitter or ordinary sentence parser can peel
    // the follow-up away from its producer. Some public CST paths retain the
    // exact authored line in `source_text_tokens` while their prepared effect
    // slice has already been simplified sentence-by-sentence. Re-probe only
    // the authored post-trigger tail so the typed dynamic count, owner, and
    // shared exiled collection survive that handoff.
    let authored_tail = semantic_grammar::parse_comma_split_tokens(&authored_raw_tokens)
        .or_else(|| semantic_grammar::parse_comma_split_tokens(source_text_tokens))
        .map(|split| split.after);
    let authored_correlated_effects = authored_tail
        .as_ref()
        .and_then(|tokens| exact_dynamic_exile_permission_bundle(tokens));
    let authored_looked_hand_cast = authored_tail
        .as_ref()
        .and_then(|tokens| exact_looked_hand_optional_cast_bundle(tokens));
    if let Some(effects) = exact_dynamic_exile_permission_bundle(effect_parse_tokens)
        .or(authored_correlated_effects)
        .or_else(|| exact_atomic_return_as_aura_bundle(effect_parse_tokens))
        .or_else(|| exact_looked_hand_optional_cast_bundle(effect_parse_tokens))
        .or(authored_looked_hand_cast)
    {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(
                LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn: inferred_max_triggers_per_turn,
                },
                line.intervening_if.clone(),
            )?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    if let Some(chunk) = parse_special_triggered_line(
        line,
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )? {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(chunk, line.intervening_if.clone())?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    if full_text_facts.has_full_party_instead
        && let Ok(trigger) = parse_trigger_clause_lexed(trigger_parse_tokens)
    {
        let effect_tokens;
        if effect_text_facts.has_full_party_condition {
            effect_tokens = effect_parse_tokens;
        } else {
            effect_tokens = semantic_grammar::parse_comma_split_tokens(full_parse_tokens)
                .map(|split| split.after)
                .unwrap_or(effect_parse_tokens);
        }
        let effects = parse_effect_sentences_lexed(effect_tokens)?;
        if !effects.is_empty() {
            return apply_chosen_option_to_triggered_chunk(
                apply_explicit_intervening_if_to_triggered_chunk(
                    LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: inferred_max_triggers_per_turn,
                    },
                    line.intervening_if.clone(),
                )?,
                trigger_surface_text,
                trigger_facts,
                inferred_max_triggers_per_turn,
                chosen_option,
                presentation_label,
            );
        }
    }

    let selected_effect_sentences = split_lexed_sentences(effect_parse_tokens);
    let selected_effect_has_token_creation_followup_after_first =
        sentences_have_token_creation_followup_after_first(&selected_effect_sentences);
    let selected_effect_has_temporary_static_followup_after_first =
        sentences_have_temporary_static_followup_after_first(&selected_effect_sentences);
    let selected_effect_has_bound_characteristic_followup_after_first =
        sentences_have_bound_characteristic_followup_after_first(&selected_effect_sentences);
    let selected_effect_has_counter_linked_land_subtype_followup_after_first =
        selected_effect_sentences.iter().skip(1).any(|sentence| {
            super::super::grammar::effects::followup_shapes::parse_counter_linked_land_subtype_followup(sentence)
                .is_some()
        });
    if let Some((first_followup_idx, mut followup_effects)) =
        returned_object_static_followup_effects(&selected_effect_sentences)?
        && let Ok(trigger) = parse_trigger_clause_lexed(trigger_parse_tokens)
    {
        let trigger_effect_sentences = selected_effect_sentences[..first_followup_idx]
            .iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>();
        let trigger_effect_tokens = join_sentences_with_period(&trigger_effect_sentences);
        if let Ok(parsed_effects) = parse_effect_sentences_lexed(&trigger_effect_tokens) {
            let mut effects =
                wrap_future_draw_replacement_effects(full_parse_tokens, parsed_effects);
            if !effects.is_empty() {
                effects.append(&mut followup_effects);
                return apply_chosen_option_to_triggered_chunk(
                    apply_explicit_intervening_if_to_triggered_chunk(
                        LineAst::Triggered {
                            trigger,
                            effects,
                            max_triggers_per_turn: inferred_max_triggers_per_turn,
                        },
                        line.intervening_if.clone(),
                    )?,
                    trigger_surface_text,
                    trigger_facts,
                    inferred_max_triggers_per_turn,
                    chosen_option,
                    presentation_label,
                );
            }
        }
    }
    let selected_split_has_trailing_static_after_first = selected_effect_sentences.len() > 1
        && !selected_effect_has_token_creation_followup_after_first
        && !selected_effect_has_temporary_static_followup_after_first
        && !selected_effect_has_bound_characteristic_followup_after_first
        && selected_effect_sentences
            .iter()
            .enumerate()
            .skip(1)
            .any(|(_, sentence)| {
                !sentence_is_linked_anaphoric_conditional_effect(sentence)
                    && (parse_self_enters_with_x_counters_static_chunk(sentence).is_some()
                        || matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_))))
            });

    let full_sentences = split_lexed_sentences(full_parse_tokens);
    let has_token_creation_followup_after_first =
        sentences_have_token_creation_followup_after_first(&full_sentences);
    let has_temporary_static_followup_after_first =
        sentences_have_temporary_static_followup_after_first(&full_sentences);
    let has_bound_characteristic_followup_after_first =
        sentences_have_bound_characteristic_followup_after_first(&full_sentences);
    if full_sentences.len() > 1
        && !has_token_creation_followup_after_first
        && !has_temporary_static_followup_after_first
        && !has_bound_characteristic_followup_after_first
        && !selected_effect_has_counter_linked_land_subtype_followup_after_first
        && !selected_split_has_trailing_static_after_first
        && let Ok(first_triggered) = parse_triggered_line_lexed(full_sentences[0])
    {
        let mut chunks = Vec::with_capacity(full_sentences.len());
        chunks.push(apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(
                first_triggered,
                line.intervening_if.clone(),
            )?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option.clone(),
            presentation_label,
        )?);

        let mut parsed_all_static = true;
        for sentence in full_sentences.iter().skip(1) {
            if sentence_is_linked_anaphoric_conditional_effect(sentence) {
                parsed_all_static = false;
                break;
            } else if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(sentence) {
                chunks.push(chunk);
            } else if let Some(abilities) = parse_static_ability_ast_line_lexed(sentence)? {
                chunks.push(LineAst::StaticAbilities(abilities));
            } else {
                parsed_all_static = false;
                break;
            }
        }
        if parsed_all_static {
            return Ok(LineAst::Multiple(chunks));
        }
    }

    let effect_sentences = split_lexed_sentences(effect_parse_tokens);
    let effect_has_token_creation_followup_after_first =
        sentences_have_token_creation_followup_after_first(&effect_sentences);
    let effect_has_temporary_static_followup_after_first =
        sentences_have_temporary_static_followup_after_first(&effect_sentences);
    let effect_has_bound_characteristic_followup_after_first =
        sentences_have_bound_characteristic_followup_after_first(&effect_sentences);
    let effect_is_linked_typed_bundle =
        crate::runtime_backend::effect_sentences::parse_typed_effect_bundle_lexed(
            effect_parse_tokens,
        )
        .is_some();
    let effect_is_linked_collect_evidence =
        is_optional_source_exile_collect_evidence_procedure(effect_parse_tokens);
    if let Some(effects) = linked_created_token_next_turn_sacrifice_effects(effect_parse_tokens)?
        && let Ok(trigger) = parse_trigger_clause_lexed(trigger_parse_tokens)
    {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(
                LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn: inferred_max_triggers_per_turn,
                },
                line.intervening_if.clone(),
            )?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }
    if effect_sentences.len() > 1
        && !effect_has_token_creation_followup_after_first
        && !effect_has_temporary_static_followup_after_first
        && !effect_has_bound_characteristic_followup_after_first
        && !selected_effect_has_counter_linked_land_subtype_followup_after_first
        // A sentence that looks static in isolation may modify the exact
        // exiled card established by the preceding resolution instructions.
        // Keep any complete typed bundle together so its linked target and
        // duration survive into the triggered ability instead of becoming a
        // top-level battlefield static ability.
        && !effect_is_linked_typed_bundle
        && let Some(first_static_idx) =
            effect_sentences
                .iter()
                .enumerate()
                .skip(1)
                .find_map(|(idx, sentence)| {
                    (!sentence_is_linked_anaphoric_conditional_effect(sentence)
                        && (parse_self_enters_with_x_counters_static_chunk(sentence).is_some()
                            || matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))))
                    .then_some(idx)
                })
        && let Ok(trigger) = parse_trigger_clause_lexed(trigger_parse_tokens)
    {
        let trigger_effect_sentences = effect_sentences[..first_static_idx]
            .iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>();
        let trigger_effect_tokens = join_sentences_with_period(&trigger_effect_sentences);
        let effects = wrap_future_draw_replacement_effects(
            full_parse_tokens,
            parse_effect_sentences_lexed(&trigger_effect_tokens)?,
        );
        if !effects.is_empty() {
            let mut chunks = Vec::new();
            chunks.push(apply_chosen_option_to_triggered_chunk(
                apply_explicit_intervening_if_to_triggered_chunk(
                    LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: inferred_max_triggers_per_turn,
                    },
                    line.intervening_if.clone(),
                )?,
                trigger_surface_text,
                trigger_facts,
                inferred_max_triggers_per_turn,
                chosen_option.clone(),
                presentation_label,
            )?);

            for sentence in effect_sentences.iter().skip(first_static_idx) {
                if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(sentence) {
                    chunks.push(chunk);
                } else if let Some(abilities) = parse_static_ability_ast_line_lexed(sentence)? {
                    chunks.push(LineAst::StaticAbilities(abilities));
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "could not parse trailing static sentence in triggered line '{}'",
                        line.info.raw_line
                    )));
                }
            }
            return Ok(LineAst::Multiple(chunks));
        }
    }

    if !token_word_refs(effect_parse_tokens).is_empty()
        && (!full_parse_tokens_have_triggered_intervening_if_clause(full_parse_tokens)
            || effect_is_linked_collect_evidence)
        && (!full_text_facts.has_if_you_do
            || effect_is_linked_typed_bundle
            || effect_is_linked_collect_evidence)
        && (!full_text_facts.has_if_you_dont || effect_is_linked_typed_bundle)
        && !effect_text_facts.starts_with_if
    {
        let direct_trigger = parse_trigger_clause_lexed(trigger_parse_tokens).map(|mut trigger| {
            if full_text_has_non_mana_activated_ability_qualifier(full_parse_tokens) {
                mark_non_mana_activated_trigger(&mut trigger);
            }
            trigger
        });
        let direct_effects =
            if effect_is_linked_typed_bundle || effect_is_linked_collect_evidence {
                // These procedures deliberately correlate effects across their
                // authored sentence boundaries. The ordinary boundary-preserving
                // route parses each sentence in isolation and loses the linked
                // value, player, or object-set provenance.
                parse_effect_sentences_lexed(effect_parse_tokens)
            } else {
                parse_effect_sentences_preserving_source_boundaries(effect_parse_tokens)
            }
            .map(|effects| wrap_future_draw_replacement_effects(full_parse_tokens, effects));
        if let (Ok(trigger), Ok(effects)) = (direct_trigger, direct_effects)
            && !effects.is_empty()
        {
            return apply_chosen_option_to_triggered_chunk(
                apply_explicit_intervening_if_to_triggered_chunk(
                    LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: inferred_max_triggers_per_turn,
                    },
                    line.intervening_if.clone(),
                )?,
                trigger_surface_text,
                trigger_facts,
                inferred_max_triggers_per_turn,
                chosen_option,
                presentation_label,
            );
        }
    }

    let mut parsed = apply_explicit_intervening_if_to_triggered_chunk(
        parse_triggered_line_lexed(full_parse_tokens)?,
        line.intervening_if.clone(),
    )?;
    if full_text_has_non_mana_activated_ability_qualifier(full_parse_tokens) {
        mark_non_mana_activated_line(&mut parsed);
    }
    apply_chosen_option_to_triggered_chunk(
        parsed,
        trigger_surface_text,
        trigger_facts,
        inferred_max_triggers_per_turn,
        chosen_option,
        presentation_label,
    )
}

#[cfg(test)]
fn parse_triggered_text_for_test(
    full_text: &str,
    trigger_text: &str,
    effect_text: &str,
) -> Result<LineAst, CardTextError> {
    let full_tokens = lex_line(full_text, 0).expect("full triggered line should lex");
    let trigger_tokens = lex_line(trigger_text, 0).expect("trigger clause should lex");
    let effect_tokens = lex_line(effect_text, 0).expect("trigger effects should lex");
    parse_triggered_line(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: full_text.to_string(),
            source_tokens: full_tokens.clone(),
            normalized: NormalizedLine {
                original: full_text.to_string(),
                normalized: full_text.to_string(),
                char_map: Vec::new(),
            },
            semantic_facts: Default::default(),
        },
        full_text,
        &full_tokens,
        &trigger_tokens,
        &effect_tokens,
        None,
        None,
        None,
        None,
    )
}

#[cfg(test)]
#[test]
fn collect_evidence_if_do_procedure_reaches_the_public_trigger_route() {
    let full = "When this creature dies, you may exile it and collect evidence 4. If you do, return this card to the battlefield tapped.";
    let effects = "you may exile it and collect evidence 4. If you do, return this card to the battlefield tapped.";
    let effect_tokens = lex_line(effects, 0).expect("collect-evidence effects should lex");
    assert!(
        is_optional_source_exile_collect_evidence_procedure(&effect_tokens),
        "sentences={:#?}",
        split_lexed_sentences(&effect_tokens)
            .iter()
            .map(|sentence| token_word_refs(sentence))
            .collect::<Vec<_>>()
    );
    let parsed = parse_triggered_text_for_test(full, "this creature dies", effects)
        .expect("collect-evidence death trigger should parse");
    let debug = format!("{parsed:#?}");
    assert!(
        debug.contains("ChooseObjectsWithAggregateConstraint"),
        "{debug}"
    );
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
    assert!(debug.contains("ReturnToBattlefield"), "{debug}");

    let near_miss = lex_line(
        "you may exile it. If you do, return this card to the battlefield tapped.",
        0,
    )
    .expect("near-miss procedure should lex");
    assert!(!is_optional_source_exile_collect_evidence_procedure(
        &near_miss
    ));
}

#[cfg(test)]
#[test]
fn quantified_token_rules_reach_the_public_trigger_semantic_handoff() {
    let effects = "each opponent creates a 1/1 red Pirate creature token with \"This token can't block\" and \"Creatures you control attack each combat if able.\"";
    let full = format!("When this creature enters, {effects}");
    let parsed = parse_triggered_text_for_test(&full, "this creature enters", effects)
        .expect("the quantified token-rule trigger should parse from source tokens");
    let debug = format!("{parsed:#?}");
    assert!(debug.contains("CreateTokenWithMods"), "{debug}");
    assert!(debug.contains("CantBlock"), "{debug}");
    assert!(debug.contains("MustAttack"), "{debug}");
    assert!(
        !debug.contains("MustBlockSpecificAttacker"),
        "a quoted token rule escaped into the trigger resolution: {debug}"
    );
}

#[cfg(test)]
#[test]
fn created_token_next_turn_sacrifice_stays_inside_the_trigger() {
    let effects = "create a Lander token. At the beginning of the end step on your next turn, sacrifice that token.";
    let effect_tokens = lex_line(effects, 0).expect("linked token procedure should lex");
    let direct = linked_created_token_next_turn_sacrifice_effects(&effect_tokens)
        .expect("linked token helper should not fail")
        .unwrap_or_else(|| {
            panic!(
                "linked token helper did not claim the exact surface: {:#?}",
                split_lexed_sentences(&effect_tokens)
                    .iter()
                    .map(|sentence| token_word_refs(sentence))
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(direct.len(), 2, "{direct:#?}");
    let full = format!("When this creature enters, {effects}");
    let parsed = parse_triggered_text_for_test(&full, "this creature enters", effects)
        .expect("linked token delayed sacrifice should parse");
    let effects = match &parsed {
        LineAst::Triggered { effects, .. } => effects,
        LineAst::Ability(ability) => ability
            .effects_ast
            .as_ref()
            .expect("runtime-backed trigger should retain its typed effects"),
        _ => panic!("both sentences must remain one trigger: {parsed:#?}"),
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CreateTokenWithMods { .. },
            ..
        }),
        EffectAst::DelayedUntilEndStepOfExtraTurn {
            effects: delayed, ..
        },
    ] = effects.as_slice()
    else {
        panic!("unexpected linked token procedure: {effects:#?}");
    };
    let debug = format!("{delayed:#?}");
    assert!(debug.contains(IT_TAG), "{debug}");
    assert!(!debug.contains("token: true"), "{debug}");
}

#[cfg(test)]
#[test]
fn dynamic_death_group_token_creation_reaches_the_public_trigger_semantic_handoff() {
    let effects = "create a green Fungus Dinosaur creature token with base power and toughness each equal to the total power of those creatures.";
    let full = format!("Whenever one or more nontoken creatures you control die, {effects}");
    let parsed = parse_triggered_text_for_test(
        &full,
        "one or more nontoken creatures you control die",
        effects,
    )
    .expect("aggregate death-group token creation should parse from intact source tokens");
    let debug = format!("{parsed:#?}");
    assert!(debug.contains("CreateTokenWithMods"), "{debug}");
    assert!(debug.contains("TotalPower"), "{debug}");
    assert!(
        debug.contains(ironsmith_core::ZONE_CHANGE_GROUP_TAG),
        "{debug}"
    );

    let effect_tail = lex_line(effects, 0).expect("effect-only source tail should lex");
    assert!(
        dynamic_zone_change_group_token_creation_from_authored_trigger(&effect_tail)
            .expect("effect-only public handoff should parse")
            .is_some(),
        "the prepared effect-tail route must retain the same dynamic group payload"
    );

    let fixed_full = "Whenever one or more nontoken creatures you control die, create a 0/0 green Fungus Dinosaur creature token.";
    let mut downgraded = parse_triggered_text_for_test(
        fixed_full,
        "one or more nontoken creatures you control die",
        "create a 0/0 green Fungus Dinosaur creature token.",
    )
    .expect("fixed token shell should parse before source reconciliation");
    let intact_source = lex_line(&full, 0).expect("intact dynamic source should lex");
    reconcile_dynamic_zone_change_group_token_creation(&mut downgraded, &intact_source)
        .expect("intact source should restore the dynamic payload after surface reparsing");
    let reconciled_debug = format!("{downgraded:#?}");
    assert!(
        reconciled_debug.contains("TotalPower"),
        "{reconciled_debug}"
    );
    assert!(
        reconciled_debug.contains(ironsmith_core::ZONE_CHANGE_GROUP_TAG),
        "{reconciled_debug}"
    );

    let fixed = lex_line(
        "Whenever one or more nontoken creatures you control die, create a 0/0 green Fungus Dinosaur creature token.",
        0,
    )
    .expect("fixed token near miss should lex");
    assert!(
        dynamic_zone_change_group_token_creation_from_authored_trigger(&fixed)
            .expect("near-miss probe should not error")
            .is_none()
    );
}

#[cfg(test)]
#[test]
fn dynamic_static_ability_token_count_survives_the_public_trigger_handoff() {
    let effects = "create X Blood tokens, where X is the number of abilities from among flying, first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, reach, trample, and vigilance found among creatures you control.";
    let full = format!("When Odric enters, {effects}");
    let intact = lex_line(&full, 0).expect("authored static-ability aggregate should lex");
    let recovered = dynamic_static_ability_count_token_creation_from_authored_trigger(&intact)
        .expect("authored aggregate probe should not error")
        .expect("authored aggregate should be recovered from its create-verb boundary");
    let debug = format!("{recovered:#?}");
    assert!(debug.contains("StaticAbilitiesAmong"), "{debug}");
    assert!(debug.contains("Vigilance"), "{debug}");

    let fixed = lex_line(
        "When Odric enters, create X Blood tokens, where X is the number of creatures you control.",
        0,
    )
    .expect("ordinary count near miss should lex");
    assert!(
        dynamic_static_ability_count_token_creation_from_authored_trigger(&fixed)
            .expect("ordinary count should not error")
            .is_none()
    );
}

#[cfg(test)]
#[test]
fn dynamic_exile_permission_bundle_reaches_the_public_trigger_route() {
    let effects = "exile cards equal to its power from the top of its owner's library. You may cast spells from among those cards for as long as they remain exiled, and mana of any type can be spent to cast them.";
    let authored_tokens = lex_line(effects, 0).expect("authored linked bundle should lex");
    assert!(
        is_authored_dynamic_exile_permission_bundle(&authored_tokens),
        "the public CST guard must normalize sentence casing and possessive apostrophes"
    );
    let full = format!("When enchanted creature dies, {effects}");
    let parsed = parse_triggered_text_for_test(&full, "enchanted creature dies", effects)
        .expect("linked dynamic exile trigger should parse");
    let LineAst::Triggered {
        effects: parsed_effects,
        ..
    } = parsed
    else {
        panic!("expected one triggered line: {parsed:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: PlayerAst::ItsOwner,
                    ..
                },
            action:
                SubjectVerbActionAst::ExileTopOfLibrary {
                    count,
                    tags,
                    face_down: false,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    tag,
                    allow_land: false,
                    ..
                },
            ..
        }),
    ] = parsed_effects.as_slice()
    else {
        panic!("expected linked dynamic exile and permission: {parsed_effects:#?}");
    };
    assert_eq!(tags, std::slice::from_ref(tag));
    assert!(matches!(
        count.unhinted(),
        Value::PowerOf(spec)
            if matches!(spec.as_ref(), crate::target::ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")
    ));

    // The public document route can hand semantic lowering an independently
    // simplified effect slice even though LineInfo still retains the exact
    // authored trigger line. That lossy slice must not erase the correlated
    // dynamic count, owner, or plural permission.
    let full_tokens = lex_line(&full, 0).expect("full authored trigger should lex");
    let trigger_tokens = lex_line("enchanted creature dies", 0).expect("trigger clause should lex");
    let lossy_effect_tokens = lex_line(
        "exile the top card of your library. You may cast that card for as long as it remains exiled, and mana of any type can be spent to cast that spell.",
        0,
    )
    .expect("lossy public effect slice should lex");
    let recovered = parse_triggered_line(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: full.clone(),
            source_tokens: full_tokens.clone(),
            normalized: NormalizedLine {
                original: full.clone(),
                normalized: full.clone(),
                char_map: Vec::new(),
            },
            semantic_facts: Default::default(),
        },
        &full,
        &full_tokens,
        &trigger_tokens,
        &lossy_effect_tokens,
        None,
        None,
        None,
        None,
    )
    .expect("authored source tail should recover the linked bundle");
    let recovered_debug = format!("{recovered:#?}");
    for required in [
        "ExileTopOfLibrary",
        "PowerOf",
        "ItsOwner",
        "GrantPlayTaggedForAsLongAsExiled",
    ] {
        assert!(
            recovered_debug.contains(required),
            "missing {required}: {recovered_debug}"
        );
    }

    let near_miss = "exile the top card of its owner's library. You may cast spells from among those cards for as long as they remain exiled.";
    let near_miss_tokens = lex_line(near_miss, 0).expect("near miss should lex");
    assert!(!is_authored_dynamic_exile_permission_bundle(
        &near_miss_tokens
    ));
    assert!(
        crate::runtime_backend::effect_sentences::parse_typed_effect_bundle_lexed(
            &near_miss_tokens
        )
        .is_none(),
        "a fixed-count exile must not inherit the dynamic linked-bundle preemption"
    );
}

#[cfg(test)]
#[test]
fn targeted_same_name_graveyard_cast_keeps_target_and_optional_normal_payment() {
    let effects = "you may cast target card with the same name as that spell from your graveyard.";
    let full = format!("Whenever you cast an instant or sorcery spell from your hand, {effects}");
    let parsed = parse_triggered_text_for_test(
        &full,
        "you cast an instant or sorcery spell from your hand",
        effects,
    )
    .expect("the targeted same-name graveyard cast should reach the public trigger route");
    let LineAst::Triggered {
        effects: parsed_effects,
        ..
    } = parsed
    else {
        panic!("expected one triggered line: {parsed:#?}");
    };
    let [
        EffectAst::TagAffected {
            effect: target_effect,
            tag: target_tag,
        },
        EffectAst::May {
            effects: may_effects,
        },
    ] = parsed_effects.as_slice()
    else {
        panic!("expected targeted card followed by optional cast: {parsed_effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::TargetOnly {
                target: TargetAst::Object(filter, _, _),
                explicit_declaration: true,
            },
        ..
    }) = target_effect.as_ref()
    else {
        panic!("expected an explicit object target: {target_effect:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert!(matches!(
        filter.tagged_constraints.as_slice(),
        [crate::target::TaggedObjectConstraint {
            tag,
            relation: crate::target::TaggedOpbjectRelation::SameNameAsTagged,
        }] if tag.as_str() == "triggering"
    ));
    assert!(matches!(
        may_effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CastTagged {
                tag,
                player: PlayerAst::You,
                allow_land: false,
                as_copy: false,
                without_paying_mana_cost: false,
                additional_mana_cost: None,
                cost_reduction: None,
                mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::Normal,
                ..
            },
            ..
        })] if tag == target_tag
    ));

    let free_cast = lex_line(
        "you may cast target card with the same name as that spell from your graveyard without paying its mana cost.",
        0,
    )
    .expect("free-cast near miss should lex");
    assert!(exact_target_same_name_graveyard_may_cast_bundle(&free_cast).is_none());
}

#[cfg(test)]
#[test]
fn looked_hand_optional_cast_authored_guard_keeps_possessive_and_may() {
    let tokens = lex_line(
        "look at that player's hand. You may cast a spell from among those cards without paying its mana cost.",
        0,
    )
    .expect("looked-hand optional cast should lex");
    assert!(is_authored_look_hand_optional_cast_bundle(&tokens));
    assert!(exact_looked_hand_optional_cast_bundle(&tokens).is_some());

    let mandatory = lex_line(
        "look at that player's hand. Cast a spell from among those cards without paying its mana cost.",
        0,
    )
    .expect("mandatory near miss should lex");
    assert!(!is_authored_look_hand_optional_cast_bundle(&mandatory));
}

#[cfg(test)]
#[test]
fn atomic_return_as_aura_bundle_preempts_returned_object_static_split() {
    let effects = "return it to the battlefield. It's an Aura enchantment with enchant creature you control and \"{G}{W}: Enchanted creature gains indestructible until end of turn,\" and it loses all other abilities.";
    let tokens = lex_line(effects, 0).expect("atomic Aura return should lex");
    let bundled = exact_atomic_return_as_aura_bundle(&tokens)
        .expect("typed Aura return should remain one atomic resolution bundle");
    assert!(matches!(
        bundled.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ReturnToBattlefield {
                as_aura: Some(as_aura),
                ..
            },
            ..
        })] if as_aura.remove_all_abilities && as_aura.granted_abilities.len() == 1
    ));

    let near_miss = lex_line(
        "return it to the battlefield. It's an Aura enchantment with enchant creature you control.",
        0,
    )
    .expect("plain Aura return should lex");
    assert!(
        exact_atomic_return_as_aura_bundle(&near_miss).is_none(),
        "an Aura return without both the quoted grant and ability loss must stay on its ordinary route"
    );
}

#[cfg(test)]
#[test]
fn library_origin_source_pump_unblockable_preemption_is_exact() {
    fn is_library_origin(trigger: &TriggerSpec) -> bool {
        match trigger {
            TriggerSpec::WithIntro { trigger, .. } => is_library_origin(trigger),
            TriggerSpec::PutIntoGraveyardFromZone {
                from: Zone::Library,
                one_or_more: true,
                ..
            } => true,
            _ => false,
        }
    }

    let full = "Whenever one or more cards are put into your graveyard from your library, this creature gets +1/+1 until end of turn and can't be blocked this turn.";
    let full_tokens = lex_line(full, 0).expect("exact library-origin line should lex");
    let parsed = parse_library_origin_source_pump_unblockable_triggered_line(&full_tokens)
        .expect("exact library-origin preemption should parse")
        .expect("exact library-origin preemption should claim the line");
    let LineAst::Triggered {
        trigger, effects, ..
    } = parsed
    else {
        panic!("expected one triggered line: {parsed:#?}");
    };
    assert!(is_library_origin(&trigger), "{trigger:#?}");
    assert_eq!(effects.len(), 2, "{effects:#?}");

    let hand_origin = lex_line(
        "Whenever one or more cards are put into your graveyard from your hand, this creature gets +1/+1 until end of turn and can't be blocked this turn.",
        0,
    )
    .expect("lex nonlibrary near miss");
    assert!(
        parse_library_origin_source_pump_unblockable_triggered_line(&hand_origin)
            .expect("near miss should remain parseable")
            .is_none()
    );
}

#[cfg(test)]
#[test]
fn generic_triggered_source_pump_unblockable_keeps_both_effects() {
    let full = "Whenever you cast a noncreature spell, this creature gets +1/+0 until end of turn and can't be blocked this turn.";
    let effects = "this creature gets +1/+0 until end of turn and can't be blocked this turn.";
    let parsed = parse_triggered_text_for_test(full, "you cast a noncreature spell", effects)
        .expect("source pump and unblockable trigger should parse");
    let effects = match &parsed {
        LineAst::Triggered { effects, .. } => effects.as_slice(),
        LineAst::Ability(ability) => ability
            .effects_ast
            .as_deref()
            .expect("runtime-backed trigger should retain its effect AST"),
        _ => panic!("expected one triggered line: {parsed:#?}"),
    };
    assert!(
        matches!(
            effects,
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Pump {
                        power: Value::Fixed(1),
                        toughness: Value::Fixed(0),
                        target: TargetAst::Source(_),
                        duration: Until::EndOfTurn,
                        ..
                    },
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Cant {
                        duration: Until::EndOfTurn,
                        ..
                    },
                    ..
                })
            ]
        ),
        "{effects:#?}"
    );

    let near_miss = parse_triggered_text_for_test(
        "Whenever you cast a noncreature spell, this creature can't be blocked this turn.",
        "you cast a noncreature spell",
        "this creature can't be blocked this turn.",
    )
    .expect("ordinary unblockable trigger should stay parseable");
    let effects = match &near_miss {
        LineAst::Triggered { effects, .. } => effects.as_slice(),
        LineAst::Ability(ability) => ability
            .effects_ast
            .as_deref()
            .expect("runtime-backed trigger should retain its effect AST"),
        _ => panic!("expected one triggered near miss: {near_miss:#?}"),
    };
    assert_eq!(effects.len(), 1, "{effects:#?}");
}

#[cfg(test)]
#[test]
fn exiled_last_counter_qualifier_stays_on_the_trigger_side_of_the_comma() {
    let exact = lex_line(
        "When the last time counter is removed from this card while it's exiled, creatures can't be blocked this turn.",
        0,
    )
    .expect("exiled last-counter trigger should lex");
    let parsed = parse_exiled_last_counter_triggered_line(&exact)
        .expect("exiled last-counter trigger should parse")
        .expect("typed exiled qualifier should be recognized");
    let LineAst::Triggered {
        trigger, effects, ..
    } = parsed
    else {
        panic!("expected one triggered line: {parsed:#?}");
    };
    assert!(
        matches!(
            trigger,
            TriggerSpec::CounterRemovedFrom {
                ref filter,
                counter_type: Some(crate::CounterType::Time),
                last: true,
                ..
            } if filter.source
        ),
        "{trigger:#?}"
    );
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Cant {
                    restriction: crate::effect::Restriction::BeBlocked(filter),
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one creature blocking restriction: {effects:#?}");
    };
    assert_eq!(filter.card_types, vec![CardType::Creature], "{filter:#?}");
    assert!(filter.any_of.is_empty(), "{filter:#?}");

    let near_miss = lex_line(
        "When the last time counter is removed from this card while it's on the battlefield, creatures can't be blocked this turn.",
        0,
    )
    .expect("last-counter near miss should lex");
    assert!(
        parse_exiled_last_counter_triggered_line(&near_miss)
            .expect("near miss should not error")
            .is_none()
    );
}

#[cfg(test)]
#[test]
fn source_spell_surface_repair_does_not_erase_a_zone_change_trigger_arm() {
    let full = "Whenever you cast a white spell or a Plains you control enters, you gain 1 life.";
    let parsed = parse_triggered_text_for_test(
        full,
        "you cast a white spell or a Plains you control enters",
        "you gain 1 life.",
    )
    .expect("cast-or-entry trigger should reach the public semantic route");
    let trigger = match &parsed {
        LineAst::Triggered { trigger, .. } => trigger,
        LineAst::Ability(ability) => ability
            .trigger_spec
            .as_ref()
            .expect("runtime-backed trigger should retain its trigger spec"),
        _ => panic!("expected one triggered line: {parsed:#?}"),
    };
    assert!(
        matches!(
            trigger,
            TriggerSpec::WithIntro { trigger, .. }
                if matches!(trigger.as_ref(), TriggerSpec::Either(_, _))
        ),
        "{trigger:#?}"
    );
}

#[test]
fn semantic_trigger_root_restores_single_target_source_exclusion() {
    fn spell_cast_target(trigger: &TriggerSpec) -> Option<&ObjectFilter> {
        match trigger {
            TriggerSpec::SpellCast {
                filter: Some(filter),
                ..
            } => filter.targets_only_object.as_deref(),
            TriggerSpec::WithIntro { trigger, .. } => spell_cast_target(trigger),
            TriggerSpec::Either(left, right) => {
                spell_cast_target(left).or_else(|| spell_cast_target(right))
            }
            TriggerSpec::AnyOf(branches) => branches.iter().find_map(spell_cast_target),
            _ => None,
        }
    }

    fn line_spell_cast_target(line: &LineAst) -> Option<&ObjectFilter> {
        match line {
            LineAst::Multiple(chunks) => chunks.iter().find_map(line_spell_cast_target),
            LineAst::Triggered { trigger, .. } => spell_cast_target(trigger),
            LineAst::Ability(parsed) => parsed.trigger_spec.as_ref().and_then(spell_cast_target),
            _ => None,
        }
    }

    let parse = |full_text: &str, trigger_text: &str| {
        super::super::util::with_card_source_reference_context(
            "Ivy, Gleeful Spellthief",
            &[CardType::Creature],
            &[Subtype::Faerie, Subtype::Rogue],
            || {
                parse_triggered_text_for_test(
                    full_text,
                    trigger_text,
                    "you may copy that spell. The copy targets Ivy.",
                )
            },
        )
        .expect("semantic triggered line should parse")
    };

    let excluding = parse(
        "Whenever a player casts a spell that targets only a single creature other than Ivy, you may copy that spell. The copy targets Ivy.",
        "a player casts a spell that targets only a single creature other than Ivy",
    );
    let excluding_target = line_spell_cast_target(&excluding)
        .unwrap_or_else(|| panic!("missing nested spell target filter: {excluding:#?}"));
    assert!(excluding_target.other, "{excluding_target:#?}");
    assert_eq!(
        excluding_target.source_surface,
        Some(crate::target::SourceReferenceSurface::ShortName(
            "Ivy".to_string()
        ))
    );

    let ordinary = parse(
        "Whenever a player casts a spell that targets only a single creature, you may copy that spell. The copy targets Ivy.",
        "a player casts a spell that targets only a single creature",
    );
    let ordinary_target = line_spell_cast_target(&ordinary)
        .unwrap_or_else(|| panic!("missing ordinary nested spell target filter: {ordinary:#?}"));
    assert!(!ordinary_target.other, "{ordinary_target:#?}");
}

#[test]
fn triggered_semantic_split_keeps_effect_backed_static_surfaces_in_resolution()
-> Result<(), CardTextError> {
    let linked_entry = parse_triggered_text_for_test(
        "Whenever a creature you control attacks alone, draw a card. Then you may put a creature card with mana value 3 or less from your hand onto the battlefield. It enters tapped and attacking and gains indestructible until end of turn.",
        "a creature you control attacks alone",
        "draw a card. Then you may put a creature card with mana value 3 or less from your hand onto the battlefield. It enters tapped and attacking and gains indestructible until end of turn.",
    )?;
    let linked_entry_debug = format!("{linked_entry:#?}");
    assert!(
        linked_entry_debug.contains("May")
            && linked_entry_debug.contains("battlefield_tapped: true")
            && linked_entry_debug.contains("battlefield_attacking: true")
            && linked_entry_debug.contains("GrantAbilitiesToTarget")
            && linked_entry_debug.contains("Indestructible"),
        "{linked_entry_debug}"
    );
    assert!(
        !linked_entry_debug.contains("StaticAbilities"),
        "the moved object's entry follow-up must not become a source static tail: \
         {linked_entry_debug}"
    );

    let conditional_create = parse_triggered_text_for_test(
        "Whenever you cast an artifact spell, you may pay {2}. If you do, create a 0/0 colorless Construct artifact creature token with \"This token gets +1/+1 for each artifact you control.\"",
        "you cast an artifact spell",
        "you may pay {2}. If you do, create a 0/0 colorless Construct artifact creature token with \"This token gets +1/+1 for each artifact you control.\"",
    )?;
    let conditional_create_debug = format!("{conditional_create:#?}");
    assert!(
        conditional_create_debug.contains("IfResult")
            && conditional_create_debug.contains("CreateToken"),
        "{conditional_create_debug}"
    );
    assert!(
        !conditional_create_debug.contains("StaticAbilities"),
        "the token's quoted rule must not become a source static tail: \
         {conditional_create_debug}"
    );

    for (full_text, trigger_text, effect_text) in [
        (
            "At the beginning of each combat, you may reveal the top card of your library. If you reveal a creature card this way, this creature becomes a copy of that card until end of turn, except it has flying.",
            "at the beginning of each combat",
            "you may reveal the top card of your library. If you reveal a creature card this way, this creature becomes a copy of that card until end of turn, except it has flying.",
        ),
        (
            "Whenever one or more creatures you control are put into exile, you may choose a creature card from among them. Until end of turn, target token you control becomes a copy of it, except it has flying.",
            "one or more creatures you control are put into exile",
            "you may choose a creature card from among them. Until end of turn, target token you control becomes a copy of it, except it has flying.",
        ),
    ] {
        let copy = parse_triggered_text_for_test(full_text, trigger_text, effect_text)?;
        let copy_debug = format!("{copy:#?}");
        assert!(
            copy_debug.contains("BecomeCopy") && copy_debug.contains("Flying"),
            "{copy_debug}"
        );
        assert!(
            !copy_debug.contains("StaticAbilities"),
            "the copy exception must not become a source static tail: {copy_debug}"
        );
    }

    fn contains_stack_copy(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::CopySpell { .. }
                    | crate::cards::builders::SubjectVerbActionAst::CopySpellForEachTarget { .. },
                ..
            })
        ) {
            return true;
        }
        let mut found = false;
        crate::runtime_backend::effect_ast_traversal::for_each_nested_effects(
            effect,
            true,
            |nested| found |= nested.iter().any(contains_stack_copy),
        );
        found
    }

    fn contains_plural_retarget(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::RetargetStackObject {
                    copy_reference_plural: true,
                    ..
                },
                ..
            })
        ) {
            return true;
        }
        let mut found = false;
        crate::runtime_backend::effect_ast_traversal::for_each_nested_effects(
            effect,
            true,
            |nested| found |= nested.iter().any(contains_plural_retarget),
        );
        found
    }

    fn delayed_contains_copy_and_retarget(effect: &EffectAst) -> bool {
        match effect {
            EffectAst::DelayedTriggerThisTurn { effects, .. }
            | EffectAst::DelayedTriggerForDuration { effects, .. } => {
                return effects.iter().any(contains_stack_copy)
                    && effects.iter().any(contains_plural_retarget);
            }
            _ => {}
        }
        let mut found = false;
        crate::runtime_backend::effect_ast_traversal::for_each_nested_effects(
            effect,
            true,
            |nested| {
                found |= nested.iter().any(delayed_contains_copy_and_retarget);
            },
        );
        found
    }

    let leori = parse_triggered_text_for_test(
        "Whenever Leori deals combat damage to a player, choose a planeswalker type. Until end of turn, whenever you activate an ability of a planeswalker of that type, copy that ability. You may choose new targets for the copies.",
        "Leori deals combat damage to a player",
        "choose a planeswalker type. Until end of turn, whenever you activate an ability of a planeswalker of that type, copy that ability. You may choose new targets for the copies.",
    )?;
    let leori_effects = match &leori {
        LineAst::Triggered { effects, .. } => effects.as_slice(),
        LineAst::Ability(parsed) => parsed
            .effects_ast
            .as_deref()
            .expect("Leori's triggered ability should preserve typed effects"),
        other => panic!("expected one Leori triggered ability: {other:#?}"),
    };
    assert!(
        leori_effects.iter().any(delayed_contains_copy_and_retarget),
        "the copied-object retarget must execute inside the delayed trigger: {leori:#?}"
    );

    Ok(())
}

#[test]
fn source_sentence_boundaries_preserve_jointly_parsed_reference_flow() {
    let independent = lex_line(
        "Put a +1/+1 counter on this creature. Each opponent loses 1 life.",
        0,
    )
    .expect("Aatchik-style effects should lex");
    let independent = parse_effect_sentences_preserving_source_boundaries(&independent)
        .expect("Aatchik-style effects should parse");
    assert_eq!(independent.len(), 2, "{independent:#?}");
    assert!(
        independent
            .iter()
            .all(|effect| matches!(effect, EffectAst::SourceSentence { .. })),
        "independent direct sentences should retain their authored boundary: {independent:#?}"
    );
    assert!(
        independent.iter().all(|effect| matches!(
            effect,
            EffectAst::SourceSentence {
                leading_then: false,
                ..
            }
        )),
        "ordinary sentence boundaries must not acquire ordering provenance: {independent:#?}"
    );

    let explicit_then = lex_line(
        "Draw two cards. Then discard a card unless you attacked this turn.",
        0,
    )
    .expect("explicit-then effects should lex");
    let explicit_then = parse_effect_sentences_preserving_source_boundaries(&explicit_then)
        .expect("explicit-then effects should parse");
    let [
        EffectAst::SourceSentence {
            leading_then: false,
            ..
        },
        EffectAst::SourceSentence {
            leading_then: true, ..
        },
    ] = explicit_then.as_slice()
    else {
        panic!("leading Then should be preserved on only the second sentence: {explicit_then:#?}");
    };

    let ordered = lex_line(
        "Starting with you, each player chooses up to five permanents they control. All permanents other than this creature that weren't chosen this way phase out.",
        0,
    )
    .expect("Disciple-style ordered choices should lex");
    let ordered = parse_effect_sentences_preserving_source_boundaries(&ordered)
        .expect("Disciple-style ordered choices should parse");
    let [
        EffectAst::SourceSentence {
            starting_with_controller: true,
            ..
        },
        EffectAst::SourceSentence {
            starting_with_controller: false,
            ..
        },
    ] = ordered.as_slice()
    else {
        panic!("the explicit participant ordering must remain on the first sentence: {ordered:#?}");
    };
    let ordered_single = lex_line(
        "Starting with you, each player chooses up to five permanents they control.",
        0,
    )
    .expect("single-sentence ordered choices should lex");
    let ordered_single = parse_effect_sentences_preserving_source_boundaries(&ordered_single)
        .expect("single-sentence ordered choices should parse");
    assert!(matches!(
        ordered_single.as_slice(),
        [EffectAst::SourceSentence {
            starting_with_controller: true,
            ..
        }]
    ));
    let unordered_single = lex_line("Each player chooses up to five permanents they control.", 0)
        .expect("unordered participant choice should lex");
    let unordered_single = parse_effect_sentences_preserving_source_boundaries(&unordered_single)
        .expect("unordered participant choice should parse");
    assert!(
        !unordered_single.iter().any(|effect| matches!(
            effect,
            EffectAst::SourceSentence {
                starting_with_controller: true,
                ..
            }
        )),
        "an ordinary player loop must not acquire explicit participant ordering: \
         {unordered_single:#?}"
    );

    let full_trigger = lex_line(
        "When this creature enters, starting with you, each player chooses up to five permanents they control. All permanents other than this creature that weren't chosen this way phase out.",
        0,
    )
    .expect("Disciple-style trigger should lex");
    let trigger_effects = lex_line(
        "Each player chooses up to five permanents they control. All permanents other than this creature that weren't chosen this way phase out.",
        0,
    )
    .expect("Disciple-style trigger effects should lex");
    let trigger_clause = lex_line("This creature enters, starting with you", 0)
        .expect("Disciple-style trigger clause should lex");
    let surfaced_trigger = parse_triggered_line(
        test_line_info(
            "When this creature enters, starting with you, each player chooses up to five permanents they control. All permanents other than this creature that weren't chosen this way phase out.",
        ),
        "when this creature enters, starting with you, each player chooses up to five permanents they control. all permanents other than this creature that weren't chosen this way phase out.",
        &full_trigger,
        &trigger_clause,
        &trigger_effects,
        None,
        None,
        None,
        None,
    )
    .expect("Disciple-style trigger should parse through the semantic line path");
    let surfaced_effects = match &surfaced_trigger {
        LineAst::Triggered { effects, .. } => effects.as_slice(),
        LineAst::Ability(parsed) => parsed
            .effects_ast
            .as_deref()
            .expect("the parsed trigger must retain its semantic effects"),
        _ => panic!("Disciple-style line must remain a trigger: {surfaced_trigger:#?}"),
    };
    assert!(
        matches!(
            surfaced_effects,
            [
                EffectAst::SourceSentence {
                    starting_with_controller: true,
                    ..
                },
                EffectAst::SourceSentence {
                    starting_with_controller: false,
                    ..
                }
            ]
        ),
        "the trigger split must not swallow participant ordering: {surfaced_effects:#?}"
    );

    let linked = "Reveal the top card of your library and put that card into your hand. You lose life equal to its mana value.";
    let tokens = lex_line(linked, 0).expect("linked trigger effects should lex");
    let effects = parse_effect_sentences_preserving_source_boundaries(&tokens)
        .expect("linked trigger effects should keep their joint parse");
    assert_eq!(effects.len(), 2, "{effects:#?}");
    assert!(
        effects
            .iter()
            .all(|effect| matches!(effect, EffectAst::SourceSentence { .. })),
        "joint parsing should retain a stable boundary without losing reference flow: {effects:#?}"
    );
}

fn lower_spell_or_activated_ability_x_cost_trigger(
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
    max_triggers_per_turn: Option<u32>,
) -> Result<Option<LineAst>, CardTextError> {
    if semantic_grammar::parse_spell_or_activated_ability_x_cost_trigger_tokens(
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )
    .is_none()
    {
        return Ok(None);
    }

    Ok(Some(LineAst::Triggered {
        trigger: spell_or_activated_ability_x_cost_trigger_spec(),
        effects: parse_effect_sentences_lexed(effect_parse_tokens)?,
        max_triggers_per_turn,
    }))
}

pub(crate) fn parse_special_triggered_line(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if let Some(chunk) = lower_special_rewrite_triggered_head(
        line,
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) =
        lower_special_rewrite_triggered_divvy(line, trigger_parse_tokens, effect_parse_tokens)?
    {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = lower_special_rewrite_triggered_oath(line, trigger_parse_tokens)? {
        return Ok(Some(chunk));
    }
    lower_special_rewrite_triggered_tail(line, trigger_parse_tokens)
}

fn lower_special_rewrite_triggered_head(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if let Some(effects) = exact_target_same_name_graveyard_may_cast_bundle(effect_parse_tokens) {
        return Ok(Some(LineAst::Triggered {
            trigger: parse_trigger_clause_lexed(trigger_parse_tokens)?,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }
    if line.presentation == Some(PresentationLabel::CaseToSolve) {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects: vec![EffectAst::SolveCase],
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::PreviousTurnCreatureEntryDraw)
    ) {
        let trigger = TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any);
        let effects = vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Draw {
                count: Value::Fixed(1),
            },
        )];
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::ObjectEnteredBattlefieldLastTurn(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::You)
                        .other(),
                ),
                if_true: effects,
                if_false: Vec::new(),
            }],
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if let Some(_spec) = semantic_grammar::parse_combat_death_blocked_damage_tokens(
        trigger_parse_tokens,
        effect_parse_tokens,
    ) {
        let trigger = TriggerSpec::ThisDies;
        let effects = parse_effect_sentences_lexed(effect_parse_tokens)?;
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if let Some(chunk) = lower_spell_or_activated_ability_x_cost_trigger(
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
        line.max_triggers_per_turn,
    )? {
        return Ok(Some(chunk));
    }

    if let Some(chunk) = lower_spell_cast_snow_mana_enter_counter_static_chunk(
        trigger_parse_tokens,
        effect_parse_tokens,
        line.intervening_if.as_ref(),
    )? {
        return Ok(Some(chunk));
    }

    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::SecondSpellSuspend)
    ) {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        let triggering_tag = TagKey::from("triggering");
        let triggering_spell = TargetAst::Tagged(triggering_tag.clone(), None);
        let mut suspend_filter = ObjectFilter::default();
        suspend_filter.alternative_cast = Some(crate::filter::AlternativeCastKind::Suspend);
        let effects = vec![
            EffectAst::subject_verb_copy_spell(
                triggering_spell.clone(),
                Value::Fixed(1),
                PlayerAst::Implicit,
                false,
                false,
                Vec::new(),
            ),
            EffectAst::subject_verb_exile(triggering_spell.clone(), false),
            EffectAst::subject_verb_put_counters(
                crate::object::CounterType::Time,
                Value::Fixed(4),
                triggering_spell.clone(),
                None,
                false,
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::Not(Box::new(PredicateAst::TaggedMatches(
                    triggering_tag,
                    suspend_filter,
                ))),
                if_true: vec![EffectAst::subject_verb_grant_abilities_to_target(
                    triggering_spell,
                    vec![GrantedAbilityAst::KeywordAction(KeywordAction::Marker(
                        "suspend",
                    ))],
                    Until::Forever,
                )],
                if_false: Vec::new(),
            },
        ];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_triggered_ability_functional_zones_from_facts(
                &trigger,
                &line.info.semantic_facts.triggered_ability.functional_zones,
            ),
            Some(line.info.raw_line.clone()),
            None,
            line.presentation.as_ref(),
            ReferenceImports::default(),
        ))));
    }

    if semantic_grammar::parse_blocks_or_becomes_blocked_first_strike_tokens(full_parse_tokens)
        .is_some()
    {
        let trigger = TriggerSpec::ThisBecomesBlockedByObject(ObjectFilter::creature());
        let effects = if effect_parse_tokens.is_empty() {
            vec![EffectAst::subject_verb_grant_abilities_to_target(
                TargetAst::Tagged(TagKey::from("triggering"), None),
                vec![GrantedAbilityAst::KeywordAction(KeywordAction::FirstStrike)],
                Until::EndOfTurn,
            )]
        } else {
            parse_effect_sentences_lexed(effect_parse_tokens)?
        };
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    Ok(None)
}

fn lower_special_rewrite_triggered_divvy(
    line: &RewriteTriggeredLine,
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(&line.full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::DifferentNamesLibraryDivvy)
    ) {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::ThisEntersBattlefield {
                origin_condition: None,
            }
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let mut effects = if effect_parse_tokens.is_empty() {
            return Err(CardTextError::InvariantViolation(
                "typed library-divvy trigger is missing carried effect tokens".to_string(),
            ));
        } else {
            let grouped = split_lexed_sentences(effect_parse_tokens)
                .into_iter()
                .take(2)
                .map(|sentence| sentence.to_vec())
                .collect::<Vec<_>>();
            parse_effect_sentences_lexed(&join_sentences_with_period(&grouped))?
        };
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
            vec![Zone::Library],
            TagKey::from("divvy_source"),
        ));
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Opponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        });
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    Ok(None)
}

fn lower_special_rewrite_triggered_oath(
    line: &RewriteTriggeredLine,
    trigger_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(&line.full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::OpponentLandMajoritySearch)
    ) {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any)
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let mut basic_land = ObjectFilter::land().with_supertype(crate::types::Supertype::Basic);
        basic_land.set_explicit_card_noun(true);
        let effects = vec![
            EffectAst::subject_verb_explicit_target_only_for_chooser(
                TargetAst::Player(
                    PlayerFilter::OpponentWithMoreControlledObjectsThan {
                        player: Box::new(PlayerFilter::Active),
                        filter: Box::new(ObjectFilter::land()),
                    },
                    Some(crate::TextSpan::synthetic()),
                ),
                PlayerAst::Active,
            ),
            EffectAst::MayByPlayer {
                player: PlayerAst::Active,
                effects: vec![EffectAst::subject_verb_search_library(
                    basic_land,
                    Zone::Battlefield,
                    PlayerAst::Active,
                    PlayerAst::Active,
                    crate::effect::SearchSelectionMode::Exact,
                    false,
                    None,
                    true,
                    ChoiceCount::exactly(1),
                    None,
                    None,
                    crate::effect::SearchResultReferenceSurface::ThatCard,
                    false,
                    false,
                    false,
                )],
            },
        ];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_triggered_ability_functional_zones_from_facts(
                &trigger,
                &line.info.semantic_facts.triggered_ability.functional_zones,
            ),
            Some(line.info.raw_line.clone()),
            None,
            None,
            ReferenceImports::default(),
        ))));
    }

    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(&line.full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::OpponentCreatureMajorityConsult)
    ) {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any)
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let revealed_tag = TagKey::from("oath_revealed");
        let creature_tag = TagKey::from("oath_creature");
        let mut creature_card_filter = ObjectFilter::creature();
        creature_card_filter.zone = None;
        let effects = vec![
            EffectAst::subject_verb_explicit_target_only_for_chooser(
                TargetAst::Player(
                    PlayerFilter::OpponentWithMoreControlledObjectsThan {
                        player: Box::new(PlayerFilter::Active),
                        filter: Box::new(ObjectFilter::creature()),
                    },
                    Some(crate::TextSpan::synthetic()),
                ),
                PlayerAst::Active,
            ),
            EffectAst::MayByPlayer {
                player: PlayerAst::Active,
                effects: vec![
                    EffectAst::subject_verb_consult_top_of_library(
                        PlayerAst::Active,
                        crate::cards::builders::LibraryConsultModeAst::Reveal,
                        creature_card_filter,
                        crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch,
                        revealed_tag.clone(),
                        creature_tag.clone(),
                    ),
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(creature_tag.clone(), None),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::ForEachTagged {
                        tag: revealed_tag,
                        effects: vec![EffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object(
                                creature_tag.as_str(),
                            ),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                                Zone::Graveyard,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        }],
                    },
                ],
            },
        ];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_triggered_ability_functional_zones_from_facts(
                &trigger,
                &line.info.semantic_facts.triggered_ability.functional_zones,
            ),
            Some(line.info.raw_line.clone()),
            None,
            None,
            ReferenceImports::default(),
        ))));
    }

    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(&line.full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::OpponentGraveyardMinorityReturn)
    ) {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any)
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let mut graveyard_creature_filter = ObjectFilter::creature();
        graveyard_creature_filter.zone = Some(Zone::Graveyard);

        let mut return_filter = graveyard_creature_filter.clone();
        return_filter.owner = Some(PlayerFilter::IteratedPlayer);

        let effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::AnOpponentHasFewerThanPlayer {
                player: PlayerAst::That,
                filter: graveyard_creature_filter,
            },
            if_true: vec![EffectAst::MayByPlayer {
                player: PlayerAst::That,
                effects: vec![EffectAst::subject_verb_return_to_hand(
                    TargetAst::Object(return_filter, None, None),
                    false,
                )],
            }],
            if_false: Vec::new(),
        }];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_triggered_ability_functional_zones_from_facts(
                &trigger,
                &line.info.semantic_facts.triggered_ability.functional_zones,
            ),
            Some(line.info.raw_line.clone()),
            None,
            None,
            ReferenceImports::default(),
        ))));
    }

    Ok(None)
}

fn lower_special_rewrite_triggered_tail(
    line: &RewriteTriggeredLine,
    trigger_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if let Some(
        semantic_grammar::SpecialTriggeredProgram::RandomDiscardCreatureReturnUnlessLife { life },
    ) = semantic_grammar::parse_special_triggered_program_tokens(&line.full_parse_tokens)
    {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::You)
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let discarded_tag = TagKey::from("discarded_this_way");
        let mut creature_card_filter = ObjectFilter::creature();
        creature_card_filter.zone = Some(Zone::Graveyard);
        creature_card_filter.owner = Some(PlayerFilter::You);
        let effects = vec![
            EffectAst::subject_verb_discard(
                PlayerAst::You,
                crate::effect::Value::Fixed(1),
                true,
                false,
                None,
                Some(discarded_tag.clone()),
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::You,
                    tag: discarded_tag.clone(),
                    filter: creature_card_filter,
                    mode: ironsmith_core::TaggedObjectMatchMode::CurrentOrLastKnown,
                },
                if_true: vec![EffectAst::UnlessPays {
                    effects: vec![EffectAst::subject_verb_return_to_battlefield(
                        TargetAst::Tagged(discarded_tag, None),
                        false,
                        false,
                        false,
                        ReturnControllerAst::Preserve,
                        None,
                    )],
                    player: PlayerAst::Any,
                    cost: TotalCost::from_cost(Cost::life(life)),
                    before_delayed_step: false,
                }],
                if_false: Vec::new(),
            },
        ];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_triggered_ability_functional_zones_from_facts(
                &trigger,
                &line.info.semantic_facts.triggered_ability.functional_zones,
            ),
            Some(line.info.raw_line.clone()),
            None,
            None,
            ReferenceImports::default(),
        ))));
    }

    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(&line.full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::OpponentCombatAttackPile)
    ) {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::BeginningOfCombat(PlayerFilter::Opponent)
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let effects = vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::That,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::subject_verb_cant(
                crate::effect::Restriction::attack(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(TagKey::from("divvy_chosen")),
                ),
                Until::EndOfTurn,
                None,
            ),
        ];
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    Ok(None)
}

/// Build the display text for the first-equip-cost alternative static ability.
/// Capitalises the leading "you" and strips the trailing period.
fn capitalize_first_equip_cost_alternative_display(tokens: &[OwnedLexToken]) -> String {
    let rendered = render_token_slice(tokens);
    let s = rendered.trim().trim_end_matches('.');
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub(crate) fn parse_static_line(
    info: LineInfo,
    parse_tokens: &[OwnedLexToken],
    chosen_option: Option<&ChosenOptionContext>,
) -> Result<LineAst, CardTextError> {
    parse_static_line_impl(
        &RewriteStaticLine {
            info,
            parse_tokens: parse_tokens.to_vec(),
            chosen_option: chosen_option.cloned(),
        },
        parse_tokens,
    )
}

fn parse_static_line_impl(
    line: &RewriteStaticLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let chosen_option = line.chosen_option.as_ref();
    if crate::runtime_backend::grammar::abilities::is_cast_as_though_flash_with_next_cleanup_sacrifice_line_lexed(
        parse_tokens,
    ) {
        let sacrifice_source = EffectAst::subject_verb_sacrifice(
            PlayerAst::You,
            ObjectFilter::source(),
            1,
            None,
        );
        return wrap_chosen_option_static_chunk(
            LineAst::Multiple(vec![
                LineAst::StaticAbility(
                    StaticAbility::flash()
                        .with_text("You may cast this spell as though it had flash")
                        .into(),
                ),
                LineAst::Statement {
                    effects: vec![EffectAst::Conditional {
                        predicate: PredicateAst::And(
                            Box::new(PredicateAst::SourceWasCast),
                            Box::new(PredicateAst::Not(Box::new(
                                PredicateAst::ThisSpellWasCastAtSorceryTiming,
                            ))),
                        ),
                        if_true: vec![EffectAst::DelayedUntilNextCleanupStep {
                            player: PlayerFilter::Any,
                            effects: vec![sacrifice_source],
                        }],
                        if_false: Vec::new(),
                    }],
                },
            ]),
            chosen_option,
        );
    }
    if let Some(prototype) =
        crate::runtime_backend::grammar::abilities::parse_prototype_keyword_tokens(parse_tokens)
    {
        return wrap_chosen_option_static_chunk(
            LineAst::Abilities(vec![KeywordAction::Prototype {
                cost: prototype.cost,
                power_toughness: prototype.power_toughness,
            }]),
            chosen_option,
        );
    }
    let source_partner_label =
        crate::runtime_backend::lexer::lex_line(&line.info.raw_line, line.info.line_index)
            .ok()
            .and_then(|tokens| {
                keyword_special_grammar::parse_partner_visible_label_tokens(&tokens)
            });
    if let Some(visible_label) = source_partner_label
        .or_else(|| keyword_special_grammar::parse_partner_visible_label_tokens(&line.parse_tokens))
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::partner().with_text(visible_label).into()),
            chosen_option,
        );
    }
    if let Some(variant) = semantic_grammar::parse_partner_variant_label_tokens(&line.parse_tokens)
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::partner_variant(variant.display).into()),
            chosen_option,
        );
    }
    let special_shape = semantic_grammar::parse_static_special_line_tokens(parse_tokens);
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::BlackManaMayBePaidWithLife)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::krrik_black_mana_may_be_paid_with_life().into()),
            chosen_option,
        );
    }
    if is_minimum_spell_total_mana_three_line_lexed(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::minimum_spell_total_mana(3).into()),
            chosen_option,
        );
    }
    if is_players_cant_pay_life_or_sacrifice_line_lexed(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate().into(),
            ),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::BoastTwice)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::boast_twice_each_turn().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::DraftRule)
    ) {
        let display = render_token_slice(parse_tokens)
            .trim()
            .trim_end_matches('.')
            .to_string();
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::draft_rule_text(display).into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::HiddenAgenda)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::hidden_agenda().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::DoubleAgenda)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::double_agenda().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::AnyNumberNamedDeckConstruction)
    ) {
        let display = render_token_slice(parse_tokens)
            .trim()
            .trim_end_matches('.')
            .to_string();
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::deck_construction_rule_text(display).into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::FirstEquipCostAlternative)
    ) {
        let display = capitalize_first_equip_cost_alternative_display(parse_tokens);
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::first_equip_cost_alternative(display).into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::EquipAtInstantSpeed)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::equip_abilities_any_time().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::AdditionalVoteTime)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::vote_additional_time_while_voting().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::AdditionalVote)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::vote_additional_vote_while_voting().into()),
            chosen_option,
        );
    }
    if let Some(count) = semantic_grammar::parse_additional_land_play_count_tokens(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::additional_land_plays(count).into()),
            chosen_option,
        );
    }
    if let Some(chunk) = try_lower_hideaway_tokens(parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }
    if let Some(chunk) = try_lower_partner_with_tokens(parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }

    let lexed = parse_tokens;
    if let Some(abilities) = crate::runtime_backend::families::keyword_static::
        parse_attached_anthem_reach_shadow_permission_line(lexed)
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbilities(abilities),
            chosen_option,
        );
    }
    if semantic_grammar::parse_level_up_intro_tokens(lexed).is_some() {
        if let Some(level_up) = parse_level_up_line_lexed(&lexed)? {
            return Ok(LineAst::Ability(level_up));
        }
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::DoesntUntap)
    ) {
        let chunk =
            LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
                StaticAbility::doesnt_untap(),
            )]);
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    if let Some(ability) = parse_spell_additional_life_cost_per_target_line(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    if let Some(ability) = parse_spell_cost_increase_per_target_beyond_first_line(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    // A quoted cost modifier is the ability granted by the subject before
    // the quote, not a cost modifier whose spell filter includes that outer
    // subject. The static AST router binds the quoted ability to its grant
    // before the broad cost parser scans the whole line for "spells ... cost".
    // Keep that same precedence at the CST-to-semantic boundary: this is the
    // document path used by ordinary card compilation.
    if lexed.iter().any(|token| token.kind == TokenKind::Quote)
        && let Some(abilities) = parse_static_ability_ast_line_lexed(&lexed)?
    {
        return wrap_chosen_option_static_chunk(LineAst::StaticAbilities(abilities), chosen_option);
    }
    if let Some(abilities) = parse_spell_and_player_activated_ability_cost_modifier_line(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbilities(abilities.into_iter().map(Into::into).collect()),
            chosen_option,
        );
    }
    // Keep a compound spell-cost line intact before the broad single cost
    // modifier parser accepts its left clause and discards the terminal
    // countering restriction. The specialized parser reuses one typed spell
    // filter for both executable static abilities.
    if let Some(abilities) = crate::runtime_backend::families::keyword_static::
        parse_spells_cost_reduction_and_cant_be_countered_line(&lexed)?
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbilities(abilities.into_iter().map(Into::into).collect()),
            chosen_option,
        );
    }
    // Preserve a shared first-spell filter across the coordinated reduction
    // and flash permission before the ordinary cost parser consumes only the
    // left side of the sentence.
    if let Some(abilities) = crate::runtime_backend::families::keyword_static::
        parse_first_spell_cost_reduction_and_flash_line(&lexed)?
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbilities(abilities),
            chosen_option,
        );
    }
    if let Some(ability) = parse_spells_cost_modifier_line(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    if let Some(chunk) = parse_compound_buff_and_unblockable_static_chunk(parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }
    if semantic_grammar::parse_combined_spell_and_activation_tax_tokens(lexed).is_some()
        && let Some(abilities) = parse_static_ability_ast_line_lexed(&lexed)?
    {
        return wrap_chosen_option_static_chunk(LineAst::StaticAbilities(abilities), chosen_option);
    }
    if let Some(ability) =
        crate::runtime_backend::families::keyword_static::parse_double_counters_replacement_line(
            &lexed,
        )?
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    if has_standard_menace_reminder(&line.info.source_tokens)
        && matches!(
            parse_ability_line_lexed(&lexed).as_deref(),
            Some([KeywordAction::Menace])
        )
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::menace()
                    .with_text(STANDARD_MENACE_REMINDER)
                    .into(),
            ),
            chosen_option,
        );
    }
    if has_standard_flanking_reminder(&line.info.raw_line)
        && matches!(
            parse_ability_line_lexed(&lexed).as_deref(),
            Some([KeywordAction::Flanking])
        )
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::flanking()
                    .with_text(STANDARD_FLANKING_REMINDER)
                    .into(),
            ),
            chosen_option,
        );
    }
    if let Some(actions) = semantic_grammar::parse_source_keyword_tail_tokens(lexed)
        .and_then(|tail| parse_ability_line_lexed(tail.ability_tokens))
    {
        return wrap_chosen_option_static_chunk(LineAst::Abilities(actions), chosen_option);
    }
    if let Some(abilities) =
        crate::runtime_backend::families::keyword_static::parse_additional_land_play_line(&lexed)?
    {
        let abilities = abilities
            .into_iter()
            .map(crate::cards::builders::StaticAbilityAst::Static)
            .collect();
        return wrap_chosen_option_static_chunk(LineAst::StaticAbilities(abilities), chosen_option);
    }
    // A complete comma-separated keyword line is one authored ability line,
    // even when an individual keyword (for example cascade) also has a
    // specialized static-ability representation. Keep the group provenance
    // before the broad static parser claims each member independently.
    if let Some(actions) = parse_ability_line_lexed(&lexed)
        && actions.len() > 1
    {
        return wrap_chosen_option_static_chunk(LineAst::Abilities(actions), chosen_option);
    }
    match parse_static_ability_ast_line_lexed(&lexed) {
        Ok(Some(mut abilities)) => {
            restore_copy_static_variant_source_display(&mut abilities, &line.info.raw_line);
            return wrap_chosen_option_static_chunk(
                LineAst::StaticAbilities(abilities),
                chosen_option,
            );
        }
        Ok(None) => {}
        Err(_)
            if parse_tokens
                .iter()
                .any(|token| token.kind == TokenKind::Period) => {}
        Err(err) => return Err(err),
    }
    if semantic_grammar::parse_skip_keyword_action_probe_tokens(parse_tokens).is_none()
        && let Some(actions) = parse_ability_line_lexed(&lexed)
    {
        return wrap_chosen_option_static_chunk(LineAst::Abilities(actions), chosen_option);
    }
    if let Some(chunk) = parse_split_static_chunk(line, parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }
    if semantic_grammar::parse_ability_word_marker_tokens(parse_tokens).is_some() {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::keyword_marker(render_token_slice(parse_tokens).trim().to_string())
                    .into(),
            ),
            chosen_option,
        );
    }
    Err(CardTextError::ParseError(format!(
        "rewrite static lowering could not reconstitute static line '{}'",
        line.info.raw_line
    )))
}

#[test]
fn ability_word_marker_detection_uses_token_kinds() {
    let marker_tokens = lex_line("Landfall", 0).expect("marker should lex");
    assert!(semantic_grammar::parse_ability_word_marker_tokens(&marker_tokens).is_some());

    let sentence_tokens = lex_line(
        "Landfall — Whenever a land enters under your control, draw a card.",
        0,
    )
    .expect("sentence should lex");
    assert!(semantic_grammar::parse_ability_word_marker_tokens(&sentence_tokens).is_none());
}

#[test]
fn additional_land_play_static_count_uses_token_words() {
    let tokens = lex_line(
        "You may play two additional lands on each of your turns.",
        0,
    )
    .expect("lexes");
    assert_eq!(
        semantic_grammar::parse_additional_land_play_count_tokens(&tokens),
        Some(2)
    );

    let non_match = lex_line("You may play an additional land this turn.", 0).expect("lexes");
    assert_eq!(
        semantic_grammar::parse_additional_land_play_count_tokens(&non_match),
        None
    );
}

#[cfg(test)]
pub(crate) fn parse_keyword_line_for_test(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    kind: RewriteKeywordLineKind,
) -> Result<LineAst, CardTextError> {
    parse_keyword_line_with_full_tokens_for_test(info, text, parse_tokens, parse_tokens, kind)
}

#[cfg(test)]
pub(crate) fn parse_keyword_line_with_full_tokens_for_test(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    full_parse_tokens: &[OwnedLexToken],
    kind: RewriteKeywordLineKind,
) -> Result<LineAst, CardTextError> {
    super::super::keyword_registry::parse_keyword_payload_for_kind(
        info,
        text,
        parse_tokens,
        full_parse_tokens,
        kind,
    )
}

#[cfg(test)]
fn test_line_info(raw_line: &str) -> LineInfo {
    LineInfo {
        line_index: 0,
        display_line_index: 0,
        raw_line: raw_line.to_string(),
        source_tokens: lex_line(raw_line, 0).unwrap_or_default(),
        normalized: NormalizedLine {
            original: raw_line.to_string(),
            normalized: raw_line.to_ascii_lowercase(),
            char_map: Vec::new(),
        },
        semantic_facts: Default::default(),
    }
}

#[test]
fn protected_battle_surface_binds_the_pre_lowering_damage_target_inside_opponent_loop() {
    fn battle_damage() -> EffectAst {
        let mut battle = ObjectFilter::default();
        battle.zone = Some(Zone::Battlefield);
        battle.card_types.push(CardType::Battle);
        EffectAst::subject_verb_damage(Value::Fixed(1), TargetAst::Object(battle, None, None))
    }

    fn damage_filter(effect: &EffectAst) -> &ObjectFilter {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::DealDamage {
                    target: TargetAst::Object(filter, None, _),
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected a typed non-targeted Battle damage action: {effect:#?}");
        };
        filter
    }

    let mut effects = vec![EffectAst::ForEachOpponent {
        effects: vec![battle_damage()],
    }];
    bind_protected_battle_iteration_in_effects(&mut effects, false);
    let [EffectAst::ForEachOpponent { effects: nested }] = effects.as_slice() else {
        panic!("expected the opponent loop to remain intact: {effects:#?}");
    };
    assert_eq!(
        damage_filter(&nested[0]).protected_by,
        Some(PlayerFilter::IteratedPlayer)
    );

    let mut outside_loop = vec![battle_damage()];
    bind_protected_battle_iteration_in_effects(&mut outside_loop, false);
    assert_eq!(
        damage_filter(&outside_loop[0]).protected_by,
        None,
        "ordinary each-Battle damage must not acquire an iterated opponent"
    );
}

#[test]
fn standard_menace_reminder_is_typed_without_broad_keyword_expansion() {
    let standard = lex_line(STANDARD_MENACE_REMINDER, 0).expect("standard reminder should lex");
    let bare = lex_line("Menace", 0).expect("bare menace should lex");
    let nonstandard = lex_line(
        "Menace (This creature can't be blocked by only one creature.)",
        0,
    )
    .expect("nonstandard reminder should lex");

    assert!(has_standard_menace_reminder(&standard));
    assert!(!has_standard_menace_reminder(&bare));
    assert!(!has_standard_menace_reminder(&nonstandard));
}

#[test]
fn standard_flanking_reminder_is_typed_without_broad_keyword_expansion() {
    assert!(has_standard_flanking_reminder(STANDARD_FLANKING_REMINDER));
    assert!(!has_standard_flanking_reminder("Flanking"));
    assert!(!has_standard_flanking_reminder(
        "Flanking (Whenever a creature without flanking blocks this creature, it gets -1/-1 until end of turn.)"
    ));
}

#[test]
fn graveyard_copy_cast_accepts_only_the_standard_copy_cast_reminder_suffix() {
    let full = lex_line(
        "Exile up to one target legendary or Rat card from your graveyard and copy it. You may cast the copy. (You still pay its costs. A copy of a permanent spell becomes a token.)",
        0,
    )
    .expect("standard copy-cast reminder should lex");
    let effects = exact_graveyard_card_copy_cast_sequence(&full)
        .expect("the standard reminder suffix should preserve the typed copy-cast sequence");
    let debug = format!("{effects:#?}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("as_copy: true"), "{debug}");
    assert!(
        debug.contains("copy_cast_reminder_surface: true"),
        "{debug}"
    );
    assert!(!debug.contains("CopySpell"), "{debug}");

    let unrelated = lex_line(
        "Exile up to one target legendary or Rat card from your graveyard and copy it. You may cast the copy. You gain 1 life.",
        0,
    )
    .expect("near-miss copy-cast suffix should lex");
    assert!(exact_graveyard_card_copy_cast_sequence(&unrelated).is_none());
}

#[test]
fn graveyard_copy_cast_accepts_conditional_copy_and_one_cast_result_tail() {
    let conditional = lex_line(
        "Exile up to one target Assassin card or card with freerunning from your graveyard. If you do, copy it. You may cast the copy.",
        0,
    )
    .expect("conditional copy-cast sequence should lex");
    let conditional_effects = exact_graveyard_card_copy_cast_sequence(&conditional)
        .expect("the registered conditional copy-cast family should stay typed");
    let conditional_debug = format!("{conditional_effects:#?}");
    assert!(
        conditional_debug.contains("CastTagged"),
        "{conditional_debug}"
    );
    assert!(
        conditional_debug.contains("as_copy: true"),
        "{conditional_debug}"
    );
    assert!(
        conditional_debug.contains("IfResult"),
        "{conditional_debug}"
    );
    assert!(
        !conditional_debug.contains("CopySpell"),
        "{conditional_debug}"
    );

    let with_cast_result = lex_line(
        "Exile up to one target black card from your graveyard and copy it. You may cast the copy. If you do, you lose 2 life.",
        0,
    )
    .expect("copy-cast result sequence should lex");
    let result_effects = exact_graveyard_card_copy_cast_sequence(&with_cast_result)
        .expect("one exact cast-result tail should follow the typed copy-cast prefix");
    let result_debug = format!("{result_effects:#?}");
    assert!(result_debug.contains("CastTagged"), "{result_debug}");
    assert!(result_debug.contains("LoseLife"), "{result_debug}");
    assert!(result_debug.contains("IfResult"), "{result_debug}");
    assert!(!result_debug.contains("CopySpell"), "{result_debug}");

    let wrong_result = lex_line(
        "Exile up to one target black card from your graveyard and copy it. You may cast the copy. If you don't, you lose 2 life.",
        0,
    )
    .expect("wrong-result near miss should lex");
    assert!(exact_graveyard_card_copy_cast_sequence(&wrong_result).is_none());

    let unrelated_tail = lex_line(
        "Exile up to one target black card from your graveyard and copy it. You may cast the copy. You gain 2 life.",
        0,
    )
    .expect("unrelated-tail near miss should lex");
    assert!(exact_graveyard_card_copy_cast_sequence(&unrelated_tail).is_none());
}

#[cfg(test)]
fn test_rewrite_triggered_line(raw_line: &str, full_text: &str) -> RewriteTriggeredLine {
    RewriteTriggeredLine {
        info: test_line_info(raw_line),
        full_text: full_text.to_string(),
        full_parse_tokens: lex_line(full_text, 0).unwrap_or_default(),
        intervening_if: None,
        presentation: None,
        max_triggers_per_turn: Some(1),
        chosen_option: None,
    }
}

#[test]
fn tagged_characteristic_addition_is_a_bound_effect_followup() {
    let tokens = lex_line(
        "Put target artifact onto the battlefield. That permanent is an enchantment in addition to its other types.",
        0,
    )
    .expect("bound characteristic fixture should lex");
    let sentences = split_lexed_sentences(&tokens);
    assert!(sentences_have_bound_characteristic_followup_after_first(
        &sentences
    ));

    let tokens = lex_line(
        "Draw a card. Creatures you control are artifacts in addition to their other types.",
        0,
    )
    .expect("independent static fixture should lex");
    let sentences = split_lexed_sentences(&tokens);
    assert!(!sentences_have_bound_characteristic_followup_after_first(
        &sentences
    ));
}

#[test]
fn triggered_line_source_text_keeps_raw_do_this_only_once_suffix() {
    let raw_line = "Whenever Pantlaza or another Dinosaur you control enters, you may discover X, where X is that creature's toughness. Do this only once each turn.";
    let full_text = "whenever pantlaza or another dinosaur you control enters, you may discover x, where x is that creature's toughness";
    let line = test_rewrite_triggered_line(raw_line, full_text);

    assert_eq!(triggered_line_source_text(&line), raw_line);
}

#[test]
fn triggered_line_source_text_keeps_labelled_raw_do_this_only_once_suffix() {
    let raw_line = "Mold Earth — Whenever one or more lands enter under an opponent's control without being played, you may search your library for a Plains card, put it onto the battlefield tapped, then shuffle. Do this only once each turn.";
    let full_text = "whenever one or more lands enter under an opponent's control without being played, you may search your library for a plains card, put it onto the battlefield tapped, then shuffle";
    let line = test_rewrite_triggered_line(raw_line, full_text);

    assert_eq!(triggered_line_source_text(&line), raw_line);
}

pub(crate) fn normalize_exert_followup_source_reference_tokens(
    source_ref: &str,
    followup_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    semantic_grammar::normalize_exert_followup_source_tokens(source_ref, followup_tokens)
}

pub(crate) fn parse_exert_attack_keyword_line(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let sentence_tokens = split_lexed_sentences(parse_tokens);
    let Some(head_tokens) = sentence_tokens.first().copied() else {
        return Err(CardTextError::ParseError(format!(
            "rewrite keyword lowering could not parse exert attack line '{}'",
            line.info.raw_line
        )));
    };
    let semantic_grammar::ExertAttackHead {
        only_if_not_exerted_this_turn,
        source_ref,
    } = semantic_grammar::parse_exert_attack_head_tokens(head_tokens).map_err(|message| {
        CardTextError::ParseError(format!(
            "rewrite keyword lowering {message} '{}'",
            line.info.raw_line
        ))
    })?;

    let followup = sentence_tokens
        .get(1)
        .and_then(|tokens| semantic_grammar::parse_exert_reflexive_followup_tokens(tokens));
    let linked_trigger = if let Some(followup) = followup {
        let normalized_followup_tokens = normalize_exert_followup_source_reference_tokens(
            source_ref.as_str(),
            followup.effect_tokens,
        );
        let effects_ast = parse_effect_sentences_lexed(&normalized_followup_tokens)?;
        let prepared = rewrite_prepare_effects_with_trigger_context_for_lowering(
            None,
            &effects_ast,
            ReferenceImports::default(),
        )?;
        let lowered = materialize_prepared_effects_with_trigger_context(&prepared)?;
        Some(crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::state_based("When you do"),
            effects: lowered.effects,
            choices: lowered.choices,
            intervening_if: None,
            presentation_label: None,
        })
    } else if sentence_tokens
        .get(1)
        .is_some_and(|tokens| semantic_grammar::parse_when_followup_intro_tokens(tokens))
    {
        return Err(CardTextError::ParseError(format!(
            "rewrite keyword lowering expected exert reflexive followup '{}'",
            line.info.raw_line
        )));
    } else {
        None
    };

    Ok(LineAst::StaticAbility(
        StaticAbility::exert_attack(
            only_if_not_exerted_this_turn,
            linked_trigger,
            line.info.raw_line.clone(),
        )
        .into(),
    ))
}

fn rewrite_copy_count_to_times_paid_label_rewrite(effects: &mut [EffectAst], label: &str) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpell { target, count, .. },
            ..
        }) = effect
            && let crate::cards::builders::TargetAst::Source(_) = target
            && let crate::effect::Value::Count(filter) = count
            && filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == IT_TAG)
        {
            *count = crate::effect::Value::TimesPaidLabel(label.into());
        }
        // Recurse into every nested-effect scope through the shared traversal
        // helper so new wrapper variants are covered automatically (the previous
        // hand-rolled match silently skipped RepeatEffects/ManaRestricted and the
        // newer ChooseOneOf/IfEffectDidNotHappen/TagAffected variants).
        crate::runtime_backend::model::effect_ast_traversal::for_each_nested_effects_mut(
            effect,
            true,
            |nested| rewrite_copy_count_to_times_paid_label_rewrite(nested, label),
        );
    }
}

pub(crate) fn parse_gift_keyword_line(line: &RewriteKeywordLine) -> Result<LineAst, CardTextError> {
    let spec =
        semantic_grammar::parse_standard_gift_spec_tokens(&line.parse_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "rewrite keyword lowering could not parse gift line '{}'",
                line.info.raw_line
            ))
        })?;
    let cost = OptionalCost::custom(
        line.info.raw_line.trim(),
        TotalCost::from_cost(Cost::effect(
            crate::effects::ChoosePlayerEffect::new(
                PlayerFilter::You,
                PlayerFilter::Opponent,
                "gifted_player",
            )
            .remember_as_chosen_player(),
        )),
    );

    Ok(LineAst::GiftKeyword {
        cost: cost.into(),
        effects: standard_gift_effects(spec.variant),
        followup_text: standard_gift_followup_text(spec.variant).to_string(),
        timing: spec.timing,
    })
}

fn standard_gift_followup_text(variant: semantic_grammar::StandardGiftVariant) -> &'static str {
    match variant {
        semantic_grammar::StandardGiftVariant::Card => "the chosen player draws a card.",
        semantic_grammar::StandardGiftVariant::Treasure => {
            "the chosen player creates a Treasure token."
        }
        semantic_grammar::StandardGiftVariant::Food => "the chosen player creates a Food token.",
        semantic_grammar::StandardGiftVariant::TappedFish => {
            "the chosen player creates a tapped 1/1 blue Fish creature token."
        }
        semantic_grammar::StandardGiftVariant::ExtraTurn => {
            "the chosen player takes an extra turn after this one."
        }
        semantic_grammar::StandardGiftVariant::Octopus => {
            "the chosen player creates an 8/8 blue Octopus creature token."
        }
    }
}

fn standard_gift_effects(variant: semantic_grammar::StandardGiftVariant) -> Vec<EffectAst> {
    match variant {
        semantic_grammar::StandardGiftVariant::Card => vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Chosen,
            SubjectVerbActionAst::Draw {
                count: crate::effect::Value::Fixed(1),
            },
        )],
        semantic_grammar::StandardGiftVariant::Treasure => {
            vec![standard_gift_create_token_effect(
                "Treasure",
                crate::runtime_backend::token_definition::TokenDefinitionSpec::Builtin(
                    crate::runtime_backend::token_definition::BuiltinTokenShape::Treasure,
                ),
                false,
            )]
        }
        semantic_grammar::StandardGiftVariant::Food => {
            vec![standard_gift_create_token_effect(
                "Food",
                crate::runtime_backend::token_definition::TokenDefinitionSpec::Builtin(
                    crate::runtime_backend::token_definition::BuiltinTokenShape::Food,
                ),
                false,
            )]
        }
        semantic_grammar::StandardGiftVariant::TappedFish => {
            vec![standard_gift_create_token_effect(
                "1/1 blue Fish creature",
                fixed_standard_gift_creature_definition(
                    "Fish",
                    Subtype::Fish,
                    ColorSet::BLUE,
                    (1, 1),
                ),
                true,
            )]
        }
        semantic_grammar::StandardGiftVariant::ExtraTurn => {
            vec![EffectAst::subject_verb_extra_turn_after_turn(
                PlayerAst::Chosen,
                crate::cards::builders::ExtraTurnAnchorAst::CurrentTurn,
            )]
        }
        semantic_grammar::StandardGiftVariant::Octopus => vec![standard_gift_create_token_effect(
            "8/8 blue Octopus creature",
            fixed_standard_gift_creature_definition(
                "Octopus",
                Subtype::Octopus,
                ColorSet::BLUE,
                (8, 8),
            ),
            false,
        )],
    }
}

fn fixed_standard_gift_creature_definition(
    name: &str,
    subtype: Subtype,
    colors: ColorSet,
    power_toughness: (i32, i32),
) -> crate::runtime_backend::token_definition::TokenDefinitionSpec {
    crate::runtime_backend::token_definition::TokenDefinitionSpec::Creature(
        crate::runtime_backend::token_definition::CreatureTokenShape {
            name: name.to_string(),
            card_types: vec![CardType::Creature],
            subtypes: vec![subtype],
            power_toughness,
            legendary: false,
            colors,
            use_source_chosen_color: false,
            use_source_chosen_creature_type: false,
            keywords: Vec::new(),
            rules: Default::default(),
        },
    )
}

fn standard_gift_create_token_effect(
    name: &str,
    definition: crate::runtime_backend::token_definition::TokenDefinitionSpec,
    tapped: bool,
) -> EffectAst {
    EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Chosen,
        SubjectVerbActionAst::CreateTokenWithMods {
            name: name.to_string(),
            definition,
            count: crate::effect::Value::Fixed(1),
            dynamic_power_toughness: None,
            player: PlayerAst::Chosen,
            actor_surface_explicit: false,
            attached_to: None,
            tapped,
            attacking: false,
            attack_target_player: None,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            next_end_step_player: PlayerFilter::Any,
            granted_abilities: Vec::new(),
            ability_presentation: None,
        },
    )
}

pub(crate) fn parse_keyword_special_cases(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if let Some(chunk) = try_lower_hideaway_keyword(parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_partner_variant_keyword(line, parse_tokens) {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_partner_with_tokens(parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_parse_optional_cost_with_cast_trigger(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_parse_chosen_type_behold_two_additional_cost(line, parse_tokens) {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_parse_optional_behold_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_parse_optional_waterbend_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    Ok(None)
}

fn try_parse_chosen_type_behold_two_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Option<LineAst> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost
        || token_word_refs(parse_tokens)
            != [
                "as", "an", "additional", "cost", "to", "cast", "this", "spell", "you", "may",
                "choose", "a", "creature", "type", "and", "behold", "two", "cards", "of", "that",
                "type",
            ]
    {
        return None;
    }

    let mut battlefield = ObjectFilter::creature()
        .controlled_by(PlayerFilter::You)
        .in_zone(Zone::Battlefield);
    battlefield.chosen_creature_type = true;
    let mut hand = ObjectFilter::default()
        .owned_by(PlayerFilter::You)
        .in_zone(Zone::Hand);
    hand.chosen_creature_type = true;
    let mut behold = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter {
            any_of: vec![battlefield, hand],
            ..Default::default()
        },
        crate::effect::ChoiceCount::exactly(2),
        PlayerFilter::You,
        "beheld_chosen_type",
    )
    .in_zones(vec![Zone::Battlefield, Zone::Hand])
    .with_description("Behold two cards of the chosen type");
    behold.reveal = true;

    let total_cost = TotalCost::from_costs(vec![
        Cost::validated_effect(crate::effect::Effect::choose_creature_type(
            PlayerFilter::You,
            vec![],
        )),
        Cost::validated_effect(crate::effect::Effect::new(behold)),
    ]);
    let mut optional_cost = OptionalCost::custom(line.info.raw_line.trim(), total_cost);
    optional_cost.reference = crate::cost::OptionalCostRef::new(
        crate::cost::OptionalCostKind::Additional,
    );
    Some(LineAst::OptionalCost(optional_cost.into()))
}

pub(crate) fn try_parse_optional_waterbend_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let Some(generic) = semantic_grammar::parse_optional_waterbend_generic_tokens(parse_tokens)
    else {
        return Ok(None);
    };

    let total_cost =
        crate::runtime_backend::lowering::compile_support::waterbend_optional_total_cost(generic);
    Ok(Some(LineAst::OptionalCost(
        OptionalCost::custom(line.info.raw_line.trim(), total_cost).into(),
    )))
}

fn try_lower_partner_variant_keyword(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Option<LineAst> {
    let visible_tokens = if line.full_parse_tokens.is_empty() {
        parse_tokens
    } else {
        line.full_parse_tokens.as_slice()
    };
    let variant = semantic_grammar::parse_partner_variant_label_tokens(visible_tokens)?;
    Some(LineAst::StaticAbility(
        StaticAbility::partner_variant(variant.display).into(),
    ))
}

fn try_lower_hideaway_keyword(
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    try_lower_hideaway_tokens(parse_tokens)
}

fn try_lower_hideaway_tokens(
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let Some(shape) = semantic_grammar::parse_hideaway_keyword_tokens(parse_tokens)? else {
        return Ok(None);
    };
    Ok(Some(hideaway_line_ast(shape.count)))
}

fn hideaway_line_ast(count: i32) -> LineAst {
    let looked_tag = TagKey::from("hideaway_looked");
    let chosen_tag = TagKey::from("hideaway_exiled");
    let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
    choose_filter.zone = Some(Zone::Library);

    LineAst::Triggered {
        trigger: TriggerSpec::ThisEntersBattlefield {
            origin_condition: None,
        },
        effects: vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::You,
                crate::effect::Value::Fixed(count),
                looked_tag.clone(),
            ),
            EffectAst::ChooseObjects {
                filter: choose_filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
            },
            EffectAst::subject_verb_exile(TargetAst::Tagged(chosen_tag.clone(), None), true),
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                looked_tag,
                Some(chosen_tag),
                LibraryBottomOrderAst::Random,
                PlayerAst::You,
            ),
        ],
        max_triggers_per_turn: None,
    }
}

#[test]
fn hideaway_special_case_uses_parse_tokens() {
    let tokens = lex_line("Hideaway 5.", 0).expect("hideaway should lex");
    assert!(
        try_lower_hideaway_tokens(&tokens)
            .expect("hideaway should lower")
            .is_some()
    );

    let non_numeric = lex_line("Hideaway X.", 0).expect("hideaway should lex");
    assert!(try_lower_hideaway_tokens(&non_numeric).is_err());

    let reminder = lex_line("Hideaway 5 reminder", 0).expect("hideaway should lex");
    assert!(
        try_lower_hideaway_tokens(&reminder)
            .expect("extra words should not match the closed-form special case")
            .is_none()
    );
}

fn try_lower_partner_with_tokens(
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let Some(partner_name) = partner_with_name_from_tokens(parse_tokens) else {
        return Ok(None);
    };

    let mut filter = ObjectFilter::default();
    filter.name = Some(partner_name.clone());

    Ok(Some(LineAst::Multiple(vec![
        LineAst::StaticAbility(StaticAbility::partner_with(partner_name.clone()).into()),
        LineAst::Triggered {
            trigger: TriggerSpec::ThisEntersBattlefield {
                origin_condition: None,
            },
            effects: vec![EffectAst::MayByPlayer {
                player: PlayerAst::Target,
                effects: vec![EffectAst::subject_verb_search_library(
                    filter,
                    Zone::Hand,
                    PlayerAst::Target,
                    PlayerAst::Target,
                    crate::effect::SearchSelectionMode::Exact,
                    true,
                    Some(crate::effect::SearchResultReferenceSurface::ThatCard),
                    true,
                    ChoiceCount::up_to(1),
                    None,
                    None,
                    crate::effect::SearchResultReferenceSurface::ThatCard,
                    false,
                    false,
                    false,
                )],
            }],
            max_triggers_per_turn: None,
        },
    ])))
}

fn partner_with_name_from_tokens(tokens: &[OwnedLexToken]) -> Option<String> {
    keyword_special_grammar::parse_partner_with_name_tokens(tokens)
}

#[test]
fn partner_name_and_visible_label_trim_on_lexed_reminder_tokens() {
    let partner_with_tokens = lex_line(
        "Partner with Toothy, Imaginary Friend (When this creature enters...)",
        0,
    )
    .expect("partner-with line should lex");
    assert_eq!(
        partner_with_name_from_tokens(&partner_with_tokens).as_deref(),
        Some("Toothy, Imaginary Friend")
    );

    let partner_label_tokens = lex_line(
        "Partner - Friends forever (You can have two commanders.)",
        0,
    )
    .expect("partner label should lex");
    assert_eq!(
        keyword_special_grammar::parse_partner_visible_label_tokens(&partner_label_tokens)
            .as_deref(),
        Some("Partner - Friends forever")
    );
}

pub(crate) fn try_parse_optional_cost_with_cast_trigger(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let Some(shape) =
        keyword_special_grammar::parse_optional_cost_with_cast_trigger_tokens(parse_tokens)
    else {
        return Ok(None);
    };

    let head_effects = parse_effect_sentences_lexed(shape.optional_cost_effect_tokens)?;
    let [
        EffectAst::ChooseObjects {
            filter,
            count,
            player,
            ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: sacrificed_player,
                    ..
                },
            action:
                SubjectVerbActionAst::SacrificeAll {
                    filter: sacrificed_filter,
                },
        }),
    ] = head_effects.as_slice()
    else {
        return Ok(None);
    };
    if *player != crate::cards::builders::PlayerAst::Implicit
        || *sacrificed_player != crate::cards::builders::PlayerAst::Implicit
        || count.min != 1
        || count.max.is_some()
        || !matches!(sacrificed_filter, crate::target::ObjectFilter { tagged_constraints, .. } if tagged_constraints.iter().any(|constraint| constraint.tag.as_str() == IT_TAG))
    {
        return Ok(None);
    }

    let head_words = token_word_refs(shape.label_tokens);
    let label = format!(
        "As an additional cost to cast this spell, {}",
        head_words.join(" ")
    );
    let cost = OptionalCost::custom(
        label.clone(),
        TotalCost::from_cost(Cost::sacrifice(filter.clone())),
    )
    .repeatable();
    let mut effects = parse_effect_sentences_lexed(shape.followup_effect_tokens)?;
    rewrite_copy_count_to_times_paid_label_rewrite(&mut effects, &label);
    let followup_words = token_word_refs(shape.followup_effect_tokens);

    Ok(Some(LineAst::OptionalCostWithCastTrigger {
        cost: cost.into(),
        effects,
        followup_text: format!("When you do, {}", followup_words.join(" ")),
    }))
}

pub(crate) fn try_parse_optional_behold_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let Some(shape) =
        keyword_special_grammar::parse_optional_keyword_additional_cost_tokens(parse_tokens)
    else {
        return Ok(None);
    };

    let total_cost = parse_activation_cost(shape.cost_tokens)?;
    if total_cost.mana_cost().is_some() || total_cost.costs().len() != 1 {
        return Ok(None);
    }

    let mut optional_cost = OptionalCost::custom(line.info.raw_line.trim(), total_cost);
    if let Some(subtype) = shape.behold_subtype {
        optional_cost.reference = crate::cost::OptionalCostRef::with_discriminator(
            crate::cost::OptionalCostKind::Behold,
            subtype.to_string(),
        );
    }

    Ok(Some(LineAst::OptionalCost(optional_cost.into())))
}

pub(crate) fn rewrite_modal_to_parsed_item(
    modal: RewriteModalBlock,
) -> Result<ParsedCardItem, CardTextError> {
    let Some(mut header) = parse_modal_header(&modal.header, &modal.header_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "rewrite modal lowering could not parse modal header '{}'",
            modal.header.raw_line
        )));
    };

    if let Some(replacement) = header.x_replacement.as_ref() {
        replace_modal_header_x_in_effects_ast(
            &mut header.common_prefix_effects_ast,
            replacement,
            header.line_text.as_str(),
        )?;
    }

    let mut modes = Vec::with_capacity(modal.modes.len());
    for mode in modal.modes {
        let mut effects_ast = mode.effects_ast;
        if let Some(replacement) = header.x_replacement.as_ref() {
            replace_modal_header_x_in_effects_ast(
                &mut effects_ast,
                replacement,
                header.line_text.as_str(),
            )?;
        }
        modes.push(ParsedModalModeAst {
            info: mode.info,
            description: mode.text,
            point_cost: mode.point_cost,
            additional_mana_cost: mode.additional_mana_cost,
            effects_ast,
        });
    }

    specialize_modal_common_target_suffix(
        &mut modes,
        &header.common_suffix_effects_ast,
        header.line_text.as_str(),
    )?;

    Ok(ParsedCardItem::Modal(ParsedModalAst { header, modes }))
}

/// Specialize a demonstrative modal-header suffix into every bare target
/// mode. The shared clause supplies zone/controller facts, while each bullet
/// supplies its own target characteristic. Each resulting mode therefore
/// owns a complete executable target action, and the modal model separately
/// records that its trailing action was authored only once.
fn specialize_modal_common_target_suffix(
    modes: &mut [ParsedModalModeAst],
    suffix: &[EffectAst],
    header_text: &str,
) -> Result<(), CardTextError> {
    if suffix.is_empty() {
        return Ok(());
    }
    let [EffectAst::SubjectVerb(common)] = suffix else {
        return Err(CardTextError::ParseError(format!(
            "unsupported modal common suffix in '{header_text}'"
        )));
    };
    let SubjectVerbActionAst::ReturnToHand {
        target: TargetAst::Object(common_filter, _, _),
        ..
    } = &common.action
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported modal common target action in '{header_text}'"
        )));
    };

    for mode in modes {
        let [EffectAst::SubjectVerb(mode_target)] = mode.effects_ast.as_slice() else {
            return Err(CardTextError::ParseError(format!(
                "modal common target suffix requires one bare target per mode in '{header_text}'"
            )));
        };
        let SubjectVerbActionAst::TargetOnly {
            target: TargetAst::Object(mode_filter, target_span, object_span),
            ..
        } = &mode_target.action
        else {
            return Err(CardTextError::ParseError(format!(
                "modal common target suffix requires object target modes in '{header_text}'"
            )));
        };

        let mut specialized = common.clone();
        let SubjectVerbActionAst::ReturnToHand { target, .. } = &mut specialized.action else {
            unreachable!("common suffix action was validated above");
        };
        *target = TargetAst::Object(
            merge_filters(common_filter, mode_filter),
            target_span.clone(),
            object_span.clone(),
        );
        mode.effects_ast = vec![EffectAst::SubjectVerb(specialized)];
    }
    Ok(())
}
