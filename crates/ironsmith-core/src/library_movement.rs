//! Structural library movement for activated mana abilities (CR 605.1a,
//! Comprehensive Rules effective 2026-09-25). This inspects authored actions,
//! including optional branches, not the current library or replacement state.
use crate::effect::*;
use crate::{ChooseSpec, Cost, TotalCost, TotalCostKind, Zone};
use std::any::Any;

pub trait LibraryMovement {
    fn inspect_library_movement(&self, context: &mut LibraryMovementContext) -> bool;

    fn moves_library_cards(&self) -> bool {
        self.inspect_library_movement(&mut LibraryMovementContext::default())
    }
}

impl<E: LibraryMovement> LibraryMovement for Cost<E> {
    fn inspect_library_movement(&self, context: &mut LibraryMovementContext) -> bool {
        match self {
            Cost::Mill(_) => true,
            Cost::Effect(effect) => effect.inspect_library_movement(context),
            _ => false,
        }
    }
}

impl<C: LibraryMovement> LibraryMovement for TotalCost<C> {
    fn inspect_library_movement(&self, context: &mut LibraryMovementContext) -> bool {
        match self.kind() {
            TotalCostKind::All(costs) => costs
                .iter()
                .any(|item| item.inspect_library_movement(context)),
            TotalCostKind::OneOf(branches) => branches
                .iter()
                .any(|item| item.inspect_library_movement(context)),
        }
    }
}

/// Tags preserve the zone of cards selected/revealed earlier in the same
/// program. Looking alone isn't movement; subsequently moving that selection is.
#[derive(Default)]
pub struct LibraryMovementContext {
    pub library_tags: std::collections::HashSet<crate::tag::TagKey>,
    pub iterated_library: bool,
}

impl LibraryMovementContext {
    fn filter_selects_library(&self, filter: &crate::ObjectFilter) -> bool {
        filter.zone == Some(Zone::Library)
            || filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::TaggedOpbjectRelation::IsTaggedObject
                    && self.library_tags.contains(&constraint.tag)
            })
    }

    fn selects_library(&self, spec: &ChooseSpec) -> bool {
        match spec.base() {
            ChooseSpec::Object(filter)
            | ChooseSpec::All(filter)
            | ChooseSpec::ObjectOrPlayer(filter, _) => self.filter_selects_library(filter),
            ChooseSpec::Tagged(tag) => self.library_tags.contains(tag),
            ChooseSpec::Iterated => self.iterated_library,
            _ => false,
        }
    }

    pub fn inspect_iteration<E: LibraryMovement>(&mut self, library: bool, effects: &[E]) -> bool {
        let previous = self.iterated_library;
        self.iterated_library = library;
        let moves = effects
            .iter()
            .any(|effect| effect.inspect_library_movement(self));
        self.iterated_library = previous;
        moves
    }
}

/// The leaf semantics are shared by compiler and runtime payloads. `None`
/// delegates to the phase's child visitor; `Some(false)` deliberately stops at
/// a deferred effect, whose later resolution isn't part of this activation.
pub fn payload_library_movement<E: LibraryMovement + 'static, C: LibraryMovement + 'static>(
    payload: &dyn Any,
    context: &mut LibraryMovementContext,
) -> Option<bool> {
    macro_rules! is_any {
        ($($ty:ty),+ $(,)?) => { false $(|| payload.is::<$ty>())+ };
    }
    if is_any!(
        ScheduleDelayedTriggerEffect<E>,
        ScheduleEffectsWhenTaggedLeavesEffect<E>,
        ReflexiveTriggerEffect<E>,
        HauntExileEffect<E>,
        RegisterDrawReplacementEffect<E>,
        PreventDamageEffect<E>,
        PreventAllDamageEffect<E>,
        PreventAllDamageToTargetEffect<E>,
        GrantRepeatableManaPaymentActionUntilEndOfTurnEffect<E>,
        GrantPlayTaggedEffect<C>
    ) {
        return Some(false);
    }
    if is_any!(
        DrawCardsEffect,
        DrawForEachTaggedMatchingEffect,
        MillEffect,
        SurveilEffect,
        ShuffleGraveyardIntoLibraryEffect,
        ShuffleHandAndGraveyardIntoLibraryEffect,
        ExileTopOfLibraryEffect,
        ExploreEffect,
        ManifestDreadEffect,
        ManifestTopCardOfLibraryEffect,
        DiscoverEffect,
        RestartGameEffect,
        RippleEffect
    ) {
        return Some(true);
    }
    if let Some(e) = payload.downcast_ref::<LocalRewriteEffect<E>>() {
        // These are explicitly scoped self-replacements, not replacement
        // effects registered by a different ability or game object.
        return Some(
            e.zone_replacements.iter().any(|replacement| {
                replacement.replacement_zone == Zone::Library
                    && replacement.from_zone != Some(Zone::Library)
            }) || e.effect.inspect_library_movement(context),
        );
    }
    if let Some(e) = payload.downcast_ref::<UnlessPaysEffect<E>>() {
        return Some(
            e.cost.inspect_library_movement(context)
                || e.effects
                    .iter()
                    .any(|effect| effect.inspect_library_movement(context)),
        );
    }
    if let Some(e) = payload.downcast_ref::<LookAtTopCardsEffect>() {
        context.library_tags.insert(e.tag.clone());
        return Some(false);
    }
    if let Some(e) = payload.downcast_ref::<RevealTopEffect>() {
        context.library_tags.extend(e.tag.iter().cloned());
        return Some(false);
    }
    if let Some(e) = payload.downcast_ref::<ChooseObjectsEffect>() {
        if e.zone == Some(Zone::Library)
            || e.additional_zones.contains(&Zone::Library)
            || context.filter_selects_library(&e.filter)
        {
            context.library_tags.insert(e.tag.clone());
        }
        return Some(false);
    }
    if let Some(e) = payload.downcast_ref::<TagMatchingObjectsEffect>() {
        if e.zone == Some(Zone::Library)
            || e.additional_zones.contains(&Zone::Library)
            || context.filter_selects_library(&e.filter)
            || e.source_tags
                .iter()
                .any(|tag| context.library_tags.contains(tag))
        {
            context.library_tags.insert(e.tag.clone());
        }
        return Some(false);
    }
    if let Some(e) = payload.downcast_ref::<ForEachObject<E>>() {
        return Some(
            context.inspect_iteration(context.filter_selects_library(&e.filter), &e.effects),
        );
    }
    if let Some(e) = payload.downcast_ref::<ForEachTaggedEffect<E>>() {
        return Some(context.inspect_iteration(context.library_tags.contains(&e.tag), &e.effects));
    }
    if let Some(e) = payload.downcast_ref::<ConsultTopOfLibraryEffect>() {
        context.library_tags.insert(e.all_tag.clone());
        context.library_tags.insert(e.match_tag.clone());
        return Some(e.mode == LibraryConsultMode::Exile);
    }
    if let Some(e) = payload.downcast_ref::<ReturnAllToBattlefieldEffect>() {
        return Some(context.filter_selects_library(&e.filter));
    }
    if let Some(e) = payload.downcast_ref::<CastTaggedEffect<C>>() {
        return Some(
            (!e.as_copy && context.library_tags.contains(&e.tag))
                || e.alternative_cost
                    .as_ref()
                    .is_some_and(|cost| cost.inspect_library_movement(context)),
        );
    }
    if let Some(e) = payload.downcast_ref::<ManifestObjectsEffect>() {
        return Some(context.selects_library(&e.target));
    }
    if let Some(e) = payload.downcast_ref::<SearchLibraryEffect>() {
        return Some(e.destination != Zone::Library);
    }
    if let Some(e) = payload.downcast_ref::<SearchLibrarySlotsEffect>() {
        return Some(e.destination != Zone::Library);
    }
    if let Some(e) = payload.downcast_ref::<MoveToLibraryNthFromTopEffect>() {
        return Some(!context.selects_library(&e.target));
    }
    if let Some(e) = payload.downcast_ref::<MoveToLibraryTopOrBottomChoiceEffect>() {
        return Some(!context.selects_library(&e.target));
    }
    if let Some(e) = payload.downcast_ref::<ShuffleObjectsIntoLibraryEffect>() {
        return Some(!context.selects_library(&e.target));
    }
    if let Some(e) = payload.downcast_ref::<PutTaggedRemainderOnLibraryBottomEffect>() {
        return Some(!context.library_tags.contains(&e.tag));
    }
    if let Some(e) = payload.downcast_ref::<MoveToZoneEffect>() {
        let from_library = context.selects_library(&e.target);
        return Some(
            (e.zone == Zone::Library) != from_library
                || e.tagged_destinations
                    .iter()
                    .any(|(_, zone)| (*zone == Zone::Library) != from_library),
        );
    }
    if let Some(e) = payload.downcast_ref::<MayMoveToZoneEffect>() {
        return Some((e.zone == Zone::Library) != context.selects_library(&e.target));
    }
    if let Some(e) = payload.downcast_ref::<ExileEffect>() {
        return Some(context.selects_library(&e.spec));
    }
    if let Some(e) = payload.downcast_ref::<ExileUntilEffect>() {
        return Some(context.selects_library(&e.spec));
    }
    if let Some(e) = payload.downcast_ref::<ReturnToHandEffect>() {
        return Some(context.selects_library(&e.spec));
    }
    if let Some(e) = payload.downcast_ref::<PutOntoBattlefieldEffect>() {
        return Some(context.selects_library(&e.target));
    }
    if let Some(e) = payload.downcast_ref::<ExchangeZonesEffect>() {
        return Some(e.zone1 != e.zone2 && (e.zone1 == Zone::Library || e.zone2 == Zone::Library));
    }
    None
}
