//! The player a trigger implies.
//!
//! A triggered ability's event decides who "you" and "that player" name in the
//! ability's own text, so this answer seeds the reference environment. It reads
//! only the trigger, never the card's words.

use crate::cards::builders::TriggerSpec;
use crate::filter::{ObjectRef, PlayerFilter};

/// The player a trigger implies, when it names one.
///
/// A triggered ability's event decides who "you" and "that player" refer to in
/// the ability's own text, so this answer seeds the reference environment. It
/// reads only the trigger, never the card's words.
pub fn inferred_trigger_player_filter(trigger: &TriggerSpec) -> Option<PlayerFilter> {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => inferred_trigger_player_filter(trigger),
        TriggerSpec::StateBased { .. } | TriggerSpec::DayNightChanged => None,
        TriggerSpec::EntersBattlefield { filter, .. } if filter.source => None,
        TriggerSpec::EntersBattlefield { .. }
        | TriggerSpec::EntersBattlefieldOneOrMore { .. }
        | TriggerSpec::EntersBattlefieldFromZone { .. }
        | TriggerSpec::EntersBattlefieldTapped { .. }
        | TriggerSpec::EntersBattlefieldUntapped { .. } => Some(PlayerFilter::AliasedControllerOf(
            ObjectRef::tagged(crate::tag::CompilerReferenceTag::Triggering.key()),
        )),
        TriggerSpec::SpellCast { caster, .. } => {
            if *caster == PlayerFilter::Any {
                Some(PlayerFilter::IteratedPlayer)
            } else if *caster == PlayerFilter::You {
                Some(PlayerFilter::You)
            } else {
                Some(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                    crate::tag::CompilerReferenceTag::Triggering.key(),
                )))
            }
        }
        TriggerSpec::NthSpellOfTurnCast { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::SpellCountered { controller, .. } => {
            if *controller == PlayerFilter::Any {
                Some(PlayerFilter::IteratedPlayer)
            } else {
                Some(controller.clone())
            }
        }
        TriggerSpec::SpellCopied { copier, .. } => {
            if *copier == PlayerFilter::Any {
                Some(PlayerFilter::IteratedPlayer)
            } else {
                Some(copier.clone())
            }
        }
        TriggerSpec::PlayerLosesLife(_) | TriggerSpec::PlayersLoseLifeOneOrMore(_) => {
            Some(PlayerFilter::IteratedPlayer)
        }
        TriggerSpec::PlayerLosesGame(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerLosesLifeDuringTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsCard(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsCardNotDuringTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsCardExceptFirstInDrawStep(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsNthCardEachTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsNumberedCardsEachTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDiscardsCard { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerRevealsCard { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerPlaysLand { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerGivesGift(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerSearchesLibrary(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerShufflesLibrary { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerTapsForMana { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerRollsResult { .. }
        | TriggerSpec::PlayerRollsHighestNaturalResult { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerRollsDie { .. } | TriggerSpec::PlayerCoinFlipResult { .. } => {
            Some(PlayerFilter::IteratedPlayer)
        }
        TriggerSpec::AbilityActivated { .. } | TriggerSpec::AbilityTriggered { .. } => {
            Some(PlayerFilter::IteratedPlayer)
        }
        TriggerSpec::PlayerSacrifices { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::TokensCreated { player, .. } => {
            if *player == PlayerFilter::Any {
                Some(PlayerFilter::IteratedPlayer)
            } else {
                Some(player.clone())
            }
        }
        TriggerSpec::ThisDealsDamageToPlayer { .. }
        | TriggerSpec::DealsDamageToPlayer { .. }
        | TriggerSpec::DealsExactDamageToObjectOrPlayer { .. }
        | TriggerSpec::DealsNoncombatDamageToPlayer { .. }
        | TriggerSpec::ThisDealsCombatDamageToPlayer { .. }
        | TriggerSpec::DealsCombatDamageToPlayer { .. } => Some(PlayerFilter::DamagedPlayer),
        TriggerSpec::ThisAttacks
        | TriggerSpec::ThisAttacksPlayerWhoControlsAtLeast { .. }
        | TriggerSpec::ThisBecomesBlocked
        | TriggerSpec::BecomesBlocked(_)
        | TriggerSpec::BecomesBlockedByObjectWithLesserPower { .. } => {
            Some(PlayerFilter::Defending)
        }
        TriggerSpec::Attacks(filter) | TriggerSpec::AttacksOneOrMore(filter)
            if filter
                .attacking_player_or_planeswalker_controlled_by
                .is_some() =>
        {
            Some(PlayerFilter::Defending)
        }
        TriggerSpec::AttacksOneOrMoreWithMinTotal { filter, .. }
        | TriggerSpec::AttacksOneOrMoreWithExactTotal { filter, .. }
        | TriggerSpec::AttacksOneOrMoreWithAggregate { filter, .. }
            if filter
                .attacking_player_or_planeswalker_controlled_by
                .is_some() =>
        {
            Some(PlayerFilter::Defending)
        }
        TriggerSpec::AttacksYouOrPlaneswalkerYouControl(_)
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(_) => {
            Some(PlayerFilter::IteratedPlayer)
        }
        TriggerSpec::PlayerAttacksTargetWithOneOrMore { .. } => {
            // In "an opponent attacks a planeswalker ... with one or more
            // creatures, ... that player", the discourse antecedent is the
            // attacking player. The concrete event participant is the
            // attacking creature, so retain its aliased controller instead of
            // binding the defending planeswalker's controller.
            Some(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::Triggering.key(),
            )))
        }
        TriggerSpec::BeginningOfUpkeep(player)
        | TriggerSpec::BeginningOfDrawStep(player)
        | TriggerSpec::BeginningOfCombat(player)
        | TriggerSpec::BeginningOfEndStep(player)
        | TriggerSpec::BeginningOfMainPhase { player, .. }
        | TriggerSpec::BeginningOfPrecombatMain(player)
        | TriggerSpec::BeginningOfPostcombatMain { player, .. } => {
            if *player == PlayerFilter::Any {
                // `Any` phase/event triggers bind their participant from the
                // concrete event that fired the ability. This is usually the
                // active player for turn-based events, but retaining the typed
                // event binding keeps "that player" correct even when a test
                // or future turn structure dispatches the event independently
                // of the game's current active-player field.
                Some(PlayerFilter::IteratedPlayer)
            } else if matches!(
                player,
                PlayerFilter::You
                    | PlayerFilter::Specific(_)
                    | PlayerFilter::ChosenPlayer
                    | PlayerFilter::TaggedPlayer(_)
                    | PlayerFilter::ControllerOf(_)
                    | PlayerFilter::OwnerOf(_)
                    | PlayerFilter::AliasedControllerOf(_)
                    | PlayerFilter::AliasedOwnerOf(_)
            ) {
                // These filters identify one stable participant rather than
                // a set whose current member must come from the phase event.
                // Preserve that participant as the discourse antecedent for
                // relative phrases in the triggered effect ("that player",
                // "another player", and "other than that player").
                Some(player.clone())
            } else {
                Some(PlayerFilter::IteratedPlayer)
            }
        }
        TriggerSpec::KeywordAction { player, .. }
        | TriggerSpec::KeywordActionTaggedObject { player, .. }
        | TriggerSpec::KeywordActionFromSource { player, .. }
        | TriggerSpec::WinsClash { player, .. } => {
            if *player == PlayerFilter::Any {
                // Unlike each-player phase triggers, these families do not
                // prove that the resolution program owns a player-iteration
                // scope merely because their filter is `Any`.
                Some(PlayerFilter::Active)
            } else if matches!(
                player,
                PlayerFilter::You
                    | PlayerFilter::Specific(_)
                    | PlayerFilter::ChosenPlayer
                    | PlayerFilter::TaggedPlayer(_)
                    | PlayerFilter::ControllerOf(_)
                    | PlayerFilter::OwnerOf(_)
                    | PlayerFilter::AliasedControllerOf(_)
                    | PlayerFilter::AliasedOwnerOf(_)
            ) {
                Some(player.clone())
            } else {
                Some(PlayerFilter::IteratedPlayer)
            }
        }
        TriggerSpec::BeginningOfTheEndStep => Some(PlayerFilter::Active),
        TriggerSpec::BeginningOfMonarchEndStep => Some(PlayerFilter::IteratedPlayer),
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
