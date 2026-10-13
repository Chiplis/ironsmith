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
            TotalCostKind::OneOf(branches) => {
                let mut outputs = Vec::new();
                for branch in branches {
                    let mut output = context.clone();
                    if branch.inspect_library_movement(&mut output) {
                        return true;
                    }
                    outputs.push(output);
                }
                context.merge_alternatives(&outputs);
                false
            }
        }
    }
}

/// Tags preserve the zone of cards selected/revealed earlier in the same
/// program. Looking alone isn't movement; subsequently moving that selection is.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct LibraryMovementContext {
    pub library_tags: std::collections::HashSet<crate::tag::TagKey>,
    pub iterated_library: bool,
    /// A tag present in only some alternatives, or selecting multiple zones,
    /// cannot prove that a later move into a library stays within that zone.
    mixed_library_tags: std::collections::HashSet<crate::tag::TagKey>,
    iterated_mixed: bool,
}

impl LibraryMovementContext {
    pub fn filter_selects_library(&self, filter: &crate::ObjectFilter) -> bool {
        self.filter_library_origin(filter).0
    }

    fn filter_library_origin(&self, filter: &crate::ObjectFilter) -> (bool, bool) {
        if let Some(zone) = filter.zone {
            return (zone == Zone::Library, zone == Zone::Library);
        }
        let tagged = filter
            .tagged_constraints
            .iter()
            .filter(|c| c.relation == crate::TaggedOpbjectRelation::IsTaggedObject)
            .map(|c| self.tag_library_origin(&c.tag))
            .collect::<Vec<_>>();
        if !tagged.is_empty() {
            return (tagged.iter().any(|p| p.0), tagged.iter().any(|p| p.1));
        }
        let alternatives = filter
            .any_of
            .iter()
            .map(|f| self.filter_library_origin(f))
            .collect::<Vec<_>>();
        (
            alternatives.iter().any(|p| p.0),
            !alternatives.is_empty() && alternatives.iter().all(|p| p.1),
        )
    }

    pub fn tag_library_origin(&self, tag: &crate::tag::TagKey) -> (bool, bool) {
        (
            self.library_tags.contains(tag),
            self.library_tags.contains(tag) && !self.mixed_library_tags.contains(tag),
        )
    }

    pub fn bind_tag(&mut self, tag: crate::tag::TagKey, origin: (bool, bool)) {
        self.library_tags.remove(&tag);
        self.mixed_library_tags.remove(&tag);
        if origin.0 {
            self.library_tags.insert(tag.clone());
            if !origin.1 {
                self.mixed_library_tags.insert(tag);
            }
        }
    }

    pub fn selects_library(&self, spec: &ChooseSpec) -> bool {
        self.selection_library_origin(spec).0
    }

    pub fn selection_library_origin(&self, spec: &ChooseSpec) -> (bool, bool) {
        match spec.base() {
            ChooseSpec::Object(filter)
            | ChooseSpec::All(filter)
            | ChooseSpec::ObjectOrPlayer(filter, _) => self.filter_library_origin(filter),
            ChooseSpec::Tagged(tag) => self.tag_library_origin(tag),
            ChooseSpec::Iterated => (
                self.iterated_library,
                self.iterated_library && !self.iterated_mixed,
            ),
            _ => (false, false),
        }
    }

    pub fn moves_selection_to(&self, spec: &ChooseSpec, zone: Zone) -> bool {
        let (may, only) = self.selection_library_origin(spec);
        if zone == Zone::Library { !only } else { may }
    }

    pub fn inspect_sequence<E: LibraryMovement>(&mut self, effects: &[E]) -> bool {
        effects.iter().any(|e| e.inspect_library_movement(self))
    }

    /// Alternatives start from the same input. Merge possible origins only
    /// after inspecting each arm, preserving uncertainty at a later join.
    pub fn inspect_branches<'a, E: LibraryMovement + 'a>(
        &mut self,
        branches: impl IntoIterator<Item = &'a [E]>,
    ) -> bool {
        let mut outputs = Vec::new();
        for branch in branches {
            let mut output = self.clone();
            if output.inspect_sequence(branch) {
                return true;
            }
            outputs.push(output);
        }
        self.merge_alternatives(&outputs);
        false
    }

    pub fn merge_alternatives(&mut self, outputs: &[Self]) {
        if outputs.is_empty() {
            return;
        }
        self.library_tags = outputs
            .iter()
            .flat_map(|c| c.library_tags.iter().cloned())
            .collect();
        self.mixed_library_tags = self
            .library_tags
            .iter()
            .filter(|tag| !outputs.iter().all(|c| c.tag_library_origin(tag).1))
            .cloned()
            .collect();
    }

    fn inspect_exact_repetition<E: LibraryMovement>(
        &mut self,
        effects: &[E],
        mut remaining: u64,
    ) -> bool {
        let mut seen = Vec::new();
        while remaining > 0 {
            if let Some(start) = seen.iter().position(|state| state == self) {
                let cycle = (seen.len() - start) as u64;
                remaining %= cycle;
                seen.clear();
                if remaining == 0 {
                    break;
                }
            }
            seen.push(self.clone());
            if self.inspect_sequence(effects) {
                return true;
            }
            remaining -= 1;
        }
        false
    }

    /// Join zero or more iterations until no additional tag origins are
    /// reachable. A later iteration may consume a tag produced by the first.
    pub fn inspect_repetition<E: LibraryMovement>(&mut self, effects: &[E]) -> bool {
        loop {
            let previous = self.clone();
            if self.inspect_branches([effects, &[]]) {
                return true;
            }
            if *self == previous {
                return false;
            }
        }
    }

    pub fn inspect_iteration<E: LibraryMovement>(&mut self, library: bool, effects: &[E]) -> bool {
        self.inspect_iteration_origin((library, library), effects)
    }

    pub fn inspect_iteration_origin<E: LibraryMovement>(
        &mut self,
        origin: (bool, bool),
        effects: &[E],
    ) -> bool {
        let previous = (self.iterated_library, self.iterated_mixed);
        self.iterated_library = origin.0;
        self.iterated_mixed = origin.0 && !origin.1;
        let moves = self.inspect_repetition(effects);
        (self.iterated_library, self.iterated_mixed) = previous;
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
        ConniveEffect,
        LearnEffect,
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
    if let Some(e) = payload.downcast_ref::<ConditionalEffect<E>>() {
        return Some(context.inspect_branches([e.if_true.as_slice(), e.if_false.as_slice()]));
    }
    if let Some(e) = payload.downcast_ref::<IfEffect<E>>() {
        return Some(context.inspect_branches([e.then.as_slice(), e.else_.as_slice()]));
    }
    if let Some(e) = payload.downcast_ref::<MayEffect<E>>() {
        return Some(context.inspect_branches([e.effects.as_slice(), &[]]));
    }
    if let Some(e) = payload.downcast_ref::<UnlessActionEffect<E>>() {
        return Some(context.inspect_branches([e.effects.as_slice(), e.alternative.as_slice()]));
    }
    if let Some(e) = payload.downcast_ref::<CumulativeUpkeepEffect<E>>() {
        return Some(context.inspect_branches([e.payment.as_slice(), e.failure.as_slice()]));
    }
    if let Some(e) = payload.downcast_ref::<RepeatEffectsEffect<E>>() {
        return Some(match e.count.constant_integer() {
            Some(n) if n <= 0 => false,
            Some(n) => context.inspect_exact_repetition(&e.effects, n as u64),
            None => context.inspect_repetition(&e.effects),
        });
    }
    if let Some(e) = payload.downcast_ref::<RepeatProcessEffect<E>>() {
        return Some(
            context.inspect_sequence(&e.effects) || context.inspect_repetition(&e.effects),
        );
    }
    if let Some(e) = payload.downcast_ref::<ForPlayersEffect<E>>() {
        return Some(context.inspect_repetition(&e.effects));
    }
    if let Some(e) = payload.downcast_ref::<ChooseModeEffect<E>>() {
        if context.inspect_sequence(&e.common_prefix_effects) {
            return Some(true);
        }
        if matches!(e.max.unhinted(), crate::Value::Fixed(1)) {
            return Some(
                context.inspect_branches(
                    e.modes.iter().map(|mode| mode.effects.as_slice()).chain(
                        (!matches!(e.min.unhinted(), crate::Value::Fixed(n) if *n > 0))
                            .then_some(&[][..]),
                    ),
                ),
            );
        }
        // Mode order is authored order. A mode can be absent; inspect each
        // optional program so later selected modes see prior possible tags.
        return Some(
            e.modes
                .iter()
                .any(|mode| context.inspect_branches([mode.effects.as_slice(), &[]])),
        );
    }
    if let Some(e) = payload.downcast_ref::<MayCastMatchingSpellWithoutPayingManaCostEffect>() {
        return Some(e.zone == Zone::Library);
    }
    if let Some(e) = payload.downcast_ref::<CastSourceEffect>() {
        // A source unrestricted to exile may be cast from a library. This
        // context-free query must not assert the source is on the battlefield.
        return Some(!e.require_exile);
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
        let mut payment = context.clone();
        if e.cost.inspect_library_movement(&mut payment) {
            return Some(true);
        }
        let mut failure = context.clone();
        if failure.inspect_sequence(&e.effects) {
            return Some(true);
        }
        context.merge_alternatives(&[payment, failure]);
        return Some(false);
    }
    if let Some(e) = payload.downcast_ref::<LookAtTopCardsEffect>() {
        context.bind_tag(e.tag.clone(), (true, true));
        return Some(false);
    }
    if let Some(e) = payload.downcast_ref::<RevealTopEffect>() {
        if let Some(tag) = &e.tag {
            context.bind_tag(tag.clone(), (true, true));
        }
        return Some(false);
    }
    if let Some(e) = payload.downcast_ref::<ChooseObjectsEffect>() {
        let filter = context.filter_library_origin(&e.filter);
        let may = e.zone == Some(Zone::Library)
            || e.additional_zones.contains(&Zone::Library)
            || filter.0;
        let only = e.additional_zones.iter().all(|z| *z == Zone::Library)
            && (e.zone == Some(Zone::Library) || (e.zone.is_none() && filter.1));
        context.bind_tag(e.tag.clone(), (may, only));
        return Some(false);
    }
    if let Some(e) = payload.downcast_ref::<TagMatchingObjectsEffect>() {
        let filter = context.filter_library_origin(&e.filter);
        let sources = e
            .source_tags
            .iter()
            .map(|t| context.tag_library_origin(t))
            .collect::<Vec<_>>();
        let may = e.zone == Some(Zone::Library)
            || e.additional_zones.contains(&Zone::Library)
            || filter.0
            || sources.iter().any(|o| o.0);
        let only = e.additional_zones.iter().all(|z| *z == Zone::Library)
            && (e.zone == Some(Zone::Library)
                || (e.zone.is_none()
                    && (filter.1 || (!sources.is_empty() && sources.iter().all(|o| o.1)))));
        context.bind_tag(e.tag.clone(), (may, only));
        return Some(false);
    }
    if let Some(e) = payload.downcast_ref::<ForEachObject<E>>() {
        return Some(
            context.inspect_iteration_origin(context.filter_library_origin(&e.filter), &e.effects),
        );
    }
    if let Some(e) = payload.downcast_ref::<ForEachTaggedEffect<E>>() {
        return Some(
            context.inspect_iteration_origin(context.tag_library_origin(&e.tag), &e.effects),
        );
    }
    if let Some(e) = payload.downcast_ref::<ConsultTopOfLibraryEffect>() {
        context.bind_tag(e.all_tag.clone(), (true, true));
        context.bind_tag(e.match_tag.clone(), (true, true));
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
        return Some(!context.selection_library_origin(&e.target).1);
    }
    if let Some(e) = payload.downcast_ref::<MoveToLibraryTopOrBottomChoiceEffect>() {
        return Some(!context.selection_library_origin(&e.target).1);
    }
    if let Some(e) = payload.downcast_ref::<ShuffleObjectsIntoLibraryEffect>() {
        return Some(!context.selection_library_origin(&e.target).1);
    }
    if let Some(e) = payload.downcast_ref::<PutTaggedRemainderOnLibraryBottomEffect>() {
        return Some(!context.tag_library_origin(&e.tag).1);
    }
    if let Some(e) = payload.downcast_ref::<MoveToZoneEffect>() {
        return Some(
            context.moves_selection_to(&e.target, e.zone)
                || e.tagged_destinations
                    .iter()
                    .any(|(_, zone)| context.moves_selection_to(&e.target, *zone)),
        );
    }
    if let Some(e) = payload.downcast_ref::<MayMoveToZoneEffect>() {
        return Some(context.moves_selection_to(&e.target, e.zone));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    enum Step {
        Bind(&'static str, Zone),
        Move(&'static str, Zone),
    }
    impl LibraryMovement for Step {
        fn inspect_library_movement(&self, ctx: &mut LibraryMovementContext) -> bool {
            match self {
                Step::Bind(tag, zone) => {
                    ctx.bind_tag(
                        (*tag).into(),
                        (*zone == Zone::Library, *zone == Zone::Library),
                    );
                    false
                }
                Step::Move(tag, zone) => {
                    ctx.moves_selection_to(&ChooseSpec::Tagged((*tag).into()), *zone)
                }
            }
        }
    }

    #[test]
    fn mutually_exclusive_branches_do_not_share_tag_assignments() {
        let mut ctx = LibraryMovementContext::default();
        let a = [Step::Bind("card", Zone::Library)];
        let b = [
            Step::Bind("card", Zone::Hand),
            Step::Move("card", Zone::Graveyard),
        ];
        assert!(!ctx.inspect_branches([a.as_slice(), b.as_slice()]));
        assert!(ctx.moves_selection_to(&ChooseSpec::Tagged("card".into()), Zone::Graveyard));
        assert!(ctx.moves_selection_to(&ChooseSpec::Tagged("card".into()), Zone::Library));
    }

    #[test]
    fn reassigning_a_tag_replaces_its_origin() {
        let mut ctx = LibraryMovementContext::default();
        assert!(!ctx.inspect_sequence(&[
            Step::Bind("card", Zone::Library),
            Step::Bind("card", Zone::Hand),
            Step::Move("card", Zone::Graveyard)
        ]));
        assert!(ctx.moves_selection_to(&ChooseSpec::Tagged("card".into()), Zone::Library));
    }

    #[test]
    fn every_arm_selecting_from_library_proves_reordering() {
        let mut ctx = LibraryMovementContext::default();
        let branch = [Step::Bind("card", Zone::Library)];
        assert!(!ctx.inspect_branches([branch.as_slice(), branch.as_slice()]));
        assert!(!ctx.moves_selection_to(&ChooseSpec::Tagged("card".into()), Zone::Library));
        assert!(ctx.moves_selection_to(&ChooseSpec::Tagged("card".into()), Zone::Hand));
    }

    #[test]
    fn an_optional_assignment_does_not_erase_the_unselected_path() {
        let mut ctx = LibraryMovementContext::default();
        ctx.bind_tag("card".into(), (false, false));
        assert!(!ctx.inspect_branches([&[Step::Bind("card", Zone::Library)][..], &[]]));
        assert!(ctx.moves_selection_to(&ChooseSpec::Tagged("card".into()), Zone::Library));
    }
    #[test]
    fn subsequent_iterations_see_tags_bound_by_the_first() {
        let mut ctx = LibraryMovementContext::default();
        let body = [
            Step::Move("card", Zone::Hand),
            Step::Bind("card", Zone::Library),
        ];
        assert!(ctx.inspect_repetition(&body));
    }
}
