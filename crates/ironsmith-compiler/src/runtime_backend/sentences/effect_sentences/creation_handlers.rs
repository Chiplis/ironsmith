use crate::cards::builders::{
    CardTextError, ChooseOneModeAst, EffectAst, GrantedAbilityAst, IT_TAG, KeywordAction,
    ObjectRefAst, OwnedLexToken, PlayerAst, PredicateAst, SubjectAst, SubjectVerbActionAst,
    SubjectVerbRoleAst, TagKey, TargetAst,
};
use crate::color::ColorSet;
use crate::effect::Value;
use crate::static_abilities::StaticAbility;
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

fn parse_create_value_binding(tokens: &[OwnedLexToken]) -> Option<Value> {
    crate::runtime_backend::families::keyword_static::parse_where_x_is_aggregate_filter_value(tokens)
        .or_else(|| {
            crate::runtime_backend::front_end::grammar::shared_util::value_semantics::parse_turn_history_value_binding(tokens)
        })
        // This is a count of qualifying players, not of the objects named at
        // the end of the clause. Keep it ahead of the broad number-of-filter
        // parser, which can otherwise retain only "creatures" or "lands".
        .or_else(|| {
            crate::runtime_backend::front_end::grammar::values::parse_players_who_control_more_than_you_value_lexed(
                tokens,
            )
        })
        // Prefer the complete typed number-of filter family before the broad
        // value-binding dispatcher. In particular, "the number of abilities
        // from among ... found among creatures" is a StaticAbilitiesAmong
        // aggregate, not merely the number of creatures in its scope.
        .or_else(|| {
            crate::runtime_backend::families::keyword_static::parse_where_x_is_number_of_filter_value(
                tokens,
            )
        })
        .or_else(|| parse_value_binding_clause(tokens))
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

pub(crate) fn is_probable_token_name_word(word: &str) -> bool {
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

pub(crate) fn parse_copy_modifiers_from_tail(
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
        parsed.granted_abilities,
        parsed.loses_soulbond,
    ))
}

pub(crate) fn parse_next_end_step_token_delay_flags(
    tail_words: &[&str],
) -> (bool, bool, PlayerFilter) {
    super::super::util::parse_next_end_step_token_delay_flags(tail_words)
}

pub(crate) fn trailing_create_at_next_end_step_clause(
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
    Ok(parse_create_value_binding(&synthetic_tokens).map(|value| {
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
    bodies
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
    token_definition_grammar::parse_token_dynamic_power_toughness_tokens(&unquoted)
}

fn parse_quoted_token_dynamic_power_toughness(tokens: &[OwnedLexToken]) -> Option<(Value, Value)> {
    double_quoted_rule_bodies(tokens)
        .into_iter()
        .find_map(token_definition_grammar::parse_token_dynamic_power_toughness_tokens)
}

fn quoted_copy_sacrifice_ability_text(tokens: &[OwnedLexToken]) -> Option<String> {
    double_quoted_rule_bodies(tokens)
        .into_iter()
        .find_map(|body| {
            token_definition_grammar::parse_token_reminder_facts_tokens(body)
                .sacrifice_at_next_end_step
                .then(|| render_token_slice(body))
        })
}

fn append_inline_token_embedded_rule(
    definition: &mut crate::runtime_backend::token_definition::TokenDefinitionSpec,
    rule_tokens: &[OwnedLexToken],
) -> bool {
    use crate::runtime_backend::token_definition::TokenDefinitionSpec;

    let (name, rules) = match definition {
        TokenDefinitionSpec::Creature(creature) => {
            (&creature.name, &mut creature.rules.token_rules)
        }
        TokenDefinitionSpec::Artifact(artifact) => (&artifact.name, &mut artifact.token_rules),
        _ => return false,
    };
    let Some(rule) = crate::runtime_backend::front_end::grammar::token_definitions::
        parse_embedded_token_rule_tokens(rule_tokens, Some(name))
    else {
        return false;
    };
    if rules.embedded_rules.contains(&rule) {
        false
    } else {
        rules.embedded_rules.push(rule);
        true
    }
}

fn parse_inline_token_granted_abilities(
    definition: &mut crate::runtime_backend::token_definition::TokenDefinitionSpec,
    tokens: &[OwnedLexToken],
) -> Vec<GrantedAbilityAst> {
    let mut abilities = Vec::new();
    let complete_reminder = token_definition_grammar::parse_token_reminder_facts_tokens(tokens);
    token_definition_grammar::merge_token_reminder_definition(definition, &complete_reminder);
    for rule_tokens in double_quoted_rule_bodies(tokens) {
        let reminder = token_definition_grammar::parse_token_reminder_facts_tokens(rule_tokens);
        if token_definition_grammar::merge_token_reminder_definition(definition, &reminder) {
            // Specialized token rules belong to the token blueprint. This is
            // particularly important after outer dispatch strips the quoted
            // suffix before parsing the create action: restore the typed rule
            // before deciding whether a generic granted ability is needed.
            // Reapply the complete clause so an Equipment rule body cannot
            // discard a trailing `and equip` clause outside its quotes.
            token_definition_grammar::merge_token_reminder_definition(
                definition,
                &complete_reminder,
            );
            continue;
        }
        if append_inline_token_embedded_rule(definition, rule_tokens) {
            continue;
        }
        let Ok(parsed) =
            super::parse_granted_abilities_for_token_definition(definition, rule_tokens)
        else {
            // Token definitions have a number of older specialized shapes.
            // An unsupported generic nested rule must leave those paths
            // available rather than turning an otherwise parseable card into
            // a hard error.
            continue;
        };
        for ability in parsed {
            if !abilities.contains(&ability) {
                abilities.push(ability);
            }
        }
    }
    abilities
}

fn intrinsic_token_ability_represents_dynamic_power_toughness(
    definition: &crate::runtime_backend::token_definition::TokenDefinitionSpec,
    granted_abilities: &[GrantedAbilityAst],
    dynamic: &(Value, Value),
) -> bool {
    let same_values = |power: &Value, toughness: &Value| {
        power.unhinted() == dynamic.0.unhinted() && toughness.unhinted() == dynamic.1.unhinted()
    };
    if granted_abilities.iter().any(|ability| {
        let static_ability = match ability {
            GrantedAbilityAst::StaticAbility(static_ability) => static_ability,
            GrantedAbilityAst::ParsedObjectAbility { ability, .. } => {
                let crate::ability::AbilityKind::Static(static_ability) = ability.kind() else {
                    return false;
                };
                static_ability
            }
            _ => return false,
        };
        let crate::static_abilities::StaticAbilityPayload::CharacteristicDefiningPt {
            power,
            toughness,
        } = &static_ability.payload
        else {
            return false;
        };
        same_values(power, toughness)
    }) {
        return true;
    }

    let crate::runtime_backend::token_definition::TokenDefinitionSpec::Creature(creature) =
        definition
    else {
        return false;
    };
    let creature_count = Value::Count(ObjectFilter::creature().you_control());
    same_values(&creature_count, &creature_count)
        && creature.rules.token_rules.embedded_rules.contains(
            &crate::runtime_backend::token_definition::TokenEmbeddedRuleShape::
                PowerToughnessEqualCreaturesYouControl,
        )
}

fn reconcile_quoted_dynamic_power_toughness(
    definition: &crate::runtime_backend::token_definition::TokenDefinitionSpec,
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
    let ability = GrantedAbilityAst::StaticAbility(StaticAbility::characteristic_defining_pt(
        quoted_dynamic.0,
        quoted_dynamic.1,
    ));
    if !granted_abilities.contains(&ability) {
        granted_abilities.push(ability);
    }
    *dynamic_power_toughness = None;
}

fn parse_inline_copy_granted_abilities(tokens: &[OwnedLexToken]) -> Vec<StaticAbility> {
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
        let parsed = crate::runtime_backend::util::with_token_source_reference_context(
            "Token",
            &[CardType::Artifact],
            &[Subtype::Equipment],
            || super::parse_granted_abilities_for_gain_clause(rule_tokens, &clause_words, false),
        );
        let Ok((granted, false)) = parsed else {
            continue;
        };
        let Ok(lowered) =
            super::super::static_ability_helpers::lower_granted_abilities_ast_to_object_abilities(
                &granted,
            )
            .and_then(|abilities| {
                super::super::static_ability_helpers::object_abilities_to_static_carriers(
                    abilities,
                    crate::runtime_backend::lexer::render_token_slice(rule_tokens),
                )
            })
        else {
            continue;
        };
        for ability in lowered {
            if !abilities.contains(&ability) {
                abilities.push(ability);
            }
        }
    }
    abilities
}

fn attach_inline_token_granted_abilities_to_effect(
    effect: &mut EffectAst,
    tokens: &[OwnedLexToken],
) -> bool {
    if let EffectAst::SubjectVerb(subject_verb) = effect {
        match &mut subject_verb.action {
            SubjectVerbActionAst::CreateTokenCopy {
                sacrifice_at_next_end_step,
                sacrifice_at_next_end_step_ability_text,
                granted_abilities,
                ..
            }
            | SubjectVerbActionAst::CreateTokenCopyFromSource {
                sacrifice_at_next_end_step,
                sacrifice_at_next_end_step_ability_text,
                granted_abilities,
                ..
            } => {
                let mut attached = false;
                if *sacrifice_at_next_end_step && sacrifice_at_next_end_step_ability_text.is_none()
                {
                    *sacrifice_at_next_end_step_ability_text =
                        quoted_copy_sacrifice_ability_text(tokens);
                    attached |= sacrifice_at_next_end_step_ability_text.is_some();
                }
                for ability in parse_inline_copy_granted_abilities(tokens) {
                    if !granted_abilities.contains(&ability) {
                        granted_abilities.push(ability);
                        attached = true;
                    }
                }
                if attached {
                    return true;
                }
            }
            _ => {}
        }
    }

    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::CreateTokenWithMods {
            definition,
            dynamic_power_toughness,
            granted_abilities,
            ability_presentation,
            ..
        } = &mut subject_verb.action
    {
        for ability in parse_inline_token_granted_abilities(definition, tokens) {
            if !granted_abilities.contains(&ability) {
                granted_abilities.push(ability);
            }
        }
        if let Some(dynamic) = parse_quoted_token_dynamic_power_toughness(tokens) {
            reconcile_quoted_dynamic_power_toughness(
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
    crate::runtime_backend::model::effect_ast_traversal::for_each_nested_effects_mut(
        effect,
        true,
        |nested| {
            if !found {
                found = attach_inline_token_granted_abilities_to_last_create(nested, tokens);
            }
        },
    );
    found
}

/// Production sentence dispatch strips embedded token rules before parsing the
/// outer create action so quoted colons and verbs cannot win outer dispatch.
/// Reattach those original quoted bodies to the create AST after that parse.
pub(crate) fn attach_inline_token_granted_abilities_to_last_create(
    effects: &mut [EffectAst],
    tokens: &[OwnedLexToken],
) -> bool {
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

pub(crate) fn parse_create(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    // Capture the authored actor before imperative/chain normalization can
    // turn an implicit create action into the same semantic `PlayerAst::You`.
    let actor_surface_explicit = matches!(subject, Some(SubjectAst::Player(PlayerAst::You)));
    let tokens = creation_grammar::creation_body_tokens(tokens);
    if let Some(alternative) = parse_direct_token_creation_alternative(tokens, subject) {
        return Ok(alternative);
    }
    let non_article_words = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    if let Some(action) =
        creation_grammar::parse_delayed_combat_token_action_words(&non_article_words)
    {
        let effect = match action {
            creation_grammar::DelayedCombatTokenAction::Exile => EffectAst::subject_verb_exile(
                TargetAst::Object(
                    ObjectFilter::tagged(TagKey::from(IT_TAG)),
                    span_from_tokens(tokens),
                    None,
                ),
                false,
            ),
            creation_grammar::DelayedCombatTokenAction::Sacrifice => {
                EffectAst::subject_verb_sacrifice(
                    PlayerAst::Implicit,
                    ObjectFilter::tagged(TagKey::from(IT_TAG)),
                    1,
                    None,
                )
            }
        };
        return Ok(EffectAst::DelayedUntilEndOfCombat {
            effects: vec![effect],
        });
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
    let needs_equal_to_dynamic_count = matches!(
        head.count,
        creation_grammar::CreateCountHead::EqualToDynamic
    );
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
            EffectAst::ForEachObject {
                filter,
                effects: vec![effect],
            }
        } else {
            effect
        }
    };
    let wrap_for_each_player_condition = |effect: EffectAst| {
        if let Some((filter, predicate)) = &for_each_player_condition {
            let effects = vec![EffectAst::Conditional {
                predicate: predicate.clone(),
                if_true: vec![effect],
                if_false: Vec::new(),
            }];
            match filter {
                PlayerFilter::Opponent => EffectAst::ForEachOpponent { effects },
                PlayerFilter::Any => EffectAst::ForEachPlayer { effects },
                other => EffectAst::ForEachPlayersFiltered {
                    filter: other.clone(),
                    effects,
                },
            }
        } else {
            effect
        }
    };
    let wrap_delayed_create = |effect: EffectAst| {
        if let Some(player) = delayed_create_player {
            EffectAst::DelayedUntilNextEndStep {
                player,
                effects: vec![effect],
            }
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
    let mut raw_name_override: Option<String> = None;
    let mut rules_text_range: Option<(usize, usize)> = None;
    if let Some(named) = creation_grammar::parse_named_token_clause_tokens(&tail_tokens) {
        let named_words = token_word_refs(&tail_tokens[named.name.clone()]);
        if !named_words.is_empty() {
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
                granted_abilities,
                loses_soulbond,
            ) = parse_copy_modifiers_from_tail(&tail_words)?;
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
            let sacrifice_at_next_end_step_ability_text = sacrifice_at_next_end_step
                .then(|| quoted_copy_sacrifice_ability_text(&tail_tokens))
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
                        SubjectVerbActionAst::CreateTokenCopyFromSource {
                            source,
                            count: resolve_create_count(references_iterated_object),
                            player,
                            enters_tapped,
                            enters_attacking,
                            attack_target_player_or_planeswalker_controlled_by,
                            attack_target_player_only: false,
                            half_power_toughness_round_up: half_pt,
                            has_haste,
                            haste_followup_reference_surface: None,
                            exile_at_end_of_combat: false,
                            exile_at_end_of_combat_reference_surface: None,
                            loses_soulbond,
                            sacrifice_at_next_end_step,
                            sacrifice_at_next_end_step_reference_surface: None,
                            sacrifice_at_next_end_step_ability_text,
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
                            granted_abilities,
                        },
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
                SubjectVerbActionAst::CreateTokenCopy {
                    object: ObjectRefAst::Tagged(TagKey::from(IT_TAG)),
                    count: resolve_create_count(references_iterated_object),
                    player,
                    enters_tapped,
                    enters_attacking,
                    attack_target_player_or_planeswalker_controlled_by,
                    attack_target_player_only: false,
                    half_power_toughness_round_up: half_pt,
                    has_haste,
                    haste_followup_reference_surface: None,
                    exile_at_end_of_combat: false,
                    exile_at_end_of_combat_reference_surface: None,
                    loses_soulbond,
                    sacrifice_at_next_end_step,
                    sacrifice_at_next_end_step_reference_surface: None,
                    sacrifice_at_next_end_step_ability_text,
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
                    granted_abilities,
                },
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
                let raw_tail = render_token_slice(&tail_tokens[raw_tail_start..raw_tail_end])
                    .trim()
                    .to_string();
                let prefix = normalize_token_name(&name_words);
                raw_name_override = Some(if prefix.is_empty() {
                    raw_tail
                } else {
                    format!("{prefix} {raw_tail}")
                });
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
    let primary_definition_is_construct = name_words[..name_words_primary_len]
        .iter()
        .any(|word| *word == "construct");
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
    let name = raw_name_override.unwrap_or_else(|| normalize_token_name(&name_words));
    let mut definition =
        token_definition_grammar::parse_token_definition_shape_tokens(&definition_tokens)
            .or_else(|| {
                parse_prior_created_token_reference_words(&name_words).map(|_| {
                    crate::runtime_backend::token_definition::TokenDefinitionSpec::PriorCreated
                })
            })
            .ok_or_else(|| {
                CardTextError::ParseError(format!("unsupported token definition '{name}'"))
            })?;
    if let Some(postnominal_colors) =
        token_definition_grammar::parse_postnominal_token_colors_tokens(&tail_tokens)
    {
        match &mut definition {
            crate::runtime_backend::token_definition::TokenDefinitionSpec::Creature(creature) => {
                creature.colors = creature.colors.union(postnominal_colors);
            }
            crate::runtime_backend::token_definition::TokenDefinitionSpec::Artifact(artifact) => {
                artifact.colors = artifact.colors.union(postnominal_colors);
            }
            _ => {}
        }
    }
    if let crate::runtime_backend::token_definition::TokenDefinitionSpec::Creature(shape) =
        &mut definition
    {
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
        reconcile_quoted_dynamic_power_toughness(
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
        && let crate::runtime_backend::token_definition::TokenDefinitionSpec::Vehicle(vehicle) =
            &mut definition
        && vehicle.power_toughness.is_none()
    {
        vehicle.power_toughness = Some((0, 0));
    }

    let grants_unblockable = tail_surface.has_phrase(CreatePhrase::Unblockable);

    if let Some((start, end)) = rules_text_range {
        if start < end && end <= modifier_tail_words.len() {
            modifier_tail_words = modifier_tail_words[..start]
                .iter()
                .chain(modifier_tail_words[end..].iter())
                .copied()
                .collect();
        }
    }

    if let Some(where_tokens) = creation_grammar::parse_where_clause_tokens(&tail_tokens) {
        let where_value = parse_create_value_binding(where_tokens).ok_or_else(|| {
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
    if attacking
        && matches!(player, PlayerAst::That)
        && modifier_surface.has_phrase(CreatePhrase::AttackingThatPlayer)
    {
        player = PlayerAst::You;
    }
    let (sacrifice_at_next_end_step, exile_at_next_end_step, next_end_step_player) =
        parse_next_end_step_token_delay_flags(&modifier_tail_words);
    let mut granted_abilities = inline_granted_abilities;
    if modifier_surface.has(CreateWord::Decayed) {
        granted_abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Decayed));
    }
    if modifier_surface.has_phrase(CreatePhrase::HasteGrant) {
        granted_abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Haste));
    }
    if grants_unblockable {
        granted_abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Unblockable));
    }
    let references_iterated_object = attached_to_target
        .as_ref()
        .is_some_and(target_references_it);
    let create = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        player,
        SubjectVerbActionAst::CreateTokenWithMods {
            name,
            definition,
            count: resolve_create_count(references_iterated_object),
            dynamic_power_toughness,
            player,
            actor_surface_explicit,
            attached_to: attached_to_target,
            tapped,
            attacking,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            next_end_step_player,
            granted_abilities,
            ability_presentation: inline_ability_presentation,
        },
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
fn parse_direct_token_creation_alternative(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let mut inside_quotes = false;
    let mut separator = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if !inside_quotes && token.kind == TokenKind::Word && token.parser_text() == "or" {
            if separator.replace(idx).is_some() {
                return None;
            }
        }
    }

    let separator = separator?;
    let left_tokens = trim_commas(&tokens[..separator]);
    let right_tokens = trim_commas(&tokens[separator + 1..]);
    if left_tokens.is_empty()
        || right_tokens.is_empty()
        || ![left_tokens.as_slice(), right_tokens.as_slice()]
            .iter()
            .all(|branch| {
                branch.iter().any(|token| {
                    token.kind == TokenKind::Word
                        && matches!(token.parser_text(), "token" | "tokens")
                })
            })
    {
        return None;
    }

    let parse_branch = |branch: &[OwnedLexToken]| {
        let parsed = parse_create(branch, subject).ok()?;
        matches!(
            &parsed,
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::CreateTokenWithMods { .. }
                )
        )
        .then_some(parsed)
    };
    let first = parse_branch(&left_tokens)?;
    let second = parse_branch(&right_tokens)?;

    Some(EffectAst::ChooseOneOf {
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
    })
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

pub(crate) fn normalize_token_name(words: &[&str]) -> String {
    words.join(" ")
}

fn parse_investigate_for_each_count(tokens: &[OwnedLexToken]) -> Result<Value, CardTextError> {
    creation_grammar::parse_investigate_for_each_count_tokens(tokens)
}

pub(crate) fn parse_investigate(
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
    {
        if let Some(where_count) = parse_create_value_binding(where_tokens) {
            count = where_count;
            return Ok(EffectAst::subject_verb_investigate(player, count));
        }
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

pub(crate) fn parse_incubate(
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
        let Some(where_value) = parse_create_value_binding(where_tokens) else {
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
    use crate::static_abilities::StaticAbilityId;
    use crate::target::{ChooseSpec, SourceReferenceSurface};
    use ironsmith_core::TurnHistoryCount;

    fn parse_token_count(clause: &str) -> Value {
        let tokens = lex_line(clause, 0).expect("token creation should lex");
        let effect = parse_create(&tokens, None).expect("token creation should parse");
        let EffectAst::SubjectVerb(effect) = effect else {
            panic!("expected a subject-verb token creation");
        };
        let SubjectVerbActionAst::CreateTokenWithMods { count, .. } = effect.action else {
            panic!("expected a token creation with modifiers");
        };
        count
    }

    #[test]
    fn direct_token_creation_or_lowers_to_two_complete_choice_modes() {
        let tokens = lex_line("Create a Food token or a Treasure token.", 0)
            .expect("token alternative should lex");
        let parsed = parse_create(&tokens, None).expect("token creation alternative should parse");
        let EffectAst::ChooseOneOf { modes } = parsed else {
            panic!("expected a typed token-creation choice, got {parsed:#?}");
        };
        assert_eq!(modes.len(), 2);

        let names = modes
            .iter()
            .map(|mode| {
                let [EffectAst::SubjectVerb(effect)] = mode.effects.as_slice() else {
                    panic!("expected one direct create effect per mode: {mode:#?}");
                };
                let SubjectVerbActionAst::CreateTokenWithMods { name, .. } = &effect.action else {
                    panic!("expected a named token creation: {effect:#?}");
                };
                name.as_str()
            })
            .collect::<Vec<_>>();
        assert_eq!(names, ["Food", "Treasure"]);
        assert!(modes.iter().all(|mode| mode.description.is_empty()));
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
        let SubjectVerbActionAst::CreateTokenWithMods { definition, .. } = effect.action else {
            panic!("expected a token creation with modifiers");
        };
        let crate::runtime_backend::token_definition::TokenDefinitionSpec::Creature(creature) =
            definition
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
        let SubjectVerbActionAst::CreateTokenWithMods { definition, .. } = effect.action else {
            panic!("expected a token creation with modifiers");
        };
        let crate::runtime_backend::token_definition::TokenDefinitionSpec::Creature(creature) =
            definition
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
                .filter_map(|result| result.as_ref().ok())
                .map(Vec::len)
                .sum::<usize>(),
            2,
            "{parsed:#?}"
        );

        let ast = parse_create(&tokens, None).expect("create AST");
        let EffectAst::SubjectVerb(effect) = &ast else {
            panic!("expected subject-verb create AST");
        };
        let SubjectVerbActionAst::CreateTokenWithMods {
            granted_abilities, ..
        } = &effect.action
        else {
            panic!("expected create-token AST");
        };
        assert!(
            matches!(
                granted_abilities.as_slice(),
                [
                    GrantedAbilityAst::ParsedObjectAbility { .. },
                    GrantedAbilityAst::ParsedObjectAbility { .. },
                ]
            ),
            "{granted_abilities:#?}"
        );

        let (effects, _) = crate::runtime_backend::compile_support::compile_effect(
            &ast,
            &mut crate::runtime_backend::EffectLoweringContext::new(),
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
        let (effects, _) = crate::runtime_backend::compile_support::compile_effects(
            &sentence_effects,
            &mut crate::runtime_backend::EffectLoweringContext::new(),
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
        let (effects, _) = crate::runtime_backend::compile_support::compile_effects(
            &sentence_effects,
            &mut crate::runtime_backend::EffectLoweringContext::new(),
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
        let SubjectVerbActionAst::CreateTokenWithMods {
            definition,
            dynamic_power_toughness,
            ..
        } = effect.action
        else {
            panic!("expected a token creation with modifiers");
        };
        let crate::runtime_backend::token_definition::TokenDefinitionSpec::Construct(construct) =
            definition
        else {
            panic!("expected a Construct token definition");
        };

        assert_eq!(construct.power_toughness, (0, 0));
        assert_eq!(construct.artifact_scaling, None);
        assert!(dynamic_power_toughness.is_some());
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
        let SubjectVerbActionAst::CreateTokenWithMods {
            dynamic_power_toughness,
            granted_abilities,
            ..
        } = effect.action
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
        let SubjectVerbActionAst::CreateTokenWithMods {
            dynamic_power_toughness,
            granted_abilities,
            ..
        } = effect.action
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
        let ast = crate::runtime_backend::sentences::effect_sentences::parse_effect_sentence_lexed(
            &tokens,
        )
        .expect("single-sentence dispatcher should parse the token creation");
        let (effects, _) = crate::runtime_backend::compile_support::compile_effects(
            &ast,
            &mut crate::runtime_backend::EffectLoweringContext::new(),
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
        let effect =
            crate::runtime_backend::util::with_source_reference_context("Gutter Grime", || {
                parse_create(&tokens, None)
            })
            .expect("quoted external dynamic token creation should parse");
        let lowered_ast = effect.clone();
        let EffectAst::SubjectVerb(effect) = effect else {
            panic!("expected a subject-verb token creation");
        };
        let SubjectVerbActionAst::CreateTokenWithMods {
            dynamic_power_toughness,
            granted_abilities,
            ..
        } = effect.action
        else {
            panic!("expected a token creation with modifiers");
        };
        assert_eq!(dynamic_power_toughness, None);
        let [GrantedAbilityAst::StaticAbility(static_ability)] = granted_abilities.as_slice()
        else {
            panic!("expected one intrinsic token CDA: {granted_abilities:#?}");
        };
        let crate::static_abilities::StaticAbilityPayload::CharacteristicDefiningPt {
            power,
            toughness,
        } = &static_ability.payload
        else {
            panic!("expected a characteristic-defining P/T ability: {static_ability:#?}");
        };
        assert_eq!(power, toughness);
        let Value::CountersOn(spec, Some(crate::CounterType::Named("slime"))) = power.unhinted()
        else {
            panic!("expected a named-source counter value: {power:#?}");
        };
        assert!(matches!(spec.base(), ChooseSpec::Source));
        assert_eq!(
            spec.source_reference_surface(),
            Some(&SourceReferenceSurface::FullName(
                "Gutter Grime".to_string()
            ))
        );

        let (effects, _) = crate::runtime_backend::compile_support::compile_effect(
            &lowered_ast,
            &mut crate::runtime_backend::EffectLoweringContext::new(),
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

        let sentence_effects =
            crate::runtime_backend::util::with_source_reference_context("Gutter Grime", || {
                super::super::parse_effect_sentences_lexed(&tokens)
            })
            .expect("creator-bound token sentence should parse through production dispatch");
        let (effects, _) = crate::runtime_backend::compile_support::compile_effects(
            &sentence_effects,
            &mut crate::runtime_backend::EffectLoweringContext::new(),
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
        let SubjectVerbActionAst::CreateTokenWithMods {
            definition,
            dynamic_power_toughness,
            granted_abilities,
            ..
        } = &subject_verb.action
        else {
            panic!("expected a token creation with modifiers");
        };
        assert_eq!(dynamic_power_toughness, &None);
        assert_eq!(granted_abilities.len(), 1, "{granted_abilities:#?}");
        let crate::runtime_backend::token_definition::TokenDefinitionSpec::Creature(creature) =
            definition
        else {
            panic!("expected a creature token definition");
        };
        assert!(
            creature.rules.token_rules.embedded_rules.is_empty(),
            "{:#?}",
            creature.rules.token_rules
        );

        let (effects, _) = crate::runtime_backend::compile_support::compile_effect(
            &ast,
            &mut crate::runtime_backend::EffectLoweringContext::new(),
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
        let (effects, _) = crate::runtime_backend::compile_support::compile_effects(
            &sentence_effects,
            &mut crate::runtime_backend::EffectLoweringContext::new(),
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
                ability.id() == StaticAbilityId::ActivatedAbilityCostReduction
                    && format!("{ability:?}")
                        .to_ascii_lowercase()
                        .contains("equip abilities cost")
            }),
            "expected typed equip cost reduction, got {abilities:#?}"
        );
    }
}
