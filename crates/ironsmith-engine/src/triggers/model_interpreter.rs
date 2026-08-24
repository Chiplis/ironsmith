#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerModelConversionError {
    pub detail: String,
}

impl std::fmt::Display for TriggerModelConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported trigger model: {}", self.detail)
    }
}

impl std::error::Error for TriggerModelConversionError {}

fn convert_count_mode(
    mode: ironsmith_core::trigger_model::CountMode,
) -> crate::triggers::CountMode {
    match mode {
        ironsmith_core::trigger_model::CountMode::One => crate::triggers::CountMode::Each,
        ironsmith_core::trigger_model::CountMode::OneOrMore => {
            crate::triggers::CountMode::OneOrMore
        }
    }
}

fn convert_zone_change_trigger(
    trigger: ironsmith_core::trigger_model::ZoneChangeTrigger,
) -> crate::triggers::Trigger {
    let mut out = crate::triggers::zone_changes::ZoneChangeTrigger::new();
    if let Some(from_excluded) = trigger.from_excluded {
        out = out.from(crate::triggers::zone_changes::ZonePattern::AnyExcept(
            from_excluded,
        ));
    } else if let Some(from_zones) = trigger.from_zones {
        if from_zones.len() == 1 {
            out = out.from(from_zones[0]);
        } else {
            out = out.from(crate::triggers::zone_changes::ZonePattern::OneOf(
                from_zones,
            ));
        }
    } else if let Some(from) = trigger.from {
        out = out.from(from);
    }
    if let Some(to_excluded) = trigger.to_excluded {
        out = out.to(crate::triggers::zone_changes::ZonePattern::AnyExcept(
            to_excluded,
        ));
    } else if let Some(to) = trigger.to {
        out = out.to(to);
    }
    if let Some(filter) = trigger.filter {
        out = out.filter(filter);
    }
    if trigger.this {
        out = out.this();
    }
    if let Some(surface) = trigger.this_surface {
        out = out.this_surface(surface);
    }
    out = out.this_subject_number(trigger.this_subject_number);
    out = out.count(convert_count_mode(trigger.count));
    out = out.cause_filter(trigger.cause_filter);
    if let Some(origin_condition) = trigger.origin_condition {
        out = out.origin_condition(origin_condition);
    }
    if let Some(during_turn) = trigger.during_turn {
        out = out.during_turn(during_turn);
    }
    if matches!(
        trigger.timing,
        Some(ironsmith_core::TriggerTimingRestriction::DuringCombat)
    ) {
        out = out.during_combat();
    }
    if let Some(graveyard_surface) = trigger.graveyard_surface {
        out = out.graveyard_surface(graveyard_surface);
    }
    crate::triggers::Trigger::new(out)
}

fn convert_counter_put_on_trigger(
    trigger: ironsmith_core::trigger_model::CounterPutOnTrigger,
) -> crate::triggers::Trigger {
    let mut out = crate::triggers::CounterPutOnTrigger::new(trigger.filter);
    if let Some(counter_type) = trigger.counter_type {
        out = out.counter_type(counter_type);
    }
    if let Some(source_controller) = trigger.source_controller {
        out = out.source_controller(source_controller);
    }
    if trigger.include_players {
        out = out.include_players();
    }
    out = out.count(convert_count_mode(trigger.count));
    crate::triggers::Trigger::new(out)
}

fn convert_player_gets_counters_trigger(
    trigger: ironsmith_core::trigger_model::PlayerGetsCountersTrigger,
) -> crate::triggers::Trigger {
    let mut out = crate::triggers::PlayerGetsCountersTrigger::new(trigger.player);
    if let Some(counter_type) = trigger.counter_type {
        out = out.counter_type(counter_type);
    }
    out = out.count(convert_count_mode(trigger.count));
    crate::triggers::Trigger::new(out)
}

fn convert_counter_removed_from_trigger(
    trigger: ironsmith_core::trigger_model::CounterRemovedFromTrigger,
) -> crate::triggers::Trigger {
    let mut out = crate::triggers::CounterRemovedFromTrigger::new(trigger.filter);
    if let Some(counter_type) = trigger.counter_type {
        out = out.counter_type(counter_type);
    }
    if trigger.last {
        out = out.last();
    }
    if trigger.one_or_more {
        out = out.one_or_more();
    }
    if trigger.caused_by_source {
        out = out.caused_by_source();
    }
    crate::triggers::Trigger::new(out)
}

pub(crate) fn interpret_trigger_model(
    trigger: ironsmith_core::trigger_model::Trigger,
) -> Result<crate::triggers::Trigger, TriggerModelConversionError> {
    use ironsmith_core::trigger_model::{DamagedBySource, TriggerKind};

    let intro_surface = trigger.intro_surface;
    let interpreted = match trigger.kind {
        TriggerKind::StateBased { display } => crate::triggers::Trigger::state_based(display),
        TriggerKind::AnyOf(branches) => {
            let interpreted_branches = branches
                .into_iter()
                .map(interpret_trigger_model)
                .collect::<Result<Vec<_>, _>>()?;
            crate::triggers::Trigger::new(crate::triggers::AnyOfTrigger {
                branches: interpreted_branches,
            })
        }
        TriggerKind::ConditionQualified {
            trigger,
            condition,
            surface,
            stun_counter_reminder_surface,
        } => crate::triggers::Trigger::condition_qualified(
            interpret_trigger_model(*trigger)?,
            condition,
            surface,
            stun_counter_reminder_surface,
        ),
        TriggerKind::ThisAttacks => crate::triggers::Trigger::this_attacks(),
        TriggerKind::ThisAttacksWhileYouControl { filter } => {
            crate::triggers::Trigger::this_attacks_while_you_control(filter)
        }
        TriggerKind::ThisAndAnotherAttackDifferentPlayers => {
            crate::triggers::Trigger::this_and_another_attack_different_players()
        }
        TriggerKind::ThisAttacksPlayerWhoControlsAtLeast { count, filter } => {
            crate::triggers::Trigger::this_attacks_player_who_controls_at_least(count, filter)
        }
        TriggerKind::ThisAttacksPlayerWithMostLife => {
            crate::triggers::Trigger::this_attacks_player_with_most_life()
        }
        TriggerKind::ThisAttacksWithGreaterPower => {
            crate::triggers::Trigger::this_attacks_with_greater_power()
        }
        TriggerKind::ThisAttacksWithNOthers {
            count,
            display_subject,
            other_filter,
            other_surface,
        } => crate::triggers::Trigger::this_attacks_with_n_others_display_subject_filter_and_other_surface(
            count,
            display_subject,
            other_filter,
            other_surface,
        ),
        TriggerKind::ThisAttacksWithExactNOthers { count } => {
            crate::triggers::Trigger::this_attacks_with_exact_n_others(count)
        }
        TriggerKind::ThisAttacksAndIsntBlocked => {
            crate::triggers::Trigger::this_attacks_and_isnt_blocked()
        }
        TriggerKind::ThisAttacksWhileSaddled => {
            crate::triggers::Trigger::this_attacks_while_saddled()
        }
        TriggerKind::Attacks { filter } => crate::triggers::Trigger::attacks(filter),
        TriggerKind::AttacksAndIsntBlocked { filter } => {
            crate::triggers::Trigger::attacks_and_isnt_blocked(filter)
        }
        TriggerKind::AttacksWhileSaddled { filter } => {
            crate::triggers::Trigger::attacks_while_saddled(filter)
        }
        TriggerKind::AttacksOneOrMore { filter } => {
            crate::triggers::Trigger::attacks_one_or_more(filter)
        }
        TriggerKind::PlayersAttackedOneOrMore { player_filter } => {
            crate::triggers::Trigger::players_attacked_one_or_more(player_filter)
        }
        TriggerKind::PlayerAttacksOneOrMore { attacker, target } => {
            crate::triggers::Trigger::player_attacks_one_or_more(attacker, target)
        }
        TriggerKind::AttacksOneOrMoreWithMinTotal {
            filter,
            min_total_attackers,
        } => crate::triggers::Trigger::attacks_one_or_more_with_min_total(
            filter,
            min_total_attackers,
        ),
        TriggerKind::AttacksOneOrMoreWithExactTotal {
            filter,
            total_attackers,
        } => {
            crate::triggers::Trigger::attacks_one_or_more_with_exact_total(filter, total_attackers)
        }
        TriggerKind::AttacksOneOrMoreWithAggregate {
            filter,
            metric,
            comparison,
        } => crate::triggers::Trigger::attacks_one_or_more_with_aggregate(
            filter,
            metric,
            comparison,
        ),
        TriggerKind::AttacksAlone { filter } => crate::triggers::Trigger::attacks_alone(filter),
        TriggerKind::AttacksYou { filter } => crate::triggers::Trigger::attacks_you(filter),
        TriggerKind::AttacksYouOneOrMore { filter } => {
            crate::triggers::Trigger::attacks_you_one_or_more(filter)
        }
        TriggerKind::ThisBlocks => crate::triggers::Trigger::this_blocks(),
        TriggerKind::ThisBlocksObject {
            filter,
            min_blocked_objects,
        } => match min_blocked_objects {
            Some(minimum) => {
                crate::triggers::Trigger::this_blocks_objects_with_minimum(filter, minimum)
            }
            None => crate::triggers::Trigger::this_blocks_object(filter),
        },
        TriggerKind::Blocks { filter } => crate::triggers::Trigger::blocks(filter),
        TriggerKind::BlocksOneOrMore { filter } => {
            crate::triggers::Trigger::blocks_one_or_more(filter)
        }
        TriggerKind::BlocksObjectWithLesserPower { blocker, blocked } => {
            crate::triggers::Trigger::blocks_object_with_lesser_power(blocker, blocked)
        }
        TriggerKind::ThisBecomesBlocked => crate::triggers::Trigger::this_becomes_blocked(),
        TriggerKind::BecomesBlocked { filter } => crate::triggers::Trigger::becomes_blocked(filter),
        TriggerKind::ThisBecomesBlockedByObject { filter } => {
            crate::triggers::Trigger::this_becomes_blocked_by_object(filter)
        }
        TriggerKind::BecomesBlockedByObjectWithLesserPower { blocked, blocker } => {
            crate::triggers::Trigger::becomes_blocked_by_object_with_lesser_power(blocked, blocker)
        }
        TriggerKind::ThisDies => crate::triggers::Trigger::this_dies(),
        TriggerKind::ThisDiesOrIsExiled => crate::triggers::Trigger::this_dies_or_is_exiled(),
        TriggerKind::ThisDiesOrIsExiledWithSurface { surface } => {
            crate::triggers::Trigger::this_dies_or_is_exiled_with_surface(surface)
        }
        TriggerKind::ThisLeavesBattlefield => crate::triggers::Trigger::this_leaves_battlefield(),
        TriggerKind::ThisPhasesOut => crate::triggers::Trigger::this_phases_out(),
        TriggerKind::ThisMutates => crate::triggers::Trigger::this_mutates(),
        TriggerKind::LeavesBattlefield { filter } => {
            crate::triggers::Trigger::leaves_battlefield(filter)
        }
        TriggerKind::ThisBecomesMonstrous => crate::triggers::Trigger::this_becomes_monstrous(),
        TriggerKind::BecomesTapped => crate::triggers::Trigger::becomes_tapped(),
        TriggerKind::PermanentBecomesTapped { filter } => {
            crate::triggers::Trigger::permanent_becomes_tapped(filter)
        }
        TriggerKind::BecomesUntapped => crate::triggers::Trigger::becomes_untapped(),
        TriggerKind::ThisIsTurnedFaceUp => crate::triggers::Trigger::this_is_turned_face_up(),
        TriggerKind::TurnedFaceUp { filter } => crate::triggers::Trigger::turned_face_up(filter),
        TriggerKind::BecomesTargeted => crate::triggers::Trigger::becomes_targeted(),
        TriggerKind::BecomesTargetedObject { filter } => {
            crate::triggers::Trigger::becomes_targeted_object(filter)
        }
        TriggerKind::BecomesTargetedBySpell { filter } => {
            crate::triggers::Trigger::becomes_targeted_by_spell(filter)
        }
        TriggerKind::BecomesTargetedByStackObject { filter } => {
            crate::triggers::Trigger::becomes_targeted_by_stack_object(filter)
        }
        TriggerKind::BecomesTargetedObjectByStackObject { target, source } => {
            crate::triggers::Trigger::becomes_targeted_object_by_stack_object(target, source)
        }
        TriggerKind::BecomesTargetedBySourceController { target, controller } => {
            crate::triggers::Trigger::becomes_targeted_by_source_controller(target, controller)
        }
        TriggerKind::PlayerOrObjectBecomesTargetedBySourceController {
            player,
            object,
            controller,
        } => crate::triggers::Trigger::player_or_object_becomes_targeted_by_source_controller(
            player, object, controller,
        ),
        TriggerKind::ThisDealsDamage => crate::triggers::Trigger::this_deals_damage(),
        TriggerKind::ThisDealsDamageToPlayer { player, amount } => {
            crate::triggers::Trigger::this_deals_damage_to_player(player, amount)
        }
        TriggerKind::ThisDealsDamageTo { filter } => {
            crate::triggers::Trigger::this_deals_damage_to(filter)
        }
        TriggerKind::ThisDealsCombatDamage => crate::triggers::Trigger::this_deals_combat_damage(),
        TriggerKind::ThisDealsCombatDamageTo { filter } => {
            crate::triggers::Trigger::this_deals_combat_damage_to(filter)
        }
        TriggerKind::ThisDealsCombatDamageToPlayer {
            player,
            source_surface,
        } => match source_surface {
            Some(surface) => {
                crate::triggers::Trigger::this_deals_combat_damage_to_player_with_surface(
                    player, surface,
                )
            }
            None => crate::triggers::Trigger::this_deals_combat_damage_to_player(player),
        },
        TriggerKind::DealsDamage {
            filter,
            source_surface,
        } => crate::triggers::Trigger::deals_damage_with_source_surface(filter, source_surface),
        TriggerKind::DealsDamageTo {
            source,
            target,
            source_surface,
        } => crate::triggers::Trigger::deals_damage_to_with_source_surface(
            source,
            target,
            source_surface,
        ),
        TriggerKind::DealsDamageToPlayer {
            source,
            player,
            source_surface,
        } => crate::triggers::Trigger::deals_damage_to_player_with_source_surface(
            source,
            player,
            source_surface,
        ),
        TriggerKind::DealsExactDamageToObjectOrPlayer {
            source,
            object,
            player,
            player_first,
            amount,
            source_surface,
        } => crate::triggers::Trigger::deals_exact_damage_to_object_or_player_with_source_surface(
            source,
            object,
            player,
            player_first,
            amount,
            source_surface,
        ),
        TriggerKind::DealsNoncombatDamageToPlayer {
            source,
            player,
            source_surface,
            damaged_player_one_or_more,
            during_turn,
        } => {
            let mut trigger = crate::triggers::DealsDamageTrigger::noncombat_to_player(
                source,
                player,
                source_surface,
            );
            if damaged_player_one_or_more {
                trigger = trigger.damaged_player_one_or_more();
            }
            if let Some(during_turn) = during_turn {
                trigger = trigger.during_turn(during_turn);
            }
            crate::triggers::Trigger::new(trigger)
        }
        TriggerKind::DealsCombatDamage { filter } => {
            crate::triggers::Trigger::deals_combat_damage(filter)
        }
        TriggerKind::DealsCombatDamageTo { source, target } => {
            crate::triggers::Trigger::deals_combat_damage_to(source, target)
        }
        TriggerKind::DealsCombatDamageToPlayer {
            source,
            player,
            one_or_more,
        } => {
            if one_or_more {
                crate::triggers::Trigger::deals_combat_damage_to_player_one_or_more(source, player)
            } else {
                crate::triggers::Trigger::deals_combat_damage_to_player(source, player)
            }
        }
        TriggerKind::PlayerPlaysLand { player, filter } => {
            crate::triggers::Trigger::player_plays_land(player, filter)
        }
        TriggerKind::PlayerGivesGift { player } => {
            crate::triggers::Trigger::player_gives_gift(player)
        }
        TriggerKind::PlayerSearchesLibrary { player } => {
            crate::triggers::Trigger::player_searches_library(player)
        }
        TriggerKind::PlayerShufflesLibrary {
            player,
            caused_by_effect,
            source_controller_shuffles,
        } => crate::triggers::Trigger::player_shuffles_library(
            player,
            caused_by_effect,
            source_controller_shuffles,
        ),
        TriggerKind::PlayerTapsForMana { player, filter } => {
            crate::triggers::Trigger::player_taps_for_mana(player, filter)
        }
        TriggerKind::PlayerRollsResult { player, result } => {
            crate::triggers::Trigger::player_rolls_result(player, result)
        }
        TriggerKind::PlayerRollsHighestNaturalResult { player } => {
            crate::triggers::Trigger::player_rolls_highest_natural_result(player)
        }
        TriggerKind::PlayerRollsDie {
            player,
            one_or_more,
        } => crate::triggers::Trigger::player_rolls_die_with_surface(player, one_or_more),
        TriggerKind::PlayerCoinFlipResult { player, won } => {
            crate::triggers::Trigger::player_coin_flip_result(player, won)
        }
        TriggerKind::AbilityActivatedQualified {
            activator,
            filter,
            non_mana_only,
            loyalty_only,
            activation_cost_has_tap,
        } => crate::triggers::Trigger::ability_activated_qualified_with_activation_cost_tap(
            activator,
            filter,
            non_mana_only,
            loyalty_only,
            activation_cost_has_tap,
        ),
        TriggerKind::AbilityTriggered {
            another,
            source_filter,
            caused_by_source_entering,
        } => {
            if source_filter.is_some() || caused_by_source_entering {
                crate::triggers::Trigger::ability_triggered_qualified(
                    another,
                    source_filter,
                    caused_by_source_entering,
                )
            } else if another {
                crate::triggers::Trigger::another_ability_triggers()
            } else {
                crate::triggers::Trigger::ability_triggers()
            }
        }
        TriggerKind::IsDealtDamage {
            target,
            combat_only,
            noncombat_only,
            excess_only,
        } => {
            if excess_only && noncombat_only {
                crate::triggers::Trigger::is_dealt_excess_noncombat_damage(target)
            } else if combat_only {
                crate::triggers::Trigger::is_dealt_combat_damage(target)
            } else {
                crate::triggers::Trigger::is_dealt_damage(target)
            }
        }
        TriggerKind::YouGainLife => crate::triggers::Trigger::you_gain_life(),
        TriggerKind::YouGainLifeCausedBy { source } => {
            crate::triggers::Trigger::you_gain_life_caused_by(source)
        }
        TriggerKind::YouGainLifeDuringTurn { during_turn } => {
            crate::triggers::Trigger::you_gain_life_during_turn(during_turn)
        }
        TriggerKind::PlayerLosesLife { player } => {
            crate::triggers::Trigger::player_loses_life(player)
        }
        TriggerKind::PlayersLoseLifeOneOrMore { player } => {
            crate::triggers::Trigger::players_lose_life_one_or_more(player)
        }
        TriggerKind::OpponentsEachLoseExactLife { amount } => {
            crate::triggers::Trigger::opponents_each_lose_exact_life(amount)
        }
        TriggerKind::PlayerLosesGame { player } => {
            crate::triggers::Trigger::player_loses_game(player)
        }
        TriggerKind::PlayerLosesLifeDuringTurn {
            player,
            during_turn,
        } => crate::triggers::Trigger::player_loses_life_during_turn(player, during_turn),
        TriggerKind::SpellCountered { filter, controller } => {
            crate::triggers::Trigger::spell_countered(filter, controller)
        }
        TriggerKind::YouDrawCard => crate::triggers::Trigger::you_draw_card(),
        TriggerKind::PlayerDrawsCard { player } => {
            crate::triggers::Trigger::player_draws_card(player)
        }
        TriggerKind::PlayerDrawsCardNotDuringTurn {
            player,
            during_turn,
        } => crate::triggers::Trigger::player_draws_card_not_during_turn(player, during_turn),
        TriggerKind::PlayerDrawsCardExceptFirstInDrawStep { player } => {
            crate::triggers::Trigger::player_draws_card_except_first_in_draw_step(player)
        }
        TriggerKind::PlayerDrawsNthCardEachTurn {
            player,
            card_number,
        } => crate::triggers::Trigger::player_draws_nth_card_each_turn(player, card_number),
        TriggerKind::PlayerDrawsNumberedCardsEachTurn {
            player,
            card_numbers,
        } => crate::triggers::Trigger::player_draws_numbered_cards_each_turn(player, card_numbers),
        TriggerKind::PlayerDiscardsCardCausedByController {
            player,
            filter,
            controller,
            effect_like_only,
        } => crate::triggers::Trigger::player_discards_card_caused_by_controller(
            player,
            filter,
            controller,
            effect_like_only,
        ),
        TriggerKind::PlayerDiscardsCard {
            player,
            filter,
            one_or_more,
        } => {
            if one_or_more {
                crate::triggers::Trigger::player_discards_cards(player, filter)
            } else {
                crate::triggers::Trigger::player_discards_card(player, filter)
            }
        }
        TriggerKind::PlayerRevealsCard {
            player,
            filter,
            from_source,
        } => crate::triggers::Trigger::player_reveals_card(player, filter, from_source),
        TriggerKind::PlayerSacrifices {
            player,
            filter,
            one_or_more_surface,
        } => crate::triggers::Trigger::player_sacrifices_with_surface(
            player,
            filter,
            one_or_more_surface,
        ),
        TriggerKind::PermanentSacrificed { filter } => {
            crate::triggers::Trigger::permanent_sacrificed(filter)
        }
        TriggerKind::PermanentDestroyed { filter } => {
            crate::triggers::Trigger::permanent_destroyed(filter)
        }
        TriggerKind::TokensCreated {
            player,
            filter,
            one_or_more,
        } => crate::triggers::Trigger::tokens_created(player, filter, one_or_more),
        TriggerKind::Dies { filter } => crate::triggers::Trigger::dies(filter),
        TriggerKind::PutIntoGraveyard { filter } => {
            crate::triggers::Trigger::put_into_graveyard(filter)
        }
        TriggerKind::CardsLeaveYourGraveyard {
            filter,
            one_or_more,
            during_your_turn,
        } => crate::triggers::Trigger::cards_leave_your_graveyard(
            filter,
            one_or_more,
            during_your_turn,
        ),
        TriggerKind::DiesCreatureDealtDamageByThisTurn { victim, damager } => match damager {
            DamagedBySource::ThisCreature => {
                crate::triggers::Trigger::creature_dealt_damage_by_this_creature_this_turn_dies(
                    victim,
                )
            }
            DamagedBySource::EquippedCreature => {
                crate::triggers::Trigger::creature_dealt_damage_by_equipped_creature_this_turn_dies(
                    victim,
                )
            }
            DamagedBySource::EnchantedCreature => {
                crate::triggers::Trigger::creature_dealt_damage_by_enchanted_creature_this_turn_dies(
                    victim,
                )
            }
        },
        TriggerKind::DiesCreatureDealtDamageByFilteredSourceThisTurn {
            victim,
            damager_filter,
        } => crate::triggers::Trigger::creature_dealt_damage_by_filtered_source_this_turn_dies(
            victim,
            damager_filter,
        ),
        TriggerKind::SpellCastQualified {
            filter,
            mana_source_filter,
            caster,
            timing,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        } => crate::triggers::Trigger::spell_cast_qualified_with_mana_source(
            filter,
            mana_source_filter,
            caster,
            timing,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        ),
        TriggerKind::SpellCast { filter, caster } => {
            crate::triggers::Trigger::spell_cast(filter, caster)
        }
        TriggerKind::SpellCastSameNameCardInZone {
            filter,
            caster,
            zone,
            owner,
        } => crate::triggers::Trigger::spell_cast_same_name_card_in_zone(
            filter, caster, zone, owner,
        ),
        TriggerKind::NthSpellOfTurnCast { spell_number } => {
            crate::triggers::Trigger::nth_spell_of_turn_cast(spell_number)
        }
        TriggerKind::SpellCopied { filter, copier } => {
            crate::triggers::Trigger::spell_copied(filter, copier)
        }
        TriggerKind::EntersBattlefield {
            filter,
            cause_filter,
            count,
            tapped,
        } => match (count, tapped) {
            (ironsmith_core::trigger_model::CountMode::One, None) => {
                crate::triggers::Trigger::enters_battlefield(filter, cause_filter)
            }
            (ironsmith_core::trigger_model::CountMode::OneOrMore, None) => {
                crate::triggers::Trigger::enters_battlefield_one_or_more(filter, cause_filter)
            }
            (ironsmith_core::trigger_model::CountMode::One, Some(true)) => {
                crate::triggers::Trigger::enters_battlefield_tapped(filter, cause_filter)
            }
            (ironsmith_core::trigger_model::CountMode::One, Some(false)) => {
                crate::triggers::Trigger::enters_battlefield_untapped(filter, cause_filter)
            }
            (ironsmith_core::trigger_model::CountMode::OneOrMore, Some(tapped)) => {
                return Err(TriggerModelConversionError {
                    detail: format!("one-or-more tapped ETB trigger tapped={tapped}"),
                });
            }
        },
        TriggerKind::BeginningOfUpkeep { player } => {
            crate::triggers::Trigger::beginning_of_upkeep(player)
        }
        TriggerKind::BeginningOfDrawStep { player } => {
            crate::triggers::Trigger::beginning_of_draw_step(player)
        }
        TriggerKind::BeginningOfCombat { player } => {
            crate::triggers::Trigger::beginning_of_combat(player)
        }
        TriggerKind::EndOfCombat => crate::triggers::Trigger::end_of_combat(),
        TriggerKind::BeginningOfEndStep { player, surface } => match surface {
            ironsmith_core::trigger_model::EndStepSurface::Definite => {
                crate::triggers::Trigger::beginning_of_the_end_step()
            }
            ironsmith_core::trigger_model::EndStepSurface::Each => {
                crate::triggers::Trigger::beginning_of_end_step(player)
            }
            ironsmith_core::trigger_model::EndStepSurface::Monarch => {
                crate::triggers::Trigger::beginning_of_monarch_end_step()
            }
        },
        TriggerKind::BeginningOfMainPhase { player, surface } => {
            crate::triggers::Trigger::beginning_of_main_phase_with_surface(player, surface)
        }
        TriggerKind::BeginningOfPrecombatMainPhase { player } => {
            crate::triggers::Trigger::beginning_of_precombat_main_phase(player)
        }
        TriggerKind::BeginningOfPostcombatMainPhase { player, surface } => {
            crate::triggers::Trigger::beginning_of_postcombat_main_phase_with_surface(
                player, surface,
            )
        }
        TriggerKind::DayNightChanged => crate::triggers::Trigger::day_night_changed(),
        TriggerKind::ThisEntersBattlefield => crate::triggers::Trigger::this_enters_battlefield(),
        TriggerKind::ThisTransforms { destination_name } => {
            crate::triggers::Trigger::transforms_with_destination(destination_name)
        }
        TriggerKind::ThisTransformsWithSurface {
            surface,
            destination_name,
        } => crate::triggers::Trigger::transforms_with_surface_and_destination(
            surface,
            destination_name,
        ),
        TriggerKind::YouCastThisSpell => crate::triggers::Trigger::you_cast_this_spell(),
        TriggerKind::KeywordActionMatchingObject {
            action,
            player,
            filter,
        } => crate::triggers::Trigger::keyword_action_matching_object(action, player, filter),
        TriggerKind::KeywordActionMatchingObjectDuringYourTurn {
            action,
            player,
            filter,
        } => crate::triggers::Trigger::keyword_action_matching_object_during_your_turn(
            action, player, filter,
        ),
        TriggerKind::KeywordActionMatchingTaggedObject {
            action,
            player,
            source_filter,
            object_tag,
            object_filter,
            during_your_main_phase,
        } => {
            if during_your_main_phase {
                crate::triggers::Trigger::keyword_action_matching_source_and_tagged_object_during_your_main_phase(
                    action,
                    player,
                    source_filter,
                    object_tag,
                    object_filter,
                )
            } else {
                crate::triggers::Trigger::keyword_action_matching_source_and_tagged_object(
                    action,
                    player,
                    source_filter,
                    object_tag,
                    object_filter,
                )
            }
        }
        TriggerKind::KeywordAction { action, player } => {
            crate::triggers::Trigger::keyword_action(action, player)
        }
        TriggerKind::KeywordActionDuringYourTurn { action, player } => {
            crate::triggers::Trigger::keyword_action_during_your_turn(action, player)
        }
        TriggerKind::KeywordActionFromSource { action, player } => {
            crate::triggers::Trigger::keyword_action_from_source(action, player)
        }
        TriggerKind::WinsClash { player, surface } => {
            crate::triggers::Trigger::wins_clash_with_surface(player, surface)
        }
        TriggerKind::Expend { amount, player } => crate::triggers::Trigger::expend(amount, player),
        TriggerKind::SagaChapter { chapters } => crate::triggers::Trigger::saga_chapter(chapters),
        TriggerKind::FinalChapterAbilityResolved { filter } => {
            crate::triggers::Trigger::final_chapter_ability_resolved(filter)
        }
        TriggerKind::Custom { id, label } => {
            let id: &'static str = Box::leak(id.into_boxed_str());
            crate::triggers::Trigger::custom(id, label)
        }
        TriggerKind::Either { left, right } => crate::triggers::Trigger::either(
            interpret_trigger_model(*left)?,
            interpret_trigger_model(*right)?,
        ),
        TriggerKind::ZoneChange(zone_change) => convert_zone_change_trigger(zone_change),
        TriggerKind::PlayerGetsCounters(player_gets_counters) => {
            convert_player_gets_counters_trigger(player_gets_counters)
        }
        TriggerKind::CounterPutOn(counter_put_on) => convert_counter_put_on_trigger(counter_put_on),
        TriggerKind::NthCounterPutOn {
            filter,
            counter_type,
            counter_number,
        } => crate::triggers::Trigger::nth_counter_put_on(
            filter,
            counter_type,
            counter_number,
        ),
        TriggerKind::CounterRemovedFrom(counter_removed_from) => {
            convert_counter_removed_from_trigger(counter_removed_from)
        }
    };
    Ok(match intro_surface {
        Some(ironsmith_core::trigger_model::TriggerIntroSurface::When) => {
            interpreted.with_intro_surface(crate::triggers::TriggerIntroSurface::When)
        }
        Some(ironsmith_core::trigger_model::TriggerIntroSurface::Whenever) => {
            interpreted.with_intro_surface(crate::triggers::TriggerIntroSurface::Whenever)
        }
        Some(ironsmith_core::trigger_model::TriggerIntroSurface::At) => {
            interpreted.with_intro_surface(crate::triggers::TriggerIntroSurface::At)
        }
        None => interpreted,
    })
}

impl super::Trigger {
    pub fn from_delayed_trigger_spec(spec: ironsmith_core::DelayedTriggerSpec) -> Self {
        match spec {
            ironsmith_core::DelayedTriggerSpec::AsPermanentsUntap {
                player,
                source_must_be_controlled,
            } => Self::as_permanents_untap(player, source_must_be_controlled),
            ironsmith_core::DelayedTriggerSpec::BeginningOfUpkeep(player) => {
                Self::beginning_of_upkeep(player)
            }
            ironsmith_core::DelayedTriggerSpec::BeginningOfDrawStep(player) => {
                Self::beginning_of_draw_step(player)
            }
            ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(player) => {
                Self::beginning_of_end_step(player)
            }
            ironsmith_core::DelayedTriggerSpec::BeginningOfCleanupStep(player) => {
                Self::beginning_of_cleanup_step(player)
            }
            ironsmith_core::DelayedTriggerSpec::BeginningOfNextCleanupStep(player) => {
                Self::beginning_of_next_cleanup_step(player)
            }
            ironsmith_core::DelayedTriggerSpec::BeginningOfCombat(player) => {
                Self::beginning_of_combat(player)
            }
            ironsmith_core::DelayedTriggerSpec::BeginningOfMainPhase(player) => {
                Self::beginning_of_main_phase(player)
            }
            ironsmith_core::DelayedTriggerSpec::BeginningOfPrecombatMainPhase(player) => {
                Self::beginning_of_precombat_main_phase(player)
            }
            ironsmith_core::DelayedTriggerSpec::BeginningOfPostcombatMainPhase(player) => {
                Self::beginning_of_postcombat_main_phase(player)
            }
            ironsmith_core::DelayedTriggerSpec::EndOfCombat => Self::end_of_combat(),
            ironsmith_core::DelayedTriggerSpec::SourceControllerLosesControl {
                source_description,
            } => Self::source_controller_loses_control(source_description),
            ironsmith_core::DelayedTriggerSpec::ThisEntersBattlefield => {
                Self::this_enters_battlefield()
            }
            ironsmith_core::DelayedTriggerSpec::ThisEntersBattlefieldWithSurface {
                surface,
                subject_number,
            } => Self::new(
                super::zone_changes::ZoneChangeTrigger::new()
                    .to(crate::zone::Zone::Battlefield)
                    .this()
                    .this_surface(surface)
                    .this_subject_number(subject_number),
            ),
            ironsmith_core::DelayedTriggerSpec::EntersBattlefield {
                filter,
                cause_filter,
                count,
                tapped,
            } => match (count, tapped) {
                (ironsmith_core::trigger_model::CountMode::One, None) => {
                    Self::enters_battlefield(filter, cause_filter)
                }
                (ironsmith_core::trigger_model::CountMode::OneOrMore, None) => {
                    Self::enters_battlefield_one_or_more(filter, cause_filter)
                }
                (ironsmith_core::trigger_model::CountMode::One, Some(true)) => {
                    Self::enters_battlefield_tapped(filter, cause_filter)
                }
                (ironsmith_core::trigger_model::CountMode::One, Some(false)) => {
                    Self::enters_battlefield_untapped(filter, cause_filter)
                }
                _ => Self::enters_battlefield(filter, cause_filter),
            },
            ironsmith_core::DelayedTriggerSpec::ThisDies => Self::this_dies(),
            ironsmith_core::DelayedTriggerSpec::ThisLeavesBattlefield => {
                Self::this_leaves_battlefield()
            }
            ironsmith_core::DelayedTriggerSpec::ThisAttacksAndIsntBlocked => {
                Self::this_attacks_and_isnt_blocked()
            }
            ironsmith_core::DelayedTriggerSpec::ThisBlocksObject {
                filter,
                min_blocked_objects,
            } => match min_blocked_objects {
                Some(minimum) => Self::this_blocks_objects_with_minimum(filter, minimum as usize),
                None => Self::this_blocks_object(filter),
            },
            ironsmith_core::DelayedTriggerSpec::ThisBecomesBlockedByObject(filter) => {
                Self::this_becomes_blocked_by_object(filter)
            }
            ironsmith_core::DelayedTriggerSpec::Attacks(filter) => Self::attacks(filter),
            ironsmith_core::DelayedTriggerSpec::AttacksAndIsntBlocked(filter) => {
                Self::attacks_and_isnt_blocked(filter)
            }
            ironsmith_core::DelayedTriggerSpec::AttacksOneOrMore(filter) => {
                Self::attacks_one_or_more(filter)
            }
            ironsmith_core::DelayedTriggerSpec::Blocks(filter) => Self::blocks(filter),
            ironsmith_core::DelayedTriggerSpec::BlocksOneOrMore(filter) => {
                Self::blocks_one_or_more(filter)
            }
            ironsmith_core::DelayedTriggerSpec::BecomesBlocked(filter) => {
                Self::becomes_blocked(filter)
            }
            ironsmith_core::DelayedTriggerSpec::LeavesBattlefield(filter) => {
                Self::leaves_battlefield(filter)
            }
            ironsmith_core::DelayedTriggerSpec::Dies(filter) => Self::dies(filter),
            ironsmith_core::DelayedTriggerSpec::PermanentBecomesTapped(filter) => {
                Self::permanent_becomes_tapped(filter)
            }
            ironsmith_core::DelayedTriggerSpec::DealsCombatDamage(filter) => {
                Self::deals_combat_damage(filter)
            }
            ironsmith_core::DelayedTriggerSpec::DealsCombatDamageTo { source, target } => {
                Self::deals_combat_damage_to(source, target)
            }
            ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayer { source, player } => {
                Self::deals_combat_damage_to_player(source, player)
            }
            ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayerOneOrMore {
                source,
                player,
            } => Self::deals_combat_damage_to_player_one_or_more(source, player),
            ironsmith_core::DelayedTriggerSpec::IsDealtDamage(target) => {
                Self::is_dealt_damage(target)
            }
            ironsmith_core::DelayedTriggerSpec::PutIntoGraveyard(filter) => {
                Self::put_into_graveyard(filter)
            }
            ironsmith_core::DelayedTriggerSpec::PutIntoGraveyardFromZone {
                filter,
                from,
                one_or_more,
            } => {
                let trigger = super::zone_changes::ZoneChangeTrigger::new()
                    .from(from)
                    .to(crate::zone::Zone::Graveyard)
                    .filter(filter);
                if one_or_more {
                    Self::new(trigger.count(super::zone_changes::CountMode::OneOrMore))
                } else {
                    Self::new(trigger)
                }
            }
            ironsmith_core::DelayedTriggerSpec::SpellCast {
                filter,
                caster,
                timing,
                during_turn,
                min_spells_this_turn,
                exact_spells_this_turn,
                from_not_hand,
                first_spell_of_game,
            } => Self::new(
                super::SpellCastTrigger::qualified(
                    filter,
                    caster,
                    timing,
                    during_turn,
                    min_spells_this_turn,
                    exact_spells_this_turn,
                    from_not_hand,
                )
                .with_first_spell_of_game(first_spell_of_game),
            ),
            ironsmith_core::DelayedTriggerSpec::PlayerPlaysLand { player, filter } => {
                Self::player_plays_land(player, filter)
            }
            ironsmith_core::DelayedTriggerSpec::PlayerDrawsCard(player) => {
                Self::player_draws_card(player)
            }
            ironsmith_core::DelayedTriggerSpec::AbilityActivated {
                activator,
                filter,
                non_mana_only,
                loyalty_only,
                activation_cost_has_tap,
            } => Self::ability_activated_qualified_with_activation_cost_tap(
                activator,
                filter,
                non_mana_only,
                loyalty_only,
                activation_cost_has_tap,
            ),
            ironsmith_core::DelayedTriggerSpec::Either(left, right) => Self::either(
                Self::from_delayed_trigger_spec(*left),
                Self::from_delayed_trigger_spec(*right),
            ),
        }
    }

    pub fn from_model(
        trigger: ironsmith_core::trigger_model::Trigger,
    ) -> Result<Self, TriggerModelConversionError> {
        interpret_trigger_model(trigger)
    }
}

#[cfg(test)]
mod delayed_spec_tests {
    use crate::target::{ObjectFilter, PlayerFilter};

    #[test]
    fn player_plays_land_delayed_spec_uses_land_play_matcher() {
        let filter = ObjectFilter::land();
        let trigger = crate::triggers::Trigger::from_delayed_trigger_spec(
            ironsmith_core::DelayedTriggerSpec::PlayerPlaysLand {
                player: PlayerFilter::You,
                filter: filter.clone(),
            },
        );
        let matcher = trigger
            .downcast_ref::<crate::triggers::PlayerPlaysLandTrigger>()
            .expect("land-play delayed spec should preserve the runtime matcher");
        assert_eq!(matcher.player, PlayerFilter::You);
        assert_eq!(matcher.filter, filter);
    }

    #[test]
    fn source_relative_block_relations_survive_delayed_spec_interpretation() {
        let filter = ObjectFilter::creature();
        let trigger = crate::triggers::Trigger::from_delayed_trigger_spec(
            ironsmith_core::DelayedTriggerSpec::Either(
                Box::new(ironsmith_core::DelayedTriggerSpec::ThisBlocksObject {
                    filter: filter.clone(),
                    min_blocked_objects: None,
                }),
                Box::new(
                    ironsmith_core::DelayedTriggerSpec::ThisBecomesBlockedByObject(filter.clone()),
                ),
            ),
        );
        let either = trigger
            .downcast_ref::<crate::triggers::OrTrigger>()
            .expect("the delayed union should remain an executable Or trigger");
        assert_eq!(either.triggers.len(), 2);
        assert!(either.triggers.iter().any(|branch| {
            branch
                .downcast_ref::<crate::triggers::ThisBlocksObjectTrigger>()
                .is_some_and(|matcher| matcher.blocked_filter == filter)
        }));
        assert!(either.triggers.iter().any(|branch| {
            branch
                .downcast_ref::<crate::triggers::ThisBecomesBlockedByObjectTrigger>()
                .is_some_and(|matcher| matcher.blocker_filter == filter)
        }));
    }
}
