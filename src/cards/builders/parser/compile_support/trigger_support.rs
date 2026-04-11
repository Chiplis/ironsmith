use crate::cards::builders::{
    CardTextError, DamageBySpec, EffectAst, ReferenceImports, TagKey, TriggerSpec,
};
use crate::effect::{Effect, EventValueSpec};
use crate::filter::ObjectRef;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::Trigger;

use super::LoweredEffects;

pub(crate) fn compile_trigger_spec(trigger: TriggerSpec) -> Trigger {
    match trigger {
        TriggerSpec::StateBased { display, .. } => Trigger::state_based(display),
        TriggerSpec::ThisAttacks => Trigger::this_attacks(),
        TriggerSpec::ThisAttacksWithExactlyNOthers(other_count) => {
            Trigger::this_attacks_with_exact_n_others(other_count as usize)
        }
        TriggerSpec::ThisAttacksAndIsntBlocked => Trigger::this_attacks_and_isnt_blocked(),
        TriggerSpec::ThisAttacksWhileSaddled => Trigger::this_attacks_while_saddled(),
        TriggerSpec::Attacks(filter) => Trigger::attacks(filter),
        TriggerSpec::AttacksAndIsntBlocked(filter) => Trigger::attacks_and_isnt_blocked(filter),
        TriggerSpec::AttacksWhileSaddled(filter) => Trigger::attacks_while_saddled(filter),
        TriggerSpec::AttacksOneOrMore(filter) => Trigger::attacks_one_or_more(filter),
        TriggerSpec::AttacksOneOrMoreWithMinTotal {
            filter,
            min_total_attackers,
        } => Trigger::attacks_one_or_more_with_min_total(filter, min_total_attackers as usize),
        TriggerSpec::AttacksAlone(filter) => Trigger::attacks_alone(filter),
        TriggerSpec::AttacksYouOrPlaneswalkerYouControl(filter) => Trigger::attacks_you(filter),
        TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(filter) => {
            Trigger::attacks_you_one_or_more(filter)
        }
        TriggerSpec::ThisBlocks => Trigger::this_blocks(),
        TriggerSpec::ThisBlocksObject(filter) => Trigger::this_blocks_object(filter),
        TriggerSpec::Blocks(filter) => Trigger::blocks(filter),
        TriggerSpec::ThisBecomesBlocked => Trigger::this_becomes_blocked(),
        TriggerSpec::ThisDies => Trigger::this_dies(),
        TriggerSpec::ThisDiesOrIsExiled => Trigger::this_dies_or_is_exiled(),
        TriggerSpec::ThisLeavesBattlefield => Trigger::this_leaves_battlefield(),
        TriggerSpec::ThisBecomesMonstrous => Trigger::this_becomes_monstrous(),
        TriggerSpec::ThisBecomesTapped => Trigger::becomes_tapped(),
        TriggerSpec::PermanentBecomesTapped(filter) => Trigger::permanent_becomes_tapped(filter),
        TriggerSpec::ThisBecomesUntapped => Trigger::becomes_untapped(),
        TriggerSpec::ThisTurnedFaceUp => Trigger::this_is_turned_face_up(),
        TriggerSpec::TurnedFaceUp(filter) => Trigger::turned_face_up(filter),
        TriggerSpec::ThisBecomesTargeted => Trigger::becomes_targeted(),
        TriggerSpec::BecomesTargeted(filter) => Trigger::becomes_targeted_object(filter),
        TriggerSpec::ThisBecomesTargetedBySpell(filter) => {
            Trigger::becomes_targeted_by_spell(filter)
        }
        TriggerSpec::BecomesTargetedBySourceController {
            target,
            source_controller,
        } => Trigger::becomes_targeted_by_source_controller(target, source_controller),
        TriggerSpec::ThisDealsDamage => Trigger::this_deals_damage(),
        TriggerSpec::ThisDealsDamageToPlayer { player, amount } => {
            Trigger::this_deals_damage_to_player(player, amount)
        }
        TriggerSpec::ThisDealsDamageTo(filter) => Trigger::this_deals_damage_to(filter),
        TriggerSpec::ThisDealsCombatDamage => Trigger::this_deals_combat_damage(),
        TriggerSpec::ThisDealsCombatDamageTo(filter) => {
            Trigger::this_deals_combat_damage_to(filter)
        }
        TriggerSpec::DealsDamage(filter) => Trigger::deals_damage(filter),
        TriggerSpec::DealsCombatDamage(filter) => Trigger::deals_combat_damage(filter),
        TriggerSpec::DealsCombatDamageTo { source, target } => {
            Trigger::deals_combat_damage_to(source, target)
        }
        TriggerSpec::PlayerPlaysLand { player, filter } => {
            Trigger::player_plays_land(player, filter)
        }
        TriggerSpec::PlayerGivesGift(player) => Trigger::player_gives_gift(player),
        TriggerSpec::PlayerSearchesLibrary(player) => Trigger::player_searches_library(player),
        TriggerSpec::PlayerShufflesLibrary {
            player,
            caused_by_effect,
            source_controller_shuffles,
        } => Trigger::player_shuffles_library(player, caused_by_effect, source_controller_shuffles),
        TriggerSpec::PlayerTapsForMana { player, filter } => {
            Trigger::player_taps_for_mana(player, filter)
        }
        TriggerSpec::AbilityActivated {
            activator,
            filter,
            non_mana_only,
        } => Trigger::ability_activated_qualified(activator, filter, non_mana_only),
        TriggerSpec::ThisIsDealtDamage => Trigger::is_dealt_damage(ChooseSpec::Source),
        TriggerSpec::IsDealtDamage(filter) => Trigger::is_dealt_damage(ChooseSpec::Object(filter)),
        TriggerSpec::YouGainLife => Trigger::you_gain_life(),
        TriggerSpec::YouGainLifeDuringTurn(during_turn) => {
            Trigger::you_gain_life_during_turn(during_turn)
        }
        TriggerSpec::PlayerLosesLife(player) => Trigger::player_loses_life(player),
        TriggerSpec::PlayerLosesLifeDuringTurn {
            player,
            during_turn,
        } => Trigger::player_loses_life_during_turn(player, during_turn),
        TriggerSpec::YouDrawCard => Trigger::you_draw_card(),
        TriggerSpec::PlayerDrawsCard(player) => Trigger::player_draws_card(player),
        TriggerSpec::PlayerDrawsCardNotDuringTurn {
            player,
            during_turn,
        } => Trigger::player_draws_card_not_during_turn(player, during_turn),
        TriggerSpec::PlayerDrawsNthCardEachTurn {
            player,
            card_number,
        } => Trigger::player_draws_nth_card_each_turn(player, card_number),
        TriggerSpec::PlayerDiscardsCard {
            player,
            filter,
            cause_controller,
            effect_like_only,
        } => {
            if let Some(cause_controller) = cause_controller {
                Trigger::player_discards_card_caused_by_controller(
                    player,
                    filter,
                    cause_controller,
                    effect_like_only,
                )
            } else {
                Trigger::player_discards_card(player, filter)
            }
        }
        TriggerSpec::PlayerRevealsCard {
            player,
            filter,
            from_source,
        } => Trigger::player_reveals_card(player, filter, from_source),
        TriggerSpec::PlayerSacrifices { player, filter } => {
            Trigger::player_sacrifices(player, filter)
        }
        TriggerSpec::Dies(filter) => Trigger::dies(filter),
        TriggerSpec::PutIntoGraveyard(filter) => Trigger::put_into_graveyard(filter),
        TriggerSpec::PutIntoGraveyardFromZone { filter, from } => Trigger::new(
            crate::triggers::zone_changes::ZoneChangeTrigger::new()
                .from(from)
                .to(crate::zone::Zone::Graveyard)
                .filter(filter),
        ),
        TriggerSpec::CounterPutOn {
            filter,
            counter_type,
            source_controller,
            one_or_more,
        } => {
            let mut trigger = crate::triggers::CounterPutOnTrigger::new(filter);
            if let Some(counter_type) = counter_type {
                trigger = trigger.counter_type(counter_type);
            }
            if let Some(source_controller) = source_controller {
                trigger = trigger.source_controller(source_controller);
            }
            if one_or_more {
                trigger = trigger.count(crate::triggers::CountMode::OneOrMore);
            }
            Trigger::new(trigger)
        }
        TriggerSpec::DiesCreatureDealtDamageByThisTurn { victim, damager } => match damager {
            DamageBySpec::ThisCreature => {
                Trigger::creature_dealt_damage_by_this_creature_this_turn_dies(victim)
            }
            DamageBySpec::EquippedCreature => {
                Trigger::creature_dealt_damage_by_equipped_creature_this_turn_dies(victim)
            }
            DamageBySpec::EnchantedCreature => {
                Trigger::creature_dealt_damage_by_enchanted_creature_this_turn_dies(victim)
            }
        },
        TriggerSpec::SpellCast {
            filter,
            caster,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        } => Trigger::spell_cast_qualified(
            filter,
            caster,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        ),
        TriggerSpec::SpellCopied { filter, copier } => Trigger::spell_copied(filter, copier),
        TriggerSpec::EntersBattlefield {
            filter,
            cause_filter,
        } => Trigger::enters_battlefield(filter, cause_filter),
        TriggerSpec::EntersBattlefieldOneOrMore {
            filter,
            cause_filter,
        } => Trigger::enters_battlefield_one_or_more(filter, cause_filter),
        TriggerSpec::EntersBattlefieldFromZone {
            mut filter,
            from,
            owner,
            one_or_more,
            cause_filter,
        } => {
            if let Some(owner) = owner {
                filter.owner = Some(owner);
            }
            let trigger = crate::triggers::ZoneChangeTrigger::new()
                .from(from)
                .to(crate::zone::Zone::Battlefield)
                .filter(filter)
                .cause_filter(cause_filter);
            if one_or_more {
                Trigger::new(trigger.count(crate::triggers::CountMode::OneOrMore))
            } else {
                Trigger::new(trigger)
            }
        }
        TriggerSpec::EntersBattlefieldTapped {
            filter,
            cause_filter,
        } => Trigger::enters_battlefield_tapped(filter, cause_filter),
        TriggerSpec::EntersBattlefieldUntapped {
            filter,
            cause_filter,
        } => Trigger::enters_battlefield_untapped(filter, cause_filter),
        TriggerSpec::BeginningOfUpkeep(player) => Trigger::beginning_of_upkeep(player),
        TriggerSpec::BeginningOfDrawStep(player) => Trigger::beginning_of_draw_step(player),
        TriggerSpec::BeginningOfCombat(player) => Trigger::beginning_of_combat(player),
        TriggerSpec::BeginningOfEndStep(player) => Trigger::beginning_of_end_step(player),
        TriggerSpec::BeginningOfPrecombatMain(player) => {
            Trigger::beginning_of_precombat_main_phase(player)
        }
        TriggerSpec::BeginningOfPostcombatMain(player) => {
            Trigger::beginning_of_postcombat_main_phase(player)
        }
        TriggerSpec::ThisEntersBattlefield => Trigger::this_enters_battlefield(),
        TriggerSpec::ThisEntersBattlefieldFromZone {
            mut subject_filter,
            from,
            owner,
        } => {
            if let Some(owner) = owner {
                subject_filter.owner = Some(owner);
            }
            Trigger::new(
                crate::triggers::ZoneChangeTrigger::new()
                    .from(from)
                    .to(crate::zone::Zone::Battlefield)
                    .filter(subject_filter)
                    .this(),
            )
        }
        TriggerSpec::ThisDealsCombatDamageToPlayer => Trigger::this_deals_combat_damage_to_player(),
        TriggerSpec::DealsCombatDamageToPlayer { source, player } => {
            Trigger::deals_combat_damage_to_player(source, player)
        }
        TriggerSpec::DealsCombatDamageToPlayerOneOrMore { source, player } => {
            Trigger::deals_combat_damage_to_player_one_or_more(source, player)
        }
        TriggerSpec::YouCastThisSpell => Trigger::you_cast_this_spell(),
        TriggerSpec::KeywordAction {
            action,
            player,
            source_filter,
        } => match source_filter {
            Some(filter) => Trigger::keyword_action_matching_object(action, player, filter),
            None => Trigger::keyword_action(action, player),
        },
        TriggerSpec::KeywordActionFromSource { action, player } => {
            Trigger::keyword_action_from_source(action, player)
        }
        TriggerSpec::WinsClash { player } => Trigger::wins_clash(player),
        TriggerSpec::Expend { player, amount } => Trigger::expend(amount, player),
        TriggerSpec::SagaChapter(chapters) => Trigger::saga_chapter(chapters),
        TriggerSpec::HauntedCreatureDies => Trigger::custom(
            "haunted_creature_dies",
            "When the creature it haunts dies".to_string(),
        ),
        TriggerSpec::Either(left, right) => {
            Trigger::either(compile_trigger_spec(*left), compile_trigger_spec(*right))
        }
    }
}

pub(crate) fn ensure_concrete_trigger_spec(trigger: &TriggerSpec) -> Result<(), CardTextError> {
    match trigger {
        TriggerSpec::Either(left, right) => {
            ensure_concrete_trigger_spec(left)?;
            ensure_concrete_trigger_spec(right)?;
            Ok(())
        }
        _ => Ok(()),
    }
}

fn trigger_binds_iterated_player(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::SpellCast { .. }
        | TriggerSpec::SpellCopied { .. }
        | TriggerSpec::PlayerLosesLife(_)
        | TriggerSpec::PlayerLosesLifeDuringTurn { .. }
        | TriggerSpec::PlayerDrawsCard(_)
        | TriggerSpec::PlayerDrawsCardNotDuringTurn { .. }
        | TriggerSpec::PlayerDrawsNthCardEachTurn { .. }
        | TriggerSpec::PlayerDiscardsCard { .. }
        | TriggerSpec::PlayerRevealsCard { .. }
        | TriggerSpec::PlayerPlaysLand { .. }
        | TriggerSpec::PlayerGivesGift(_)
        | TriggerSpec::PlayerSearchesLibrary(_)
        | TriggerSpec::PlayerShufflesLibrary { .. }
        | TriggerSpec::PlayerTapsForMana { .. }
        | TriggerSpec::PlayerSacrifices { .. }
        | TriggerSpec::BeginningOfUpkeep(_)
        | TriggerSpec::BeginningOfDrawStep(_)
        | TriggerSpec::BeginningOfCombat(_)
        | TriggerSpec::BeginningOfEndStep(_)
        | TriggerSpec::BeginningOfPrecombatMain(_)
        | TriggerSpec::BeginningOfPostcombatMain(_)
        | TriggerSpec::DealsCombatDamageToPlayerOneOrMore { .. }
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControl(_)
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(_)
        | TriggerSpec::KeywordAction { .. }
        | TriggerSpec::KeywordActionFromSource { .. }
        | TriggerSpec::WinsClash { .. }
        | TriggerSpec::Expend { .. } => true,
        TriggerSpec::StateBased { .. } => false,
        TriggerSpec::BecomesTargetedBySourceController {
            source_controller, ..
        } => *source_controller != PlayerFilter::Any,
        TriggerSpec::Either(left, right) => {
            trigger_binds_iterated_player(left) && trigger_binds_iterated_player(right)
        }
        _ => false,
    }
}

pub(crate) fn inferred_trigger_player_filter(trigger: &TriggerSpec) -> Option<PlayerFilter> {
    match trigger {
        TriggerSpec::StateBased { .. } => None,
        TriggerSpec::EntersBattlefield { .. }
        | TriggerSpec::EntersBattlefieldOneOrMore { .. }
        | TriggerSpec::EntersBattlefieldFromZone { .. }
        | TriggerSpec::EntersBattlefieldTapped { .. }
        | TriggerSpec::EntersBattlefieldUntapped { .. } => Some(PlayerFilter::ControllerOf(
            ObjectRef::tagged(TagKey::from("triggering")),
        )),
        TriggerSpec::SpellCast { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::SpellCopied { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerLosesLife(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerLosesLifeDuringTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsCard(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsCardNotDuringTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsNthCardEachTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDiscardsCard { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerRevealsCard { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerPlaysLand { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerGivesGift(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerSearchesLibrary(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerShufflesLibrary { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerTapsForMana { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::AbilityActivated { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerSacrifices { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::ThisDealsDamageToPlayer { .. }
        | TriggerSpec::ThisDealsCombatDamageToPlayer
        | TriggerSpec::DealsCombatDamageToPlayer { .. } => Some(PlayerFilter::DamagedPlayer),
        TriggerSpec::ThisAttacks => Some(PlayerFilter::Defending),
        TriggerSpec::AttacksYouOrPlaneswalkerYouControl(_)
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(_) => {
            Some(PlayerFilter::IteratedPlayer)
        }
        TriggerSpec::BeginningOfUpkeep(player)
        | TriggerSpec::BeginningOfDrawStep(player)
        | TriggerSpec::BeginningOfCombat(player)
        | TriggerSpec::BeginningOfEndStep(player)
        | TriggerSpec::BeginningOfPrecombatMain(player)
        | TriggerSpec::BeginningOfPostcombatMain(player)
        | TriggerSpec::KeywordAction { player, .. }
        | TriggerSpec::KeywordActionFromSource { player, .. }
        | TriggerSpec::WinsClash { player } => {
            if *player == PlayerFilter::Any {
                Some(PlayerFilter::Active)
            } else {
                Some(PlayerFilter::IteratedPlayer)
            }
        }
        TriggerSpec::BecomesTargetedBySourceController {
            source_controller, ..
        } => {
            if *source_controller == PlayerFilter::Any {
                Some(PlayerFilter::Active)
            } else {
                Some(PlayerFilter::IteratedPlayer)
            }
        }
        TriggerSpec::Either(left, right) => {
            let left_filter = inferred_trigger_player_filter(left);
            let right_filter = inferred_trigger_player_filter(right);
            if left_filter == right_filter {
                left_filter
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn trigger_binds_player_reference_context(trigger: &TriggerSpec) -> bool {
    trigger_binds_iterated_player(trigger)
        || inferred_trigger_player_filter(trigger)
            .as_ref()
            .is_some_and(PlayerFilter::mentions_iterated_player)
}

pub(crate) fn trigger_supports_event_value(trigger: &TriggerSpec, spec: &EventValueSpec) -> bool {
    match spec {
        EventValueSpec::Amount | EventValueSpec::LifeAmount => match trigger {
            TriggerSpec::YouGainLife
            | TriggerSpec::YouGainLifeDuringTurn(_)
            | TriggerSpec::PlayerLosesLife(_)
            | TriggerSpec::PlayerLosesLifeDuringTurn { .. }
            | TriggerSpec::ThisIsDealtDamage
            | TriggerSpec::IsDealtDamage(_)
            | TriggerSpec::ThisDealsDamage
            | TriggerSpec::ThisDealsDamageTo(_)
            | TriggerSpec::DealsDamage(_)
            | TriggerSpec::ThisDealsCombatDamage
            | TriggerSpec::ThisDealsCombatDamageTo(_)
            | TriggerSpec::DealsCombatDamage(_)
            | TriggerSpec::DealsCombatDamageTo { .. }
            | TriggerSpec::ThisDealsCombatDamageToPlayer
            | TriggerSpec::DealsCombatDamageToPlayer { .. }
            | TriggerSpec::DealsCombatDamageToPlayerOneOrMore { .. }
            | TriggerSpec::KeywordAction { .. }
            | TriggerSpec::KeywordActionFromSource { .. }
            | TriggerSpec::CounterPutOn { .. } => true,
            TriggerSpec::StateBased { .. } => false,
            TriggerSpec::Either(left, right) => {
                trigger_supports_event_value(left, spec)
                    && trigger_supports_event_value(right, spec)
            }
            _ => false,
        },
        EventValueSpec::BlockersBeyondFirst { .. } => match trigger {
            TriggerSpec::ThisBecomesBlocked => true,
            TriggerSpec::Either(left, right) => {
                trigger_supports_event_value(left, spec)
                    && trigger_supports_event_value(right, spec)
            }
            _ => false,
        },
    }
}

pub(crate) fn compile_trigger_effects(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let lowered =
        compile_trigger_effects_with_imports(trigger, effects, &ReferenceImports::default())?;
    Ok((lowered.effects.to_vec(), lowered.choices))
}

pub(crate) fn compile_trigger_effects_with_imports(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
    imports: &ReferenceImports,
) -> Result<LoweredEffects, CardTextError> {
    let prepared = super::rewrite_prepare_effects_with_trigger_context_for_lowering(
        trigger,
        effects,
        imports.clone(),
    )?;
    super::materialize_prepared_effects_with_trigger_context(&prepared)
}
