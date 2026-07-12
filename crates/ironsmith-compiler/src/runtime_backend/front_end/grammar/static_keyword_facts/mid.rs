use crate::effect::Comparison;
use crate::target::ObjectFilter;
use crate::types::CardType;

use super::super::super::lexer::{OwnedLexToken, TokenWordView, trim_lexed_commas};
use super::super::filters::parse_object_filter_with_grammar_entrypoint_lexed;
use super::super::shared_util::value_shapes::parse_quantity_comparison_prefix_words;
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttachedChoiceSubject {
    Equipment,
    Aura,
    Permanent,
    Artifact,
    Enchantment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AttachedColorChoiceFact<'a> {
    pub(crate) subject: AttachedChoiceSubject,
    pub(crate) choice_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RedirectDamageToSourceFact;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CastMarkerFact;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CostModifierDirectionFact {
    Less,
    More,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CostTargetFact {
    You,
    Opponent,
    AnyPlayer,
    Object(ObjectFilter),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum KnownSpellCostConditionFact {
    LifeTotalLessThanStarting,
    AttackedThisTurn,
    CreatureDiedThisTurn,
    Night,
    Bargained,
    SacrificedArtifactThisTurn,
    CommittedCrimeThisTurn,
    CreatureLeftBattlefieldUnderYourControlThisTurn,
    CastThisTurn {
        another: bool,
        card_types: Vec<CardType>,
    },
    NotStartingPlayer,
    CreatureIsAttackingYou,
    CreatureCardPutIntoYourGraveyardThisTurn,
    DistinctCardTypesInYourGraveyardOrMore(u32),
    CardsInYourGraveyardOrMore {
        count: u32,
        card_types: Vec<CardType>,
    },
    OpponentHasPoisonCountersOrMore(u32),
    OpponentHasCardsInGraveyardOrMore(u32),
    NoCardsInHandMatching(ObjectFilter),
    OnlyCreatureCardsInHandNamed(String),
    CardInYourGraveyardMatching(ObjectFilter),
    TargetsLargeControlledCreature,
    Target(CostTargetFact),
    OpponentHasNoCardsInHand,
    OpponentControlsLandsOrMore(u32),
    OpponentControlsMoreCreaturesThanYou(u32),
    TotalCreatureCardsInAllGraveyardsOrMore(u32),
    OpponentCastSpellsThisTurnOrMore(u32),
    OpponentDrewCardsThisTurnOrMore(u32),
    YouWereDealtDamageByCreaturesThisTurnOrMore(u32),
    AssassinOrCommanderDealtCombatDamage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FirstSpellEachTurnCostFact;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpellCastActorFact {
    You,
    Opponent,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SpellCostBetweenFact<'a> {
    pub(crate) actor: Option<SpellCastActorFact>,
    pub(crate) from_your_graveyard: bool,
    pub(crate) descriptor_segments: Vec<&'a [OwnedLexToken]>,
    pub(crate) target_tokens: Option<&'a [OwnedLexToken]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CostComponentBoundary {
    pub(crate) cost_token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CyclingCostAlternativeFact<'a> {
    pub(crate) condition_tokens: Option<&'a [OwnedLexToken]>,
    pub(crate) replacement_cost_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivatedAbilityCostActorFact {
    You,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActivatedAbilityCostTailFact {
    pub(crate) excludes_mana_abilities: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TrailingTargetConditionFact<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AlternativeCostPayerFact {
    You,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ForetellCostModifierFact {
    pub(crate) direction: CostModifierDirectionFact,
    pub(crate) during_any_players_turn: bool,
}

pub(crate) fn parse_attached_color_choice_fact(
    tokens: &[OwnedLexToken],
) -> Option<AttachedColorChoiceFact<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let rest = primitives::parse_word_sequence_prefix(&words, &["as", "this"])?;
    let consumed = words.len().checked_sub(rest.len())?;
    let subject = match words.get(consumed).copied()? {
        "equipment" => AttachedChoiceSubject::Equipment,
        "aura" => AttachedChoiceSubject::Aura,
        "permanent" => AttachedChoiceSubject::Permanent,
        "artifact" => AttachedChoiceSubject::Artifact,
        "enchantment" => AttachedChoiceSubject::Enchantment,
        _ => return None,
    };
    let becomes_start = consumed + 1;
    let after_attached = primitives::parse_word_sequence_prefix(
        words.get(becomes_start..)?,
        &["becomes", "attached", "to"],
    )?;
    let after_attached_word = words.len().checked_sub(after_attached.len())?;
    let choose =
        primitives::parse_word_sequence_span(words.get(after_attached_word..)?, &["choose"])?;
    let choose_word = after_attached_word + choose.start;
    if choose_word <= 6 {
        return None;
    }
    let choose_token = view.token_boundary_for_word(choose_word)?;
    Some(AttachedColorChoiceFact {
        subject,
        choice_tokens: tokens.get(choose_token..)?,
    })
}

pub(crate) fn parse_redirect_damage_to_source_fact(
    tokens: &[OwnedLexToken],
) -> Option<RedirectDamageToSourceFact> {
    let words = TokenWordView::new(tokens).word_refs();
    if words.len() != 19 {
        return None;
    }
    let prefix = [
        "all", "damage", "that", "would", "be", "dealt", "to", "you", "and", "other",
    ];
    primitives::parse_word_sequence_prefix(&words, &prefix)?;
    if !matches!(words.get(10).copied(), Some("permanent" | "permanents")) {
        return None;
    }
    primitives::parse_word_sequence_complete(
        words.get(11..)?,
        &[
            "you", "control", "is", "dealt", "to", "this", "creature", "instead",
        ],
    )?;
    Some(RedirectDamageToSourceFact)
}

pub(crate) fn parse_cast_marker_fact(tokens: &[OwnedLexToken]) -> Option<CastMarkerFact> {
    contains_word(&TokenWordView::new(tokens).word_refs(), "cast").then_some(CastMarkerFact)
}

pub(crate) fn parse_cost_modifier_direction_words(
    words: &[&str],
) -> Option<CostModifierDirectionFact> {
    let less = primitives::parse_word_sequence_span(words, &["less"]).is_some();
    let more = primitives::parse_word_sequence_span(words, &["more"]).is_some();
    match (less, more) {
        (true, false) => Some(CostModifierDirectionFact::Less),
        (false, true) => Some(CostModifierDirectionFact::More),
        _ => None,
    }
}

pub(crate) fn parse_cost_modifier_direction_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CostModifierDirectionFact> {
    parse_cost_modifier_direction_words(&TokenWordView::new(tokens).word_refs())
}

pub(crate) fn parse_this_spell_target_fact(tokens: &[OwnedLexToken]) -> Option<CostTargetFact> {
    parse_target_fact(tokens, false, true)
}

pub(crate) fn parse_cost_modifier_target_fact(tokens: &[OwnedLexToken]) -> Option<CostTargetFact> {
    parse_target_fact(tokens, true, false)
}

fn parse_target_fact(
    tokens: &[OwnedLexToken],
    allow_plural_player: bool,
    require_target_head: bool,
) -> Option<CostTargetFact> {
    let tokens = trim_lexed_commas(tokens);
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let (target_start, target_words) =
        if let Some(rest) = primitives::parse_word_sequence_prefix(&words, &["it", "targets"]) {
            (words.len().checked_sub(rest.len())?, rest)
        } else if let Some(rest) =
            primitives::parse_word_sequence_prefix(&words, &["this", "spell", "targets"])
        {
            (words.len().checked_sub(rest.len())?, rest)
        } else if !require_target_head {
            (0, words.as_slice())
        } else {
            return None;
        };
    if target_words.is_empty() {
        return None;
    }
    if primitives::parse_word_sequence_prefix(target_words, &["you"]).is_some() {
        return Some(CostTargetFact::You);
    }
    if primitives::parse_word_sequence_prefix(target_words, &["an", "opponent"]).is_some()
        || primitives::parse_word_sequence_prefix(target_words, &["opponent"]).is_some()
        || (allow_plural_player
            && primitives::parse_word_sequence_prefix(target_words, &["opponents"]).is_some())
    {
        return Some(CostTargetFact::Opponent);
    }
    if primitives::parse_word_sequence_prefix(target_words, &["a", "player"]).is_some()
        || primitives::parse_word_sequence_prefix(target_words, &["player"]).is_some()
        || (allow_plural_player
            && primitives::parse_word_sequence_prefix(target_words, &["players"]).is_some())
    {
        return Some(CostTargetFact::AnyPlayer);
    }
    let target_token = view.token_boundary_for_word(target_start)?;
    let filter = parse_object_filter_with_grammar_entrypoint_lexed(
        trim_lexed_commas(tokens.get(target_token..)?),
        false,
    )
    .ok()?;
    Some(CostTargetFact::Object(filter))
}

pub(crate) fn parse_known_spell_cost_condition(
    tokens: &[OwnedLexToken],
) -> Option<KnownSpellCostConditionFact> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if words.is_empty() {
        return None;
    }

    if exact(
        &words,
        &[
            "your", "life", "total", "is", "less", "than", "your", "starting", "life", "total",
        ],
    ) {
        return Some(KnownSpellCostConditionFact::LifeTotalLessThanStarting);
    }
    if exact_any(
        &words,
        &[
            &["you", "attacked", "this", "turn"],
            &["youve", "attacked", "this", "turn"],
        ],
    ) {
        return Some(KnownSpellCostConditionFact::AttackedThisTurn);
    }
    if exact_any(
        &words,
        &[
            &["a", "creature", "died", "this", "turn"],
            &["creature", "died", "this", "turn"],
        ],
    ) {
        return Some(KnownSpellCostConditionFact::CreatureDiedThisTurn);
    }
    if exact_any(&words, &[&["its", "night"], &["it", "is", "night"]]) {
        return Some(KnownSpellCostConditionFact::Night);
    }
    if exact_any(
        &words,
        &[
            &["its", "bargained"],
            &["it's", "bargained"],
            &["it", "is", "bargained"],
            &["this", "spell", "is", "bargained"],
            &["this", "spell", "was", "bargained"],
        ],
    ) {
        return Some(KnownSpellCostConditionFact::Bargained);
    }
    if exact_any(
        &words,
        &[
            &["youve", "sacrificed", "an", "artifact", "this", "turn"],
            &["you", "sacrificed", "an", "artifact", "this", "turn"],
        ],
    ) {
        return Some(KnownSpellCostConditionFact::SacrificedArtifactThisTurn);
    }
    if exact_any(
        &words,
        &[
            &["youve", "committed", "a", "crime", "this", "turn"],
            &["you", "committed", "a", "crime", "this", "turn"],
        ],
    ) {
        return Some(KnownSpellCostConditionFact::CommittedCrimeThisTurn);
    }
    if exact(
        &words,
        &[
            "a",
            "creature",
            "left",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ],
    ) {
        return Some(KnownSpellCostConditionFact::CreatureLeftBattlefieldUnderYourControlThisTurn);
    }

    if has_suffix(&words, &["this", "turn"]) {
        if starts_with_any(
            &words,
            &[
                &["youve", "cast", "another"],
                &["you've", "cast", "another"],
                &["you", "cast", "another"],
                &["you", "ve", "cast", "another"],
            ],
        ) {
            return Some(KnownSpellCostConditionFact::CastThisTurn {
                another: true,
                card_types: mentioned_instant_sorcery_types(&words),
            });
        }
        if starts_with_any(
            &words,
            &[
                &["youve", "cast"],
                &["you've", "cast"],
                &["you", "cast"],
                &["you", "ve", "cast"],
            ],
        ) {
            let card_types = mentioned_instant_sorcery_types(&words);
            if !card_types.is_empty() {
                return Some(KnownSpellCostConditionFact::CastThisTurn {
                    another: false,
                    card_types,
                });
            }
        }
    }

    if exact(&words, &["you", "werent", "the", "starting", "player"]) {
        return Some(KnownSpellCostConditionFact::NotStartingPlayer);
    }
    if exact(&words, &["a", "creature", "is", "attacking", "you"]) {
        return Some(KnownSpellCostConditionFact::CreatureIsAttackingYou);
    }
    if exact(
        &words,
        &[
            "a",
            "creature",
            "card",
            "was",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "anywhere",
            "this",
            "turn",
        ],
    ) {
        return Some(KnownSpellCostConditionFact::CreatureCardPutIntoYourGraveyardThisTurn);
    }

    if starts_with(&words, &["there", "are"])
        && contains_all_words(&words, &["card", "types", "graveyard"])
        && let Some((count, _)) = at_least_quantity(&words, 2)
    {
        return Some(KnownSpellCostConditionFact::DistinctCardTypesInYourGraveyardOrMore(count));
    }
    if starts_with(&words, &["you", "have"])
        && has_suffix(&words, &["in", "your", "graveyard"])
        && let Some((count, _)) = at_least_quantity(&words, 2)
    {
        return Some(KnownSpellCostConditionFact::CardsInYourGraveyardOrMore {
            count,
            card_types: mentioned_instant_sorcery_types(&words),
        });
    }
    if let Some(fact) = parse_opponent_has_threshold(&words) {
        return Some(fact);
    }

    if let Some(rest) = primitives::parse_word_sequence_prefix(&words, &["there", "are", "no"])
        && has_suffix(&words, &["in", "your", "hand"])
    {
        let start_word = words.len().checked_sub(rest.len())?;
        let start_token = view.token_boundary_for_word(start_word)?;
        if let Ok(filter) = parse_object_filter_with_grammar_entrypoint_lexed(
            trim_lexed_commas(tokens.get(start_token..)?),
            false,
        ) {
            return Some(KnownSpellCostConditionFact::NoCardsInHandMatching(filter));
        }
    }
    if let Some(name) = parse_only_creature_cards_in_hand_named(&words) {
        return Some(KnownSpellCostConditionFact::OnlyCreatureCardsInHandNamed(
            name,
        ));
    }
    if let Some(rest) = primitives::parse_word_sequence_prefix(&words, &["there", "is"])
        && has_suffix(&words, &["in", "your", "graveyard"])
    {
        let start_word = words.len().checked_sub(rest.len())?;
        let start_token = view.token_boundary_for_word(start_word)?;
        if let Ok(filter) = parse_object_filter_with_grammar_entrypoint_lexed(
            trim_lexed_commas(tokens.get(start_token..)?),
            false,
        ) {
            return Some(KnownSpellCostConditionFact::CardInYourGraveyardMatching(
                filter,
            ));
        }
    }

    if exact(
        &words,
        &[
            "it", "targets", "a", "spell", "or", "ability", "that", "targets", "a", "creature",
            "you", "control", "with", "power", "7", "or", "greater",
        ],
    ) {
        return Some(KnownSpellCostConditionFact::TargetsLargeControlledCreature);
    }
    if let Some(target) = parse_this_spell_target_fact(tokens) {
        return Some(KnownSpellCostConditionFact::Target(target));
    }
    if exact_any(
        &words,
        &[
            &["an", "opponent", "has", "no", "cards", "in", "hand"],
            &["opponent", "has", "no", "cards", "in", "hand"],
        ],
    ) {
        return Some(KnownSpellCostConditionFact::OpponentHasNoCardsInHand);
    }
    if let Some(fact) = parse_remaining_threshold_condition(&words) {
        return Some(fact);
    }
    if starts_with(
        &words,
        &[
            "you",
            "dealt",
            "combat",
            "damage",
            "to",
            "a",
            "player",
            "this",
            "turn",
            "with",
            "an",
            "assassin",
            "or",
            "commander",
        ],
    ) {
        return Some(KnownSpellCostConditionFact::AssassinOrCommanderDealtCombatDamage);
    }
    None
}

pub(crate) fn parse_first_spell_each_turn_cost_fact(
    tokens: &[OwnedLexToken],
) -> Option<FirstSpellEachTurnCostFact> {
    let words = TokenWordView::new(tokens).word_refs();
    contains_all_words(&words, &["first", "each", "turn"]).then_some(())?;
    let has_cost = primitives::parse_word_sequence_span(&words, &["cost"]).is_some()
        || primitives::parse_word_sequence_span(&words, &["costs"]).is_some();
    has_cost.then_some(FirstSpellEachTurnCostFact)
}

pub(crate) fn parse_spell_cost_between_fact(tokens: &[OwnedLexToken]) -> SpellCostBetweenFact<'_> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let actor = if (contains_word(&words, "opponent") || contains_word(&words, "opponents"))
        && (contains_word(&words, "cast") || contains_word(&words, "casts"))
    {
        Some(SpellCastActorFact::Opponent)
    } else if primitives::parse_word_sequence_span(&words, &["you", "cast"]).is_some() {
        Some(SpellCastActorFact::You)
    } else {
        None
    };
    let from_your_graveyard =
        primitives::parse_word_sequence_span(&words, &["from", "your", "graveyard"]).is_some();
    let target_tokens = ["target", "targets"].into_iter().find_map(|target_word| {
        let span = primitives::parse_word_sequence_span(&words, &["that", target_word])?;
        let target_start_word = span.start + span.len;
        let target_start_token = view.token_boundary_for_word(target_start_word)?;
        let tail = trim_lexed_commas(tokens.get(target_start_token..)?);
        (!tail.is_empty()).then_some(tail)
    });

    let mut descriptor_segments = Vec::new();
    let mut idx = 0usize;
    while idx < tokens.len() {
        if !tokens[idx].is_word("spell") && !tokens[idx].is_word("spells") {
            idx += 1;
            continue;
        }
        let mut start = idx;
        while start > 0
            && !tokens[start - 1].is_word("and")
            && !tokens[start - 1].is_word("or")
            && !tokens[start - 1].is_comma()
        {
            start -= 1;
        }
        let descriptor = trim_lexed_commas(&tokens[start..idx]);
        if !descriptor.is_empty() {
            descriptor_segments.push(descriptor);
        }
        idx += 1;
    }

    SpellCostBetweenFact {
        actor,
        from_your_graveyard,
        descriptor_segments,
        target_tokens,
    }
}

pub(crate) fn parse_cost_component_boundary(
    tokens: &[OwnedLexToken],
    start_token: usize,
) -> Option<CostComponentBoundary> {
    let mut idx = start_token;
    while idx < tokens.len() {
        if tokens[idx].is_word("cost") || tokens[idx].is_word("costs") {
            let amount = tokens.get(idx + 1..)?;
            if leaf::parse_leaf_number_or_x_prefix_tokens(amount).is_some()
                || leaf::parse_leaf_mana_cost_prefix_tokens(amount).is_some()
            {
                return Some(CostComponentBoundary { cost_token: idx });
            }
        }
        idx += 1;
    }
    None
}

pub(crate) fn parse_where_x_clause_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let span = primitives::parse_word_sequence_span(&words, &["where", "x", "is"])?;
    let token = view.token_boundary_for_word(span.start)?;
    Some(trim_lexed_commas(tokens.get(token..)?))
}

pub(crate) fn parse_cycling_cost_alternative_fact(
    tokens: &[OwnedLexToken],
) -> Option<CyclingCostAlternativeFact<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let (condition_words, body_start) = if starts_with(&words, &["as", "long", "as"]) {
        let body = primitives::parse_word_sequence_span(&words, &["you", "may", "pay"])?;
        if body.start < 3 {
            return None;
        }
        (Some((3usize, body.start)), body.start)
    } else {
        (None, 0)
    };
    primitives::parse_word_sequence_prefix(words.get(body_start..)?, &["you", "may", "pay"])?;
    let rather = primitives::parse_word_sequence_span(
        words.get(body_start..)?,
        &["rather", "than", "pay", "cycling", "costs"],
    )?;
    let rather_word = body_start + rather.start;
    let cost_start_word = body_start + 3;
    let cost_start_token = view.token_boundary_for_word_or_end(cost_start_word)?;
    let cost_end_token = view.token_boundary_for_word(rather_word)?;
    let condition_tokens = condition_words.and_then(|(start, end)| {
        let start_token = view.token_boundary_for_word(start)?;
        let end_token = view.token_boundary_for_word(end)?;
        Some(trim_lexed_commas(&tokens[start_token..end_token]))
    });
    Some(CyclingCostAlternativeFact {
        condition_tokens,
        replacement_cost_tokens: trim_lexed_commas(&tokens[cost_start_token..cost_end_token]),
    })
}

pub(crate) fn parse_activated_ability_cost_actor(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedAbilityCostActorFact> {
    let words = TokenWordView::new(tokens).word_refs();
    if exact(&words, &["you"]) {
        Some(ActivatedAbilityCostActorFact::You)
    } else if exact_any(&words, &[&["your", "opponents"], &["opponents"]]) {
        Some(ActivatedAbilityCostActorFact::Opponent)
    } else {
        None
    }
}

pub(crate) fn parse_activated_ability_cost_tail(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedAbilityCostTailFact> {
    let words = TokenWordView::new(tokens).word_refs();
    primitives::parse_word_sequence_span(&words, &["to", "activate"]).map(|_| ())?;
    Some(ActivatedAbilityCostTailFact {
        excludes_mana_abilities: primitives::parse_word_sequence_span(
            &words,
            &["unless", "theyre", "mana", "abilities"],
        )
        .is_some()
            || primitives::parse_word_sequence_span(
                &words,
                &["unless", "they're", "mana", "abilities"],
            )
            .is_some(),
    })
}

pub(crate) fn parse_trailing_target_condition(
    tokens: &[OwnedLexToken],
) -> Option<TrailingTargetConditionFact<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let span = primitives::parse_word_sequence_span(&words, &["if", "it", "targets"])
        .or_else(|| primitives::parse_word_sequence_span(&words, &["if", "it", "target"]))?;
    let target_word = span.start + span.len;
    let target_token = view.token_boundary_for_word_or_end(target_word)?;
    let target_tokens = trim_lexed_commas(tokens.get(target_token..)?);
    Some(TrailingTargetConditionFact { target_tokens })
}

pub(crate) fn parse_alternative_cost_payer(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCostPayerFact> {
    let words = TokenWordView::new(tokens).word_refs();
    if primitives::parse_word_sequence_span(&words, &["you", "pay"]).is_some() {
        Some(AlternativeCostPayerFact::You)
    } else if primitives::parse_word_sequence_span(&words, &["your", "opponents", "pay"]).is_some()
        || primitives::parse_word_sequence_span(&words, &["opponents", "pay"]).is_some()
        || primitives::parse_word_sequence_span(&words, &["opponent", "pays"]).is_some()
    {
        Some(AlternativeCostPayerFact::Opponent)
    } else {
        None
    }
}

pub(crate) fn parse_foretell_cost_modifier_fact(
    tokens: &[OwnedLexToken],
) -> Option<ForetellCostModifierFact> {
    let words = TokenWordView::new(tokens).word_refs();
    starts_with(
        &words,
        &["foretelling", "cards", "from", "your", "hand", "costs"],
    )
    .then_some(())?;
    let during_any_players_turn =
        primitives::parse_word_sequence_span(&words, &["on", "any", "players", "turn"]).is_some()
            || primitives::parse_word_sequence_span(&words, &["on", "any", "player", "turn"])
                .is_some()
            || primitives::parse_word_sequence_span(&words, &["on", "any", "player", "s", "turn"])
                .is_some();
    Some(ForetellCostModifierFact {
        direction: parse_cost_modifier_direction_words(&words)?,
        during_any_players_turn,
    })
}

fn parse_opponent_has_threshold(words: &[&str]) -> Option<KnownSpellCostConditionFact> {
    let count_start = if starts_with(words, &["an", "opponent", "has"]) {
        3
    } else if starts_with(words, &["opponent", "has"]) {
        2
    } else {
        return None;
    };
    let (count, rest) = at_least_quantity(words, count_start)?;
    let tail = words.get(rest..)?;
    if exact_any(tail, &[&["poison", "counters"], &["poison", "counter"]]) {
        Some(KnownSpellCostConditionFact::OpponentHasPoisonCountersOrMore(count))
    } else if exact_any(
        tail,
        &[
            &["cards", "in", "their", "graveyard"],
            &["cards", "in", "his", "graveyard"],
            &["cards", "in", "her", "graveyard"],
            &["card", "in", "their", "graveyard"],
        ],
    ) {
        Some(KnownSpellCostConditionFact::OpponentHasCardsInGraveyardOrMore(count))
    } else {
        None
    }
}

fn parse_remaining_threshold_condition(words: &[&str]) -> Option<KnownSpellCostConditionFact> {
    if starts_with(words, &["an", "opponent", "controls"]) {
        let (count, rest) = at_least_quantity(words, 3)?;
        let tail = words.get(rest..)?;
        if exact_any(tail, &[&["lands"], &["land"]]) {
            return Some(KnownSpellCostConditionFact::OpponentControlsLandsOrMore(
                count,
            ));
        }
        if exact_any(
            tail,
            &[
                &["more", "creatures", "than", "you"],
                &["more", "creature", "than", "you"],
            ],
        ) {
            return Some(KnownSpellCostConditionFact::OpponentControlsMoreCreaturesThanYou(count));
        }
    }
    if starts_with(words, &["there", "are"]) {
        let (count, rest) = at_least_quantity(words, 2)?;
        if exact(
            words.get(rest..)?,
            &["creature", "cards", "total", "in", "all", "graveyards"],
        ) {
            return Some(
                KnownSpellCostConditionFact::TotalCreatureCardsInAllGraveyardsOrMore(count),
            );
        }
    }
    if starts_with_any(words, &[&["an", "opponent", "cast"], &["opponent", "cast"]]) {
        let start = if starts_with(words, &["an"]) { 3 } else { 2 };
        let (count, rest) = at_least_quantity(words, start)?;
        if exact_any(
            words.get(rest..)?,
            &[&["spells", "this", "turn"], &["spell", "this", "turn"]],
        ) {
            return Some(KnownSpellCostConditionFact::OpponentCastSpellsThisTurnOrMore(count));
        }
    }
    if starts_with_any(
        words,
        &[
            &["an", "opponent", "has", "drawn"],
            &["opponent", "has", "drawn"],
        ],
    ) {
        let start = if starts_with(words, &["an"]) { 4 } else { 3 };
        let (count, rest) = at_least_quantity(words, start)?;
        if exact_any(
            words.get(rest..)?,
            &[&["cards", "this", "turn"], &["card", "this", "turn"]],
        ) {
            return Some(KnownSpellCostConditionFact::OpponentDrewCardsThisTurnOrMore(count));
        }
    }
    let damage_start = if starts_with(words, &["you", "have", "been", "dealt", "damage", "by"]) {
        Some(6)
    } else if starts_with(words, &["youve", "been", "dealt", "damage", "by"]) {
        Some(5)
    } else {
        None
    };
    if let Some(start) = damage_start {
        let (count, rest) = at_least_quantity(words, start)?;
        if exact_any(
            words.get(rest..)?,
            &[
                &["creatures", "this", "turn"],
                &["creature", "this", "turn"],
            ],
        ) {
            return Some(
                KnownSpellCostConditionFact::YouWereDealtDamageByCreaturesThisTurnOrMore(count),
            );
        }
    }
    None
}

fn parse_only_creature_cards_in_hand_named(words: &[&str]) -> Option<String> {
    let first_surface = starts_with(words, &["you", "have", "no", "other", "creature", "cards"])
        && primitives::parse_word_sequence_span(words, &["or", "if"]).is_some();
    let second_surface = starts_with(
        words,
        &[
            "the", "only", "other", "creature", "cards", "in", "your", "hand", "are", "named",
        ],
    );
    if !first_surface && !second_surface {
        return None;
    }
    let named = primitives::parse_word_sequence_span(words, &["named"])?;
    let name = words.get(named.start + named.len..)?.join(" ");
    (!name.is_empty()).then_some(name)
}

fn at_least_quantity(words: &[&str], start: usize) -> Option<(u32, usize)> {
    let parsed = parse_quantity_comparison_prefix_words(words.get(start..)?, false, false)?;
    let count = strict_at_least(&parsed.comparison)?;
    Some((count, start + parsed.consumed_words))
}

fn strict_at_least(comparison: &Comparison) -> Option<u32> {
    match comparison {
        Comparison::GreaterThanOrEqual(value) if *value >= 0 => Some(*value as u32),
        Comparison::GreaterThan(value) if *value >= -1 => Some((*value + 1) as u32),
        _ => None,
    }
}

fn mentioned_instant_sorcery_types(words: &[&str]) -> Vec<CardType> {
    let mut types = Vec::new();
    if contains_word(words, "instant") || contains_word(words, "instants") {
        types.push(CardType::Instant);
    }
    if contains_word(words, "sorcery") || contains_word(words, "sorceries") {
        types.push(CardType::Sorcery);
    }
    types
}

fn exact(words: &[&str], expected: &[&str]) -> bool {
    primitives::parse_word_sequence_complete(words, expected).is_some()
}

fn exact_any(words: &[&str], expected: &[&[&str]]) -> bool {
    expected.iter().any(|phrase| exact(words, phrase))
}

fn starts_with(words: &[&str], expected: &[&str]) -> bool {
    primitives::parse_word_sequence_prefix(words, expected).is_some()
}

fn starts_with_any(words: &[&str], expected: &[&[&str]]) -> bool {
    expected.iter().any(|phrase| starts_with(words, phrase))
}

fn has_suffix(words: &[&str], expected: &[&str]) -> bool {
    primitives::parse_word_sequence_suffix(words, expected).is_some()
}

fn contains_word(words: &[&str], expected: &str) -> bool {
    primitives::parse_word_sequence_span(words, &[expected]).is_some()
}

fn contains_all_words(words: &[&str], expected: &[&str]) -> bool {
    expected.iter().all(|word| contains_word(words, word))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("static keyword fact fixture should lex")
    }

    #[test]
    fn parses_mid_static_keyword_facts() {
        assert_eq!(
            parse_known_spell_cost_condition(&lex("you attacked this turn")),
            Some(KnownSpellCostConditionFact::AttackedThisTurn)
        );
        assert_eq!(
            parse_known_spell_cost_condition(&lex("an opponent controls seven or more lands")),
            Some(KnownSpellCostConditionFact::OpponentControlsLandsOrMore(7))
        );
        assert_eq!(
            parse_cost_modifier_direction_tokens(&lex("two less to cast")),
            Some(CostModifierDirectionFact::Less)
        );
        assert!(parse_where_x_clause_tokens(&lex("less to cast, where X is three")).is_some());
    }
}
