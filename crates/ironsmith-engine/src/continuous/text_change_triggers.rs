//! Immutable authored-word traversal of complete typed trigger definitions.
//!
//! Retained models are the semantic input. Native matchers enter only through
//! an exact, field-complete adapter below; opaque callbacks and unsupported
//! combinations remain checked holds. Presentation strings never select an
//! execution path. Captured event snapshots and player choices are not models
//! of authored words and are never traversed here.

use super::text_change_predicates::{
    rewrite_choose_spec_words, rewrite_condition_words, rewrite_filter_comparison_words,
    rewrite_filter_words, rewrite_player_filter_words,
};
use super::text_changes::TextChangeDomainError as Error;
use crate::triggers::{Trigger, TriggerIntroSurface};
use ironsmith_core::{ObjectFilter, PlayerFilter, TextChange, trigger_model as model};

/// The caller memoizes changed immutable definitions. No-op rewrites preserve
/// the original matcher allocation, including any earlier captured clone.
pub(crate) fn rewrite_trigger_words(trigger: &Trigger, change: TextChange) -> Result<Trigger, Error> {
    let original = complete_model(trigger)?;
    let rewritten = rewrite_trigger_model_words(&original, change)?;
    if rewritten == original { return Ok(trigger.clone()); }
    let mut result = Trigger::from_model(rewritten).map_err(|_| Error::TriggeredAbility)?;
    if let Some(intro) = trigger.intro_surface() { result = result.with_intro_surface(intro); }
    Ok(result)
}

fn rewrite_optional_filter(filter: &mut Option<ObjectFilter>, change: TextChange) -> Result<(), Error> {
    if let Some(filter) = filter { *filter = rewrite_filter_words(filter, change)?; }
    Ok(())
}

fn rewrite_optional_player(player: &mut Option<PlayerFilter>, change: TextChange) -> Result<(), Error> {
    if let Some(player) = player { *player = rewrite_player_filter_words(player, change)?; }
    Ok(())
}

fn rewrite_cause_words(cause: &mut Option<ironsmith_core::CauseFilter>, change: TextChange)
    -> Result<(), Error>
{
    if let Some(ironsmith_core::CauseFilter {
        cause_type: _, source_filter, controller_filter: _,
    }) = cause {
        rewrite_optional_filter(source_filter, change)?;
    }
    Ok(())
}

fn rewrite_attack_target_words(target: &mut model::AttackTargetRestriction, change: TextChange)
    -> Result<(), Error>
{
    match target {
        model::AttackTargetRestriction::Player(player)
        | model::AttackTargetRestriction::PlaneswalkerControlledBy(player)
        | model::AttackTargetRestriction::PlayerOrPlaneswalkerControlledBy(player) => {
            *player = rewrite_player_filter_words(player, change)?;
        }
    }
    Ok(())
}

fn rewrite_zone_change_words(zone: &mut model::ZoneChangeTrigger, change: TextChange)
    -> Result<(), Error>
{
    let model::ZoneChangeTrigger {
        from: _, from_zones: _, from_excluded: _, to: _, to_excluded: _,
        filter, this: _, this_surface: _, this_subject_number: _, count: _,
        cause_filter, during_own_resolution: _, during_turn, timing: _,
        origin_condition, graveyard_surface: _,
    } = zone;
    rewrite_optional_filter(filter, change)?;
    rewrite_cause_words(cause_filter, change)?;
    rewrite_optional_player(during_turn, change)?;
    if let Some(origin) = origin_condition {
        match origin {
            model::ZoneChangeOriginCondition::MovedFromOrCastFrom {
                zone: _, zone_owner, caster, subject_surface: _,
            } => {
                rewrite_optional_player(zone_owner, change)?;
                rewrite_optional_player(caster, change)?;
            }
        }
    }
    Ok(())
}

fn rewrite_trigger_model_words(original: &model::Trigger, change: TextChange)
    -> Result<model::Trigger, Error>
{
    use model::TriggerKind as K;
    let mut rewritten = original.clone();
    // This is an exhaustive inventory of model fields. Omitted fields below
    // are explicitly bound to `_`: they are identities, references, flags,
    // fixed quantities, or presentation, rather than literal word predicates.
    match &mut rewritten.kind {
        K::StateBased { display: _ } | K::Custom { id: _, label: _ } => {
            return Err(Error::TriggeredAbility);
        }
        K::AnyOf(branches) => {
            *branches = branches.iter().map(|branch| rewrite_trigger_model_words(branch, change))
                .collect::<Result<_, _>>()?;
        }
        K::Either { left, right } => {
            **left = rewrite_trigger_model_words(left, change)?;
            **right = rewrite_trigger_model_words(right, change)?;
        }
        K::ConditionQualified { trigger, condition, surface: _, stun_counter_reminder_surface: _ } => {
            **trigger = rewrite_trigger_model_words(trigger, change)?;
            *condition = rewrite_condition_words(condition, change)?;
        }
        K::ZoneGated { trigger, zones: _ } => {
            **trigger = rewrite_trigger_model_words(trigger, change)?;
        }
        K::ThisAttacksWhileYouControl { filter }
        | K::ThisAttacksPlayerWhoControlsAtLeast { count: _, filter }
        | K::Attacks { filter } | K::AttacksAndIsntBlocked { filter }
        | K::AttacksAndIsntBlockedOneOrMore { filter } | K::AttacksWhileSaddled { filter }
        | K::AttacksOneOrMore { filter } | K::AttacksAlone { filter }
        | K::AttacksYou { filter } | K::AttacksYouOneOrMore { filter }
        | K::AttacksOneOrMoreWithMinTotal { filter, min_total_attackers: _ }
        | K::AttacksOneOrMoreWithExactTotal { filter, total_attackers: _ }
        | K::ThisBlocksObject { filter, min_blocked_objects: _ }
        | K::Blocks { filter } | K::BlocksOneOrMore { filter }
        | K::BecomesBlocked { filter } | K::ThisBecomesBlockedByObject { filter }
        | K::LeavesBattlefield { filter } | K::TurnedFaceUp { filter }
        | K::PermanentBecomesTapped { filter, one_or_more: _ }
        | K::PermanentBecomesUntapped { filter, one_or_more: _ }
        | K::BecomesTargetedObject { filter } | K::BecomesTargetedBySpell { filter }
        | K::BecomesTargetedByStackObject { filter } | K::ThisDealsDamageTo { filter }
        | K::ThisDealsCombatDamageTo { filter } | K::DealsCombatDamage { filter }
        | K::DealsDamage { filter, source_surface: _ }
        | K::PermanentSacrificed { filter } | K::PermanentDestroyed { filter }
        | K::Dies { filter } | K::PutIntoGraveyard { filter }
        | K::CardsLeaveYourGraveyard { filter, one_or_more: _, during_your_turn: _ }
        | K::PermanentTransforms { filter } | K::FinalChapterAbilityResolved { filter }
        | K::NthCounterPutOn { filter, counter_type: _, counter_number: _ }
        | K::PhasingChanged { filter, phased_in: _, one_or_more: _ }
        | K::PermanentMutates { filter } | K::AttacksPlayerAlone { filter }
        | K::BecomesBlockedOneOrMore { filter } => {
            *filter = rewrite_filter_words(filter, change)?;
        }
        K::ThisAttacksWithNOthers {
            count: _, display_subject: _, other_filter, other_surface: _, subject_filter,
        } => {
            rewrite_optional_filter(other_filter, change)?;
            rewrite_optional_filter(subject_filter, change)?;
        }
        K::PlayersAttackedOneOrMore { player_filter } => {
            *player_filter = rewrite_player_filter_words(player_filter, change)?;
        }
        K::PlayerAttacksOneOrMore { attacker, target }
        | K::PlayerAttacksTargetWithOneOrMore { attacker, target } => {
            *attacker = rewrite_player_filter_words(attacker, change)?;
            rewrite_attack_target_words(target, change)?;
        }
        K::AttacksOneOrMoreWithAggregate { filter, metric: _, comparison } => {
            *filter = rewrite_filter_words(filter, change)?;
            *comparison = rewrite_filter_comparison_words(comparison, change)?;
        }
        K::BlocksOrBecomesBlockedByObject { subject, other } => {
            *subject = rewrite_filter_words(subject, change)?;
            *other = rewrite_filter_words(other, change)?;
        }
        K::BlocksObjectWithLesserPower { blocker, blocked }
        | K::BlocksObject { blocker, blocked }
        | K::BecomesBlockedByObjectWithLesserPower { blocked, blocker } => {
            *blocker = rewrite_filter_words(blocker, change)?;
            *blocked = rewrite_filter_words(blocked, change)?;
        }
        K::BecomesTargetedObjectByStackObject { target, source }
        | K::BecomesTargetedByAbilitySource { target, source }
        | K::DealsDamageTo { source, target, source_surface: _ }
        | K::DealsCombatDamageTo { source, target } => {
            *source = rewrite_filter_words(source, change)?;
            *target = rewrite_filter_words(target, change)?;
        }
        K::BecomesTargetedBySourceController { target, controller } => {
            *target = rewrite_filter_words(target, change)?;
            *controller = rewrite_player_filter_words(controller, change)?;
        }
        K::PlayerOrObjectBecomesTargetedBySourceController {
            player, object, controller, source_kind: _, once_per_stack_object: _,
        } => {
            *player = rewrite_player_filter_words(player, change)?;
            *object = rewrite_filter_words(object, change)?;
            *controller = rewrite_player_filter_words(controller, change)?;
        }
        K::ThisDealsDamageToPlayer { player, amount } => {
            *player = rewrite_player_filter_words(player, change)?;
            if let Some(amount) = amount { *amount = rewrite_filter_comparison_words(amount, change)?; }
        }
        K::ThisDealsCombatDamageToPlayer { player, source_surface: _ }
        | K::PlayerGivesGift { player } | K::PlayerSearchesLibrary { player }
        | K::PlayerShufflesLibrary { player, caused_by_effect: _, source_controller_shuffles: _ }
        | K::PlayerRollsResult { player, result: _ } | K::PlayerRollsHighestNaturalResult { player }
        | K::PlayerRollsDie { player, one_or_more: _ } | K::PlayerCoinFlipResult { player, won: _ }
        | K::PlayerLosesLife { player } | K::PlayersLoseLifeOneOrMore { player }
        | K::PlayerLosesGame { player } | K::PlayerDrawsCard { player }
        | K::PlayerDrawsCardExceptFirstInDrawStep { player }
        | K::PlayerDrawsNthCardEachTurn { player, card_number: _ }
        | K::PlayerDrawsNumberedCardsEachTurn { player, card_numbers: _ }
        | K::BeginningOfUpkeep { player } | K::BeginningOfDrawStep { player }
        | K::BeginningOfCombat { player } | K::BeginningOfEndStep { player, surface: _ }
        | K::BeginningOfMainPhase { player, surface: _ }
        | K::BeginningOfPrecombatMainPhase { player }
        | K::BeginningOfPostcombatMainPhase { player, surface: _ }
        | K::KeywordAction { action: _, player }
        | K::KeywordActionDuringYourTurn { action: _, player }
        | K::KeywordActionFromSource { action: _, player }
        | K::WinsClash { player, surface: _ } | K::Expend { amount: _, player }
        | K::PlayerRollsToVisitAttractions { player } | K::PlayerDrawsFirstCardInOwnDrawStep { player }
        | K::RingBearerChosen { player } | K::PlayerRollsNthDie { player, ordinal: _ }
        | K::PlayerPaysLife { player } | K::PlayerBecomesMonarch { player } => {
            *player = rewrite_player_filter_words(player, change)?;
        }
        K::DealsDamageToPlayer { source, player, source_surface: _ }
        | K::DealsCombatDamageToPlayer { source, player, one_or_more: _, each_damaged_player: _, per_source_controller: _ } => {
            *source = rewrite_filter_words(source, change)?;
            *player = rewrite_player_filter_words(player, change)?;
        }
        K::DealsExactDamageToObjectOrPlayer {
            source, object, player, player_first: _, amount: _, source_surface: _,
        } => {
            *source = rewrite_filter_words(source, change)?;
            *object = rewrite_filter_words(object, change)?;
            *player = rewrite_player_filter_words(player, change)?;
        }
        K::DealsNoncombatDamageToPlayer {
            source, player, source_surface: _, damaged_player_one_or_more: _, during_turn,
        } => {
            *source = rewrite_filter_words(source, change)?;
            *player = rewrite_player_filter_words(player, change)?;
            rewrite_optional_player(during_turn, change)?;
        }
        K::PlayerPlaysLand { player, filter } | K::PlayerTapsForMana { player, filter }
        | K::PlayerRevealsCard { player, filter, from_source: _, first_draw_pair: _ }
        | K::PlayerSacrifices { player, filter, one_or_more_surface: _ }
        | K::TokensCreated { player, filter, one_or_more: _ }
        | K::KeywordActionMatchingObject { action: _, player, filter }
        | K::KeywordActionMatchingObjectDuringYourTurn { action: _, player, filter }
        | K::KeywordActionMatchingObjectOneOrMore { action: _, player, filter }
        | K::PlayerTurnsFaceUp { player, filter } => {
            *player = rewrite_player_filter_words(player, change)?;
            *filter = rewrite_filter_words(filter, change)?;
        }
        K::AbilityActivatedQualified {
            activator, filter, non_mana_only: _, loyalty_only: _, activation_cost_has_tap: _,
        } => {
            *activator = rewrite_player_filter_words(activator, change)?;
            *filter = rewrite_filter_words(filter, change)?;
        }
        K::AbilityTriggered { source_filter, .. } => {
            rewrite_optional_filter(source_filter, change)?;
        }
        K::IsDealtDamage {
            target, combat_only: _, noncombat_only: _, excess_only: _, minimum: _, single_source: _,
        } => *target = rewrite_choose_spec_words(target, change)?,
        K::YouGainLifeCausedBy { source } => *source = rewrite_filter_words(source, change)?,
        K::YouGainLifeDuringTurn { during_turn } => {
            *during_turn = rewrite_player_filter_words(during_turn, change)?;
        }
        K::PlayerLosesLifeDuringTurn { player, during_turn }
        | K::PlayerDrawsCardNotDuringTurn { player, during_turn }
        | K::PlayerDrawsCardDuringTurn { player, during_turn } => {
            *player = rewrite_player_filter_words(player, change)?;
            *during_turn = rewrite_player_filter_words(during_turn, change)?;
        }
        K::SpellCountered { filter, controller } => {
            rewrite_optional_filter(filter, change)?;
            *controller = rewrite_player_filter_words(controller, change)?;
        }
        K::PlayerDiscardsCardCausedByController {
            player, filter, controller, effect_like_only: _, one_or_more: _,
        } => {
            *player = rewrite_player_filter_words(player, change)?;
            rewrite_optional_filter(filter, change)?;
            *controller = rewrite_player_filter_words(controller, change)?;
        }
        K::PlayerDiscardsCard { player, filter, one_or_more: _ }
        | K::CardsMilled { player, filter, one_or_more: _, per_player: _ } => {
            *player = rewrite_player_filter_words(player, change)?;
            rewrite_optional_filter(filter, change)?;
        }
        K::DiesCreatureDealtDamageByThisTurn { victim, damager: _ } => {
            *victim = rewrite_filter_words(victim, change)?;
        }
        K::DiesCreatureDealtDamageByFilteredSourceThisTurn { victim, damager_filter } => {
            *victim = rewrite_filter_words(victim, change)?;
            *damager_filter = rewrite_filter_words(damager_filter, change)?;
        }
        K::SpellCastQualified {
            filter, mana_source_filter, caster, timing: _, during_turn,
            min_spells_this_turn: _, exact_spells_this_turn: _, from_not_hand: _,
        } => {
            rewrite_optional_filter(filter, change)?;
            rewrite_optional_filter(mana_source_filter, change)?;
            *caster = rewrite_player_filter_words(caster, change)?;
            rewrite_optional_player(during_turn, change)?;
        }
        K::SpellCast { filter, caster } => {
            rewrite_optional_filter(filter, change)?;
            *caster = rewrite_player_filter_words(caster, change)?;
        }
        K::SpellCastSameNameCardInZone { filter, caster, zone: _, owner } => {
            rewrite_optional_filter(filter, change)?;
            *caster = rewrite_player_filter_words(caster, change)?;
            *owner = rewrite_player_filter_words(owner, change)?;
        }
        K::SpellCopied { filter, copier } => {
            rewrite_optional_filter(filter, change)?;
            *copier = rewrite_player_filter_words(copier, change)?;
        }
        K::EntersBattlefield { filter, cause_filter, count, tapped } => {
            // The current tapped/untapped interpreter drops cause filters and
            // cannot represent grouped tapped entries. Neither is a complete
            // reconstruction, even when this replacement word is absent.
            if tapped.is_some() && (cause_filter.is_some() || *count == model::CountMode::OneOrMore) {
                return Err(Error::TriggeredAbility);
            }
            *filter = rewrite_filter_words(filter, change)?;
            rewrite_cause_words(cause_filter, change)?;
        }
        K::KeywordActionMatchingTaggedObject {
            action: _, player, source_filter, object_tag: _, object_filter, during_your_main_phase: _,
        } => {
            *player = rewrite_player_filter_words(player, change)?;
            *source_filter = rewrite_filter_words(source_filter, change)?;
            *object_filter = rewrite_filter_words(object_filter, change)?;
        }
        K::ZoneChange(zone) => rewrite_zone_change_words(zone, change)?,
        K::PlayerGetsCounters(model::PlayerGetsCountersTrigger { player, counter_type: _, count: _ }) => {
            *player = rewrite_player_filter_words(player, change)?;
        }
        K::CounterPutOn(model::CounterPutOnTrigger {
            filter, counter_type: _, source_controller, count: _, include_players: _, one_or_more_objects: _,
        }) => {
            *filter = rewrite_filter_words(filter, change)?;
            rewrite_optional_player(source_controller, change)?;
        }
        K::CounterRemovedFrom(model::CounterRemovedFromTrigger {
            filter, counter_type: _, last: _, one_or_more: _, caused_by_source: _,
        }) => *filter = rewrite_filter_words(filter, change)?,
        K::PlayerChangesTapState { player, filter, tapped: _, one_or_more: _, during_untap_step } => {
            *player = rewrite_player_filter_words(player, change)?;
            *filter = rewrite_filter_words(filter, change)?;
            rewrite_optional_player(during_untap_step, change)?;
        }
        K::AttachmentChanged { attachment, recipient, attached: _ } => {
            *attachment = rewrite_filter_words(attachment, change)?;
            *recipient = rewrite_filter_words(recipient, change)?;
        }
        K::PlayerAttackDeclaration { attacker, defender, grouping: _ } => {
            *attacker = rewrite_player_filter_words(attacker, change)?;
            *defender = rewrite_player_filter_words(defender, change)?;
        }
        K::PlayerGainsLife { player, during_turn } => {
            *player = rewrite_player_filter_words(player, change)?;
            rewrite_optional_player(during_turn, change)?;
        }
        K::ControlChanged(model::ControlChangeTrigger { filter, change: direction }) => {
            *filter = rewrite_filter_words(filter, change)?;
            match direction {
                model::ControlChangeDirection::Gained { player, from } => {
                    *player = rewrite_player_filter_words(player, change)?;
                    rewrite_optional_player(from, change)?;
                }
                model::ControlChangeDirection::Lost { player } => {
                    *player = rewrite_player_filter_words(player, change)?;
                }
            }
        }
        K::PlayerBecomesTargeted { player, source_controller, source_kind: _ } => {
            *player = rewrite_player_filter_words(player, change)?;
            *source_controller = rewrite_player_filter_words(source_controller, change)?;
        }
        K::PermanentTransformsInto { filter, destination } => {
            *filter = rewrite_filter_words(filter, change)?;
            *destination = rewrite_filter_words(destination, change)?;
        }
        K::PlayerRollsResultMatching { player, result, natural: _ } => {
            *player = rewrite_player_filter_words(player, change)?;
            *result = rewrite_filter_comparison_words(result, change)?;
        }
        K::ThisAttacks | K::ThisAndAnotherAttackDifferentPlayers | K::ThisAttacksPlayerWithMostLife
        | K::ThisAttacksWithGreaterPower | K::ThisAttacksWithExactNOthers { count: _ }
        | K::ThisAttacksAndIsntBlocked | K::ThisAttacksWhileSaddled | K::ThisBlocks
        | K::ThisBecomesBlocked | K::ThisDies | K::ThisDiesOrIsExiled
        | K::ThisDiesOrIsExiledWithSurface { surface: _ } | K::ThisLeavesBattlefield
        | K::ThisPhasesOut | K::ThisMutates | K::ThisBecomesMonstrous
        | K::ClassBecomesLevel { level: _ } | K::BecomesTapped | K::BecomesUntapped
        | K::ThisIsTurnedFaceUp | K::BecomesTargeted | K::ThisDealsDamage
        | K::ThisDealsCombatDamage | K::YouGainLife | K::OpponentsEachLoseExactLife { amount: _ }
        | K::YouDrawCard | K::Miracle | K::NthSpellOfTurnCast { spell_number: _ }
        | K::EndOfCombat | K::DayNightChanged | K::ThisEntersBattlefield
        | K::ThisTransforms { destination_name: _ }
        | K::ThisTransformsWithSurface { surface: _, destination_name: _ }
        | K::YouCastThisSpell | K::SagaChapter { chapters: _ }
        | K::DungeonRoom { room: _, leads_to: _ } => {}
    }
    Ok(rewritten)
}

fn complete_model(trigger: &Trigger) -> Result<model::Trigger, Error> {
    if let Some(model) = trigger.compiled_model() { return Ok(model.clone()); }
    let kind = native_kind(trigger)?;
    Ok(model::Trigger {
        label: String::new(), kind,
        intro_surface: trigger.intro_surface().map(|intro| match intro {
            TriggerIntroSurface::When => model::TriggerIntroSurface::When,
            TriggerIntroSurface::Whenever => model::TriggerIntroSurface::Whenever,
            TriggerIntroSurface::At => model::TriggerIntroSurface::At,
        }),
    })
}

/// Compare concrete native data, never Trigger::PartialEq (which compares
/// display strings). Every admitted type here derives structural PartialEq
/// and contains no opaque closures or nested runtime Trigger values.
fn exact_native_kind<T: crate::triggers::TriggerMatcher + PartialEq + 'static>(
    native: &T, kind: model::TriggerKind,
) -> Result<model::TriggerKind, Error> {
    let candidate = Trigger::from_model(model::Trigger {
        label: String::new(), kind: kind.clone(), intro_surface: None,
    }).map_err(|_| Error::TriggeredAbility)?;
    if candidate.downcast_ref::<T>() == Some(native) { Ok(kind) }
    else { Err(Error::TriggeredAbility) }
}

fn native_count(count: &crate::triggers::CountMode) -> model::CountMode {
    match count {
        crate::triggers::CountMode::Each => model::CountMode::One,
        crate::triggers::CountMode::OneOrMore => model::CountMode::OneOrMore,
    }
}

fn native_kind(trigger: &Trigger) -> Result<model::TriggerKind, Error> {
    use crate::triggers as native;
    use model::TriggerKind as K;
    // These combinators have a complete typed field inventory but their
    // runtime PartialEq would compare nested trigger labels. Recurse through
    // complete models instead. An arbitrary N-way Or is held because its
    // native grouping semantics must not be silently changed to AnyOf.
    if let Some(native::AnyOfTrigger { branches }) = trigger.downcast_ref::<native::AnyOfTrigger>() {
        return Ok(K::AnyOf(branches.iter().map(complete_model).collect::<Result<_, _>>()?));
    }
    if let Some(native::OrTrigger { triggers }) = trigger.downcast_ref::<native::OrTrigger>() {
        let [left, right] = triggers.as_slice() else { return Err(Error::TriggeredAbility); };
        return Ok(K::Either { left: Box::new(complete_model(left)?), right: Box::new(complete_model(right)?) });
    }
    if let Some(native::ZoneGatedTrigger { trigger, zones }) =
        trigger.downcast_ref::<native::ZoneGatedTrigger>()
    {
        return Ok(K::ZoneGated { trigger: Box::new(complete_model(trigger)?), zones: zones.clone() });
    }
    if let Some(native::ConditionQualifiedTrigger {
        trigger, condition, surface, stun_counter_reminder_surface,
    }) = trigger.downcast_ref::<native::ConditionQualifiedTrigger>() {
        return Ok(K::ConditionQualified {
            trigger: Box::new(complete_model(trigger)?), condition: condition.clone(),
            surface: surface.clone(), stun_counter_reminder_surface: *stun_counter_reminder_surface,
        });
    }
    if let Some(native) = trigger.downcast_ref::<native::ZoneChangeTrigger>() {
        let (from, from_zones, from_excluded) = match &native.from {
            native::ZonePattern::Any => (None, None, None),
            native::ZonePattern::Specific(zone) => (Some(*zone), None, None),
            native::ZonePattern::OneOf(zones) => (None, Some(zones.clone()), None),
            native::ZonePattern::AnyExcept(zone) => (None, None, Some(*zone)),
        };
        let (to, to_excluded) = match &native.to {
            native::ZonePattern::Any => (None, None),
            native::ZonePattern::Specific(zone) => (Some(*zone), None),
            native::ZonePattern::AnyExcept(zone) => (None, Some(*zone)),
            native::ZonePattern::OneOf(_) => return Err(Error::TriggeredAbility),
        };
        return exact_native_kind(native, K::ZoneChange(model::ZoneChangeTrigger {
            from, from_zones, from_excluded, to, to_excluded,
            filter: Some(native.object_filter.clone()), this: native.this_object,
            this_surface: native.this_object_surface.clone(), this_subject_number: native.this_object_subject_number,
            count: native_count(&native.count_mode), cause_filter: native.cause_filter.clone(),
            during_own_resolution: native.during_own_resolution, during_turn: native.during_turn.clone(),
            timing: native.timing, origin_condition: native.origin_condition.clone(),
            graveyard_surface: native.graveyard_surface,
        }));
    }
    if let Some(native) = trigger.downcast_ref::<native::SpellCastTrigger>() {
        let kind = if let Some((zone, owner)) = &native.same_name_card_in_zone {
            K::SpellCastSameNameCardInZone {
                filter: native.filter.clone(), caster: native.caster.clone(), zone: *zone, owner: owner.clone(),
            }
        } else {
            K::SpellCastQualified {
                filter: native.filter.clone(), mana_source_filter: native.mana_source_filter.clone(),
                caster: native.caster.clone(), timing: native.timing, during_turn: native.during_turn.clone(),
                min_spells_this_turn: native.min_spells_this_turn,
                exact_spells_this_turn: native.exact_spells_this_turn, from_not_hand: native.from_not_hand,
            }
        };
        // first_spell_of_game, count_all_spells_this_turn, and qualified
        // same-name combinations have no complete model here; equality holds
        // them rather than discarding those native conditions.
        return exact_native_kind(native, kind);
    }
    if let Some(native) = trigger.downcast_ref::<native::AttacksTrigger>() {
        let filter = native.filter.clone();
        let kind = if let Some((metric, comparison)) = &native.aggregate_constraint {
            K::AttacksOneOrMoreWithAggregate { filter, metric: *metric, comparison: comparison.clone() }
        } else if let Some(total_attackers) = native.max_total_attackers {
            K::AttacksOneOrMoreWithExactTotal { filter, total_attackers }
        } else if native.min_total_attackers > 1 {
            K::AttacksOneOrMoreWithMinTotal { filter, min_total_attackers: native.min_total_attackers }
        } else if native.one_or_more {
            K::AttacksOneOrMore { filter }
        } else {
            K::Attacks { filter }
        };
        return exact_native_kind(native, kind);
    }
    if let Some(native) = trigger.downcast_ref::<native::ThisAttacksWithNOthersTrigger>() {
        let kind = if native.exact {
            K::ThisAttacksWithExactNOthers { count: native.other_count }
        } else {
            K::ThisAttacksWithNOthers {
                count: native.other_count, display_subject: native.display_subject.clone(),
                other_filter: native.other_filter.clone(), other_surface: native.other_surface,
                subject_filter: native.subject_filter.clone(),
            }
        };
        return exact_native_kind(native, kind);
    }
    if let Some(native) = trigger.downcast_ref::<native::YouDiscardCardTrigger>() {
        let kind = if let Some(controller) = &native.cause_controller {
            K::PlayerDiscardsCardCausedByController {
                player: native.player.clone(), filter: native.filter.clone(), controller: controller.clone(),
                effect_like_only: native.effect_like_only, one_or_more: native.one_or_more,
            }
        } else {
            K::PlayerDiscardsCard {
                player: native.player.clone(), filter: native.filter.clone(), one_or_more: native.one_or_more,
            }
        };
        return exact_native_kind(native, kind);
    }
    if let Some(native) = trigger.downcast_ref::<native::CounterPutOnTrigger>() {
        let kind = if let Some(counter_number) = native.counter_number {
            K::NthCounterPutOn {
                filter: native.filter.clone(), counter_type: native.counter_type.ok_or(Error::TriggeredAbility)?,
                counter_number,
            }
        } else {
            K::CounterPutOn(model::CounterPutOnTrigger {
                filter: native.filter.clone(), counter_type: native.counter_type,
                source_controller: native.source_controller.clone(), count: native_count(&native.count_mode),
                include_players: native.include_players, one_or_more_objects: native.one_or_more_objects,
            })
        };
        return exact_native_kind(native, kind);
    }
    if let Some(native) = trigger.downcast_ref::<native::PlayerGetsCountersTrigger>() {
        return exact_native_kind(native, K::PlayerGetsCounters(model::PlayerGetsCountersTrigger {
            player: native.player.clone(), counter_type: native.counter_type, count: native_count(&native.count_mode),
        }));
    }
    if let Some(native) = trigger.downcast_ref::<native::CounterRemovedFromTrigger>() {
        return exact_native_kind(native, K::CounterRemovedFrom(model::CounterRemovedFromTrigger {
            filter: native.filter.clone(), counter_type: native.counter_type,
            last: native.last, one_or_more: native.one_or_more, caused_by_source: native.caused_by_source,
        }));
    }
    if let Some(native) = trigger.downcast_ref::<native::ControlChangedTrigger>() {
        return exact_native_kind(native, K::ControlChanged(native.clone()));
    }
    macro_rules! exact {
        ($ty:ident, $native:ident, $kind:expr) => {
            if let Some($native) = trigger.downcast_ref::<native::$ty>() {
                return exact_native_kind($native, $kind);
            }
        };
    }
    exact!(BlocksTrigger, n, if n.one_or_more {
        K::BlocksOneOrMore { filter: n.filter.clone() }
    } else { K::Blocks { filter: n.filter.clone() } });
    exact!(AttacksYouTrigger, n, if n.one_or_more {
        K::AttacksYouOneOrMore { filter: n.filter.clone() }
    } else { K::AttacksYou { filter: n.filter.clone() } });
    exact!(AttacksAloneTrigger, n, if n.per_player {
        K::AttacksPlayerAlone { filter: n.filter.clone() }
    } else { K::AttacksAlone { filter: n.filter.clone() } });
    exact!(AttacksAndIsntBlockedTrigger, n, if n.one_or_more {
        K::AttacksAndIsntBlockedOneOrMore { filter: n.filter.clone() }
    } else { K::AttacksAndIsntBlocked { filter: n.filter.clone() } });
    exact!(BecomesBlockedTrigger, n, if n.one_or_more {
        K::BecomesBlockedOneOrMore { filter: n.filter.clone() }
    } else { K::BecomesBlocked { filter: n.filter.clone() } });
    exact!(AttacksWhileSaddledTrigger, n, K::AttacksWhileSaddled { filter: n.filter.clone() });
    exact!(ThisBlocksObjectTrigger, n, K::ThisBlocksObject {
        filter: n.blocked_filter.clone(), min_blocked_objects: n.min_blocked_objects,
    });
    exact!(ThisBecomesBlockedByObjectTrigger, n, K::ThisBecomesBlockedByObject { filter: n.blocker_filter.clone() });
    exact!(PlayersAttackedTrigger, n, K::PlayersAttackedOneOrMore { player_filter: n.player_filter.clone() });
    exact!(PlayerAttacksOneOrMoreTrigger, n, if n.group_by_target {
        K::PlayerAttacksTargetWithOneOrMore { attacker: n.attacker.clone(), target: n.target.clone() }
    } else { K::PlayerAttacksOneOrMore { attacker: n.attacker.clone(), target: n.target.clone() } });
    exact!(ThisAttacksTrigger, n, K::ThisAttacks);
    exact!(ThisBlocksTrigger, n, K::ThisBlocks);
    exact!(SpellCopiedTrigger, n, K::SpellCopied { filter: n.filter.clone(), copier: n.copier.clone() });
    exact!(SpellCounteredTrigger, n, K::SpellCountered { filter: n.filter.clone(), controller: n.controller.clone() });
    exact!(DealsCombatDamageToPlayerTrigger, n, K::DealsCombatDamageToPlayer {
        source: n.filter.clone(), player: n.player.clone(), one_or_more: n.one_or_more,
        each_damaged_player: n.each_damaged_player,
        per_source_controller: n.per_source_controller,
    });
    exact!(ThisDealsCombatDamageToPlayerTrigger, n, K::ThisDealsCombatDamageToPlayer {
        player: n.player.clone(), source_surface: n.source_surface.clone(),
    });
    exact!(ThisDealsDamageToTrigger, n, if n.combat_only {
        K::ThisDealsCombatDamageTo { filter: n.target_filter.clone() }
    } else { K::ThisDealsDamageTo { filter: n.target_filter.clone() } });
    exact!(DealsDamageToTrigger, n, if n.combat_only {
        K::DealsCombatDamageTo { source: n.source_filter.clone(), target: n.target_filter.clone() }
    } else {
        K::DealsDamageTo {
            source: n.source_filter.clone(), target: n.target_filter.clone(), source_surface: n.source_surface,
        }
    });
    exact!(DealsExactDamageToObjectOrPlayerTrigger, n, K::DealsExactDamageToObjectOrPlayer {
        source: n.source_filter.clone(), object: n.object_filter.clone(), player: n.player_filter.clone(),
        player_first: n.player_first, amount: n.amount, source_surface: n.source_surface,
    });
    exact!(DealsDamageTrigger, n, if let Some(player) = &n.damaged_player {
        if n.noncombat_only {
            K::DealsNoncombatDamageToPlayer {
                source: n.filter.clone(), player: player.clone(), source_surface: n.source_surface,
                damaged_player_one_or_more: n.damaged_player_one_or_more, during_turn: n.during_turn.clone(),
            }
        } else {
            K::DealsDamageToPlayer { source: n.filter.clone(), player: player.clone(), source_surface: n.source_surface }
        }
    } else if n.combat_only {
        K::DealsCombatDamage { filter: n.filter.clone() }
    } else { K::DealsDamage { filter: n.filter.clone(), source_surface: n.source_surface } });
    exact!(BecomesTargetedObjectTrigger, n, K::BecomesTargetedObject { filter: n.filter.clone() });
    exact!(BecomesTargetedByAbilitySourceTrigger, n, K::BecomesTargetedByAbilitySource {
        target: n.target_filter.clone(), source: n.source_filter.clone(),
    });
    exact!(BecomesTargetedBySourceControllerTrigger, n, K::BecomesTargetedBySourceController {
        target: n.target_filter.clone(), controller: n.source_controller.clone(),
    });
    exact!(PlayerOrObjectBecomesTargetedBySourceControllerTrigger, n, K::PlayerOrObjectBecomesTargetedBySourceController {
        player: n.player_filter.clone(), object: n.object_filter.clone(), controller: n.source_controller.clone(),
        source_kind: n.source_kind, once_per_stack_object: n.once_per_stack_object,
    });
    exact!(PlayerBecomesTargetedTrigger, n, K::PlayerBecomesTargeted {
        player: n.player_filter.clone(), source_controller: n.source_controller.clone(), source_kind: n.source_kind,
    });
    exact!(AbilityActivatedTrigger, n, K::AbilityActivatedQualified {
        activator: n.activator.clone(), filter: n.filter.clone(), non_mana_only: n.non_mana_only,
        loyalty_only: n.loyalty_only, activation_cost_has_tap: n.activation_cost_has_tap,
    });
    exact!(AbilityTriggeredTrigger, n, K::AbilityTriggered {
        another: n.another, source_filter: n.source_filter.clone(), caused_by_source_entering: n.caused_by_source_entering,
        caused_by_source_attacking: n.caused_by_source_attacking,
    });
    exact!(IsDealtDamageTrigger, n, K::IsDealtDamage {
        target: n.target.clone(), combat_only: n.combat_only, noncombat_only: n.noncombat_only,
        excess_only: n.excess_only, minimum: n.minimum, single_source: n.single_source,
    });
    exact!(PlayerGainsLifeTrigger, n, K::PlayerGainsLife { player: n.player.clone(), during_turn: n.during_turn.clone() });
    exact!(PlayerChangesTapStateTrigger, n, K::PlayerChangesTapState {
        player: n.player.clone(), filter: n.filter.clone(), tapped: n.tapped,
        one_or_more: n.one_or_more, during_untap_step: n.during_untap_step.clone(),
    });
    exact!(CardsMilledTrigger, n, K::CardsMilled {
        player: n.player.clone(), filter: n.filter.clone(), one_or_more: n.one_or_more, per_player: n.per_player,
    });
    exact!(AttachmentChangedTrigger, n, K::AttachmentChanged {
        attachment: n.attachment.clone(), recipient: n.recipient.clone(), attached: n.attached,
    });
    exact!(PhasingChangedTrigger, n, K::PhasingChanged {
        filter: n.filter.clone(), phased_in: n.phased_in, one_or_more: n.one_or_more,
    });
    exact!(PlayerAttackDeclarationTrigger, n, K::PlayerAttackDeclaration {
        attacker: n.attacker.clone(), defender: n.defender.clone(), grouping: n.grouping,
    });
    exact!(PlayerRevealsCardTrigger, n, K::PlayerRevealsCard {
        player: n.player.clone(), filter: n.filter.clone(), from_source: n.from_source, first_draw_pair: n.first_draw_pair,
    });
    exact!(PermanentBecomesTappedTrigger, n, K::PermanentBecomesTapped { filter: n.filter.clone(), one_or_more: n.one_or_more });
    exact!(PermanentBecomesUntappedTrigger, n, K::PermanentBecomesUntapped { filter: n.filter.clone(), one_or_more: n.one_or_more });
    exact!(PermanentSacrificedTrigger, n, K::PermanentSacrificed { filter: n.filter.clone() });
    exact!(PermanentDestroyedTrigger, n, K::PermanentDestroyed { filter: n.filter.clone() });
    exact!(FinalChapterAbilityResolvedTrigger, n, K::FinalChapterAbilityResolved { filter: n.filter.clone() });
    exact!(PermanentMutatesTrigger, n, K::PermanentMutates { filter: n.filter.clone() });
    exact!(DiesDamagedByFilteredSourceThisTurnTrigger, n, K::DiesCreatureDealtDamageByFilteredSourceThisTurn {
        victim: n.victim.clone(), damager_filter: n.damager_filter.clone(),
    });
    exact!(CardsLeaveYourGraveyardTrigger, n, K::CardsLeaveYourGraveyard {
        filter: n.filter.clone(), one_or_more: n.one_or_more, during_your_turn: n.during_your_turn,
    });
    exact!(EntersBattlefieldTappedTrigger, n, K::EntersBattlefield {
        filter: n.filter.clone(), cause_filter: None, count: model::CountMode::One, tapped: Some(true),
    });
    exact!(EntersBattlefieldUntappedTrigger, n, K::EntersBattlefield {
        filter: n.filter.clone(), cause_filter: None, count: model::CountMode::One, tapped: Some(false),
    });
    exact!(BeginningOfUpkeepTrigger, n, K::BeginningOfUpkeep { player: n.player.clone() });
    Err(Error::TriggeredAbility)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::{RawEvent, ZoneChangeEvent};
    use crate::events::cause::EventCause;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use crate::triggers::{CountMode, TriggerContext, ZoneChangeTrigger};
    use ironsmith_core::{
        CardType, ChoiceAggregateMetric, Color, ColorSet, Condition, FilterComparison,
        LinkedExileDefinition, LinkedExilePair, ManaCost, ManaSymbol, SourceReferenceSurface,
        Subtype, TaggedObjectConstraint, TaggedOpbjectRelation, TurnHistoryCount, Value,
        ValueComparisonOperator, Zone,
    };

    const A: PlayerId = PlayerId::from_index(0);
    fn red_to_blue() -> TextChange { TextChange::color(Color::Red, Color::Blue).unwrap() }
    fn red() -> ObjectFilter {
        ObjectFilter { colors: Some(ColorSet::RED), ..ObjectFilter::creature() }
    }
    fn blue() -> ObjectFilter {
        ObjectFilter { colors: Some(ColorSet::BLUE), ..ObjectFilter::creature() }
    }
    fn retained(kind: model::TriggerKind) -> Trigger {
        Trigger::from_model(model::Trigger {
            label: "Red Human is an immutable presentation label".into(), kind,
            intro_surface: Some(model::TriggerIntroSurface::When),
        }).unwrap()
    }
    fn creature(game: &mut GameState, colors: ColorSet, subtype: Subtype) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), "Red Human")
            .card_types(vec![CardType::Creature]).subtypes(vec![subtype])
            .color_indicator(colors).power_toughness(PowerToughness::fixed(2, 3)).build();
        game.create_object_from_card(&card, A, Zone::Battlefield)
    }
    fn death_batch(game: &GameState, objects: &[ObjectId]) -> RawEvent {
        RawEvent::new(ZoneChangeEvent::batch_with_snapshots(
            objects.to_vec(), Zone::Battlefield, Zone::Graveyard, EventCause::effect(),
            objects.iter().map(|id| ObjectSnapshot::from_object(game.object(*id).unwrap(), game)).collect(),
        ), Default::default())
    }

    #[test]
    fn native_grouped_deaths_rewrite_predicates_and_counts_without_editing_captured_events() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = creature(&mut game, ColorSet::COLORLESS, Subtype::Human);
        let first_red = creature(&mut game, ColorSet::RED, Subtype::Human);
        let second_red = creature(&mut game, ColorSet::RED, Subtype::Human);
        let blue_creature = creature(&mut game, ColorSet::BLUE, Subtype::Human);
        let event = death_batch(&game, &[first_red, second_red, blue_creature]);
        let ctx = TriggerContext::for_source(source, A, &game);
        for count in [CountMode::Each, CountMode::OneOrMore] {
            let original = Trigger::new(ZoneChangeTrigger::new()
                .from(Zone::Battlefield).to(Zone::Graveyard).filter(red()).count(count.clone()))
                .with_intro_surface(TriggerIntroSurface::Whenever);
            let captured = original.clone();
            let changed = rewrite_trigger_words(&original, red_to_blue()).unwrap();
            assert!(original.matches(&event, &ctx));
            assert!(changed.matches(&event, &ctx));
            assert_eq!(original.event_value_amount(&event, &ctx), Some(2));
            assert_eq!(changed.event_value_amount(&event, &ctx), Some(1));
            assert_eq!(captured.event_value_amount(&event, &ctx), Some(2));
            assert_eq!(original.trigger_count_with_context(&event, &ctx),
                if count == CountMode::Each { 2 } else { 1 });
            assert_eq!(changed.trigger_count_with_context(&event, &ctx), 1);
            assert_eq!(changed.simultaneous_trigger_key(&event), original.simultaneous_trigger_key(&event));
            assert_eq!(changed.intro_surface(), Some(TriggerIntroSurface::Whenever));
            assert_eq!(original.runtime_matcher_identity(), captured.runtime_matcher_identity());
            assert_ne!(original.runtime_matcher_identity(), changed.runtime_matcher_identity());
        }
        let snapshots = &event.downcast::<ZoneChangeEvent>().unwrap().snapshots;
        assert_eq!(snapshots[0].object_id, first_red);
        assert_eq!(snapshots[0].colors, ColorSet::RED);
        assert_eq!(snapshots[2].colors, ColorSet::BLUE);
    }

    #[test]
    fn native_source_reference_remains_this_exact_source_after_a_word_rewrite() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = creature(&mut game, ColorSet::BLUE, Subtype::Human);
        let other = creature(&mut game, ColorSet::BLUE, Subtype::Human);
        let original = Trigger::new(ZoneChangeTrigger::new()
            .from(Zone::Battlefield).to(Zone::Graveyard).this()
            .this_surface(SourceReferenceSurface::FullName("Red Human".into())).filter(red()));
        let changed = rewrite_trigger_words(&original, red_to_blue()).unwrap();
        let ctx = TriggerContext::for_source(source, A, &game);
        assert!(changed.matches(&death_batch(&game, &[source]), &ctx));
        assert!(!changed.matches(&death_batch(&game, &[other]), &ctx));
        let matcher = changed.downcast_ref::<ZoneChangeTrigger>().unwrap();
        assert!(matcher.this_object);
        assert_eq!(matcher.this_object_surface,
            Some(SourceReferenceSurface::FullName("Red Human".into())));
        assert_eq!(original.downcast_ref::<ZoneChangeTrigger>().unwrap().object_filter, red());
    }

    #[test]
    fn negated_type_predicate_changes_and_captured_subtype_does_not() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = creature(&mut game, ColorSet::COLORLESS, Subtype::Human);
        let human = creature(&mut game, ColorSet::RED, Subtype::Human);
        let vampire = creature(&mut game, ColorSet::BLUE, Subtype::Vampire);
        let filter = ObjectFilter { excluded_subtypes: vec![Subtype::Human], ..ObjectFilter::creature() };
        let original = Trigger::new(ZoneChangeTrigger::new()
            .from(Zone::Battlefield).to(Zone::Graveyard).filter(filter));
        let changed = rewrite_trigger_words(&original,
            TextChange::creature_type(Subtype::Human, Subtype::Vampire).unwrap()).unwrap();
        let human_death = death_batch(&game, &[human]);
        let vampire_death = death_batch(&game, &[vampire]);
        let ctx = TriggerContext::for_source(source, A, &game);
        assert!(!original.matches(&human_death, &ctx));
        assert!(changed.matches(&human_death, &ctx));
        assert!(original.matches(&vampire_death, &ctx));
        assert!(!changed.matches(&vampire_death, &ctx));
        assert_eq!(human_death.downcast::<ZoneChangeEvent>().unwrap().snapshots[0].subtypes,
            vec![Subtype::Human]);
    }

    #[test]
    fn movement_history_under_negation_reads_new_words_and_keeps_history_receipts() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = creature(&mut game, ColorSet::COLORLESS, Subtype::Human);
        let red_creature = creature(&mut game, ColorSet::RED, Subtype::Human);
        let movement = death_batch(&game, &[red_creature]);
        game.turn_store.turn_history.record_event(&movement, None, None);
        let history = |filter| Condition::Not(Box::new(Condition::ValueComparison {
            left: Value::TurnHistoryCount(TurnHistoryCount::MovedZones {
                filter, from: Some(Zone::Battlefield), to: Some(Zone::Graveyard),
            }),
            operator: ValueComparisonOperator::GreaterThan, right: Value::Fixed(0),
        }));
        let original = Trigger::condition_qualified(
            Trigger::new(ZoneChangeTrigger::new().from(Zone::Battlefield).to(Zone::Graveyard)),
            history(red()), "while no red creature has died".into(), false,
        );
        let changed = rewrite_trigger_words(&original, red_to_blue()).unwrap();
        let ctx = TriggerContext::for_source(source, A, &game);
        assert!(!original.matches(&movement, &ctx));
        assert!(changed.matches(&movement, &ctx));
        let rewritten = changed.downcast_ref::<crate::triggers::ConditionQualifiedTrigger>().unwrap();
        assert_eq!(rewritten.condition, history(blue()));
        assert_eq!(rewritten.surface, "while no red creature has died");
        assert_eq!(movement.downcast::<ZoneChangeEvent>().unwrap().snapshots[0].colors, ColorSet::RED);
    }

    #[test]
    fn retained_union_visits_cause_origin_and_player_predicates_without_changing_names_or_choices() {
        let chosen = ObjectFilter {
            colors: Some(ColorSet::RED), chosen_color: true, chosen_creature_type: true,
            name: Some("Red Human".into()),
            exact_mana_cost: Some(ManaCost::from_pips(vec![vec![ManaSymbol::Red]])),
            tagged_constraints: vec![TaggedObjectConstraint {
                tag: "red-captured".into(), relation: TaggedOpbjectRelation::SameObjectId,
            }],
            ..ObjectFilter::default()
        };
        let mut zone = model::ZoneChangeTrigger::new().from(Zone::Hand).to(Zone::Battlefield)
            .filter(chosen.clone()).this_surface(SourceReferenceSurface::FullName("Red Human".into()))
            .cause_filter(Some(ironsmith_core::CauseFilter::from_source(red())));
        zone.origin_condition = Some(model::ZoneChangeOriginCondition::MovedFromOrCastFrom {
            zone: Zone::Hand,
            zone_owner: Some(PlayerFilter::ControlsMost { filter: Box::new(red()) }),
            caster: Some(PlayerFilter::TaggedPlayer("red-player".into())),
            subject_surface: model::OriginConditionSubjectSurface::That("that red creature".into()),
        });
        let original = retained(model::TriggerKind::AnyOf(vec![
            model::Trigger::new(zone),
            model::Trigger::player_becomes_monarch(PlayerFilter::ControlsFewestTied { filter: Box::new(red()) }),
        ]));
        let changed = rewrite_trigger_words(&original, red_to_blue()).unwrap();
        let model::TriggerKind::AnyOf(branches) = &changed.compiled_model().unwrap().kind else { panic!("union"); };
        let model::TriggerKind::ZoneChange(zone) = &branches[0].kind else { panic!("zone change"); };
        let mut expected = chosen.clone();
        expected.colors = Some(ColorSet::BLUE);
        assert_eq!(zone.filter, Some(expected));
        assert_eq!(zone.this_surface, Some(SourceReferenceSurface::FullName("Red Human".into())));
        assert_eq!(zone.cause_filter.as_ref().unwrap().source_filter, Some(blue()));
        assert_eq!(zone.origin_condition, Some(model::ZoneChangeOriginCondition::MovedFromOrCastFrom {
            zone: Zone::Hand,
            zone_owner: Some(PlayerFilter::ControlsMost { filter: Box::new(blue()) }),
            caster: Some(PlayerFilter::TaggedPlayer("red-player".into())),
            subject_surface: model::OriginConditionSubjectSurface::That("that red creature".into()),
        }));
        assert_eq!(branches[1].kind, model::TriggerKind::PlayerBecomesMonarch {
            player: PlayerFilter::ControlsFewestTied { filter: Box::new(blue()) },
        });
        assert_eq!(changed.compiled_model().unwrap().label, original.compiled_model().unwrap().label);
        assert_eq!(changed.intro_surface(), original.intro_surface());
        assert_eq!(chosen.colors, Some(ColorSet::RED));
    }

    #[test]
    fn dynamic_comparison_and_attack_grouping_are_both_retained() {
        let original = Trigger::new(crate::triggers::AttacksTrigger::one_or_more_with_aggregate(
            red(), ChoiceAggregateMetric::Power,
            FilterComparison::GreaterThanOrEqualExpr(Box::new(Value::Count(red()))),
        ));
        let changed = rewrite_trigger_words(&original, red_to_blue()).unwrap();
        let matcher = changed.downcast_ref::<crate::triggers::AttacksTrigger>().unwrap();
        assert!(matcher.one_or_more);
        assert_eq!(matcher.filter, blue());
        assert_eq!(matcher.aggregate_constraint, Some((ChoiceAggregateMetric::Power,
            FilterComparison::GreaterThanOrEqualExpr(Box::new(Value::Count(blue()))))));
        assert_eq!(matcher.min_total_attackers, 1);
        assert_eq!(matcher.max_total_attackers, None);
    }

    #[test]
    fn reveal_link_definition_and_previously_captured_trigger_stay_immutable() {
        let pair = LinkedExilePair { definition: LinkedExileDefinition([7; 32]), pair: 9 };
        let original = Trigger::new(crate::triggers::PlayerRevealsCardTrigger {
            player: PlayerFilter::ChosenPlayer, filter: red(), from_source: true, first_draw_pair: Some(pair),
        }).with_intro_surface(TriggerIntroSurface::When);
        let captured = original.clone();
        let changed = rewrite_trigger_words(&original, red_to_blue()).unwrap();
        let matcher = changed.downcast_ref::<crate::triggers::PlayerRevealsCardTrigger>().unwrap();
        assert_eq!(matcher.first_draw_pair, Some(pair));
        assert_eq!(matcher.player, PlayerFilter::ChosenPlayer);
        assert_eq!(matcher.filter, blue());
        assert!(matcher.from_source);
        assert_eq!(captured.downcast_ref::<crate::triggers::PlayerRevealsCardTrigger>().unwrap().filter, red());
        assert_eq!(captured.runtime_matcher_identity(), original.runtime_matcher_identity());
        assert_eq!(changed.intro_surface(), Some(TriggerIntroSurface::When));
    }

    #[test]
    fn native_damage_rewrites_authored_source_quality_and_preserves_damage_receipts() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = creature(&mut game, ColorSet::COLORLESS, Subtype::Human);
        let red_source = creature(&mut game, ColorSet::RED, Subtype::Human);
        let blue_source = creature(&mut game, ColorSet::BLUE, Subtype::Human);
        let event = |damager| RawEvent::new(crate::events::DamageEvent::with_cause(
            damager, crate::events::DamageTarget::Player(A), 3, false, EventCause::effect(),
        ), Default::default()).with_source_snapshot(ObjectSnapshot::from_object(game.object(damager).unwrap(), &game));
        let red_damage = event(red_source);
        let blue_damage = event(blue_source);
        let original = Trigger::new(crate::triggers::DealsDamageTrigger::noncombat_to_player(
            red(), PlayerFilter::You, model::DamageSourceSurface::PassiveBy,
        ).damaged_player_one_or_more());
        let changed = rewrite_trigger_words(&original, red_to_blue()).unwrap();
        let ctx = TriggerContext::for_source(source, A, &game);
        assert!(original.matches(&red_damage, &ctx));
        assert!(!changed.matches(&red_damage, &ctx));
        assert!(!original.matches(&blue_damage, &ctx));
        assert!(changed.matches(&blue_damage, &ctx));
        let matcher = changed.downcast_ref::<crate::triggers::DealsDamageTrigger>().unwrap();
        assert!(matcher.noncombat_only);
        assert!(matcher.damaged_player_one_or_more);
        assert_eq!(matcher.source_surface, model::DamageSourceSurface::PassiveBy);
        assert_eq!(red_damage.source_snapshot().unwrap().colors, ColorSet::RED);
        assert_eq!(red_damage.downcast::<crate::events::DamageEvent>().unwrap().amount, 3);
    }

    #[test]
    fn absent_words_preserve_matcher_identity_but_do_not_admit_opaque_domains() {
        #[derive(Debug, Clone)]
        struct OpaqueMatcher;
        impl crate::triggers::TriggerMatcher for OpaqueMatcher {
            fn matches(&self, _: &RawEvent, _: &TriggerContext) -> bool { true }
            fn display(&self) -> String { "Whenever a red creature attacks".into() }
        }
        let original = Trigger::this_attacks();
        let changed = rewrite_trigger_words(&original, red_to_blue()).unwrap();
        assert_eq!(changed.runtime_matcher_identity(), original.runtime_matcher_identity());
        for held in [
            Trigger::new(OpaqueMatcher),
            Trigger::custom("red-callback", "Whenever a red creature attacks".into()),
            Trigger::state_based("red creature condition"),
            retained(model::TriggerKind::Custom { id: "green-callback".into(), label: "green".into() }),
        ] {
            assert!(matches!(rewrite_trigger_words(&held, red_to_blue()), Err(Error::TriggeredAbility)));
        }
    }

    #[test]
    fn native_fields_without_complete_models_fail_closed_without_mutation() {
        let native = crate::triggers::SpellCastTrigger::new(Some(red()), PlayerFilter::You)
            .with_first_spell_of_game(true);
        let original = Trigger::new(native.clone());
        assert!(matches!(rewrite_trigger_words(&original, red_to_blue()), Err(Error::TriggeredAbility)));
        assert_eq!(original.downcast_ref::<crate::triggers::SpellCastTrigger>(), Some(&native));
        let native = ZoneChangeTrigger::new().from(Zone::Battlefield).to(Zone::Graveyard)
            .filter(red()).player(crate::triggers::PlayerRelation::Opponent);
        let original = Trigger::new(native.clone());
        assert!(matches!(rewrite_trigger_words(&original, red_to_blue()), Err(Error::TriggeredAbility)));
        assert_eq!(original.downcast_ref::<ZoneChangeTrigger>(), Some(&native));
        let incompatible = retained(model::TriggerKind::EntersBattlefield {
            filter: red(), cause_filter: Some(ironsmith_core::CauseFilter::from_cost()),
            count: model::CountMode::One, tapped: Some(true),
        });
        assert!(matches!(rewrite_trigger_words(&incompatible, red_to_blue()), Err(Error::TriggeredAbility)));
    }

    #[test]
    fn native_spell_predicate_survives_model_round_trip_and_second_replacement() {
        let filter = ObjectFilter { colors: Some(ColorSet::RED), ..ObjectFilter::default() };
        let original = Trigger::spell_cast(Some(filter.clone()), PlayerFilter::You)
            .with_intro_surface(TriggerIntroSurface::Whenever);
        let changed = rewrite_trigger_words(&original, red_to_blue()).unwrap();
        let encoded = serde_json::to_vec(changed.compiled_model().unwrap()).unwrap();
        let restored = Trigger::from_model(serde_json::from_slice(&encoded).unwrap()).unwrap();
        let green = rewrite_trigger_words(&restored, TextChange::color(Color::Blue, Color::Green).unwrap()).unwrap();
        let matcher = green.downcast_ref::<crate::triggers::SpellCastTrigger>().unwrap();
        assert_eq!(matcher.filter.as_ref().unwrap().colors, Some(ColorSet::GREEN));
        assert_eq!(matcher.caster, PlayerFilter::You);
        assert_eq!(green.intro_surface(), Some(TriggerIntroSurface::Whenever));
        assert_eq!(original.downcast_ref::<crate::triggers::SpellCastTrigger>().unwrap().filter, Some(filter));
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = creature(&mut game, ColorSet::COLORLESS, Subtype::Human);
        let mut events = Vec::new();
        for colors in [ColorSet::RED, ColorSet::BLUE, ColorSet::GREEN] {
            let card = CardBuilder::new(CardId::new(), "Red spell name")
                .card_types(vec![CardType::Sorcery]).color_indicator(colors)
                .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]])).build();
            let spell = game.create_object_from_card(&card, A, Zone::Stack);
            events.push(RawEvent::new(crate::events::SpellCastEvent::from_completed_cast(
                spell, A, Zone::Hand, &game,
            ), Default::default()));
        }
        let ctx = TriggerContext::for_source(source, A, &game);
        assert_eq!(events.iter().map(|event| original.matches(event, &ctx)).collect::<Vec<_>>(),
            vec![true, false, false]);
        assert_eq!(events.iter().map(|event| restored.matches(event, &ctx)).collect::<Vec<_>>(),
            vec![false, true, false]);
        assert_eq!(events.iter().map(|event| green.matches(event, &ctx)).collect::<Vec<_>>(),
            vec![false, false, true]);
    }

}
