//! Resolve a typed predicate into the condition it denotes.
//!
//! Translating a `PredicateAst` means binding the references it names — `it`,
//! the acting player, the antecedent object — against the reference
//! environment. That is reference resolution, not lowering, so it lives here
//! beside the helpers that do the binding, and both recognition and lowering
//! reach the same answer through it.

use crate::cards::builders::TurnHistoryPredicateAst;
use crate::cards::builders::{CardTextError, PredicateAst, TagKey};
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
        PredicateAst::SourceControllersMainPhase => Condition::SourceControllersMainPhase,
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
        PredicateAst::SourceChosenOption(option) => Condition::SourceChosenOption(option.clone()),
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
            if !is_same_name_comparison_set && resolved.zone != Some(Zone::Stack) {
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
        PredicateAst::SourceBlockedOrBecameBlockedSinceLastUpkeep => {
            Condition::SourceBlockedOrBecameBlockedSinceLastUpkeep
        }
        PredicateAst::TriggeringObjectBecameTappedFirstTimeThisTurn => {
            Condition::TriggeringObjectBecameTappedFirstTimeThisTurn
        }
        PredicateAst::TriggeringObjectHadCountersPutFirstTimeThisTurn => {
            Condition::TriggeringObjectHadCountersPutFirstTimeThisTurn
        }
        PredicateAst::TargetObjectsHaveDifferentColorSets => {
            Condition::TargetObjectsHaveDifferentColorSets
        }
        PredicateAst::PlayerTaggedObjectMatches {
            player,
            tag,
            filter,
            mode,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved_tag = resolve_it_tag_key(tag, &refs)?;
            Condition::PlayerTaggedObjectMatches {
                player,
                tag: resolved_tag,
                filter: resolve_it_tag(filter, &refs)?,
                mode: *mode,
            }
        }
        PredicateAst::PlayerControls { player, filter } => {
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
                crate::tag::CompilerReferenceTag::VotedObjects
                    .as_str()
                    .into(),
                resolve_it_tag(filter, &refs)?,
            )))
        }
        PredicateAst::PlayerHasAtLeast {
            player,
            filter,
            count,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved = resolve_it_tag(filter, &refs)?;
            Condition::PlayerHasAtLeast {
                player,
                filter: resolved,
                count: *count,
            }
        }
        PredicateAst::PlayerControlsExactly {
            player,
            filter,
            count,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved = resolve_it_tag(filter, &refs)?;
            Condition::PlayerControlsExactly {
                player,
                filter: resolved,
                count: *count,
            }
        }
        PredicateAst::PlayerHasAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let resolved = resolve_it_tag(filter, &refs)?;
            Condition::PlayerHasAtLeastWithDifferentPowers {
                player,
                filter: resolved,
                count: *count,
            }
        }
        PredicateAst::PlayerControlsOrHasCardInGraveyard {
            player,
            control_filter,
            graveyard_filter,
        } => {
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
        PredicateAst::PlayerOwnsCardNamedInZones {
            player,
            name,
            zones,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerOwnsCardNamedInZones {
                player,
                name: name.clone(),
                zones: zones.clone(),
            }
        }
        PredicateAst::PlayerControlsNo { player, filter } => {
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
        PredicateAst::PlayerControlsMost { player, filter } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
            Condition::PlayerControlsMost {
                player,
                filter: resolved,
            }
        }
        PredicateAst::PlayerControlsMoreThanEachOtherPlayer { player, filter } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            let mut resolved = resolve_it_tag(filter, &refs)?;
            resolved.zone = None;
            Condition::PlayerControlsMoreThanEachOtherPlayer {
                player,
                filter: resolved,
            }
        }
        PredicateAst::PlayerControlsMoreThanYou { player, filter } => {
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
        PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerLifeAtMostHalfStartingLifeTotal { player }
        }
        PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerLifeLessThanHalfStartingLifeTotal { player }
        }
        PredicateAst::PlayerHasLessLifeThanYou { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasLessLifeThanYou { player }
        }
        PredicateAst::PlayerHasMoreLifeThanYou { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasMoreLifeThanYou { player }
        }
        PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasNoOpponentWithMoreLifeThan { player }
        }
        PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { player } => {
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
        PredicateAst::PlayerIsMonarch { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerIsMonarch { player }
        }
        PredicateAst::PlayerHasInitiative { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasInitiative { player }
        }
        PredicateAst::PlayerHasCitysBlessing { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasCitysBlessing { player }
        }
        PredicateAst::SourceIsRingBearer { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::SourceIsRingBearer { player }
        }
        PredicateAst::PlayerRingTemptedThisGameOrMore { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerRingTemptedThisGameOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::PlayerCompletedDungeon {
            player,
            dungeon_name,
        } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCompletedDungeon {
                player,
                dungeon_name: dungeon_name.clone(),
            }
        }
        PredicateAst::PlayerTappedLandForManaThisTurn { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerTappedLandForManaThisTurn { player }
        }
        PredicateAst::PlayerGainedLifeThisTurnOrMore { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerGainedLifeThisTurnOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::CreatureDiedThisTurnOrMore(count) => {
            Condition::CreatureDiedThisTurnOrMore(*count)
        }
        PredicateAst::CreatureDealtDamageBySourceDiedThisTurn {
            victim,
            damager,
            count,
        } => Condition::CreatureDealtDamageBySourceDiedThisTurn {
            victim: victim.clone(),
            damager: match damager {
                DamageBySpec::ThisCreature => DamagedBySource::ThisCreature,
                DamageBySpec::EquippedCreature => DamagedBySource::EquippedCreature,
                DamageBySpec::EnchantedCreature => DamagedBySource::EnchantedCreature,
            },
            count: *count,
        },
        PredicateAst::PlayerHadLandEnterBattlefieldThisTurn { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHadLandEnterBattlefieldThisTurn { player }
        }
        PredicateAst::PlayerDescendedThisTurn { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerDescendedThisTurn { player }
        }
        PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn { player, tag } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerTaggedObjectEnteredBattlefieldThisTurn {
                player,
                tag: tag.clone(),
            }
        }
        PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerControlsBasicLandTypesAmongLandsOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::PlayerHasCardTypesInGraveyardOrMore { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasCardTypesInGraveyardOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::PlayerCardsInHandOrMore { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCardsInHandOrMore {
                player,
                count: *count as i32,
            }
        }
        PredicateAst::PlayerCardsInHandOrFewer { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCardsInHandOrFewer {
                player,
                count: *count as i32,
            }
        }
        PredicateAst::PlayerCardsInHandAtTurnStartOrMore { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCardsInHandAtTurnStartOrMore {
                player,
                count: *count as i32,
            }
        }
        PredicateAst::PlayerCardsInHandAtTurnStartOrFewer { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCardsInHandAtTurnStartOrFewer {
                player,
                count: *count as i32,
            }
        }
        PredicateAst::PlayerHasMoreCardsInHandThanYou { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasMoreCardsInHandThanYou { player }
        }
        PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasMoreCardsInHandThanEachOtherPlayer { player }
        }
        PredicateAst::PlayerHasPoisonCountersOrMore { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerHasPoisonCountersOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::PlayerCastSpellsThisTurnOrMore { player, count } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::PlayerCastSpellsThisTurnOrMore {
                player,
                count: *count,
            }
        }
        PredicateAst::OpponentLostLifeThisTurn => Condition::OpponentLostLifeThisTurn,
        PredicateAst::AnyPlayerLostLifeThisTurnOrMore { count } => {
            Condition::AnyPlayerLostLifeThisTurnOrMore { count: *count }
        }
        PredicateAst::OpponentWasDealtDamageThisTurn => Condition::OpponentWasDealtDamageThisTurn,
        PredicateAst::YouHaveNoCardsInHand => {
            Condition::Not(Box::new(Condition::CardsInHandOrMore(1)))
        }
        PredicateAst::PlayerWouldDrawCard { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::Custom(match player {
                PlayerFilter::You => "you_would_draw_card".into(),
                PlayerFilter::Opponent => "opponent_would_draw_card".into(),
                _ => "player_would_draw_card".into(),
            })
        }
        PredicateAst::PlayerWouldProliferate { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::Custom(match player {
                PlayerFilter::You => "you_would_proliferate".into(),
                PlayerFilter::Opponent => "opponent_would_proliferate".into(),
                _ => "player_would_proliferate".into(),
            })
        }
        PredicateAst::PlayerWouldBeginExtraTurn { player } => {
            let player = resolve_non_target_player_filter(*player, &refs)?;
            Condition::Custom(match player {
                PlayerFilter::Opponent => "opponent_would_begin_extra_turn".into(),
                _ => "player_would_begin_extra_turn".into(),
            })
        }
        PredicateAst::YourTurn => Condition::YourTurn,
        PredicateAst::CreatureDiedThisTurn => Condition::CreatureDiedThisTurn,
        PredicateAst::CreatureCardPutIntoYourGraveyardThisTurn => {
            Condition::CreatureCardPutIntoYourGraveyardThisTurn
        }
        PredicateAst::PermanentLeftBattlefieldThisTurn => {
            Condition::PermanentLeftBattlefieldThisTurn
        }
        PredicateAst::NonlandPermanentLeftBattlefieldThisTurn => {
            Condition::NonlandPermanentLeftBattlefieldThisTurn
        }
        PredicateAst::SpellWasWarpedThisTurn => Condition::SpellWasWarpedThisTurn,
        PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn { surface } => {
            Condition::PermanentLeftBattlefieldUnderYourControlThisTurn { surface: *surface }
        }
        PredicateAst::ObjectEnteredBattlefieldThisTurn(filter) => {
            Condition::ObjectEnteredBattlefieldThisTurn(filter.clone())
        }
        PredicateAst::ObjectEnteredBattlefieldLastTurn(filter) => {
            Condition::ObjectEnteredBattlefieldLastTurn(filter.clone())
        }
        PredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter) => {
            Condition::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter.clone())
        }
        PredicateAst::SourceIsTapped => Condition::SourceIsTapped,
        PredicateAst::SourceIsEquipped => Condition::SourceIsEquipped,
        PredicateAst::SourceIsEnchanted => Condition::SourceIsEnchanted,
        PredicateAst::SourceIsSaddled => Condition::SourceIsSaddled,
        PredicateAst::SourceIsRenowned => Condition::SourceIsRenowned,
        PredicateAst::SourceCrewedByExactly { count, filter } => Condition::SourceCrewedByExactly {
            count: *count,
            filter: filter.clone(),
        },
        PredicateAst::SourceMatches(filter) => Condition::SourceMatches(filter.clone()),
        PredicateAst::AttachedToSourceMatches(filter) => {
            Condition::AttachedToSourceMatches(filter.clone())
        }
        PredicateAst::TriggeringObjectHadToAttackThisCombat => {
            Condition::TriggeringObjectHadToAttackThisCombat
        }
        PredicateAst::SourceHasNoCounter(counter_type) => {
            Condition::SourceHasNoCounter(*counter_type)
        }
        PredicateAst::TriggeringObjectHadNoCounter(counter_type) => {
            Condition::Not(Box::new(Condition::TriggeringObjectHadCounters {
                counter_type: *counter_type,
                min_count: 1,
            }))
        }
        PredicateAst::TriggeringObjectHadCounterAtLeast {
            counter_type,
            count,
        } => Condition::TriggeringObjectHadCounters {
            counter_type: *counter_type,
            min_count: *count,
        },
        PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count,
            surface,
        } => Condition::SourceHasCounterAtLeast {
            counter_type: *counter_type,
            count: *count,
            surface: surface.clone(),
        },
        PredicateAst::SourceHasCountersAtLeast(count) => {
            Condition::SourceHasCountersAtLeast(*count)
        }
        PredicateAst::SourceHasAttachmentsMatching {
            filter,
            comparison,
            display,
        } => Condition::CountComparison {
            count: crate::static_abilities::AnthemCountExpression::AttachedToSource(filter.clone()),
            comparison: *comparison,
            display: Some(display.clone()),
        },
        PredicateAst::SourcePowerAtLeast(count) => Condition::SourcePowerAtLeast(*count),
        PredicateAst::SourceDealtCombatDamageToPlayerThisTurn => {
            Condition::SourceDealtCombatDamageToPlayerThisTurn
        }
        PredicateAst::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype } => {
            Condition::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn {
                player: resolve_non_target_player_filter(*player, &refs)?,
                subtype: *subtype,
            }
        }
        PredicateAst::SourceAttackedThisTurn => Condition::SourceAttackedThisTurn,
        PredicateAst::SourceSuspected => Condition::SourceSuspected,
        PredicateAst::SourceCameUnderYourControlThisTurn => {
            Condition::SourceCameUnderYourControlThisTurn
        }
        PredicateAst::SourceAttackedOrBlockedThisTurn => Condition::SourceAttackedOrBlockedThisTurn,
        PredicateAst::SourceInGraveyardWithCardsAbove { filter, count } => {
            Condition::SourceInGraveyardWithCardsAbove {
                filter: filter.clone(),
                count: *count,
            }
        }
        PredicateAst::SourceIsInZone(zone) => Condition::SourceIsInZone(*zone),
        PredicateAst::YouAttackedThisTurn => Condition::AttackedThisTurn,
        PredicateAst::YouAttackedWithNOrMoreCreaturesThisTurn(count) => {
            Condition::AttackedWithNOrMoreCreaturesThisTurn(*count)
        }
        PredicateAst::YouAttackedWithExactlyNOtherCreaturesThisCombat(count) => {
            return Err(CardTextError::ParseError(format!(
                "attack-count combat predicate should have been lowered into an exact attack trigger before condition compilation (count: {count})"
            )));
        }
        PredicateAst::SourceWasCast => Condition::SourceWasCast,
        PredicateAst::ThisSpellWasCastAtSorceryTiming => Condition::ThisSpellWasCastAtSorceryTiming,
        PredicateAst::ThisSpellEscaped => Condition::ThisSpellEscaped,
        PredicateAst::NoSpellsWereCastLastTurn => Condition::NoSpellsWereCastLastTurn,
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
        PredicateAst::AttackedThisTurn => Condition::AttackedThisTurn,
        PredicateAst::SourceAttackedBattleThisTurn => Condition::SourceAttackedBattleThisTurn,
        PredicateAst::SourceIsSoulbondPaired => Condition::SourceIsSoulbondPaired,
        PredicateAst::LifeTotalOrLess(total) => Condition::LifeTotalOrLess(*total),
        PredicateAst::CardsInHandOrMore(count) => Condition::CardsInHandOrMore(*count),
        PredicateAst::PlayerRolledResultThisTurn { player, result } => {
            Condition::PlayerRolledResultThisTurn {
                player: resolve_non_target_player_filter(*player, &refs)?,
                result: *result,
            }
        }
        PredicateAst::TaggedObjectIsTopOfLibrary { tag, player } => {
            Condition::TaggedObjectIsTopOfLibrary {
                tag: tag.clone(),
                player: resolve_non_target_player_filter(*player, &refs)?,
            }
        }
        PredicateAst::SourceDevouredCreaturesOrMore(count) => {
            Condition::SourceDevouredCreaturesOrMore(*count)
        }
        PredicateAst::XValueAtLeast(value) => Condition::XValueAtLeast(*value),
        PredicateAst::ColorsOfManaSpentToCastThisSpellOrMore(count) => {
            Condition::ColorsOfManaSpentToCastThisSpellOrMore(*count)
        }
        PredicateAst::SourceControllersEndStep => Condition::SourceControllersEndStep,
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
        PredicateAst::PlayerCommittedCrimeThisTurn { player } => {
            Condition::PlayerCommittedCrimeThisTurn {
                player: resolve_non_target_player_filter(*player, &refs)?,
            }
        }
        PredicateAst::PlayerRemovedDraftCardMatching {
            player,
            filter,
            with_cards_named,
        } => Condition::PlayerRemovedDraftCardMatching {
            player: resolve_non_target_player_filter(*player, &refs)?,
            filter: filter.clone(),
            with_cards_named: with_cards_named.clone(),
        },
        PredicateAst::SourceIsAttacking => Condition::SourceIsAttacking,
        PredicateAst::SourceIsUntapped => Condition::SourceIsUntapped,
        PredicateAst::SourceIsMonstrous => Condition::SourceIsMonstrous,
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
        PredicateAst::ThisAbilityResolvedThisTurnExactly(count) => {
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
        PredicateAst::TriggeringSpellManaSpentToCastAtLeast { amount, symbol } => {
            Condition::TriggeringSpellManaSpentToCastAtLeast {
                amount: *amount,
                symbol: *symbol,
            }
        }
        PredicateAst::ColoredManaSpentToCastThisSpellAtLeast(amount) => {
            Condition::ColoredManaSpentToCastThisSpellAtLeast(*amount)
        }
        PredicateAst::TriggeringSpellColoredManaSpentToCastAtLeast(amount) => {
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
