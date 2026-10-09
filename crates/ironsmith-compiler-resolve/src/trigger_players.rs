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
        TriggerSpec::ConditionQualified {
            condition: crate::cards::builders::PredicateAst::Triggering(
                crate::cards::builders::TriggeringPredicateAst::TriggeringEventCausedBy { .. }), ..
        } => Some(PlayerFilter::TaggedPlayer(ironsmith_core::TRIGGERING_EVENT_CAUSE_CONTROLLER_TAG.into())),
        TriggerSpec::WithIntro { trigger, .. } | TriggerSpec::ConditionQualified { trigger, .. } => inferred_trigger_player_filter(trigger),
        TriggerSpec::DamageReceived { target, .. } => match target.base() {
            crate::target::ChooseSpec::Player(_) | crate::target::ChooseSpec::SpecificPlayer(_)
                | crate::target::ChooseSpec::SourceController => Some(PlayerFilter::IteratedPlayer),
            _ => None,
        },
        TriggerSpec::StateBased { .. } | TriggerSpec::DayNightChanged => None,
        // Private-zone possessors name the owner, even when a stolen permanent
        // was controlled by somebody else immediately before the move.
        TriggerSpec::ZoneChange(event) => {
            let filter = event.filter.as_ref()?;
            if let Some(owner) = filter.owner.as_ref() {
                return Some(if *owner == PlayerFilter::You {
                    PlayerFilter::You
                } else {
                    PlayerFilter::AliasedOwnerOf(ObjectRef::tagged(
                        crate::tag::CompilerReferenceTag::Triggering.bind(),
                    ))
                });
            }
            // "Whenever a creature an opponent controls dies, ... that
            // player": the only player the event names is the controller of
            // the object that left (its last-known controller, CR 603.10a),
            // exactly as for the entering-permanent triggers below.
            filter
                .controller
                .as_ref()
                .filter(|controller| {
                    !matches!(controller, PlayerFilter::You | PlayerFilter::Any)
                })
                .map(|_| {
                    PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                        crate::tag::CompilerReferenceTag::Triggering.bind(),
                    ))
                })
        }
        TriggerSpec::Dies(filter)
        | TriggerSpec::ExiledFromBattlefield(filter)
        | TriggerSpec::LeavesBattlefieldWithoutDying {
            filter,
            one_or_more: false,
        }
        | TriggerSpec::PutIntoGraveyardFromZone {
            filter,
            from: crate::zone::Zone::Battlefield,
            one_or_more: false,
            ..
        } if filter.owner.is_none()
            && filter
                .controller
                .as_ref()
                .is_some_and(|controller| {
                    !matches!(controller, PlayerFilter::You | PlayerFilter::Any)
                }) =>
        {
            // The single object that left the battlefield names one player:
            // its last-known controller (CR 603.10a), "that player".
            Some(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::Triggering.bind(),
            )))
        }
        TriggerSpec::EntersBattlefield { filter, .. } if filter.source => None,
        // "Whenever a nonland permanent an opponent owns enters under your
        // control, they lose life ...": you are the controller, so the only
        // other player the event names is the permanent's owner.
        TriggerSpec::EntersBattlefield { filter, .. }
        | TriggerSpec::EntersBattlefieldOneOrMore { filter, .. }
            if filter.controller == Some(PlayerFilter::You)
                && filter
                    .owner
                    .as_ref()
                    .is_some_and(|owner| *owner != PlayerFilter::You) =>
        {
            Some(PlayerFilter::AliasedOwnerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::Triggering.bind(),
            )))
        }
        TriggerSpec::EntersBattlefield { .. }
        | TriggerSpec::EntersBattlefieldOneOrMore { .. }
        | TriggerSpec::EntersBattlefieldFromZone { .. }
        | TriggerSpec::EntersBattlefieldTapped { .. }
        | TriggerSpec::EntersBattlefieldUntapped { .. } => Some(PlayerFilter::AliasedControllerOf(
            ObjectRef::tagged(crate::tag::CompilerReferenceTag::Triggering.bind()),
        )),
        TriggerSpec::SpellCast { caster, .. }
        | TriggerSpec::SpellCastSameNameCardInZone { caster, .. } => {
            if *caster == PlayerFilter::Any {
                Some(PlayerFilter::IteratedPlayer)
            } else if *caster == PlayerFilter::You {
                Some(PlayerFilter::You)
            } else {
                Some(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                    crate::tag::CompilerReferenceTag::Triggering.bind(),
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
        TriggerSpec::PlayerAttackDeclaration { grouping, .. } => {
            let tag = if *grouping == ironsmith_core::trigger_model::PlayerAttackGrouping::Defender {
                ironsmith_core::tag::ATTACK_DECLARATION_DEFENDER_TAG
            } else {
                ironsmith_core::tag::ATTACK_DECLARATION_ACTOR_TAG
            };
            Some(PlayerFilter::TaggedPlayer(tag.into()))
        }
        TriggerSpec::CardsMilled { .. } | TriggerSpec::PlayerChangesTapState { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerGainsLife { .. } | TriggerSpec::PlayerLosesLife(_) | TriggerSpec::PlayersLoseLifeOneOrMore(_) => {
            Some(PlayerFilter::IteratedPlayer)
        }
        // CR 607.2a: in "When this leaves the battlefield, that player ...",
        // the only possible antecedent is the player chosen by this object's
        // linked entering ability ("When this enters, target player ...").
        TriggerSpec::ThisLeavesBattlefield | TriggerSpec::ThisLeavesBattlefieldWithSurface(_) => {
            Some(PlayerFilter::TaggedPlayer(
                crate::tag::CompilerReferenceTag::LinkedTriggerPlayer.bind().into(),
            ))
        }
        TriggerSpec::PlayerBecomesMonarch(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerLosesGame(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerPaysLife(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerLosesLifeDuringTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsCardDuringTurn { .. } | TriggerSpec::PlayerDrawsFirstCardInOwnDrawStep(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsCard(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsCardNotDuringTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsCardExceptFirstInDrawStep(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsNthCardEachTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerDrawsNumberedCardsEachTurn { .. } => Some(PlayerFilter::IteratedPlayer),
        // "When a spell or ability an opponent controls causes you to discard
        // this card, that player ...": you are the discarding player, so the
        // only other player the event names is the cause's controller.
        TriggerSpec::PlayerDiscardsCard {
            player,
            cause_controller: Some(cause_controller),
            ..
        } if *player == PlayerFilter::You && *cause_controller != PlayerFilter::You => Some(
            PlayerFilter::TaggedPlayer(ironsmith_core::TagKey::from(
                ironsmith_core::TRIGGERING_EVENT_CONTROLLER_TAG,
            )),
        ),
        TriggerSpec::PlayerDiscardsCard { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerRevealsCard { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerPlaysLand { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerGivesGift(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerSearchesLibrary(_) => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerShufflesLibrary { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerTapsForMana { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::PlayerRollsToVisitAttractions { .. }
        | TriggerSpec::PlayerRollsResult { .. }
        | TriggerSpec::PlayerRollsResultMatching { .. }
        | TriggerSpec::PlayerRollsNthDie { .. }
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
        TriggerSpec::DealsCombatDamageToPlayerOneOrMore {
            source, player: PlayerFilter::You, per_source_controller: true, ..
        } if source.controller.as_ref().is_some_and(|controller| *controller != PlayerFilter::You) =>
            Some(PlayerFilter::TaggedPlayer(ironsmith_core::tag::DAMAGE_SOURCE_CONTROLLER_TAG.into())),
        // "Whenever a source an opponent controls deals damage to you, that
        // player ...": the damaged player is you, so the only other player
        // the event names is the source's controller.
        TriggerSpec::DealsDamageToPlayer { source, player, .. }
        | TriggerSpec::DealsNoncombatDamageToPlayer { source, player, .. }
        | TriggerSpec::DealsCombatDamageToPlayer { source, player }
            if *player == PlayerFilter::You
                && source
                    .controller
                    .as_ref()
                    .is_some_and(|controller| *controller != PlayerFilter::You) =>
        {
            Some(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::TriggeringSource.bind(),
            )))
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
        | TriggerSpec::AttacksPlayerAlone(_)
        | TriggerSpec::BecomesBlockedByObjectWithLesserPower { .. } => {
            Some(PlayerFilter::Defending)
        }
        // "You attack with a creature an opponent owns": the attacker is
        // under your control; the explicitly named other player is its owner.
        TriggerSpec::Attacks(filter)
            if filter.controller == Some(PlayerFilter::You)
                && filter
                    .owner
                    .as_ref()
                    .is_some_and(|owner| *owner != PlayerFilter::You) =>
        {
            Some(PlayerFilter::AliasedOwnerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::Triggering.bind(),
            )))
        }
        // "... attack you ..., that player": you are the defender, so the
        // only player antecedent is the attacking player.
        TriggerSpec::Attacks(filter)
        | TriggerSpec::AttacksOneOrMore(filter)
        | TriggerSpec::AttacksAndIsntBlockedOneOrMore(filter)
            if filter.targets_only_player == Some(PlayerFilter::You) =>
        {
            Some(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::Triggering.bind(),
            )))
        }
        // "a player attacks you [or a planeswalker you control] ..., that
        // player": you are the defender, so "that player" is the attacker.
        TriggerSpec::Attacks(filter)
        | TriggerSpec::AttacksOneOrMore(filter)
        | TriggerSpec::AttacksOneOrMoreWithMinTotal { filter, .. }
        | TriggerSpec::AttacksOneOrMoreWithExactTotal { filter, .. }
        | TriggerSpec::AttacksOneOrMoreWithAggregate { filter, .. }
            if filter.attacking_player_or_planeswalker_controlled_by
                == Some(PlayerFilter::You) =>
        {
            Some(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::Triggering.bind(),
            )))
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
        // "Whenever an opponent attacks with creatures, ... that opponent":
        // the attacking player is the only player the event names.
        TriggerSpec::AttacksOneOrMore(filter)
            if filter
                .controller
                .as_ref()
                .is_some_and(|controller| *controller != PlayerFilter::You) =>
        {
            Some(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::Triggering.bind(),
            )))
        }
        TriggerSpec::AttacksYouOrPlaneswalkerYouControl(_)
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(_) => {
            Some(PlayerFilter::IteratedPlayer)
        }
        // "Whenever an opponent attacks you and/or one or more planeswalkers
        // you control, ... that player's library": the attacking player.
        TriggerSpec::PlayerAttacksOneOrMore { attacker, .. } if *attacker != PlayerFilter::You => {
            Some(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::Triggering.bind(),
            )))
        }
        TriggerSpec::PlayerAttacksTargetWithOneOrMore { .. } => {
            // In "an opponent attacks a planeswalker ... with one or more
            // creatures, ... that player", the discourse antecedent is the
            // attacking player. The concrete event participant is the
            // attacking creature, so retain its aliased controller instead of
            // binding the defending planeswalker's controller.
            Some(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::Triggering.bind(),
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
        TriggerSpec::PlayerBecomesTargeted { .. }
        | TriggerSpec::PlayerTurnsFaceUp { .. } => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::BeginningOfMonarchEndStep => Some(PlayerFilter::IteratedPlayer),
        TriggerSpec::BecomesTargetedBySourceController {
            source_controller, ..
        }
        | TriggerSpec::PlayerOrObjectBecomesTargetedBySourceController {
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
