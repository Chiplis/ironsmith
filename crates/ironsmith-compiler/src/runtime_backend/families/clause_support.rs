use crate::cards::builders::{
    CardTextError, EffectAst, KeywordAction, LineAst, PredicateAst, StaticAbilityAst,
    SubjectVerbActionAst, TargetAst, TriggerSpec,
};
use crate::effect::{EventValueSpec, Value};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

use super::activation_and_restrictions::keyword_action_costs::maybe_strip_leading_damage_subject_tokens;
use super::activation_and_restrictions::{
    parse_ability_phrase, parse_named_number, parse_single_word_keyword_action,
    parse_triggered_times_each_turn_lexed,
};
use super::grammar::clause_support::{
    self as clause_grammar, ProtectionTargetKind, SourceTriggerKind, TriggerDelimiterKind,
};
use super::grammar::primitives::TokenWordView;
use super::grammar::structure::{
    find_trigger_effect_list_tail_split_lexed,
    rewrite_attached_controller_trigger_effect_tokens_lexed,
    split_first_time_each_turn_trigger_suffix_lexed, split_state_triggered_clause_lexed,
    split_triggered_conditional_clause_lexed,
};
use super::lexer::{OwnedLexToken, split_lexed_sentences};
use super::object_filters::parse_object_filter_lexed;
use super::util::{
    parse_card_type, parse_color, parse_filter_counter_constraint_words,
    parse_flashback_keyword_line, parse_subtype_flexible, strip_leading_word_refs_any, trim_commas,
};
use crate::runtime_backend::grammar::shared_util::value_semantics::parse_filter_comparison_tokens;

const TWO_WORD_KEYWORD_ACTIONS: &[(&[&str], KeywordAction)] = &[
    (&["first", "strike"], KeywordAction::FirstStrike),
    (&["double", "strike"], KeywordAction::DoubleStrike),
    (&["battle", "cry"], KeywordAction::BattleCry),
    (&["split", "second"], KeywordAction::SplitSecond),
    (&["read", "ahead"], KeywordAction::ReadAhead),
    (&["for", "mirrodin"], KeywordAction::ForMirrodin),
    (&["living", "weapon"], KeywordAction::LivingWeapon),
    (&["umbra", "armor"], KeywordAction::UmbraArmor),
    (
        &["doctor", "companion"],
        KeywordAction::Marker("doctor companion"),
    ),
];

fn predicate_counts_creatures_died_this_turn(predicate: &PredicateAst) -> bool {
    matches!(
        predicate,
        PredicateAst::CreatureDiedThisTurn | PredicateAst::CreatureDiedThisTurnOrMore(_)
    )
}

fn bind_event_amount_to_creatures_died_this_turn(value: &mut Value) {
    match value {
        Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount) => {
            *value = Value::CreaturesDiedThisTurn;
        }
        Value::EventValueOffset(EventValueSpec::Amount, offset)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, offset) => {
            *value = Value::Add(
                Box::new(Value::CreaturesDiedThisTurn),
                Box::new(Value::Fixed(*offset)),
            );
        }
        Value::Add(left, right) | Value::Min(left, right) => {
            bind_event_amount_to_creatures_died_this_turn(left);
            bind_event_amount_to_creatures_died_this_turn(right);
        }
        Value::Scaled(inner, _)
        | Value::DividedRoundedDown(inner, _)
        | Value::HalfRoundedDown(inner)
        | Value::SurfaceHinted { value: inner, .. } => {
            bind_event_amount_to_creatures_died_this_turn(inner);
        }
        _ => {}
    }
}

fn bind_creatures_died_condition_amounts(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
                SubjectVerbActionAst::GainLife { amount }
                | SubjectVerbActionAst::PutCounters { count: amount, .. } => {
                    bind_event_amount_to_creatures_died_this_turn(amount);
                }
                _ => {}
            },
            EffectAst::Conditional {
                if_true, if_false, ..
            } => {
                bind_creatures_died_condition_amounts(if_true);
                bind_creatures_died_condition_amounts(if_false);
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum AttackedPlayerFilterKind {
    Any,
    AnyPlayerOrPlaneswalker,
    Enchanted,
    Opponent,
    You,
}

const ATTACKED_PLAYER_FILTERS: &[(&[&str], AttackedPlayerFilterKind)] = &[
    (
        &["enchanted", "player"],
        AttackedPlayerFilterKind::Enchanted,
    ),
    (&["you"], AttackedPlayerFilterKind::You),
    (&["an", "opponent"], AttackedPlayerFilterKind::Opponent),
    (&["opponent"], AttackedPlayerFilterKind::Opponent),
    (&["a", "player"], AttackedPlayerFilterKind::Any),
    (&["player"], AttackedPlayerFilterKind::Any),
    (&["any", "player"], AttackedPlayerFilterKind::Any),
    (
        &["a", "player", "or", "planeswalker"],
        AttackedPlayerFilterKind::AnyPlayerOrPlaneswalker,
    ),
    (
        &["a", "player", "or", "a", "planeswalker"],
        AttackedPlayerFilterKind::AnyPlayerOrPlaneswalker,
    ),
    (
        &["player", "or", "planeswalker"],
        AttackedPlayerFilterKind::AnyPlayerOrPlaneswalker,
    ),
];

fn two_word_keyword_action(words: &[&str]) -> Option<KeywordAction> {
    TWO_WORD_KEYWORD_ACTIONS
        .iter()
        .find_map(|(phrase, action)| (*phrase == words).then(|| action.clone()))
}

fn attacked_player_filter_from_words(words: &[&str]) -> Option<(PlayerFilter, bool)> {
    ATTACKED_PLAYER_FILTERS
        .iter()
        .find_map(|(phrase, filter)| (*phrase == words).then_some(*filter))
        .map(|filter| match filter {
            AttackedPlayerFilterKind::Any => (PlayerFilter::Any, true),
            AttackedPlayerFilterKind::AnyPlayerOrPlaneswalker => (PlayerFilter::Any, false),
            AttackedPlayerFilterKind::Enchanted => (
                PlayerFilter::TaggedPlayer(crate::TagKey::from("enchanted")),
                true,
            ),
            AttackedPlayerFilterKind::Opponent => (PlayerFilter::Opponent, true),
            AttackedPlayerFilterKind::You => (PlayerFilter::You, true),
        })
}
fn is_and_word(word: &str) -> bool {
    word == "and"
}

fn is_keyword_with_count_word(word: &str) -> bool {
    matches!(
        word,
        "ward"
            | "toxic"
            | "afflict"
            | "afterlife"
            | "fabricate"
            | "renown"
            | "backup"
            | "bushido"
            | "bloodthirst"
    )
}

fn protection_from_colored_spells_action(tokens: &[OwnedLexToken]) -> Option<KeywordAction> {
    if !clause_grammar::parse_protection_from_colored_spells_tokens(tokens) {
        return None;
    }

    let mut filter = ObjectFilter::spell();
    filter.colors = Some(all_magic_colors());
    Some(KeywordAction::ProtectionFromFilter(filter))
}

fn all_magic_colors() -> crate::color::ColorSet {
    crate::color::ColorSet::WHITE
        .union(crate::color::ColorSet::BLUE)
        .union(crate::color::ColorSet::BLACK)
        .union(crate::color::ColorSet::RED)
        .union(crate::color::ColorSet::GREEN)
}

fn protection_from_each_mana_value_among_action(tokens: &[OwnedLexToken]) -> Option<KeywordAction> {
    let chain = clause_grammar::parse_protection_chain_tokens(tokens)?;
    let target = chain.targets.first()?;
    let ProtectionTargetKind::EachManaValueAmong { filter_word_first } = target.kind else {
        return None;
    };
    let view = TokenWordView::new(tokens);
    let filter_token_first = *view.token_start_indices().get(filter_word_first)?;
    let filter_tokens = trim_commas(&tokens[filter_token_first..]);
    (!filter_tokens.is_empty())
        .then(|| parse_object_filter_lexed(&filter_tokens, false).ok())
        .flatten()
        .map(KeywordAction::ProtectionFromEachManaValueAmong)
}

pub(crate) fn parse_protection_chain(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    let chain = clause_grammar::parse_protection_chain_tokens(tokens)?;
    let words = TokenWordView::new(tokens).word_refs();
    let mut actions = Vec::new();
    for target in &chain.targets {
        let action = match target.kind {
            ProtectionTargetKind::EachManaValueAmong { filter_word_first } => {
                let filter_token_first = *TokenWordView::new(tokens)
                    .token_start_indices()
                    .get(filter_word_first)?;
                let filter_tokens = trim_commas(&tokens[filter_token_first..]);
                parse_object_filter_lexed(&filter_tokens, false)
                    .ok()
                    .map(KeywordAction::ProtectionFromEachManaValueAmong)
            }
            ProtectionTargetKind::Spell => {
                Some(KeywordAction::ProtectionFromFilter(ObjectFilter::spell()))
            }
            ProtectionTargetKind::PermanentCastThisTurn => {
                let mut filter = ObjectFilter::permanent();
                filter.cast_this_turn = true;
                Some(KeywordAction::ProtectionFromFilter(filter))
            }
            ProtectionTargetKind::ManaValue {
                comparison_word_first,
            } => {
                let comparison_tail = words.get(comparison_word_first..)?;
                let (comparison, consumed) =
                    parse_filter_comparison_tokens("mana value", comparison_tail, &words)
                        .ok()??;
                (consumed == comparison_tail.len()).then(|| {
                    let mut filter = ObjectFilter::default();
                    filter.mana_value = Some(comparison);
                    KeywordAction::ProtectionFromFilter(filter)
                })
            }
            ProtectionTargetKind::PermanentWithCounter { counter_word_first } => {
                let counter_words = words.get(counter_word_first..)?;
                parse_filter_counter_constraint_words(counter_words).and_then(
                    |(with_counter, consumed)| {
                        (consumed == counter_words.len()).then(|| {
                            let mut filter = ObjectFilter::permanent();
                            filter.with_counter = Some(with_counter);
                            KeywordAction::ProtectionFromFilter(filter)
                        })
                    },
                )
            }
            ProtectionTargetKind::ChosenPlayer => Some(KeywordAction::ProtectionFromChosenPlayer),
            ProtectionTargetKind::ChosenColor => Some(KeywordAction::ProtectionFromChosenColor),
            ProtectionTargetKind::Colorless => Some(KeywordAction::ProtectionFromColorless),
            ProtectionTargetKind::Everything => Some(KeywordAction::ProtectionFromEverything),
            ProtectionTargetKind::AllColors => Some(KeywordAction::ProtectionFromAllColors),
            ProtectionTargetKind::Named => parse_color(target.value)
                .map(KeywordAction::ProtectionFrom)
                .or_else(|| {
                    parse_card_type(target.value).map(KeywordAction::ProtectionFromCardType)
                })
                .or_else(|| {
                    parse_subtype_flexible(target.value).map(KeywordAction::ProtectionFromSubtype)
                }),
        }?;
        crate::slice_primitives::push_unique(&mut actions, action);
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

fn color_only_hexproof_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    clause_grammar::parse_color_only_hexproof_filter_words(words)
}

fn parse_hexproof_from_chain(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    let words_view = TokenWordView::new(tokens);
    let words = words_view.word_refs();
    let first_word_idx = if words.first().is_some_and(|word| is_and_word(word)) {
        1
    } else {
        0
    };
    if words.len().saturating_sub(first_word_idx) < 3
        || words.get(first_word_idx).copied() != Some("hexproof")
        || words.get(first_word_idx + 1).copied() != Some("from")
    {
        return None;
    }

    color_only_hexproof_filter_words(&words[first_word_idx + 2..])
        .map(|filter| vec![KeywordAction::HexproofFrom(filter)])
}

pub(crate) fn parse_ability_line_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    fn parse_simple_keyword_phrase_lexed(tokens: &[OwnedLexToken]) -> Option<KeywordAction> {
        let words_view = TokenWordView::new(tokens);
        let words = words_view.word_refs();
        let words = strip_leading_word_refs_any(&words, &["and"]);
        if words.is_empty() {
            return None;
        }

        if clause_grammar::parse_casualty_planeswalker_copy_prefix_words(&words) {
            return Some(KeywordAction::VariableCasualtyPlaneswalkerCopy);
        }

        if words.len() == 1 {
            return parse_single_word_keyword_action(words[0]);
        }

        let parse_count_keyword =
            |expected: &str, ctor: fn(u32) -> KeywordAction| -> Option<KeywordAction> {
                if !words
                    .first()
                    .is_some_and(|word| is_keyword_with_count_word(word))
                    || !is_keyword_with_count_word(expected)
                    || !words.first().is_some_and(|word| *word == expected)
                {
                    return None;
                }
                words
                    .get(1)
                    .and_then(|word| parse_named_number(word))
                    .map(ctor)
            };

        if let Some(action) = parse_count_keyword("ward", KeywordAction::Ward) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("toxic", KeywordAction::Toxic) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("afflict", KeywordAction::Afflict) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("afterlife", KeywordAction::Afterlife) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("fabricate", KeywordAction::Fabricate) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("renown", KeywordAction::Renown) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("backup", KeywordAction::Backup) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("bushido", KeywordAction::Bushido) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("bloodthirst", KeywordAction::Bloodthirst) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("tribute", KeywordAction::Tribute) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("rampage", KeywordAction::Rampage) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("annihilator", KeywordAction::Annihilator) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("soulshift", KeywordAction::Soulshift) {
            return Some(action);
        }
        if let Some(action) = super::activation_and_restrictions::keyword_action_costs::parse_dynamic_soulshift_keyword_action(&words)
        {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("modular", KeywordAction::Modular) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("graft", KeywordAction::Graft) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("firebending", KeywordAction::Firebending) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("fading", KeywordAction::Fading) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("vanishing", KeywordAction::Vanishing) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("mobilize", KeywordAction::Mobilize) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("casualty", KeywordAction::Casualty) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("amplify", KeywordAction::Amplify) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("devour", KeywordAction::Devour) {
            return Some(action);
        }
        if words.first().is_some_and(|word| *word == "dredge")
            && let Some(amount) = words.get(1)
            && parse_named_number(amount).is_some()
        {
            return Some(KeywordAction::StaticMarkerText(format!("Dredge {amount}")));
        }

        if clause_grammar::parse_read_ahead_prefix_words(&words) {
            return Some(KeywordAction::ReadAhead);
        }

        if let Some(action) = two_word_keyword_action(words) {
            return Some(action);
        }

        None
    }

    fn parse_protection_chain_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
        if let Some(action) = protection_from_colored_spells_action(tokens) {
            return Some(vec![action]);
        }
        if let Some(action) = protection_from_each_mana_value_among_action(tokens) {
            return Some(vec![action]);
        }
        parse_protection_chain(tokens)
    }

    fn parse_hexproof_from_chain_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
        parse_hexproof_from_chain(tokens)
    }

    if let Some(actions) = parse_flashback_keyword_line(tokens) {
        return Some(actions);
    }
    let words = TokenWordView::new(tokens).word_refs();
    if let Some(action) =
        super::activation_and_restrictions::keyword_action_costs::parse_dynamic_soulshift_keyword_action(&words)
    {
        return Some(vec![action]);
    }
    if let Some(action @ KeywordAction::CumulativeUpkeep { .. }) = parse_ability_phrase(tokens) {
        return Some(vec![action]);
    }

    let segments = clause_grammar::parse_ability_segments_tokens(tokens);
    let mut actions = Vec::new();
    for span in segments {
        let segment = &tokens[span.first..span.end];
        if segment.is_empty() {
            continue;
        }
        if let Some(protection_actions) = parse_protection_chain_lexed(segment) {
            actions.extend(protection_actions);
            continue;
        }

        if let Some(hexproof_actions) = parse_hexproof_from_chain_lexed(segment) {
            actions.extend(hexproof_actions);
            continue;
        }

        if let Some(action) = parse_simple_keyword_phrase_lexed(segment) {
            actions.push(action);
            continue;
        }

        let and_spans = clause_grammar::parse_conjoined_segments_tokens(segment);
        let and_parts = and_spans
            .iter()
            .map(|span| &segment[span.first..span.end])
            .collect::<Vec<_>>();
        if and_parts.len() > 1 {
            let mut all_ok = true;
            for part in &and_parts {
                if part.is_empty() {
                    continue;
                }
                if let Some(action) = parse_simple_keyword_phrase_lexed(part) {
                    actions.push(action);
                    continue;
                }
                if let Some(action) = parse_ability_phrase(part) {
                    actions.push(action);
                } else {
                    all_ok = false;
                    break;
                }
            }
            if !all_ok {
                return None;
            }
            continue;
        }

        if let Some(action) = parse_ability_phrase(segment) {
            actions.push(action);
            continue;
        }

        let and_spans = clause_grammar::parse_conjoined_segments_tokens(segment);
        let and_parts = and_spans
            .iter()
            .map(|span| &segment[span.first..span.end])
            .collect::<Vec<_>>();
        if and_parts.len() > 1 {
            let mut all_ok = true;
            for part in &and_parts {
                if part.is_empty() {
                    continue;
                }
                if let Some(action) = parse_ability_phrase(part) {
                    actions.push(action);
                } else {
                    all_ok = false;
                    break;
                }
            }
            if !all_ok {
                return None;
            }
        } else {
            return None;
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

pub(crate) fn parse_effect_sentences_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    super::effect_sentences::parse_effect_sentences_lexed(tokens)
}

fn parse_effect_sentences_or_single_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_sentences_lexed(tokens).or_else(|original_err| {
        let sentences = split_lexed_sentences(tokens);
        if sentences.len() == 1 {
            super::effect_sentences::parse_effect_sentence_lexed(sentences[0])
        } else {
            Err(original_err)
        }
    })
}

pub(crate) fn parse_triggered_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    if clause_grammar::parse_monstrous_damage_hand_trigger_tokens(tokens) {
        return Ok(LineAst::Triggered {
            trigger: TriggerSpec::ThisBecomesMonstrous,
            effects: vec![EffectAst::ForEachOpponent {
                effects: vec![EffectAst::subject_verb_damage(
                    Value::CardsInHand(PlayerFilter::IteratedPlayer),
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
            }],
            max_triggers_per_turn: None,
        });
    }

    fn parse_triggered_times_each_turn_lexed_from_sentences(
        tokens: &[OwnedLexToken],
    ) -> Option<u32> {
        split_lexed_sentences(tokens)
            .iter()
            .find_map(|sentence| parse_triggered_times_each_turn_lexed(sentence))
    }

    let trigger_intro = clause_grammar::parse_trigger_intro_tokens(tokens);
    let start_idx = trigger_intro.body_first;

    if let Some(effect_start) = clause_grammar::parse_combined_x_cost_trigger_tokens(tokens) {
        let mut spell_filter = ObjectFilter::instant_or_sorcery();
        spell_filter.has_x_in_cost = true;
        let mut ability_filter = ObjectFilter::default();
        ability_filter.has_x_in_cost = true;
        return Ok(LineAst::Triggered {
            trigger: TriggerSpec::Either(
                Box::new(TriggerSpec::SpellCast {
                    filter: Some(spell_filter),
                    caster: PlayerFilter::You,
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
            ),
            effects: parse_effect_sentences_lexed(&tokens[effect_start..])?,
            max_triggers_per_turn: None,
        });
    }

    if start_idx < tokens.len() {
        let trigger_body = &tokens[start_idx..];
        if let Some(prefix) = clause_grammar::parse_source_trigger_prefix_tokens(trigger_body) {
            let split_idx = start_idx + prefix.effect_first;
            let effects_tokens = trim_commas(&tokens[split_idx..]);
            match prefix.kind {
                SourceTriggerKind::BecomesBlocked => {
                    if clause_grammar::parse_blocked_damage_effect_tokens(&effects_tokens) {
                        let attacking = ObjectFilter {
                            zone: Some(Zone::Battlefield),
                            card_types: vec![CardType::Creature],
                            attacking: true,
                            ..ObjectFilter::default()
                        };
                        let blocking = ObjectFilter {
                            zone: Some(Zone::Battlefield),
                            card_types: vec![CardType::Creature],
                            blocking: true,
                            ..ObjectFilter::default()
                        };
                        return Ok(LineAst::Triggered {
                            trigger: TriggerSpec::ThisBecomesBlocked,
                            effects: vec![
                                EffectAst::subject_verb_damage_each(Value::Fixed(2), attacking),
                                EffectAst::subject_verb_damage_each(Value::Fixed(2), blocking),
                            ],
                            max_triggers_per_turn: None,
                        });
                    }
                    if !effects_tokens.is_empty()
                        && let Ok(effects) = parse_effect_sentences_lexed(&effects_tokens)
                    {
                        return Ok(LineAst::Triggered {
                            trigger: TriggerSpec::ThisBecomesBlocked,
                            effects,
                            max_triggers_per_turn: None,
                        });
                    }
                }
                SourceTriggerKind::LeavesBattlefield => {
                    let trigger_tokens = trim_commas(&tokens[start_idx..split_idx]);
                    if !effects_tokens.is_empty()
                        && let Ok(trigger) = parse_trigger_clause_lexed(&trigger_tokens)
                        && let Ok(effects) = parse_effect_sentences_lexed(&effects_tokens)
                    {
                        return Ok(LineAst::Triggered {
                            trigger,
                            effects,
                            max_triggers_per_turn: None,
                        });
                    }
                }
            }
        }
    }

    if let Some(spec) = split_triggered_conditional_clause_lexed(tokens, start_idx) {
        let (trigger_tokens, max_triggers_from_trigger_clause) =
            split_first_time_each_turn_trigger_suffix_lexed(spec.trigger_tokens);
        if let Ok(trigger) = parse_trigger_clause_lexed(trigger_tokens) {
            let rewritten_effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                trigger_tokens,
                spec.effects_tokens,
            );
            if let Ok(mut effects) =
                parse_effect_sentences_or_single_sentence_lexed(&rewritten_effects_tokens)
            {
                if predicate_counts_creatures_died_this_turn(&spec.predicate) {
                    bind_creatures_died_condition_amounts(&mut effects);
                }
                let mut max_triggers_per_turn =
                    parse_triggered_times_each_turn_lexed_from_sentences(&rewritten_effects_tokens);
                if let Some(max) = max_triggers_from_trigger_clause {
                    max_triggers_per_turn =
                        Some(max_triggers_per_turn.map_or(max, |existing| existing.min(max)));
                }
                return Ok(LineAst::Triggered {
                    trigger,
                    effects: vec![EffectAst::Conditional {
                        predicate: spec.predicate,
                        if_true: effects,
                        if_false: Vec::new(),
                    }],
                    max_triggers_per_turn,
                });
            }
        }
    }

    let delimiter_facts = clause_grammar::parse_trigger_delimiters_tokens(tokens);
    if let Some(split_idx) = delimiter_facts.first_comma {
        let trigger_tokens = &tokens[start_idx..split_idx];
        let trigger_word_view = TokenWordView::new(trigger_tokens);
        let trigger_words = trigger_word_view.word_refs();
        let attack_shape = clause_grammar::parse_attack_with_shape_tokens(trigger_tokens);
        if let Some(shape) = attack_shape.as_ref()
            && shape.attacked_words.is_none()
        {
            let subject_words = &trigger_words[shape.subject_words.clone()];
            if let Some(player) =
                super::activation_and_restrictions::parse_trigger_subject_player_filter(
                    subject_words,
                )
            {
                let mut object_tokens = &trigger_tokens[shape.object_token_first..];
                let mut min_total_attackers = None;
                let mut exact_total_attackers = None;
                let mut one_or_more = false;
                if let Some((count, stripped)) =
                    super::activation_and_restrictions::parse_leading_or_more_quantifier(
                        object_tokens,
                    )
                {
                    one_or_more = true;
                    object_tokens = stripped;
                    if count > 1 {
                        min_total_attackers = Some(count);
                    }
                } else if let Some((count, stripped)) =
                    super::activation_and_restrictions::parse_leading_exactly_quantifier(
                        object_tokens,
                    )
                {
                    one_or_more = true;
                    exact_total_attackers = Some(count);
                    object_tokens = stripped;
                }
                if !object_tokens.is_empty() {
                    let mut filter = super::object_filters::parse_object_filter_lexed(
                        object_tokens,
                        false,
                    )
                    .map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported attacking-object filter in trigger clause (clause: '{}')",
                            trigger_words.join(" ")
                        ))
                    })?;
                    if filter.controller.is_none() {
                        filter.controller = Some(player);
                    }
                    filter.set_union_one_or_more(one_or_more);
                    let trigger = if let Some(total_attackers) = exact_total_attackers {
                        TriggerSpec::AttacksOneOrMoreWithExactTotal {
                            filter,
                            total_attackers,
                        }
                    } else if let Some(min_total_attackers) = min_total_attackers {
                        TriggerSpec::AttacksOneOrMoreWithMinTotal {
                            filter,
                            min_total_attackers,
                        }
                    } else if one_or_more {
                        TriggerSpec::AttacksOneOrMore(filter)
                    } else {
                        TriggerSpec::Attacks(filter)
                    };
                    let effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                        trigger_tokens,
                        &tokens[split_idx + 1..],
                    );
                    let effects = parse_effect_sentences_lexed(&effects_tokens)?;
                    return Ok(LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: None,
                    });
                }
            }
        }

        if let Some(shape) = attack_shape.as_ref()
            && let Some(attacked_range) = shape.attacked_words.clone()
        {
            let subject_words = &trigger_words[shape.subject_words.clone()];
            let attacked_words = &trigger_words[attacked_range];
            if let (Some(player), Some((attacked_player, attacked_target_must_be_player))) = (
                super::activation_and_restrictions::parse_trigger_subject_player_filter(
                    subject_words,
                ),
                attacked_player_filter_from_words(attacked_words),
            ) {
                let mut object_tokens = &trigger_tokens[shape.object_token_first..];
                let mut min_total_attackers = None;
                let mut exact_total_attackers = None;
                let mut one_or_more = false;
                if let Some((count, stripped)) =
                    super::activation_and_restrictions::parse_leading_or_more_quantifier(
                        object_tokens,
                    )
                {
                    one_or_more = true;
                    object_tokens = stripped;
                    if count > 1 {
                        min_total_attackers = Some(count);
                    }
                } else if let Some((count, stripped)) =
                    super::activation_and_restrictions::parse_leading_exactly_quantifier(
                        object_tokens,
                    )
                {
                    one_or_more = true;
                    exact_total_attackers = Some(count);
                    object_tokens = stripped;
                }
                if !object_tokens.is_empty() {
                    let mut filter = super::object_filters::parse_object_filter_lexed(
                        object_tokens,
                        false,
                    )
                    .map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported attacking-object filter in trigger clause (clause: '{}')",
                            trigger_words.join(" ")
                        ))
                    })?;
                    if filter.controller.is_none() {
                        filter.controller = Some(player);
                    }
                    filter.set_union_one_or_more(one_or_more);
                    filter.attacking_player_or_planeswalker_controlled_by = Some(attacked_player);
                    if attacked_target_must_be_player {
                        filter.targets_only_player = Some(PlayerFilter::Any);
                    }
                    let trigger = if let Some(total_attackers) = exact_total_attackers {
                        TriggerSpec::AttacksOneOrMoreWithExactTotal {
                            filter,
                            total_attackers,
                        }
                    } else if let Some(min_total_attackers) = min_total_attackers {
                        TriggerSpec::AttacksOneOrMoreWithMinTotal {
                            filter,
                            min_total_attackers,
                        }
                    } else if one_or_more {
                        TriggerSpec::AttacksOneOrMore(filter)
                    } else {
                        TriggerSpec::Attacks(filter)
                    };
                    let effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                        trigger_tokens,
                        &tokens[split_idx + 1..],
                    );
                    let effects = parse_effect_sentences_lexed(&effects_tokens)?;
                    return Ok(LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: None,
                    });
                }
            }
        }
    }

    if let Some(delimiter) = delimiter_facts.first_comma_or_then {
        let mut split_idx = delimiter.index;
        let first_split_idx = split_idx;
        if delimiter.kind == TriggerDelimiterKind::Comma && trigger_intro.is_non_at_intro {
            let trigger_prefix_tokens = &tokens[start_idx..split_idx];
            let tail = &tokens[split_idx + 1..];
            if let Some(next_comma_rel) =
                find_trigger_effect_list_tail_split_lexed(trigger_prefix_tokens, tail)
            {
                let candidate_idx = split_idx + 1 + next_comma_rel;
                if candidate_idx > start_idx && candidate_idx + 1 < tokens.len() {
                    split_idx = candidate_idx;
                }
            }
        }

        let (trigger_tokens, max_triggers_from_trigger_clause) =
            split_first_time_each_turn_trigger_suffix_lexed(&tokens[start_idx..split_idx]);
        if let Ok(trigger) = parse_trigger_clause_lexed(trigger_tokens) {
            let effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                trigger_tokens,
                &tokens[split_idx + 1..],
            );
            match parse_effect_sentences_lexed(&effects_tokens) {
                Ok(effects) => {
                    let mut max_triggers_per_turn =
                        parse_triggered_times_each_turn_lexed_from_sentences(&effects_tokens);
                    if let Some(max) = max_triggers_from_trigger_clause {
                        max_triggers_per_turn =
                            Some(max_triggers_per_turn.map_or(max, |existing| existing.min(max)));
                    }
                    return Ok(LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn,
                    });
                }
                Err(err) => return Err(err),
            }
        }

        let state_split =
            split_state_triggered_clause_lexed(tokens, start_idx, split_idx).or_else(|| {
                (first_split_idx != split_idx)
                    .then(|| split_state_triggered_clause_lexed(tokens, start_idx, first_split_idx))
                    .flatten()
            });
        if let Some(spec) = state_split {
            let effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                spec.trigger_tokens,
                spec.effects_tokens,
            );
            let effects = parse_effect_sentences_lexed(&effects_tokens)?;
            return Ok(LineAst::Triggered {
                trigger: TriggerSpec::StateBased {
                    condition: spec.predicate,
                    display: spec
                        .display_tokens
                        .iter()
                        .map(|token| token.slice.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                },
                effects,
                max_triggers_per_turn: None,
            });
        }
    }

    // Try every comma as a trigger/effects split point.  Prefer the split
    // that produces the **longest** effects (earliest split_idx) rather than
    // the shortest (latest split_idx).  This prevents silent truncation where
    // only the last sentence parses and the rest is absorbed into the trigger.
    let mut best_result: Option<(usize, LineAst)> = None;
    for split_idx in ((start_idx + 1)..tokens.len()).rev() {
        let (trigger_tokens, max_triggers_from_trigger_clause) =
            split_first_time_each_turn_trigger_suffix_lexed(&tokens[start_idx..split_idx]);
        let effects_tokens = &tokens[split_idx..];
        if effects_tokens.is_empty() {
            continue;
        }
        if let Ok(trigger) = parse_trigger_clause_lexed(trigger_tokens) {
            let rewritten_effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                trigger_tokens,
                &effects_tokens,
            );
            let effects = parse_effect_sentences_lexed(&rewritten_effects_tokens).or_else(|_| {
                let Some(stripped) =
                    maybe_strip_leading_damage_subject_tokens(&rewritten_effects_tokens)
                else {
                    return Err(CardTextError::ParseError(String::new()));
                };
                parse_effect_sentences_lexed(stripped)
            });
            if let Ok(effects) = effects {
                let mut max_triggers_per_turn =
                    parse_triggered_times_each_turn_lexed_from_sentences(&rewritten_effects_tokens);
                if let Some(max) = max_triggers_from_trigger_clause {
                    max_triggers_per_turn =
                        Some(max_triggers_per_turn.map_or(max, |existing| existing.min(max)));
                }
                let effect_token_count = effects_tokens.len();
                let line_ast = LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn,
                };
                // Keep the split that produces the most effect tokens
                // (earliest split point = most effects).
                if best_result
                    .as_ref()
                    .map_or(true, |(prev_count, _)| effect_token_count > *prev_count)
                {
                    best_result = Some((effect_token_count, line_ast));
                }
            }
        }
    }
    if let Some((effect_count, line_ast)) = best_result {
        // Reject splits where the effects cover too little of the total line
        // AND there are multiple sentences (periods) in the line.  A single-
        // sentence triggered ability can legitimately have a long trigger and
        // short effect; the truncation problem only arises when multi-sentence
        // lines silently lose entire sentences.
        let total_token_count = tokens.len().saturating_sub(start_idx);
        let period_count = tokens
            .iter()
            .filter(|t| t.kind == crate::runtime_backend::lexer::TokenKind::Period)
            .count();
        if period_count >= 2 && total_token_count > 15 && effect_count * 4 < total_token_count {
            return Err(CardTextError::ParseError(format!(
                "triggered line effects cover too few tokens ({effect_count}/{total_token_count}), \
                 likely missing unsupported clauses (line: '{}')",
                TokenWordView::new(tokens).word_refs().join(" ")
            )));
        }
        return Ok(line_ast);
    }

    Err(CardTextError::ParseError(format!(
        "unsupported triggered line (clause: '{}')",
        TokenWordView::new(tokens).word_refs().join(" ")
    )))
}

pub(crate) fn parse_trigger_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<TriggerSpec, CardTextError> {
    super::activation_and_restrictions::parse_trigger_clause_lexed(tokens)
}

pub(crate) fn parse_static_ability_ast_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    super::keyword_static::parse_static_ability_ast_line_lexed(tokens)
}

#[cfg(test)]
mod tests {
    use super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn typed_protection_chain_preserves_keyword_actions() {
        let tokens = lex_line("Protection from red and from blue", 0).unwrap();
        let actions = parse_ability_line_lexed(&tokens).unwrap();
        assert_eq!(
            actions,
            vec![
                KeywordAction::ProtectionFrom(crate::color::Color::Red.into()),
                KeywordAction::ProtectionFrom(crate::color::Color::Blue.into()),
            ]
        );
    }

    #[test]
    fn typed_attack_with_shape_preserves_attacked_player_filter() {
        let tokens = lex_line(
            "Whenever you attack an opponent with one or more creatures, draw a card.",
            0,
        )
        .unwrap();
        let parsed = parse_triggered_line_lexed(&tokens).unwrap();
        let LineAst::Triggered {
            trigger: TriggerSpec::AttacksOneOrMore(filter),
            effects,
            ..
        } = &parsed
        else {
            panic!("expected aggregate attack trigger, got {parsed:#?}");
        };
        assert!(filter.union_is_one_or_more());
        assert_eq!(
            filter
                .attacking_player_or_planeswalker_controlled_by
                .as_ref(),
            Some(&PlayerFilter::Opponent)
        );
        assert!(format!("{effects:#?}").contains("Draw"));
    }

    #[test]
    fn typed_attack_with_group_keeps_that_many_bound_to_attacker_count() {
        let tokens = lex_line(
            "Whenever you attack with one or more creatures, target player mills that many cards.",
            0,
        )
        .unwrap();
        let parsed = parse_triggered_line_lexed(&tokens).unwrap();
        let LineAst::Triggered {
            trigger: TriggerSpec::AttacksOneOrMore(filter),
            effects,
            ..
        } = &parsed
        else {
            panic!("expected aggregate attack trigger, got {parsed:#?}");
        };
        assert!(filter.union_is_one_or_more());
        assert!(matches!(
            effects.as_slice(),
            [EffectAst::SubjectVerb(subject_verb)]
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::Mill { count }
                        if matches!(
                            count.unhinted(),
                            Value::EventValue(EventValueSpec::Amount)
                        )
                )
        ));
    }

    #[test]
    fn typed_attack_with_group_preserves_player_or_planeswalker_target() {
        let tokens = lex_line(
            "Whenever you attack a player or planeswalker with one or more creatures with power 1 or less, draw a card.",
            0,
        )
        .unwrap();
        let parsed = parse_triggered_line_lexed(&tokens).unwrap();
        let LineAst::Triggered {
            trigger: TriggerSpec::AttacksOneOrMore(filter),
            ..
        } = &parsed
        else {
            panic!("expected aggregate attack trigger, got {parsed:#?}");
        };
        assert!(filter.union_is_one_or_more());
        assert_eq!(
            filter
                .attacking_player_or_planeswalker_controlled_by
                .as_ref(),
            Some(&PlayerFilter::Any)
        );
        assert!(filter.targets_only_player.is_none());
        assert!(filter.power.is_some());
    }

    #[test]
    fn typed_source_prefix_preserves_blocked_trigger() {
        let tokens = lex_line("Whenever this creature becomes blocked, draw a card.", 0).unwrap();
        let parsed = parse_triggered_line_lexed(&tokens).unwrap();
        assert!(
            matches!(
                parsed,
                LineAst::Triggered {
                    trigger: TriggerSpec::ThisBecomesBlocked,
                    ..
                }
            ),
            "{parsed:#?}"
        );
    }
}
