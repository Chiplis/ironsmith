//! Cost modification static abilities.
//!
//! These abilities modify the costs of spells being cast.

use super::{StaticAbilityId, StaticAbilityKind, text_utils::join_with_and};
use crate::color::{Color, ColorSet};
use crate::effect::Value;
use crate::filter::{AlternativeCastKind, Comparison};
use crate::mana::ManaCost;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;

fn describe_comparison(cmp: &Comparison) -> String {
    let describe_values = |values: &[i32]| -> String {
        match values.len() {
            0 => String::new(),
            1 => values[0].to_string(),
            2 => format!("{} or {}", values[0], values[1]),
            _ => {
                let head = values[..values.len() - 1]
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{head}, or {}", values[values.len() - 1])
            }
        }
    };
    match cmp {
        Comparison::Equal(v) => v.to_string(),
        Comparison::OneOf(values) => describe_values(values),
        Comparison::NotEqual(v) => format!("not equal to {v}"),
        Comparison::LessThan(v) => format!("less than {v}"),
        Comparison::LessThanOrEqual(v) => format!("{v} or less"),
        Comparison::GreaterThan(v) => format!("greater than {v}"),
        Comparison::GreaterThanOrEqual(v) => format!("{v} or greater"),
        Comparison::EqualExpr(_)
        | Comparison::NotEqualExpr(_)
        | Comparison::LessThanExpr(_)
        | Comparison::LessThanOrEqualExpr(_)
        | Comparison::GreaterThanExpr(_)
        | Comparison::GreaterThanOrEqualExpr(_) => "a dynamic value".to_string(),
    }
}

fn describe_card_type(card_type: CardType) -> &'static str {
    card_type.name()
}

fn describe_colors(colors: ColorSet) -> String {
    let mut words = Vec::new();
    if colors.contains(Color::White) {
        words.push("white".to_string());
    }
    if colors.contains(Color::Blue) {
        words.push("blue".to_string());
    }
    if colors.contains(Color::Black) {
        words.push("black".to_string());
    }
    if colors.contains(Color::Red) {
        words.push("red".to_string());
    }
    if colors.contains(Color::Green) {
        words.push("green".to_string());
    }
    join_with_and(&words)
}

fn describe_player_filter_for_spell_target(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::You => "you".to_string(),
        PlayerFilter::Opponent => "an opponent".to_string(),
        PlayerFilter::Any => "a player".to_string(),
        PlayerFilter::Target(inner) => {
            format!("target {}", describe_player_filter_for_spell_target(inner))
        }
        _ => "a player".to_string(),
    }
}

fn describe_alternative_cast_kind(kind: AlternativeCastKind) -> &'static str {
    match kind {
        AlternativeCastKind::Flashback => "flashback",
        AlternativeCastKind::JumpStart => "jump-start",
        AlternativeCastKind::Escape => "escape",
        AlternativeCastKind::Madness => "madness",
        AlternativeCastKind::Miracle => "miracle",
    }
}

fn scaled_basic_land_type_count(value: &Value) -> Option<(i32, &ObjectFilter)> {
    match value {
        Value::BasicLandTypesAmong(filter) => Some((1, filter)),
        Value::Add(left, right) => {
            let (left_factor, left_filter) = scaled_basic_land_type_count(left)?;
            let (right_factor, right_filter) = scaled_basic_land_type_count(right)?;
            if left_filter != right_filter {
                return None;
            }
            Some((left_factor + right_factor, left_filter))
        }
        _ => None,
    }
}

fn describe_cost_modifier_amount(amount: &Value) -> (String, Option<String>) {
    match amount {
        Value::Fixed(n) => (format!("{{{n}}}"), None),
        Value::X => ("{X}".to_string(), None),
        Value::Count(filter) => (
            "{1}".to_string(),
            Some(format!("for each {}", filter.description())),
        ),
        Value::CountScaled(filter, multiplier) => (
            format!("{{{multiplier}}}"),
            Some(format!("for each {}", filter.description())),
        ),
        Value::BasicLandTypesAmong(filter) => (
            "{1}".to_string(),
            Some(format!(
                "for each basic land type among {}",
                filter.description()
            )),
        ),
        Value::Add(_, _) if scaled_basic_land_type_count(amount).is_some() => {
            let (multiplier, filter) = scaled_basic_land_type_count(amount)
                .expect("checked is_some above for scaled basic land type count");
            (
                format!("{{{multiplier}}}"),
                Some(format!(
                    "for each basic land type among {}",
                    filter.description()
                )),
            )
        }
        Value::TotalPower(filter) => (
            "{X}".to_string(),
            Some(format!(
                "where X is the total power of {}",
                filter.description()
            )),
        ),
        Value::TotalToughness(filter) => (
            "{X}".to_string(),
            Some(format!(
                "where X is the total toughness of {}",
                filter.description()
            )),
        ),
        Value::TotalManaValue(filter) => (
            "{X}".to_string(),
            Some(format!(
                "where X is the total mana value of {}",
                filter.description()
            )),
        ),
        Value::GreatestPower(filter) => (
            "{X}".to_string(),
            Some(format!(
                "where X is the greatest power among {}",
                filter.description()
            )),
        ),
        Value::GreatestManaValue(filter) => (
            "{X}".to_string(),
            Some(format!(
                "where X is the greatest mana value among {}",
                filter.description()
            )),
        ),
        Value::DistinctNames(filter) => (
            "{X}".to_string(),
            Some(format!(
                "where X is the number of differently named {}",
                filter.description()
            )),
        ),
        Value::PartySize(player) => {
            let owner = match player {
                PlayerFilter::You => "your",
                PlayerFilter::Opponent => "an opponent's",
                _ => "a player's",
            };
            (
                "{1}".to_string(),
                Some(format!("for each creature in {owner} party")),
            )
        }
        Value::LifeGainedThisTurn(player) => {
            let phrase = match player {
                PlayerFilter::You => "the amount of life you gained this turn".to_string(),
                PlayerFilter::Opponent => {
                    "the amount of life your opponents gained this turn".to_string()
                }
                _ => format!(
                    "the amount of life {} gained this turn",
                    describe_player_filter_for_spell_target(player)
                ),
            };
            ("{X}".to_string(), Some(format!("where X is {phrase}")))
        }
        Value::NoncombatDamageDealtToPlayersThisTurn(player) => {
            let phrase = match player {
                PlayerFilter::You => {
                    "the total amount of noncombat damage dealt to you this turn".to_string()
                }
                PlayerFilter::Opponent => {
                    "the total amount of noncombat damage dealt to your opponents this turn"
                        .to_string()
                }
                _ => format!(
                    "the total amount of noncombat damage dealt to {} this turn",
                    describe_player_filter_for_spell_target(player)
                ),
            };
            ("{X}".to_string(), Some(format!("where X is {phrase}")))
        }
        Value::CountersOnSource(counter_type) => (
            "{1}".to_string(),
            Some(format!(
                "for each {} counter on this permanent",
                counter_type.description()
            )),
        ),
        Value::CardTypesInGraveyard(player) => {
            let owner = match player {
                PlayerFilter::You => "your",
                PlayerFilter::Opponent => "an opponent's",
                _ => "a player's",
            };
            (
                "{1}".to_string(),
                Some(format!(
                    "for each card type among cards in {owner} graveyard"
                )),
            )
        }
        _ => ("{X}".to_string(), None),
    }
}

fn describe_cost_modifier_mana_cost(cost: &ManaCost) -> String {
    cost.to_oracle()
}

fn describe_cost_modifier_condition_prefix(condition: &crate::ConditionExpr) -> String {
    match condition {
        crate::ConditionExpr::YourTurn => "During your turn".to_string(),
        crate::ConditionExpr::Not(inner)
            if matches!(inner.as_ref(), crate::ConditionExpr::YourTurn) =>
        {
            "During turns other than yours".to_string()
        }
        crate::ConditionExpr::SourceIsTapped => "As long as this permanent is tapped".to_string(),
        crate::ConditionExpr::SourceIsUntapped => {
            "As long as this permanent is untapped".to_string()
        }
        crate::ConditionExpr::SourceIsEquipped => {
            "As long as this permanent is equipped".to_string()
        }
        crate::ConditionExpr::SourceIsEnchanted => {
            "As long as this permanent is enchanted".to_string()
        }
        other => format!("As long as {other:?}"),
    }
}

fn describe_cost_modifier_with_condition(
    body: String,
    condition: &Option<crate::ConditionExpr>,
) -> String {
    if let Some(condition) = condition {
        format!(
            "{}, {}",
            describe_cost_modifier_condition_prefix(condition),
            body
        )
    } else {
        body
    }
}

fn cost_modifier_condition_is_active(
    condition: &Option<crate::ConditionExpr>,
    game: &crate::game_state::GameState,
    source: crate::ids::ObjectId,
) -> bool {
    let Some(condition) = condition else {
        return true;
    };
    let Some(source_obj) = game.object(source) else {
        return false;
    };
    let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
        controller: source_obj.controller,
        source,
        defending_player: None,
        attacking_player: None,
        filter_source: Some(source),
        triggering_event: None,
        trigger_identity: None,
        ability_index: None,
        options: Default::default(),
    };
    crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx)
}

fn describe_spell_filter(filter: &ObjectFilter) -> String {
    let mut qualifiers = Vec::<String>::new();
    if let Some(colors) = filter.colors {
        let color_text = describe_colors(colors);
        if !color_text.is_empty() {
            qualifiers.push(color_text);
        }
    }
    for card_type in &filter.excluded_card_types {
        qualifiers.push(format!("non{}", describe_card_type(*card_type)));
    }
    if !filter.subtypes.is_empty() {
        let subtypes = filter
            .subtypes
            .iter()
            .map(|subtype| format!("{subtype:?}"))
            .collect::<Vec<_>>();
        qualifiers.push(join_with_and(&subtypes));
    }
    if !filter.card_types.is_empty() {
        let types = filter
            .card_types
            .iter()
            .map(|card_type| describe_card_type(*card_type).to_string())
            .collect::<Vec<_>>();
        qualifiers.push(join_with_and(&types));
    }

    let mut description = if qualifiers.is_empty() {
        "spells".to_string()
    } else {
        format!("{} spells", qualifiers.join(" "))
    };
    match filter.cast_by.as_ref() {
        Some(PlayerFilter::You) => description.push_str(" you cast"),
        Some(PlayerFilter::Opponent) => description.push_str(" your opponents cast"),
        _ => {}
    }
    if let Some(power) = &filter.power {
        description.push_str(" with power ");
        description.push_str(&describe_comparison(power));
    }
    if let Some(toughness) = &filter.toughness {
        description.push_str(" with toughness ");
        description.push_str(&describe_comparison(toughness));
    }
    if let Some(mana_value) = &filter.mana_value {
        description.push_str(" with mana value ");
        description.push_str(&describe_comparison(mana_value));
    }
    if let Some(player_filter) = &filter.targets_player {
        description.push_str(" that target ");
        description.push_str(&describe_player_filter_for_spell_target(player_filter));
    }
    if let Some(object_filter) = &filter.targets_object {
        description.push_str(" that target ");
        description.push_str(&object_filter.description());
    }
    if let Some(kind) = filter.alternative_cast {
        description.push_str(" with ");
        description.push_str(describe_alternative_cast_kind(kind));
    }

    description
}

fn describe_flashback_cost_subject(filter: &ObjectFilter) -> Option<&'static str> {
    if filter.alternative_cast != Some(AlternativeCastKind::Flashback)
        || !filter.card_types.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || filter.colors.is_some()
        || filter.power.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
    {
        return None;
    }
    match filter.cast_by.as_ref() {
        Some(PlayerFilter::You) => Some("Flashback costs you pay"),
        Some(PlayerFilter::Opponent) => Some("Flashback costs your opponents pay"),
        None | Some(PlayerFilter::Any) => Some("Flashback costs"),
        _ => None,
    }
}

/// Affinity for artifacts - This spell costs {1} less to cast for each artifact you control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AffinityForArtifacts;

impl StaticAbilityKind for AffinityForArtifacts {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AffinityForArtifacts
    }

    fn display(&self) -> String {
        "Affinity for artifacts".to_string()
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn has_affinity(&self) -> bool {
        true
    }
}

/// Delve - Each card you exile from your graveyard while casting this spell pays for {1}.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Delve;

impl StaticAbilityKind for Delve {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Delve
    }

    fn display(&self) -> String {
        "Delve".to_string()
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn has_delve(&self) -> bool {
        true
    }
}

/// Convoke - Your creatures can help cast this spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Convoke;

impl StaticAbilityKind for Convoke {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Convoke
    }

    fn display(&self) -> String {
        "Convoke".to_string()
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn has_convoke(&self) -> bool {
        true
    }
}

/// Improvise - Your artifacts can help cast this spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Improvise;

impl StaticAbilityKind for Improvise {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Improvise
    }

    fn display(&self) -> String {
        "Improvise".to_string()
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn has_improvise(&self) -> bool {
        true
    }
}

/// Cost reduction: "Spells cost {N} less to cast"
#[derive(Debug, Clone, PartialEq)]
pub struct CostReduction {
    pub filter: ObjectFilter,
    pub reduction: Value,
    pub condition: Option<crate::ConditionExpr>,
}

impl CostReduction {
    pub fn new(filter: ObjectFilter, reduction: Value) -> Self {
        Self {
            filter,
            reduction,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl StaticAbilityKind for CostReduction {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CostReduction
    }

    fn display(&self) -> String {
        let (amount_text, tail) = describe_cost_modifier_amount(&self.reduction);
        if let Some(subject) = describe_flashback_cost_subject(&self.filter) {
            let mut line = format!("{subject} cost {amount_text} less");
            if let Some(tail) = tail {
                line.push(' ');
                line.push_str(&tail);
            }
            return describe_cost_modifier_with_condition(line, &self.condition);
        }
        let mut line = format!(
            "{} cost {} less to cast",
            describe_spell_filter(&self.filter),
            amount_text
        );
        if let Some(tail) = tail {
            line.push(' ');
            line.push_str(&tail);
        }
        describe_cost_modifier_with_condition(line, &self.condition)
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn cost_reduction(&self) -> Option<&CostReduction> {
        Some(self)
    }

    fn is_active(&self, game: &crate::game_state::GameState, source: crate::ids::ObjectId) -> bool {
        cost_modifier_condition_is_active(&self.condition, game, source)
    }
}

/// Activated-ability cost reduction:
/// "Activated abilities of <objects> cost {N} less to activate."
#[derive(Debug, Clone, PartialEq)]
pub struct ActivatedAbilityCostReduction {
    pub filter: ObjectFilter,
    pub reduction: u32,
    pub minimum_total_mana: Option<u32>,
}

impl ActivatedAbilityCostReduction {
    pub fn new(filter: ObjectFilter, reduction: u32) -> Self {
        Self {
            filter,
            reduction,
            minimum_total_mana: None,
        }
    }

    pub fn with_minimum_total_mana(mut self, minimum_total_mana: u32) -> Self {
        self.minimum_total_mana = Some(minimum_total_mana);
        self
    }
}

impl StaticAbilityKind for ActivatedAbilityCostReduction {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ActivatedAbilityCostReduction
    }

    fn display(&self) -> String {
        let mut line = format!(
            "Activated abilities of {} cost {{{}}} less to activate",
            self.filter.description(),
            self.reduction
        );
        if let Some(minimum) = self.minimum_total_mana
            && minimum == 1
        {
            line.push_str(". This effect can't reduce the mana in that cost to less than one mana");
        }
        line
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn activated_ability_cost_reduction(&self) -> Option<&ActivatedAbilityCostReduction> {
        Some(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThisSpellCostCondition {
    Always,
    YourTurn,
    NotYourTurn,
    YouLifeTotalOrLess(i32),
    OpponentHasNoCardsInHand,
    OpponentControlsLandsOrMore(u32),
    OpponentControlsAtLeastNMoreCreaturesThanYou(u32),
    TotalCreatureCardsInAllGraveyardsOrMore(u32),
    OpponentCastSpellsThisTurnOrMore(u32),
    OpponentDrewCardsThisTurnOrMore(u32),
    ConditionExpr {
        condition: crate::ConditionExpr,
        display: String,
    },
    TargetsPlayer(PlayerFilter),
    TargetsObject(ObjectFilter),
    YouCastSpellsThisTurnOrMore {
        count: u32,
        card_types: Vec<CardType>,
    },
    YouGainedLifeThisTurnOrMore(u32),
    OpponentHasPoisonCountersOrMore(u32),
    OpponentHasCardsInGraveyardOrMore(u32),
    DistinctCardTypesInYourGraveyardOrMore(u32),
    LifeTotalLessThanStarting,
    IsNight,
    YouSacrificedArtifactThisTurn,
    YouCommittedCrimeThisTurn,
    CreatureLeftBattlefieldUnderYourControlThisTurn,
    YouHaveCardsInYourGraveyardOrMore(u32),
    YouHaveCardsOfTypesInYourGraveyardOrMore {
        count: u32,
        card_types: Vec<CardType>,
    },
    OnlyCreatureCardsInHandNamed(String),
    NoCardsInHandMatching {
        filter: ObjectFilter,
        display: String,
    },
    CardInYourGraveyardMatching {
        filter: ObjectFilter,
        display: String,
    },
    NotStartingPlayer,
    CreatureCardPutIntoYourGraveyardThisTurn,
    CreatureIsAttackingYou,
}

pub fn describe_this_spell_cost_condition(condition: &ThisSpellCostCondition) -> Option<String> {
    match condition {
        ThisSpellCostCondition::Always => None,
        ThisSpellCostCondition::YourTurn => Some("it's your turn".to_string()),
        ThisSpellCostCondition::NotYourTurn => Some("it isn't your turn".to_string()),
        ThisSpellCostCondition::YouLifeTotalOrLess(n) => Some(format!("you have {n} or less life")),
        ThisSpellCostCondition::OpponentHasNoCardsInHand => {
            Some("an opponent has no cards in hand".to_string())
        }
        ThisSpellCostCondition::OpponentControlsLandsOrMore(n) => {
            Some(format!("an opponent controls {n} or more lands"))
        }
        ThisSpellCostCondition::OpponentControlsAtLeastNMoreCreaturesThanYou(n) => Some(format!(
            "an opponent controls at least {n} more creatures than you"
        )),
        ThisSpellCostCondition::TotalCreatureCardsInAllGraveyardsOrMore(n) => Some(format!(
            "there are {n} or more creature cards total in all graveyards"
        )),
        ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(n) => {
            Some(format!("an opponent cast {n} or more spells this turn"))
        }
        ThisSpellCostCondition::OpponentDrewCardsThisTurnOrMore(n) => {
            Some(format!("an opponent has drawn {n} or more cards this turn"))
        }
        ThisSpellCostCondition::ConditionExpr { display, .. } => Some(display.clone()),
        ThisSpellCostCondition::TargetsPlayer(player) => Some(format!(
            "it targets {}",
            describe_player_filter_for_spell_target(player)
        )),
        ThisSpellCostCondition::TargetsObject(filter) => {
            Some(format!("it targets {}", filter.description()))
        }
        ThisSpellCostCondition::YouCastSpellsThisTurnOrMore { count, card_types } => {
            let amount = if *count <= 1 {
                "another".to_string()
            } else {
                format!("{count} or more")
            };
            let type_prefix = if card_types.is_empty() {
                String::new()
            } else {
                let names = card_types
                    .iter()
                    .map(|card_type| describe_card_type(*card_type).to_string())
                    .collect::<Vec<_>>();
                format!("{} ", join_with_and(&names))
            };
            Some(format!(
                "you've cast {amount} {type_prefix}spell{} this turn",
                if *count <= 1 { "" } else { "s" }
            ))
        }
        ThisSpellCostCondition::YouGainedLifeThisTurnOrMore(n) => Some(if *n <= 1 {
            "you gained life this turn".to_string()
        } else {
            format!("you've gained {n} or more life this turn")
        }),
        ThisSpellCostCondition::OpponentHasPoisonCountersOrMore(n) => {
            Some(format!("an opponent has {n} or more poison counters"))
        }
        ThisSpellCostCondition::OpponentHasCardsInGraveyardOrMore(n) => Some(format!(
            "an opponent has {n} or more cards in their graveyard"
        )),
        ThisSpellCostCondition::DistinctCardTypesInYourGraveyardOrMore(n) => Some(format!(
            "there are {n} or more card types among cards in your graveyard"
        )),
        ThisSpellCostCondition::LifeTotalLessThanStarting => {
            Some("your life total is less than your starting life total".to_string())
        }
        ThisSpellCostCondition::IsNight => Some("it's night".to_string()),
        ThisSpellCostCondition::YouSacrificedArtifactThisTurn => {
            Some("you've sacrificed an artifact this turn".to_string())
        }
        ThisSpellCostCondition::YouCommittedCrimeThisTurn => {
            Some("you've committed a crime this turn".to_string())
        }
        ThisSpellCostCondition::CreatureLeftBattlefieldUnderYourControlThisTurn => {
            Some("a creature left the battlefield under your control this turn".to_string())
        }
        ThisSpellCostCondition::YouHaveCardsInYourGraveyardOrMore(n) => {
            Some(format!("you have {n} or more cards in your graveyard"))
        }
        ThisSpellCostCondition::YouHaveCardsOfTypesInYourGraveyardOrMore { count, card_types } => {
            let type_text = card_types
                .iter()
                .map(|card_type| describe_card_type(*card_type).to_string())
                .collect::<Vec<_>>();
            Some(format!(
                "you have {count} or more {} cards in your graveyard",
                join_with_and(&type_text)
            ))
        }
        ThisSpellCostCondition::OnlyCreatureCardsInHandNamed(name) => Some(format!(
            "the only other creature cards in your hand are named {name}"
        )),
        ThisSpellCostCondition::NoCardsInHandMatching { display, .. } => Some(display.clone()),
        ThisSpellCostCondition::CardInYourGraveyardMatching { display, .. } => {
            Some(display.clone())
        }
        ThisSpellCostCondition::NotStartingPlayer => {
            Some("you weren't the starting player".to_string())
        }
        ThisSpellCostCondition::CreatureCardPutIntoYourGraveyardThisTurn => {
            Some("a creature card was put into your graveyard from anywhere this turn".to_string())
        }
        ThisSpellCostCondition::CreatureIsAttackingYou => {
            Some("a creature is attacking you".to_string())
        }
    }
}

fn this_spell_condition_eval_ctx<'a>(
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
) -> crate::condition_eval::ExternalEvaluationContext<'a> {
    crate::condition_eval::ExternalEvaluationContext {
        controller,
        source,
        defending_player: None,
        attacking_player: None,
        filter_source: Some(source),
        triggering_event: None,
        trigger_identity: None,
        ability_index: None,
        options: Default::default(),
    }
}

fn chosen_targets_match(
    game: &crate::game_state::GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    chosen_targets: &[crate::game_state::Target],
    player_filter: Option<&PlayerFilter>,
    object_filter: Option<&ObjectFilter>,
) -> bool {
    if chosen_targets.is_empty() {
        return false;
    }
    let opponents = game
        .turn_order
        .iter()
        .copied()
        .filter(|player_id| *player_id != controller)
        .collect::<Vec<_>>();
    let filter_ctx = crate::filter::FilterContext::new(controller)
        .with_source(source)
        .with_active_player(game.turn.active_player)
        .with_opponents(opponents);
    let matches_player = player_filter.is_none_or(|filter| {
        chosen_targets.iter().any(|target| match target {
            crate::game_state::Target::Player(player_id) => {
                filter.matches_player(*player_id, &filter_ctx)
            }
            _ => false,
        })
    });
    let matches_object = object_filter.is_none_or(|filter| {
        chosen_targets.iter().any(|target| match target {
            crate::game_state::Target::Object(object_id) => game
                .object(*object_id)
                .is_some_and(|object| filter.matches(object, &filter_ctx, game)),
            _ => false,
        })
    });
    matches_player && matches_object
}

fn normalize_name_for_match(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn names_match(lhs: &str, rhs: &str) -> bool {
    lhs.eq_ignore_ascii_case(rhs) || normalize_name_for_match(lhs) == normalize_name_for_match(rhs)
}

pub fn this_spell_cost_condition_is_active_for_cast(
    game: &crate::game_state::GameState,
    source: crate::ids::ObjectId,
    condition: &ThisSpellCostCondition,
    chosen_targets: &[crate::game_state::Target],
) -> bool {
    if matches!(condition, ThisSpellCostCondition::Always) {
        return true;
    }
    let Some(source_obj) = game.object(source) else {
        return false;
    };
    let controller = source_obj.controller;

    match condition {
        ThisSpellCostCondition::Always => true,
        ThisSpellCostCondition::YourTurn => game.turn.active_player == controller,
        ThisSpellCostCondition::NotYourTurn => game.turn.active_player != controller,
        ThisSpellCostCondition::YouLifeTotalOrLess(n) => game
            .player(controller)
            .is_some_and(|player| player.life <= *n),
        ThisSpellCostCondition::OpponentHasNoCardsInHand => game
            .players
            .iter()
            .filter(|player| player.is_in_game() && player.id != controller)
            .any(|player| player.hand.is_empty()),
        ThisSpellCostCondition::OpponentControlsLandsOrMore(n) => game
            .players
            .iter()
            .filter(|player| player.is_in_game() && player.id != controller)
            .any(|player| {
                let lands = game
                    .battlefield
                    .iter()
                    .filter_map(|&object_id| game.object(object_id))
                    .filter(|object| {
                        object.controller == player.id
                            && game.object_has_card_type(object.id, CardType::Land)
                    })
                    .count();
                lands >= *n as usize
            }),
        ThisSpellCostCondition::OpponentControlsAtLeastNMoreCreaturesThanYou(n) => {
            let your_creatures = game
                .battlefield
                .iter()
                .filter_map(|&object_id| game.object(object_id))
                .filter(|object| {
                    object.controller == controller
                        && game.object_has_card_type(object.id, CardType::Creature)
                })
                .count();
            game.players
                .iter()
                .filter(|player| player.is_in_game() && player.id != controller)
                .any(|player| {
                    let opponent_creatures = game
                        .battlefield
                        .iter()
                        .filter_map(|&object_id| game.object(object_id))
                        .filter(|object| {
                            object.controller == player.id
                                && game.object_has_card_type(object.id, CardType::Creature)
                        })
                        .count();
                    opponent_creatures >= your_creatures.saturating_add(*n as usize)
                })
        }
        ThisSpellCostCondition::TotalCreatureCardsInAllGraveyardsOrMore(n) => {
            let total_creatures = game
                .players
                .iter()
                .filter(|player| player.is_in_game())
                .flat_map(|player| player.graveyard.iter().copied())
                .filter(|card_id| game.object_has_card_type(*card_id, CardType::Creature))
                .count();
            total_creatures >= *n as usize
        }
        ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(n) => game
            .players
            .iter()
            .filter(|player| player.is_in_game() && player.id != controller)
            .any(|player| {
                game.spells_cast_this_turn
                    .get(&player.id)
                    .copied()
                    .unwrap_or(0)
                    >= *n
            }),
        ThisSpellCostCondition::OpponentDrewCardsThisTurnOrMore(n) => game
            .players
            .iter()
            .filter(|player| player.is_in_game() && player.id != controller)
            .any(|player| {
                game.cards_drawn_this_turn
                    .get(&player.id)
                    .copied()
                    .unwrap_or(0)
                    >= *n
            }),
        ThisSpellCostCondition::ConditionExpr {
            condition: expr, ..
        } => {
            let eval_ctx = this_spell_condition_eval_ctx(source, controller);
            crate::condition_eval::evaluate_condition_external(game, expr, &eval_ctx)
        }
        ThisSpellCostCondition::TargetsPlayer(filter) => {
            chosen_targets_match(game, source, controller, chosen_targets, Some(filter), None)
        }
        ThisSpellCostCondition::TargetsObject(filter) => {
            chosen_targets_match(game, source, controller, chosen_targets, None, Some(filter))
        }
        ThisSpellCostCondition::YouCastSpellsThisTurnOrMore { count, card_types } => {
            if card_types.is_empty() {
                game.spells_cast_this_turn
                    .get(&controller)
                    .copied()
                    .unwrap_or(0)
                    >= *count
            } else {
                let matching = game
                    .spells_cast_this_turn_snapshots
                    .iter()
                    .filter(|snapshot| snapshot.controller == controller)
                    .filter(|snapshot| {
                        card_types
                            .iter()
                            .any(|card_type| snapshot.card_types.contains(card_type))
                    })
                    .count();
                matching >= *count as usize
            }
        }
        ThisSpellCostCondition::YouGainedLifeThisTurnOrMore(n) => {
            game.life_gained_this_turn
                .get(&controller)
                .copied()
                .unwrap_or(0)
                >= *n
        }
        ThisSpellCostCondition::OpponentHasPoisonCountersOrMore(n) => game
            .players
            .iter()
            .filter(|player| player.is_in_game() && player.id != controller)
            .any(|player| player.poison_counters >= *n),
        ThisSpellCostCondition::OpponentHasCardsInGraveyardOrMore(n) => game
            .players
            .iter()
            .filter(|player| player.is_in_game() && player.id != controller)
            .any(|player| player.graveyard.len() >= *n as usize),
        ThisSpellCostCondition::LifeTotalLessThanStarting => game
            .player(controller)
            .is_some_and(|player| player.life < player.starting_life),
        ThisSpellCostCondition::IsNight => game.is_night,
        ThisSpellCostCondition::YouSacrificedArtifactThisTurn => {
            game.artifacts_sacrificed_this_turn
                .get(&controller)
                .copied()
                .unwrap_or(0)
                > 0
        }
        ThisSpellCostCondition::YouCommittedCrimeThisTurn => {
            game.crimes_committed_this_turn
                .get(&controller)
                .copied()
                .unwrap_or(0)
                > 0
        }
        ThisSpellCostCondition::CreatureLeftBattlefieldUnderYourControlThisTurn => {
            game.creatures_left_battlefield_under_controller_this_turn
                .get(&controller)
                .copied()
                .unwrap_or(0)
                > 0
        }
        ThisSpellCostCondition::DistinctCardTypesInYourGraveyardOrMore(n) => {
            let Some(player) = game.player(controller) else {
                return false;
            };
            let mut card_types = std::collections::HashSet::<CardType>::new();
            for &card_id in &player.graveyard {
                if let Some(card) = game.object(card_id) {
                    for card_type in &card.card_types {
                        card_types.insert(*card_type);
                    }
                }
            }
            card_types.len() >= *n as usize
        }
        ThisSpellCostCondition::YouHaveCardsInYourGraveyardOrMore(n) => game
            .player(controller)
            .is_some_and(|player| player.graveyard.len() >= *n as usize),
        ThisSpellCostCondition::YouHaveCardsOfTypesInYourGraveyardOrMore { count, card_types } => {
            let Some(player) = game.player(controller) else {
                return false;
            };
            let matching = player
                .graveyard
                .iter()
                .filter(|card_id| {
                    game.object(**card_id).is_some_and(|object| {
                        card_types
                            .iter()
                            .any(|card_type| object.card_types.contains(card_type))
                    })
                })
                .count();
            matching >= *count as usize
        }
        ThisSpellCostCondition::OnlyCreatureCardsInHandNamed(name) => {
            let Some(player) = game.player(controller) else {
                return false;
            };
            player
                .hand
                .iter()
                .filter_map(|card_id| game.object(*card_id))
                .filter(|object| object.card_types.contains(&CardType::Creature))
                .all(|object| names_match(&object.name, name))
        }
        ThisSpellCostCondition::NoCardsInHandMatching { filter, .. } => {
            let Some(player) = game.player(controller) else {
                return false;
            };
            let opponents = game
                .turn_order
                .iter()
                .copied()
                .filter(|player_id| *player_id != controller)
                .collect::<Vec<_>>();
            let filter_ctx = crate::filter::FilterContext::new(controller)
                .with_source(source)
                .with_active_player(game.turn.active_player)
                .with_opponents(opponents);
            !player.hand.iter().any(|card_id| {
                game.object(*card_id)
                    .is_some_and(|object| filter.matches(object, &filter_ctx, game))
            })
        }
        ThisSpellCostCondition::CardInYourGraveyardMatching { filter, .. } => {
            let Some(player) = game.player(controller) else {
                return false;
            };
            let opponents = game
                .turn_order
                .iter()
                .copied()
                .filter(|player_id| *player_id != controller)
                .collect::<Vec<_>>();
            let filter_ctx = crate::filter::FilterContext::new(controller)
                .with_source(source)
                .with_active_player(game.turn.active_player)
                .with_opponents(opponents);
            player.graveyard.iter().any(|card_id| {
                game.object(*card_id)
                    .is_some_and(|object| filter.matches(object, &filter_ctx, game))
            })
        }
        ThisSpellCostCondition::NotStartingPlayer => game
            .turn_order
            .first()
            .is_some_and(|starting_player| *starting_player != controller),
        ThisSpellCostCondition::CreatureCardPutIntoYourGraveyardThisTurn => {
            let Some(player) = game.player(controller) else {
                return false;
            };
            player.graveyard.iter().any(|card_id| {
                game.object(*card_id).is_some_and(|object| {
                    game.object_has_card_type(object.id, CardType::Creature)
                        && game
                            .objects_put_into_graveyard_this_turn
                            .contains(&object.stable_id)
                })
            })
        }
        ThisSpellCostCondition::CreatureIsAttackingYou => {
            game.combat.as_ref().is_some_and(|combat| {
                combat.attackers.iter().any(|attacker| {
                    matches!(
                        attacker.target,
                        crate::combat_state::AttackTarget::Player(player) if player == controller
                    )
                })
            })
        }
    }
}

/// Cost reduction: "If <condition>, this spell costs {N} less to cast."
#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellCostReduction {
    pub reduction: Value,
    pub condition: ThisSpellCostCondition,
}

impl ThisSpellCostReduction {
    pub fn new(reduction: Value, condition: ThisSpellCostCondition) -> Self {
        Self {
            reduction,
            condition,
        }
    }
}

impl StaticAbilityKind for ThisSpellCostReduction {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ThisSpellCostReduction
    }

    fn display(&self) -> String {
        let (amount_text, tail) = describe_cost_modifier_amount(&self.reduction);
        let mut line = format!("This spell costs {amount_text} less to cast");
        if let Some(tail) = tail {
            line.push(' ');
            line.push_str(&tail);
        }
        let Some(condition_text) = describe_this_spell_cost_condition(&self.condition) else {
            return line;
        };

        format!("If {condition_text}, {line}")
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn this_spell_cost_reduction(&self) -> Option<&ThisSpellCostReduction> {
        Some(self)
    }

    fn is_active(&self, game: &crate::game_state::GameState, source: crate::ids::ObjectId) -> bool {
        this_spell_cost_condition_is_active_for_cast(game, source, &self.condition, &[])
    }
}

/// Cost reduction with mana symbols for this spell:
/// "If <condition>, this spell costs {U}{U} less to cast."
#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellCostReductionManaCost {
    pub reduction: ManaCost,
    pub condition: ThisSpellCostCondition,
}

impl ThisSpellCostReductionManaCost {
    pub fn new(reduction: ManaCost, condition: ThisSpellCostCondition) -> Self {
        Self {
            reduction,
            condition,
        }
    }
}

impl StaticAbilityKind for ThisSpellCostReductionManaCost {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ThisSpellCostReductionManaCost
    }

    fn display(&self) -> String {
        let amount_text = describe_cost_modifier_mana_cost(&self.reduction);
        let Some(condition_text) = describe_this_spell_cost_condition(&self.condition) else {
            return format!("This spell costs {amount_text} less to cast");
        };

        format!("If {condition_text}, this spell costs {amount_text} less to cast")
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn this_spell_cost_reduction_mana_cost(&self) -> Option<&ThisSpellCostReductionManaCost> {
        Some(self)
    }

    fn is_active(&self, game: &crate::game_state::GameState, source: crate::ids::ObjectId) -> bool {
        this_spell_cost_condition_is_active_for_cast(game, source, &self.condition, &[])
    }
}

/// Cost increase: "Spells cost {N} more to cast"
#[derive(Debug, Clone, PartialEq)]
pub struct CostIncrease {
    pub filter: ObjectFilter,
    pub increase: Value,
    pub condition: Option<crate::ConditionExpr>,
}

impl CostIncrease {
    pub fn new(filter: ObjectFilter, increase: Value) -> Self {
        Self {
            filter,
            increase,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl StaticAbilityKind for CostIncrease {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CostIncrease
    }

    fn display(&self) -> String {
        let (amount_text, tail) = describe_cost_modifier_amount(&self.increase);
        if let Some(subject) = describe_flashback_cost_subject(&self.filter) {
            let mut line = format!("{subject} cost {amount_text} more");
            if let Some(tail) = tail {
                line.push(' ');
                line.push_str(&tail);
            }
            return describe_cost_modifier_with_condition(line, &self.condition);
        }
        let mut line = format!(
            "{} cost {} more to cast",
            describe_spell_filter(&self.filter),
            amount_text
        );
        if let Some(tail) = tail {
            line.push(' ');
            line.push_str(&tail);
        }
        describe_cost_modifier_with_condition(line, &self.condition)
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn cost_increase(&self) -> Option<&CostIncrease> {
        Some(self)
    }

    fn is_active(&self, game: &crate::game_state::GameState, source: crate::ids::ObjectId) -> bool {
        cost_modifier_condition_is_active(&self.condition, game, source)
    }
}

/// Mana-symbol cost reduction: "Spells cost {B} less to cast"
#[derive(Debug, Clone, PartialEq)]
pub struct CostReductionManaCost {
    pub filter: ObjectFilter,
    pub reduction: ManaCost,
    pub condition: Option<crate::ConditionExpr>,
}

impl CostReductionManaCost {
    pub fn new(filter: ObjectFilter, reduction: ManaCost) -> Self {
        Self {
            filter,
            reduction,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl StaticAbilityKind for CostReductionManaCost {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CostReductionManaCost
    }

    fn display(&self) -> String {
        let line = format!(
            "{} cost {} less to cast",
            describe_spell_filter(&self.filter),
            describe_cost_modifier_mana_cost(&self.reduction)
        );
        describe_cost_modifier_with_condition(line, &self.condition)
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn cost_reduction_mana_cost(&self) -> Option<&CostReductionManaCost> {
        Some(self)
    }

    fn is_active(&self, game: &crate::game_state::GameState, source: crate::ids::ObjectId) -> bool {
        cost_modifier_condition_is_active(&self.condition, game, source)
    }
}

/// Mana-symbol cost increase: "Spells cost {B} more to cast"
#[derive(Debug, Clone, PartialEq)]
pub struct CostIncreaseManaCost {
    pub filter: ObjectFilter,
    pub increase: ManaCost,
    pub condition: Option<crate::ConditionExpr>,
}

impl CostIncreaseManaCost {
    pub fn new(filter: ObjectFilter, increase: ManaCost) -> Self {
        Self {
            filter,
            increase,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl StaticAbilityKind for CostIncreaseManaCost {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CostIncreaseManaCost
    }

    fn display(&self) -> String {
        let line = format!(
            "{} cost {} more to cast",
            describe_spell_filter(&self.filter),
            describe_cost_modifier_mana_cost(&self.increase)
        );
        describe_cost_modifier_with_condition(line, &self.condition)
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn cost_increase_mana_cost(&self) -> Option<&CostIncreaseManaCost> {
        Some(self)
    }

    fn is_active(&self, game: &crate::game_state::GameState, source: crate::ids::ObjectId) -> bool {
        cost_modifier_condition_is_active(&self.condition, game, source)
    }
}

/// Cost increase per additional target:
/// "This spell costs {N} more to cast for each target beyond the first."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostIncreasePerAdditionalTarget {
    pub amount: u32,
}

impl CostIncreasePerAdditionalTarget {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl StaticAbilityKind for CostIncreasePerAdditionalTarget {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CostIncreasePerAdditionalTarget
    }

    fn display(&self) -> String {
        format!(
            "This spell costs {{{}}} more to cast for each target beyond the first",
            self.amount
        )
    }

    fn modifies_costs(&self) -> bool {
        true
    }

    fn cost_increase_per_additional_target(&self) -> Option<u32> {
        Some(self.amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Value;
    use crate::static_abilities::ThisSpellCostCondition::Always;
    use crate::target::PlayerFilter;

    #[test]
    fn test_affinity() {
        let affinity = AffinityForArtifacts;
        assert_eq!(affinity.id(), StaticAbilityId::AffinityForArtifacts);
        assert!(affinity.modifies_costs());
    }

    #[test]
    fn test_delve() {
        let delve = Delve;
        assert_eq!(delve.id(), StaticAbilityId::Delve);
        assert!(delve.modifies_costs());
    }

    #[test]
    fn test_convoke() {
        let convoke = Convoke;
        assert_eq!(convoke.id(), StaticAbilityId::Convoke);
        assert!(convoke.modifies_costs());
    }

    #[test]
    fn this_spell_cost_reduction_display_keeps_dynamic_tail() {
        let reduction =
            ThisSpellCostReduction::new(Value::CardTypesInGraveyard(PlayerFilter::You), Always);

        assert_eq!(
            reduction.display(),
            "This spell costs {1} less to cast for each card type among cards in your graveyard"
        );
    }
}
