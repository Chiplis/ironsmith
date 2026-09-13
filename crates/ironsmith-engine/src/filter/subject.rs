//! Data access policies for a live object and its retained last-known information.
//! Predicate interpretation lives in `matching::matches_subject`.
use super::*;
use crate::derived_view::DerivedGameView;
use crate::game_state::StackEntry;
use crate::mana::ManaCost;

#[derive(Clone, Copy)]
pub(crate) enum ObjectSubject<'a> {
    Live(&'a Object),
    Snapshot(&'a ObjectSnapshot),
}

impl<'a> ObjectSubject<'a> {
    pub(crate) fn is_live(self) -> bool {
        matches!(self, Self::Live(_))
    }
    pub(crate) fn is_snapshot(self) -> bool {
        matches!(self, Self::Snapshot(_))
    }
    pub(crate) fn object_id(self) -> ObjectId {
        match self {
            Self::Live(object) => object.id,
            Self::Snapshot(snapshot) => snapshot.object_id,
        }
    }
    pub(crate) fn stable_id(self) -> StableId {
        match self {
            Self::Live(object) => object.stable_id,
            Self::Snapshot(snapshot) => snapshot.stable_id,
        }
    }
    pub(crate) fn zone(self) -> Zone {
        match self {
            Self::Live(object) => object.zone,
            Self::Snapshot(snapshot) => snapshot.zone,
        }
    }
    pub(crate) fn owner(self) -> PlayerId {
        match self {
            Self::Live(object) => object.owner,
            Self::Snapshot(snapshot) => snapshot.owner,
        }
    }
    pub(crate) fn is_token(self) -> bool {
        match self {
            Self::Live(object) => object.kind == ObjectKind::Token,
            Self::Snapshot(snapshot) => snapshot.is_token,
        }
    }
    pub(crate) fn mana_cost(self) -> Option<&'a ManaCost> {
        match self {
            Self::Live(object) => object.mana_cost.as_deref(),
            Self::Snapshot(snapshot) => snapshot.mana_cost.as_ref(),
        }
    }
    pub(crate) fn counters(self) -> &'a std::collections::HashMap<CounterType, u32> {
        match self {
            Self::Live(object) => &object.counters,
            Self::Snapshot(snapshot) => &snapshot.counters,
        }
    }
    pub(crate) fn attachments(self) -> &'a [ObjectId] {
        match self {
            Self::Live(object) => &object.attachments,
            Self::Snapshot(snapshot) => &snapshot.attachments,
        }
    }
    pub(crate) fn mana_value(self) -> i32 {
        match self {
            Self::Live(object) => object_mana_value_for_filter(object),
            Self::Snapshot(snapshot) => snapshot_mana_value_for_filter(snapshot),
        }
    }
    pub(crate) fn has_recorded_cast_order(self) -> bool {
        match self {
            Self::Live(_) => false,
            Self::Snapshot(snapshot) => snapshot.cast_order_this_turn.is_some(),
        }
    }
    pub(crate) fn mana_sources_spent_to_cast(self) -> Option<&'a [ObjectSnapshot]> {
        match self {
            Self::Live(object) => object
                .cast_tagged_objects
                .get(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG)
                .map(Vec::as_slice),
            Self::Snapshot(snapshot) => Some(&snapshot.mana_sources_spent_to_cast),
        }
    }
    pub(crate) fn controller(self, game: &GameState) -> Option<PlayerId> {
        match self {
            Self::Live(object) => game.current_controller(object.id),
            Self::Snapshot(snapshot) => Some(snapshot.controller),
        }
    }
    pub(crate) fn face_down(self, game: &GameState) -> bool {
        match self {
            Self::Live(object) => game.is_face_down(object.id),
            Self::Snapshot(snapshot) => snapshot.face_down,
        }
    }
    pub(crate) fn tapped(self, game: &GameState) -> bool {
        match self {
            Self::Live(object) => game.is_tapped(object.id),
            Self::Snapshot(snapshot) => snapshot.tapped,
        }
    }
    pub(crate) fn goaded(self, game: &GameState) -> bool {
        match self {
            Self::Live(object) => game.is_goaded(object.id),
            Self::Snapshot(snapshot) => snapshot
                .goaded
                .unwrap_or_else(|| game.is_goaded(snapshot.object_id)),
        }
    }
    pub(crate) fn attacking(self, game: &GameState) -> bool {
        match self {
            Self::Live(object) => game
                .combat
                .as_ref()
                .is_some_and(|combat| crate::combat_state::is_attacking(combat, object.id)),
            Self::Snapshot(snapshot) => snapshot.attacking,
        }
    }
    pub(crate) fn can_attack(self, game: &GameState) -> bool {
        match self {
            Self::Live(object) => crate::rules::combat::can_attack(object, game),
            Self::Snapshot(snapshot) => game
                .object(snapshot.object_id)
                .is_some_and(|object| crate::rules::combat::can_attack(object, game)),
        }
    }
    pub(super) fn card_types<'b>(
        self,
        chars: Option<&'b CalculatedCharacteristics>,
    ) -> &'b [CardType]
    where
        'a: 'b,
    {
        match self {
            Self::Live(object) => filter_card_types(object, chars),
            Self::Snapshot(snapshot) => &snapshot.card_types,
        }
    }
    pub(super) fn subtypes<'b>(self, chars: Option<&'b CalculatedCharacteristics>) -> &'b [Subtype]
    where
        'a: 'b,
    {
        match self {
            Self::Live(object) => filter_subtypes(object, chars),
            Self::Snapshot(snapshot) => &snapshot.subtypes,
        }
    }
    pub(super) fn supertypes<'b>(
        self,
        chars: Option<&'b CalculatedCharacteristics>,
    ) -> &'b [Supertype]
    where
        'a: 'b,
    {
        match self {
            Self::Live(object) => filter_supertypes(object, chars),
            Self::Snapshot(snapshot) => &snapshot.supertypes,
        }
    }
    pub(super) fn colors(self, chars: Option<&CalculatedCharacteristics>) -> ColorSet {
        match self {
            Self::Live(object) => filter_colors(object, chars),
            Self::Snapshot(snapshot) => snapshot.colors,
        }
    }
    pub(super) fn matches_subtype(
        self,
        chars: Option<&CalculatedCharacteristics>,
        subtype: Subtype,
        game: &GameState,
    ) -> bool {
        match self {
            Self::Live(object) => filter_subject_matches_subtype(
                object,
                chars.map(|chars| LayeredSubject { object, chars }).as_ref(),
                subtype,
                game,
            ),
            Self::Snapshot(snapshot) => snapshot_matches_subtype(snapshot, subtype, game),
        }
    }
    pub(super) fn cast_player(self, entry: Option<&StackEntry>) -> Option<PlayerId> {
        match self {
            Self::Live(_) => entry.map(|entry| entry.controller),
            Self::Snapshot(snapshot) => Some(snapshot.controller),
        }
    }
    pub(super) fn source_power(
        self,
        source: &Object,
        game: &GameState,
        allow_calculated: bool,
    ) -> Option<i32> {
        match self {
            Self::Live(_) => resolve_object_power_for_filter(
                source,
                game,
                PtReference::Effective,
                allow_calculated,
            ),
            Self::Snapshot(_) => game.calculated_power(source.id).or_else(|| source.power()),
        }
    }
    pub(crate) fn matches(
        self,
        filter: &ObjectFilter,
        ctx: &FilterContext,
        game: &GameState,
    ) -> bool {
        match self {
            Self::Live(object) => filter.matches(object, ctx, game),
            Self::Snapshot(snapshot) => filter.matches_snapshot(snapshot, ctx, game),
        }
    }
    pub(super) fn matches_nested(
        self,
        filter: &ObjectFilter,
        ctx: &FilterContext,
        game: &GameState,
        allow_calculated: bool,
        view: Option<&DerivedGameView<'_>>,
    ) -> bool {
        match self {
            Self::Live(object) => {
                filter.matches_internal(object, ctx, game, allow_calculated, view)
            }
            Self::Snapshot(snapshot) => filter.matches_snapshot(snapshot, ctx, game),
        }
    }
    pub(super) fn power(
        self,
        chars: Option<&CalculatedCharacteristics>,
        game: &GameState,
        reference: PtReference,
        allow_calculated: bool,
    ) -> Option<i32> {
        match self {
            Self::Live(object) => resolve_layered_object_power_for_filter(
                object,
                chars,
                game,
                reference,
                allow_calculated,
            ),
            Self::Snapshot(snapshot) => resolve_snapshot_power_for_filter(snapshot, reference),
        }
    }
    pub(super) fn toughness(
        self,
        chars: Option<&CalculatedCharacteristics>,
        game: &GameState,
        reference: PtReference,
        allow_calculated: bool,
    ) -> Option<i32> {
        match self {
            Self::Live(object) => resolve_layered_object_toughness_for_filter(
                object,
                chars,
                game,
                reference,
                allow_calculated,
            ),
            Self::Snapshot(snapshot) => resolve_snapshot_toughness_for_filter(snapshot, reference),
        }
    }
    pub(super) fn calculated_chars(
        self,
        filter: &ObjectFilter,
        game: &GameState,
        allow_calculated_pt: bool,
        view: Option<&DerivedGameView<'_>>,
    ) -> Option<std::sync::Arc<CalculatedCharacteristics>> {
        let Self::Live(object) = self else {
            return None;
        };
        let needs_pt = filter.uses_power_or_toughness_characteristics();
        let needs_non_pt = filter.uses_non_pt_battlefield_characteristics();
        let should_consider_adjusted_object = allow_calculated_pt && (needs_pt || needs_non_pt);
        let should_calculate_chars = should_consider_adjusted_object
            && match view {
                Some(view) if object.zone == Zone::Battlefield => {
                    needs_pt || view.requires_battlefield_characteristic_calculation(object.id)
                }
                Some(_) => true,
                None => true,
            };
        let calculated_chars: Option<std::sync::Arc<CalculatedCharacteristics>> =
            if should_calculate_chars {
                if object.zone == Zone::Battlefield {
                    view.and_then(|view| {
                        if needs_pt
                            || view.requires_battlefield_characteristic_calculation(object.id)
                        {
                            view.calculated_characteristics_arc(object.id)
                        } else {
                            None
                        }
                    })
                    .or_else(|| game.calculated_characteristics_arc(object.id))
                } else {
                    view.and_then(|view| view.current_characteristics_arc(object.id))
                        .or_else(|| {
                            game.current_characteristics(object.id)
                                .map(std::sync::Arc::new)
                        })
                }
            } else {
                None
            };
        calculated_chars
    }
    fn cast_origin(
        self,
        game: &GameState,
        entry: Option<&StackEntry>,
        exclude: bool,
    ) -> Option<Zone> {
        match self {
            Self::Live(object) => {
                entry.and_then(|entry| stack_spell_cast_origin_zone(object, entry))
            }
            Self::Snapshot(snapshot) => {
                if exclude && snapshot.kind == ObjectKind::SpellCopy {
                    return None;
                }
                game.cast_origin_snapshot(snapshot.object_id)
                    .map(|origin| origin.zone)
                    .or_else(|| {
                        let object = game.object(snapshot.object_id)?;
                        let entry = game
                            .stack
                            .iter()
                            .find(|entry| entry.object_id == snapshot.object_id)?;
                        stack_spell_cast_origin_zone(object, entry)
                    })
            }
        }
    }

    // Outer None is an unavailable/mismatched zone; inner None means the
    // predicate has no live stack entry (always the case for historical reads).
    pub(super) fn stack_context<'g>(
        self,
        filter: &ObjectFilter,
        ctx: &FilterContext,
        game: &'g GameState,
    ) -> Option<Option<&'g StackEntry>> {
        let wants_stack = filter.zone == Some(Zone::Stack)
            || filter.stack_kind.is_some()
            || filter.excluded_cast_origin_zone.is_some()
            || filter.target_count.is_some()
            || filter.targets_only_player.is_some()
            || filter.targets_only_object.is_some()
            || filter.targets_player.is_some()
            || filter.targets_object.is_some()
            || (filter.zone.is_some_and(|zone| zone != Zone::Stack) && self.zone() == Zone::Stack);
        let entry = if self.is_live() && wants_stack {
            game.stack
                .iter()
                .find(|entry| entry.object_id == self.object_id())
        } else {
            None
        };
        if self.is_live() && wants_stack {
            let prospective_spell = self.zone() == Zone::Stack
                && filter.stack_kind == Some(StackObjectKind::Spell)
                && ctx.caster.is_some()
                && filter.target_count.is_none()
                && filter.targets_only_player.is_none()
                && filter.targets_only_object.is_none()
                && filter.targets_player.is_none()
                && filter.targets_object.is_none();
            if (filter.zone == Some(Zone::Stack) || filter.stack_kind.is_some())
                && entry.is_none()
                && !prospective_spell
            {
                return None;
            }
        }
        if let Some(zone) = filter.zone
            && zone != self.zone()
        {
            // A live stack filter is established by its entry above. Historical
            // snapshots require their retained zone. Non-stack filters on a
            // spell mean its cast origin, with the original live-history fallback.
            let live_stack_entry = self.is_live() && zone == Zone::Stack;
            let cast_from_zone = self.zone() == Zone::Stack
                && zone != Zone::Stack
                && (filter.stack_kind == Some(StackObjectKind::Spell)
                    || (self.is_live()
                        && game
                            .turn_store
                            .turn_history
                            .spell_cast_order(self.object_id())
                            .is_some()))
                && self.cast_origin(game, entry, false) == Some(zone);
            if !live_stack_entry && !cast_from_zone {
                return None;
            }
        }
        if let Some(zone) = filter.excluded_cast_origin_zone
            && (self.zone() != Zone::Stack || self.cast_origin(game, entry, true) == Some(zone))
        {
            return None;
        }
        if self.is_live()
            && let Some(kind) = filter.stack_kind
        {
            if let Some(entry) = entry {
                if !ObjectFilter::stack_entry_matches_kind(entry, kind) {
                    return None;
                }
            } else if !(self.zone() == Zone::Stack
                && kind == StackObjectKind::Spell
                && ctx.caster.is_some())
            {
                return None;
            }
        }
        Some(entry)
    }
}
