//! Typed authored predicates used by the layer-3 text owner.
//!
//! Callers must establish that these predicates belong to authored text. The
//! same runtime models can also be synthesized by keyword rules; this visitor
//! does not turn those inferred rules into word occurrences. Every transform
//! works on a clone, so an unsupported nested domain cannot publish half an edit.

use super::text_changes::TextChangeDomainError;
use ironsmith_core::{
    AnthemCountExpression, AttachmentConditionHost, ChoiceAggregateConstraint, ChooseSpec,
    CastEventQuantity, Condition, EventValueSpec, FilterComparison, ObjectFilter,
    PlayerFilter, PriorEffectMetricQuery, TextChange, TextWord, TurnHistoryCondition, TurnHistoryCount, Value,
};

type RewriteResult<T> = Result<T, TextChangeDomainError>;

/// Preserve binding, identity, count and presentation wrappers while visiting
/// the actual authored selection predicates. In particular, a tagged reference
/// is never rebound to a newly rendered name.
pub(crate) fn rewrite_choose_spec_words(spec: &ChooseSpec, change: TextChange) -> RewriteResult<ChooseSpec> {
    let mut rewritten = spec.clone();
    match &mut rewritten {
        ChooseSpec::SurfaceHinted { spec, .. } | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _) => **spec = rewrite_choose_spec_words(spec, change)?,
        ChooseSpec::WithCountValue(spec, _, value) => {
            **spec = rewrite_choose_spec_words(spec, change)?;
            *value = rewrite_value_words(value, change)?;
        }
        ChooseSpec::Player(player) | ChooseSpec::EachPlayer(player)
        | ChooseSpec::PlayerOrPlaneswalker(player) => *player = rewrite_player_filter_words(player, change)?,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => *filter = rewrite_filter_words(filter, change)?,
        ChooseSpec::ObjectOrPlayer(filter, player) => {
            *filter = rewrite_filter_words(filter, change)?;
            *player = rewrite_player_filter_words(player, change)?;
        }
        ChooseSpec::SpecificObject(_) | ChooseSpec::SpecificPlayer(_)
        | ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget | ChooseSpec::AttackedPlayerOrPlaneswalker
        | ChooseSpec::Source | ChooseSpec::SourceController | ChooseSpec::SourceOwner
        | ChooseSpec::Tagged(_) | ChooseSpec::Iterated => {}
    }
    Ok(rewritten)
}

pub(crate) fn rewrite_player_filter_words(player: &PlayerFilter, change: TextChange) -> RewriteResult<PlayerFilter> {
    let mut rewritten = player.clone();
    match &mut rewritten {
        PlayerFilter::WasDealtDamageBySourceThisGame { base }
        | PlayerFilter::LostLifeThisTurn { base }
        | PlayerFilter::CardsInHandAtLeastMoreThanYou { base, .. }
        | PlayerFilter::HasMoreLifeThanYou { base }
        | PlayerFilter::MaxSpeed { base, .. }
        | PlayerFilter::OpponentOf(base) | PlayerFilter::PlayerToLeftOf(base) | PlayerFilter::Target(base)
        | PlayerFilter::AliasedTarget(base) => **base = rewrite_player_filter_words(base, change)?,
        PlayerFilter::WasDealtCombatDamageBySourcesThisGame { base, sources }
        | PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { base, sources, .. } => {
            **base = rewrite_player_filter_words(base, change)?;
            **sources = rewrite_filter_words(sources, change)?;
        }
        PlayerFilter::OpponentWithMoreControlledObjectsThan { player, filter, .. } => {
            **player = rewrite_player_filter_words(player, change)?;
            **filter = rewrite_filter_words(filter, change)?;
        }
        PlayerFilter::ControlsMost { filter } | PlayerFilter::ControlsFewestTied { filter } => {
            **filter = rewrite_filter_words(filter, change)?;
        }
        PlayerFilter::Excluding { base, excluded } => {
            **base = rewrite_player_filter_words(base, change)?;
            **excluded = rewrite_player_filter_words(excluded, change)?;
        }
        PlayerFilter::Any | PlayerFilter::You | PlayerFilter::NotYou | PlayerFilter::Opponent
        | PlayerFilter::Teammate | PlayerFilter::PlayerToYourLeft | PlayerFilter::PlayerToYourRight
        | PlayerFilter::Active | PlayerFilter::Defending | PlayerFilter::Attacking
        | PlayerFilter::DamagedPlayer | PlayerFilter::EffectController | PlayerFilter::Specific(_)
        | PlayerFilter::MostLifeTied | PlayerFilter::LowestLifeTied | PlayerFilter::MostCardsInHand
        | PlayerFilter::CastCardTypeThisTurn(_) | PlayerFilter::AttackedBySourceThisTurn
        | PlayerFilter::ChosenPlayer | PlayerFilter::TaggedPlayer(_) | PlayerFilter::IteratedPlayer
        | PlayerFilter::TargetPlayerOrControllerOfTarget | PlayerFilter::ControllerOf(_)
        | PlayerFilter::OwnerOf(_) | PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {}
    }
    Ok(rewritten)
}

pub(crate) fn rewrite_filter_words(filter: &ObjectFilter, change: TextChange) -> RewriteResult<ObjectFilter> {
    // Opaque ability markers can encode a landwalk subtype or protection
    // color. Neither treating them as neutral nor parsing their strings is a
    // sound typed implementation. Keep the entire containing definition held.
    if !filter.ability_markers.is_empty() || !filter.excluded_ability_markers.is_empty() {
        return Err(TextChangeDomainError::ObjectFilter);
    }
    let mut residual = filter.clone();
    let empty = ObjectFilter::default();
    macro_rules! admit {
        ($($field:ident),* $(,)?) => { $(residual.$field = empty.$field.clone();)* };
    }
    // This positive field inventory makes nondefault future fields fail closed.
    // String-valued names and surfaces are preserved verbatim, never interpreted.
    admit!(
        zone, match_current_state, has_cumulative_upkeep, controller, cast_by, excluded_cast_origin_zone,
        cast_this_turn, first_spell_cast_each_turn, spell_cast_ordinal_each_turn,
        spell_cast_minimum_each_turn, mana_from_source_spent_to_cast, owner,
        single_graveyard, targets_player, targets_object, targets_any_of, stack_kind,
        target_count, target_set_same_controller, target_set_different_controllers,
        target_set_shared_creature_type, target_set_aggregate_constraint,
        targets_only_player, targets_only_object, targets_only_any_of, could_be_targeted_by,
        not_targeted_by_ability_from, would_destroy_object,
        card_types, all_card_types, card_type_count, excluded_card_types,
        subtypes, all_subtypes, type_or_subtype_union, union_surface, excluded_subtypes,
        supertypes, excluded_supertypes, colors, required_colors, chosen_color,
        colors_chosen_while_drafting_named, name_noted_while_drafting_named,
        chosen_land_type, has_basic_land_type, has_nonbasic_land_type,
        chosen_creature_type, chosen_card_type, excluded_chosen_creature_type,
        excluded_any_chosen_creature_type, excluded_colors, colorless, multicolored,
        monocolored, all_colors, exactly_two_colors, color_count,
        historic, nonhistoric, modified, suspected, transformed, goaded, ring_bearer,
        sticker, token, nontoken, face_down, foretold, other, tapped, untapped,
        attacking, attacking_alone, attacked_this_turn, ability_activated_this_turn,
        blocked_this_turn, was_blocked_this_turn, didnt_attack_this_turn,
        could_have_attacked_this_turn, attacking_player_or_planeswalker_controlled_by,
        attacking_player_only, protected_by, attached_to_object, attached_to_player,
        with_attached_object, without_attached_object, could_enchant_object,
        controller_controls, nonattacking, enlist_eligible, blocking, nonblocking,
        blocked, blocked_by, blocked_by_source, blocked_source_this_turn,
        crewed_by_source_this_turn, blocked_or_was_blocked_by_this_turn,
        unblocked, is_target_object, in_combat_with_source, attacking_same_defender_as_source,
        could_be_enchanted_by_source, in_combat_with, entered_since_your_last_turn_ended,
        controlled_continuously_since_turn_began, didnt_enter_battlefield_this_turn,
        entered_battlefield_this_turn, entered_battlefield_controller, turned_face_up_this_turn,
        put_onto_battlefield_with_source, put_onto_battlefield_with_source_surface,
        created_with_source, created_with_source_surface, entered_graveyard_this_turn,
        entered_graveyard_from_battlefield_this_turn, entered_graveyard_from_library_this_turn,
        milled_into_graveyard_this_turn, surveilled_this_turn, fought_this_turn, attacking_battle,
        counters_put_on_this_turn, discarded_or_cycled_this_turn_by,
        was_dealt_damage_this_turn, dealt_damage_this_turn, dealt_damage_by_source_this_turn,
        was_dealt_damage_by_source_this_game, dealt_damage_to_player_this_turn,
        dealt_damage_to_player_this_turn_combat_only, drawn_this_turn,
        power, power_parity, power_reference, power_relative_to_source,
        power_greater_than_base_power, power_comparison_to_base, power_toughness_relation,
        toughness, toughness_reference, total_power_toughness, mana_value, mana_value_parity,
        mana_value_eq_counters_on_source, exact_mana_cost, has_mana_cost,
        has_phyrexian_mana_symbol, could_produce_mana, has_tap_activated_ability,
        has_non_mana_activated_ability, no_abilities, no_x_in_cost, has_x_in_cost,
        with_counter, without_counter, total_counters_parity, name, name_surface,
        excluded_name, excluded_name_surface, name_originally_printed_in_set,
        distinct_names, distinct_mana_values, distinct_powers, distinct_creature_types,
        shares_land_type, one_per_card_type, alternative_cast,
        static_abilities, excluded_static_abilities,
        no_shared_creature_types_with, characteristic_relations,
        shares_creature_type_with_source, is_commander, noncommander,
        tagged_constraints, specific, any_of, source, source_surface, shares_name,
        shares_color, last_drawn_this_turn, mana_symbol_count, match_captured_public_destination,
    );
    if residual != empty { return Err(TextChangeDomainError::ObjectFilter); }
    let mut rewritten = filter.clone();
    for words in [&mut rewritten.colors, &mut rewritten.required_colors].into_iter().flatten() {
        change.replace_color_words(words);
    }
    change.replace_color_words(&mut rewritten.excluded_colors);
    change.replace_subtype_words(&mut rewritten.subtypes);
    change.replace_subtype_words(&mut rewritten.all_subtypes);
    change.replace_subtype_words(&mut rewritten.excluded_subtypes);
    if let Some((color, _)) = &mut rewritten.mana_symbol_count {
        // This is the adjective in "blue mana symbols in its mana cost".
        // The referenced spell's actual pips are not rewritten.
        change.replace_color_word(color);
    }
    for player in [
        &mut rewritten.controller, &mut rewritten.cast_by, &mut rewritten.owner,
        &mut rewritten.targets_player, &mut rewritten.targets_only_player,
        &mut rewritten.attacking_player_or_planeswalker_controlled_by,
        &mut rewritten.protected_by, &mut rewritten.attached_to_player,
        &mut rewritten.entered_battlefield_controller,
        &mut rewritten.discarded_or_cycled_this_turn_by,
        &mut rewritten.dealt_damage_to_player_this_turn, &mut rewritten.last_drawn_this_turn,
    ].into_iter().flatten() {
        *player = rewrite_player_filter_words(player, change)?;
    }
    for nested in [
        &mut rewritten.mana_from_source_spent_to_cast, &mut rewritten.targets_object,
        &mut rewritten.targets_only_object, &mut rewritten.not_targeted_by_ability_from,
        &mut rewritten.would_destroy_object, &mut rewritten.attached_to_object,
        &mut rewritten.with_attached_object, &mut rewritten.without_attached_object,
        &mut rewritten.could_enchant_object, &mut rewritten.controller_controls,
        &mut rewritten.blocked_or_was_blocked_by_this_turn,
    ].into_iter().flatten() {
        **nested = rewrite_filter_words(nested, change)?;
    }
    for comparison in [
        &mut rewritten.card_type_count, &mut rewritten.color_count, &mut rewritten.power,
        &mut rewritten.toughness, &mut rewritten.total_power_toughness, &mut rewritten.mana_value,
    ].into_iter().flatten() {
        *comparison = rewrite_filter_comparison_words(comparison, change)?;
    }
    if let Some(constraint) = &mut rewritten.target_set_aggregate_constraint {
        **constraint = rewrite_aggregate_constraint_words(constraint, change)?;
    }
    if let Some(constraint) = &mut rewritten.counters_put_on_this_turn {
        constraint.source_controller = rewrite_player_filter_words(&constraint.source_controller, change)?;
    }
    for nested in rewritten.any_of.iter_mut().chain(rewritten.no_shared_creature_types_with.iter_mut()) {
        *nested = rewrite_filter_words(nested, change)?;
    }
    for relation in &mut rewritten.characteristic_relations {
        relation.comparison = rewrite_filter_words(&relation.comparison, change)?;
    }
    // exact_mana_cost and could_produce_mana are symbol
    // predicates. Choice flags, historical captures and identity constraints
    // contain no literal word occurrence and retain their original meanings.
    Ok(rewritten)
}

pub(crate) fn rewrite_filter_comparison_words(comparison: &FilterComparison, change: TextChange) -> RewriteResult<FilterComparison> {
    let mut rewritten = comparison.clone();
    match &mut rewritten {
        FilterComparison::EqualExpr(value) | FilterComparison::NotEqualExpr(value)
        | FilterComparison::LessThanExpr(value) | FilterComparison::LessThanOrEqualExpr(value)
        | FilterComparison::GreaterThanExpr(value) | FilterComparison::GreaterThanOrEqualExpr(value) => {
            **value = rewrite_value_words(value, change)?;
        }
        FilterComparison::Equal(_) | FilterComparison::OneOf(_) | FilterComparison::NotEqual(_)
        | FilterComparison::LessThan(_) | FilterComparison::LessThanOrEqual(_)
        | FilterComparison::GreaterThan(_) | FilterComparison::GreaterThanOrEqual(_) => {}
    }
    Ok(rewritten)
}

pub(crate) fn rewrite_aggregate_constraint_words(constraint: &ChoiceAggregateConstraint, change: TextChange) -> RewriteResult<ChoiceAggregateConstraint> {
    let mut rewritten = constraint.clone();
    if let Some(value) = &mut rewritten.minimum { *value = rewrite_value_words(value, change)?; }
    rewritten.maximum = rewrite_value_words(&rewritten.maximum, change)?;
    Ok(rewritten)
}

fn rewrite_prior_metric_words(query: &mut PriorEffectMetricQuery, change: TextChange) -> RewriteResult<()> {
    if let Some(filter) = &mut query.filter { *filter = rewrite_filter_words(filter, change)?; }
    if let Some(player) = &mut query.player { *player = rewrite_player_filter_words(player, change)?; }
    Ok(())
}

pub(crate) fn rewrite_value_words(value: &Value, change: TextChange) -> RewriteResult<Value> {
    let mut rewritten = value.clone();
    match &mut rewritten {
        Value::SurfaceHinted { value, .. } | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _) | Value::HalfRoundedDown(value)
        | Value::PowerOfTwo(value) => {
            **value = rewrite_value_words(value, change)?;
        }
        Value::Add(left, right) | Value::Min(left, right) => {
            **left = rewrite_value_words(left, change)?;
            **right = rewrite_value_words(right, change)?;
        }
        Value::Count(filter) | Value::CountScaled(filter, _) | Value::GreatestCount(filter)
        | Value::LeastCount(filter)
        | Value::GreatestSharedCreatureTypeCount(filter) | Value::GreatestSharedNameCount(filter)
        | Value::TotalPower(filter) | Value::TotalToughness(filter) | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter) | Value::GreatestToughness(filter) | Value::GreatestManaValue(filter)
        | Value::LeastPower(filter) | Value::LeastToughness(filter) | Value::LeastManaValue(filter)
        | Value::BasicLandTypesAmong(filter) | Value::CreatureTypesAmong(filter)
        | Value::CardTypesAmong(filter) | Value::StaticAbilitiesAmong { filter, .. }
        | Value::ColorsAmong(filter) | Value::ColorPairsAmong(filter)
        | Value::DistinctCounterTypesAmong(filter) | Value::DistinctNames(filter)
        | Value::DistinctManaValues(filter) | Value::UnlockedDoorsAmong(filter) | Value::DistinctPowers(filter)
        | Value::ManaFromSourceSpentToCastThisSpell { source_filter: filter, .. } => {
            *filter = rewrite_filter_words(filter, change)?;
        }
        Value::CreaturesDiedThisTurnControlledBy(player) | Value::CountPlayers(player)
        | Value::CountPlayersWithCardsInHandAtLeast(player, _)
        | Value::CountPlayersWithCardsInGraveyardAtLeast(player, _)
        | Value::CountPlayersWithPoisonCountersAtLeast(player, _)
        | Value::PartySize(player) | Value::LifeTotal(player) | Value::LifeTotalAsTurnBegan(player)
        | Value::LifeTotalDifference(player) | Value::UnspentMana(player) | Value::Speed(player)
        | Value::StartingLifeTotal(player) | Value::HalfLifeTotalRoundedUp(player)
        | Value::HalfLifeTotalRoundedDown(player) | Value::HalfStartingLifeTotalRoundedUp(player)
        | Value::HalfStartingLifeTotalRoundedDown(player) | Value::CardsInHand(player)
        | Value::CardsInLibrary(player) | Value::DevotionToChosenColor(player)
        | Value::LifeGainedThisTurn(player) | Value::LifeLostThisTurn(player)
        | Value::CardsDiscardedThisTurn(player) | Value::AttractionsVisitedThisTurn(player)
        | Value::DamageDealtToPlayersThisTurn(player) | Value::NoncombatDamageDealtToPlayersThisTurn(player)
        | Value::MaxCardsDrawnThisTurn(player) | Value::MaxDiceRolledThisTurn(player)
        | Value::LandsEnteredBattlefieldThisTurn(player) | Value::MaxCardsInHand(player)
        | Value::CardsInGraveyard(player) | Value::SpellsCastThisTurn(player)
        | Value::SpellsCastBeforeThisTurn(player) | Value::CommanderCastCount(player)
        | Value::CommanderColorIdentityColors(player) | Value::CardTypesInGraveyard(player)
        | Value::PlayerCounters(player, _) | Value::PlayerVoteCount(player)
        | Value::MaximumLifeTotal(player) | Value::CountPlayersBelowHalfStartingLifeTotal(player) => {
            *player = rewrite_player_filter_words(player, change)?;
        }
        Value::PlayersWhoControl { players: player, filter }
        | Value::PlayersWhoControlMoreThanYou { players: player, filter }
        | Value::PlayersWhoControlAtLeastMoreThanYou { players: player, filter, .. }
        | Value::SpellsCastThisTurnMatching { player, filter, .. }
        | Value::TotalManaValueOfSpellsCastThisTurnMatching { player, filter, .. }
        | Value::CardTypesAmongSpellsCastThisTurn { player, filter } => {
            *player = rewrite_player_filter_words(player, change)?;
            *filter = rewrite_filter_words(filter, change)?;
        }
        Value::PowerOf(spec) | Value::ToughnessOf(spec) | Value::ManaValueOf(spec)
        | Value::ManaSpentToCast(spec) | Value::ColorsOf(spec) | Value::CountersOn(spec, _)
        | Value::ObjectVoteCount(spec) | Value::KicksPaidOf(spec) | Value::BasePowerOf(spec) => {
            **spec = rewrite_choose_spec_words(spec, change)?;
        }
        Value::ManaSymbolsInManaCostOf { spec, color } => {
            **spec = rewrite_choose_spec_words(spec, change)?;
            // This model authors the adjective in "blue mana symbols".
            change.replace_color_word(color);
        }
        Value::EventValue(event) | Value::EventValueOffset(event, _) => {
            // The semantic front end binds "that many" to the triggering
            // filter's authored color query. Rewrite the query consistently
            // with that predicate, never the event's captured mana symbols.
            match event {
                EventValueSpec::CastSpell(quantity) => match quantity {
                    CastEventQuantity::ManaSymbols(color) => change.replace_color_word(color),
                    CastEventQuantity::ManaValue | CastEventQuantity::DistinctTargets => {}
                },
                EventValueSpec::Amount | EventValueSpec::LifeAmount
                | EventValueSpec::BlockersBeyondFirst { .. } | EventValueSpec::DieResult
                | EventValueSpec::LifeChange { .. } | EventValueSpec::DieBatchTotal
                | EventValueSpec::DieResultsAtLeast(_) => {}
            }
        }
        Value::ManaSpentOnX(color) => {
            // Its grammar accepts either a color name or a mana code, but
            // this model retains no distinction between those word roles.
            if change.from() == TextWord::Color(*color) {
                return Err(TextChangeDomainError::Value);
            }
        }
        Value::Devotion { player, color } => {
            *player = rewrite_player_filter_words(player, change)?;
            change.replace_color_word(color);
        }
        Value::NoncombatDamageDealtBySourcesControlledThisTurn { player, colors } => {
            *player = rewrite_player_filter_words(player, change)?;
            if let Some(colors) = colors { change.replace_color_words(colors); }
        }
        Value::PriorEffectMetric { query, .. } => rewrite_prior_metric_words(query, change)?,
        Value::TurnHistoryCount(count) => *count = rewrite_turn_history_count_words(count, change)?,
        Value::DamageHistory(query) => {
            // Shared typed accessors preserve event reduction and combat mode.
            for spec in query.reference_specs_mut() { *spec = rewrite_choose_spec_words(spec, change)?; }
            for filter in query.object_filters_mut() { *filter = rewrite_filter_words(filter, change)?; }
            if let Some(player) = query.player_filter_mut() { *player = rewrite_player_filter_words(player, change)?; }
        }
        // Compiler-only unresolved references are not executable definitions.
        Value::PendingEffectMetric { .. } | Value::PendingEffectMetricOffset { .. }
        | Value::PendingPriorEffectMetric(_) | Value::PendingComparisonLeft
        | Value::PendingComparisonRight | Value::PendingComparisonDifference => {
            return Err(TextChangeDomainError::Value);
        }
        Value::Fixed(_) | Value::X | Value::XTimes(_) | Value::AnnouncedTargetTotal(_)
        | Value::CreaturesDiedThisTurn | Value::PlayersBeingAttacked | Value::SourcePower
        | Value::SourceToughness | Value::NameStickerCharacterCountOnSource { .. }
        | Value::ThisAbilityResolvedThisTurnCount | Value::SourceRegeneratedThisTurnCount
        | Value::SourceMutationCount | Value::SourceDevouredCreatureCount
        | Value::DamageDealtThisTurnByTaggedSpellCast(_) | Value::ManaSpentToCastThisSpell
        | Value::ManaSymbolSpentToCastThisSpell { .. } | Value::ManaSpentToCastTriggeringObject
        | Value::ColorsOfManaSpentToCastThisSpell | Value::EffectValue(_) | Value::EffectValueOffset(_, _)
        | Value::EffectMetric { .. } | Value::EffectMetricOffset { .. }
        | Value::WasKicked
        | Value::WasBoughtBack | Value::WasEntwined | Value::WasPaid(_) | Value::WasPaidLabel(_)
        | Value::TimesPaid(_) | Value::TimesPaidLabel(_) | Value::KickCount
        | Value::MagicGamesLostToOpponentsSinceLastWin | Value::DraftNotedHighestNumber { .. }
        | Value::DraftRemovedCardCount { .. } | Value::SourceChosenNumber { .. } | Value::LastNotedLifeTotal
        | Value::CountersOnSource(_) | Value::CountersOnFilterCandidate(_) | Value::TaggedCount
        | Value::VoteCount(_) | Value::CasterManaSpentToCastTriggeringObject => {}
    }
    Ok(rewritten)
}

pub(crate) fn rewrite_anthem_count_words(count: &AnthemCountExpression, change: TextChange) -> RewriteResult<AnthemCountExpression> {
    let mut rewritten = count.clone();
    match &mut rewritten {
        AnthemCountExpression::MatchingFilter(filter) | AnthemCountExpression::GreatestManaValueAmong(filter)
        | AnthemCountExpression::AttachedToSource(filter) | AnthemCountExpression::AttachedToAffected(filter)
        | AnthemCountExpression::CountersAmong(filter, _) | AnthemCountExpression::DistinctCounterTypesAmong(filter)
        | AnthemCountExpression::BasicLandTypesAmong(filter) | AnthemCountExpression::CreatureTypesAmong(filter) => {
            *filter = rewrite_filter_words(filter, change)?;
        }
        AnthemCountExpression::CommanderCastCount(player) | AnthemCountExpression::PlayerSpeed(player)
        | AnthemCountExpression::UnspentMana { player, .. } | AnthemCountExpression::TotalUnspentMana(player)
        | AnthemCountExpression::PlayerCounters(player, _) => {
            *player = rewrite_player_filter_words(player, change)?;
        }
        AnthemCountExpression::GraveyardsWithAtLeastCards { .. } | AnthemCountExpression::ColorsOfAffected
        | AnthemCountExpression::AffectedAttackedThisTurn | AnthemCountExpression::CountersOnSource(_)
        | AnthemCountExpression::CountersOnSourceWithSurface { .. }
        | AnthemCountExpression::CountersOnSourceWithPronoun { .. }
        | AnthemCountExpression::StickersOnSource { .. } | AnthemCountExpression::CountersOnAffected(_)
        | AnthemCountExpression::BlockingSource => {}
    }
    Ok(rewritten)
}

fn rewrite_turn_history_count_words(count: &TurnHistoryCount, change: TextChange) -> RewriteResult<TurnHistoryCount> {
    let mut rewritten = count.clone();
    match &mut rewritten {
        TurnHistoryCount::Died { filter, .. } | TurnHistoryCount::EnteredBattlefield(filter)
        | TurnHistoryCount::MovedZones { filter, .. } | TurnHistoryCount::CountersRemovedFrom { filter, .. } => {
            *filter = rewrite_filter_words(filter, change)?;
        }
        TurnHistoryCount::TokensCreated(player) | TurnHistoryCount::TurnedFaceUp(player)
        | TurnHistoryCount::PutIntoGraveyard { owner: player, .. }
        | TurnHistoryCount::OpponentsAttacked(player) | TurnHistoryCount::PlayersAttackedThisCombat(player)
        | TurnHistoryCount::PlayersDiscarded(player) | TurnHistoryCount::PlayersDealtDamage(player)
        | TurnHistoryCount::DiscardedOrCycled(player) | TurnHistoryCount::Cycled(player)
        | TurnHistoryCount::LandsPlayed(player)
        | TurnHistoryCount::CardsDrawn(player) | TurnHistoryCount::PlayersLostLife(player)
        | TurnHistoryCount::UntappedLandsAtTurnStart(player) | TurnHistoryCount::Descended(player)
        | TurnHistoryCount::KeywordActionsPerformed { player, .. }
        | TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(player)
        | TurnHistoryCount::LibrarySearches { player, .. } => {
            *player = rewrite_player_filter_words(player, change)?;
        }
        TurnHistoryCount::Sacrificed { player, filter } | TurnHistoryCount::SacrificedCardTypes { player, filter }
        | TurnHistoryCount::CreaturesAttackedWith { player, filter }
        | TurnHistoryCount::PlayersDealtCombatDamageBy { players: player, sources: filter }
        | TurnHistoryCount::SpellsCast { player, filter, .. }
        | TurnHistoryCount::MaxEnteredBattlefieldByController { player, filter } => {
            *player = rewrite_player_filter_words(player, change)?;
            *filter = rewrite_filter_words(filter, change)?;
        }
        TurnHistoryCount::CountersPutOn { source_controller, filter, .. } => {
            if let Some(player) = source_controller { *player = rewrite_player_filter_words(player, change)?; }
            *filter = rewrite_filter_words(filter, change)?;
        }
        TurnHistoryCount::DestroyedBy { filter, cause } => {
            *filter = rewrite_filter_words(filter, change)?;
            if let Some(source) = &mut cause.source_filter { *source = rewrite_filter_words(source, change)?; }
        }
        TurnHistoryCount::CastSpellsCounteredBy { caster, filter, cause } => {
            *caster = rewrite_player_filter_words(caster, change)?;
            *filter = rewrite_filter_words(filter, change)?;
            if let Some(source) = &mut cause.source_filter { *source = rewrite_filter_words(source, change)?; }
        }
        TurnHistoryCount::DamageDealtToSource | TurnHistoryCount::DamageDealtBySource => {}
    }
    Ok(rewritten)
}

fn rewrite_turn_history_condition_words(condition: &TurnHistoryCondition, change: TextChange) -> RewriteResult<TurnHistoryCondition> {
    let mut rewritten = condition.clone();
    match &mut rewritten {
        TurnHistoryCondition::ObjectAttackedDuringControllersLastTurn(filter)
        | TurnHistoryCondition::SourceCrewedByAtLeast { filter, .. }
        | TurnHistoryCondition::ManaFromSourceSpentOnTriggeringAction { source_filter: filter }
        | TurnHistoryCondition::AnotherOpponentControlsPotentialTarget { filter } => {
            *filter = rewrite_filter_words(filter, change)?;
        }
        TurnHistoryCondition::PlayerPlayedLandThisTurn(player)
        | TurnHistoryCondition::PlayerActivatedLoyaltyAbilityThisTurn(player)
        | TurnHistoryCondition::PlayerPlayedCardFromZoneThisTurn { player, .. }
        | TurnHistoryCondition::PlayerCastSpellFromZoneThisTurn { player, .. }
        | TurnHistoryCondition::PlayerActivatedAbilityOfCardInZoneThisTurn { player, .. }
        | TurnHistoryCondition::PlayerVisitedAttractionThisTurn(player)
        | TurnHistoryCondition::PlayerLostLifeLastTurn(player) => {
            *player = rewrite_player_filter_words(player, change)?;
        }
        TurnHistoryCondition::TriggeringAttackerBlockers { required, prohibited, .. } => {
            *required = rewrite_filter_words(required, change)?;
            *prohibited = rewrite_filter_words(prohibited, change)?;
        }
        TurnHistoryCondition::SpellsCastLastTurnAtLeast(_) | TurnHistoryCondition::SourceWasCast { .. }
        | TurnHistoryCondition::SourceWasCastByController { .. } | TurnHistoryCondition::SourceWasKicked { .. }
        | TurnHistoryCondition::SourceEnteredBattlefieldThisTurn { .. } | TurnHistoryCondition::SourceAttackedThisTurn { .. }
        | TurnHistoryCondition::TriggeringObjectEnlistedThisCombat | TurnHistoryCondition::TriggeringObjectWasCast
        | TurnHistoryCondition::TriggeringObjectWasCastFromZone(_) | TurnHistoryCondition::TriggeringObjectDied
        | TurnHistoryCondition::TriggeringPlayerAttackedControllerLastTurn | TurnHistoryCondition::TriggeringPlayersTurn { .. }
        | TurnHistoryCondition::ControllerTeamGainedLifeThisTurn | TurnHistoryCondition::TriggeringObjectsNoneWereCastOrNoManaSpent
        | TurnHistoryCondition::AllPlayersLifeAtMost(_) | TurnHistoryCondition::TriggeringAbilityIsManaAbility => {}
    }
    Ok(rewritten)
}

pub(crate) fn rewrite_condition_words(condition: &Condition, change: TextChange) -> RewriteResult<Condition> {
    let mut rewritten = condition.clone();
    match &mut rewritten {
        Condition::Not(inner) => **inner = rewrite_condition_words(inner, change)?,
        Condition::And(left, right) | Condition::Or(left, right) => {
            **left = rewrite_condition_words(left, change)?;
            **right = rewrite_condition_words(right, change)?;
        }
        Condition::YouControl(filter) | Condition::OpponentControls(filter)
        | Condition::YouHaveCardInHandMatching(filter) | Condition::ObjectEnteredBattlefieldThisTurn(filter)
        | Condition::TopCardOfYourLibraryMatches(filter)
        | Condition::ObjectEnteredBattlefieldLastTurn(filter)
        | Condition::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter)
        | Condition::SourceCrewedByExactly { filter, .. } | Condition::SourceMatches(filter)
        | Condition::AttachedToSourceMatches(filter) | Condition::TaggedObjectMatches(_, filter)
        | Condition::TaggedObjectMatchedLastKnown(_, filter) | Condition::TargetMatches(filter)
        | Condition::SourceInGraveyardWithCardsAbove { filter, .. }
        | Condition::SourceSoulbondPartnerMatches(filter)
        | Condition::CreatureDealtDamageBySourceDiedThisTurn { victim: filter, .. } => {
            *filter = rewrite_filter_words(filter, change)?;
        }
        Condition::PlayerControls { player, filter } | Condition::PlayerHasAtLeast { player, filter, .. }
        | Condition::PlayerControlsExactly { player, filter, .. }
        | Condition::PlayerHasAtLeastWithDifferentPowers { player, filter, .. }
        | Condition::PlayerControlsMost { player, filter }
        | Condition::PlayerControlsMoreThanEachOtherPlayer { player, filter }
        | Condition::PlayerControlsMoreThanYou { player, filter }
        | Condition::AnOpponentControlsMoreThanPlayer { player, filter }
        | Condition::AnOpponentHasFewerThanPlayer { player, filter }
        | Condition::PlayerRemovedDraftCardMatching { player, filter, .. }
        | Condition::PlayerTaggedObjectMatches { player, filter, .. } => {
            *player = rewrite_player_filter_words(player, change)?;
            *filter = rewrite_filter_words(filter, change)?;
        }
        Condition::PlayerControlsBasicLandTypesAmongLandsOrMore { player, .. }
        | Condition::PlayerLifeAtMostHalfStartingLifeTotal { player }
        | Condition::PlayerLifeLessThanHalfStartingLifeTotal { player }
        | Condition::PlayerHasLessLifeThanYou { player } | Condition::PlayerHasMoreLifeThanYou { player }
        | Condition::PlayerHasNoOpponentWithMoreLifeThan { player }
        | Condition::PlayerHasMoreLifeThanEachOtherPlayer { player } | Condition::PlayerIsMonarch { player }
        | Condition::PlayerHasInitiative { player } | Condition::PlayerHasCitysBlessing { player }
        | Condition::PlayerHasEnduringStory { player } | Condition::SourceIsRingBearer { player }
        | Condition::PlayerRingTemptedThisGameOrMore { player, .. } | Condition::PlayerCommittedCrimeThisTurn { player }
        | Condition::PlayerRolledResultThisTurn { player, .. } | Condition::PlayerCompletedDungeon { player, .. }
        | Condition::PlayerCardsInHandOrMore { player, .. } | Condition::PlayerCardsInHandOrFewer { player, .. }
        | Condition::PlayerCardsInHandAtTurnStartOrMore { player, .. } | Condition::PlayerCardsInHandAtTurnStartOrFewer { player, .. }
        | Condition::PlayerHasMoreCardsInHandThanYou { player } | Condition::PlayerHasMoreCardsInHandThanEachOtherPlayer { player }
        | Condition::PlayerHasPoisonCountersOrMore { player, .. } | Condition::PlayerHasCountersOrMore { player, .. }
        | Condition::PlayerCastSpellsThisTurnOrMore { player, .. } | Condition::PlayerTappedLandForManaThisTurn { player }
        | Condition::PlayerGainedLifeThisTurnOrMore { player, .. } | Condition::PlayerHadLandEnterBattlefieldThisTurn { player }
        | Condition::PlayerDescendedThisTurn { player } | Condition::PlayerHasCardTypesInGraveyardOrMore { player, .. }
        | Condition::TaggedObjectIsTopOfLibrary { player, .. } | Condition::PlayerTaggedObjectEnteredBattlefieldThisTurn { player, .. }
        | Condition::PlayerOwnsCardNamedInZones { player, .. } | Condition::PlayerWasMonarchAtTurnStart { player }
        | Condition::TriggeringEventCausedBy { controller: player, .. } => {
            *player = rewrite_player_filter_words(player, change)?;
        }
        Condition::ValueComparison { left, right, .. } => {
            *left = rewrite_value_words(left, change)?;
            *right = rewrite_value_words(right, change)?;
        }
        Condition::ValueIsPrime(value) => *value = rewrite_value_words(value, change)?,
        Condition::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype } => {
            *player = rewrite_player_filter_words(player, change)?;
            change.replace_subtype_word(subtype);
        }
        Condition::CardInYourGraveyard { subtypes, .. } => change.replace_subtype_words(subtypes),
        Condition::AttachmentCount { attachment, host, .. } => {
            *attachment = rewrite_filter_words(attachment, change)?;
            match host {
                AttachmentConditionHost::Matching(filter) => *filter = rewrite_filter_words(filter, change)?,
                AttachmentConditionHost::Source | AttachmentConditionHost::SourceAttachedObject => {}
            }
        }
        Condition::CountComparison { count, .. }
        | Condition::CountParity { count, .. }
        | Condition::MaxActivationsPerTurnCount(count) => {
            *count = rewrite_anthem_count_words(count, change)?;
        }
        Condition::TurnHistory(condition) => *condition = rewrite_turn_history_condition_words(condition, change)?,
        Condition::Custom(_) => return Err(TextChangeDomainError::Condition),
        // These predicates contain no authored word from a replaceable family.
        // References, optional-cost identities, names, counters and mana pips
        // remain exactly as captured in the definition.
        Condition::LifeTotalOrLess(_) | Condition::LifeTotalOrGreater(_) | Condition::CardsInHandOrMore(_)
        | Condition::YourTurn | Condition::CurrentTurnIsExtra | Condition::YourFirstTurnsOfTheGameOrFewer(_)
        | Condition::CreatureDiedThisTurn | Condition::CreatureDiedThisTurnOrMore(_)
        | Condition::CreatureCardPutIntoYourGraveyardThisTurn | Condition::CastSpellThisTurn
        | Condition::AttackedThisTurn | Condition::AttackedWithNOrMoreCreaturesThisTurn(_)
        | Condition::OpponentLostLifeThisTurn | Condition::AnyPlayerLostLifeThisTurnOrMore { .. }
        | Condition::OpponentWasDealtDamageThisTurn | Condition::OpponentWasDealtDamageThisTurnOrMore(_)
        | Condition::PermanentLeftBattlefieldThisTurn | Condition::NonlandPermanentLeftBattlefieldThisTurn
        | Condition::SpellWasWarpedThisTurn | Condition::PermanentLeftBattlefieldUnderYourControlThisTurn { .. }
        | Condition::SourceWasCast | Condition::ThisSpellWasCastAtSorceryTiming | Condition::ThisSpellEscaped
        | Condition::ThisSpellWasCastFromZone(_) | Condition::ThisSpellWasCastFromNonHand
        | Condition::NoSpellsWereCastLastTurn | Condition::SpellsWereCastLastTurnOrMore(_)
        | Condition::TargetIsTapped | Condition::TargetIsAttacking | Condition::TargetIsBlocked
        | Condition::TargetWasKicked | Condition::ThisSpellWasKicked | Condition::ThisSpellPaidLabel(_)
        | Condition::YouHaveFullParty | Condition::TargetSpellCastOrderThisTurn(_)
        | Condition::TargetSpellControllerIsPoisoned | Condition::TargetSpellManaSpentToCastAtLeast { .. }
        | Condition::TriggeringSpellManaSpentToCastAtLeast { .. }
        | Condition::ColoredManaSpentToCastThisSpellAtLeast(_) | Condition::TriggeringSpellColoredManaSpentToCastAtLeast(_)
        | Condition::TriggeringSpellWasKicked | Condition::YouControlMoreCreaturesThanTargetSpellController
        | Condition::TargetHasGreatestPowerAmongCreatures | Condition::TargetManaValueLteColorsSpentToCastThisSpell
        | Condition::ItIsNight | Condition::FirstCombatPhaseOfTurn | Condition::SourceControllersMainPhase
        | Condition::SourceControllersCombatPhase | Condition::SourceControllersEndStep | Condition::OpponentsEndStep | Condition::SourceIsTapped
        | Condition::SourceIsSaddled | Condition::SourceDevouredCreaturesOrMore(_) | Condition::SourceIsMonstrous
        | Condition::SourceHasDealtDamageSinceEntered
        | Condition::SourceIsHarnessed | Condition::SourceIsPrepared | Condition::SourceIsRenowned | Condition::SourceIsFaceDown
        | Condition::SourceHasNoCounter(_) | Condition::SourceHasCounterAtLeast { .. }
        | Condition::SourceHasCountersAtLeast(_) | Condition::SourcePowerAtLeast(_)
        | Condition::SourceDealtCombatDamageToPlayerThisTurn | Condition::ManaSpentToCastThisSpellAtLeast { .. }
        | Condition::SnowManaOfAnySpellColorSpentToCastThisSpell | Condition::TriggeringSpellSnowManaOfAnySpellColorSpentToCast
        | Condition::SameColorManaSpentToCastThisSpellAtLeast(_) | Condition::ColorsOfManaSpentToCastThisSpellOrMore(_)
        | Condition::YouControlCommander | Condition::StableObjectIsTopOfLibrary { .. }
        | Condition::TaggedObjectWasCast(_) | Condition::TaggedObjectIsSoulbondPaired(_)
        | Condition::EnchantedPermanentAttackedThisTurn | Condition::EnchantedPermanentAttackedOrBlockedSinceLastUpkeep
        | Condition::SourceBlockedOrBecameBlockedSinceLastUpkeep | Condition::TargetObjectsHaveDifferentColorSets
        | Condition::TargetIsSoulbondPaired | Condition::ThisAbilityResolvedThisTurnExactly(_)
        | Condition::ThisAbilityActivatedThisTurnAtLeast(_)
        | Condition::FirstTimeThisTurn | Condition::SourceFirstCrewedThisTurn | Condition::MaxTimesEachTurn(_)
        | Condition::DoThisMaxTimesEachTurn(_) | Condition::TriggeringObjectWasEnchanted
        | Condition::TriggeringObjectBecameTappedFirstTimeThisTurn | Condition::TriggeringObjectHadCountersPutFirstTimeThisTurn
        | Condition::TriggeringObjectHadToAttackThisCombat | Condition::YouWonTriggeringClash
        | Condition::TriggeringAbilityManaSpentToActivateAtLeast(_) | Condition::TriggeringObjectEnteredTransformed
        | Condition::EvolveEnteringCreatureIsLarger | Condition::SoulbondPairingPossible
        | Condition::SourceClassLevelAtLeast(_) | Condition::TriggeringObjectHadCounters { .. }
        | Condition::ControlCreaturesTotalPowerAtLeast(_) | Condition::SourceIsInZone(_)
        | Condition::ActivationTiming(_) | Condition::MaxActivationsPerTurn(_) | Condition::MaxActivationsPerObject(_)
        | Condition::SourceIsEquipped | Condition::SourceIsEnchanted | Condition::EnchantedPermanentIsCreature
        | Condition::EnchantedPermanentIsLand | Condition::EnchantedPermanentIsEquipment | Condition::EnchantedPermanentIsVehicle
        | Condition::EquippedCreatureTapped | Condition::EquippedCreatureUntapped | Condition::EquippedCreatureAttacking
        | Condition::SourceChosenOption(_) | Condition::SecretChoicesMatch | Condition::VoteOptionGetsMoreVotes(_)
        | Condition::VoteOptionGetsMoreVotesOrTied(_) | Condition::OwnsCardExiledWithCounter(_)
        | Condition::SourceAttackedThisTurn | Condition::SourceAttackedBattleThisTurn | Condition::SourceSuspected
        | Condition::SourceCameUnderYourControlThisTurn | Condition::SourceCameUnderYourControlSinceYourLastUpkeep
        | Condition::SourceAttackedOrBlockedThisTurn | Condition::SourceAttackedOrBlockedThisCombat
        | Condition::SourceIsUntapped | Condition::SourceIsAttacking | Condition::SourceIsBlocking
        | Condition::SourceIsSoulbondPaired | Condition::PlayerGraveyardHasCardsAtLeast { .. }
        | Condition::XValueAtLeast(_) | Condition::AllTargetsStillLegal
        | Condition::AttackedWithTotalPowerAtLeastThisCombat(_) | Condition::YouChoseAnotherRingBearer
        | Condition::CombatParticipant(_)
        | Condition::SourceCaseSolved | Condition::ThisSpellWasForetold => {}
    }
    Ok(rewritten)
}

#[cfg(test)]
mod tests {
    // Authored regression scenarios. Intentionally unrun while the user's
    // source-only change/review restriction is in force.
    use super::*;
    use ironsmith_core::{
        ChoiceAggregateMetric, ChoiceCount, Color, ColorSet, Comparison, CounterType,
        EffectId, EffectMetric, EffectMetricSource, ManaCost, ManaSymbol,
        ObjectCharacteristic, ObjectCharacteristicRelation, ObjectRef,
        SourceReferenceSurface, Subtype, TaggedObjectConstraint, TaggedOpbjectRelation,
        ValueComparisonOperator, ValueSurfaceHint, Zone,
    };

    fn black_words() -> ObjectFilter {
        ObjectFilter { colors: Some(ColorSet::BLACK), ..ObjectFilter::default() }
    }
    fn blue_words() -> ObjectFilter {
        ObjectFilter { colors: Some(ColorSet::BLUE), ..ObjectFilter::default() }
    }
    fn black_to_blue() -> TextChange { TextChange::color(Color::Black, Color::Blue).unwrap() }

    #[test]
    fn activation_history_threshold_has_no_replaceable_authored_word() {
        let condition = Condition::ThisAbilityActivatedThisTurnAtLeast(4);
        assert_eq!(rewrite_condition_words(&condition, black_to_blue()).unwrap(), condition);
    }

    #[test]
    fn nested_legality_predicates_change_without_rebinding_names_mana_or_choices() {
        let mut original = black_words();
        original.name = Some("Black Knight".into());
        original.name_surface = ironsmith_core::LiteralNameSurface::new("Black Knight");
        original.excluded_name = Some("Blue Elemental Blast".into());
        original.exact_mana_cost = Some(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]));
        original.could_produce_mana = vec![ManaSymbol::Black];
        original.chosen_color = true;
        original.chosen_creature_type = true;
        original.controller = Some(PlayerFilter::ControlsMost { filter: Box::new(black_words()) });
        original.owner = Some(PlayerFilter::OwnerOf(ObjectRef::Tagged("black-owner".into())));
        original.targets_object = Some(Box::new(black_words()));
        original.attached_to_object = Some(Box::new(black_words()));
        original.power = Some(FilterComparison::LessThanOrEqualExpr(Box::new(Value::Count(black_words()))));
        original.target_set_aggregate_constraint = Some(Box::new(ChoiceAggregateConstraint::at_most(
            ChoiceAggregateMetric::ManaValue, Value::Count(black_words()),
        )));
        original.characteristic_relations = vec![ObjectCharacteristicRelation::shares(
            vec![ObjectCharacteristic::Color], black_words(),
        )];
        original.tagged_constraints = vec![TaggedObjectConstraint {
            tag: "black-captured-object".into(), relation: TaggedOpbjectRelation::SameObjectId,
        }];
        original.source_surface = Some(SourceReferenceSurface::FullName("Black Knight".into()));

        let rewritten = rewrite_filter_words(&original, black_to_blue()).unwrap();
        let mut expected = original.clone();
        expected.colors = Some(ColorSet::BLUE);
        expected.controller = Some(PlayerFilter::ControlsMost { filter: Box::new(blue_words()) });
        expected.targets_object = Some(Box::new(blue_words()));
        expected.attached_to_object = Some(Box::new(blue_words()));
        expected.power = Some(FilterComparison::LessThanOrEqualExpr(Box::new(Value::Count(blue_words()))));
        expected.target_set_aggregate_constraint = Some(Box::new(ChoiceAggregateConstraint::at_most(
            ChoiceAggregateMetric::ManaValue, Value::Count(blue_words()),
        )));
        expected.characteristic_relations = vec![ObjectCharacteristicRelation::shares(
            vec![ObjectCharacteristic::Color], blue_words(),
        )];
        assert_eq!(rewritten, expected);
        assert_eq!(rewritten.name_surface.as_deref(), Some("Black Knight"));
        assert_eq!(rewritten.source_surface, original.source_surface);
        assert_eq!(original.colors, Some(ColorSet::BLACK));
        assert_eq!(rewritten.owner, original.owner);
        assert_eq!(rewritten.tagged_constraints, original.tagged_constraints);
    }

    #[test]
    fn dynamic_choice_visits_value_and_player_scopes_preserving_binding_wrappers() {
        let spec = ChooseSpec::WithCountValue(
            Box::new(ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Excluding {
                base: Box::new(PlayerFilter::ControlsFewestTied { filter: Box::new(black_words()) }),
                excluded: Box::new(PlayerFilter::TaggedPlayer("black-player".into())),
            }))),
            ChoiceCount::at_least(1),
            Value::Devotion { player: PlayerFilter::ChosenPlayer, color: Color::Black }
                .with_surface_hint(ValueSurfaceHint::WhereXIs),
        ).with_surface_hint(ironsmith_core::ChooseSpecSurfaceHint::SourceReference(
            SourceReferenceSurface::FullName("Black Knight".into()),
        ));
        let result = rewrite_choose_spec_words(&spec, black_to_blue()).unwrap();
        let ChooseSpec::SurfaceHinted { spec: counted, hints } = result else { panic!("surface wrapper lost"); };
        assert_eq!(hints.as_slice(), spec.surface_hints());
        let ChooseSpec::WithCountValue(target, count, value) = *counted else { panic!("dynamic choice lost"); };
        assert_eq!(count, ChoiceCount::at_least(1));
        assert_eq!(*target, ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Excluding {
            base: Box::new(PlayerFilter::ControlsFewestTied { filter: Box::new(blue_words()) }),
            excluded: Box::new(PlayerFilter::TaggedPlayer("black-player".into())),
        })));
        assert_eq!(value, Value::Devotion { player: PlayerFilter::ChosenPlayer, color: Color::Blue }
            .with_surface_hint(ValueSurfaceHint::WhereXIs));
    }

    #[test]
    fn authored_mana_color_query_and_bound_event_quantity_change_together() {
        let filter = ObjectFilter {
            mana_symbol_count: Some((Color::Black, ChoiceCount::at_least(1))),
            exact_mana_cost: Some(ManaCost::from_pips(vec![vec![ManaSymbol::Black]])),
            ..ObjectFilter::default()
        };
        let result = rewrite_filter_words(&filter, black_to_blue()).unwrap();
        assert_eq!(result.mana_symbol_count, Some((Color::Blue, ChoiceCount::at_least(1))));
        assert_eq!(result.exact_mana_cost, filter.exact_mana_cost);
        assert_eq!(rewrite_value_words(&Value::EventValue(EventValueSpec::CastSpell(
            CastEventQuantity::ManaSymbols(Color::Black),
        )), black_to_blue()).unwrap(), Value::EventValue(EventValueSpec::CastSpell(
            CastEventQuantity::ManaSymbols(Color::Blue),
        )));
        let named_query = Value::ManaSymbolsInManaCostOf {
            spec: Box::new(ChooseSpec::All(black_words())), color: Color::Black,
        };
        assert_eq!(rewrite_value_words(&named_query, black_to_blue()).unwrap(), Value::ManaSymbolsInManaCostOf {
            spec: Box::new(ChooseSpec::All(blue_words())), color: Color::Blue,
        });
        let literal_payment = Condition::ManaSpentToCastThisSpellAtLeast {
            amount: 1, symbol: Some(ManaSymbol::Black),
        };
        assert_eq!(rewrite_condition_words(&literal_payment, black_to_blue()).unwrap(), literal_payment);
    }

    #[test]
    fn reference_bound_counts_rewrite_predicates_without_changing_producer_identity() {
        let query = PriorEffectMetricQuery::new(EffectMetricSource::Outcome, EffectMetric::Count)
            .with_filter(black_words())
            .with_player(PlayerFilter::ControlsMost { filter: Box::new(black_words()) });
        let source = Value::PriorEffectMetric { effect_id: EffectId::ACTIVATION_COUNTER_COST, query };
        let result = rewrite_value_words(&source, black_to_blue()).unwrap();
        let Value::PriorEffectMetric { effect_id, query } = result else { panic!("producer binding lost"); };
        assert_eq!(effect_id, EffectId::ACTIVATION_COUNTER_COST);
        assert_eq!(query.source, EffectMetricSource::Outcome);
        assert_eq!(query.filter, Some(blue_words()));
        assert_eq!(query.player, Some(PlayerFilter::ControlsMost { filter: Box::new(blue_words()) }));
        assert_eq!(rewrite_value_words(&source, TextChange::creature_type(Subtype::Human, Subtype::Vampire).unwrap()).unwrap(), source);
    }

    #[test]
    fn conditions_rewrite_authored_types_through_history_and_attachment_predicates() {
        let elf = ObjectFilter { subtypes: vec![Subtype::Elf], ..ObjectFilter::default() };
        let vampire = ObjectFilter { subtypes: vec![Subtype::Vampire], ..ObjectFilter::default() };
        let condition = Condition::And(
            Box::new(Condition::Not(Box::new(Condition::AttachmentCount {
                attachment: elf.clone(), host: AttachmentConditionHost::Matching(elf.clone()),
                comparison: Comparison::GreaterThanOrEqual(1), display: "Elves attached to Elves".into(),
            }))),
            Box::new(Condition::ValueComparison {
                left: Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
                    player: PlayerFilter::You, filter: elf, from_zone: Some(Zone::Graveyard),
                    from_outside_hand: true, exclude_source: true, before_triggering_spell: true,
                }),
                operator: ValueComparisonOperator::GreaterThan, right: Value::Fixed(0),
            }),
        );
        let expected = Condition::And(
            Box::new(Condition::Not(Box::new(Condition::AttachmentCount {
                attachment: vampire.clone(), host: AttachmentConditionHost::Matching(vampire.clone()),
                comparison: Comparison::GreaterThanOrEqual(1), display: "Elves attached to Elves".into(),
            }))),
            Box::new(Condition::ValueComparison {
                left: Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
                    player: PlayerFilter::You, filter: vampire, from_zone: Some(Zone::Graveyard),
                    from_outside_hand: true, exclude_source: true, before_triggering_spell: true,
                }),
                operator: ValueComparisonOperator::GreaterThan, right: Value::Fixed(0),
            }),
        );
        assert_eq!(rewrite_condition_words(&condition, TextChange::creature_type(Subtype::Elf, Subtype::Vampire).unwrap()).unwrap(), expected);
    }

    #[test]
    fn generic_land_and_party_rules_do_not_author_their_implied_types() {
        let mut filter = ObjectFilter {
            subtypes: vec![Subtype::Island, Subtype::Forest],
            excluded_subtypes: vec![Subtype::Island],
            has_basic_land_type: true, chosen_land_type: true,
            ..ObjectFilter::default()
        };
        let change = TextChange::basic_land_type(Subtype::Island, Subtype::Forest).unwrap();
        let rewritten = rewrite_filter_words(&filter, change).unwrap();
        filter.subtypes = vec![Subtype::Forest];
        filter.excluded_subtypes = vec![Subtype::Forest];
        assert_eq!(rewritten, filter);
        let party = Value::PartySize(PlayerFilter::You);
        assert_eq!(rewrite_value_words(&party, TextChange::creature_type(Subtype::Wizard, Subtype::Wall).unwrap()).unwrap(), party);
        let counters = Value::CountersOnSource(CounterType::Fungus);
        assert_eq!(rewrite_value_words(&counters, TextChange::creature_type(Subtype::Fungus, Subtype::Elf).unwrap()).unwrap(), counters);
    }

    #[test]
    fn incomplete_nested_domains_fail_without_mutating_the_input() {
        let mut filter = black_words();
        filter.any_of.push(ObjectFilter {
            ability_markers: vec!["swampwalk".into()], ..ObjectFilter::default()
        });
        let original = filter.clone();
        assert_eq!(rewrite_filter_words(&filter, black_to_blue()), Err(TextChangeDomainError::ObjectFilter));
        assert_eq!(filter, original);
        let condition = Condition::And(
            Box::new(Condition::YouControl(black_words())),
            Box::new(Condition::Custom("opaque condition".into())),
        );
        let original = condition.clone();
        assert_eq!(rewrite_condition_words(&condition, black_to_blue()), Err(TextChangeDomainError::Condition));
        assert_eq!(condition, original);
        assert_eq!(rewrite_value_words(&Value::ManaSpentOnX(Color::Black), black_to_blue()), Err(TextChangeDomainError::Value));
        assert_eq!(rewrite_value_words(&Value::ManaSpentOnX(Color::Red), black_to_blue()).unwrap(), Value::ManaSpentOnX(Color::Red));
    }
}
