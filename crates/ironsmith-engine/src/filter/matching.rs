//! Shared predicate interpreter for live objects and historical snapshots.
use super::*;

pub(super) fn matches_subject(
    filter: &ObjectFilter,
    subject: ObjectSubject<'_>,
    ctx: &FilterContext,
    game: &GameState,
    allow_calculated_pt: bool,
    view: Option<&crate::derived_view::DerivedGameView<'_>>,
) -> bool {
    // Specific object check
    if let Some(id) = filter.specific
        && subject.object_id() != id
    {
        return false;
    }

    if filter.source
        && ctx
            .source
            .is_none_or(|source_id| subject.object_id() != source_id)
    {
        return false;
    }

    if filter.put_onto_battlefield_with_source
        && ctx.source.is_none_or(|source_id| {
            !game.was_put_onto_battlefield_with_source(source_id, subject.object_id())
        })
    {
        return false;
    }

    if filter.created_with_source {
        let source_stable_id = ctx
            .source
            .and_then(|source_id| game.object(source_id).map(|source| source.stable_id))
            .or_else(|| ctx.source_snapshot.as_ref().map(|source| source.stable_id));
        if source_stable_id.is_none_or(|source_stable_id| {
            !game.was_token_created_with_source(source_stable_id, subject.stable_id())
        }) {
            return false;
        }
    }

    if let Some(constraint) = &filter.counters_put_on_this_turn
        && counters_put_on_exact_object_this_turn(game, subject.object_id(), constraint, ctx)
            < constraint.minimum
    {
        return false;
    }

    if let Some(targetability) = &filter.could_be_targeted_by
        && !object_could_be_targeted_by(subject.object_id(), targetability, ctx, game)
    {
        return false;
    }

    if !filter.any_of.is_empty()
        && !filter
            .any_of
            .iter()
            .any(|filter| subject.matches_nested(filter, ctx, game, allow_calculated_pt, view))
    {
        return false;
    }

    if filter.entered_since_your_last_turn_ended && !game.is_summoning_sick(subject.object_id()) {
        return false;
    }

    if filter.didnt_enter_battlefield_this_turn
        && game
            .turn_store
            .turn_history
            .object_entered_battlefield_controller_this_turn(subject.stable_id())
            .is_some()
    {
        return false;
    }

    if subject.is_live()
        && (filter.entered_battlefield_this_turn || filter.entered_battlefield_controller.is_some())
    {
        if subject.zone() != Zone::Battlefield {
            return false;
        }
        let Some(entry_controller) = game
            .turn_store
            .turn_history
            .object_entered_battlefield_controller_this_turn(subject.stable_id())
        else {
            return false;
        };
        if let Some(filter) = &filter.entered_battlefield_controller
            && !filter.matches_player(entry_controller, ctx)
        {
            return false;
        }
    }

    if subject.is_live()
        && filter.entered_graveyard_from_battlefield_this_turn
        && (subject.zone() != Zone::Graveyard
            || !game
                .turn_store
                .turn_history
                .object_was_put_into_graveyard_from_battlefield_this_turn(subject.stable_id()))
    {
        return false;
    }

    if filter.entered_graveyard_from_library_this_turn
        && (subject.zone() != Zone::Graveyard
            || !game
                .turn_store
                .turn_history
                .object_was_put_into_graveyard_from_zone_this_turn(
                    subject.stable_id(),
                    Zone::Library,
                ))
    {
        return false;
    }

    if subject.is_live()
        && filter.entered_graveyard_this_turn
        && (subject.zone() != Zone::Graveyard
            || !game
                .turn_store
                .turn_history
                .object_was_put_into_graveyard_this_turn(subject.stable_id()))
    {
        return false;
    }

    if subject.is_live()
        && filter.surveilled_this_turn
        && !game
            .turn_store
            .turn_history
            .object_was_surveilled_this_turn(subject.stable_id())
    {
        return false;
    }

    if let Some(player_filter) = &filter.discarded_or_cycled_this_turn_by
        && subject.is_live()
    {
        let matches_player = game.players.iter().any(|player| {
            player.is_in_game()
                && player_filter.matches_player(player.id, ctx)
                && game
                    .turn_store
                    .turn_history
                    .object_was_discarded_or_cycled_by_this_turn(
                        subject.object_id(),
                        subject.stable_id(),
                        player.id,
                    )
        });
        if !matches_player {
            return false;
        }
    }

    if subject.is_live()
        && filter.was_dealt_damage_this_turn
        && !game.creature_was_damaged_this_turn(subject.object_id())
    {
        return false;
    }

    if subject.is_live()
        && filter.dealt_damage_this_turn
        && !game.source_dealt_damage_this_turn(subject.object_id())
    {
        return false;
    }

    if let Some(damager) = &filter.dealt_damage_by_source_this_turn {
        let Some(source) = ctx.source else {
            return false;
        };
        let damage_source = match damager {
            ironsmith_core::DamagedBySource::ThisCreature => Some(source),
            ironsmith_core::DamagedBySource::EquippedCreature
            | ironsmith_core::DamagedBySource::EnchantedCreature => game
                .object(source)
                .and_then(|obj| obj.attached_to.as_ref())
                .and_then(|target| match target {
                    crate::object::AttachmentTarget::Object(id) => Some(*id),
                    _ => None,
                }),
        };
        let Some(damage_source) = damage_source else {
            return false;
        };
        if !game
            .turn_store
            .turn_history
            .creature_was_damaged_by_source_identity_this_turn(
                subject.object_id(),
                Some(subject.stable_id()),
                damage_source,
                game.object(damage_source)
                    .map(|obj| obj.stable_id)
                    .or_else(|| {
                        (subject.is_snapshot() && damage_source == source)
                            .then(|| {
                                ctx.source_snapshot
                                    .as_ref()
                                    .map(|snapshot| snapshot.stable_id)
                            })
                            .flatten()
                    }),
            )
        {
            return false;
        }
    }

    if subject.is_live() && filter.was_dealt_damage_by_source_this_game {
        let Some(source) = ctx.source else {
            return false;
        };
        if !game.source_dealt_damage_to_object_this_game(source, subject.object_id()) {
            return false;
        }
    }

    if let Some(player_filter) = &filter.dealt_damage_to_player_this_turn
        && subject.is_live()
    {
        let dealt_damage_to_matching_player = game.players.iter().any(|player| {
            player.is_in_game()
                && player_filter.matches_player(player.id, ctx)
                && game
                    .turn_store
                    .turn_history
                    .source_dealt_damage_to_player_this_turn_matching(
                        subject.object_id(),
                        Some(subject.stable_id()),
                        player.id,
                        filter.dealt_damage_to_player_this_turn_combat_only,
                    )
        });
        if !dealt_damage_to_matching_player {
            return false;
        }
    }

    if subject.is_live()
        && filter.drawn_this_turn
        && !game
            .turn_store
            .turn_history
            .object_was_drawn_this_turn(subject.object_id())
    {
        return false;
    }

    let Some(stack_entry) = subject.stack_context(filter, ctx, game) else {
        return false;
    };

    let calculated_chars = subject.calculated_chars(filter, game, allow_calculated_pt, view);
    let calculated_chars_ref = calculated_chars.as_deref();
    let object_card_types = subject.card_types(calculated_chars_ref);
    let object_subtypes = subject.subtypes(calculated_chars_ref);
    let object_supertypes = subject.supertypes(calculated_chars_ref);
    let object_colors = subject.colors(calculated_chars_ref);

    if filter.modified && subject.is_live() {
        if subject.zone() != Zone::Battlefield || !object_card_types.contains(&CardType::Creature) {
            return false;
        }

        let has_counters = subject.counters().values().any(|count| *count > 0);
        let has_equipment = subject.attachments().iter().any(|attachment_id| {
            game.object(*attachment_id).is_some_and(|attachment| {
                filter_object_has_subtype_with_view(
                    attachment,
                    Subtype::Equipment,
                    allow_calculated_pt,
                    view,
                    game,
                )
            })
        });
        let has_controlled_aura = ctx.you.is_some_and(|you| {
            subject.attachments().iter().any(|attachment_id| {
                game.object(*attachment_id).is_some_and(|attachment| {
                    game.current_controller(*attachment_id)
                        .is_some_and(|controller| controller == you)
                        && filter_object_has_subtype_with_view(
                            attachment,
                            Subtype::Aura,
                            allow_calculated_pt,
                            view,
                            game,
                        )
                })
            })
        });
        if !(has_counters || has_equipment || has_controlled_aura) {
            return false;
        }
    }

    if filter.suspected
        && (subject.zone() != Zone::Battlefield || !game.is_suspected(subject.object_id()))
    {
        return false;
    }
    if filter.goaded && (subject.zone() != Zone::Battlefield || !subject.goaded(game)) {
        return false;
    }

    // Controller check
    if let Some(controller_filter) = &filter.controller
        && !subject.controller(game).is_some_and(|controller| {
            player_filter_matches_game(controller_filter, controller, game, ctx)
        })
    {
        return false;
    }

    let mut resolved_cast_player = None;

    // Caster check
    if let Some(caster_filter) = &filter.cast_by {
        let cast_player = ctx.caster.or_else(|| {
            if subject.zone() == Zone::Stack {
                subject.cast_player(stack_entry)
            } else {
                None
            }
        });
        let Some(cast_player) = cast_player else {
            return false;
        };
        if !caster_filter.matches_player(cast_player, ctx) {
            return false;
        }
        resolved_cast_player = subject.is_live().then_some(cast_player);
    }

    if filter.cast_this_turn
        && !subject.has_recorded_cast_order()
        && game
            .turn_store
            .turn_history
            .spell_cast_order(subject.object_id())
            .is_none()
    {
        return false;
    }

    if let Some(source_filter) = &filter.mana_from_source_spent_to_cast
        && !subject.mana_sources_spent_to_cast().is_some_and(|sources| {
            mana_from_matching_source_was_spent_to_cast(source_filter, sources, ctx, game)
        })
    {
        return false;
    }

    if filter.first_spell_cast_each_turn
        && !first_matching_spell_cast_each_turn_matches(
            filter,
            subject.object_id(),
            ctx,
            game,
            resolved_cast_player,
        )
    {
        return false;
    }
    if let Some(minimum) = filter.spell_cast_minimum_each_turn
        && !matching_spell_cast_ordinal_each_turn_matches(
            filter,
            minimum,
            true,
            subject.object_id(),
            ctx,
            game,
            resolved_cast_player,
        )
    {
        return false;
    }
    if let Some(ordinal) = filter.spell_cast_ordinal_each_turn
        && !matching_spell_cast_ordinal_each_turn_matches(
            filter,
            ordinal,
            false,
            subject.object_id(),
            ctx,
            game,
            resolved_cast_player,
        )
    {
        return false;
    }

    // Owner check
    if let Some(owner_filter) = &filter.owner
        && !player_filter_matches_game(owner_filter, subject.owner(), game, ctx)
    {
        return false;
    }

    if filter.type_or_subtype_union && subject.is_live() {
        let type_match = !filter.card_types.is_empty()
            && filter
                .card_types
                .iter()
                .any(|t| object_card_types.contains(t));
        let subtype_match = !filter.subtypes.is_empty()
            && filter
                .subtypes
                .iter()
                .any(|t| subject.matches_subtype(calculated_chars_ref, *t, game));
        if (!filter.card_types.is_empty() || !filter.subtypes.is_empty())
            && !(type_match || subtype_match)
        {
            return false;
        }
    } else if !filter.card_types.is_empty()
        && !filter
            .card_types
            .iter()
            .any(|t| object_card_types.contains(t))
    {
        return false;
    }

    // Card types (must have all if specified)
    if !filter.all_card_types.is_empty()
        && !filter
            .all_card_types
            .iter()
            .all(|t| object_card_types.contains(t))
    {
        return false;
    }

    // Excluded card types (must have none of these)
    if filter
        .excluded_card_types
        .iter()
        .any(|t| object_card_types.contains(t))
    {
        return false;
    }

    // Subtypes (must have at least one if specified)
    if !(filter.type_or_subtype_union && subject.is_live())
        && !filter.subtypes.is_empty()
        && !filter
            .subtypes
            .iter()
            .any(|t| subject.matches_subtype(calculated_chars_ref, *t, game))
    {
        return false;
    }
    // Compound subtype phrases such as "Eldrazi Spawn" require every
    // authored subtype, unlike the inclusive-any `subtypes` collection.
    if !filter.all_subtypes.is_empty()
        && !filter
            .all_subtypes
            .iter()
            .all(|t| subject.matches_subtype(calculated_chars_ref, *t, game))
    {
        return false;
    }

    // Excluded subtypes (must have none of these)
    if filter
        .excluded_subtypes
        .iter()
        .any(|t| subject.matches_subtype(calculated_chars_ref, *t, game))
    {
        return false;
    }
    if filter.chosen_creature_type {
        let Some(source) = ctx.source else {
            return false;
        };
        if filter.has_chosen_type_this_way_surface() {
            let Some(chosen_types) = game.chosen_subtypes(source) else {
                return false;
            };
            if !chosen_types
                .iter()
                .any(|chosen_type| object_subtypes.contains(chosen_type))
            {
                return false;
            }
        } else if let Some(chosen_type) = game.chosen_subtype(source).or_else(|| {
            ctx.source_snapshot.as_ref().filter(|snapshot| snapshot.object_id == source)
                .and_then(|snapshot| snapshot.chosen_subtype)
        }) {
            if !object_subtypes.contains(&chosen_type) {
                return false;
            }
        } else if let Some(chosen_type) = game.chosen_card_type(source) {
            if !object_card_types.contains(&chosen_type) {
                return false;
            }
        } else {
            return false;
        }
    }
    if filter.chosen_land_type {
        let Some(chosen_type) = ctx.source.and_then(|source| game.chosen_land_type(source)) else {
            return false;
        };
        if !subject.matches_subtype(calculated_chars_ref, chosen_type, game) {
            return false;
        }
    }
    if filter.has_basic_land_type
        && !object_subtypes
            .iter()
            .any(|subtype| subtype.is_basic_land_type())
    {
        return false;
    }
    if filter.has_nonbasic_land_type
        && !object_subtypes
            .iter()
            .any(|subtype| subtype.is_land_subtype() && !subtype.is_basic_land_type())
    {
        return false;
    }
    if filter.chosen_card_type {
        let Some(chosen_type) = ctx.source.and_then(|source| game.chosen_card_type(source)) else {
            return false;
        };
        if !object_card_types.contains(&chosen_type) {
            return false;
        }
    }
    if filter.excluded_chosen_creature_type {
        let Some(source) = ctx.source else {
            return false;
        };
        if let Some(chosen_type) = game.chosen_subtype(source) {
            if object_subtypes.contains(&chosen_type) {
                return false;
            }
        } else if let Some(chosen_type) = game.chosen_card_type(source) {
            if object_card_types.contains(&chosen_type) {
                return false;
            }
        } else {
            return false;
        }
    }
    if filter.excluded_any_chosen_creature_type {
        let Some(source) = ctx.source else {
            return false;
        };
        let Some(chosen_types) = game.chosen_subtypes(source) else {
            return false;
        };
        if chosen_types
            .iter()
            .any(|chosen_type| object_subtypes.contains(chosen_type))
        {
            return false;
        }
    }

    // Supertypes (must have at least one if specified)
    if !filter.supertypes.is_empty()
        && !filter
            .supertypes
            .iter()
            .any(|t| object_supertypes.contains(t))
    {
        return false;
    }

    // Excluded supertypes (must have none of these)
    if filter
        .excluded_supertypes
        .iter()
        .any(|t| object_supertypes.contains(t))
    {
        return false;
    }

    if let Some(comparison) = &filter.card_type_count {
        let count = object_card_types
            .iter()
            .enumerate()
            .filter(|(index, card_type)| !object_card_types[..*index].contains(card_type))
            .count() as i32;
        if !comparison.satisfies_with_context(count, game, ctx, stack_entry) {
            return false;
        }
    }

    // Color check
    if let Some(required_colors) = filter.required_colors
        && !object_colors.contains_all(required_colors)
    {
        return false;
    }
    if let Some(required_colors) = &filter.colors
        && required_colors.intersection(object_colors).is_empty()
    {
        return false;
    }
    if filter.chosen_color {
        let Some(chosen_color) = ctx.source.and_then(|source| game.chosen_color(source)) else {
            return false;
        };
        if !object_colors.contains(chosen_color) {
            return false;
        }
    }
    if let Some(card_name) = &filter.colors_chosen_while_drafting_named {
        let Some(player) = ctx.you else {
            return false;
        };
        let drafted = game.draft_chosen_colors(player, card_name);
        if drafted.intersection(object_colors).is_empty() {
            return false;
        }
    }

    // Excluded colors check
    if !filter.excluded_colors.is_empty()
        && !filter
            .excluded_colors
            .intersection(object_colors)
            .is_empty()
    {
        return false;
    }

    // Colorless check
    if filter.colorless && !object_colors.is_empty() {
        return false;
    }

    // Multicolored check
    if filter.multicolored && object_colors.count() < 2 {
        return false;
    }

    // Monocolored check
    if filter.monocolored && object_colors.count() != 1 {
        return false;
    }

    if let Some(require_all_colors) = filter.all_colors {
        let is_all_colors = object_colors.count() == 5;
        if require_all_colors != is_all_colors {
            return false;
        }
    }

    if let Some(require_exactly_two_colors) = filter.exactly_two_colors {
        let is_exactly_two_colors = object_colors.count() == 2;
        if require_exactly_two_colors != is_exactly_two_colors {
            return false;
        }
    }
    if let Some(color_count_cmp) = &filter.color_count {
        let color_count = object_colors.count() as i32;
        if !color_count_cmp.satisfies_with_context(color_count, game, ctx, stack_entry) {
            return false;
        }
    }

    let is_historic = object_card_types.contains(&CardType::Artifact)
        || object_supertypes.contains(&Supertype::Legendary)
        || object_subtypes.contains(&Subtype::Saga);
    if filter.historic && !is_historic {
        return false;
    }
    if filter.nonhistoric && is_historic {
        return false;
    }

    // Token/nontoken check
    if filter.token && !subject.is_token() {
        return false;
    }
    if filter.nontoken && subject.is_token() {
        return false;
    }
    if let Some(require_face_down) = filter.face_down
        && subject.face_down(game) != require_face_down
    {
        return false;
    }
    if filter.foretold && !game.is_foretold(subject.object_id()) {
        return false;
    }

    // "Other" ordinarily excludes announced target objects. When the
    // filter itself is an exact tagged-set reference, however, it means
    // the other member of that set relative to a temporarily rebound
    // source (for example, each of two chosen creatures affecting the
    // other). In that shape, exclude the source rather than the full
    // announced target set.
    let other_member_of_tagged_set = filter.other
        && filter.set_quantifier_surface() == Some(ironsmith_core::SetQuantifierSurface::Those)
        && filter.tagged_constraints.len() == 1
        && filter.tagged_constraints[0].relation == TaggedOpbjectRelation::IsTaggedObject;
    // An explicit "other than this [object]" reference stays relative
    // to the source even when other targets have already been announced.
    let other_relative_to_source = other_member_of_tagged_set || filter.source_surface.is_some();
    if filter.other
        && (ctx.target_objects.is_empty() || other_relative_to_source)
        && let Some(source_id) = ctx.source
        && (subject.object_id() == source_id
            || (subject.is_snapshot()
                && game
                    .object(source_id)
                    .is_some_and(|source| source.stable_id == subject.stable_id()))
            || (game.object(source_id).is_none()
                && ctx.source_snapshot.as_ref().is_some_and(|source| {
                    source.object_id == source_id && source.stable_id == subject.stable_id()
                })))
    {
        return false;
    }
    if filter.other
        && !other_relative_to_source
        && ctx.target_objects.iter().any(|target| {
            target.object_id == subject.object_id() || target.stable_id == subject.stable_id()
        })
    {
        return false;
    }
    if filter.is_target_object
        && !ctx.target_objects.iter().any(|target| {
            target.object_id == subject.object_id() || target.stable_id == subject.stable_id()
        })
    {
        return false;
    }

    let is_tapped = subject.tapped(game);
    if filter.tapped && !is_tapped {
        return false;
    }
    if filter.untapped && is_tapped {
        return false;
    }
    if filter.enlist_eligible && !object_is_enlist_eligible(game, subject.object_id()) {
        return false;
    }
    if filter.attacking && !subject.attacking(game) {
        return false;
    }
    if filter.attacking_alone {
        let Some(combat) = game.combat.as_ref() else {
            return false;
        };
        let controller = game.controller_of_id(subject.object_id());
        if !subject.attacking(game)
            || combat
                .attackers
                .iter()
                .filter(|attacker| game.controller_of_id(attacker.creature) == controller)
                .count()
                != 1
        {
            return false;
        }
    }
    if filter.attacked_this_turn && !game.creature_attacked_this_turn(subject.object_id()) {
        return false;
    }
    if filter.ability_activated_this_turn
        && !game
            .turn_store
            .turn_history
            .activated_abilities_this_turn
            .iter()
            .any(|(source, _)| *source == subject.object_id())
    {
        return false;
    }
    if filter.blocked_this_turn && !game.creature_blocked_this_turn(subject.object_id()) {
        return false;
    }
    if filter.didnt_attack_this_turn && game.creature_attacked_this_turn(subject.object_id()) {
        return false;
    }
    if filter.could_have_attacked_this_turn && !subject.can_attack(game) {
        return false;
    }
    if let Some(player_filter) = &filter.attacking_player_or_planeswalker_controlled_by {
        let defending_player = if filter.attacking_player_only {
            attacking_player_for_object(subject.object_id(), game)
        } else {
            attacking_defending_player_for_object(subject.object_id(), game)
        };
        let Some(defending_player) = defending_player else {
            return false;
        };
        if !player_filter.matches_player(defending_player, ctx) {
            return false;
        }
    }
    if let Some(player_filter) = &filter.protected_by {
        let Some(protector) = game.battle_protector(subject.object_id()) else {
            return false;
        };
        if !player_filter.matches_player(protector, ctx) {
            return false;
        }
    }
    if subject.is_live()
        && filter.blocking
        && !game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_blocking(combat, subject.object_id()))
    {
        return false;
    }
    if filter.nonattacking && subject.attacking(game) {
        return false;
    }
    if subject.is_live()
        && filter.nonblocking
        && game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_blocking(combat, subject.object_id()))
    {
        return false;
    }
    if subject.is_live()
        && filter.blocked
        && !game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_blocked(combat, subject.object_id()))
    {
        return false;
    }
    if subject.is_live()
        && filter.unblocked
        && !game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_unblocked(combat, subject.object_id()))
    {
        return false;
    }
    if let Some(blocker_ref) = &filter.blocked_by
        && !creature_was_blocked_by_ref(game, ctx, subject.object_id(), blocker_ref)
    {
        return false;
    }
    if filter.blocked_by_source {
        let Some(source_id) = ctx.source else {
            return false;
        };
        let Some(combat) = &game.combat else {
            return false;
        };
        if !combat
            .blockers
            .get(&subject.object_id())
            .is_some_and(|blockers| blockers.contains(&source_id))
        {
            return false;
        }
    }
    if filter.in_combat_with_source
        && !object_is_in_combat_with_source_lki(game, ctx, subject.object_id())
    {
        return false;
    }
    if let Some(reference) = &filter.in_combat_with {
        let partners = resolve_object_ref_ids(reference, ctx);
        let Some(combat) = &game.combat else {
            return false;
        };
        if partners.is_empty()
            || !partners.iter().any(|partner| {
                crate::combat_state::get_blockers(combat, *partner).contains(&subject.object_id())
                    || crate::combat_state::get_blocked_attacker(combat, *partner)
                        .is_some_and(|attacker| attacker == subject.object_id())
            })
        {
            return false;
        }
    }

    // Power check
    if let Some(power_cmp) = &filter.power {
        if let Some(power) = subject.power(
            calculated_chars_ref,
            game,
            filter.power_reference,
            allow_calculated_pt,
        ) {
            if !power_cmp.satisfies_with_context(power, game, ctx, stack_entry) {
                return false;
            }
        } else {
            return false; // No power means not a creature
        }
    }
    if let Some(power_parity) = filter.power_parity {
        if let Some(power) = subject.power(
            calculated_chars_ref,
            game,
            filter.power_reference,
            allow_calculated_pt,
        ) {
            if !power_parity.matches(power, game, ctx) {
                return false;
            }
        } else {
            return false;
        }
    }
    if filter.power_greater_than_base_power {
        let Some(effective_power) = subject.power(
            calculated_chars_ref,
            game,
            PtReference::Effective,
            allow_calculated_pt,
        ) else {
            return false;
        };
        let Some(base_power) = subject.power(
            calculated_chars_ref,
            game,
            PtReference::Base,
            allow_calculated_pt,
        ) else {
            return false;
        };
        if effective_power <= base_power {
            return false;
        }
    }
    if let Some(relation) = filter.power_toughness_relation {
        let Some(power) = subject.power(
            calculated_chars_ref,
            game,
            PtReference::Effective,
            allow_calculated_pt,
        ) else {
            return false;
        };
        let Some(toughness) = subject.toughness(
            calculated_chars_ref,
            game,
            PtReference::Effective,
            allow_calculated_pt,
        ) else {
            return false;
        };
        match relation {
            PowerToughnessRelation::PowerGreaterThanToughness if power <= toughness => {
                return false;
            }
            PowerToughnessRelation::ToughnessGreaterThanPower if toughness <= power => {
                return false;
            }
            PowerToughnessRelation::NotEqual if power == toughness => return false,
            _ => {}
        }
    }

    if let Some(relation) = filter.power_relative_to_source {
        let Some(candidate_power) = subject.power(
            calculated_chars_ref,
            game,
            PtReference::Effective,
            allow_calculated_pt,
        ) else {
            return false;
        };
        let Some(source_id) = ctx.source else {
            return false;
        };
        let Some(source_obj) = game.object(source_id) else {
            return false;
        };
        let Some(source_power) = subject.source_power(source_obj, game, allow_calculated_pt) else {
            return false;
        };
        match relation {
            SourcePowerRelation::LessThanSource => {
                if candidate_power >= source_power {
                    return false;
                }
            }
        }
    }

    // Toughness check
    if let Some(toughness_cmp) = &filter.toughness {
        if let Some(toughness) = subject.toughness(
            calculated_chars_ref,
            game,
            filter.toughness_reference,
            allow_calculated_pt,
        ) {
            if !toughness_cmp.satisfies_with_context(toughness, game, ctx, stack_entry) {
                return false;
            }
        } else {
            return false;
        }
    }
    if let Some(total_cmp) = &filter.total_power_toughness {
        let Some(power) = subject.power(
            calculated_chars_ref,
            game,
            PtReference::Effective,
            allow_calculated_pt,
        ) else {
            return false;
        };
        let Some(toughness) = subject.toughness(
            calculated_chars_ref,
            game,
            PtReference::Effective,
            allow_calculated_pt,
        ) else {
            return false;
        };
        if !total_cmp.satisfies_with_context(power + toughness, game, ctx, stack_entry) {
            return false;
        }
    }

    // Mana value check
    if let Some(mv_cmp) = &filter.mana_value {
        let mv = subject.mana_value();
        if !mv_cmp.satisfies_with_context(mv, game, ctx, stack_entry) {
            return false;
        }
    }
    if let Some(mana_value_parity) = filter.mana_value_parity {
        let mv = subject.mana_value();
        if !mana_value_parity.matches(mv, game, ctx) {
            return false;
        }
    }
    if let Some(counter_type) = filter.mana_value_eq_counters_on_source {
        let Some(source_id) = ctx.source else {
            return false;
        };
        let Some(source) = game.object(source_id) else {
            return false;
        };
        let required = source.counters.get(&counter_type).copied().unwrap_or(0) as i32;
        let mv = subject.mana_value();
        if mv != required {
            return false;
        }
    }
    if let Some(required_cost) = &filter.exact_mana_cost
        && subject.mana_cost() != Some(required_cost)
    {
        return false;
    }
    if let Some(total_counters_parity) = filter.total_counters_parity {
        let total_counters = subject.counters().values().copied().sum::<u32>() as i32;
        if !total_counters_parity.matches(total_counters, game, ctx) {
            return false;
        }
    }

    // Has mana cost check (must have a non-empty mana cost)
    if filter.has_mana_cost
        && !(subject.zone() == Zone::Stack
            && (filter.zone == Some(Zone::Stack)
                || filter.stack_kind == Some(StackObjectKind::Spell)))
    {
        match subject.mana_cost() {
            Some(mc) if !mc.is_empty() => {} // Has a mana cost, OK
            _ => return false,               // No mana cost or empty
        }
    }
    if filter.has_phyrexian_mana_symbol
        && !subject.mana_cost().is_some_and(|cost| {
            cost.pips().iter().any(|pip| {
                pip.iter()
                    .any(|symbol| matches!(symbol, crate::mana::ManaSymbol::Life(_)))
            })
        })
    {
        return false;
    }

    // No X in cost check
    if filter.no_x_in_cost
        && let Some(mc) = subject.mana_cost()
        && mc.has_x()
    {
        return false;
    }
    if filter.has_x_in_cost && !subject.mana_cost().is_some_and(|cost| cost.has_x()) {
        return false;
    }

    if let Some(sticker) = filter.sticker
        && game.sticker_count_on_object(subject.object_id(), sticker, None) == 0
    {
        return false;
    }

    match subject {
        ObjectSubject::Live(object) => {
            if let Some(chars) = calculated_chars_ref {
                filter.matches_shared_tail(
                    &LayeredSubject { object, chars },
                    ctx,
                    game,
                    stack_entry,
                )
            } else {
                filter.matches_shared_tail(object, ctx, game, stack_entry)
            }
        }
        ObjectSubject::Snapshot(snapshot) => {
            filter.matches_shared_tail(snapshot, ctx, game, stack_entry)
        }
    }
}
