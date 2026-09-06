use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::{
    CardTextError, ChooseOneModeAst, EffectAst, GrantedAbilityAst, KeywordAction, ObjectRefAst,
    OwnedLexToken, PlayerAst, PredicateAst, StaticAbilityAst, SubjectAst, SubjectVerbActionAst,
    SubjectVerbRoleAst, TagKey, TargetAst, GrantActionAst, TokenActionAst, DelayedEffectAst, ObjectChoiceEffectAst, ConditionalEffectAst,
};
use crate::color::ColorSet;
use crate::effect::Value;
use crate::model::CompilerStaticAbilityCore as StaticAbility;
use crate::recognition::{ParseOutcome, RuleId};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use ironsmith_core::ValueSurfaceHint;

use super::super::grammar::effects as creation_grammar;
use super::super::grammar::lowering_surfaces::parse_prior_created_token_reference_words;
use super::super::grammar::primitives as grammar;
use super::super::grammar::structure::parse_who_player_predicate_lexed;
use super::super::grammar::token_definitions as token_definition_grammar;
use super::super::keyword_static::parse_value_binding_clause;
use super::super::lexer::{TokenKind, render_token_slice, token_word_refs};
use super::super::object_filters::parse_object_filter;
use super::super::util::{
    parse_target_phrase, parse_value, span_from_tokens, trim_commas, value_contains_unbound_x,
};
use super::clause_pattern_helpers::extract_subject_player;
use super::dispatch_entry::{target_references_it, with_where_x_surface_hints};
use creation_grammar::{CreationPhrase as CreatePhrase, CreationWordClass as CreateWord};

fn parse_create_value_binding(tokens: &[OwnedLexToken]) -> Result<Option<Value>, CardTextError> {
    let span = span_from_tokens(tokens);
    let resolve = |registry: &'static str, values: Vec<(&'static str, Option<Value>)>| {
        let mut candidates = Vec::new();
        for (id, value) in values {
            let Some(value) = value else { continue };
            if candidates
                .iter()
                .any(|candidate: &RegistryCandidate<Value>| candidate.value == value)
            {
                continue;
            }
            candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(RuleId::new(id), HeadDiscriminator::grammar(id)),
                value,
                span,
            ));
        }
        match resolve_registry_candidates(RuleId::new(registry), candidates, Vec::new()) {
            ParseOutcome::Match(matched) => Ok(Some(matched.value.value)),
            ParseOutcome::NoMatch => Ok(None),
            ParseOutcome::Error(diagnostic) => Err(diagnostic.into_card_text_error()),
        }
    };

    let static_ability_count =
        crate::keyword_static::parse_where_x_is_number_of_filter_value(tokens)
            .filter(|value| matches!(value.unhinted(), Value::StaticAbilitiesAmong { .. }));
    let specific = resolve(
        "create-value-binding-specific-registry",
        vec![
            ("create-count-static-abilities", static_ability_count),
            (
                "create-count-relative-aggregate",
                crate::keyword_static::parse_where_x_is_aggregate_filter_value(tokens),
            ),
            (
                "create-count-turn-history",
                crate::grammar::shared_util::value_semantics::parse_turn_history_value_binding(
                    tokens,
                ),
            ),
            (
                "create-count-relative-players",
                crate::grammar::values::parse_players_who_control_more_than_you_value_lexed(tokens),
            ),
        ],
    )?;
    if specific.is_some() {
        return Ok(specific);
    }

    // The typed value-expression grammar owns every `where X` amount it can
    // prove (it resolves hand counts, trigger references, and arithmetic to
    // their canonical typed values). The broad object-filter count covers
    // only the remainder, so the two candidates are structurally disjoint.
    let value_expression = parse_value_binding_clause(tokens);
    let object_filter_count = if value_expression.is_none() {
        crate::keyword_static::parse_where_x_is_number_of_filter_value(tokens)
    } else {
        None
    };
    resolve(
        "create-value-binding-generic-registry",
        vec![
            ("create-count-object-filter", object_filter_count),
            ("create-count-value-expression", value_expression),
        ],
    )
}

fn reject_lossy_for_each_fallback(
    tokens: &[OwnedLexToken],
    full_clause_words: &[&str],
) -> Result<(), CardTextError> {
    creation_grammar::validate_creation_count_fallback_tokens(tokens, full_clause_words)
}

fn replace_dynamic_construct_pt_definition_placeholder(tokens: &mut [OwnedLexToken]) -> bool {
    for token in tokens {
        let Some(power_toughness) = creation_grammar::parse_pt_word(token.parser_text()) else {
            continue;
        };
        let contains_x = matches!(power_toughness.power, creation_grammar::PtComponent::X)
            || matches!(power_toughness.toughness, creation_grammar::PtComponent::X);
        if contains_x {
            return token.replace_word("0/0");
        }
    }
    false
}

pub fn is_probable_token_name_word(word: &str) -> bool {
    if !word
        .chars()
        .all(|ch| ch.is_ascii_alphabetic() || ch == '\'' || ch == '-')
    {
        return false;
    }
    !matches!(
        word,
        "legendary"
            | "artifact"
            | "enchantment"
            | "creature"
            | "token"
            | "tokens"
            | "white"
            | "blue"
            | "black"
            | "red"
            | "green"
            | "colorless"
    )
}

pub fn parse_copy_modifiers_from_tail(
    tail_words: &[&str],
) -> Result<
    (
        Option<ColorSet>,
        Option<Vec<CardType>>,
        Option<Vec<Subtype>>,
        Vec<CardType>,
        Vec<Subtype>,
        Vec<Supertype>,
        Option<(i32, i32)>,
        bool,
        Option<u32>,
        Vec<StaticAbility>,
        bool,
    ),
    CardTextError,
> {
    let parsed = creation_grammar::parse_copy_modifier_words(tail_words)?;
    Ok((
        parsed.set_colors,
        parsed.set_card_types,
        parsed.set_subtypes,
        parsed.added_card_types,
        parsed.added_subtypes,
        parsed.removed_supertypes,
        parsed.set_base_power_toughness,
        parsed.set_base_power_toughness_to_source_totals,
        parsed.starting_loyalty,
        parsed.granted_abilities,
        parsed.loses_soulbond,
    ))
}

pub fn parse_next_end_step_token_delay_flags(tail_words: &[&str]) -> (bool, bool, PlayerFilter) {
    super::super::util::parse_next_end_step_token_delay_flags(tail_words)
}

pub fn trailing_create_at_next_end_step_clause(
    tail_words: &[&str],
) -> Option<(usize, PlayerFilter)> {
    creation_grammar::parse_trailing_create_delay_words(tail_words)
        .map(|spec| (spec.start_word, spec.player))
}

fn parse_create_equal_to_dynamic_count(
    tail_tokens: &[OwnedLexToken],
) -> Result<Option<(Value, usize)>, CardTextError> {
    let Some(spec) = creation_grammar::parse_equal_to_count_clause_tokens(tail_tokens) else {
        return Ok(None);
    };
    let references_prior_result = spec
        .value_tokens
        .iter()
        .any(|token| token.is_word("result"));
    let mut synthetic_tokens = super::super::lexer::synthetic_word_tokens(["where", "x", "is"]);
    synthetic_tokens.extend_from_slice(spec.value_tokens);
    Ok(parse_create_value_binding(&synthetic_tokens)?.map(|value| {
        let value = if references_prior_result {
            value.with_surface_hint(ValueSurfaceHint::PriorEffectResult)
        } else {
            value
        };
        (
            value.with_surface_hint(ValueSurfaceHint::EqualTo),
            spec.cut_token,
        )
    }))
}

fn double_quoted_rule_bodies(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    let mut bodies = Vec::new();
    let mut open = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Quote {
            continue;
        }
        if let Some(start) = open.take() {
            if start < idx {
                bodies.push(&tokens[start..idx]);
            }
        } else {
            open = Some(idx + 1);
        }
    }
    // Preprocessed Oracle text can retain an opening quote while folding the
    // final quote into the sentence terminator. The remainder of that same
    // physical sentence is still the authored rule body; retaining it here
    // prevents a later quoted rule in an inline token list from disappearing.
    if let Some(start) = open
        && start < tokens.len()
    {
        bodies.push(&tokens[start..]);
    }
    bodies
}

/// Return the final authored sentence that actually creates a token, but only
/// when that same sentence contains a quoted rule after the create verb.
///
/// The public lowering boundary can retain an entire physical line here. A
/// later sentence such as `Those creatures have "..."` grants an ability to
/// the already-created set; it is not an inline token-blueprint rule. Keeping
/// this sentence boundary prevents that grant from also being copied into the
/// token definition.
fn inline_quoted_token_creation_sentence(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let mut sentence_start = 0usize;
    let mut inside_quote = false;
    let mut saw_create = false;
    let mut saw_quote_after_create = false;
    let mut last_create_sentence = None;

    for (idx, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            if !inside_quote && saw_create {
                saw_quote_after_create = true;
            }
            inside_quote = !inside_quote;
            continue;
        }
        if inside_quote {
            continue;
        }
        if token.is_word("create") || token.is_word("creates") {
            saw_create = true;
        }
        if token.kind == TokenKind::Period {
            if saw_create {
                last_create_sentence = Some((sentence_start, idx + 1, saw_quote_after_create));
            }
            sentence_start = idx + 1;
            saw_create = false;
            saw_quote_after_create = false;
        }
    }

    if saw_create {
        last_create_sentence = Some((sentence_start, tokens.len(), saw_quote_after_create));
    }
    let (start, end, has_inline_quote) = last_create_sentence?;
    has_inline_quote.then_some(&tokens[start..end])
}

fn tokens_outside_double_quoted_rules(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut in_quote = false;
    tokens
        .iter()
        .filter_map(|token| {
            if token.kind == TokenKind::Quote {
                in_quote = !in_quote;
                return None;
            }
            (!in_quote).then(|| token.clone())
        })
        .collect()
}

fn parse_unquoted_token_dynamic_power_toughness(
    tokens: &[OwnedLexToken],
) -> Option<(Value, Value)> {
    let mut in_quote = false;
    let unquoted = tokens
        .iter()
        .filter_map(|token| {
            if token.kind == TokenKind::Quote {
                in_quote = !in_quote;
                return None;
            }
            (!in_quote).then(|| token.clone())
        })
        .collect::<Vec<_>>();
    (0..unquoted.len()).find_map(|start| {
        token_definition_grammar::parse_token_dynamic_power_toughness_tokens(&unquoted[start..])
    })
}

fn parse_quoted_token_dynamic_power_toughness(tokens: &[OwnedLexToken]) -> Option<(Value, Value)> {
    double_quoted_rule_bodies(tokens)
        .into_iter()
        .find_map(token_definition_grammar::parse_token_dynamic_power_toughness_tokens)
}

fn quoted_copy_sacrifice_ability_surface(
    tokens: &[OwnedLexToken],
) -> Option<crate::model::ast::TokenCopySacrificeAbilitySurface> {
    use crate::model::ast::{
        TokenCopySacrificeAbilitySurface, TokenCopySacrificeEndStepSurface,
        TokenCopySacrificeSubjectSurface,
    };

    double_quoted_rule_bodies(tokens)
        .into_iter()
        .find_map(|body| {
            let facts = token_definition_grammar::parse_token_reminder_facts_tokens(body);
            if !facts.sacrifice_at_next_end_step {
                return None;
            }
            let words = token_word_refs(body);
            let end_step = if words
                .windows(3)
                .any(|window| window == ["of", "your", "end"])
            {
                TokenCopySacrificeEndStepSurface::Your
            } else {
                TokenCopySacrificeEndStepSurface::The
            };
            let subject = if words
                .windows(2)
                .any(|window| window == ["this", "permanent"])
            {
                TokenCopySacrificeSubjectSurface::Permanent
            } else {
                TokenCopySacrificeSubjectSurface::Token
            };
            Some(TokenCopySacrificeAbilitySurface { end_step, subject })
        })
}

fn append_inline_token_embedded_rule(
    definition: &mut crate::model::token_definition::TokenDefinitionSpec,
    rule_tokens: &[OwnedLexToken],
) -> bool {
    use crate::model::token_definition::TokenDefinitionSpec;

    let (name, rules) = match definition {
        TokenDefinitionSpec::Creature(creature) => {
            (&creature.name, &mut creature.rules.token_rules)
        }
        TokenDefinitionSpec::Artifact(artifact) => (&artifact.name, &mut artifact.token_rules),
        _ => return false,
    };
    let Some(rule) = crate::grammar::token_definitions::parse_embedded_token_rule_tokens(
        rule_tokens,
        Some(name),
    ) else {
        return false;
    };
    if rules
        .embedded_rules
        .iter()
        .any(|existing| existing == &rule)
    {
        false
    } else {
        rules.embedded_rules.push(rule);
        true
    }
}

fn quoted_rule_creates_a_nested_token(tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(tokens);
    crate::word_primitives::parse_sequence_start(&words, &["create"]).is_some_and(|create| {
        words[create + 1..]
            .iter()
            .any(|word| matches!(*word, "token" | "tokens"))
    })
}

/// Return the complete rules list from an authored token pronoun clause when
/// that list mixes ordinary and quoted abilities. Parsing only the quoted
/// body loses siblings such as `indestructible` and a trailing `equip`;
/// parsing the complete list lets the ordinary grant grammar preserve all
/// three typed abilities and their source order.
pub fn mixed_pronoun_token_rule_list(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let start = tokens.iter().enumerate().find_map(|(index, token)| {
        ((token.is_word("it")
            && tokens
                .get(index + 1)
                .is_some_and(|next| next.is_word("has")))
            || (token.is_word("they")
                && tokens
                    .get(index + 1)
                    .is_some_and(|next| next.is_word("have"))))
        .then_some(index)
    })?;
    let rules = tokens.get(start + 2..)?;
    let open =
        crate::slice_primitives::select_position(rules, |token| token.kind == TokenKind::Quote)?;
    let close = rules
        .iter()
        .enumerate()
        .skip(open + 1)
        .find_map(|(index, token)| (token.kind == TokenKind::Quote).then_some(index))?;
    let prefix = trim_commas(rules.get(..open)?);
    let suffix = trim_commas(rules.get(close + 1..)?);
    (!prefix.is_empty() && !suffix.is_empty()).then_some(rules)
}

fn parse_inline_token_granted_abilities(
    definition: &mut crate::model::token_definition::TokenDefinitionSpec,
    tokens: &[OwnedLexToken],
) -> Vec<GrantedAbilityAst> {
    fn preserve_authored_trigger_intro(
        ability: &mut GrantedAbilityAst,
        rule_tokens: &[OwnedLexToken],
    ) {
        let GrantedAbilityAst::ParsedObjectAbility { ability, .. } = ability else {
            return;
        };
        let Some(ast_intro) = rule_tokens
            .first()
            .and_then(|token| match token.parser_text() {
                "when" => Some(crate::model::ast::TriggerIntroSurfaceAst::When),
                "whenever" => Some(crate::model::ast::TriggerIntroSurfaceAst::Whenever),
                "at" => Some(crate::model::ast::TriggerIntroSurfaceAst::At),
                _ => None,
            })
        else {
            return;
        };

        let crate::model::CompilerAbilityKindCore::Triggered(triggered) = ability.kind_mut() else {
            return;
        };
        if !matches!(
            triggered.trigger,
            crate::model::ast::TriggerSpec::WithIntro { .. }
        ) {
            triggered.trigger = crate::model::ast::TriggerSpec::WithIntro {
                intro: ast_intro,
                trigger: Box::new(triggered.trigger.clone()),
            };
        }

        // Keep the authored intro on both compiler views used by reference
        // preparation and final lowering.
        ability.trigger_spec = Some(Box::new(triggered.trigger.clone()));
    }

    let mut abilities = Vec::new();
    if let Some(rule_tokens) = mixed_pronoun_token_rule_list(tokens)
        && let Ok(parsed) =
            super::parse_granted_abilities_for_token_definition(definition, rule_tokens)
    {
        for mut ability in parsed {
            preserve_authored_trigger_intro(&mut ability, rule_tokens);
            if !abilities.iter().any(|existing| existing == &ability) {
                abilities.push(ability);
            }
        }
        // Every member of this grammar-proven list has been retained as an
        // executable granted ability.  Do not run the per-quote reminder
        // fallback afterward: it can compact only one member and would both
        // duplicate the activation and drop the ordinary keyword/P/T grant.
        if !abilities.is_empty() {
            return abilities;
        }
    }
    // The public create-clause route reconstructs a compact token-definition
    // slice that may omit quoted suffixes. Recover the typed authored order
    // and named self surfaces from the complete clause before reminder
    // merging adds the executable specialized rules.
    if let crate::model::token_definition::TokenDefinitionSpec::Creature(creature) = definition {
        let presentations = token_definition_grammar::authored_inline_rule_presentations(
            tokens,
            Some(&creature.name),
        );
        for presentation in presentations {
            if !creature
                .rules
                .authored_inline_rules
                .iter()
                .any(|existing| existing == &presentation)
            {
                creature.rules.authored_inline_rules.push(presentation);
            }
        }
    }
    // The quoted grant and the trailing `and equip {N}` form one Equipment
    // payload, but the latter sits outside the quoted-rule slices handled
    // below. Recover only those typed Equipment facts from the complete
    // clause; merging all complete-clause reminder facts would also leak
    // keywords from nested tokens inside quotes onto the outer token.
    let complete_reminder = token_definition_grammar::parse_token_reminder_facts_tokens(tokens);
    token_definition_grammar::merge_token_equipment_reminder_definition(
        definition,
        &complete_reminder,
    );
    let outer_tokens = tokens_outside_double_quoted_rules(tokens);
    let outer_reminder = token_definition_grammar::parse_token_reminder_facts_tokens(&outer_tokens);
    token_definition_grammar::merge_token_reminder_definition(definition, &outer_reminder);
    for rule_tokens in double_quoted_rule_bodies(tokens) {
        // Reminder parsing intentionally scans a whole quoted token rule so
        // specialized rules such as a dies-triggered token creation can be
        // retained in the blueprint. Keywords inside the token created by
        // that rule, however, belong to the nested token. Preserve the outer
        // token's already-established keyword set across that broad scan.
        let outer_keywords = quoted_rule_creates_a_nested_token(rule_tokens)
            .then(|| match definition {
                crate::model::token_definition::TokenDefinitionSpec::Creature(creature) => {
                    Some(creature.keywords.clone())
                }
                _ => None,
            })
            .flatten();
        let reminder = token_definition_grammar::parse_token_reminder_facts_tokens(rule_tokens);
        let conflicting_combat_restriction =
            match (&*definition, reminder.creature_combat_restriction()) {
                (
                    crate::model::token_definition::TokenDefinitionSpec::Creature(creature),
                    Some(incoming),
                ) => creature
                    .rules
                    .combat_restriction
                    .as_ref()
                    .is_some_and(|existing| existing != incoming),
                _ => false,
            };
        // `combat_restriction` is the token blueprint's compact slot for one
        // intrinsic restriction. A second independently quoted rule must
        // continue through the ordinary granted-ability parser; replacing
        // the first slot here silently discards one of the two.
        let merged = !conflicting_combat_restriction
            && token_definition_grammar::merge_token_reminder_definition(definition, &reminder);
        if let Some(keywords) = outer_keywords
            && let crate::model::token_definition::TokenDefinitionSpec::Creature(creature) =
                definition
        {
            creature.keywords = keywords;
        }
        // A complete quoted static rule with its own filtered subject is a
        // granted ability of the created token. Reminder extraction may have
        // already cached the same text as an embedded token-rule shape,
        // which makes the generic ability parser think the rule was fully
        // lowered even though that compact cache cannot carry the filtered
        // grant. Probe a surface-neutral copy first and keep only the typed
        // filtered grant; intrinsic self rules continue through the compact
        // reminder path below.
        let mut filtered_probe = definition.clone();
        match &mut filtered_probe {
            crate::model::token_definition::TokenDefinitionSpec::Creature(creature) => {
                creature.rules.combat_restriction = None;
                creature.rules.token_rules.embedded_rules.clear();
            }
            crate::model::token_definition::TokenDefinitionSpec::Artifact(artifact) => {
                artifact.token_rules.embedded_rules.clear();
            }
            _ => {}
        }
        if let Ok(Some(ability)) =
            crate::keyword_static::parse_attacks_each_combat_if_able_line(rule_tokens)
        {
            let ability = GrantedAbilityAst::StaticAbility(Box::new(ability));
            if !abilities.iter().any(|existing| existing == &ability) {
                abilities.push(ability);
            }
            continue;
        }
        // This probe exists only to recover complete filtered static grants.
        // Trigger and activation introductions cannot produce one of those
        // static carriers. Keep them on their dedicated typed paths instead
        // of entering the complete grant parser twice while the enclosing
        // create statement is still being recognized.
        let starts_triggered_rule = rule_tokens
            .first()
            .is_some_and(|token| token.is_any_word(&["when", "whenever", "at"]));
        let rule_words = token_word_refs(rule_tokens);
        let starts_intrinsic_self_rule = crate::word_primitives::parse_any_sequence_prefix(
            &rule_words,
            &[&["this", "token"], &["this", "creature"]],
        );
        let filtered_grant_probe = (!starts_triggered_rule
            && !starts_intrinsic_self_rule
            && !rule_tokens
                .iter()
                .any(|token| token.kind == TokenKind::Colon))
        .then(|| super::parse_granted_abilities_for_token_definition(&filtered_probe, rule_tokens));
        if let Some(Ok(parsed)) = filtered_grant_probe {
            let filtered_grants = parsed.into_iter().filter(|ability| match ability {
                GrantedAbilityAst::StaticAbility(ability) => match ability.as_ref() {
                    StaticAbilityAst::GrantStaticAbility { .. }
                    | StaticAbilityAst::GrantKeywordAction { .. }
                    | StaticAbilityAst::GrantObjectAbility { .. } => true,
                    StaticAbilityAst::Static(static_ability) => matches!(
                        &static_ability.payload,
                        ironsmith_core::StaticAbilityPayload::GrantAbility(_)
                            | ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(_)
                    ),
                    _ => false,
                },
                GrantedAbilityAst::ParsedObjectAbility { ability, .. } => {
                    let crate::model::CompilerAbilityKindCore::Static(ability) = ability.kind()
                    else {
                        return false;
                    };
                    matches!(
                        &ability.payload,
                        ironsmith_core::StaticAbilityPayload::GrantAbility(_)
                            | ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(_)
                    )
                }
                _ => false,
            });
            let mut attached_filtered_grant = false;
            for ability in filtered_grants {
                if !abilities.iter().any(|existing| existing == &ability) {
                    abilities.push(ability);
                }
                attached_filtered_grant = true;
            }
            if attached_filtered_grant {
                continue;
            }
        }
        if merged {
            // Specialized token rules belong to the token blueprint. This is
            // particularly important after outer dispatch strips the quoted
            // suffix before parsing the create action: restore the typed rule
            // before deciding whether a generic granted ability is needed.
            // Reapply the outer clause so an Equipment rule body cannot
            // discard a trailing `and equip` clause outside its quotes. Do
            // not merge keywords from a nested token rule into this token.
            token_definition_grammar::merge_token_reminder_definition(definition, &outer_reminder);
            continue;
        }
        if !conflicting_combat_restriction
            && append_inline_token_embedded_rule(definition, rule_tokens)
        {
            continue;
        }
        let parse_definition = conflicting_combat_restriction.then(|| {
            let mut parse_definition = definition.clone();
            if let crate::model::token_definition::TokenDefinitionSpec::Creature(creature) =
                &mut parse_definition
            {
                creature.rules.combat_restriction = None;
            }
            parse_definition
        });
        let final_grant_parse = super::parse_granted_abilities_for_token_definition(
            parse_definition.as_ref().unwrap_or(definition),
            rule_tokens,
        );
        let Ok(parsed) = final_grant_parse else {
            // Token definitions have a number of older specialized shapes.
            // An unsupported generic nested rule must leave those paths
            // available rather than turning an otherwise parseable card into
            // a hard error.
            continue;
        };
        for mut ability in parsed {
            preserve_authored_trigger_intro(&mut ability, rule_tokens);
            if !abilities.iter().any(|existing| existing == &ability) {
                abilities.push(ability);
            }
        }
    }
    abilities
}

fn intrinsic_token_ability_represents_dynamic_power_toughness(
    definition: &crate::model::token_definition::TokenDefinitionSpec,
    granted_abilities: &[GrantedAbilityAst],
    dynamic: &(Value, Value),
) -> bool {
    let same_values = |power: &Value, toughness: &Value| {
        power.unhinted() == dynamic.0.unhinted() && toughness.unhinted() == dynamic.1.unhinted()
    };
    if granted_abilities.iter().any(|ability| {
        let payload = match ability {
            GrantedAbilityAst::StaticAbility(static_ability) => {
                let StaticAbilityAst::Static(static_ability) = static_ability.as_ref() else {
                    return false;
                };
                &static_ability.payload
            }
            GrantedAbilityAst::ParsedObjectAbility { ability, .. } => {
                let crate::model::CompilerAbilityKindCore::Static(static_ability) = ability.kind()
                else {
                    return false;
                };
                &static_ability.payload
            }
            _ => return false,
        };
        let ironsmith_core::StaticAbilityPayload::CharacteristicDefiningPt { power, toughness } =
            payload
        else {
            return false;
        };
        same_values(power, toughness)
    }) {
        return true;
    }

    let crate::model::token_definition::TokenDefinitionSpec::Creature(creature) = definition else {
        return false;
    };
    let creature_count = Value::Count(ObjectFilter::creature().you_control());
    same_values(&creature_count, &creature_count)
        && creature
            .rules
            .token_rules
            .embedded_rules
            .iter()
            .any(|rule| {
                rule == &crate::model::token_definition::TokenEmbeddedRuleShape::
                PowerToughnessEqualCreaturesYouControl
            })
}

fn incorporate_quoted_dynamic_power_toughness(
    definition: &crate::model::token_definition::TokenDefinitionSpec,
    granted_abilities: &mut Vec<GrantedAbilityAst>,
    dynamic_power_toughness: &mut Option<(Value, Value)>,
    quoted_dynamic: (Value, Value),
) {
    if intrinsic_token_ability_represents_dynamic_power_toughness(
        definition,
        granted_abilities,
        &quoted_dynamic,
    ) {
        *dynamic_power_toughness = None;
        return;
    }

    // A P/T rule authored inside quotes is an intrinsic characteristic-
    // defining ability of the token, even when it references only the
    // creating player's state. Treating it as a one-time post-creation base
    // P/T assignment loses both its runtime behavior and its token-blueprint
    // surface.
    let ability = GrantedAbilityAst::StaticAbility(Box::new(StaticAbilityAst::Static(
        StaticAbility::characteristic_defining_pt(quoted_dynamic.0, quoted_dynamic.1),
    )));
    if !granted_abilities
        .iter()
        .any(|existing| existing == &ability)
    {
        granted_abilities.push(ability);
    }
    *dynamic_power_toughness = None;
}

fn parse_inline_copy_granted_abilities(tokens: &[OwnedLexToken]) -> Vec<GrantedAbilityAst> {
    let mut abilities = Vec::new();
    for rule_tokens in double_quoted_rule_bodies(tokens) {
        if token_definition_grammar::parse_token_reminder_facts_tokens(rule_tokens)
            .sacrifice_at_next_end_step
        {
            // Copy-token delayed-sacrifice text has a dedicated AST field and
            // must not also be installed as an ordinary granted ability.
            continue;
        }
        let clause_words = token_word_refs(rule_tokens);
        let parsed = (|| -> Result<_, CardTextError> {
            if let Some(ability) =
                super::gain_ability::parse_granted_activated_or_triggered_ability_for_gain(
                    rule_tokens,
                    &clause_words,
                )?
            {
                return Ok((vec![ability], false));
            }
            super::parse_granted_abilities_for_gain_clause(rule_tokens, &clause_words, false)
        })();
        let Ok((granted, false)) = parsed else {
            continue;
        };
        for ability in granted {
            if !abilities.iter().any(|existing| existing == &ability) {
                abilities.push(ability);
            }
        }
    }
    abilities
}

fn merge_inline_copy_granted_ability(
    granted_abilities: &mut Vec<GrantedAbilityAst>,
    candidate: GrantedAbilityAst,
) -> bool {
    if granted_abilities
        .iter()
        .any(|existing| existing == &candidate)
    {
        return false;
    }
    granted_abilities.push(candidate);
    true
}

fn attach_inline_token_granted_abilities_to_effect(
    effect: &mut EffectAst,
    tokens: &[OwnedLexToken],
) -> bool {
    if let EffectAst::SubjectVerb(subject_verb) = effect {
        match &mut subject_verb.action {
            SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy {
                sacrifice_at_next_end_step,
                sacrifice_at_next_end_step_ability_surface,
                granted_abilities,
                ..
            })
            | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource {
                sacrifice_at_next_end_step,
                sacrifice_at_next_end_step_ability_surface,
                granted_abilities,
                ..
            }) => {
                let mut attached = false;
                if *sacrifice_at_next_end_step
                    && sacrifice_at_next_end_step_ability_surface.is_none()
                {
                    *sacrifice_at_next_end_step_ability_surface =
                        quoted_copy_sacrifice_ability_surface(tokens);
                    attached |= sacrifice_at_next_end_step_ability_surface.is_some();
                }
                for ability in parse_inline_copy_granted_abilities(tokens) {
                    attached |= merge_inline_copy_granted_ability(granted_abilities, ability);
                }
                if attached {
                    return true;
                }
            }
            _ => {}
        }
    }

    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            definition,
            dynamic_power_toughness,
            granted_abilities,
            ability_presentation,
            ..
        }) = &mut subject_verb.action
    {
        for ability in parse_inline_token_granted_abilities(definition, tokens) {
            if !granted_abilities
                .iter()
                .any(|existing| existing == &ability)
            {
                granted_abilities.push(ability);
            }
        }
        if let Some(dynamic) = parse_quoted_token_dynamic_power_toughness(tokens) {
            incorporate_quoted_dynamic_power_toughness(
                definition,
                granted_abilities,
                dynamic_power_toughness,
                dynamic,
            );
        }
        if ability_presentation.is_none() {
            *ability_presentation = Some(ironsmith_core::TokenAbilityPresentation::InlineWith);
        }
        return true;
    }

    let mut found = false;
    crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
        if !found {
            found = attach_inline_token_granted_abilities_to_last_create(nested, tokens);
        }
    });
    found
}

/// Production sentence dispatch strips embedded token rules before parsing the
/// outer create action so quoted colons and verbs cannot win outer dispatch.
/// Reattach those original quoted bodies to the create AST after that parse.
pub fn attach_inline_token_granted_abilities_to_last_create(
    effects: &mut [EffectAst],
    tokens: &[OwnedLexToken],
) -> bool {
    let Some(tokens) = inline_quoted_token_creation_sentence(tokens) else {
        return false;
    };
    if double_quoted_rule_bodies(tokens).is_empty() {
        return false;
    }
    for effect in effects.iter_mut().rev() {
        if attach_inline_token_granted_abilities_to_effect(effect, tokens) {
            return true;
        }
    }
    false
}

/// Keep an inline quoted copy exception on the replacement copy only.
///
/// A cross-sentence self replacement is parsed jointly so its target can be
/// shared. During that joint parse, the quoted `except the token has ...`
/// suffix can be observed once by the initial copy clause and again as a
/// coordinated set grant. The final authored create sentence proves that the
/// rule belongs to the replacement copy, so remove only those two exact
/// duplicates after the replacement copy has retained the typed grant.
pub fn recognize_inline_copy_self_replacement_grants(
    effects: &mut [EffectAst],
    tokens: &[OwnedLexToken],
) -> bool {
    fn copy_has_grants(effect: &EffectAst, expected: &[GrantedAbilityAst]) -> bool {
        let direct = match effect {
            EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
                SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy {
                    granted_abilities, ..
                })
                | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource {
                    granted_abilities, ..
                }) => granted_abilities == expected,
                _ => false,
            },
            _ => false,
        };
        if direct {
            return true;
        }
        let mut found = false;
        crate::model::visit::for_each_nested_effects(effect, true, |nested| {
            found |= nested
                .iter()
                .any(|effect| copy_has_grants(effect, expected));
        });
        found
    }

    fn clear_matching_copy_grants(effects: &mut [EffectAst], expected: &[GrantedAbilityAst]) {
        for effect in effects {
            if let EffectAst::SubjectVerb(subject_verb) = effect {
                match &mut subject_verb.action {
                    SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy {
                        granted_abilities, ..
                    })
                    | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource {
                        granted_abilities, ..
                    }) if granted_abilities == expected => granted_abilities.clear(),
                    _ => {}
                }
            }
            crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                clear_matching_copy_grants(nested, expected);
            });
        }
    }

    fn is_duplicate_token_set_grant(effect: &EffectAst, expected: &[GrantedAbilityAst]) -> bool {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            return false;
        };
        let SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
            filter,
            abilities,
            duration,
            condition,
            set_quantifier_surface,
            lock_filter_at_resolution,
        }) = &subject_verb.action
        else {
            return false;
        };
        let mut token_filter = ObjectFilter::default();
        token_filter.token = true;
        filter == &token_filter
            && abilities == expected
            && *duration == crate::effect::Until::Forever
            && condition.is_none()
            && set_quantifier_surface.is_none()
            && *lock_filter_at_resolution
    }

    fn remove_duplicate_token_set_grants(
        effects: &mut Vec<EffectAst>,
        expected: &[GrantedAbilityAst],
    ) -> bool {
        fn remove_from_effect(effect: &mut EffectAst, expected: &[GrantedAbilityAst]) -> bool {
            let mut changed = false;
            match effect {
                EffectAst::Coordination(coordination) => {
                    for member in &mut coordination.members {
                        changed |= remove_duplicate_token_set_grants(&mut member.effects, expected);
                    }
                    coordination
                        .members
                        .retain(|member| !member.effects.is_empty());
                }
                _ => crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                    for effect in nested {
                        changed |= remove_from_effect(effect, expected);
                    }
                }),
            }
            changed
        }

        let before = effects.len();
        effects.retain(|effect| !is_duplicate_token_set_grant(effect, expected));
        let mut changed = effects.len() != before;
        for effect in effects {
            changed |= remove_from_effect(effect, expected);
        }
        changed
    }

    let Some(replacement_sentence) = inline_quoted_token_creation_sentence(tokens) else {
        return false;
    };
    let unquoted_replacement = tokens_outside_double_quoted_rules(replacement_sentence);
    let replacement_words = token_word_refs(&unquoted_replacement);
    if !crate::word_primitives::sequence_occurs(&replacement_words, &["instead", "create"])
        || !crate::word_primitives::sequence_occurs(
            &replacement_words,
            &["except", "the", "token", "has"],
        )
    {
        return false;
    }
    let expected = parse_inline_copy_granted_abilities(replacement_sentence);
    if expected.is_empty() {
        return false;
    }

    let mut changed = false;
    for effect in effects {
        let EffectAst::SelfReplacement {
            if_true, if_false, ..
        } = effect
        else {
            continue;
        };
        if !if_true
            .iter()
            .any(|effect| copy_has_grants(effect, &expected))
        {
            continue;
        }
        clear_matching_copy_grants(if_false, &expected);
        remove_duplicate_token_set_grants(if_true, &expected);
        changed = true;
    }
    changed
}

/// Attach a separately authored pronoun sentence whose ability list mixes a
/// keyword, a quoted rule, and a trailing activation. The caller proves that
/// this sentence immediately follows one token creation.
pub fn attach_mixed_pronoun_token_rules_to_last_create(
    effects: &mut [EffectAst],
    tokens: &[OwnedLexToken],
) -> bool {
    if mixed_pronoun_token_rule_list(tokens).is_none() {
        return false;
    }
    fn mark_combined_separate_sentence(effect: &mut EffectAst) -> bool {
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                ability_presentation,
                ..
            }) = &mut subject_verb.action
        {
            *ability_presentation =
                Some(ironsmith_core::TokenAbilityPresentation::SeparateSentenceCombined);
            return true;
        }

        let mut found = false;
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested.iter_mut().rev() {
                if !found {
                    found = mark_combined_separate_sentence(nested_effect);
                }
            }
        });
        found
    }

    for effect in effects.iter_mut().rev() {
        if attach_inline_token_granted_abilities_to_effect(effect, tokens) {
            return mark_combined_separate_sentence(effect);
        }
    }
    false
}

/// "Create your choice of a Clue token, a Food token, or a Treasure token" —
/// exactly one of the listed tokens is created, so lower one create mode per
/// option instead of splitting into sequential creates.
fn parse_create_choice_of_options(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_word_refs(tokens);
    if words.get(..3) != Some(&["your", "choice", "of"][..]) {
        return Ok(None);
    }
    let mut consumed = 0usize;
    let mut seen_words = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_some() {
            seen_words += 1;
        }
        if seen_words == 3 {
            consumed = idx + 1;
            break;
        }
    }
    let rest = &tokens[consumed..];
    let mut segments: Vec<Vec<OwnedLexToken>> = vec![Vec::new()];
    for token in rest {
        if token.kind == TokenKind::Comma || token.is_word("or") {
            if !segments.last().is_some_and(Vec::is_empty) {
                segments.push(Vec::new());
            }
            continue;
        }
        segments
            .last_mut()
            .expect("segment list is never empty")
            .push(token.clone());
    }
    segments.retain(|segment| !segment.is_empty());
    if segments.len() < 2 {
        return Ok(None);
    }
    let mut options = Vec::new();
    for segment in &segments {
        let display = format!("Create {}", render_token_slice(segment));
        let effect = parse_create(segment, None)?;
        options.push((display, Box::new(effect)));
    }
    Ok(Some(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenChoice { options }),
    )))
}

pub fn lower_complete_simple_create_shape(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    let head = creation_grammar::parse_create_head_tokens(tokens).ok_or_else(|| {
        CardTextError::ParseError(
            "complete simple create shape is missing a token head".to_string(),
        )
    })?;
    let tail_tokens = crate::util::trim_edge_punctuation(head.tail_tokens);
    let attached_to = if tail_tokens.is_empty() {
        None
    } else if let Some(attached) = creation_grammar::parse_attachment_clause_tokens(&tail_tokens)
        && crate::util::trim_edge_punctuation(attached.prefix_tokens).is_empty()
    {
        Some(parse_target_phrase(attached.target_tokens)?)
    } else {
        return Err(CardTextError::ParseError(
            "complete simple create shape has an unsupported trailing modifier".to_string(),
        ));
    };
    let count = creation_grammar::create_count_head_value(&head.count);
    let name = normalize_token_name(&head.name_words);
    let definition =
        token_definition_grammar::parse_token_definition_shape_tokens(head.name_tokens)
            .ok_or_else(|| {
                CardTextError::ParseError(format!("unsupported token definition '{name}'"))
            })?;
    Ok(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            name,
            definition,
            count,
            dynamic_power_toughness: None,
            player: PlayerAst::Implicit,
            actor_surface_explicit: false,
            attached_to,
            tapped: false,
            attacking: false,
            attack_target_player: None,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            next_end_step_player: PlayerFilter::Any,
            granted_abilities: Vec::new(),
            ability_presentation: None,
        }),
    ))
}

#[path = "creation_handlers/create_clause_readings.rs"]
mod create_clause_readings;

pub fn parse_create(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    // Capture the authored actor before imperative/chain normalization can
    // turn an implicit create action into the same semantic `PlayerAst::You`.
    let actor_surface_explicit = matches!(subject, Some(SubjectAst::Player(PlayerAst::You | PlayerAst::ItsOwner | PlayerAst::ItsController)));
    let authored_dynamic_count = if let Some(binding) =
        crate::grammar::effects::dispatch_entry_shapes::parse_where_x_usage_shape_tokens(tokens)
    {
        parse_create_value_binding(crate::util::trim_edge_punctuation_tokens(
            binding.binding_tokens,
        ))?
    } else {
        None
    };
    let tokens = creation_grammar::creation_body_tokens(tokens);
    let input = create_clause_readings::CreateClause { tokens, subject };
    match create_clause_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(matched.value.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }
    let mut player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let clause_words = token_word_refs(tokens);
    let head = creation_grammar::parse_create_head_tokens(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "create clause missing token (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let mut count_value = creation_grammar::create_count_head_value(&head.count);
    if let Some(authored_dynamic_count) = authored_dynamic_count
        && (value_contains_unbound_x(&count_value)
            || matches!(
                authored_dynamic_count.unhinted(),
                Value::StaticAbilitiesAmong { .. }
            ))
    {
        count_value = with_where_x_surface_hints(authored_dynamic_count, tokens);
    }
    if let Some(ability_token_index) = crate::lexer::parser_token_word_positions(tokens)
        .into_iter()
        .find_map(|(index, word)| matches!(word, "ability" | "abilities").then_some(index))
        && let Some(value) = crate::keyword_static::parse_static_abilities_among_scope_value(
            &tokens[ability_token_index..],
        )
    {
        count_value = with_where_x_surface_hints(value, tokens);
    }
    let needs_equal_to_dynamic_count = matches!(
        head.count,
        creation_grammar::CreateCountHead::EqualToDynamic
    );
    let authored_appositive_name = token_definition_grammar::leading_appositive_token_name(tokens);
    let mut definition_tokens = head.name_tokens.to_vec();
    let mut name_words = head.name_words;
    let mut tail_tokens = head.tail_tokens.to_vec();
    if needs_equal_to_dynamic_count {
        let Some((dynamic_count, equal_token_idx)) =
            parse_create_equal_to_dynamic_count(&tail_tokens)?
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported dynamic token count in create clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        count_value = dynamic_count;
        tail_tokens.truncate(equal_token_idx);
    }
    let mut delayed_create_player = None;
    let initial_tail_words = token_word_refs(&tail_tokens);
    if let Some((clause_start, player)) =
        trailing_create_at_next_end_step_clause(&initial_tail_words)
    {
        delayed_create_player = Some(player);
        if let Some(cut_idx) =
            creation_grammar::CreationTokens::new(&tail_tokens).boundary(clause_start)
        {
            tail_tokens.truncate(cut_idx);
        }
    }
    let mut attached_to_target: Option<TargetAst> = None;
    if let Some(attached) = creation_grammar::parse_attachment_clause_tokens(&tail_tokens) {
        attached_to_target = Some(parse_target_phrase(attached.target_tokens)?);
        tail_tokens = attached.prefix_tokens.to_vec();
    }
    let tail_words = token_word_refs(&tail_tokens);
    let tail_surface = creation_grammar::CreationWords::new(&tail_words);
    if attached_to_target.is_some() && tail_surface.has(CreateWord::Copy) {
        return Err(CardTextError::ParseError(format!(
            "unsupported aura-copy attachment fanout clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let for_each_clause = creation_grammar::parse_for_each_clause_tokens(&tail_tokens);
    let for_each_idx = for_each_clause.as_ref().map(|clause| clause.start_word);
    let mut for_each_dynamic_count: Option<Value> = None;
    let mut for_each_object_filter: Option<ObjectFilter> = None;
    let mut for_each_player_condition: Option<(PlayerFilter, PredicateAst)> = None;
    if let Some(for_each_clause) = for_each_clause {
        let filter_tokens = for_each_clause.filter_tokens;
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter after 'for each' in create clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if let Some(parsed) = parse_create_for_each_player_condition(filter_tokens, &clause_words)?
        {
            for_each_player_condition = Some(parsed);
            if player == PlayerAst::Implicit {
                player = PlayerAst::You;
            }
        } else if let Some(dynamic) =
            creation_grammar::parse_creation_for_each_dynamic_count_tokens(filter_tokens)
        {
            for_each_dynamic_count = Some(dynamic.with_surface_hint(ValueSurfaceHint::ForEach));
        } else {
            reject_lossy_for_each_fallback(filter_tokens, &clause_words)?;
            let filter = parse_object_filter(filter_tokens, false)?;
            for_each_object_filter = Some(filter);
        }
    }
    if let Some(where_tokens) = creation_grammar::parse_where_clause_tokens(&tail_tokens)
        && let Some(where_value) = parse_create_value_binding(where_tokens)?
        && (value_contains_unbound_x(&count_value)
            || matches!(where_value.unhinted(), Value::StaticAbilitiesAmong { .. }))
    {
        count_value = with_where_x_surface_hints(where_value, tokens);
    }
    let resolve_create_count = |references_iterated_object: bool| {
        if let Some(dynamic) = for_each_dynamic_count.clone() {
            return dynamic;
        }
        if let Some(filter) = for_each_object_filter.clone() {
            if references_iterated_object {
                return count_value.clone();
            }
            return Value::Count(filter);
        }
        count_value.clone()
    };
    let wrap_for_each_when_needed = |effect: EffectAst, references_iterated_object: bool| {
        if references_iterated_object && let Some(filter) = for_each_object_filter.clone() {
            EffectAst::ForEach(ForEachEffectAst::ForEachObject {
                filter,
                effects: vec![effect],
            })
        } else {
            effect
        }
    };
    let wrap_for_each_player_condition = |effect: EffectAst| {
        if let Some((filter, predicate)) = &for_each_player_condition {
            let effects = vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: predicate.clone(),
                if_true: vec![effect],
                if_false: Vec::new(),
            })];
            match filter {
                PlayerFilter::Opponent => EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects }),
                PlayerFilter::Any => EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }),
                other => EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                    filter: other.clone(),
                    effects,
                }),
            }
        } else {
            effect
        }
    };
    let wrap_delayed_create = |effect: EffectAst| {
        if let Some(player) = delayed_create_player {
            EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep {
                player,
                effects: vec![effect],
            })
        } else {
            effect
        }
    };
    let mut tapped = false;
    let mut attacking = false;
    let mut modifier_tail_words = tail_words.clone();
    // The `for each` suffix describes the dynamic count, not the token being
    // created.  In particular, `create a Treasure token for each tapped ...`
    // must not make the Treasure enter tapped merely because the counted
    // objects are tapped.
    if let Some(for_each_idx) = for_each_idx {
        modifier_tail_words.truncate(for_each_idx);
    }
    let mut raw_name_override = authored_appositive_name;
    let mut rules_text_range: Option<(usize, usize)> = None;
    if let Some(named) = creation_grammar::parse_named_token_clause_tokens(&tail_tokens) {
        let named_words = token_word_refs(&tail_tokens[named.name.clone()]);
        if !named_words.is_empty() {
            let authored_name = render_token_slice(&tail_tokens[named.name.clone()])
                .trim()
                .to_string();
            if tail_tokens[named.name.clone()]
                .iter()
                .any(OwnedLexToken::is_comma)
            {
                raw_name_override = Some(authored_name);
            }
            name_words.push("named");
            name_words.extend(named_words);
            definition_tokens.extend_from_slice(&tail_tokens[named.clause]);
        }
    }
    name_words.retain(|word| {
        if *word == "tapped" {
            tapped = true;
            return false;
        }
        if *word == "attacking" {
            attacking = true;
            return false;
        }
        true
    });
    name_words.retain(|word| !matches!(*word, "and" | "or"));
    let name_words_primary_len = name_words.len();
    if name_words.is_empty() {
        if tail_surface.has(CreateWord::Copy) {
            let (
                set_colors,
                set_card_types,
                set_subtypes,
                added_card_types,
                added_subtypes,
                removed_supertypes,
                set_base_power_toughness,
                set_base_power_toughness_to_source_totals,
                starting_loyalty,
                granted_abilities,
                loses_soulbond,
            ) = parse_copy_modifiers_from_tail(&tail_words)?;
            let granted_abilities = granted_abilities
                .into_iter()
                .map(|ability| {
                    GrantedAbilityAst::StaticAbility(Box::new(StaticAbilityAst::Static(ability)))
                })
                .collect();
            let half_pt = tail_surface.has(CreateWord::Half)
                && tail_surface.has(CreateWord::Power)
                && tail_surface.has(CreateWord::Toughness);
            let has_haste = tail_surface.has_phrase(CreatePhrase::HasteGrant)
                || tail_surface.has(CreateWord::Haste);
            let token_modifier_words = tail_surface
                .location(CreateWord::Token)
                .map(|idx| &tail_words[..idx])
                .unwrap_or_default();
            let copy_modifier_words = tail_surface
                .location(CreateWord::Copy)
                .map(|idx| &tail_words[..idx])
                .unwrap_or_default();
            let token_modifier_surface = creation_grammar::CreationWords::new(token_modifier_words);
            let copy_modifier_surface = creation_grammar::CreationWords::new(copy_modifier_words);
            let mut enters_tapped = tapped
                || token_modifier_surface.has(CreateWord::Tapped)
                || copy_modifier_surface.has(CreateWord::Tapped);
            let mut enters_attacking = attacking
                || token_modifier_surface.has(CreateWord::Attacking)
                || copy_modifier_surface.has(CreateWord::Attacking);
            let mut attack_target_player_or_planeswalker_controlled_by = None;
            if player == PlayerAst::Implicit {
                player = PlayerAst::You;
            }
            let (sacrifice_at_next_end_step, exile_at_next_end_step, next_end_step_player) =
                parse_next_end_step_token_delay_flags(&tail_words);
            let sacrifice_at_next_end_step_ability_surface = sacrifice_at_next_end_step
                .then(|| quoted_copy_sacrifice_ability_surface(&tail_tokens))
                .flatten();
            if let Some(source_clause) =
                creation_grammar::parse_copy_source_clause_tokens(&tail_tokens)
            {
                enters_tapped = source_clause.enters_tapped;
                enters_attacking = source_clause.enters_attacking;
                attack_target_player_or_planeswalker_controlled_by = source_clause
                    .attacks_that_player_or_planeswalker
                    .then_some(PlayerAst::That);
                if !source_clause.source_tokens.is_empty() {
                    if let Some(token_word_idx) =
                        creation_grammar::CreationWords::new(&clause_words)
                            .location(CreateWord::Token)
                    {
                        let token_prefix =
                            creation_grammar::CreationWords::new(&clause_words[..token_word_idx]);
                        enters_tapped |= token_prefix.has(CreateWord::Tapped);
                        enters_attacking |= token_prefix.has(CreateWord::Attacking);
                    }
                    let source = parse_target_phrase(&source_clause.source_tokens)?;
                    let references_iterated_object = target_references_it(&source);
                    let create = EffectAst::subject_verb(
                        SubjectVerbRoleAst::Actor,
                        player,
                        SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource {
                            source,
                            count: resolve_create_count(references_iterated_object),
                            player,
                            enters_tapped,
                            enters_attacking,
                            attack_target_player_or_planeswalker_controlled_by,
                            entry_tapped_attacking_followup: false,
                            attack_target_player_only: false,
                            half_power_toughness_round_up: half_pt,
                            has_haste,
                            haste_followup_reference_surface: None,
                            exile_at_end_of_combat: false,
                            exile_at_end_of_combat_reference_surface: None,
                            loses_soulbond,
                            sacrifice_at_next_end_step,
                            sacrifice_at_next_end_step_reference_surface: None,
                            sacrifice_at_next_end_step_ability_surface,
                            exile_at_next_end_step,
                            exile_at_next_end_step_reference_surface: None,
                            next_end_step_player: next_end_step_player.clone(),
                            set_colors,
                            set_card_types,
                            set_subtypes,
                            added_card_types,
                            added_subtypes,
                            removed_supertypes,
                            set_base_power_toughness,
                            set_base_power_toughness_to_source_totals,
                            starting_loyalty,
                            granted_abilities,
                        }),
                    );
                    return Ok(wrap_for_each_player_condition(wrap_delayed_create(
                        wrap_for_each_when_needed(create, references_iterated_object),
                    )));
                }
            }
            let references_iterated_object = true;
            let create = EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                player,
                SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy {
                    object: ObjectRefAst::Tagged(crate::tag::CompilerReferenceTag::It.bind()),
                    count: resolve_create_count(references_iterated_object),
                    player,
                    enters_tapped,
                    enters_attacking,
                    attack_target_player_or_planeswalker_controlled_by,
                    entry_tapped_attacking_followup: false,
                    attack_target_player_only: false,
                    half_power_toughness_round_up: half_pt,
                    has_haste,
                    haste_followup_reference_surface: None,
                    exile_at_end_of_combat: false,
                    exile_at_end_of_combat_reference_surface: None,
                    loses_soulbond,
                    sacrifice_at_next_end_step,
                    sacrifice_at_next_end_step_reference_surface: None,
                    sacrifice_at_next_end_step_ability_surface,
                    exile_at_next_end_step,
                    exile_at_next_end_step_reference_surface: None,
                    next_end_step_player,
                    set_colors,
                    set_card_types,
                    set_subtypes,
                    added_card_types,
                    added_subtypes,
                    removed_supertypes,
                    set_base_power_toughness,
                    set_base_power_toughness_to_source_totals,
                    starting_loyalty,
                    granted_abilities,
                }),
            );
            return Ok(wrap_for_each_player_condition(wrap_delayed_create(
                wrap_for_each_when_needed(create, references_iterated_object),
            )));
        }
        return Err(CardTextError::ParseError(
            "create clause missing token name".to_string(),
        ));
    }
    if let Some(with_idx) = tail_surface.location(CreateWord::With) {
        let with_tail_end = for_each_idx.unwrap_or(tail_words.len());
        if with_idx + 1 < with_tail_end {
            let with_words = &tail_words[with_idx + 1..with_tail_end];
            let with_surface = creation_grammar::CreationWords::new(with_words);
            let has_equipment_rules_subject = with_surface.has_phrase(CreatePhrase::EquipmentRules);
            let rules_text_start = with_surface.location(CreateWord::RulesTextStart);
            let mut include_end = rules_text_start.unwrap_or(with_words.len());
            if include_end > 0
                && let Some(named_pos) =
                    creation_grammar::CreationWords::new(&with_words[..include_end])
                        .location(CreateWord::Named)
            {
                include_end = named_pos;
            }
            let preserve_rules_tail = rules_text_start
                .is_some_and(|start| start < with_words.len())
                && creation_grammar::CreationWords::new(&with_words[include_end..])
                    .has(CreateWord::PreserveRulesTail);
            let preserve_rules_tail = preserve_rules_tail || has_equipment_rules_subject;
            let definition_with_words = if preserve_rules_tail || include_end == 0 {
                with_words.len()
            } else {
                include_end
            };
            if !preserve_rules_tail
                && let Some(with_tokens) = creation_grammar::CreationTokens::new(&tail_tokens)
                    .word_range(with_idx..with_idx + 1 + definition_with_words)
            {
                definition_tokens.extend_from_slice(with_tokens);
            }
            if preserve_rules_tail {
                let start = with_idx + 1 + include_end;
                if start < with_tail_end {
                    rules_text_range = Some((start, with_tail_end));
                }
                let token_surface = creation_grammar::CreationTokens::new(&tail_tokens);
                let raw_tail_start = token_surface
                    .boundary(with_idx)
                    .unwrap_or(with_idx.min(tail_tokens.len()));
                let raw_tail_end = if let Some(for_each_idx) = for_each_idx {
                    token_surface
                        .boundary(for_each_idx)
                        .unwrap_or(tail_tokens.len())
                } else {
                    tail_tokens.len()
                };
                definition_tokens.extend_from_slice(&tail_tokens[raw_tail_start..raw_tail_end]);
                // Keep rules in the blueprint without treating their text
                // as an explicit token name. Only an authored name overrides
                // the name derived from the token's subtypes.
            }
            if include_end > 0 {
                name_words.extend(with_words[..include_end].iter().copied());
                if preserve_rules_tail {
                    // Keep quoted token rules text tails so token lowering can
                    // reconstruct granted abilities instead of dropping them.
                    name_words.extend(with_words[include_end..].iter().copied());
                }
            } else {
                // Preserve quoted token rules text so token compilation can
                // attach the ability to the created token definition.
                name_words.extend(with_words.iter().copied());
            }
        }
    }
    // Preserve a quoted dynamic value provisionally. Once the token's quoted
    // abilities have been parsed below, this remains only as a compatibility
    // fallback for external references that cannot yet become an intrinsic
    // token CDA.
    let mut dynamic_power_toughness =
        parse_unquoted_token_dynamic_power_toughness(&definition_tokens)
            .or_else(|| parse_unquoted_token_dynamic_power_toughness(tokens))
            .or_else(|| parse_quoted_token_dynamic_power_toughness(tokens));
    let primary_definition_is_construct = crate::word_primitives::sequence_occurs(
        &name_words[..name_words_primary_len],
        &["construct"],
    );
    if let Some((pt_idx, pt)) = creation_grammar::first_pt_word(&name_words)
        && pt_idx < name_words_primary_len
    {
        let component_value = |component| match component {
            creation_grammar::PtComponent::Fixed(value) => Some(Value::Fixed(value)),
            creation_grammar::PtComponent::X => Some(Value::X),
            creation_grammar::PtComponent::Star => None,
        };
        let contains_x = matches!(pt.power, creation_grammar::PtComponent::X)
            || matches!(pt.toughness, creation_grammar::PtComponent::X);
        if contains_x
            && let (Some(power), Some(toughness)) =
                (component_value(pt.power), component_value(pt.toughness))
        {
            dynamic_power_toughness = Some((power, toughness));
            if primary_definition_is_construct
                && !replace_dynamic_construct_pt_definition_placeholder(&mut definition_tokens)
            {
                return Err(CardTextError::InvariantViolation(
                    "dynamic Construct power/toughness was absent from its definition tokens"
                        .to_string(),
                ));
            }
            name_words[pt_idx] = "0/0";
        }
        let prefix_words = &name_words[..pt_idx];
        let prefix_surface = creation_grammar::CreationWords::new(prefix_words);
        let keep_prefix = prefix_surface.has_phrase(CreatePhrase::NotLegendary)
            || prefix_surface.has(CreateWord::Legendary)
            || prefix_words
                .first()
                .is_some_and(|word| is_probable_token_name_word(word));
        if !keep_prefix {
            name_words = name_words[pt_idx..].to_vec();
            if let Some(first_pt_token) =
                token_definition_grammar::first_token_definition_pt_token(&definition_tokens)
            {
                definition_tokens.drain(..first_pt_token);
            }
        }
    }
    let has_raw_name_override = raw_name_override.is_some();
    let name = raw_name_override.unwrap_or_else(|| normalize_token_name(&name_words));
    let mut definition =
        token_definition_grammar::parse_token_definition_shape_tokens(&definition_tokens)
            .or_else(|| {
                parse_prior_created_token_reference_words(&name_words)
                    .map(|_| crate::model::token_definition::TokenDefinitionSpec::PriorCreated)
            })
            .ok_or_else(|| {
                CardTextError::ParseError(format!("unsupported token definition '{name}'"))
            })?;
    if has_raw_name_override {
        match &mut definition {
            crate::model::token_definition::TokenDefinitionSpec::Vehicle(shape) => {
                shape.name = name.clone();
            }
            crate::model::token_definition::TokenDefinitionSpec::Artifact(shape) => {
                shape.name = name.clone();
            }
            crate::model::token_definition::TokenDefinitionSpec::Creature(shape) => {
                shape.name = name.clone();
            }
            _ => {}
        }
    }
    if let Some(postnominal_colors) =
        token_definition_grammar::parse_postnominal_token_colors_tokens(&tail_tokens)
    {
        match &mut definition {
            crate::model::token_definition::TokenDefinitionSpec::Creature(creature) => {
                creature.colors = creature.colors.union(postnominal_colors);
            }
            crate::model::token_definition::TokenDefinitionSpec::Artifact(artifact) => {
                artifact.colors = artifact.colors.union(postnominal_colors);
            }
            _ => {}
        }
    }
    if let crate::model::token_definition::TokenDefinitionSpec::Creature(shape) = &mut definition {
        let (use_source_chosen_color, use_source_chosen_creature_type) =
            token_definition_grammar::source_chosen_token_characteristics(&clause_words);
        shape.use_source_chosen_color |= use_source_chosen_color;
        shape.use_source_chosen_creature_type |= use_source_chosen_creature_type;
    }
    // The token-definition slice is intentionally reconstructed and may omit
    // quoted suffixes that are not needed to identify the token blueprint.
    // Ability payloads must come from the complete create clause so every
    // quoted group remains available and in source order.
    let inline_ability_presentation = (!double_quoted_rule_bodies(tokens).is_empty())
        .then_some(ironsmith_core::TokenAbilityPresentation::InlineWith);
    let mut inline_granted_abilities =
        parse_inline_token_granted_abilities(&mut definition, tokens);
    if let Some(quoted_dynamic) = parse_quoted_token_dynamic_power_toughness(tokens) {
        incorporate_quoted_dynamic_power_toughness(
            &definition,
            &mut inline_granted_abilities,
            &mut dynamic_power_toughness,
            quoted_dynamic,
        );
    } else if dynamic_power_toughness.as_ref().is_some_and(|dynamic| {
        intrinsic_token_ability_represents_dynamic_power_toughness(
            &definition,
            &inline_granted_abilities,
            dynamic,
        )
    }) {
        dynamic_power_toughness = None;
    }
    if dynamic_power_toughness.is_some()
        && let crate::model::token_definition::TokenDefinitionSpec::Vehicle(vehicle) =
            &mut definition
        && vehicle.power_toughness.is_none()
    {
        vehicle.power_toughness = Some((0, 0));
    }

    let grants_unblockable = tail_surface.has_phrase(CreatePhrase::Unblockable);

    if let Some((start, end)) = rules_text_range
        && start < end
        && end <= modifier_tail_words.len()
    {
        modifier_tail_words = modifier_tail_words[..start]
            .iter()
            .chain(modifier_tail_words[end..].iter())
            .copied()
            .collect();
    }

    if let Some(where_tokens) = creation_grammar::parse_where_clause_tokens(&tail_tokens) {
        let where_value = parse_create_value_binding(where_tokens)?.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-x clause in create clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let where_value = with_where_x_surface_hints(where_value, tokens);
        if let Some((power, toughness)) = dynamic_power_toughness.as_mut() {
            if value_contains_unbound_x(power) {
                *power = where_value.clone();
            }
            if value_contains_unbound_x(toughness) {
                *toughness = where_value.clone();
            }
        }
        if let Some(where_word_idx) = tail_surface.location(CreateWord::Where) {
            modifier_tail_words.truncate(where_word_idx);
        }
    }

    let modifier_surface = creation_grammar::CreationWords::new(&modifier_tail_words);
    tapped |= modifier_surface.has(CreateWord::Tapped);
    attacking |= modifier_surface.has(CreateWord::Attacking);
    let attack_target_player = (attacking
        && modifier_surface.has_phrase(CreatePhrase::AttackingThatPlayer))
    .then_some(PlayerAst::That);
    // Some legacy subject scans can mistake the trailing `that player` for
    // the create actor. It is the attack target; an otherwise implicit create
    // remains controlled by the ability's controller.
    if attack_target_player.is_some() && matches!(player, PlayerAst::That) {
        player = PlayerAst::You;
    }
    let (sacrifice_at_next_end_step, exile_at_next_end_step, next_end_step_player) =
        parse_next_end_step_token_delay_flags(&modifier_tail_words);
    let mut granted_abilities = inline_granted_abilities;
    if modifier_surface.has(CreateWord::Decayed) {
        granted_abilities.push(KeywordAction::Decayed.into());
    }
    if modifier_surface.has_phrase(CreatePhrase::HasteGrant) {
        granted_abilities.push(KeywordAction::Haste.into());
    }
    if grants_unblockable {
        granted_abilities.push(KeywordAction::Unblockable.into());
    }
    let references_iterated_object = attached_to_target
        .as_ref()
        .is_some_and(target_references_it);
    let create = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        player,
        SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            name,
            definition,
            count: resolve_create_count(references_iterated_object),
            dynamic_power_toughness,
            player,
            actor_surface_explicit,
            attached_to: attached_to_target,
            tapped,
            attacking,
            attack_target_player,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            next_end_step_player,
            granted_abilities,
            ability_presentation: inline_ability_presentation,
        }),
    );
    Ok(wrap_for_each_player_condition(wrap_delayed_create(
        wrap_for_each_when_needed(create, references_iterated_object),
    )))
}

/// Parse an authored resolution choice between two complete token-creation
/// instructions, such as "create a Food token or a Treasure token".
///
/// This deliberately requires both sides of the top-level `or` to parse as
/// complete direct token creations. It therefore does not reinterpret
/// disjunctions inside token characteristics, copy sources, or quoted rules
/// text as modal choices.
fn direct_token_creation_alternative_separator(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut inside_quotes = false;
    let mut separator = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if !inside_quotes
            && token.kind == TokenKind::Word
            && token.parser_text() == "or"
            && separator.replace(idx).is_some()
        {
            return None;
        }
    }

    let separator = separator?;
    let left_tokens = trim_commas(&tokens[..separator]);
    let right_tokens = trim_commas(&tokens[separator + 1..]);
    (!left_tokens.is_empty()
        && !right_tokens.is_empty()
        && [left_tokens.as_slice(), right_tokens.as_slice()]
            .iter()
            .all(|branch| {
                branch.iter().any(|token| {
                    token.kind == TokenKind::Word
                        && matches!(token.parser_text(), "token" | "tokens")
                })
            }))
    .then_some(separator)
}

/// Find an authored conjunction between complete token blueprints.
///
/// Work from right to left so an ability list in the first blueprint (for
/// example, `with flying and vigilance and a 4/4 ... token`) stays attached
/// to that blueprint. A separator is accepted only when both sides contain a
/// complete token head; ordinary keyword conjunctions therefore do not
/// become separate create actions.
fn direct_token_creation_conjunction_separator(tokens: &[OwnedLexToken]) -> Option<(usize, usize)> {
    if token_word_refs(tokens).get(..3) == Some(&["your", "choice", "of"][..]) {
        return None;
    }
    let mut inside_quotes = false;
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote { inside_quotes = !inside_quotes; continue; }
        if inside_quotes || !(token.kind == TokenKind::Comma || token.is_word("and")) { continue; }
        let mut next = idx + 1;
        if tokens.get(next).is_some_and(|token| token.is_word("and")) { next += 1; }
        let left = trim_commas(&tokens[..idx]);
        let right = trim_commas(&tokens[next..]);
        // A new creation operand starts with its own count, article, or
        // creation verb. A color conjunction belongs to the current blueprint.
        let starts_operand = right.first().is_some_and(|token|
            token.is_any_word(&["a", "an", "x", "that", "twice", "create", "creates"]))
            || crate::grammar::leaf::parse_leaf_number_prefix_tokens(&right).is_some();
        if starts_operand && creation_grammar::parse_create_head_tokens(&left).is_some()
            && creation_grammar::parse_create_head_tokens(&right).is_some() {
            return Some((idx, next));
        }
    }
    None
}

fn parse_direct_token_creation_conjunction(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let (separator, next) = direct_token_creation_conjunction_separator(tokens)?;
    let left_tokens = trim_commas(&tokens[..separator]);
    let right_tokens = trim_commas(&tokens[next..]);
    let first = crate::grammar::primitives::probe_shape(parse_create(&left_tokens, subject))?;
    let second = crate::grammar::primitives::probe_shape(parse_create(&right_tokens, subject))?;
    let mut effects = vec![first];
    match second {
        EffectAst::Coordination(program) if program.kind == crate::model::CoordinationKindAst::Conjunction => {
            effects.extend(program.members.into_iter().flat_map(|member| member.effects));
        }
        other => effects.push(other),
    }
    let coordination = creation_grammar::coordination::coordination_from_effects(
        crate::model::CoordinationKindAst::Conjunction,
        crate::model::CoordinationOperatorAst::And,
        crate::model::EffectOrderingAst::Unordered,
        effects,
    )?;
    Some(EffectAst::Coordination(coordination))
}

pub(super) fn is_direct_token_creation_alternative_candidate(tokens: &[OwnedLexToken]) -> bool {
    direct_token_creation_alternative_separator(creation_grammar::creation_body_tokens(tokens))
        .is_some()
}

fn parse_direct_token_creation_alternative(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let separator = direct_token_creation_alternative_separator(tokens)?;
    let left_tokens = trim_commas(&tokens[..separator]);
    let right_tokens = trim_commas(&tokens[separator + 1..]);

    let parse_branch = |branch: &[OwnedLexToken]| {
        let parsed = crate::grammar::primitives::probe_shape(parse_create(branch, subject))?;
        matches!(
            &parsed,
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { .. })
                )
        )
        .then_some(parsed)
    };
    let first = parse_branch(&left_tokens)?;
    let second = parse_branch(&right_tokens)?;

    Some(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf {
        modes: vec![
            ChooseOneModeAst {
                description: String::new(),
                effects: vec![first],
            },
            ChooseOneModeAst {
                description: String::new(),
                effects: vec![second],
            },
        ],
    }))
}

fn parse_create_for_each_player_condition(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<(PlayerFilter, PredicateAst)>, CardTextError> {
    let (filter, who_tokens) = if let Some(rest) =
        grammar::match_word_prefix(tokens, &["opponent", "who"])
            .or_else(|| grammar::match_word_prefix(tokens, &["opponents", "who"]))
    {
        (
            PlayerFilter::Opponent,
            &tokens[tokens.len() - rest.len() - 1..],
        )
    } else if let Some(rest) = grammar::match_word_prefix(tokens, &["player", "who"])
        .or_else(|| grammar::match_word_prefix(tokens, &["players", "who"]))
    {
        (PlayerFilter::Any, &tokens[tokens.len() - rest.len() - 1..])
    } else {
        return Ok(None);
    };

    let predicate = parse_who_player_predicate_lexed(who_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported player predicate after create for-each clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    Ok(Some((filter, predicate)))
}

pub fn normalize_token_name(words: &[&str]) -> String {
    words.join(" ")
}

fn parse_investigate_for_each_count(tokens: &[OwnedLexToken]) -> Result<Value, CardTextError> {
    creation_grammar::parse_investigate_for_each_count_tokens(tokens)
}

pub fn parse_investigate(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    if tokens.is_empty() {
        return Ok(EffectAst::subject_verb_investigate(player, Value::Fixed(1)));
    }

    if let Some(filter_tokens) = creation_grammar::parse_for_each_prefix_tokens(tokens) {
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter after 'for each' in investigate clause (clause: '{}')",
                token_word_refs(tokens).join(" ")
            )));
        }

        let count = parse_investigate_for_each_count(filter_tokens)?;

        return Ok(EffectAst::subject_verb_investigate(player, count));
    }

    let token_surface = creation_grammar::CreationTokens::new(tokens);
    let (mut count, used) = if token_surface.token_is(0, CreateWord::Once) {
        (Value::Fixed(1), 1)
    } else if token_surface.token_is(0, CreateWord::Twice) {
        (Value::Fixed(2), 1)
    } else {
        parse_value(tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing investigate count (clause: '{}')",
                token_word_refs(tokens).join(" ")
            ))
        })?
    };

    let trailing = trim_commas(&tokens[used..]);
    let trailing_words = token_word_refs(&trailing);
    if let Some(filter_tokens) = creation_grammar::parse_for_each_prefix_tokens(&trailing) {
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter after 'for each' in investigate clause (clause: '{}')",
                token_word_refs(tokens).join(" ")
            )));
        }

        let each_count = parse_investigate_for_each_count(filter_tokens)?.into_unhinted();
        count = match (count, each_count) {
            (Value::Fixed(1), Value::Count(filter)) => {
                Value::CountScaled(filter, 1).with_surface_hint(ValueSurfaceHint::ForEach)
            }
            (Value::Fixed(1), each_count) => each_count,
            (Value::Fixed(multiplier), Value::Count(filter)) => {
                Value::CountScaled(filter, multiplier).with_surface_hint(ValueSurfaceHint::ForEach)
            }
            (multiplier, each_count) => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported scaled investigate for-each clause (count: '{multiplier:?}', each: '{each_count:?}')"
                )));
            }
        };
        return Ok(EffectAst::subject_verb_investigate(player, count));
    }

    if matches!(count, Value::X)
        && creation_grammar::CreationWords::new(&trailing_words).first_is(CreateWord::Time)
        && let Some(where_tokens) = creation_grammar::parse_where_clause_tokens(&trailing)
        && let Some(where_count) = parse_create_value_binding(where_tokens)?
    {
        count = where_count;
        return Ok(EffectAst::subject_verb_investigate(player, count));
    }
    let trailing_ok = creation_grammar::parse_time_only_words(&trailing_words);
    if !trailing_ok {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing investigate clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_investigate(player, count))
}

pub fn parse_incubate(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let (mut amount, used) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing incubate amount (clause: '{}')",
            token_word_refs(tokens).join(" ")
        ))
    })?;
    let mut count = Value::Fixed(1);

    let mut trailing = trim_commas(&tokens[used..]).to_vec();
    let mut trailing_words = token_word_refs(&trailing);
    if creation_grammar::CreationWords::new(&trailing_words).first_is(CreateWord::Once) {
        count = Value::Fixed(1);
        trailing = trim_commas(&trailing[1..]).to_vec();
        trailing_words = token_word_refs(&trailing);
    } else if creation_grammar::CreationWords::new(&trailing_words).first_is(CreateWord::Twice) {
        count = Value::Fixed(2);
        trailing = trim_commas(&trailing[1..]).to_vec();
        trailing_words = token_word_refs(&trailing);
    } else if let Some((parsed_count, count_used)) = parse_value(&trailing) {
        let count_tail = trim_commas(&trailing[count_used..]).to_vec();
        let count_tail_words = token_word_refs(&count_tail);
        if creation_grammar::CreationWords::new(&count_tail_words).first_is(CreateWord::Time) {
            count = parsed_count;
            trailing = trim_commas(&count_tail[1..]).to_vec();
            trailing_words = token_word_refs(&trailing);
        }
    } else if creation_grammar::CreationWords::new(&trailing_words).first_is(CreateWord::Time) {
        trailing = trim_commas(&trailing[1..]).to_vec();
        trailing_words = token_word_refs(&trailing);
    }

    if let Some(where_tokens) = creation_grammar::parse_where_clause_tokens(&trailing) {
        let Some(where_value) = parse_create_value_binding(where_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing incubate where clause (clause: '{}')",
                token_word_refs(tokens).join(" ")
            )));
        };
        let where_value = with_where_x_surface_hints(where_value, tokens);
        if value_contains_unbound_x(&amount) {
            amount = where_value;
        } else if value_contains_unbound_x(&count) {
            count = where_value;
        } else {
            return Err(CardTextError::ParseError(format!(
                "incubate where clause did not bind X (clause: '{}')",
                token_word_refs(tokens).join(" ")
            )));
        }
        let where_word_idx = creation_grammar::CreationWords::new(&trailing_words)
            .location(CreateWord::Where)
            .unwrap_or(trailing_words.len());
        let where_token_idx = creation_grammar::CreationTokens::new(&trailing)
            .boundary(where_word_idx)
            .unwrap_or(trailing.len());
        trailing = trim_commas(&trailing[..where_token_idx]).to_vec();
        trailing_words = token_word_refs(&trailing);
    }

    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing incubate clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_incubate(player, amount, count))
}

#[cfg(test)]
mod tests {
    use super::super::super::lexer::lex_line;
    use super::*;
    use crate::cards::builders::SubjectVerbEffectAst;
    use crate::static_abilities::StaticAbilityId;
    use crate::target::{ChooseSpec, SourceReferenceSurface};
    use ironsmith_core::TurnHistoryCount;

    fn parse_token_count(clause: &str) -> Value {
        let tokens = lex_line(clause, 0).expect("token creation should lex");
        let effect = parse_create(&tokens, None).expect("token creation should parse");
        let EffectAst::SubjectVerb(effect) = effect else {
            panic!("expected a subject-verb token creation");
        };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { count, .. }) = effect.action else {
            panic!("expected a token creation with modifiers");
        };
        count
    }

    #[test]
    fn targeted_token_copy_keeps_pt_type_and_keyword_exception_bundle() {
        let tokens = lex_line(
            "Create two tokens that are copies of target noncreature permanent, except they're 3/3 Dragon creatures in addition to their other types, and they have flying.",
            0,
        )
        .expect("targeted copy-token sentence should lex");
        let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
            .expect("the complete copy exception should parse through the public effect route");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource {
                        count,
                        source: TargetAst::Object(filter, ..),
                        added_card_types,
                        added_subtypes,
                        set_base_power_toughness,
                        granted_abilities,
                        ..
                    }),
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected one typed targeted copy-token action: {effects:#?}");
        };
        assert_eq!(count, &Value::Fixed(2));
        assert!(filter.excluded_card_types.contains(&CardType::Creature));
        assert_eq!(added_card_types, &[CardType::Creature]);
        assert_eq!(added_subtypes, &[Subtype::Dragon]);
        assert_eq!(set_base_power_toughness, &Some((3, 3)));
        assert_eq!(granted_abilities.len(), 1, "{granted_abilities:#?}");

        let changed = lex_line(
            "Create two tokens that are copies of target noncreature permanent, except they're 3/3 Dragon creatures in addition to their other types.",
            0,
        )
        .expect("changed copy-token sentence should lex");
        let changed = crate::effect_sentences::parse_effect_sentence_lexed(&changed)
            .expect("the no-keyword variant should remain parseable");
        let debug = format!("{changed:#?}");
        assert!(!debug.contains("Flying"), "{debug}");
    }

    #[test]
    fn singular_copy_token_keeps_coordinated_characteristic_exception_atomic() {
        let tokens = lex_line(
            "Create a token that's a copy of that creature, except it's 1/1 and it's a Nightmare in addition to its other types.",
            0,
        )
        .expect("copy-token exception should lex");
        let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
            .expect("the complete copy-token exception should parse through the public route");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource {
                        set_base_power_toughness,
                        added_subtypes,
                        ..
                    }),
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected one typed copy-token action: {effects:#?}");
        };
        assert_eq!(set_base_power_toughness, &Some((1, 1)));
        assert_eq!(added_subtypes, &[Subtype::Nightmare]);

        let conditional = lex_line(
            "If you do, create a token that's a copy of that creature, except it's 1/1 and it's a Nightmare in addition to its other types.",
            0,
        )
        .expect("conditional copy-token exception should lex");
        let conditional = crate::effect_sentences::parse_effect_sentence_lexed(&conditional)
            .expect("nested consequence routing should retain the complete copy exception");
        assert!(
            format!("{conditional:#?}").contains("CreateTokenCopyFromSource"),
            "conditional route lost the copy-token action: {conditional:#?}"
        );

        let ordinary = lex_line("Create a 1/1 Nightmare creature token and draw a card.", 0)
            .expect("ordinary coordinated creation should lex");
        let ordinary = crate::effect_sentences::parse_effect_sentence_lexed(&ordinary)
            .expect("ordinary coordinated creation should keep its separate draw");
        assert!(
            ordinary.iter().any(|effect| {
                matches!(
                    effect,
                    EffectAst::Coordination(coordination)
                        if coordination.members.len() == 2
                )
            }) || ordinary.len() == 2,
            "non-copy coordination must not be absorbed: {ordinary:#?}"
        );
    }

    #[test]
    fn later_quoted_set_grant_is_not_attached_to_token_blueprint() {
        let create_tokens = lex_line("Each player creates a green Elephant creature token.", 0)
            .expect("token sentence should lex");
        let mut effects = crate::effect_sentences::parse_effect_sentence_lexed(&create_tokens)
            .expect("quantified token sentence should parse");
        assert!(
            matches!(
                effects.as_slice(),
                [EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects: nested })]
                    if matches!(
                        nested.as_slice(),
                        [EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                            action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { .. }),
                            ..
                        })]
                    )
            ),
            "the quantified creator must remain the token controller: {effects:#?}"
        );
        let authored_tokens = lex_line(
            "Each player creates a green Elephant creature token. Those creatures have \"This token's power and toughness are each equal to the number of creature cards in its controller's graveyard.\"",
            0,
        )
        .expect("two-sentence token rule should lex");

        assert!(
            inline_quoted_token_creation_sentence(&authored_tokens).is_none(),
            "a quote in the following sentence is an external set grant"
        );
        assert!(!attach_inline_token_granted_abilities_to_last_create(
            &mut effects,
            &authored_tokens,
        ));
        assert!(
            !format!("{effects:#?}").contains("CharacteristicDefining"),
            "the later set grant must not be duplicated inside the token definition: {effects:#?}"
        );

        let inline_tokens = lex_line(
            "Create a green Elephant creature token with \"This token's power and toughness are each equal to the number of creature cards in its controller's graveyard.\"",
            0,
        )
        .expect("inline token rule should lex");
        assert!(inline_quoted_token_creation_sentence(&inline_tokens).is_some());
    }

    #[test]
    fn direct_token_creation_or_lowers_to_two_complete_choice_modes() {
        let tokens = lex_line("Create a Food token or a Treasure token.", 0)
            .expect("token alternative should lex");
        let parsed = parse_create(&tokens, None).expect("token creation alternative should parse");
        let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf { modes }) = parsed else {
            panic!("expected a typed token-creation choice, got {parsed:#?}");
        };
        assert_eq!(modes.len(), 2);

        let names = modes
            .iter()
            .map(|mode| {
                let [EffectAst::SubjectVerb(effect)] = mode.effects.as_slice() else {
                    panic!("expected one direct create effect per mode: {mode:#?}");
                };
                let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { name, .. }) = &effect.action else {
                    panic!("expected a named token creation: {effect:#?}");
                };
                name.as_str()
            })
            .collect::<Vec<_>>();
        assert_eq!(names, ["Food", "Treasure"]);
        assert!(modes.iter().all(|mode| mode.description.is_empty()));
    }

    #[test]
    fn serial_token_blueprints_keep_their_own_stats_and_colors() {
        fn collect(effect: &EffectAst, result: &mut Vec<(i32, i32)>) {
            match effect {
                EffectAst::Coordination(program) => {
                    for member in &program.members {
                        for effect in &member.effects { collect(effect, result); }
                    }
                }
                EffectAst::SubjectVerb(effect) => {
                    let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { definition, .. }) = &effect.action else {
                        panic!("expected token creation: {effect:#?}");
                    };
                    let crate::model::token_definition::TokenDefinitionSpec::Creature(creature) = definition else {
                        panic!("expected creature blueprint: {definition:#?}");
                    };
                    assert_eq!(creature.colors.count(), 2);
                    result.push(creature.power_toughness);
                }
                _ => panic!("unexpected token program: {effect:#?}"),
            }
        }
        let tokens = lex_line("Create a 2/3 white and blue Bird creature token with flying, a 4/5 black and red Spirit creature token, and a 6/7 green and white Beast creature token.", 0).unwrap();
        let parsed = parse_create(&tokens, None).unwrap();
        let mut stats = Vec::new();
        collect(&parsed, &mut stats);
        assert_eq!(stats, [(2, 3), (4, 5), (6, 7)]);
    }

    #[test]
    fn filter_disjunction_is_not_a_direct_token_creation_alternative() {
        let tokens = lex_line(
            "Create a Treasure token for each tapped Assassin, Pirate, and/or Vehicle you control.",
            0,
        )
        .expect("token creation should lex");
        assert!(!is_direct_token_creation_alternative_candidate(&tokens));
    }

    #[test]
    fn postnominal_token_colors_preserve_compound_subtypes_and_colors() {
        let tokens = lex_line(
            "Create a 1/1 Sand Warrior creature token that is red, green, and white.",
            0,
        )
        .expect("Sand Warrior creation should lex");
        let effect = parse_create(&tokens, None).expect("Sand Warrior creation should parse");
        let EffectAst::SubjectVerb(effect) = effect else {
            panic!("expected a subject-verb token creation");
        };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { definition, .. }) = effect.action else {
            panic!("expected a token creation with modifiers");
        };
        let crate::model::token_definition::TokenDefinitionSpec::Creature(creature) = definition
        else {
            panic!("expected a creature token definition");
        };

        assert_eq!(creature.subtypes, vec![Subtype::Sand, Subtype::Warrior]);
        assert_eq!(
            creature.colors,
            ColorSet::RED.union(ColorSet::GREEN).union(ColorSet::WHITE)
        );
    }

    #[test]
    fn contracted_postnominal_token_colors_are_typed() {
        let tokens = lex_line(
            "Create an 8/8 Beast creature token that's red, green, and white.",
            0,
        )
        .expect("contracted postnominal color creation should lex");
        let effect = parse_create(&tokens, None).expect("postnominal color creation should parse");
        let EffectAst::SubjectVerb(effect) = effect else {
            panic!("expected a subject-verb token creation");
        };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { definition, .. }) = effect.action else {
            panic!("expected a token creation with modifiers");
        };
        let crate::model::token_definition::TokenDefinitionSpec::Creature(creature) = definition
        else {
            panic!("expected a creature token definition");
        };

        assert_eq!(
            creature.colors,
            ColorSet::RED.union(ColorSet::GREEN).union(ColorSet::WHITE)
        );
    }

    #[test]
    fn inline_artifact_token_rules_parse_each_quoted_ability() {
        let tokens = lex_line(
            "Create Tamiyo's Notebook, a legendary colorless Book artifact token with \"Spells you cast cost {2} less to cast\" and \"{T}: Draw a card.\"",
            0,
        )
        .expect("Notebook creation should lex");
        let definition = token_definition_grammar::parse_token_definition_shape_text(
            "Tamiyo's Notebook, a legendary colorless Book artifact token",
        )
        .expect("Notebook token definition");
        let parsed = double_quoted_rule_bodies(&tokens)
            .into_iter()
            .map(|body| {
                super::super::parse_granted_abilities_for_token_definition(&definition, body)
            })
            .collect::<Vec<_>>();

        assert_eq!(parsed.len(), 2, "{parsed:#?}");
        assert!(parsed.iter().all(Result::is_ok), "{parsed:#?}");
        assert_eq!(
            parsed
                .iter()
                .filter_map(|result| crate::grammar::primitives::probe_shape(result.as_ref()))
                .map(Vec::len)
                .sum::<usize>(),
            2,
            "{parsed:#?}"
        );

        let ast = parse_create(&tokens, None).expect("create AST");
        let EffectAst::SubjectVerb(effect) = &ast else {
            panic!("expected subject-verb create AST");
        };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            granted_abilities, ..
        }) = &effect.action
        else {
            panic!("expected create-token AST");
        };
        assert!(
            matches!(
                granted_abilities.as_slice(),
                [
                    GrantedAbilityAst::StaticAbility(_),
                    GrantedAbilityAst::ParsedObjectAbility { .. },
                ]
            ),
            "{granted_abilities:#?}"
        );

        let (effects, _) = crate::compile_support::compile_effect(
            &ast,
            &mut crate::model::facts::EffectLoweringContext::new(),
        )
        .expect("create AST should lower");
        let create = effects
            .iter()
            .find_map(crate::effect::Effect::as_create_token)
            .expect("lowered create-token effect");
        assert!(
            create
                .token
                .abilities
                .iter()
                .any(|ability| matches!(ability.kind, crate::ability::AbilityKind::Activated(_))),
            "{:#?}",
            create.token.abilities
        );

        let sentence_effects = super::super::parse_effect_sentences_lexed(&tokens)
            .expect("Notebook sentence should parse through production dispatch");
        let (effects, _) = crate::compile_support::compile_effects(
            &sentence_effects,
            &mut crate::model::facts::EffectLoweringContext::new(),
        )
        .expect("production Notebook AST should lower");
        let create = effects
            .iter()
            .find_map(crate::effect::Effect::as_create_token)
            .expect("production create-token effect");
        assert!(
            create
                .token
                .abilities
                .iter()
                .any(|ability| matches!(ability.kind, crate::ability::AbilityKind::Activated(_))),
            "sentence AST: {sentence_effects:#?}\nlowered: {:#?}",
            create.token.abilities
        );
    }

    #[test]
    fn nested_token_keywords_stay_inside_the_quoted_token_rule() {
        let tokens = lex_line(
            "Create a 0/2 red Dragon Egg creature token with defender and \"When this token dies, create a 2/2 red Dragon creature token with flying and '{R}: This token gets +1/+0 until end of turn.'\".",
            0,
        )
        .expect("nested token creation should lex");
        let sentence_effects = super::super::parse_effect_sentences_lexed(&tokens)
            .expect("nested token creation should parse through production dispatch");
        let (effects, _) = crate::compile_support::compile_effects(
            &sentence_effects,
            &mut crate::model::facts::EffectLoweringContext::new(),
        )
        .expect("nested token creation should lower");
        let egg = effects
            .iter()
            .find_map(crate::effect::Effect::as_create_token)
            .expect("outer effect should create the Egg");

        let outer_static_ids = egg
            .token
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                crate::ability::AbilityKind::Static(static_ability) => Some(static_ability.id()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            outer_static_ids.contains(&StaticAbilityId::Defender),
            "the outer Egg should retain defender: {:#?}",
            egg.token.abilities
        );
        assert!(
            !outer_static_ids.contains(&StaticAbilityId::Flying),
            "flying from the quoted nested Dragon must not attach to the outer Egg: {:#?}",
            egg.token.abilities
        );

        let dies_trigger = egg
            .token
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                crate::ability::AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("the outer Egg should retain its quoted dies trigger");
        let dragon = dies_trigger
            .effects
            .flattened_default_effects()
            .iter()
            .find_map(crate::effect::Effect::as_create_token)
            .expect("the Egg's dies trigger should create the inner Dragon");
        assert!(
            dragon.token.abilities.iter().any(|ability| {
                matches!(
                    &ability.kind,
                    crate::ability::AbilityKind::Static(static_ability)
                        if static_ability.id() == StaticAbilityId::Flying
                )
            }),
            "the nested Dragon should retain flying: {:#?}",
            dragon.token.abilities
        );
        assert!(
            dragon
                .token
                .abilities
                .iter()
                .any(|ability| matches!(ability.kind, crate::ability::AbilityKind::Activated(_))),
            "the nested Dragon should retain firebreathing: {:#?}",
            dragon.token.abilities
        );
    }

    #[test]
    fn separate_token_pronoun_sentence_attaches_quoted_activation() {
        let tokens = lex_line(
            "Create a 1/1 colorless Triskelavite artifact creature token with flying. It has \"Sacrifice this token: This token deals 1 damage to any target.\"",
            0,
        )
        .expect("Triskelavite creation should lex");
        let quoted = double_quoted_rule_bodies(&tokens);
        assert_eq!(quoted.len(), 1, "{tokens:#?}");
        let definition = token_definition_grammar::parse_token_definition_shape_text(
            "1/1 colorless Triskelavite artifact creature token with flying",
        )
        .expect("Triskelavite token definition");
        let parsed =
            super::super::parse_granted_abilities_for_token_definition(&definition, quoted[0])
                .expect("quoted sacrifice activation should parse");
        assert!(
            matches!(
                parsed.as_slice(),
                [GrantedAbilityAst::ParsedObjectAbility { .. }]
            ),
            "{parsed:#?}"
        );

        let sentence_effects = super::super::parse_effect_sentences_lexed(&tokens)
            .expect("Triskelavite sentence should parse through production dispatch");
        let (effects, _) = crate::compile_support::compile_effects(
            &sentence_effects,
            &mut crate::model::facts::EffectLoweringContext::new(),
        )
        .expect("production Triskelavite AST should lower");
        let create = effects
            .iter()
            .find_map(crate::effect::Effect::as_create_token)
            .expect("production create-token effect");
        assert!(
            create
                .token
                .abilities
                .iter()
                .any(|ability| matches!(ability.kind, crate::ability::AbilityKind::Activated(_))),
            "sentence AST: {sentence_effects:#?}\nlowered: {:#?}",
            create.token.abilities
        );
    }

    #[test]
    fn dynamic_construct_pt_uses_a_zero_definition_without_artifact_scaling() {
        let tokens = lex_line(
            "Create an X/X colorless Construct artifact creature token, where X is the number of creatures you control.",
            0,
        )
        .expect("dynamic Construct creation should lex");
        let effect = parse_create(&tokens, None).expect("dynamic Construct creation should parse");
        let EffectAst::SubjectVerb(effect) = effect else {
            panic!("expected a subject-verb token creation");
        };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            definition,
            dynamic_power_toughness,
            ..
        }) = effect.action
        else {
            panic!("expected a token creation with modifiers");
        };
        let crate::model::token_definition::TokenDefinitionSpec::Construct(construct) = definition
        else {
            panic!("expected a Construct token definition");
        };

        assert_eq!(construct.power_toughness, (0, 0));
        assert_eq!(construct.artifact_scaling, None);
        assert!(dynamic_power_toughness.is_some());
    }

    #[test]
    fn unquoted_dynamic_base_pt_keeps_the_one_or_more_zone_change_group() {
        let tokens = lex_line(
            "create a green Fungus Dinosaur creature token with base power and toughness each equal to the total power of those creatures.",
            0,
        )
        .expect("unquoted dynamic token creation should lex");
        let effect = parse_create(&tokens, None)
            .expect("unquoted dynamic token creation should parse through production");
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            panic!("expected a subject-verb token creation");
        };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            dynamic_power_toughness: Some((power, toughness)),
            ..
        }) = subject_verb.action
        else {
            panic!("expected a token creation with typed dynamic base P/T");
        };
        for value in [power, toughness] {
            let Value::TotalPower(filter) = value else {
                panic!("expected total power of the matched death group, got {value:#?}");
            };
            assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
            assert!(filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str()
                        == crate::tag::CompilerReferenceTag::ZoneChangeGroup.as_str()
            }));
        }
    }

    #[test]
    fn quoted_dynamic_token_pt_is_represented_once_as_an_intrinsic_ability() {
        let tokens = lex_line(
            "Create a green Ooze creature token with \"This token's power and toughness are each equal to the number of slime counters on this enchantment.\"",
            0,
        )
        .expect("quoted dynamic token creation should lex");
        let effect =
            parse_create(&tokens, None).expect("quoted dynamic token creation should parse");
        let EffectAst::SubjectVerb(effect) = effect else {
            panic!("expected a subject-verb token creation");
        };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            dynamic_power_toughness,
            granted_abilities,
            ..
        }) = effect.action
        else {
            panic!("expected a token creation with modifiers");
        };
        assert_eq!(dynamic_power_toughness, None);
        assert_eq!(granted_abilities.len(), 1, "{granted_abilities:#?}");
        assert!(
            format!("{granted_abilities:#?}").contains("CharacteristicDefining"),
            "{granted_abilities:#?}"
        );
    }

    #[test]
    fn quoted_distinct_dynamic_token_pt_is_an_intrinsic_ability() {
        let tokens = lex_line(
            "Create a green Ooze creature token with \"This token's power is equal to the number of card types among cards in your graveyard and its toughness is equal to that number plus 1.\"",
            0,
        )
        .expect("quoted distinct dynamic token creation should lex");
        let effect =
            parse_create(&tokens, None).expect("quoted dynamic token creation should parse");
        let EffectAst::SubjectVerb(effect) = effect else {
            panic!("expected a subject-verb token creation");
        };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            dynamic_power_toughness,
            granted_abilities,
            ..
        }) = effect.action
        else {
            panic!("expected a token creation with modifiers");
        };

        assert_eq!(dynamic_power_toughness, None);
        assert_eq!(granted_abilities.len(), 1, "{granted_abilities:#?}");
        assert!(
            format!("{granted_abilities:#?}").contains("CharacteristicDefining"),
            "{granted_abilities:#?}"
        );
    }

    #[test]
    fn direct_effect_dispatch_keeps_multiple_quoted_rules_in_token_blueprint() {
        let tokens = lex_line(
            "Create a 0/1 colorless Goblin Construct artifact creature token with \"This token can't block\" and \"At the beginning of your upkeep, this token deals 1 damage to you.\"",
            0,
        )
        .expect("multi-rule token creation should lex");
        let quoted = double_quoted_rule_bodies(&tokens);
        assert_eq!(quoted.len(), 2, "{tokens:#?}");
        let definition = token_definition_grammar::parse_token_definition_shape_text(
            "0/1 colorless Goblin Construct artifact creature token",
        )
        .expect("Goblin Construct token definition");
        let upkeep_rule =
            super::super::parse_granted_abilities_for_token_definition(&definition, quoted[1])
                .expect("quoted upkeep-damage rule should parse");
        assert!(
            matches!(
                upkeep_rule.as_slice(),
                [GrantedAbilityAst::ParsedObjectAbility { .. }]
            ),
            "{upkeep_rule:#?}"
        );
        let ast = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
            .expect("single-sentence dispatcher should parse the token creation");
        let (effects, _) = crate::compile_support::compile_effects(
            &ast,
            &mut crate::model::facts::EffectLoweringContext::new(),
        )
        .expect("multi-rule token creation should lower");
        let create = effects
            .iter()
            .find_map(crate::effect::Effect::as_create_token)
            .expect("lowered create-token effect");

        assert_eq!(effects.len(), 1, "{effects:#?}");
        assert!(
            create
                .token
                .abilities
                .iter()
                .any(|ability| matches!(ability.kind, crate::ability::AbilityKind::Static(_))),
            "{:#?}",
            create.token.abilities
        );
        assert!(
            create
                .token
                .abilities
                .iter()
                .any(|ability| matches!(ability.kind, crate::ability::AbilityKind::Triggered(_))),
            "{:#?}",
            create.token.abilities
        );
    }

    #[test]
    fn quoted_external_dynamic_pt_becomes_one_creator_bound_intrinsic_cda() {
        let tokens = lex_line(
            "Create a green Ooze creature token with \"This token's power and toughness are each equal to the number of slime counters on Gutter Grime.\"",
            0,
        )
        .expect("quoted external dynamic token creation should lex");
        let quoted_dynamic = parse_quoted_token_dynamic_power_toughness(&tokens)
            .expect("quoted external dynamic P/T rule should parse");
        assert_eq!(quoted_dynamic.0, quoted_dynamic.1);
        let effect = parse_create(&tokens, None)
            .expect("quoted external dynamic token creation should parse");
        let lowered_ast = effect.clone();
        let EffectAst::SubjectVerb(effect) = effect else {
            panic!("expected a subject-verb token creation");
        };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            dynamic_power_toughness,
            granted_abilities,
            ..
        }) = effect.action
        else {
            panic!("expected a token creation with modifiers");
        };
        assert_eq!(dynamic_power_toughness, None);
        let [GrantedAbilityAst::StaticAbility(static_ability)] = granted_abilities.as_slice()
        else {
            panic!("expected one intrinsic token CDA: {granted_abilities:#?}");
        };
        let crate::cards::builders::StaticAbilityAst::Static(static_ability) =
            static_ability.as_ref()
        else {
            panic!("expected a compiler static ability: {static_ability:#?}");
        };
        let ironsmith_core::StaticAbilityPayload::CharacteristicDefiningPt { power, toughness } =
            &static_ability.payload
        else {
            panic!("expected a characteristic-defining P/T ability: {static_ability:#?}");
        };
        assert_eq!(power, toughness);
        let Value::CountersOn(spec, Some(crate::CounterType::Named(counter_name))) =
            power.unhinted()
        else {
            panic!("expected a named-source counter value: {power:#?}");
        };
        assert_eq!(counter_name.as_str(), "slime");
        assert!(matches!(spec.base(), ChooseSpec::Source));
        assert_eq!(
            spec.source_reference_surface(),
            Some(&SourceReferenceSurface::FullName(
                "Gutter Grime".to_string()
            ))
        );

        let (effects, _) = crate::compile_support::compile_effect(
            &lowered_ast,
            &mut crate::model::facts::EffectLoweringContext::new(),
        )
        .expect("creator-bound token CDA should lower");
        assert!(
            !format!("{effects:#?}").contains("SetBasePowerToughnessEffect"),
            "{effects:#?}"
        );
        let create = effects
            .iter()
            .find_map(crate::effect::Effect::as_create_token)
            .expect("lowered create-token effect");
        assert_eq!(
            create
                .token
                .abilities
                .iter()
                .filter(|ability| matches!(
                    &ability.kind,
                    crate::ability::AbilityKind::Static(static_ability)
                        if static_ability.id() == StaticAbilityId::CharacteristicDefiningPT
                ))
                .count(),
            1,
            "{:#?}",
            create.token.abilities
        );

        let sentence_effects = super::super::parse_effect_sentences_lexed(&tokens)
            .expect("creator-bound token sentence should parse through production dispatch");
        let (effects, _) = crate::compile_support::compile_effects(
            &sentence_effects,
            &mut crate::model::facts::EffectLoweringContext::new(),
        )
        .expect("production creator-bound token sentence should lower");
        assert!(
            !format!("{effects:#?}").contains("SetBasePowerToughnessEffect"),
            "production dispatch duplicated the creator-bound CDA: {effects:#?}"
        );
        let create = effects
            .iter()
            .find_map(crate::effect::Effect::as_create_token)
            .expect("production lowered create-token effect");
        assert_eq!(
            create
                .token
                .abilities
                .iter()
                .filter(|ability| matches!(
                    &ability.kind,
                    crate::ability::AbilityKind::Static(static_ability)
                        if static_ability.id() == StaticAbilityId::CharacteristicDefiningPT
                ))
                .count(),
            1,
            "{:#?}",
            create.token.abilities
        );
    }

    #[test]
    fn quoted_union_count_cda_lowers_once_without_an_outer_base_pt_effect() {
        let tokens = lex_line(
            "Create a white Gnome Soldier artifact creature token with \"This token's power and toughness are each equal to the number of artifacts and/or creatures you control.\"",
            0,
        )
        .expect("quoted dynamic token creation should lex");
        let ast = parse_create(&tokens, None).expect("quoted dynamic token creation should parse");
        let EffectAst::SubjectVerb(subject_verb) = &ast else {
            panic!("expected a subject-verb token creation");
        };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            definition,
            dynamic_power_toughness,
            granted_abilities,
            ..
        }) = &subject_verb.action
        else {
            panic!("expected a token creation with modifiers");
        };
        assert_eq!(dynamic_power_toughness, &None);
        assert_eq!(granted_abilities.len(), 1, "{granted_abilities:#?}");
        let crate::model::token_definition::TokenDefinitionSpec::Creature(creature) = definition
        else {
            panic!("expected a creature token definition");
        };
        assert!(
            creature.rules.token_rules.embedded_rules.is_empty(),
            "{:#?}",
            creature.rules.token_rules
        );

        let (effects, _) = crate::compile_support::compile_effect(
            &ast,
            &mut crate::model::facts::EffectLoweringContext::new(),
        )
        .expect("create AST should lower");
        assert!(
            !format!("{effects:#?}").contains("SetBasePowerToughnessEffect"),
            "{effects:#?}"
        );
        let create = effects
            .iter()
            .find_map(crate::effect::Effect::as_create_token)
            .expect("lowered create-token effect");
        let characteristic_abilities = create
            .token
            .abilities
            .iter()
            .filter(|ability| {
                matches!(
                    &ability.kind,
                    crate::ability::AbilityKind::Static(static_ability)
                        if static_ability.id() == StaticAbilityId::CharacteristicDefiningPT
                )
            })
            .count();
        assert_eq!(characteristic_abilities, 1, "{:#?}", create.token.abilities);

        // Production sentence dispatch strips the quoted rule before parsing
        // the outer create action, then reattaches it as a parsed object
        // ability. That representation must suppress the same compatibility
        // fallback as the direct create parser above.
        let sentence_effects = super::super::parse_effect_sentences_lexed(&tokens)
            .expect("quoted dynamic token sentence should parse through production dispatch");
        let (effects, _) = crate::compile_support::compile_effects(
            &sentence_effects,
            &mut crate::model::facts::EffectLoweringContext::new(),
        )
        .expect("production token sentence should lower");
        assert!(
            !format!("{effects:#?}").contains("SetBasePowerToughnessEffect"),
            "production sentence dispatch duplicated the intrinsic token CDA: {effects:#?}"
        );
        let create = effects
            .iter()
            .find_map(crate::effect::Effect::as_create_token)
            .expect("production lowered create-token effect");
        let characteristic_abilities = create
            .token
            .abilities
            .iter()
            .filter(|ability| {
                matches!(
                    &ability.kind,
                    crate::ability::AbilityKind::Static(static_ability)
                        if static_ability.id() == StaticAbilityId::CharacteristicDefiningPT
                )
            })
            .count();
        assert_eq!(characteristic_abilities, 1, "{:#?}", create.token.abilities);
    }

    #[test]
    fn dynamic_token_count_cards_keep_equal_to_and_for_each_semantics() {
        let shared_graveyard_union = parse_token_count(
            "Create a 1/1 green Insect creature token for each artifact and/or creature card in your graveyard.",
        );
        let Value::Count(shared_graveyard_filter) = shared_graveyard_union.unhinted() else {
            panic!("expected a typed shared-domain count: {shared_graveyard_union:#?}");
        };
        assert!(
            shared_graveyard_filter.any_of.is_empty(),
            "{shared_graveyard_filter:#?}"
        );
        assert_eq!(
            shared_graveyard_filter.card_types,
            [CardType::Artifact, CardType::Creature],
            "{shared_graveyard_filter:#?}"
        );
        assert_eq!(
            shared_graveyard_filter.owner,
            Some(PlayerFilter::You),
            "{shared_graveyard_filter:#?}"
        );
        assert_eq!(
            shared_graveyard_filter.zone,
            Some(crate::Zone::Graveyard),
            "{shared_graveyard_filter:#?}"
        );

        let ferrafor = parse_token_count(
            "Create a number of 1/1 green Saproling creature tokens equal to the number of counters among creatures target player controls.",
        );
        assert!(ferrafor.has_surface_hint(ValueSurfaceHint::EqualTo));
        assert!(matches!(ferrafor.unhinted(), Value::CountersOn(_, None)));

        let hare = parse_token_count(
            "Create a number of 1/1 white Rabbit creature tokens equal to the number of other creatures you control named Hare Apparent.",
        );
        assert!(hare.has_surface_hint(ValueSurfaceHint::EqualTo));
        assert!(matches!(hare.unhinted(), Value::Count(_)));

        let heidegger = parse_token_count(
            "Create a number of 1/1 white Soldier creature tokens equal to the number of opponents who control more creatures than you.",
        );
        assert!(heidegger.has_surface_hint(ValueSurfaceHint::EqualTo));
        assert!(matches!(
            heidegger.unhinted(),
            Value::PlayersWhoControlMoreThanYou {
                players: PlayerFilter::Opponent,
                filter,
            } if filter.card_types == [CardType::Creature]
        ));

        let prior_result =
            parse_token_count("Create a number of Treasure tokens equal to the result.");
        assert!(prior_result.has_surface_hint(ValueSurfaceHint::EqualTo));
        assert!(prior_result.has_surface_hint(ValueSurfaceHint::PriorEffectResult));

        let hornbeetle = parse_token_count(
            "Create a 1/1 green Insect creature token for each +1/+1 counter you've put on creatures under your control this turn.",
        );
        assert!(hornbeetle.has_surface_hint(ValueSurfaceHint::ForEach));
        assert!(matches!(
            hornbeetle.unhinted(),
            Value::TurnHistoryCount(TurnHistoryCount::CountersPutOn {
                counter_type: Some(crate::object::CounterType::PlusOnePlusOne),
                filter,
            }) if filter.card_types == [CardType::Creature]
                && filter.controller == Some(PlayerFilter::You)
        ));
    }

    #[test]
    fn quoted_copy_exception_lowers_source_relative_equip_cost_reduction() {
        let tokens = lex_line(
            "create a token that's a copy of it, except it has \"This Equipment's equip abilities cost {2} less to activate.\"",
            0,
        )
        .expect("copy exception should lex");
        let abilities = parse_inline_copy_granted_abilities(&tokens);
        assert!(
            abilities.iter().any(|ability| {
                matches!(
                    ability,
                    GrantedAbilityAst::StaticAbility(ability)
                        if matches!(
                            ability.as_ref(),
                            crate::cards::builders::StaticAbilityAst::Static(ability)
                                if ability.id()
                                    == StaticAbilityId::ActivatedAbilityCostReduction
                        )
                ) && format!("{ability:?}")
                    .to_ascii_lowercase()
                    .contains("equip abilities cost")
            }),
            "expected typed equip cost reduction, got {abilities:#?}"
        );
    }

    #[test]
    fn saw_in_half_copy_exception_survives_public_sentence_routing() {
        for qualifier in ["power", "base power"] {
            let text = format!(
                "Destroy target creature. If that creature dies this way, its controller creates two tokens that are copies of that creature, except their {qualifier} is half that creature's power and their {qualifier_toughness} is half that creature's toughness. Round up each time.",
                qualifier_toughness = if qualifier == "power" {
                    "toughness"
                } else {
                    "base toughness"
                }
            );
            let tokens = lex_line(&text, 0).expect("Saw in Half text should lex");
            let effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens)
                .expect("Saw in Half text should parse through the public route");
            let [
                _,
                EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                    effects: result, ..
                }),
            ] = effects.as_slice()
            else {
                panic!("expected destroy followed by result-gated copy: {effects:#?}");
            };
            let [EffectAst::SubjectVerb(copy)] = result.as_slice() else {
                panic!("expected one typed copy action: {result:#?}");
            };
            assert!(matches!(
                &copy.action,
                SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource {
                    half_power_toughness_round_up: true,
                    ..
                })
            ));
        }
    }

    #[test]
    fn quoted_copy_exception_lowers_a_conditional_fight_trigger() {
        let tokens = lex_line(
            "create a token that's a copy of target permanent, except the token has \"When this token enters, if it's a creature, it fights up to one target creature you don't control.\"",
            0,
        )
        .expect("copy exception should lex");
        let abilities = parse_inline_copy_granted_abilities(&tokens);
        assert!(
            abilities.iter().any(|ability| {
                let debug = format!("{ability:#?}").to_ascii_lowercase();
                debug.contains("fight") && debug.contains("creature")
            }),
            "expected a typed conditional fight trigger carrier, got {abilities:#?}"
        );
    }

    #[test]
    fn mixed_pronoun_token_rules_keep_prefix_quote_and_trailing_activation() {
        let create_tokens = lex_line(
            "Create a colorless Equipment artifact token named Stoneforged Blade.",
            0,
        )
        .expect("token creation should lex");
        let mut effects =
            vec![parse_create(&create_tokens, None).expect("token creation should parse")];
        let rule_tokens = lex_line(
            "It has indestructible, \"Equipped creature gets +5/+5 and has double strike,\" and equip {0}.",
            0,
        )
        .expect("mixed token rules should lex");
        assert!(attach_mixed_pronoun_token_rules_to_last_create(
            &mut effects,
            &rule_tokens
        ));
        let [effect] = effects.as_slice() else {
            panic!("expected one token creation: {effects:#?}");
        };
        let EffectAst::SubjectVerb(subject_verb) = &effect else {
            panic!("expected token creation: {effect:#?}");
        };
        let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            granted_abilities,
            ability_presentation,
            ..
        }) = &subject_verb.action
        else {
            panic!("expected token creation with modifiers: {subject_verb:#?}");
        };
        assert_eq!(
            *ability_presentation,
            Some(ironsmith_core::TokenAbilityPresentation::SeparateSentenceCombined)
        );
        let debug = format!("{granted_abilities:#?}");
        assert!(debug.contains("Indestructible"), "{debug}");
        assert!(debug.contains("Anthem"), "{debug}");
        assert!(debug.contains("DoubleStrike"), "{debug}");
        assert!(debug.contains("Equip {0}"), "{debug}");
    }

    #[test]
    fn named_inline_token_death_trigger_keeps_authored_intro_frequency() {
        for (intro, expected) in [
            ("When", crate::model::ast::TriggerIntroSurfaceAst::When),
            (
                "Whenever",
                crate::model::ast::TriggerIntroSurfaceAst::Whenever,
            ),
        ] {
            let tokens = lex_line(
                &format!(
                    "Create Ember, a legendary 6/6 red Dragon creature token with flying and \"{intro} Ember dies, create two Treasure tokens.\""
                ),
                0,
            )
            .expect("named token trigger should lex");
            let effect = parse_create(&tokens, None).expect("named token trigger should parse");
            let EffectAst::SubjectVerb(subject_verb) = effect else {
                panic!("expected token creation: {effect:#?}");
            };
            let SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                granted_abilities, ..
            }) = subject_verb.action
            else {
                panic!("expected token creation modifiers: {subject_verb:#?}");
            };
            let typed_intro = granted_abilities.iter().find_map(|ability| {
                let GrantedAbilityAst::ParsedObjectAbility { ability, .. } = ability else {
                    return None;
                };
                let crate::model::ast::TriggerSpec::WithIntro { intro, .. } =
                    ability.trigger_spec.as_deref()?
                else {
                    return None;
                };
                Some(*intro)
            });
            assert_eq!(typed_intro, Some(expected), "{granted_abilities:#?}");
        }
    }
}
