//! Resolve a typed predicate into the condition it denotes.
//!
//! Translating a `PredicateAst` means binding the references it names — `it`,
//! the acting player, the antecedent object — against the reference
//! environment. That is reference resolution, not lowering, so it lives here
//! beside the helpers that do the binding, and both recognition and lowering
//! reach the same answer through it.

use crate::cards::builders::TurnHistoryPredicateAst;
use crate::cards::builders::{CardTextError, PredicateAst, TagKey, PlayerPredicateAst, SourcePredicateAst, TriggeringPredicateAst, TurnEventPredicateAst};
use crate::effect::{Condition, Value};
use crate::filter::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::model::reference_state::ReferenceEnv;
use crate::reference_helpers::{
    resolve_it_tag, resolve_it_tag_key, resolve_non_target_player_filter, resolve_value_it_tag,
};
use crate::tag_support::filter_references_tag;
use crate::types::CardType;
use crate::zone::Zone;
use ironsmith_compiler_semantic::model::DamageBySpec;
use ironsmith_compiler_semantic::model_impl::ast::TriggerFrequencyPredicateAst;
use ironsmith_core::DamagedBySource;

pub fn resolve_condition_from_predicate(
    predicate: &PredicateAst,
    refs: &ReferenceEnv,
    saved_last_tag: &Option<TagKey>,
) -> Result<Condition, CardTextError> {
    Ok(match predicate {
        PredicateAst::ItIsNight => Condition::ItIsNight,
        PredicateAst::FirstCombatPhaseOfTurn => Condition::FirstCombatPhaseOfTurn,
        PredicateAst::Source(SourcePredicateAst::SourceControllersMainPhase) => Condition::SourceControllersMainPhase,
        PredicateAst::ItIsLandCard => {
            let mut filter = ObjectFilter {
                zone: None,
                card_types: vec![CardType::Land],
                ..Default::default()
            };
            filter.zone = None;
            if let Some(tag) = saved_last_tag.clone() {
                Condition::TaggedObjectMatches(tag.into(), filter)
            } else {
                Condition::TargetMatches(filter)
            }
        }
        PredicateAst::ItIsSoulbondPaired => {
            if let Some(tag) = saved_last_tag.clone() {
                Condition::TaggedObjectIsSoulbondPaired(tag.into())
            } else {
                Condition::TargetIsSoulbondPaired
            }
        }
        PredicateAst::Source(SourcePredicateAst::SourceChosenOption(option)) => Condition::SourceChosenOption(option.clone()),
        PredicateAst::ItMatches(filter) => {
            let mut resolved = filter.clone();
            // A same-name relation whose right-hand side is still the
            // implicit `__it__` binding describes an existential comparison
            // set (for example, "it has the same name as a card in your
            // graveyard"). Preserve that set's zone for runtime evaluation;
            // ordinary identity predicates continue to ignore the referenced
            // object's current zone.
            let is_same_name_comparison_set = filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                    && constraint.relation == TaggedOpbjectRelation::SameNameAsTagged
            });
            if !is_same_name_comparison_set && resolved.zone == Some(Zone::Battlefield) {
                resolved.zone = None;
            }
            if let Some(tag) = saved_last_tag.clone() {
                Condition::TaggedObjectMatches(tag.into(), resolved)
            } else if refs.has_source_object_antecedent() {
                Condition::SourceMatches(resolved)
            } else {
                Condition::TargetMatches(resolved)
            }
        }
        PredicateAst::ItMatchedLastKnown(filter) => {
            let mut resolved = filter.clone();
            // Battlefield-origin identity predicates deliberately ignore the
            // filter constructor's live-zone restriction and consult only the
            // stored snapshot. A stack spell is different: `spell` is part of
            // the historical identity being tested, not merely its location.
            // Preserve that typed origin so both evaluation and rendering know
            // a countered object was a spell rather than a permanent.
            if resolved.zone == Some(Zone::Stack) {
                resolved
                    .stack_kind
                    .get_or_insert(crate::filter::StackObjectKind::Spell);
            } else {
                resolved.zone = None;
            }
            let tag = saved_last_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "past-tense object predicate has no snapshot-bearing antecedent".to_string(),
                )
            })?;
            Condition::TaggedObjectMatchedLastKnown(tag.into(), resolved)
        }
        PredicateAst::TargetMatches(filter) => {
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
            if let Some(tag) = saved_last_tag.clone()
                && !filter_references_tag(&resolved, tag.as_str())
            {
                Condition::TaggedObjectMatches(tag.into(), resolved)
            } else if resolved.source && resolved.zone != Some(Zone::Exile) {
                resolved.source = false;
                Condition::SourceMatches(resolved)
            } else {
                Condition::TargetMatches(resolved)
            }
        }
        PredicateAst::TaggedMatches(tag, filter) => {
            let resolved_tag = resolve_it_tag_key(tag, &refs)?;
            Condition::TaggedObjectMatches(resolved_tag, resolve_it_tag(filter, &refs)?)
        }
        PredicateAst::TaggedWasCast(tag) => {
            let resolved_tag = resolve_it_tag_key(tag, &refs)?;
            Condition::TaggedObjectWasCast(resolved_tag)
        }
        PredicateAst::EnchantedPermanentAttackedThisTurn => {
            Condition::EnchantedPermanentAttackedThisTurn
        }
        PredicateAst::EnchantedPermanentAttackedOrBlockedSinceLastUpkeep => {
            Condition::EnchantedPermanentAttackedOrBlockedSinceLastUpkeep
        }
        PredicateAst::Source(SourcePredicateAst::SourceBlockedOrBecameBlockedSinceLastUpkeep) => {
            Condition::SourceBlockedOrBecameBlockedSinceLastUpkeep
        }
        PredicateAst::Triggering(TriggeringPredicateAst::TriggeringObjectBecameTappedFirstTimeThisTurn) => {
            Condition::TriggeringObjectBecameTappedFirstTimeThisTurn
        }
        PredicateAst::Triggering(TriggeringPredicateAst::TriggeringObjectHadCountersPutFirstTimeThisTurn) => {
            Condition::TriggeringObjectHadCountersPutFirstTimeThisTurn
        }
        PredicateAst::TargetObjectsHaveDifferentColorSets => {
            Condition::TargetObjectsHaveDifferentColorSets
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerTaggedObjectMatches {
            player,
            tag,
            filter,
            mode,
        }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved_tag = resolve_it_tag_key(tag, &refs)?;
            Condition::PlayerTaggedObjectMatches {
                player,
                tag: resolved_tag,
                filter: resolve_it_tag(filter, &refs)?,
                mode: *mode,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerControls { player, filter }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved = resolve_it_tag(filter, &refs)?;
            Condition::PlayerControls {
                player,
                filter: resolved,
            }
        }
        PredicateAst::VoteOptionGetsMoreVotes { option } => {
            Condition::VoteOptionGetsMoreVotes(option.clone())
        }
        PredicateAst::SecretChoicesMatch => Condition::SecretChoicesMatch,
        PredicateAst::VoteOptionGetsMoreVotesOrTied { option } => {
            Condition::VoteOptionGetsMoreVotesOrTied(option.clone())
        }
        PredicateAst::NoVoteObjectsMatched { filter } => {
            Condition::Not(Box::new(Condition::TaggedObjectMatches(
                (crate::tag::CompilerReferenceTag::VotedObjects.bind()).into(),
                resolve_it_tag(filter, &refs)?,
            )))
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerHasAtLeast {
            player,
            filter,
            count,
        }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved = resolve_it_tag(filter, &refs)?;
            Condition::PlayerHasAtLeast {
                player,
                filter: resolved,
                count: *count,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerControlsExactly {
            player,
            filter,
            count,
        }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved = resolve_it_tag(filter, &refs)?;
            Condition::PlayerControlsExactly {
                player,
                filter: resolved,
                count: *count,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerHasAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved = resolve_it_tag(filter, &refs)?;
            Condition::PlayerHasAtLeastWithDifferentPowers {
                player,
                filter: resolved,
                count: *count,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerControlsOrHasCardInGraveyard {
            player,
            control_filter,
            graveyard_filter,
        }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved_control = resolve_it_tag(control_filter, &refs)?;
            resolved_control.zone = None;
            let resolved_graveyard = resolve_it_tag(graveyard_filter, &refs)?;
            Condition::Or(
                Box::new(Condition::PlayerControls {
                    player: player.clone(),
                    filter: resolved_control,
                }),
                Box::new(Condition::PlayerControls {
                    player,
                    filter: resolved_graveyard,
                }),
            )
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerOwnsCardNamedInZones {
            player,
            name,
            zones,
        }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerOwnsCardNamedInZones {
                player,
                name: name.clone(),
                zones: zones.clone(),
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerControlsNo { player, filter }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
            if player == PlayerFilter::Any {
                return Ok(Condition::PlayerControlsExactly {
                    player,
                    filter: resolved,
                    count: 0,
                });
            }
            Condition::Not(Box::new(Condition::PlayerControls {
                player,
                filter: resolved,
            }))
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerControlsMost { player, filter }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
            Condition::PlayerControlsMost {
                player,
                filter: resolved,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerControlsMoreThanEachOtherPlayer { player, filter }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
            Condition::PlayerControlsMoreThanEachOtherPlayer {
                player,
                filter: resolved,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerControlsMoreThanYou { player, filter }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
            Condition::PlayerControlsMoreThanYou {
                player,
                filter: resolved,
            }
        }
        PredicateAst::AnOpponentHasFewerThanPlayer { player, filter } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::AnOpponentHasFewerThanPlayer {
                player,
                filter: resolve_it_tag(filter, &refs)?,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerLifeAtMostHalfStartingLifeTotal { player }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerLifeLessThanHalfStartingLifeTotal { player }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerHasLessLifeThanYou { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasLessLifeThanYou { player }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerHasMoreLifeThanYou { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasMoreLifeThanYou { player }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerHasNoOpponentWithMoreLifeThan { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasNoOpponentWithMoreLifeThan { player }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasMoreLifeThanEachOtherPlayer { player }
        }
        PredicateAst::CountParity {
            count,
            even,
            display,
        } => Condition::CountParity {
            count: count.clone(),
            even: *even,
            display: display.clone(),
        },
        PredicateAst::Player(PlayerPredicateAst::PlayerIsMonarch { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerIsMonarch { player }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerHasInitiative { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasInitiative { player }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerHasCitysBlessing { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasCitysBlessing { player }
        }
        PredicateAst::Source(SourcePredicateAst::SourceIsRingBearer { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::SourceIsRingBearer { player }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerRingTemptedThisGameOrMore { player, count }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerRingTemptedThisGameOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerCompletedDungeon {
            player,
            dungeon_name,
        }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCompletedDungeon {
                player,
                dungeon_name: dungeon_name.clone(),
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerTappedLandForManaThisTurn { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerTappedLandForManaThisTurn { player }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerGainedLifeThisTurnOrMore { player, count }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerGainedLifeThisTurnOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::TurnEvents(TurnEventPredicateAst::CreatureDiedThisTurnOrMore(count)) => {
            Condition::CreatureDiedThisTurnOrMore(*count)
        }
        PredicateAst::TurnEvents(TurnEventPredicateAst::CreatureDealtDamageBySourceDiedThisTurn {
            victim,
            damager,
            count,
        }) => Condition::CreatureDealtDamageBySourceDiedThisTurn {
            victim: victim.clone(),
            damager: match damager {
                DamageBySpec::ThisCreature => DamagedBySource::ThisCreature,
                DamageBySpec::EquippedCreature => DamagedBySource::EquippedCreature,
                DamageBySpec::EnchantedCreature => DamagedBySource::EnchantedCreature,
            },
            count: *count,
        },
        PredicateAst::Player(PlayerPredicateAst::PlayerHadLandEnterBattlefieldThisTurn { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHadLandEnterBattlefieldThisTurn { player }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerDescendedThisTurn { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerDescendedThisTurn { player }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn { player, tag }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerTaggedObjectEnteredBattlefieldThisTurn {
                player,
                tag: tag.clone().into(),
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerControlsBasicLandTypesAmongLandsOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerHasCardTypesInGraveyardOrMore { player, count }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasCardTypesInGraveyardOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerCardsInHandOrMore { player, count }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCardsInHandOrMore {
                player,
                count: *count as i32,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerCardsInHandOrFewer { player, count }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCardsInHandOrFewer {
                player,
                count: *count as i32,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerCardsInHandAtTurnStartOrMore { player, count }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCardsInHandAtTurnStartOrMore {
                player,
                count: *count as i32,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerCardsInHandAtTurnStartOrFewer { player, count }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCardsInHandAtTurnStartOrFewer {
                player,
                count: *count as i32,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerHasMoreCardsInHandThanYou { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasMoreCardsInHandThanYou { player }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasMoreCardsInHandThanEachOtherPlayer { player }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerHasPoisonCountersOrMore { player, count }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasPoisonCountersOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerCastSpellsThisTurnOrMore { player, count }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCastSpellsThisTurnOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::TurnEvents(TurnEventPredicateAst::OpponentLostLifeThisTurn) => Condition::OpponentLostLifeThisTurn,
        PredicateAst::TurnEvents(TurnEventPredicateAst::AnyPlayerLostLifeThisTurnOrMore { count }) => {
            Condition::AnyPlayerLostLifeThisTurnOrMore { count: *count }
        }
        PredicateAst::TurnEvents(TurnEventPredicateAst::OpponentWasDealtDamageThisTurn) => Condition::OpponentWasDealtDamageThisTurn,
        PredicateAst::YouHaveNoCardsInHand => {
            Condition::Not(Box::new(Condition::CardsInHandOrMore(1)))
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerWouldDrawCard { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::Custom(match player {
                PlayerFilter::You => "you_would_draw_card".into(),
                PlayerFilter::Opponent => "opponent_would_draw_card".into(),
                _ => "player_would_draw_card".into(),
            })
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerWouldProliferate { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::Custom(match player {
                PlayerFilter::You => "you_would_proliferate".into(),
                PlayerFilter::Opponent => "opponent_would_proliferate".into(),
                _ => "player_would_proliferate".into(),
            })
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerWouldBeginExtraTurn { player }) => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::Custom(match player {
                PlayerFilter::Opponent => "opponent_would_begin_extra_turn".into(),
                _ => "player_would_begin_extra_turn".into(),
            })
        }
        PredicateAst::YourTurn => Condition::YourTurn,
        PredicateAst::TurnEvents(TurnEventPredicateAst::CreatureDiedThisTurn) => Condition::CreatureDiedThisTurn,
        PredicateAst::TurnEvents(TurnEventPredicateAst::CreatureCardPutIntoYourGraveyardThisTurn) => {
            Condition::CreatureCardPutIntoYourGraveyardThisTurn
        }
        PredicateAst::TurnEvents(TurnEventPredicateAst::PermanentLeftBattlefieldThisTurn) => {
            Condition::PermanentLeftBattlefieldThisTurn
        }
        PredicateAst::TurnEvents(TurnEventPredicateAst::NonlandPermanentLeftBattlefieldThisTurn) => {
            Condition::NonlandPermanentLeftBattlefieldThisTurn
        }
        PredicateAst::TurnEvents(TurnEventPredicateAst::SpellWasWarpedThisTurn) => Condition::SpellWasWarpedThisTurn,
        PredicateAst::TurnEvents(TurnEventPredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn { surface }) => {
            Condition::PermanentLeftBattlefieldUnderYourControlThisTurn { surface: *surface }
        }
        PredicateAst::TurnEvents(TurnEventPredicateAst::ObjectEnteredBattlefieldThisTurn(filter)) => {
            Condition::ObjectEnteredBattlefieldThisTurn(filter.clone())
        }
        PredicateAst::TurnEvents(TurnEventPredicateAst::ObjectEnteredBattlefieldLastTurn(filter)) => {
            Condition::ObjectEnteredBattlefieldLastTurn(filter.clone())
        }
        PredicateAst::TurnEvents(TurnEventPredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter)) => {
            Condition::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter.clone())
        }
        PredicateAst::Source(SourcePredicateAst::SourceIsTapped) => Condition::SourceIsTapped,
        PredicateAst::Source(SourcePredicateAst::SourceIsEquipped) => Condition::SourceIsEquipped,
        PredicateAst::Source(SourcePredicateAst::SourceIsEnchanted) => Condition::SourceIsEnchanted,
        PredicateAst::Source(SourcePredicateAst::SourceIsSaddled) => Condition::SourceIsSaddled,
        PredicateAst::Source(SourcePredicateAst::SourceIsRenowned) => Condition::SourceIsRenowned,
        PredicateAst::Source(SourcePredicateAst::SourceCrewedByExactly { count, filter }) => Condition::SourceCrewedByExactly {
            count: *count,
            filter: filter.clone(),
        },
        PredicateAst::Source(SourcePredicateAst::SourceMatches(filter)) => Condition::SourceMatches(filter.clone()),
        PredicateAst::AttachedToSourceMatches(filter) => {
            Condition::AttachedToSourceMatches(filter.clone())
        }
        PredicateAst::Triggering(TriggeringPredicateAst::TriggeringObjectHadToAttackThisCombat) => {
            Condition::TriggeringObjectHadToAttackThisCombat
        }
        PredicateAst::Source(SourcePredicateAst::SourceHasNoCounter(counter_type)) => {
            Condition::SourceHasNoCounter(*counter_type)
        }
        PredicateAst::Triggering(TriggeringPredicateAst::TriggeringObjectHadNoCounter(counter_type)) => {
            Condition::Not(Box::new(Condition::TriggeringObjectHadCounters {
                counter_type: *counter_type,
                min_count: 1,
            }))
        }
        PredicateAst::Triggering(TriggeringPredicateAst::TriggeringObjectHadCounterAtLeast {
            counter_type,
            count,
        }) => Condition::TriggeringObjectHadCounters {
            counter_type: *counter_type,
            min_count: *count,
        },
        PredicateAst::Source(SourcePredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count,
            surface,
        }) => Condition::SourceHasCounterAtLeast {
            counter_type: *counter_type,
            count: *count,
            surface: surface.clone(),
        },
        PredicateAst::Source(SourcePredicateAst::SourceHasCountersAtLeast(count)) => {
            Condition::SourceHasCountersAtLeast(*count)
        }
        PredicateAst::Source(SourcePredicateAst::SourceHasAttachmentsMatching {
            filter,
            comparison,
            display,
        }) => Condition::CountComparison {
            count: crate::static_abilities::AnthemCountExpression::AttachedToSource(filter.clone()),
            comparison: *comparison,
            display: Some(display.clone()),
        },
        PredicateAst::Source(SourcePredicateAst::SourcePowerAtLeast(count)) => Condition::SourcePowerAtLeast(*count),
        PredicateAst::Source(SourcePredicateAst::SourceDealtCombatDamageToPlayerThisTurn) => {
            Condition::SourceDealtCombatDamageToPlayerThisTurn
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype }) => {
            Condition::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn {
                player: resolve_non_target_player_filter(*player, &refs)?,
                subtype: *subtype,
            }
        }
        PredicateAst::Source(SourcePredicateAst::SourceAttackedThisTurn) => Condition::SourceAttackedThisTurn,
        PredicateAst::Source(SourcePredicateAst::SourceSuspected) => Condition::SourceSuspected,
        PredicateAst::Source(SourcePredicateAst::SourceCameUnderYourControlThisTurn) => {
            Condition::SourceCameUnderYourControlThisTurn
        }
        PredicateAst::Source(SourcePredicateAst::SourceAttackedOrBlockedThisTurn) => Condition::SourceAttackedOrBlockedThisTurn,
        PredicateAst::Source(SourcePredicateAst::SourceInGraveyardWithCardsAbove { filter, count }) => {
            Condition::SourceInGraveyardWithCardsAbove {
                filter: filter.clone(),
                count: *count,
            }
        }
        PredicateAst::Source(SourcePredicateAst::SourceIsInZone(zone)) => Condition::SourceIsInZone(*zone),
        PredicateAst::TurnEvents(TurnEventPredicateAst::YouAttackedThisTurn) => Condition::AttackedThisTurn,
        PredicateAst::TurnEvents(TurnEventPredicateAst::YouAttackedWithNOrMoreCreaturesThisTurn(count)) => {
            Condition::AttackedWithNOrMoreCreaturesThisTurn(*count)
        }
        PredicateAst::TurnEvents(TurnEventPredicateAst::YouAttackedWithExactlyNOtherCreaturesThisCombat(count)) => {
            return Err(CardTextError::ParseError(format!(
                "attack-count combat predicate should have been lowered into an exact attack trigger before condition compilation (count: {count})"
            )));
        }
        PredicateAst::Source(SourcePredicateAst::SourceWasCast) => Condition::SourceWasCast,
        PredicateAst::ThisSpellWasCastAtSorceryTiming => Condition::ThisSpellWasCastAtSorceryTiming,
        PredicateAst::ThisSpellEscaped => Condition::ThisSpellEscaped,
        PredicateAst::TurnEvents(TurnEventPredicateAst::NoSpellsWereCastLastTurn) => Condition::NoSpellsWereCastLastTurn,
        PredicateAst::YouHaveFullParty => Condition::YouHaveFullParty,
        PredicateAst::ThisSpellWasKicked => Condition::ThisSpellWasKicked,
        PredicateAst::ThisSpellPaidLabel(label) => Condition::ThisSpellPaidLabel(label.clone()),
        PredicateAst::CountComparison {
            count,
            comparison,
            display,
        } => Condition::CountComparison {
            count: count.clone(),
            comparison: comparison.clone(),
            display: display.clone(),
        },
        PredicateAst::Bound(condition) => condition.as_ref().clone(),
        PredicateAst::YouControl(filter) => Condition::YouControl(filter.clone()),
        PredicateAst::TurnEvents(TurnEventPredicateAst::AttackedThisTurn) => Condition::AttackedThisTurn,
        PredicateAst::Source(SourcePredicateAst::SourceAttackedBattleThisTurn) => Condition::SourceAttackedBattleThisTurn,
        PredicateAst::Source(SourcePredicateAst::SourceIsSoulbondPaired) => Condition::SourceIsSoulbondPaired,
        PredicateAst::LifeTotalOrLess(total) => Condition::LifeTotalOrLess(*total),
        PredicateAst::CardsInHandOrMore(count) => Condition::CardsInHandOrMore(*count),
        PredicateAst::Player(PlayerPredicateAst::PlayerRolledResultThisTurn { player, result }) => {
            Condition::PlayerRolledResultThisTurn {
                player: resolve_non_target_player_filter(*player, &refs)?,
                result: *result,
            }
        }
        PredicateAst::TaggedObjectIsTopOfLibrary { tag, player } => {
            Condition::TaggedObjectIsTopOfLibrary {
                tag: tag.clone().into(),
                player: resolve_non_target_player_filter(*player, &refs)?,
            }
        }
        PredicateAst::Source(SourcePredicateAst::SourceDevouredCreaturesOrMore(count)) => {
            Condition::SourceDevouredCreaturesOrMore(*count)
        }
        PredicateAst::XValueAtLeast(value) => Condition::XValueAtLeast(*value),
        PredicateAst::ColorsOfManaSpentToCastThisSpellOrMore(count) => {
            Condition::ColorsOfManaSpentToCastThisSpellOrMore(*count)
        }
        PredicateAst::Source(SourcePredicateAst::SourceControllersEndStep) => Condition::SourceControllersEndStep,
        PredicateAst::YouHaveCardInHandMatching(filter) => {
            Condition::YouHaveCardInHandMatching(filter.clone())
        }
        PredicateAst::YourFirstTurnsOfTheGameOrFewer(count) => {
            Condition::YourFirstTurnsOfTheGameOrFewer(*count)
        }
        PredicateAst::AttachmentCount {
            attachment,
            host,
            comparison,
            display,
        } => Condition::AttachmentCount {
            attachment: attachment.clone(),
            host: host.clone(),
            comparison: comparison.clone(),
            display: display.clone(),
        },
        PredicateAst::Player(PlayerPredicateAst::PlayerCommittedCrimeThisTurn { player }) => {
            Condition::PlayerCommittedCrimeThisTurn {
                player: resolve_non_target_player_filter(*player, &refs)?,
            }
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerRemovedDraftCardMatching {
            player,
            filter,
            with_cards_named,
        }) => Condition::PlayerRemovedDraftCardMatching {
            player: resolve_non_target_player_filter(*player, &refs)?,
            filter: filter.clone(),
            with_cards_named: with_cards_named.clone(),
        },
        PredicateAst::Source(SourcePredicateAst::SourceIsAttacking) => Condition::SourceIsAttacking,
        PredicateAst::Source(SourcePredicateAst::SourceIsUntapped) => Condition::SourceIsUntapped,
        PredicateAst::Source(SourcePredicateAst::SourceIsMonstrous) => Condition::SourceIsMonstrous,
        PredicateAst::EquippedCreatureAttacking => Condition::EquippedCreatureAttacking,
        PredicateAst::EquippedCreatureTapped => Condition::EquippedCreatureTapped,
        PredicateAst::EquippedCreatureUntapped => Condition::EquippedCreatureUntapped,
        PredicateAst::EnchantedPermanentIsCreature => Condition::EnchantedPermanentIsCreature,
        PredicateAst::EnchantedPermanentIsLand => Condition::EnchantedPermanentIsLand,
        PredicateAst::EnchantedPermanentIsEquipment => Condition::EnchantedPermanentIsEquipment,
        PredicateAst::EnchantedPermanentIsVehicle => Condition::EnchantedPermanentIsVehicle,
        PredicateAst::ControlCreaturesTotalPowerAtLeast(total) => {
            Condition::ControlCreaturesTotalPowerAtLeast(*total)
        }
        PredicateAst::CardInYourGraveyard {
            card_types,
            subtypes,
        } => Condition::CardInYourGraveyard {
            card_types: card_types.clone(),
            subtypes: subtypes.clone(),
        },
        PredicateAst::ActivationTiming(timing) => Condition::ActivationTiming(*timing),
        PredicateAst::MaxActivationsPerTurn(limit) => Condition::MaxActivationsPerTurn(*limit),
        PredicateAst::CurrentTurnIsExtra => Condition::CurrentTurnIsExtra,
        PredicateAst::TriggerFrequency(frequency) => match frequency {
            TriggerFrequencyPredicateAst::FirstTimeThisTurn => Condition::FirstTimeThisTurn,
            TriggerFrequencyPredicateAst::SourceFirstCrewedThisTurn => {
                Condition::SourceFirstCrewedThisTurn
            }
            TriggerFrequencyPredicateAst::MaxTimesEachTurn(limit) => {
                Condition::MaxTimesEachTurn(*limit)
            }
            TriggerFrequencyPredicateAst::DoThisMaxTimesEachTurn(limit) => {
                Condition::DoThisMaxTimesEachTurn(*limit)
            }
        },
        PredicateAst::TargetWasKicked => Condition::TargetWasKicked,
        PredicateAst::TurnEvents(TurnEventPredicateAst::ThisAbilityResolvedThisTurnExactly(count)) => {
            Condition::ThisAbilityResolvedThisTurnExactly(*count)
        }
        PredicateAst::TargetSpellCastOrderThisTurn(order) => {
            Condition::TargetSpellCastOrderThisTurn(*order)
        }
        PredicateAst::TargetSpellControllerIsPoisoned => Condition::TargetSpellControllerIsPoisoned,
        PredicateAst::TargetSpellNoManaSpentToCast => {
            Condition::Not(Box::new(Condition::TargetSpellManaSpentToCastAtLeast {
                amount: 1,
                symbol: None,
            }))
        }
        PredicateAst::YouControlMoreCreaturesThanTargetSpellController => {
            Condition::YouControlMoreCreaturesThanTargetSpellController
        }
        PredicateAst::TargetIsBlocked => Condition::TargetIsBlocked,
        PredicateAst::TargetHasGreatestPowerAmongCreatures => {
            Condition::TargetHasGreatestPowerAmongCreatures
        }
        PredicateAst::TargetManaValueLteColorsSpentToCastThisSpell => {
            Condition::TargetManaValueLteColorsSpentToCastThisSpell
        }
        PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
            Condition::ManaSpentToCastThisSpellAtLeast {
                amount: *amount,
                symbol: *symbol,
            }
        }
        PredicateAst::Triggering(TriggeringPredicateAst::TriggeringSpellManaSpentToCastAtLeast { amount, symbol }) => {
            Condition::TriggeringSpellManaSpentToCastAtLeast {
                amount: *amount,
                symbol: *symbol,
            }
        }
        PredicateAst::ColoredManaSpentToCastThisSpellAtLeast(amount) => {
            Condition::ColoredManaSpentToCastThisSpellAtLeast(*amount)
        }
        PredicateAst::Triggering(TriggeringPredicateAst::TriggeringSpellColoredManaSpentToCastAtLeast(amount)) => {
            Condition::TriggeringSpellColoredManaSpentToCastAtLeast(*amount)
        }
        PredicateAst::SnowManaOfAnySpellColorSpentToCastThisSpell => {
            Condition::SnowManaOfAnySpellColorSpentToCastThisSpell
        }
        PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(amount) => {
            Condition::SameColorManaSpentToCastThisSpellAtLeast(*amount)
        }
        PredicateAst::ThisSpellWasCastFromZone(zone) => Condition::ThisSpellWasCastFromZone(*zone),
        PredicateAst::ThisSpellWasCastFromNonHand => Condition::ThisSpellWasCastFromNonHand,
        PredicateAst::TurnHistory(predicate) => Condition::TurnHistory(match predicate {
            TurnHistoryPredicateAst::SpellsCastLastTurnAtLeast(count) => {
                ironsmith_core::TurnHistoryCondition::SpellsCastLastTurnAtLeast(*count)
            }
            TurnHistoryPredicateAst::SourceCrewedByAtLeast { count, filter } => {
                ironsmith_core::TurnHistoryCondition::SourceCrewedByAtLeast {
                    count: *count,
                    filter: resolve_it_tag(filter, &refs)?,
                }
            }
            TurnHistoryPredicateAst::SourceWasCast { surface } => {
                ironsmith_core::TurnHistoryCondition::SourceWasCast {
                    surface: surface.clone(),
                }
            }
            TurnHistoryPredicateAst::SourceWasCastByController { surface } => {
                ironsmith_core::TurnHistoryCondition::SourceWasCastByController {
                    surface: surface.clone(),
                }
            }
            TurnHistoryPredicateAst::SourceWasKicked { surface } => {
                ironsmith_core::TurnHistoryCondition::SourceWasKicked {
                    surface: surface.clone(),
                }
            }
            TurnHistoryPredicateAst::SourceEnteredBattlefieldThisTurn { surface } => {
                ironsmith_core::TurnHistoryCondition::SourceEnteredBattlefieldThisTurn {
                    surface: surface.clone(),
                }
            }
            TurnHistoryPredicateAst::SourceAttackedThisTurn { surface } => {
                ironsmith_core::TurnHistoryCondition::SourceAttackedThisTurn {
                    surface: surface.clone(),
                }
            }
            TurnHistoryPredicateAst::TriggeringObjectEnlistedThisCombat => {
                ironsmith_core::TurnHistoryCondition::TriggeringObjectEnlistedThisCombat
            }
            TurnHistoryPredicateAst::TriggeringObjectWasCast => {
                ironsmith_core::TurnHistoryCondition::TriggeringObjectWasCast
            }
            TurnHistoryPredicateAst::TriggeringObjectWasCastFromZone(zone) => {
                ironsmith_core::TurnHistoryCondition::TriggeringObjectWasCastFromZone(*zone)
            }
            TurnHistoryPredicateAst::PlayerPlayedLandThisTurn(player) => {
                ironsmith_core::TurnHistoryCondition::PlayerPlayedLandThisTurn(
                    resolve_non_target_player_filter(*player, &refs)?,
                )
            }
            TurnHistoryPredicateAst::TriggeringObjectDied => {
                ironsmith_core::TurnHistoryCondition::TriggeringObjectDied
            }
            TurnHistoryPredicateAst::PlayerPlayedCardFromZoneThisTurn { player, zone } => {
                ironsmith_core::TurnHistoryCondition::PlayerPlayedCardFromZoneThisTurn {
                    player: resolve_non_target_player_filter(*player, &refs)?,
                    zone: *zone,
                }
            }
            TurnHistoryPredicateAst::PlayerCastSpellFromZoneThisTurn { player, zone } => {
                ironsmith_core::TurnHistoryCondition::PlayerCastSpellFromZoneThisTurn {
                    player: resolve_non_target_player_filter(*player, &refs)?,
                    zone: *zone,
                }
            }
            TurnHistoryPredicateAst::PlayerActivatedAbilityOfCardInZoneThisTurn {
                player,
                zone,
            } => ironsmith_core::TurnHistoryCondition::PlayerActivatedAbilityOfCardInZoneThisTurn {
                player: resolve_non_target_player_filter(*player, &refs)?,
                zone: *zone,
            },
            TurnHistoryPredicateAst::PlayerVisitedAttractionThisTurn(player) => {
                ironsmith_core::TurnHistoryCondition::PlayerVisitedAttractionThisTurn(
                    resolve_non_target_player_filter(*player, &refs)?,
                )
            }
            TurnHistoryPredicateAst::TriggeringPlayerAttackedControllerLastTurn => {
                ironsmith_core::TurnHistoryCondition::TriggeringPlayerAttackedControllerLastTurn
            }
            TurnHistoryPredicateAst::PlayerLostLifeLastTurn(player) => {
                ironsmith_core::TurnHistoryCondition::PlayerLostLifeLastTurn(
                    resolve_non_target_player_filter(*player, &refs)?,
                )
            }
            TurnHistoryPredicateAst::TriggeringPlayersTurn { definite_player } => {
                ironsmith_core::TurnHistoryCondition::TriggeringPlayersTurn {
                    definite_player: *definite_player,
                }
            }
            TurnHistoryPredicateAst::ControllerTeamGainedLifeThisTurn => {
                ironsmith_core::TurnHistoryCondition::ControllerTeamGainedLifeThisTurn
            }
            TurnHistoryPredicateAst::TriggeringObjectsNoneWereCastOrNoManaSpent => {
                ironsmith_core::TurnHistoryCondition::TriggeringObjectsNoneWereCastOrNoManaSpent
            }
            TurnHistoryPredicateAst::ManaFromSourceSpentOnTriggeringAction { source_filter } => {
                ironsmith_core::TurnHistoryCondition::ManaFromSourceSpentOnTriggeringAction {
                    source_filter: resolve_it_tag(source_filter, &refs)?,
                }
            }
            TurnHistoryPredicateAst::AllPlayersLifeAtMost(amount) => {
                ironsmith_core::TurnHistoryCondition::AllPlayersLifeAtMost(*amount)
            }
            TurnHistoryPredicateAst::AnotherOpponentControlsPotentialTarget { filter } => {
                ironsmith_core::TurnHistoryCondition::AnotherOpponentControlsPotentialTarget {
                    filter: resolve_it_tag(filter, &refs)?,
                }
            }
            TurnHistoryPredicateAst::TriggeringAttackerBlockers {
                required,
                required_count,
                prohibited,
            } => ironsmith_core::TurnHistoryCondition::TriggeringAttackerBlockers {
                required: resolve_it_tag(required, &refs)?,
                required_count: *required_count,
                prohibited: resolve_it_tag(prohibited, &refs)?,
            },
            TurnHistoryPredicateAst::TriggeringAbilityIsManaAbility => {
                ironsmith_core::TurnHistoryCondition::TriggeringAbilityIsManaAbility
            }
        }),
        PredicateAst::ValueComparison {
            left,
            operator,
            right,
        } => {
            if let (
                Value::X,
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(amount),
            ) = (left, operator, right)
                && *amount >= 0
            {
                Condition::XValueAtLeast(*amount as u32)
            } else if let (
                Value::TotalPower(filter),
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(amount),
            ) = (left, operator, right)
                && *amount >= 0
                && *filter == ObjectFilter::creature().you_control()
            {
                Condition::ControlCreaturesTotalPowerAtLeast(*amount as u32)
            } else {
                Condition::ValueComparison {
                    left: resolve_value_it_tag(left, &refs)?,
                    operator: *operator,
                    right: resolve_value_it_tag(right, &refs)?,
                }
            }
        }
        PredicateAst::ValueIsPrime(value) => {
            Condition::ValueIsPrime(resolve_value_it_tag(value, &refs)?)
        }
        PredicateAst::Not(inner) => {
            let inner = resolve_condition_from_predicate(inner, refs, saved_last_tag)?;
            Condition::Not(Box::new(inner))
        }
        PredicateAst::And(left, right) => {
            let left = resolve_condition_from_predicate(left, refs, saved_last_tag)?;
            let right = resolve_condition_from_predicate(right, refs, saved_last_tag)?;
            Condition::And(Box::new(left), Box::new(right))
        }
        PredicateAst::Or(left, right) => {
            let left = resolve_condition_from_predicate(left, refs, saved_last_tag)?;
            let right = resolve_condition_from_predicate(right, refs, saved_last_tag)?;
            Condition::Or(Box::new(left), Box::new(right))
        }
    })
}
