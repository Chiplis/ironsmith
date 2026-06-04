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
    if let Some(from_zones) = trigger.from_zones {
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
    if let Some(to) = trigger.to {
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
    out = out.count(convert_count_mode(trigger.count));
    out = out.cause_filter(trigger.cause_filter);
    if let Some(during_turn) = trigger.during_turn {
        out = out.during_turn(during_turn);
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
    crate::triggers::Trigger::counter_removed_from(trigger.filter)
}

pub(crate) fn interpret_trigger_model(
    trigger: ironsmith_core::trigger_model::Trigger,
) -> Result<crate::triggers::Trigger, TriggerModelConversionError> {
    use ironsmith_core::trigger_model::{DamagedBySource, TriggerKind};

    let intro_surface = trigger.intro_surface;
    let interpreted = match trigger.kind {
        TriggerKind::StateBased { display } => crate::triggers::Trigger::state_based(display),
        TriggerKind::ThisAttacks => crate::triggers::Trigger::this_attacks(),
        TriggerKind::ThisAttacksPlayerWithMostLife => {
            crate::triggers::Trigger::this_attacks_player_with_most_life()
        }
        TriggerKind::ThisAttacksWithGreaterPower => {
            crate::triggers::Trigger::this_attacks_with_greater_power()
        }
        TriggerKind::ThisAttacksWithNOthers {
            count,
            display_subject,
        } => crate::triggers::Trigger::this_attacks_with_n_others_display_subject(
            count,
            display_subject,
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
        TriggerKind::AttacksOneOrMoreWithMinTotal {
            filter,
            min_total_attackers,
        } => crate::triggers::Trigger::attacks_one_or_more_with_min_total(
            filter,
            min_total_attackers,
        ),
        TriggerKind::AttacksAlone { filter } => crate::triggers::Trigger::attacks_alone(filter),
        TriggerKind::AttacksYou { filter } => crate::triggers::Trigger::attacks_you(filter),
        TriggerKind::AttacksYouOneOrMore { filter } => {
            crate::triggers::Trigger::attacks_you_one_or_more(filter)
        }
        TriggerKind::ThisBlocks => crate::triggers::Trigger::this_blocks(),
        TriggerKind::ThisBlocksObject { filter } => {
            crate::triggers::Trigger::this_blocks_object(filter)
        }
        TriggerKind::Blocks { filter } => crate::triggers::Trigger::blocks(filter),
        TriggerKind::BlocksOneOrMore { filter } => {
            crate::triggers::Trigger::blocks_one_or_more(filter)
        }
        TriggerKind::ThisBecomesBlocked => crate::triggers::Trigger::this_becomes_blocked(),
        TriggerKind::ThisBecomesBlockedByObject { filter } => {
            crate::triggers::Trigger::this_becomes_blocked_by_object(filter)
        }
        TriggerKind::ThisDies => crate::triggers::Trigger::this_dies(),
        TriggerKind::ThisDiesOrIsExiled => crate::triggers::Trigger::this_dies_or_is_exiled(),
        TriggerKind::ThisLeavesBattlefield => crate::triggers::Trigger::this_leaves_battlefield(),
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
        TriggerKind::ThisDealsCombatDamageToPlayer { player } => {
            crate::triggers::Trigger::this_deals_combat_damage_to_player(player)
        }
        TriggerKind::DealsDamage { filter } => crate::triggers::Trigger::deals_damage(filter),
        TriggerKind::DealsDamageToPlayer { source, player } => {
            let mut trigger = crate::triggers::combat::DealsDamageTrigger::new(source);
            trigger.damaged_player = Some(player);
            crate::triggers::Trigger::new(trigger)
        }
        TriggerKind::DealsNoncombatDamageToPlayer { source, player } => {
            crate::triggers::Trigger::deals_noncombat_damage_to_player(source, player)
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
        TriggerKind::AbilityActivatedQualified {
            activator,
            filter,
            non_mana_only,
            loyalty_only,
        } => crate::triggers::Trigger::ability_activated_qualified(
            activator,
            filter,
            non_mana_only,
            loyalty_only,
        ),
        TriggerKind::IsDealtDamage {
            target,
            combat_only,
        } => {
            if combat_only {
                crate::triggers::Trigger::is_dealt_combat_damage(target)
            } else {
                crate::triggers::Trigger::is_dealt_damage(target)
            }
        }
        TriggerKind::YouGainLife => crate::triggers::Trigger::you_gain_life(),
        TriggerKind::YouGainLifeDuringTurn { during_turn } => {
            crate::triggers::Trigger::you_gain_life_during_turn(during_turn)
        }
        TriggerKind::PlayerLosesLife { player } => {
            crate::triggers::Trigger::player_loses_life(player)
        }
        TriggerKind::PlayerLosesGame { player } => {
            crate::triggers::Trigger::player_loses_game(player)
        }
        TriggerKind::PlayerLosesLifeDuringTurn {
            player,
            during_turn,
        } => crate::triggers::Trigger::player_loses_life_during_turn(player, during_turn),
        TriggerKind::PlayerLostGame { player } => {
            crate::triggers::Trigger::player_lost_game(player)
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
        TriggerKind::PlayerDiscardsCard { player, filter } => {
            crate::triggers::Trigger::player_discards_card(player, filter)
        }
        TriggerKind::PlayerRevealsCard {
            player,
            filter,
            from_source,
        } => crate::triggers::Trigger::player_reveals_card(player, filter, from_source),
        TriggerKind::PlayerSacrifices { player, filter } => {
            crate::triggers::Trigger::player_sacrifices(player, filter)
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
        TriggerKind::SpellCastQualified {
            filter,
            caster,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        } => crate::triggers::Trigger::spell_cast_qualified(
            filter,
            caster,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        ),
        TriggerKind::SpellCast { filter, caster } => {
            crate::triggers::Trigger::spell_cast(filter, caster)
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
        TriggerKind::BeginningOfEndStep { player } => {
            crate::triggers::Trigger::beginning_of_end_step(player)
        }
        TriggerKind::BeginningOfPrecombatMainPhase { player } => {
            crate::triggers::Trigger::beginning_of_precombat_main_phase(player)
        }
        TriggerKind::BeginningOfPostcombatMainPhase { player } => {
            crate::triggers::Trigger::beginning_of_postcombat_main_phase(player)
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
        TriggerKind::KeywordActionMatchingTaggedObject {
            action,
            player,
            source_filter,
            object_tag,
            object_filter,
        } => crate::triggers::Trigger::keyword_action_matching_source_and_tagged_object(
            action,
            player,
            source_filter,
            object_tag,
            object_filter,
        ),
        TriggerKind::KeywordAction { action, player } => {
            crate::triggers::Trigger::keyword_action(action, player)
        }
        TriggerKind::KeywordActionFromSource { action, player } => {
            crate::triggers::Trigger::keyword_action_from_source(action, player)
        }
        TriggerKind::WinsClash { player } => crate::triggers::Trigger::wins_clash(player),
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
            ironsmith_core::DelayedTriggerSpec::BeginningOfUpkeep(player) => {
                Self::beginning_of_upkeep(player)
            }
            ironsmith_core::DelayedTriggerSpec::BeginningOfDrawStep(player) => {
                Self::beginning_of_draw_step(player)
            }
            ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(player) => {
                Self::beginning_of_end_step(player)
            }
            ironsmith_core::DelayedTriggerSpec::EndOfCombat => Self::end_of_combat(),
            ironsmith_core::DelayedTriggerSpec::ThisDies => Self::this_dies(),
            ironsmith_core::DelayedTriggerSpec::ThisLeavesBattlefield => {
                Self::this_leaves_battlefield()
            }
            ironsmith_core::DelayedTriggerSpec::ThisAttacksAndIsntBlocked => {
                Self::this_attacks_and_isnt_blocked()
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
            ironsmith_core::DelayedTriggerSpec::LeavesBattlefield(filter) => {
                Self::leaves_battlefield(filter)
            }
            ironsmith_core::DelayedTriggerSpec::Dies(filter) => Self::dies(filter),
            ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayer { source, player } => {
                Self::deals_combat_damage_to_player(source, player)
            }
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
                during_turn,
                min_spells_this_turn,
                exact_spells_this_turn,
                from_not_hand,
            } => Self::spell_cast_qualified(
                filter,
                caster,
                during_turn,
                min_spells_this_turn,
                exact_spells_this_turn,
                from_not_hand,
            ),
        }
    }

    pub fn from_model(
        trigger: ironsmith_core::trigger_model::Trigger,
    ) -> Result<Self, TriggerModelConversionError> {
        interpret_trigger_model(trigger)
    }
}
