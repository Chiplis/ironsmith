use crate::effect::{Condition, Effect, Restriction, Value};
use crate::filter::Comparison;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};

fn comparison_mentions_iterated_player(comparison: &Comparison) -> bool {
    match comparison {
        Comparison::EqualExpr(value)
        | Comparison::NotEqualExpr(value)
        | Comparison::LessThanExpr(value)
        | Comparison::LessThanOrEqualExpr(value)
        | Comparison::GreaterThanExpr(value)
        | Comparison::GreaterThanOrEqualExpr(value) => value_mentions_iterated_player(value),
        _ => false,
    }
}

fn comparison_contains_pending_effect_metric(comparison: &Comparison) -> bool {
    match comparison {
        Comparison::EqualExpr(value)
        | Comparison::NotEqualExpr(value)
        | Comparison::LessThanExpr(value)
        | Comparison::LessThanOrEqualExpr(value)
        | Comparison::GreaterThanExpr(value)
        | Comparison::GreaterThanOrEqualExpr(value) => value_contains_pending_effect_metric(value),
        _ => false,
    }
}

pub(crate) fn object_filter_mentions_iterated_player(filter: &ObjectFilter) -> bool {
    [
        filter.controller.as_ref(),
        filter.cast_by.as_ref(),
        filter.owner.as_ref(),
        filter.targets_player.as_ref(),
        filter.targets_only_player.as_ref(),
        filter
            .attacking_player_or_planeswalker_controlled_by
            .as_ref(),
        filter.attached_to_player.as_ref(),
        filter.entered_battlefield_controller.as_ref(),
        filter.discarded_or_cycled_this_turn_by.as_ref(),
        filter.dealt_damage_to_player_this_turn.as_ref(),
    ]
    .into_iter()
    .flatten()
    .any(PlayerFilter::mentions_iterated_player)
        || filter
            .targets_object
            .as_deref()
            .is_some_and(object_filter_mentions_iterated_player)
        || filter
            .targets_only_object
            .as_deref()
            .is_some_and(object_filter_mentions_iterated_player)
        || filter
            .attached_to_object
            .as_deref()
            .is_some_and(object_filter_mentions_iterated_player)
        || filter
            .no_shared_creature_types_with
            .iter()
            .any(object_filter_mentions_iterated_player)
        || filter
            .any_of
            .iter()
            .any(object_filter_mentions_iterated_player)
        || [
            filter.power.as_ref(),
            filter.toughness.as_ref(),
            filter.total_power_toughness.as_ref(),
            filter.mana_value.as_ref(),
            filter.color_count.as_ref(),
        ]
        .into_iter()
        .flatten()
        .any(comparison_mentions_iterated_player)
}

fn object_filter_contains_pending_effect_metric(filter: &ObjectFilter) -> bool {
    filter
        .targets_object
        .as_deref()
        .is_some_and(object_filter_contains_pending_effect_metric)
        || filter
            .targets_only_object
            .as_deref()
            .is_some_and(object_filter_contains_pending_effect_metric)
        || filter
            .no_shared_creature_types_with
            .iter()
            .any(object_filter_contains_pending_effect_metric)
        || filter
            .any_of
            .iter()
            .any(object_filter_contains_pending_effect_metric)
        || [
            filter.power.as_ref(),
            filter.toughness.as_ref(),
            filter.total_power_toughness.as_ref(),
            filter.mana_value.as_ref(),
            filter.color_count.as_ref(),
        ]
        .into_iter()
        .flatten()
        .any(comparison_contains_pending_effect_metric)
}

pub(crate) fn choose_spec_mentions_iterated_player(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _) => choose_spec_mentions_iterated_player(spec),
        ChooseSpec::WithCountValue(spec, _, value) => {
            choose_spec_mentions_iterated_player(spec) || value_mentions_iterated_player(value)
        }
        ChooseSpec::Player(player)
        | ChooseSpec::PlayerOrPlaneswalker(player)
        | ChooseSpec::EachPlayer(player) => player.mentions_iterated_player(),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            object_filter_mentions_iterated_player(filter)
        }
        _ => false,
    }
}

fn choose_spec_contains_pending_effect_metric(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _) => choose_spec_contains_pending_effect_metric(spec),
        ChooseSpec::WithCountValue(spec, _, value) => {
            choose_spec_contains_pending_effect_metric(spec)
                || value_contains_pending_effect_metric(value)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            object_filter_contains_pending_effect_metric(filter)
        }
        _ => false,
    }
}

pub(crate) fn value_mentions_iterated_player(value: &Value) -> bool {
    match value {
        Value::SurfaceHinted { value, .. }
        | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => value_mentions_iterated_player(value),
        Value::Add(left, right) | Value::Min(left, right) => {
            value_mentions_iterated_player(left) || value_mentions_iterated_player(right)
        }
        Value::Count(filter)
        | Value::CountScaled(filter, _)
        | Value::GreatestCount(filter)
        | Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestToughness(filter)
        | Value::GreatestManaValue(filter)
        | Value::LeastPower(filter)
        | Value::LeastToughness(filter)
        | Value::LeastManaValue(filter)
        | Value::BasicLandTypesAmong(filter)
        | Value::CreatureTypesAmong(filter)
        | Value::CardTypesAmong(filter)
        | Value::ColorsAmong(filter)
        | Value::DistinctNames(filter)
        | Value::DistinctPowers(filter) => object_filter_mentions_iterated_player(filter),
        Value::PlayersWhoControlMoreThanYou { players, filter }
        | Value::PlayersWhoControlAtLeastMoreThanYou {
            players, filter, ..
        } => players.mentions_iterated_player() || object_filter_mentions_iterated_player(filter),
        Value::StaticAbilitiesAmong { filter, .. } => {
            object_filter_mentions_iterated_player(filter)
        }
        Value::PriorEffectMetric { query, .. } | Value::PendingPriorEffectMetric(query) => {
            query
                .filter
                .as_ref()
                .is_some_and(object_filter_mentions_iterated_player)
                || query
                    .player
                    .as_ref()
                    .is_some_and(PlayerFilter::mentions_iterated_player)
        }
        Value::PowerOf(spec)
        | Value::ToughnessOf(spec)
        | Value::ManaValueOf(spec)
        | Value::ManaSymbolsInManaCostOf { spec, .. }
        | Value::CountersOn(spec, _) => choose_spec_mentions_iterated_player(spec),
        Value::CreaturesDiedThisTurnControlledBy(player)
        | Value::CountPlayers(player)
        | Value::PartySize(player)
        | Value::LifeTotal(player)
        | Value::LifeTotalAsTurnBegan(player)
        | Value::LifeTotalDifference(player)
        | Value::UnspentMana(player)
        | Value::Speed(player)
        | Value::StartingLifeTotal(player)
        | Value::HalfLifeTotalRoundedUp(player)
        | Value::HalfLifeTotalRoundedDown(player)
        | Value::HalfStartingLifeTotalRoundedUp(player)
        | Value::HalfStartingLifeTotalRoundedDown(player)
        | Value::CardsInHand(player)
        | Value::CardsInLibrary(player)
        | Value::DevotionToChosenColor(player)
        | Value::LifeGainedThisTurn(player)
        | Value::LifeLostThisTurn(player)
        | Value::CardsDiscardedThisTurn(player)
        | Value::DamageDealtToPlayersThisTurn(player)
        | Value::NoncombatDamageDealtToPlayersThisTurn(player)
        | Value::MaxCardsDrawnThisTurn(player)
        | Value::MaxDiceRolledThisTurn(player)
        | Value::LandsEnteredBattlefieldThisTurn(player)
        | Value::MaxCardsInHand(player)
        | Value::CardsInGraveyard(player)
        | Value::SpellsCastThisTurn(player)
        | Value::SpellsCastBeforeThisTurn(player)
        | Value::CommanderCastCount(player)
        | Value::CardTypesInGraveyard(player)
        | Value::PlayerCounters(player, _)
        | Value::PlayerVoteCount(player) => player.mentions_iterated_player(),
        Value::NoncombatDamageDealtBySourcesControlledThisTurn { player, .. }
        | Value::Devotion { player, .. } => player.mentions_iterated_player(),
        Value::SpellsCastThisTurnMatching { player, filter, .. } => {
            player.mentions_iterated_player() || object_filter_mentions_iterated_player(filter)
        }
        Value::TurnHistoryCount(query) => {
            use ironsmith_core::TurnHistoryCount;

            match query {
                TurnHistoryCount::Died(filter)
                | TurnHistoryCount::EnteredBattlefield(filter)
                | TurnHistoryCount::MovedZones { filter, .. }
                | TurnHistoryCount::CountersPutOn { filter, .. } => {
                    object_filter_mentions_iterated_player(filter)
                }
                TurnHistoryCount::TokensCreated(player)
                | TurnHistoryCount::OpponentsAttacked(player)
                | TurnHistoryCount::PlayersDiscarded(player)
                | TurnHistoryCount::PlayersDealtDamage(player)
                | TurnHistoryCount::DiscardedOrCycled(player)
                | TurnHistoryCount::Cycled(player)
                | TurnHistoryCount::PlayersLostLife(player)
                | TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(player) => {
                    player.mentions_iterated_player()
                }
                TurnHistoryCount::PutIntoGraveyard { owner, .. } => {
                    owner.mentions_iterated_player()
                }
                TurnHistoryCount::Sacrificed { player, filter }
                | TurnHistoryCount::CreaturesAttackedWith { player, filter } => {
                    player.mentions_iterated_player()
                        || object_filter_mentions_iterated_player(filter)
                }
                TurnHistoryCount::PlayersDealtCombatDamageBy { players, sources } => {
                    players.mentions_iterated_player()
                        || object_filter_mentions_iterated_player(sources)
                }
                TurnHistoryCount::SpellsCast { player, filter, .. } => {
                    player.mentions_iterated_player()
                        || object_filter_mentions_iterated_player(filter)
                }
            }
        }
        _ => false,
    }
}

pub(crate) fn value_contains_pending_effect_metric(value: &Value) -> bool {
    match value {
        Value::PendingEffectMetric { .. }
        | Value::PendingEffectMetricOffset { .. }
        | Value::PendingPriorEffectMetric(_) => true,
        Value::SurfaceHinted { value, .. }
        | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => value_contains_pending_effect_metric(value),
        Value::Add(left, right) | Value::Min(left, right) => {
            value_contains_pending_effect_metric(left)
                || value_contains_pending_effect_metric(right)
        }
        Value::Count(filter)
        | Value::CountScaled(filter, _)
        | Value::GreatestCount(filter)
        | Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestToughness(filter)
        | Value::GreatestManaValue(filter)
        | Value::LeastPower(filter)
        | Value::LeastToughness(filter)
        | Value::LeastManaValue(filter)
        | Value::BasicLandTypesAmong(filter)
        | Value::CreatureTypesAmong(filter)
        | Value::CardTypesAmong(filter)
        | Value::ColorsAmong(filter)
        | Value::DistinctNames(filter)
        | Value::DistinctPowers(filter) => object_filter_contains_pending_effect_metric(filter),
        Value::PlayersWhoControlMoreThanYou { filter, .. }
        | Value::PlayersWhoControlAtLeastMoreThanYou { filter, .. } => {
            object_filter_contains_pending_effect_metric(filter)
        }
        Value::StaticAbilitiesAmong { filter, .. } => {
            object_filter_contains_pending_effect_metric(filter)
        }
        Value::PowerOf(spec)
        | Value::ToughnessOf(spec)
        | Value::ManaValueOf(spec)
        | Value::ManaSymbolsInManaCostOf { spec, .. }
        | Value::CountersOn(spec, _) => choose_spec_contains_pending_effect_metric(spec),
        Value::SpellsCastThisTurnMatching { filter, .. } => {
            object_filter_contains_pending_effect_metric(filter)
        }
        _ => false,
    }
}

fn anthem_count_mentions_iterated_player(count: &ironsmith_core::AnthemCountExpression) -> bool {
    use ironsmith_core::AnthemCountExpression;
    match count {
        AnthemCountExpression::MatchingFilter(filter)
        | AnthemCountExpression::GreatestManaValueAmong(filter)
        | AnthemCountExpression::AttachedToSource(filter)
        | AnthemCountExpression::AttachedToAffected(filter)
        | AnthemCountExpression::CountersAmong(filter, _)
        | AnthemCountExpression::DistinctCounterTypesAmong(filter)
        | AnthemCountExpression::BasicLandTypesAmong(filter)
        | AnthemCountExpression::CreatureTypesAmong(filter) => {
            object_filter_mentions_iterated_player(filter)
        }
        AnthemCountExpression::CommanderCastCount(player)
        | AnthemCountExpression::PlayerSpeed(player)
        | AnthemCountExpression::UnspentMana { player, .. } => player.mentions_iterated_player(),
        _ => false,
    }
}

pub(crate) fn condition_mentions_iterated_player(condition: &Condition) -> bool {
    use Condition::*;
    match condition {
        YouControl(filter)
        | OpponentControls(filter)
        | YouHaveCardInHandMatching(filter)
        | ObjectEnteredBattlefieldThisTurn(filter)
        | ObjectEnteredBattlefieldLastTurn(filter)
        | ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter)
        | SourceCrewedByExactly { filter, .. }
        | SourceMatches(filter)
        | AttachedToSourceMatches(filter)
        | TaggedObjectMatches(_, filter)
        | TargetMatches(filter) => object_filter_mentions_iterated_player(filter),
        AttachmentCount {
            attachment, host, ..
        } => {
            object_filter_mentions_iterated_player(attachment)
                || matches!(
                    host,
                    ironsmith_core::AttachmentConditionHost::Matching(filter)
                        if object_filter_mentions_iterated_player(filter)
                )
        }
        PlayerControls { player, filter }
        | PlayerHasAtLeast { player, filter, .. }
        | PlayerControlsExactly { player, filter, .. }
        | PlayerHasAtLeastWithDifferentPowers { player, filter, .. }
        | PlayerControlsMost { player, filter }
        | PlayerControlsMoreThanEachOtherPlayer { player, filter }
        | PlayerControlsMoreThanYou { player, filter }
        | AnOpponentControlsMoreThanPlayer { player, filter }
        | AnOpponentHasFewerThanPlayer { player, filter }
        | PlayerTaggedObjectMatches { player, filter, .. } => {
            player.mentions_iterated_player() || object_filter_mentions_iterated_player(filter)
        }
        PlayerControlsBasicLandTypesAmongLandsOrMore { player, .. }
        | PlayerLifeAtMostHalfStartingLifeTotal { player }
        | PlayerLifeLessThanHalfStartingLifeTotal { player }
        | PlayerHasLessLifeThanYou { player }
        | PlayerHasMoreLifeThanYou { player }
        | PlayerHasNoOpponentWithMoreLifeThan { player }
        | PlayerHasMoreLifeThanEachOtherPlayer { player }
        | PlayerIsMonarch { player }
        | PlayerHasInitiative { player }
        | PlayerHasCitysBlessing { player }
        | SourceIsRingBearer { player }
        | PlayerRingTemptedThisGameOrMore { player, .. }
        | PlayerCommittedCrimeThisTurn { player }
        | PlayerRolledResultThisTurn { player, .. }
        | PlayerCompletedDungeon { player, .. }
        | PlayerCardsInHandOrMore { player, .. }
        | PlayerCardsInHandOrFewer { player, .. }
        | PlayerCardsInHandAtTurnStartOrMore { player, .. }
        | PlayerCardsInHandAtTurnStartOrFewer { player, .. }
        | PlayerHasMoreCardsInHandThanYou { player }
        | PlayerHasMoreCardsInHandThanEachOtherPlayer { player }
        | PlayerHasPoisonCountersOrMore { player, .. }
        | PlayerCastSpellsThisTurnOrMore { player, .. }
        | PlayerTappedLandForManaThisTurn { player }
        | PlayerGainedLifeThisTurnOrMore { player, .. }
        | PlayerHadLandEnterBattlefieldThisTurn { player }
        | PlayerDescendedThisTurn { player }
        | PlayerHasCardTypesInGraveyardOrMore { player, .. }
        | PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, .. }
        | TaggedObjectIsTopOfLibrary { player, .. }
        | PlayerTaggedObjectEnteredBattlefieldThisTurn { player, .. }
        | PlayerOwnsCardNamedInZones { player, .. } => player.mentions_iterated_player(),
        ValueComparison { left, right, .. } => {
            value_mentions_iterated_player(left) || value_mentions_iterated_player(right)
        }
        CountComparison { count, .. } => anthem_count_mentions_iterated_player(count),
        CountParity { count, .. } => anthem_count_mentions_iterated_player(count),
        Not(inner) => condition_mentions_iterated_player(inner),
        And(left, right) | Or(left, right) => {
            condition_mentions_iterated_player(left) || condition_mentions_iterated_player(right)
        }
        _ => false,
    }
}

fn restriction_mentions_iterated_player(restriction: &Restriction) -> bool {
    use Restriction::*;
    match restriction {
        AdditionalLandPlays(player, _)
        | GainLife(player)
        | SearchLibraries(player)
        | CastSpellsOnlyAsSorcery(player)
        | ActivateNonManaAbilities(player)
        | DrawCards(player)
        | DrawExtraCards(player)
        | PoisonCounters(player)
        | LoseLife(player)
        | DamageCauseLifeLoss(player)
        | ChangeLifeTotal(player)
        | LoseGame(player)
        | WinGame(player)
        | BecomeMonarch(player)
        | LoseUnspentMana(player, _)
        | BeTargetedPlayer(player) => player.mentions_iterated_player(),
        CastSpellsMatching(player, filter) | CastMoreThanOneSpellEachTurn(player, filter) => {
            player.mentions_iterated_player() || object_filter_mentions_iterated_player(filter)
        }
        ActivateAbilitiesOf(filter)
        | ActivateTapAbilitiesOf(filter)
        | ActivateNonManaAbilitiesOf(filter)
        | Attack(filter)
        | AttackAlone(filter)
        | Block(filter)
        | MustBeBlocked(filter)
        | BlockAlone(filter)
        | Untap(filter)
        | BeBlocked(filter)
        | BeDestroyed(filter)
        | BeRegenerated(filter)
        | BeSacrificed(filter)
        | HaveCountersPlaced(filter)
        | BeTargeted(filter)
        | BeCountered(filter)
        | Transform(filter)
        | PhaseOut(filter)
        | AttackOrBlock(filter)
        | AttackOrBlockAlone(filter) => object_filter_mentions_iterated_player(filter),
        AttackPlayerOrPlaneswalkersControlledBy { attackers, player } => {
            object_filter_mentions_iterated_player(attackers) || player.mentions_iterated_player()
        }
        BlockSpecificAttacker { blockers, attacker }
        | MustBlockSpecificAttacker { blockers, attacker }
        | BeTargetedFrom(blockers, attacker) => {
            object_filter_mentions_iterated_player(blockers)
                || object_filter_mentions_iterated_player(attacker)
        }
        BeTargetedPlayerFrom(player, source) => {
            player.mentions_iterated_player() || object_filter_mentions_iterated_player(source)
        }
        PreventDamage => false,
    }
}

pub(crate) fn effect_mentions_iterated_player(effect: &Effect) -> bool {
    if effect
        .target_spec()
        .is_some_and(choose_spec_mentions_iterated_player)
    {
        return true;
    }
    if let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>()
        && restriction_mentions_iterated_player(&cant.restriction)
    {
        return true;
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>()
        && condition_mentions_iterated_player(&conditional.condition)
    {
        return true;
    }
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && (choose.chooser.mentions_iterated_player()
            || object_filter_mentions_iterated_player(&choose.filter)
            || choose
                .count_value
                .as_ref()
                .is_some_and(value_mentions_iterated_player))
    {
        return true;
    }
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && (draw.player.mentions_iterated_player() || value_mentions_iterated_player(&draw.count))
    {
        return true;
    }
    if let Some(exile_top) = effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
        && (exile_top.player.mentions_iterated_player()
            || value_mentions_iterated_player(&exile_top.count))
    {
        return true;
    }
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>()
        && value_mentions_iterated_player(&damage.amount)
    {
        return true;
    }
    if let Some(create) = effect.downcast_ref::<crate::effects::CreateTokenEffect>()
        && (create.controller.mentions_iterated_player()
            || create.next_end_step_player.mentions_iterated_player()
            || create
                .controller_target
                .as_ref()
                .is_some_and(choose_spec_mentions_iterated_player)
            || value_mentions_iterated_player(&create.count))
    {
        return true;
    }
    if let Some(copy) = effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>()
        && (copy.controller.mentions_iterated_player()
            || copy.next_end_step_player.mentions_iterated_player()
            || value_mentions_iterated_player(&copy.count))
    {
        return true;
    }
    if let Some(search) = effect.downcast_ref::<crate::effects::SearchLibraryEffect>()
        && (search.chooser.mentions_iterated_player()
            || search.player.mentions_iterated_player()
            || object_filter_mentions_iterated_player(&search.filter)
            || search
                .library_position_from_top
                .as_ref()
                .is_some_and(value_mentions_iterated_player))
    {
        return true;
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<Effect>>()
        && may
            .decider
            .as_ref()
            .is_some_and(PlayerFilter::mentions_iterated_player)
    {
        return true;
    }
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect<Effect>>()
        && unless_pays.player.mentions_iterated_player()
    {
        return true;
    }
    if let Some(unless_action) = effect.downcast_ref::<crate::effects::UnlessActionEffect<Effect>>()
        && unless_action.player.mentions_iterated_player()
    {
        return true;
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect<Effect>>()
        && for_players.filter.mentions_iterated_player()
    {
        return true;
    }
    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>()
        && object_filter_mentions_iterated_player(&for_each.filter)
    {
        return true;
    }
    if let Some(delayed) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
        && (delayed.controller.mentions_iterated_player()
            || delayed
                .target_filter
                .as_ref()
                .is_some_and(object_filter_mentions_iterated_player))
    {
        return true;
    }
    if let Some(delayed) =
        effect.downcast_ref::<crate::effects::ScheduleEffectsWhenTaggedLeavesEffect>()
        && delayed.controller.mentions_iterated_player()
    {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        found |= effect_mentions_iterated_player(child);
    });
    found
}

pub(crate) fn effect_contains_pending_effect_metric(effect: &Effect) -> bool {
    if effect
        .target_spec()
        .is_some_and(choose_spec_contains_pending_effect_metric)
    {
        return true;
    }
    if effect.as_choose_mode().is_some_and(|modal| {
        value_contains_pending_effect_metric(&modal.min)
            || value_contains_pending_effect_metric(&modal.max)
            || value_contains_pending_effect_metric(&modal.choose_count)
            || value_contains_pending_effect_metric(&modal.min_choose_count)
    }) {
        return true;
    }
    macro_rules! value_field {
        ($type:ty, $field:ident) => {
            if let Some(value_effect) = effect.downcast_ref::<$type>()
                && value_contains_pending_effect_metric(&value_effect.$field)
            {
                return true;
            }
        };
    }
    value_field!(crate::effects::DealDamageEffect, amount);
    value_field!(crate::effects::DrawCardsEffect, count);
    value_field!(crate::effects::PutCountersEffect, amount);
    value_field!(crate::effects::RemoveCountersEffect, count);
    value_field!(crate::effects::RepeatEffectsEffect, count);
    value_field!(crate::effects::CreateTokenEffect, count);
    value_field!(crate::effects::CreateTokenCopyEffect, count);
    value_field!(crate::effects::DiscardEffect, count);
    value_field!(crate::effects::MillEffect, count);
    value_field!(crate::effects::ScryEffect, count);
    value_field!(crate::effects::SurveilEffect, count);
    value_field!(crate::effects::FatesealEffect, count);
    value_field!(crate::effects::ExileTopOfLibraryEffect, count);
    value_field!(crate::effects::InvestigateEffect, count);
    value_field!(crate::effects::GainLifeEffect, amount);
    value_field!(crate::effects::LoseLifeEffect, amount);
    value_field!(crate::effects::SetLifeTotalEffect, amount);
    value_field!(crate::effects::PoisonCountersEffect, count);
    value_field!(crate::effects::AdditionalLandPlaysEffect, count);
    value_field!(crate::effects::PreventDamageEffect, amount);
    value_field!(crate::effects::AddManaOfLandProducedTypesEffect, amount);
    value_field!(crate::effects::ConniveEffect, count);
    value_field!(crate::effects::RemoveUpToCountersEffect, max_count);
    if let Some(create) = effect.downcast_ref::<crate::effects::IncubateEffect>()
        && (value_contains_pending_effect_metric(&create.amount)
            || value_contains_pending_effect_metric(&create.count))
    {
        return true;
    }
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && choose
            .count_value
            .as_ref()
            .is_some_and(value_contains_pending_effect_metric)
    {
        return true;
    }
    if let Some(search) = effect.downcast_ref::<crate::effects::SearchLibraryEffect>()
        && search
            .library_position_from_top
            .as_ref()
            .is_some_and(value_contains_pending_effect_metric)
    {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        found |= effect_contains_pending_effect_metric(child);
    });
    found
}

pub(crate) fn effects_contain_pending_effect_metric(effects: &[Effect]) -> bool {
    effects.iter().any(effect_contains_pending_effect_metric)
}
