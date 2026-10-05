mod context;
use crate::effect::Condition;
use crate::effect::Value;
use crate::effects::helpers::resolve_value;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::{FilterContext, ObjectFilterExt as _, player_filter_matches_game};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::target::PlayerFilter;
use crate::zone::Zone;
use context::ConditionContext;

use crate::triggers::{TriggerEvent, TriggerIdentity};
use ironsmith_core::DamagedBySource;

const CREWERS_TAG: &str = "crewed_it_this_turn";
const FIRST_CREWED_THIS_TURN_TAG: &str = "__first_crewed_this_turn";
const IMPLICIT_IT_TAG: &str = "__it__";

fn source_is_face_down_or_alternate_face(game: &GameState, source: ObjectId) -> bool {
    // `SourceIsFaceDown` is also used by daybound/nightbound lowering to mean
    // "this DFC is currently showing its alternate face."
    game.is_face_down(source) || game.transform_count(source) % 2 == 1
}

#[cfg(test)]
mod prime_value_tests {
    use super::is_prime_integer;

    #[test]
    fn primality_rejects_nonpositive_units_and_composites() {
        for value in [-7, 0, 1, 4, 9, 25, 49] {
            assert!(!is_prime_integer(value), "{value} must not be prime");
        }
        for value in [2, 3, 5, 31, 97] {
            assert!(is_prime_integer(value), "{value} must be prime");
        }
    }
}

fn attachment_count_condition_matches(
    game: &GameState,
    source: ObjectId,
    attachment: &crate::target::ObjectFilter,
    host: &ironsmith_core::AttachmentConditionHost,
    comparison: &crate::effect::Comparison,
    filter_ctx: &FilterContext,
) -> bool {
    let host_satisfies = |host_id: ObjectId| {
        game.object(host_id).is_some_and(|host_object| {
            let count = host_object
                .attachments
                .iter()
                .filter(|attachment_id| {
                    game.object(**attachment_id)
                        .is_some_and(|object| attachment.matches(object, filter_ctx, game))
                })
                .count() as i32;
            comparison.evaluate(count)
        })
    };

    match host {
        ironsmith_core::AttachmentConditionHost::Source => host_satisfies(source),
        ironsmith_core::AttachmentConditionHost::SourceAttachedObject => game
            .object(source)
            .and_then(|source_object| source_object.attached_to)
            .and_then(|target| target.object_id())
            .is_some_and(host_satisfies),
        ironsmith_core::AttachmentConditionHost::Matching(host_filter) => {
            game.battlefield.iter().copied().any(|host_id| {
                game.object(host_id).is_some_and(|host_object| {
                    host_filter.matches(host_object, filter_ctx, game) && host_satisfies(host_id)
                })
            })
        }
    }
}

fn source_was_cast(
    game: &GameState,
    source: ObjectId,
    triggering_event: Option<&TriggerEvent>,
) -> bool {
    if let Some(event) = triggering_event
        && let Some(etb) = event.downcast::<crate::events::EnterBattlefieldEvent>()
        && etb.object == source
    {
        return etb.from == Zone::Stack && !resolved_from_uncast_spell_copy(game, source, None);
    }
    if let Some(event) = triggering_event
        && let Some(zc) = event.downcast::<crate::events::ZoneChangeEvent>()
        && zc.to == Zone::Battlefield
        && zc.objects.contains(&source)
    {
        return zc.from == Zone::Stack && !resolved_from_uncast_spell_copy(game, source, None);
    }
    game.turn_store
        .turn_history
        .spell_cast_order(source)
        .is_some()
}

/// Whether `source` was cast and its most recent cast this turn was from `zone`.
fn source_was_cast_from_zone(
    game: &GameState,
    source: ObjectId,
    triggering_event: Option<&TriggerEvent>,
    zone: Zone,
) -> bool {
    // "When you cast this spell from ..." reads the cast event itself: the
    // event names the origin zone even before turn history records it.
    if let Some(cast) =
        triggering_event.and_then(|event| event.downcast::<crate::events::spells::SpellCastEvent>())
        && cast.spell == source
    {
        return cast.from_zone == zone;
    }
    if !source_was_cast(game, source, triggering_event) {
        return false;
    }
    let stable_id = game.object(source).map(|obj| obj.stable_id).or_else(|| {
        triggering_event
            .and_then(TriggerEvent::snapshot)
            .map(|snapshot| snapshot.stable_id)
    });
    stable_id.and_then(|stable_id| game.turn_store.turn_history.latest_cast_zone(stable_id))
        == Some(zone)
}

/// The object a "cast it from ..." intervening-if asks about: the permanent
/// whose entering triggered the ability. A self-ETB trigger ("When this
/// creature enters, if you cast it from your hand") names the source itself;
/// Wild Pair's "Whenever a creature enters, if you cast it from your hand"
/// names the entering creature.
fn cast_condition_subject(source: ObjectId, triggering_event: Option<&TriggerEvent>) -> ObjectId {
    let Some(event) = triggering_event else {
        return source;
    };
    let entering = event
        .downcast::<crate::events::EnterBattlefieldEvent>()
        .map(|etb| etb.object)
        .or_else(|| {
            event
                .downcast::<crate::events::zones::ZoneChangeEvent>()
                .filter(|zc| zc.to == Zone::Battlefield && zc.objects.len() == 1)
                .map(|zc| zc.objects[0])
        });
    entering.unwrap_or(source)
}

/// Whether `source` was cast and its most recent cast was from a zone other
/// than its owner's hand.
fn source_was_cast_from_non_hand(
    game: &GameState,
    source: ObjectId,
    triggering_event: Option<&TriggerEvent>,
) -> bool {
    if !source_was_cast(game, source, triggering_event) {
        return false;
    }
    let stable_id = game.object(source).map(|obj| obj.stable_id).or_else(|| {
        triggering_event
            .and_then(TriggerEvent::snapshot)
            .map(|snapshot| snapshot.stable_id)
    });
    stable_id
        .and_then(|stable_id| game.turn_store.turn_history.latest_cast_zone(stable_id))
        .is_some_and(|zone| zone != Zone::Hand)
}

/// CR 707.10 / 707.10f: a copy of a permanent spell becomes a token as it
/// resolves, and it was never cast. Tokens are never otherwise on the stack, so
/// a token that entered from the stack without a cast record (CR 707.12 cast
/// copies have one) came from an uncast spell copy.
fn resolved_from_uncast_spell_copy(
    game: &GameState,
    object_id: ObjectId,
    snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> bool {
    let (kind, stable_id) = match game.object(object_id) {
        Some(obj) => (obj.kind, obj.stable_id),
        None => match snapshot {
            Some(snapshot) => (snapshot.kind, snapshot.stable_id),
            None => return false,
        },
    };
    matches!(
        kind,
        crate::object::ObjectKind::Token | crate::object::ObjectKind::SpellCopy
    ) && game
        .turn_store
        .turn_history
        .latest_cast_zone(stable_id)
        .is_none()
}

fn tagged_object_was_cast(game: &GameState, tag: &crate::TagKey, ctx: &ExecutionContext) -> bool {
    let Some(tagged) = ctx.get_tagged_all(tag.as_str()) else {
        return false;
    };
    for snapshot in tagged {
        if let Some(event) = &ctx.triggering_event
            && let Some(etb) = event.downcast::<crate::events::EnterBattlefieldEvent>()
            && etb.from == Zone::Stack
            && etb.object == snapshot.object_id
            && !resolved_from_uncast_spell_copy(game, snapshot.object_id, Some(snapshot))
        {
            return true;
        }
        if let Some(event) = &ctx.triggering_event
            && let Some(zc) = event.downcast::<crate::events::ZoneChangeEvent>()
            && zc.from == Zone::Stack
            && zc.to == Zone::Battlefield
            && (zc.objects.contains(&snapshot.object_id)
                || zc.result_objects.contains(&snapshot.object_id))
            && !resolved_from_uncast_spell_copy(game, snapshot.object_id, Some(snapshot))
        {
            return true;
        }
        if game
            .turn_store
            .turn_history
            .spell_cast_order(snapshot.object_id)
            .is_some()
        {
            return true;
        }
    }
    false
}

fn target_objects_have_different_color_sets(game: &GameState, ctx: &ExecutionContext) -> bool {
    let mut colors = ctx.targets.iter().filter_map(|target| {
        let crate::effects::ResolvedTarget::Object(object_id) = target else {
            return None;
        };
        game.current_colors(*object_id).or_else(|| {
            ctx.target_snapshots
                .get(object_id)
                .map(|snapshot| snapshot.colors)
        })
    });
    let Some(first) = colors.next() else {
        return false;
    };
    let Some(second) = colors.next() else {
        return false;
    };
    second != first || colors.any(|candidate| candidate != first)
}

fn mana_pool_amount(
    spent: &crate::player::ManaPool,
    symbol: Option<crate::mana::ManaSymbol>,
) -> u32 {
    if let Some(symbol) = symbol {
        spent.amount(symbol)
    } else {
        spent.total()
    }
}

fn mana_pool_colored_total(spent: &crate::player::ManaPool) -> u32 {
    spent.white + spent.blue + spent.black + spent.red + spent.green
}

fn triggering_spell_mana_spent_at_least(
    game: &GameState,
    triggering_event: Option<&TriggerEvent>,
    amount: u32,
    symbol: Option<crate::mana::ManaSymbol>,
) -> bool {
    let Some(event) = triggering_event else {
        return false;
    };
    let Some(spell_cast) = event.downcast::<crate::events::SpellCastEvent>() else {
        return false;
    };
    if let Some(snapshot) = spell_cast.snapshot.as_ref() {
        return mana_pool_amount(&snapshot.mana_spent_to_cast, symbol) >= amount;
    }
    game.object(spell_cast.spell)
        .is_some_and(|obj| mana_pool_amount(&obj.mana_spent_to_cast, symbol) >= amount)
}

/// "Whenever you clash, ... If you won, ...": the triggering clash event
/// names its winner (CR 701.30c); the condition holds only when that winner
/// is this ability's controller.
fn you_won_triggering_clash(triggering_event: Option<&TriggerEvent>, controller: PlayerId) -> bool {
    let Some(event) = triggering_event
        .and_then(|event| event.downcast::<crate::events::other::KeywordActionEvent>())
    else {
        return false;
    };
    event.action == crate::events::other::KeywordActionKind::Clash
        && event
            .player_tags
            .get(&crate::tag::TagKey::from("winner"))
            .is_some_and(|winners| winners.contains(&controller))
}

/// The entering permanent shows the back face of a transforming double-faced
/// card and has not transformed since it entered: it entered transformed
/// (CR 712.14). Transform-like families keep the front face at the lower card
/// id.
fn triggering_object_entered_transformed(
    game: &GameState,
    triggering_event: Option<&TriggerEvent>,
) -> bool {
    let Some(object_id) = triggering_event.and_then(|event| event.object_id()) else {
        return false;
    };
    let Some(object) = game.object(object_id) else {
        return false;
    };
    if object.linked_face_layout != crate::card::LinkedFaceLayout::TransformLike
        || game.transform_count(object_id) != 0
    {
        return false;
    }
    let Some(current) = game.displayed_face_definition(object) else {
        return false;
    };
    let Some(other) = game
        .linked_face_definition_by_name_or_id(object.other_face_name.as_deref(), object.other_face)
    else {
        return false;
    };
    current.card.id.0 > other.card.id.0
}

fn triggering_spell_was_kicked(game: &GameState, triggering_event: Option<&TriggerEvent>) -> bool {
    let Some(spell_cast) =
        triggering_event.and_then(|event| event.downcast::<crate::events::SpellCastEvent>())
    else {
        return false;
    };
    if let Some(snapshot) = spell_cast.snapshot.as_ref() {
        return snapshot.optional_costs_paid.was_kicked();
    }
    game.object(spell_cast.spell)
        .is_some_and(|obj| obj.optional_costs_paid.was_kicked())
}

fn triggering_spell_colored_mana_spent_at_least(
    game: &GameState,
    triggering_event: Option<&TriggerEvent>,
    amount: u32,
) -> bool {
    let Some(event) = triggering_event else {
        return false;
    };
    let Some(spell_cast) = event.downcast::<crate::events::SpellCastEvent>() else {
        return false;
    };
    if let Some(snapshot) = spell_cast.snapshot.as_ref() {
        return mana_pool_colored_total(&snapshot.mana_spent_to_cast) >= amount;
    }
    game.object(spell_cast.spell)
        .is_some_and(|obj| mana_pool_colored_total(&obj.mana_spent_to_cast) >= amount)
}

/// CR 707.10: a copy of a spell isn't cast. It copies the casting method (the
/// alternative cost paid) but not the zone the original was cast from, so an
/// uncast spell copy was never cast from any zone. A copy that was itself cast
/// (CR 707.12) has its own cast record and is unaffected.
fn is_uncast_spell_copy(game: &GameState, source: ObjectId) -> bool {
    game.object(source).is_some_and(|obj| {
        obj.kind == crate::object::ObjectKind::SpellCopy
            && game
                .turn_store
                .turn_history
                .latest_cast_zone(obj.stable_id)
                .is_none()
            && game
                .turn_store
                .turn_history
                .spell_cast_order(source)
                .is_none()
    })
}

fn this_spell_was_cast_from_zone(
    game: &GameState,
    source: ObjectId,
    ctx: &ExecutionContext,
    zone: Zone,
) -> bool {
    if is_uncast_spell_copy(game, source) {
        return false;
    }
    match ctx.casting_method.origin_method() {
        crate::alternative_cast::CastingMethod::GrantedFlashback => zone == Zone::Graveyard,
        crate::alternative_cast::CastingMethod::GrantedEscape { .. } => zone == Zone::Graveyard,
        crate::alternative_cast::CastingMethod::PlayFrom {
            zone: from_zone, ..
        } => *from_zone == zone,
        crate::alternative_cast::CastingMethod::SplitOtherHalfPlayFrom {
            zone: from_zone, ..
        }
        | crate::alternative_cast::CastingMethod::FaceDownPlayFrom {
            zone: from_zone, ..
        } => *from_zone == zone,
        // A native alternative (dash, evoke, blitz...) reports the hand, but a
        // commander can use it from the command zone (CR 903.8): the recorded
        // cast origin is authoritative.
        crate::alternative_cast::CastingMethod::Alternative(idx) => {
            recorded_cast_zone(game, source)
                .or_else(|| {
                    game.object(source)
                        .and_then(|obj| obj.alternative_casts.get(*idx))
                        .map(|method| method.cast_from_zone())
                })
                .is_some_and(|cast_zone| cast_zone == zone)
        }
        crate::alternative_cast::CastingMethod::AlternativePrice { .. } => false,
        crate::alternative_cast::CastingMethod::Normal
        | crate::alternative_cast::CastingMethod::FaceDown
        | crate::alternative_cast::CastingMethod::SplitOtherHalf
        | crate::alternative_cast::CastingMethod::Fuse => false,
    }
}

fn this_spell_was_cast_from_non_hand(
    game: &GameState,
    source: ObjectId,
    ctx: &ExecutionContext,
) -> bool {
    if is_uncast_spell_copy(game, source) {
        return false;
    }
    match ctx.casting_method.origin_method() {
        crate::alternative_cast::CastingMethod::AlternativePrice { .. } => false,
        crate::alternative_cast::CastingMethod::Normal
        | crate::alternative_cast::CastingMethod::FaceDown
        | crate::alternative_cast::CastingMethod::SplitOtherHalf
        | crate::alternative_cast::CastingMethod::Fuse => {
            recorded_cast_zone(game, source).is_some_and(|cast_zone| cast_zone != Zone::Hand)
        }
        crate::alternative_cast::CastingMethod::GrantedFlashback
        | crate::alternative_cast::CastingMethod::GrantedEscape { .. } => true,
        crate::alternative_cast::CastingMethod::PlayFrom { zone, .. }
        | crate::alternative_cast::CastingMethod::SplitOtherHalfPlayFrom { zone, .. }
        | crate::alternative_cast::CastingMethod::FaceDownPlayFrom { zone, .. } => {
            *zone != Zone::Hand
        }
        crate::alternative_cast::CastingMethod::Alternative(idx) => {
            recorded_cast_zone(game, source)
                .or_else(|| {
                    game.object(source)
                        .and_then(|obj| obj.alternative_casts.get(*idx))
                        .map(|method| method.cast_from_zone())
                })
                .is_some_and(|cast_zone| cast_zone != Zone::Hand)
        }
    }
}

/// The zone this object was most recently cast from, per the turn's cast
/// history (the origin recorded at CR 601.2a).
fn recorded_cast_zone(game: &GameState, source: ObjectId) -> Option<Zone> {
    let stable_id = game.object(source)?.stable_id;
    game.turn_store.turn_history.latest_cast_zone(stable_id)
}

fn source_escaped(game: &GameState, source: ObjectId) -> bool {
    game.object(source)
        .is_some_and(|obj| obj.optional_costs_paid.was_paid_label("Escape"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ExecutionContext;
    use crate::events::cause::EventCause;
    use crate::events::{DamageEvent, DamageTarget, RawEvent};
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::AttachmentTarget;
    use crate::player::ManaPool;
    use crate::provenance::ProvNodeId;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn add_hand_card(game: &mut GameState, id_raw: u32, name: &str, owner_index: usize) {
        let card = CardBuilder::new(CardId::from_raw(id_raw), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let owner = game.players[owner_index].id;
        game.create_object_from_card(&card, owner, Zone::Hand);
    }

    fn add_battlefield_land(game: &mut GameState, id_raw: u32, name: &str, owner_index: usize) {
        let card = CardBuilder::new(CardId::from_raw(id_raw), name)
            .card_types(vec![CardType::Land])
            .build();
        let owner = game.players[owner_index].id;
        game.create_object_from_card(&card, owner, Zone::Battlefield);
    }

    fn add_battlefield_permanent(
        game: &mut GameState,
        id_raw: u32,
        name: &str,
        owner_index: usize,
        card_type: CardType,
        subtype: Option<Subtype>,
    ) -> ObjectId {
        let mut builder =
            CardBuilder::new(CardId::from_raw(id_raw), name).card_types(vec![card_type]);
        if let Some(subtype) = subtype {
            builder = builder.subtypes(vec![subtype]);
        }
        let card = builder.build();
        let owner = game.players[owner_index].id;
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    fn attach_for_test(game: &mut GameState, attachment: ObjectId, host: ObjectId) {
        game.object_mut(attachment).unwrap().attached_to = Some(AttachmentTarget::Object(host));
        game.object_mut(host).unwrap().attachments.push(attachment);
    }

    #[test]
    fn ability_resolution_ordinal_condition_uses_activated_ability_index() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = game.players[0].id;
        let source = ObjectId(8_100);
        let ability_index = 4;
        for _ in 0..3 {
            game.record_activated_ability_resolved(source, ability_index);
        }
        let ctx = ExecutionContext::new_default(source, alice).with_ability_index(ability_index);

        assert!(
            evaluate_condition(
                &game,
                &Condition::ThisAbilityResolvedThisTurnExactly(3),
                &ctx,
            )
            .expect("activated ordinal condition should evaluate")
        );
        assert!(
            !evaluate_condition(
                &game,
                &Condition::ThisAbilityResolvedThisTurnExactly(2),
                &ctx,
            )
            .expect("activated ordinal near miss should evaluate")
        );
    }

    #[test]
    fn attachment_count_is_evaluated_per_matching_host() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = game.players[0].id;
        let first_host =
            add_battlefield_permanent(&mut game, 100, "First Host", 0, CardType::Creature, None);
        let second_host =
            add_battlefield_permanent(&mut game, 101, "Second Host", 0, CardType::Creature, None);
        let first_equipment = add_battlefield_permanent(
            &mut game,
            102,
            "First Equipment",
            0,
            CardType::Artifact,
            Some(Subtype::Equipment),
        );
        let second_equipment = add_battlefield_permanent(
            &mut game,
            103,
            "Second Equipment",
            0,
            CardType::Artifact,
            Some(Subtype::Equipment),
        );
        attach_for_test(&mut game, first_equipment, first_host);
        attach_for_test(&mut game, second_equipment, second_host);

        let condition = Condition::AttachmentCount {
            attachment: crate::target::ObjectFilter::default().with_subtype(Subtype::Equipment),
            host: ironsmith_core::AttachmentConditionHost::Matching(
                crate::target::ObjectFilter::creature().you_control(),
            ),
            comparison: crate::effect::Comparison::GreaterThanOrEqual(2),
            display: "two or more Equipment are attached to a creature you control".to_string(),
        };
        let ctx = ExecutionContext::new_default(first_host, alice);
        assert!(
            !evaluate_condition(&game, &condition, &ctx).unwrap(),
            "attachments on different hosts must not be aggregated"
        );

        game.object_mut(second_host)
            .unwrap()
            .attachments
            .retain(|id| *id != second_equipment);
        game.object_mut(second_equipment).unwrap().attached_to =
            Some(AttachmentTarget::Object(first_host));
        game.object_mut(first_host)
            .unwrap()
            .attachments
            .push(second_equipment);
        assert!(
            evaluate_condition(&game, &condition, &ctx).unwrap(),
            "two attachments on one matching host should satisfy the comparison"
        );
    }

    #[test]
    fn source_attached_object_count_honors_other_source_filtering() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = game.players[0].id;
        let enchanted = add_battlefield_permanent(
            &mut game,
            110,
            "Enchanted Creature",
            0,
            CardType::Creature,
            None,
        );
        let other_host = add_battlefield_permanent(
            &mut game,
            111,
            "Other Creature",
            0,
            CardType::Creature,
            None,
        );
        let source_aura = add_battlefield_permanent(
            &mut game,
            112,
            "Source Aura",
            0,
            CardType::Enchantment,
            Some(Subtype::Aura),
        );
        let other_aura = add_battlefield_permanent(
            &mut game,
            113,
            "Other Aura",
            0,
            CardType::Enchantment,
            Some(Subtype::Aura),
        );
        attach_for_test(&mut game, source_aura, enchanted);
        attach_for_test(&mut game, other_aura, other_host);

        let mut other_aura_filter =
            crate::target::ObjectFilter::default().with_subtype(Subtype::Aura);
        other_aura_filter.other = true;
        let condition = Condition::AttachmentCount {
            attachment: other_aura_filter,
            host: ironsmith_core::AttachmentConditionHost::SourceAttachedObject,
            comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
            display: "another Aura is attached to enchanted creature".to_string(),
        };
        let ctx = ExecutionContext::new_default(source_aura, alice);
        assert!(
            !evaluate_condition(&game, &condition, &ctx).unwrap(),
            "the source Aura and an Aura on another creature must not count"
        );

        game.object_mut(other_host)
            .unwrap()
            .attachments
            .retain(|id| *id != other_aura);
        game.object_mut(other_aura).unwrap().attached_to =
            Some(AttachmentTarget::Object(enchanted));
        game.object_mut(enchanted)
            .unwrap()
            .attachments
            .push(other_aura);
        assert!(
            evaluate_condition(&game, &condition, &ctx).unwrap(),
            "another Aura on the source's enchanted creature should count"
        );
    }

    #[test]
    fn cast_from_hand_intervening_if_uses_recorded_cast_zone() {
        // Wakening Sun's Avatar: "When this creature enters, if you cast it
        // from your hand, destroy all non-Dinosaur creatures." The intervening
        // if is evaluated without an execution context.
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = game.players[0].id;
        let avatar = add_battlefield_permanent(
            &mut game,
            120,
            "Wakening Sun's Avatar",
            0,
            CardType::Creature,
            Some(Subtype::Dinosaur),
        );
        let spell_id = game.new_object_id();
        let mut snapshot =
            crate::snapshot::ObjectSnapshot::for_testing(spell_id, alice, "Wakening Sun's Avatar");
        snapshot.stable_id = game.object(avatar).unwrap().stable_id;
        let cast = RawEvent::new(
            crate::events::spells::SpellCastEvent::new_with_snapshot(
                spell_id,
                alice,
                Zone::Hand,
                snapshot,
            ),
            ProvNodeId::default(),
        );
        game.turn_store.turn_history.record_event(&cast, None, None);

        let from_hand = Condition::ThisSpellWasCastFromZone(Zone::Hand);
        let from_graveyard = Condition::ThisSpellWasCastFromZone(Zone::Graveyard);
        let verify = |condition: &Condition, entered_from: Zone| {
            let etb = RawEvent::new(
                crate::events::EnterBattlefieldEvent::new(avatar, entered_from),
                ProvNodeId::default(),
            );
            crate::triggers::verify_intervening_if(
                &game, condition, alice, &etb, avatar, None, None,
            )
        };

        assert!(
            verify(&from_hand, Zone::Stack),
            "resolving a spell cast from hand must satisfy the intervening if"
        );
        assert!(
            !verify(&from_graveyard, Zone::Stack),
            "a hand cast must not satisfy a graveyard-cast condition"
        );
        assert!(
            !verify(&from_hand, Zone::Graveyard),
            "entering without resolving from the stack is not casting it"
        );
    }

    #[test]
    fn triggering_spell_mana_spent_condition_uses_spell_cast_snapshot() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let spell = game.new_object_id();
        let mut snapshot = crate::snapshot::ObjectSnapshot::for_testing(spell, alice, "Big Spell");
        snapshot.mana_spent_to_cast = ManaPool {
            red: 2,
            colorless: 2,
            ..ManaPool::default()
        };
        let event = RawEvent::new(
            crate::events::spells::SpellCastEvent::new_with_snapshot(
                spell,
                alice,
                Zone::Hand,
                snapshot,
            ),
            ProvNodeId::default(),
        );
        let ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);

        assert!(
            evaluate_condition(
                &game,
                &Condition::TriggeringSpellManaSpentToCastAtLeast {
                    amount: 4,
                    symbol: None,
                },
                &ctx,
            )
            .expect("triggering spell total mana condition should evaluate")
        );
        assert!(
            !evaluate_condition(
                &game,
                &Condition::Not(Box::new(
                    Condition::TriggeringSpellColoredManaSpentToCastAtLeast(1)
                )),
                &ctx,
            )
            .expect("triggering spell colored mana condition should evaluate")
        );
    }

    #[test]
    fn resolution_triggering_tag_conditions_use_the_trigger_event_without_seeded_tags() {
        for (subtype, expected) in [(Subtype::Plains, true), (Subtype::Island, false)] {
            let mut game = GameState::new(vec!["Alice".to_string()], 20);
            let alice = game.players[0].id;
            let source = game.new_object_id();
            let land_card = CardBuilder::new(CardId::from_raw(20), "Triggering Land")
                .card_types(vec![CardType::Land])
                .subtypes(vec![subtype])
                .build();
            let land = game.create_object_from_card(&land_card, alice, Zone::Battlefield);
            let snapshot = crate::snapshot::ObjectSnapshot::from_object(
                game.object(land).expect("triggering land should exist"),
                &game,
            );
            let mut change = crate::events::ZoneChangeEvent::with_cause(
                land,
                Zone::Hand,
                Zone::Battlefield,
                EventCause::effect(),
                Some(snapshot.clone()),
            );
            change.destination_snapshots = vec![snapshot];
            let event = TriggerEvent::new_with_provenance(change, ProvNodeId::default());
            let filter = crate::target::ObjectFilter::default().with_subtype(Subtype::Plains);
            let mut ctx = ExecutionContext::new_default(source, alice);
            // Stack construction normally seeds both fields, but wrapper and
            // self-replacement paths may preserve only the triggering event.
            // Resolution must agree with the external condition evaluator.
            ctx.triggering_event = Some(event);

            for condition in [
                Condition::TaggedObjectMatches(crate::TagKey::from("triggering"), filter.clone()),
                Condition::TaggedObjectMatchedLastKnown(
                    crate::TagKey::from("triggering"),
                    filter.clone(),
                ),
            ] {
                assert_eq!(
                    evaluate_condition(&game, &condition, &ctx)
                        .expect("triggering-object condition should evaluate"),
                    expected,
                    "{subtype:?} should have the same result for current and LKI triggering tags",
                );
            }
        }
    }

    #[test]
    fn graveyard_cast_or_ability_activation_history_uses_actor_and_origin_zone() {
        let cast_condition = Condition::TurnHistory(
            ironsmith_core::TurnHistoryCondition::PlayerCastSpellFromZoneThisTurn {
                player: PlayerFilter::You,
                zone: Zone::Graveyard,
            },
        );
        let activation_condition = Condition::TurnHistory(
            ironsmith_core::TurnHistoryCondition::PlayerActivatedAbilityOfCardInZoneThisTurn {
                player: PlayerFilter::You,
                zone: Zone::Graveyard,
            },
        );
        let combined = Condition::Or(
            Box::new(cast_condition.clone()),
            Box::new(activation_condition.clone()),
        );

        let mut cast_game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = cast_game.players[0].id;
        let bob = cast_game.players[1].id;
        let source = cast_game.new_object_id();
        let cast_ctx = ExecutionContext::new_default(source, alice);
        assert!(
            !evaluate_condition(&cast_game, &combined, &cast_ctx).unwrap(),
            "the disjunction must be false before either history event"
        );

        let hand_spell = cast_game.new_object_id();
        let hand_cast = RawEvent::new(
            crate::events::SpellCastEvent::new(hand_spell, alice, Zone::Hand),
            ProvNodeId::default(),
        );
        cast_game
            .turn_store
            .turn_history
            .record_event(&hand_cast, None, None);
        let opponents_graveyard_spell = cast_game.new_object_id();
        let opponents_graveyard_cast = RawEvent::new(
            crate::events::SpellCastEvent::new(opponents_graveyard_spell, bob, Zone::Graveyard),
            ProvNodeId::default(),
        );
        cast_game
            .turn_store
            .turn_history
            .record_event(&opponents_graveyard_cast, None, None);
        assert!(
            !evaluate_condition(&cast_game, &combined, &cast_ctx).unwrap(),
            "a hand cast and an opponent's graveyard cast must not satisfy your condition"
        );

        let graveyard_spell = cast_game.new_object_id();
        let graveyard_cast = RawEvent::new(
            crate::events::SpellCastEvent::new(graveyard_spell, alice, Zone::Graveyard),
            ProvNodeId::default(),
        );
        cast_game
            .turn_store
            .turn_history
            .record_event(&graveyard_cast, None, None);
        assert!(
            evaluate_condition(&cast_game, &cast_condition, &cast_ctx).unwrap()
                && evaluate_condition(&cast_game, &combined, &cast_ctx).unwrap(),
            "your graveyard cast must satisfy the cast branch and the disjunction"
        );

        let mut activation_game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = activation_game.players[0].id;
        let bob = activation_game.players[1].id;
        let source = activation_game.new_object_id();
        let activation_ctx = ExecutionContext::new_default(source, alice);

        for (activator, zone) in [(alice, Zone::Battlefield), (bob, Zone::Graveyard)] {
            let ability_source = activation_game.new_object_id();
            let mut snapshot = crate::snapshot::ObjectSnapshot::for_testing(
                ability_source,
                activator,
                "Ability Source",
            );
            snapshot.zone = zone;
            let activation = RawEvent::new(
                crate::events::AbilityActivatedEvent::new(ability_source, activator, false)
                    .with_snapshot(Some(snapshot.clone())),
                ProvNodeId::default(),
            );
            activation_game
                .turn_store
                .turn_history
                .record_event(&activation, Some(snapshot), None);
        }
        assert!(
            !evaluate_condition(&activation_game, &combined, &activation_ctx).unwrap(),
            "your battlefield activation and an opponent's graveyard activation must not qualify"
        );

        let graveyard_source = activation_game.new_object_id();
        let mut graveyard_snapshot = crate::snapshot::ObjectSnapshot::for_testing(
            graveyard_source,
            alice,
            "Graveyard Ability Source",
        );
        graveyard_snapshot.zone = Zone::Graveyard;
        let graveyard_activation = RawEvent::new(
            crate::events::AbilityActivatedEvent::new(graveyard_source, alice, false)
                .with_snapshot(Some(graveyard_snapshot.clone())),
            ProvNodeId::default(),
        );
        activation_game.turn_store.turn_history.record_event(
            &graveyard_activation,
            Some(graveyard_snapshot),
            None,
        );
        assert!(
            evaluate_condition(&activation_game, &activation_condition, &activation_ctx).unwrap()
                && evaluate_condition(&activation_game, &combined, &activation_ctx).unwrap(),
            "your graveyard-card activation must satisfy the activation branch and disjunction"
        );
    }

    #[test]
    fn visited_attraction_history_is_typed_and_player_scoped() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);
        let condition = Condition::TurnHistory(
            ironsmith_core::TurnHistoryCondition::PlayerVisitedAttractionThisTurn(
                PlayerFilter::You,
            ),
        );

        let bobs_visit = RawEvent::new(
            crate::events::KeywordActionEvent::new(
                crate::events::KeywordActionKind::VisitAttraction,
                bob,
                source,
                1,
            ),
            ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&bobs_visit, None, None);
        assert!(
            !evaluate_condition(&game, &condition, &ctx).unwrap(),
            "another player's visit must not satisfy your visit history"
        );

        let alices_open = RawEvent::new(
            crate::events::KeywordActionEvent::new(
                crate::events::KeywordActionKind::OpenAttraction,
                alice,
                source,
                1,
            ),
            ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&alices_open, None, None);
        assert!(
            !evaluate_condition(&game, &condition, &ctx).unwrap(),
            "opening an Attraction must not be treated as visiting one"
        );

        let alices_visit = RawEvent::new(
            crate::events::KeywordActionEvent::new(
                crate::events::KeywordActionKind::VisitAttraction,
                alice,
                source,
                1,
            ),
            ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&alices_visit, None, None);
        assert!(
            evaluate_condition(&game, &condition, &ctx).unwrap(),
            "your typed visit action must satisfy the condition"
        );

        game.turn_store.turn_history.clear_for_new_turn();
        assert!(
            !evaluate_condition(&game, &condition, &ctx).unwrap(),
            "the visit condition must reset with ordinary turn history"
        );
    }

    #[test]
    fn evaluate_player_has_more_cards_in_hand_than_each_other_player_requires_unique_leader() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let condition = Condition::PlayerHasMoreCardsInHandThanEachOtherPlayer {
            player: PlayerFilter::Any,
        };

        add_hand_card(&mut game, 1, "Mountain", 0);
        add_hand_card(&mut game, 2, "Island", 1);
        add_hand_card(&mut game, 3, "Forest", 1);

        let ctx = ExecutionContext::new_default(source, alice);
        assert!(
            evaluate_condition(&game, &condition, &ctx)
                .expect("unique hand-size leader should evaluate"),
            "expected Bob to satisfy the unique-leader condition"
        );

        add_hand_card(&mut game, 4, "Plains", 0);
        assert!(
            !evaluate_condition(&game, &condition, &ctx).expect("ties should evaluate cleanly"),
            "expected tie for most cards in hand to fail the condition"
        );
    }

    #[test]
    fn evaluate_player_has_more_life_than_each_other_player_requires_unique_leader() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let condition = Condition::PlayerHasMoreLifeThanEachOtherPlayer {
            player: PlayerFilter::Any,
        };

        game.players[1].life = 21;
        let ctx = ExecutionContext::new_default(source, alice);
        assert!(
            evaluate_condition(&game, &condition, &ctx)
                .expect("unique life leader should evaluate"),
            "expected Bob to satisfy the unique-leader life condition"
        );

        game.players[0].life = 21;
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("tied life totals should evaluate cleanly"),
            "expected tie for most life to fail the condition"
        );
    }

    #[test]
    fn unique_control_leader_quantifies_candidates_in_all_contexts() {
        for counts in [[1, 3, 2], [3, 1, 2], [2, 2, 1], [0, 0, 0]] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            let alice = game.players[0].id;
            let source = game.new_object_id();
            for (player, count) in counts.into_iter().enumerate() {
                for n in 0..count {
                    add_battlefield_land(&mut game, 100 + player as u32 * 10 + n, "Land", player);
                }
            }
            let max = *counts.iter().max().unwrap();
            let unique = counts.iter().filter(|n| **n == max).count() == 1;
            for player in [PlayerFilter::Any, PlayerFilter::You, PlayerFilter::Opponent] {
                let expected = unique
                    && match player {
                        PlayerFilter::You => counts[0] == max,
                        PlayerFilter::Opponent => counts[0] != max,
                        _ => true,
                    };
                let condition = Condition::PlayerControlsMoreThanEachOtherPlayer {
                    player,
                    filter: crate::target::ObjectFilter::land(),
                };
                let external = ExternalEvaluationContext {
                    controller: alice,
                    source,
                    ..Default::default()
                };
                let execution = ExecutionContext::new_default(source, alice);
                assert_eq!(
                    evaluate_condition_external(&game, &condition, &external),
                    expected,
                    "external {counts:?} {condition:?}"
                );
                assert_eq!(
                    evaluate_condition_cast_time(&game, &condition, alice, source),
                    expected,
                    "cast {counts:?} {condition:?}"
                );
                assert_eq!(
                    evaluate_condition(&game, &condition, &execution).unwrap(),
                    expected,
                    "execution {counts:?} {condition:?}"
                );
            }
        }
    }

    #[test]
    fn evaluate_player_controls_more_than_each_other_player_requires_unique_leader() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let condition = Condition::PlayerControlsMoreThanEachOtherPlayer {
            player: PlayerFilter::You,
            filter: crate::target::ObjectFilter::land(),
        };

        add_battlefield_land(&mut game, 5, "Plains", 0);
        add_battlefield_land(&mut game, 6, "Island", 1);
        add_battlefield_land(&mut game, 7, "Swamp", 1);

        let ctx = ExecutionContext::new_default(source, alice);
        assert!(
            !evaluate_condition(&game, &condition, &ctx).expect("lower land count should evaluate"),
            "expected lower land count to fail the unique-leader condition"
        );

        add_battlefield_land(&mut game, 8, "Mountain", 0);
        add_battlefield_land(&mut game, 9, "Forest", 0);
        assert!(
            evaluate_condition(&game, &condition, &ctx)
                .expect("unique land leader should evaluate"),
            "expected strict land-count leader to satisfy the condition"
        );

        add_battlefield_land(&mut game, 10, "Wastes", 1);
        assert!(
            !evaluate_condition(&game, &condition, &ctx).expect("ties should evaluate cleanly"),
            "expected tie for most lands to fail the condition"
        );
    }

    #[test]
    fn player_controls_global_greatest_power_uses_all_battlefield_creatures_as_domain() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let alice_creature = CardBuilder::new(CardId::from_raw(71_561), "Alice Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let bob_creature = CardBuilder::new(CardId::from_raw(71_562), "Bob Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(5, 5))
            .build();
        let source = game.create_object_from_card(&alice_creature, alice, Zone::Battlefield);
        game.create_object_from_card(&bob_creature, bob, Zone::Battlefield);

        let global_creatures = crate::target::ObjectFilter::creature().in_zone(Zone::Battlefield);
        let mut controlled_greatest = global_creatures.clone().controlled_by(PlayerFilter::You);
        controlled_greatest.power = Some(crate::filter::Comparison::EqualExpr(Box::new(
            Value::GreatestPower(global_creatures),
        )));
        let condition = Condition::PlayerControls {
            player: PlayerFilter::You,
            filter: controlled_greatest,
        };
        let ctx = ExecutionContext::new_default(source, alice);

        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("global greatest-power condition should evaluate"),
            "an opponent's stronger creature must make the condition false"
        );

        let tied_creature = CardBuilder::new(CardId::from_raw(71_563), "Tied Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(5, 5))
            .build();
        game.create_object_from_card(&tied_creature, alice, Zone::Battlefield);
        assert!(
            evaluate_condition(&game, &condition, &ctx)
                .expect("tied global greatest-power condition should evaluate"),
            "controlling one creature tied for greatest power must satisfy the condition"
        );
    }

    #[test]
    fn frodo_ring_bearer_threshold_condition_requires_bearer_and_two_temptations() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let frodo = CardBuilder::new(CardId::from_raw(71_571), "Frodo, Adventurous Hobbit")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 3))
            .build();
        let other_creature = CardBuilder::new(CardId::from_raw(71_572), "Other Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source = game.create_object_from_card(&frodo, alice, Zone::Battlefield);
        let other_source = game.create_object_from_card(&other_creature, alice, Zone::Battlefield);
        let condition = Condition::And(
            Box::new(Condition::SourceIsRingBearer {
                player: PlayerFilter::You,
            }),
            Box::new(Condition::PlayerRingTemptedThisGameOrMore {
                player: PlayerFilter::You,
                count: 2,
            }),
        );
        let ctx = ExecutionContext::new_default(source, alice);

        game.set_ring_bearer(alice, source);
        game.increment_ring_temptations(bob);
        game.increment_ring_temptations(bob);
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("opponent temptations should evaluate cleanly"),
            "Frodo's draw gate must use its controller's Ring temptation count"
        );

        game.increment_ring_temptations(alice);
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("one temptation should evaluate cleanly"),
            "Frodo's draw gate must stay false before the Ring has tempted you twice"
        );

        game.increment_ring_temptations(alice);
        assert!(
            evaluate_condition(&game, &condition, &ctx)
                .expect("two temptations should evaluate cleanly"),
            "Frodo's draw gate should be true once he is your Ring-bearer and you have two temptations"
        );

        game.set_ring_bearer(alice, other_source);
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("different Ring-bearer should evaluate cleanly"),
            "Frodo's draw gate must require the source itself to be your Ring-bearer"
        );

        game.clear_ring_bearer(alice);
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("missing Ring-bearer should evaluate cleanly"),
            "Frodo's draw gate must stay false when the source is no longer your Ring-bearer"
        );
    }

    #[test]
    fn evaluate_player_has_no_opponent_with_more_life_than_allows_ties() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let condition = Condition::PlayerHasNoOpponentWithMoreLifeThan {
            player: PlayerFilter::Specific(alice),
        };

        let ctx = ExecutionContext::new_default(source, alice);
        assert!(
            evaluate_condition(&game, &condition, &ctx).expect("tied life totals should evaluate"),
            "expected tied life totals to satisfy the no-opponent-has-more-life condition"
        );

        game.players[1].life = 21;
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("higher life total should evaluate cleanly"),
            "expected an opposing higher life total to fail the condition"
        );
    }

    #[test]
    fn evaluate_object_put_into_graveyard_from_battlefield_condition_uses_lki_controller() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.new_object_id();
        let land = CardBuilder::new(CardId::from_raw(9), "Test Land")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
        let snapshot = {
            let object = game.object(land_id).expect("land exists");
            crate::snapshot::ObjectSnapshot::from_object(object, &game)
        };
        let zone_change = crate::events::RawEvent::new(
            crate::events::ZoneChangeEvent::with_cause(
                land_id,
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::effect(),
                Some(snapshot.clone()),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&zone_change, Some(snapshot), None);

        let condition = Condition::ObjectPutIntoGraveyardFromBattlefieldThisTurn(
            crate::target::ObjectFilter::land().controlled_by(PlayerFilter::You),
        );

        assert!(
            evaluate_condition(
                &game,
                &condition,
                &ExecutionContext::new_default(source, alice)
            )
            .expect("land-graveyard condition should evaluate"),
            "expected Alice's historical land to satisfy the condition"
        );
        assert!(
            !evaluate_condition(
                &game,
                &condition,
                &ExecutionContext::new_default(source, bob)
            )
            .expect("land-graveyard condition should evaluate"),
            "expected Bob not to satisfy Alice's historical land condition"
        );
    }

    #[test]
    fn descended_this_turn_uses_permanent_card_lki_and_graveyard_owner() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.new_object_id();
        let condition = Condition::PlayerDescendedThisTurn {
            player: PlayerFilter::You,
        };

        let instant = CardBuilder::new(CardId::from_raw(91), "Test Instant")
            .card_types(vec![CardType::Instant])
            .build();
        let instant_id = game.create_object_from_card(&instant, alice, Zone::Hand);
        let instant_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(instant_id).expect("instant exists"),
            &game,
        );
        let instant_event = crate::events::RawEvent::new(
            crate::events::ZoneChangeEvent::with_cause(
                instant_id,
                Zone::Hand,
                Zone::Graveyard,
                EventCause::effect(),
                Some(instant_snapshot.clone()),
            ),
            ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&instant_event, Some(instant_snapshot), None);

        let token_id = game.new_object_id();
        let mut token_snapshot =
            crate::snapshot::ObjectSnapshot::for_testing(token_id, alice, "Test Creature Token")
                .with_card_types(vec![CardType::Creature]);
        token_snapshot.is_token = true;
        let token_event = crate::events::RawEvent::new(
            crate::events::ZoneChangeEvent::with_cause(
                token_id,
                Zone::Battlefield,
                Zone::Graveyard,
                EventCause::effect(),
                Some(token_snapshot.clone()),
            ),
            ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&token_event, Some(token_snapshot), None);

        assert!(
            !evaluate_condition(
                &game,
                &condition,
                &ExecutionContext::new_default(source, alice),
            )
            .expect("nonpermanent cards and tokens should evaluate cleanly"),
            "an instant card and a creature token must not count as descending"
        );

        let bob_creature = CardBuilder::new(CardId::from_raw(92), "Bob's Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let bob_creature_id = game.create_object_from_card(&bob_creature, bob, Zone::Library);
        let bob_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(bob_creature_id).expect("Bob's creature exists"),
            &game,
        );
        let bob_event = crate::events::RawEvent::new(
            crate::events::ZoneChangeEvent::with_cause(
                bob_creature_id,
                Zone::Library,
                Zone::Graveyard,
                EventCause::effect(),
                Some(bob_snapshot.clone()),
            ),
            ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&bob_event, Some(bob_snapshot), None);

        assert!(
            !evaluate_condition(
                &game,
                &condition,
                &ExecutionContext::new_default(source, alice),
            )
            .expect("Alice's descend condition should evaluate cleanly"),
            "a permanent card put into Bob's graveyard must not make Alice descend"
        );
        assert!(
            evaluate_condition(
                &game,
                &condition,
                &ExecutionContext::new_default(source, bob),
            )
            .expect("Bob's descend condition should evaluate cleanly"),
            "a permanent card put into Bob's graveyard should make Bob descend"
        );

        let alice_land = CardBuilder::new(CardId::from_raw(93), "Alice's Land")
            .card_types(vec![CardType::Land])
            .build();
        let alice_land_id = game.create_object_from_card(&alice_land, alice, Zone::Hand);
        let alice_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(alice_land_id).expect("Alice's land exists"),
            &game,
        );
        let alice_event = crate::events::RawEvent::new(
            crate::events::ZoneChangeEvent::with_cause(
                alice_land_id,
                Zone::Hand,
                Zone::Graveyard,
                EventCause::effect(),
                Some(alice_snapshot.clone()),
            ),
            ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&alice_event, Some(alice_snapshot), None);

        assert!(
            evaluate_condition(
                &game,
                &condition,
                &ExecutionContext::new_default(source, alice),
            )
            .expect("Alice's descend condition should evaluate cleanly"),
            "a permanent card put into Alice's graveyard from hand should make Alice descend"
        );
    }

    #[test]
    fn evaluate_object_entered_battlefield_condition_uses_lki_controller() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.new_object_id();
        let artifact = CardBuilder::new(CardId::from_raw(10), "Test Artifact")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact_id = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        let snapshot = {
            let object = game.object(artifact_id).expect("artifact exists");
            crate::snapshot::ObjectSnapshot::from_object(object, &game)
        };
        let etb = crate::events::RawEvent::new(
            crate::events::EnterBattlefieldEvent::new(artifact_id, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&etb, Some(snapshot), None);

        let condition = Condition::ObjectEnteredBattlefieldThisTurn(
            crate::target::ObjectFilter::artifact().controlled_by(PlayerFilter::You),
        );

        assert!(
            evaluate_condition(
                &game,
                &condition,
                &ExecutionContext::new_default(source, alice)
            )
            .expect("artifact-entered condition should evaluate"),
            "expected Alice's historical artifact ETB to satisfy the condition"
        );
        assert!(
            !evaluate_condition(
                &game,
                &condition,
                &ExecutionContext::new_default(source, bob)
            )
            .expect("artifact-entered condition should evaluate"),
            "expected Bob not to satisfy Alice's historical artifact condition"
        );
    }

    #[test]
    fn evaluate_external_target_matches_uses_triggering_snapshot_and_source_power() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source_card = CardBuilder::new(CardId::from_raw(11), "Small Watcher")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let larger_card = CardBuilder::new(CardId::from_raw(12), "Departing Giant")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(5, 5))
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let departing = game.create_object_from_card(&larger_card, alice, Zone::Battlefield);
        let snapshot = {
            let object = game.object(departing).expect("departing creature exists");
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, &game,
            )
        };
        game.move_object_by_effect(departing, Zone::Graveyard);
        let event = TriggerEvent::new_with_provenance(
            crate::events::ZoneChangeEvent::with_cause(
                departing,
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::effect(),
                Some(snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let condition = Condition::TargetMatches(crate::target::ObjectFilter {
            card_types: vec![CardType::Creature],
            power: Some(crate::target::Comparison::GreaterThanExpr(Box::new(
                Value::PowerOf(Box::new(crate::target::ChooseSpec::Source)),
            ))),
            ..crate::target::ObjectFilter::default()
        });
        let ctx = ExternalEvaluationContext {
            controller: alice,
            source,
            defending_player: None,
            attacking_player: None,
            filter_source: None,
            iterated_player: None,
            triggering_event: Some(&event),
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };

        assert!(
            evaluate_condition_external(&game, &condition, &ctx),
            "trigger-time target condition should compare the LKI creature to the source's power"
        );
    }

    #[test]
    fn last_known_tagged_match_never_falls_back_to_current_characteristics() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = game.players[0].id;
        let creature_card = CardBuilder::new(CardId::from_raw(91), "Changed Object")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let object = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
        let mut noncreature_snapshot = {
            let object = game.object(object).expect("object exists");
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, &game,
            )
        };
        noncreature_snapshot.card_types.clear();
        noncreature_snapshot.power = None;
        noncreature_snapshot.toughness = None;

        let condition = Condition::TaggedObjectMatchedLastKnown(
            crate::TagKey::from("triggering"),
            crate::target::ObjectFilter::creature(),
        );

        let mut effect_ctx = ExecutionContext::new_default(object, alice);
        effect_ctx.set_tagged_objects("triggering", vec![noncreature_snapshot.clone()]);
        assert!(
            !evaluate_condition(&game, &condition, &effect_ctx)
                .expect("last-known body condition should evaluate"),
            "current creature characteristics must not override a noncreature snapshot"
        );

        let event = TriggerEvent::new_with_provenance(
            crate::events::ZoneChangeEvent::with_cause(
                object,
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::effect(),
                Some(noncreature_snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let external_ctx = ExternalEvaluationContext {
            controller: alice,
            source: object,
            defending_player: None,
            attacking_player: None,
            filter_source: None,
            iterated_player: None,
            triggering_event: Some(&event),
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };
        assert!(
            !evaluate_condition_external(&game, &condition, &external_ctx),
            "trigger-time LKI condition must not inspect the current creature"
        );

        let creature_snapshot = {
            let object = game.object(object).expect("object still exists");
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, &game,
            )
        };
        effect_ctx.set_tagged_objects("triggering", vec![creature_snapshot]);
        assert!(
            evaluate_condition(&game, &condition, &effect_ctx)
                .expect("matching last-known body condition should evaluate")
        );
    }

    #[test]
    fn player_tagged_last_known_match_uses_snapshot_controller_even_if_current_object_exists() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let card = CardBuilder::new(CardId::from_raw(92), "Changed Controller")
            .card_types(vec![CardType::Artifact])
            .build();
        let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(object).expect("object exists"),
            &game,
        );
        game.set_current_controller(object, bob)
            .expect("finite controller fixture must refresh successfully");
        assert_eq!(game.controller_of_id(object), Some(bob));

        let mut effect_ctx = ExecutionContext::new_default(object, alice);
        effect_ctx.set_tagged_objects("returned_0", vec![snapshot]);
        let last_known = Condition::PlayerTaggedObjectMatches {
            player: PlayerFilter::You,
            tag: crate::TagKey::from("returned_0"),
            filter: crate::target::ObjectFilter::default(),
            mode: crate::effect::TaggedObjectMatchMode::LastKnown,
        };
        assert!(
            evaluate_condition(&game, &last_known, &effect_ctx)
                .expect("last-known player predicate should evaluate"),
            "Alice controlled the captured object even though Bob controls its current form"
        );

        let current = Condition::PlayerTaggedObjectMatches {
            player: PlayerFilter::You,
            tag: crate::TagKey::from("returned_0"),
            filter: crate::target::ObjectFilter::default(),
            mode: crate::effect::TaggedObjectMatchMode::CurrentOrLastKnown,
        };
        assert!(
            !evaluate_condition(&game, &current, &effect_ctx)
                .expect("current player predicate should evaluate"),
            "ordinary this-way predicates must continue to prefer the current object"
        );
    }

    #[test]
    fn wave_of_rats_condition_true_when_source_dealt_combat_damage_to_player_this_turn() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let rat = CardBuilder::new(CardId::from_raw(13), "Wave of Rats")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 2))
            .build();
        let rat_id = game.create_object_from_card(&rat, alice, Zone::Battlefield);
        let source_snapshot = {
            let object = game.object(rat_id).expect("Wave of Rats exists");
            crate::snapshot::ObjectSnapshot::from_object(object, &game)
        };
        let damage = RawEvent::new(
            DamageEvent::with_cause(
                rat_id,
                DamageTarget::Player(bob),
                4,
                true,
                EventCause::effect(),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&damage, None, Some(source_snapshot));

        let ctx = ExecutionContext::new_default(rat_id, alice);
        assert!(
            evaluate_condition(
                &game,
                &Condition::SourceDealtCombatDamageToPlayerThisTurn,
                &ctx
            )
            .expect("source combat-damage condition should evaluate"),
            "expected Wave of Rats condition to pass after combat damage to a player"
        );
    }

    #[test]
    fn wave_of_rats_condition_false_without_combat_damage_to_player_this_turn() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);
        assert!(
            !evaluate_condition(
                &game,
                &Condition::SourceDealtCombatDamageToPlayerThisTurn,
                &ctx
            )
            .expect("source combat-damage condition should evaluate"),
            "expected Wave of Rats condition to fail without combat damage"
        );
    }

    #[test]
    fn first_combat_phase_condition_requires_started_first_combat() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);
        let condition = Condition::FirstCombatPhaseOfTurn;

        game.turn.phase = crate::game_state::Phase::Combat;
        game.turn_store.combat_phases_started_this_turn = 0;
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("first combat condition should evaluate"),
            "combat phase without a started combat count should not pass"
        );

        game.turn_store.combat_phases_started_this_turn = 1;
        assert!(
            evaluate_condition(&game, &condition, &ctx)
                .expect("first combat condition should evaluate"),
            "first started combat phase should pass"
        );

        game.turn_store.combat_phases_started_this_turn = 2;
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("first combat condition should evaluate"),
            "later combat phases should not pass"
        );
    }

    #[test]
    fn target_is_attacking_uses_combat_membership_not_tapped_status() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source =
            add_battlefield_permanent(&mut game, 120, "Source", 0, CardType::Creature, None);
        let untapped_attacker = add_battlefield_permanent(
            &mut game,
            121,
            "Vigilant Attacker",
            0,
            CardType::Creature,
            None,
        );
        let tapped_nonattacker = add_battlefield_permanent(
            &mut game,
            122,
            "Tapped Nonattacker",
            0,
            CardType::Creature,
            None,
        );
        game.tap(tapped_nonattacker);
        game.combat = Some(crate::combat_state::CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: untapped_attacker,
                target: crate::combat_state::AttackTarget::Player(bob),
            }],
            ..crate::combat_state::CombatState::default()
        });

        let attacking_ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            crate::effects::ResolvedTarget::Object(untapped_attacker),
        ]);
        let nonattacking_ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            crate::effects::ResolvedTarget::Object(tapped_nonattacker),
        ]);

        assert!(
            evaluate_condition(&game, &Condition::TargetIsAttacking, &attacking_ctx)
                .expect("attacking condition should evaluate"),
            "an untapped vigilant creature remains attacking"
        );
        assert!(
            !evaluate_condition(&game, &Condition::TargetIsAttacking, &nonattacking_ctx)
                .expect("nonattacking condition should evaluate"),
            "a tapped creature outside combat is not attacking"
        );
    }
}

fn player_has_card_in_hand_matching(
    game: &GameState,
    player: PlayerId,
    filter: &crate::target::ObjectFilter,
    filter_source: Option<ObjectId>,
) -> bool {
    let filter_ctx = game.filter_context_for(player, filter_source);
    game.player(player).is_some_and(|state| {
        state.hand.iter().any(|&card_id| {
            game.object(card_id)
                .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
        })
    })
}

fn player_life_compares_to_half_starting(
    game: &GameState,
    player: PlayerId,
    inclusive: bool,
) -> bool {
    game.player(player).is_some_and(|state| {
        let doubled_life = i64::from(state.life) * 2;
        if inclusive {
            doubled_life <= i64::from(state.starting_life)
        } else {
            doubled_life < i64::from(state.starting_life)
        }
    })
}

/// A comparison need not materialize its arithmetic as an i32 effect amount.
/// Widen the existing arithmetic nodes before comparing (for example a life
/// difference spanning MIN..MAX, or starting life + 10). Leaves retain their
/// ordinary typed resolver and all context/choice errors. This is bounded
/// arithmetic, not an unbounded Value or gameplay-count representation.
fn resolve_comparison_operand(
    game: &GameState,
    value: &Value,
    ctx: &ExecutionContext,
) -> Result<i64, ExecutionError> {
    let overflow = || {
        ExecutionError::UnresolvableValue(
            "comparison arithmetic is outside the supported integer range".into(),
        )
    };
    match value.unhinted() {
        Value::DamageHistory(query) => {
            crate::effects::helpers::resolve_damage_history_for_comparison(game, query, ctx)
        }

        Value::Add(left, right) => resolve_comparison_operand(game, left, ctx)?
            .checked_add(resolve_comparison_operand(game, right, ctx)?)
            .ok_or_else(overflow),
        Value::Scaled(inner, multiplier) => resolve_comparison_operand(game, inner, ctx)?
            .checked_mul(i64::from(*multiplier))
            .ok_or_else(overflow),
        Value::Min(left, right) => Ok(resolve_comparison_operand(game, left, ctx)?
            .min(resolve_comparison_operand(game, right, ctx)?)),
        Value::HalfRoundedDown(inner) => {
            Ok(resolve_comparison_operand(game, inner, ctx)?.div_euclid(2))
        }
        Value::DividedRoundedDown(inner, divisor) if *divisor != 0 => {
            resolve_comparison_operand(game, inner, ctx)?
                .checked_div_euclid(i64::from(*divisor))
                .ok_or_else(overflow)
        }
        _ => resolve_value(game, value, ctx).map(i64::from),
    }
}

fn compare_resolved_values(
    game: &GameState,
    left: &Value,
    operator: crate::effect::ValueComparisonOperator,
    right: &Value,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    use crate::effect::ValueComparisonOperator as Op;
    let left = resolve_comparison_operand(game, left, ctx)?;
    let right = resolve_comparison_operand(game, right, ctx)?;
    Ok(match operator {
        Op::GreaterThan => left > right,
        Op::GreaterThanOrEqual => left >= right,
        Op::Equal => left == right,
        Op::LessThan => left < right,
        Op::LessThanOrEqual => left <= right,
        Op::NotEqual => left != right,
    })
}

fn evaluate_value_comparison(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    left: &Value,
    operator: crate::effect::ValueComparisonOperator,
    right: &Value,
    triggering_event: Option<&TriggerEvent>,
    defending_player: Option<PlayerId>,
    attacking_player: Option<PlayerId>,
    iterated_player: Option<PlayerId>,
    ability_identity: (Option<crate::triggers::TriggerIdentity>, Option<usize>),
) -> bool {
    let mut ctx = ExecutionContext::new_default(source, controller);
    ctx.iteration.iterated_player = iterated_player;
    // "If you haven't added mana with this ability this turn": the ability's
    // own resolution count needs its identity outside resolution too.
    ctx.trigger_identity = ability_identity.0;
    ctx.ability_index = ability_identity.1;
    if let Some(attached) = game
        .object(source)
        .and_then(|source| source.attached_to.as_ref())
        .and_then(|target| target.object_id())
        .and_then(|id| game.object(id))
    {
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(attached, game);
        for tag in ["enchanted", "equipped"] {
            ctx.set_tagged_objects(tag, vec![snapshot.clone()]);
        }
    }
    if let Some(event) = triggering_event {
        ctx = ctx.with_triggering_event(event.clone());
        if let Some(snapshot) = event.snapshot() {
            ctx.set_tagged_objects("triggering", vec![snapshot.clone()]);
        }
        if let Some(cast) = event.downcast::<crate::events::SpellCastEvent>()
            && let Some(spell) = game.object(cast.spell)
            && let Some(snapshots) = spell
                .cast_tagged_objects
                .get(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG)
        {
            ctx.set_tagged_objects(
                ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG,
                snapshots.clone(),
            );
        }
    }
    // Explicit declaration context takes precedence over any event-derived
    // combat context; ordinary callers leave these fields unspecified.
    if let Some(player) = defending_player {
        ctx.combat.defending_player = Some(player);
    }
    if let Some(player) = attacking_player {
        ctx.combat.attacking_player = Some(player);
    }
    let source_exiled = game
        .get_exiled_with_source_links(source)
        .iter()
        .filter_map(|id| {
            game.object(*id).map(|obj| {
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    obj, game,
                )
            })
        })
        .collect::<Vec<_>>();
    if !source_exiled.is_empty() {
        ctx.set_tagged_objects(crate::tag::SOURCE_EXILED_TAG, source_exiled);
    }
    let compare = |exec: &ExecutionContext| -> Result<bool, ExecutionError> {
        compare_resolved_values(game, left, operator, right, exec)
    };
    match compare(&ctx) {
        Ok(result) => result,
        // "as long as an opponent has 10 or less life": a quantified opponent
        // in a static/trigger condition is satisfied by any opponent.
        Err(ExecutionError::UnresolvableValue(message))
            if message == crate::effects::helpers::AN_OPPONENT_CHOICE_REQUIRED =>
        {
            crate::effects::helpers::an_opponent_choice_candidates(game, &ctx)
                .into_iter()
                .any(|opponent| {
                    let probe = an_opponent_probe_context(&ctx, opponent);
                    matches!(compare(&probe), Ok(true))
                })
        }
        Err(_) => false,
    }
}

fn evaluate_value_is_prime(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    value: &Value,
    triggering_event: Option<&TriggerEvent>,
) -> bool {
    let mut ctx = ExecutionContext::new_default(source, controller);
    if let Some(event) = triggering_event {
        ctx = ctx.with_triggering_event(event.clone());
        if let Some(snapshot) = event.snapshot() {
            ctx.set_tagged_objects("triggering", vec![snapshot.clone()]);
        }
        if let Some(cast) = event.downcast::<crate::events::SpellCastEvent>()
            && let Some(spell) = game.object(cast.spell)
            && let Some(snapshots) = spell
                .cast_tagged_objects
                .get(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG)
        {
            ctx.set_tagged_objects(
                ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG,
                snapshots.clone(),
            );
        }
    }
    let source_exiled = game
        .get_exiled_with_source_links(source)
        .iter()
        .filter_map(|id| {
            game.object(*id).map(|obj| {
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    obj, game,
                )
            })
        })
        .collect::<Vec<_>>();
    if !source_exiled.is_empty() {
        ctx.set_tagged_objects(crate::tag::SOURCE_EXILED_TAG, source_exiled);
    }
    let Ok(value) = resolve_value(game, value, &ctx) else {
        return false;
    };
    is_prime_integer(value)
}

fn is_prime_integer(value: i32) -> bool {
    if value < 2 {
        return false;
    }
    if value % 2 == 0 {
        return value == 2;
    }
    let mut divisor = 3;
    while divisor <= value / divisor {
        if value % divisor == 0 {
            return false;
        }
        divisor += 2;
    }
    true
}

fn condition_count_for_player(
    game: &GameState,
    source: ObjectId,
    player_filter: &PlayerFilter,
    candidate: PlayerId,
    filter: &crate::target::ObjectFilter,
) -> usize {
    let opponents: Vec<PlayerId> = game
        .players
        .iter()
        .filter(|p| p.id != candidate)
        .map(|p| p.id)
        .collect();
    let mut filter_ctx = crate::filter::FilterContext::new(candidate)
        .with_source(source)
        .with_opponents(opponents);
    if *player_filter == PlayerFilter::IteratedPlayer {
        filter_ctx = filter_ctx.with_iterated_player(Some(candidate));
    }
    condition_objects_for_zone(game, filter.zone)
        .filter(|obj| condition_object_matches_player_zone(game, obj, candidate, filter.zone))
        .filter(|obj| filter.matches(obj, &filter_ctx, game))
        .count()
}

fn any_opponent_controls_more_than_player(
    game: &GameState,
    source: ObjectId,
    player_filter: &PlayerFilter,
    player_id: PlayerId,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let player_count = condition_count_for_player(game, source, player_filter, player_id, filter);
    game.players
        .iter()
        .filter(|p| p.id != player_id && p.is_in_game())
        .any(|opponent| {
            condition_count_for_player(game, source, player_filter, opponent.id, filter)
                > player_count
        })
}

fn any_opponent_has_fewer_than_player(
    game: &GameState,
    source: ObjectId,
    player_filter: &PlayerFilter,
    player_id: PlayerId,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let player_count = condition_count_for_player(game, source, player_filter, player_id, filter);
    game.players
        .iter()
        .filter(|p| p.id != player_id && p.is_in_game())
        .any(|opponent| {
            condition_count_for_player(game, source, player_filter, opponent.id, filter)
                < player_count
        })
}

fn player_controls_more_than_each_other_player(
    game: &GameState,
    source: ObjectId,
    player_filter: &PlayerFilter,
    player_id: PlayerId,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let player_count = condition_count_for_player(game, source, player_filter, player_id, filter);
    game.players
        .iter()
        .filter(|candidate| candidate.is_in_game())
        .all(|candidate| {
            candidate.id == player_id
                || player_count
                    > condition_count_for_player(game, source, player_filter, candidate.id, filter)
        })
}

fn player_has_more_life_than_each_other_player(game: &GameState, player_id: PlayerId) -> bool {
    let Some(life) = game.player(player_id).map(|p| p.life) else {
        return false;
    };
    game.players
        .iter()
        .filter(|candidate| candidate.is_in_game())
        .all(|candidate| candidate.id == player_id || life > candidate.life)
}

fn player_poison_counters_or_more(game: &GameState, player_id: PlayerId, count: u32) -> bool {
    game.player(player_id)
        .map(|player| player.poison_counters >= count)
        .unwrap_or(false)
}

/// "No opponent has more life than <player>": "opponent" is relative to the
/// ability's controller (the only player an unqualified "opponent" can be
/// anchored to), so the controller's own life total and teammates' life
/// totals never falsify the condition.
fn player_has_no_opponent_with_more_life_than(
    game: &GameState,
    controller: PlayerId,
    player_id: PlayerId,
) -> bool {
    let Some(life) = game.player(player_id).map(|p| p.life) else {
        return false;
    };
    game.players
        .iter()
        .filter(|candidate| candidate.is_in_game())
        .filter(|candidate| game.are_opponents(controller, candidate.id))
        .all(|candidate| candidate.id == player_id || life >= candidate.life)
}

fn triggering_event_object_matches(
    game: &GameState,
    ctx: &ExternalEvaluationContext<'_>,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let Some(event) = ctx.triggering_event else {
        return false;
    };
    let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
    if ctx.options.triggering_object_current {
        triggering_event_object_matches_at_resolution(game, event, filter, &filter_ctx)
    } else {
        triggering_event_object_matches_with_filter_context(game, event, filter, &filter_ctx)
    }
}

fn triggering_event_object_matches_with_filter_context(
    game: &GameState,
    event: &TriggerEvent,
    filter: &crate::target::ObjectFilter,
    filter_ctx: &crate::filter::FilterContext,
) -> bool {
    // ETB is a post-transition test. Its ordinary ZoneChange snapshot is
    // origin LKI, so only the exact completed destination receipt can prove
    // entry characteristics (including entry counters and static effects).
    if let Some(change) = event.downcast::<crate::events::ZoneChangeEvent>()
        && change.to == Zone::Battlefield
    {
        return change
            .destination_objects()
            .first()
            .and_then(|id| change.destination_snapshot(*id))
            .is_some_and(|snapshot| filter.matches_snapshot(snapshot, filter_ctx, game));
    }
    // Other event snapshots retain their original event frame, notably LTB.
    if let Some(snapshot) = event.snapshot() {
        return filter.matches_snapshot(snapshot, filter_ctx, game);
    }
    event
        .object_id()
        .and_then(|id| game.object(id))
        .is_some_and(|object| filter.matches(object, filter_ctx, game))
}

fn triggering_event_object_matches_at_resolution(
    game: &GameState,
    event: &TriggerEvent,
    filter: &crate::target::ObjectFilter,
    filter_ctx: &crate::filter::FilterContext,
) -> bool {
    let entry = event
        .downcast::<crate::events::ZoneChangeEvent>()
        .filter(|change| change.to == Zone::Battlefield);
    let id = entry
        .and_then(|change| change.destination_objects().first().copied())
        .or_else(|| event.object_id())
        .or_else(|| event.snapshot().map(|snapshot| snapshot.object_id));
    let Some(id) = id else {
        return false;
    };
    if let Some(object) = game.object(id) {
        // A current predicate rechecks the actual referenced incarnation.
        // Its earlier successful event snapshot cannot override a failed recheck.
        return filter.matches(object, filter_ctx, game);
    }
    if let Some(departure) = game.turn_store.turn_history.source_departure_snapshot(id) {
        return filter.matches_snapshot(departure, filter_ctx, game);
    }
    let snapshot = if let Some(entry) = entry {
        entry.destination_snapshot(id)
    } else {
        event.snapshot()
    };
    snapshot
        .filter(|snapshot| snapshot.object_id == id)
        .is_some_and(|snapshot| filter.matches_snapshot(snapshot, filter_ctx, game))
}

fn triggering_event_object_matched_last_known(
    game: &GameState,
    ctx: &ExternalEvaluationContext<'_>,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let Some(snapshot) = ctx.triggering_event.and_then(TriggerEvent::snapshot) else {
        return false;
    };
    // Last-known characteristics belong to the event object, but relative
    // expressions (such as its power versus this source's power) still need
    // the ability source as their evaluation anchor.
    let filter_ctx = game.filter_context_for(ctx.controller, Some(ctx.source));
    triggering_event_object_matched_last_known_with_filter_context(
        game,
        snapshot,
        filter,
        &filter_ctx,
    )
}

fn triggering_event_object_matched_last_known_with_filter_context(
    game: &GameState,
    snapshot: &crate::snapshot::ObjectSnapshot,
    filter: &crate::target::ObjectFilter,
    filter_ctx: &crate::filter::FilterContext,
) -> bool {
    filter.matches_snapshot(snapshot, filter_ctx, game)
}

#[derive(Debug, Clone, Copy)]
struct SharedConditionContext<'a> {
    controller: PlayerId,
    source: ObjectId,
    filter_source: Option<ObjectId>,
    triggering_event: Option<&'a TriggerEvent>,
    trigger_identity: Option<TriggerIdentity>,
    ability_index: Option<usize>,
}

fn object_matching_was_put_into_graveyard_from_battlefield_this_turn(
    game: &GameState,
    ctx: SharedConditionContext<'_>,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
    game.turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .any(|record| {
            // A simultaneous batch record covers every object it moved.
            let Some(event) = record
                .event
                .downcast::<crate::events::zones::ZoneChangeEvent>()
                .filter(|event| event.from == Zone::Battlefield && event.to == Zone::Graveyard)
            else {
                return false;
            };
            event
                .snapshots()
                .iter()
                .chain(record.object_snapshot.as_ref())
                .any(|snapshot| filter.matches_snapshot(snapshot, &filter_ctx, game))
        })
}

fn damage_source_for_condition(
    game: &GameState,
    ctx: SharedConditionContext<'_>,
    damager: &DamagedBySource,
) -> Option<ObjectId> {
    match damager {
        DamagedBySource::ThisCreature => Some(ctx.source),
        DamagedBySource::EquippedCreature | DamagedBySource::EnchantedCreature => game
            .object(ctx.source)
            .and_then(|obj| obj.attached_to.as_ref())
            .and_then(|target| match target {
                crate::object::AttachmentTarget::Object(id) => Some(*id),
                _ => None,
            }),
    }
}

fn creatures_dealt_damage_by_source_died_this_turn(
    game: &GameState,
    ctx: SharedConditionContext<'_>,
    victim_filter: &crate::target::ObjectFilter,
    damager: &DamagedBySource,
) -> u32 {
    let Some(source_id) = damage_source_for_condition(game, ctx, damager) else {
        return 0;
    };
    let source_stable_id = game.object(source_id).map(|obj| obj.stable_id);
    let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
    let mut count = 0;
    for record in game
        .turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
    {
        let Some(event) = record
            .event
            .downcast::<crate::events::zones::ZoneChangeEvent>()
        else {
            continue;
        };
        if !event.is_dies() {
            continue;
        }
        for victim_id in &event.objects {
            // Each victim of a simultaneous batch record has its own LKI.
            let snapshot = event
                .snapshots()
                .iter()
                .find(|snapshot| snapshot.object_id == *victim_id)
                .or_else(|| {
                    record
                        .object_snapshot
                        .as_ref()
                        .filter(|snapshot| snapshot.object_id == *victim_id)
                });
            let victim_matches = if let Some(snapshot) = snapshot {
                victim_filter.matches_snapshot(snapshot, &filter_ctx, game)
            } else {
                game.object(*victim_id)
                    .is_some_and(|obj| victim_filter.matches(obj, &filter_ctx, game))
            };
            if !victim_matches {
                continue;
            }
            let victim_stable_id = snapshot.map(|snapshot| snapshot.stable_id);
            if game
                .turn_store
                .turn_history
                .creature_was_damaged_by_source_identity_this_turn(
                    *victim_id,
                    victim_stable_id,
                    source_id,
                    source_stable_id,
                )
            {
                count += 1;
            }
        }
    }
    count
}

fn creature_card_was_put_into_your_graveyard_this_turn(game: &GameState, player: PlayerId) -> bool {
    let Some(player_state) = game.player(player) else {
        return false;
    };
    player_state.graveyard.iter().any(|card_id| {
        game.object(*card_id).is_some_and(|object| {
            game.object_has_card_type(object.id, crate::types::CardType::Creature)
                && game
                    .turn_store
                    .turn_history
                    .object_was_put_into_graveyard_this_turn(object.stable_id)
        })
    })
}

fn source_crewed_by_exactly(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    filter_source: Option<ObjectId>,
    triggering_event: Option<&TriggerEvent>,
    count: u32,
    filter: &crate::target::ObjectFilter,
) -> bool {
    if let Some(event) = triggering_event
        && let Some(keyword_action) = event.downcast::<crate::events::KeywordActionEvent>()
        && keyword_action.action == crate::events::KeywordActionKind::Crew
    {
        let filter_ctx = game.filter_context_for(controller, filter_source);
        return keyword_action
            .object_tags
            .get(&crate::TagKey::from(CREWERS_TAG))
            .map(|crewers| {
                crewers
                    .iter()
                    .filter(|snapshot| filter.matches_snapshot(snapshot, &filter_ctx, game))
                    .count() as u32
            })
            .unwrap_or(0)
            == count;
    }

    let filter_ctx = game.filter_context_for(controller, filter_source);
    game.turn_store
        .turn_history
        .crewed_this_turn
        .get(&source)
        .map(|crewers| {
            crewers
                .iter()
                .filter(|id| {
                    game.object(**id)
                        .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
                })
                .count() as u32
        })
        .unwrap_or(0)
        == count
}

fn source_first_crewed_this_turn(
    _game: &GameState,
    source: ObjectId,
    triggering_event: Option<&TriggerEvent>,
) -> bool {
    if let Some(event) = triggering_event
        && let Some(keyword_action) = event.downcast::<crate::events::KeywordActionEvent>()
        && keyword_action.action == crate::events::KeywordActionKind::Crew
    {
        return keyword_action
            .object_tags
            .get(&crate::TagKey::from(FIRST_CREWED_THIS_TURN_TAG))
            .is_some_and(|snapshots| {
                snapshots
                    .iter()
                    .any(|snapshot| snapshot.object_id == source)
            });
    }

    false
}

fn source_crewed_by_exactly_from_resolution_tags(
    game: &GameState,
    ctx: &ExecutionContext,
    count: u32,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let Some(crewers) = ctx.get_tagged_all("crewed_it_this_turn") else {
        return source_crewed_by_exactly(
            game,
            ctx.controller,
            ctx.source,
            Some(ctx.source),
            ctx.triggering_event.as_ref(),
            count,
            filter,
        );
    };
    let filter_ctx = ctx.filter_context(game);
    crewers
        .iter()
        .filter(|snapshot| {
            if let Some(obj) = game.object(snapshot.object_id) {
                filter.matches(obj, &filter_ctx, game)
            } else {
                filter.matches_snapshot(snapshot, &filter_ctx, game)
            }
        })
        .count() as u32
        == count
}

fn object_matching_entered_battlefield_this_turn(
    game: &GameState,
    ctx: SharedConditionContext<'_>,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
    game.turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .any(|record| {
            let entered = record
                .event
                .downcast::<crate::events::EnterBattlefieldEvent>()
                .is_some()
                || record
                    .event
                    .downcast::<crate::events::zones::ZoneChangeEvent>()
                    .is_some_and(|event| event.is_etb());
            entered
                && record
                    .object_snapshot
                    .as_ref()
                    .is_some_and(|snapshot| filter.matches_snapshot(snapshot, &filter_ctx, game))
        })
}

fn object_matching_entered_battlefield_last_turn(
    game: &GameState,
    ctx: SharedConditionContext<'_>,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
    game.turn_store
        .entered_battlefield_last_turn
        .iter()
        .any(|snapshot| {
            if filter.other && snapshot.object_id == ctx.source {
                return false;
            }
            filter.matches_snapshot(snapshot, &filter_ctx, game)
        })
}

fn condition_filter_context(
    game: &GameState,
    you: PlayerId,
    source: ObjectId,
    player_filter: &PlayerFilter,
    triggering_event: Option<&TriggerEvent>,
) -> crate::filter::FilterContext {
    let opponents: Vec<PlayerId> = game
        .players
        .iter()
        .filter(|p| p.id != you)
        .map(|p| p.id)
        .collect();
    let mut ctx = crate::filter::FilterContext::new(you)
        .with_source(source)
        .with_opponents(opponents);
    if *player_filter == PlayerFilter::IteratedPlayer {
        ctx = ctx.with_iterated_player(Some(you));
    }

    let Some(event) = triggering_event else {
        return ctx;
    };
    let Some(object_id) = event.object_id() else {
        return ctx;
    };
    let Some(snapshot) = event.snapshot().cloned().or_else(|| {
        game.object(object_id)
            .map(|obj| crate::snapshot::ObjectSnapshot::from_object(obj, game))
    }) else {
        return ctx;
    };

    ctx.target_objects.push(snapshot.clone());
    ctx.tagged_objects
        .entry(crate::tag::TagKey::from("triggering"))
        .or_default()
        .push(snapshot);
    if let Some(entry) = game.stack.iter().find(|entry| entry.object_id == object_id) {
        ctx.target_objects
            .extend(entry.targets.iter().filter_map(|target| {
                match target {
            crate::game_state::Target::Object(target_id) => game.object(*target_id).map(|object| {
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    object, game,
                )
            }),
            crate::game_state::Target::Player(_) => None,
        }
            }));
    }
    ctx
}

fn triggering_object_had_to_attack_this_combat(
    game: &GameState,
    triggering_event: Option<&TriggerEvent>,
) -> bool {
    triggering_event
        .and_then(|event| event.object_id())
        .is_some_and(|object_id| {
            game.combat
                .as_ref()
                .is_some_and(|combat| combat.creature_had_to_attack_this_combat(object_id))
        })
}

fn triggering_object_became_tapped_first_time_this_turn(
    game: &GameState,
    triggering_event: Option<&TriggerEvent>,
) -> bool {
    let Some(permanent) = triggering_event
        .and_then(|event| event.downcast::<crate::events::PermanentTappedEvent>())
        .map(|event| event.permanent)
    else {
        return false;
    };
    game.turn_store
        .turn_history
        .projected_records()
        .filter_map(|record| {
            record
                .event
                .downcast::<crate::events::PermanentTappedEvent>()
        })
        .filter(|event| event.permanent == permanent)
        .count()
        == 1
}

fn counter_addition_object(event: &TriggerEvent) -> Option<ObjectId> {
    if let Some(event) = event.downcast::<crate::events::CounterPlacedEvent>() {
        return Some(event.permanent);
    }
    event
        .downcast::<crate::events::MarkersChangedEvent>()
        .filter(|event| event.is_added())
        .and_then(crate::events::MarkersChangedEvent::object)
}

fn triggering_object_had_counters_put_first_time_this_turn(
    game: &GameState,
    triggering_event: Option<&TriggerEvent>,
) -> bool {
    let Some(object_id) = triggering_event.and_then(counter_addition_object) else {
        return false;
    };
    game.turn_store
        .turn_history
        .projected_records()
        .filter_map(|record| counter_addition_object(&record.event))
        .filter(|candidate| *candidate == object_id)
        .count()
        == 1
}

fn player_hand_count_at_turn_start(game: &GameState, player_id: PlayerId) -> Option<i32> {
    game.turn_store
        .hand_sizes_at_turn_start
        .get(&player_id)
        .copied()
        .map(|count| count as i32)
}

fn evaluate_turn_history_condition(
    game: &GameState,
    condition: &ironsmith_core::TurnHistoryCondition,
    ctx: SharedConditionContext<'_>,
) -> bool {
    use ironsmith_core::TurnHistoryCondition;

    let matching_players =
        |filter: &PlayerFilter| matching_condition_players_simple(game, ctx.controller, filter);
    let triggering_snapshot = || {
        ctx.triggering_event
            .and_then(TriggerEvent::snapshot)
            .cloned()
            .or_else(|| {
                ctx.triggering_event
                    .and_then(TriggerEvent::object_id)
                    .and_then(|id| game.object(id))
                    .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game))
            })
    };

    match condition {
        TurnHistoryCondition::SpellsCastLastTurnAtLeast(count) => {
            game.turn_store.spells_cast_last_turn_total >= *count
        }
        TurnHistoryCondition::SourceCrewedByAtLeast { count, filter } => {
            let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
            game.turn_store
                .turn_history
                .crewed_this_turn
                .get(&ctx.source)
                .map(|crewers| {
                    crewers
                        .iter()
                        .filter(|id| {
                            game.object(**id)
                                .is_some_and(|object| filter.matches(object, &filter_ctx, game))
                        })
                        .count() as u32
                })
                .unwrap_or(0)
                >= *count
        }
        TurnHistoryCondition::SourceWasCast { .. } => {
            source_was_cast(game, ctx.source, ctx.triggering_event)
        }
        TurnHistoryCondition::SourceWasCastByController { .. } => {
            source_was_cast(game, ctx.source, ctx.triggering_event)
                && game
                    .object(ctx.source)
                    .map(|source| game.controller_of(source))
                    .or_else(|| {
                        ctx.triggering_event
                            .and_then(TriggerEvent::snapshot)
                            .map(|snapshot| snapshot.controller)
                    })
                    == Some(ctx.controller)
        }
        TurnHistoryCondition::SourceWasKicked { .. } => game
            .object(ctx.source)
            .is_some_and(|object| object.optional_costs_paid.was_kicked()),
        TurnHistoryCondition::SourceEnteredBattlefieldThisTurn { .. } => game
            .object(ctx.source)
            .map(|source| source.stable_id)
            .or_else(|| {
                ctx.triggering_event
                    .and_then(TriggerEvent::snapshot)
                    .map(|s| s.stable_id)
            })
            .is_some_and(|stable_id| {
                game.turn_store
                    .turn_history
                    .entered_battlefield_snapshots_this_turn()
                    .iter()
                    .any(|snapshot| snapshot.stable_id == stable_id)
            }),
        TurnHistoryCondition::ObjectAttackedDuringControllersLastTurn(filter) => {
            let mut filter_ctx = FilterContext::new(ctx.controller).with_source(ctx.source);
            if let Some(attached) = game
                .object(ctx.source)
                .and_then(|source| source.attached_to.as_ref())
                .and_then(|target| target.object_id())
                .and_then(|id| game.object(id))
            {
                let snapshot = crate::snapshot::ObjectSnapshot::from_object(attached, game);
                for tag in ["enchanted", "equipped"] {
                    filter_ctx
                        .tagged_objects
                        .insert(crate::tag::TagKey::from(tag), vec![snapshot.clone()]);
                }
            }
            game.battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .any(|object| {
                    filter.matches(object, &filter_ctx, game)
                        && game
                            .last_turn_history_for_player(game.controller_of(object))
                            .is_some_and(|history| {
                                history.creatures_attacked_this_turn.contains(&object.id)
                            })
                })
        }
        TurnHistoryCondition::SourceAttackedThisTurn { .. } => {
            game.creature_attacked_this_turn(ctx.source)
        }
        TurnHistoryCondition::TriggeringObjectEnlistedThisCombat => {
            let triggering_stable_id = triggering_snapshot().map(|snapshot| snapshot.stable_id);
            triggering_stable_id.is_some_and(|stable_id| {
                game.turn_store
                    .turn_history
                    .projected_records()
                    .filter_map(|record| {
                        record.event.downcast::<crate::events::KeywordActionEvent>()
                    })
                    .any(|event| {
                        event.action == crate::events::KeywordActionKind::Enlist
                            && event.combat_phase
                                == Some(game.turn_store.combat_phases_started_this_turn)
                            && event
                                .snapshot
                                .as_ref()
                                .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                    })
            })
        }
        TurnHistoryCondition::TriggeringObjectWasCast => {
            triggering_snapshot().is_some_and(|snapshot| {
                game.turn_store
                    .turn_history
                    .projected_records()
                    .filter_map(|record| record.event.downcast::<crate::events::SpellCastEvent>())
                    .any(|event| {
                        event
                            .snapshot
                            .as_ref()
                            .is_some_and(|cast| cast.stable_id == snapshot.stable_id)
                    })
            })
        }
        TurnHistoryCondition::TriggeringObjectWasCastFromZone(zone) => triggering_snapshot()
            .is_some_and(|snapshot| {
                game.turn_store
                    .turn_history
                    .object_was_cast_from_zone(snapshot.stable_id, *zone)
            }),
        TurnHistoryCondition::PlayerPlayedLandThisTurn(player) => {
            let players = matching_players(player);
            game.turn_store
                .turn_history
                .projected_records()
                .any(|record| {
                    record
                        .event
                        .downcast::<crate::events::LandPlayedEvent>()
                        .is_some_and(|event| players.contains(&event.player))
                })
        }
        TurnHistoryCondition::PlayerActivatedLoyaltyAbilityThisTurn(player) => {
            let players = matching_players(player);
            game.turn_store
                .turn_history
                .projected_records()
                .any(|record| {
                    record
                        .event
                        .downcast::<crate::events::AbilityActivatedEvent>()
                        .is_some_and(|event| {
                            event.is_loyalty_ability && players.contains(&event.activator)
                        })
                })
        }
        TurnHistoryCondition::TriggeringObjectDied => ctx
            .triggering_event
            .and_then(|event| event.downcast::<crate::events::zones::ZoneChangeEvent>())
            .is_some_and(|event| event.to == Zone::Graveyard),
        TurnHistoryCondition::PlayerPlayedCardFromZoneThisTurn { player, zone } => {
            let players = matching_players(player);
            game.turn_store
                .turn_history
                .projected_records()
                .any(|record| {
                    record
                        .event
                        .downcast::<crate::events::SpellCastEvent>()
                        .is_some_and(|event| {
                            event.from_zone == *zone && players.contains(&event.caster)
                        })
                        || record
                            .event
                            .downcast::<crate::events::LandPlayedEvent>()
                            .is_some_and(|event| {
                                event.from_zone == *zone && players.contains(&event.player)
                            })
                })
        }
        TurnHistoryCondition::PlayerCastSpellFromZoneThisTurn { player, zone } => {
            let players = matching_players(player);
            game.turn_store
                .turn_history
                .projected_records()
                .any(|record| {
                    record
                        .event
                        .downcast::<crate::events::SpellCastEvent>()
                        .is_some_and(|event| {
                            event.from_zone == *zone && players.contains(&event.caster)
                        })
                })
        }
        TurnHistoryCondition::PlayerActivatedAbilityOfCardInZoneThisTurn { player, zone } => {
            let players = matching_players(player);
            game.turn_store
                .turn_history
                .projected_records()
                .any(|record| {
                    record
                        .event
                        .downcast::<crate::events::AbilityActivatedEvent>()
                        .is_some_and(|event| {
                            players.contains(&event.activator)
                                && event
                                    .snapshot
                                    .as_ref()
                                    .or(record.object_snapshot.as_ref())
                                    .is_some_and(|snapshot| snapshot.zone == *zone)
                        })
                })
        }
        TurnHistoryCondition::PlayerVisitedAttractionThisTurn(player) => {
            let players = matching_players(player);
            game.turn_store
                .turn_history
                .projected_records()
                .filter_map(|record| record.event.downcast::<crate::events::KeywordActionEvent>())
                .any(|event| {
                    event.action == crate::events::KeywordActionKind::VisitAttraction
                        && players.contains(&event.player)
                })
        }
        TurnHistoryCondition::TriggeringPlayerAttackedControllerLastTurn => {
            let Some(triggering_player) = ctx
                .triggering_event
                .and_then(|event| event.trigger_player().or_else(|| event.player()))
            else {
                return false;
            };
            let Some(history) = game.last_turn_history_for_player(triggering_player) else {
                return false;
            };
            history.projected_records().any(|record| {
                record
                    .event
                    .downcast::<crate::events::combat::CreatureAttackedEvent>()
                    .is_some_and(|attack| {
                        matches!(
                            attack.target,
                            crate::triggers::AttackEventTarget::Player(player)
                                if player == ctx.controller
                        ) && record
                            .object_snapshot
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.controller == triggering_player)
                    })
            })
        }
        TurnHistoryCondition::PlayerLostLifeLastTurn(player) => {
            let players = matching_players(player);
            players.iter().any(|player| {
                game.last_turn_history_for_player(*player)
                    .is_some_and(|history| history.total_life_lost_for_players(&[*player]) > 0)
            })
        }
        TurnHistoryCondition::TriggeringPlayersTurn { .. } => ctx
            .triggering_event
            .and_then(|event| event.trigger_player().or_else(|| event.player()))
            .is_some_and(|player| game.is_active_player(player)),
        TurnHistoryCondition::ControllerTeamGainedLifeThisTurn => {
            let mut team = vec![ctx.controller];
            team.extend(
                game.filter_context_for(ctx.controller, ctx.filter_source)
                    .teammates,
            );
            game.turn_store
                .turn_history
                .total_life_gained_for_players(&team)
                > 0
        }
        TurnHistoryCondition::TriggeringObjectsNoneWereCastOrNoManaSpent => ctx
            .triggering_event
            .and_then(|event| event.downcast::<crate::events::zones::ZoneChangeEvent>())
            .is_some_and(|event| {
                if event.to != Zone::Battlefield {
                    return false;
                }
                let none_were_cast = event.from != Zone::Stack;
                let no_mana_was_spent = if !event.snapshots().is_empty() {
                    event
                        .snapshots()
                        .iter()
                        .all(|snapshot| snapshot.mana_spent_to_cast.total() == 0)
                } else {
                    event.destination_objects().iter().all(|object_id| {
                        game.object(*object_id)
                            .is_none_or(|object| object.mana_spent_to_cast.total() == 0)
                    })
                };
                none_were_cast || no_mana_was_spent
            }),
        TurnHistoryCondition::ManaFromSourceSpentOnTriggeringAction { source_filter } => {
            let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
            let matching_snapshot = |snapshot: &crate::snapshot::ObjectSnapshot| {
                source_filter.matches_snapshot(snapshot, &filter_ctx, game)
            };
            if let Some(cast) = ctx
                .triggering_event
                .and_then(|event| event.downcast::<crate::events::SpellCastEvent>())
            {
                let tag = crate::tag::TagKey::from(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG);
                game.object(cast.spell)
                    .and_then(|spell| spell.cast_tagged_objects.get(&tag))
                    .is_some_and(|snapshots| snapshots.iter().any(matching_snapshot))
            } else {
                ctx.triggering_event
                    .and_then(|event| event.downcast::<crate::events::AbilityActivatedEvent>())
                    .is_some_and(|activation| {
                        activation.mana_sources_spent.iter().any(matching_snapshot)
                    })
            }
        }
        TurnHistoryCondition::AllPlayersLifeAtMost(amount) => game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .all(|player| player.life <= *amount),
        TurnHistoryCondition::AnotherOpponentControlsPotentialTarget { filter } => {
            let Some(cast) = ctx
                .triggering_event
                .and_then(|event| event.downcast::<crate::events::SpellCastEvent>())
            else {
                return false;
            };
            let Some(entry) = game
                .stack
                .iter()
                .find(|entry| entry.object_id == cast.spell)
            else {
                return false;
            };
            let Some(existing_target_controller) = entry.targets.iter().find_map(|target| {
                let crate::game_state::Target::Object(object_id) = target else {
                    return None;
                };
                game.object(*object_id)
                    .map(|object| game.controller_of(object))
            }) else {
                return false;
            };
            let opponents = game
                .filter_context_for(ctx.controller, ctx.filter_source)
                .opponents;
            let mut candidate_filter = filter.clone();
            candidate_filter.zone = Some(Zone::Battlefield);
            candidate_filter.could_be_targeted_by =
                Some(crate::filter::TargetabilityConstraint::by_stack_object(
                    crate::filter::ObjectRef::Specific(cast.spell),
                ));
            let filter_ctx = game.filter_context_for(ctx.controller, Some(cast.spell));
            game.battlefield.iter().copied().any(|object_id| {
                game.object(object_id).is_some_and(|object| {
                    let controller = game.controller_of(object);
                    opponents.contains(&controller)
                        && controller != existing_target_controller
                        && candidate_filter.matches(object, &filter_ctx, game)
                })
            })
        }
        TurnHistoryCondition::TriggeringAttackerBlockers {
            required,
            required_count,
            prohibited,
        } => {
            // The per-pair CreatureBlocked event, or the attacker's single
            // "becomes blocked" event ("becomes blocked by two or more
            // creatures").
            let Some(attacker) = ctx.triggering_event.and_then(|event| {
                event
                    .downcast::<crate::events::combat::CreatureBlockedEvent>()
                    .map(|blocked| blocked.attacker)
                    .or_else(|| {
                        event
                            .downcast::<crate::events::combat::CreatureBecameBlockedEvent>()
                            .map(|blocked| blocked.attacker)
                    })
            }) else {
                return false;
            };
            let Some(combat) = game.combat.as_ref() else {
                return false;
            };
            let filter_ctx = game.filter_context_for(ctx.controller, Some(ctx.source));
            let blockers = crate::combat_state::get_blockers(combat, attacker);
            let required_matches = blockers
                .iter()
                .filter(|blocker| {
                    game.object(**blocker)
                        .is_some_and(|object| required.matches(object, &filter_ctx, game))
                })
                .count() as u32;
            required_matches >= *required_count
                && !blockers.iter().any(|blocker| {
                    game.object(*blocker)
                        .is_some_and(|object| prohibited.matches(object, &filter_ctx, game))
                })
        }
        TurnHistoryCondition::TriggeringAbilityIsManaAbility => ctx
            .triggering_event
            .and_then(|event| event.downcast::<crate::events::AbilityActivatedEvent>())
            .is_some_and(|event| event.is_mana_ability),
    }
}

/// Condition evaluation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionEvaluationMode {
    /// Cast-time evaluation: no full execution context is available yet.
    CastTime {
        controller: PlayerId,
        source: ObjectId,
    },
    /// Resolution-time evaluation: full execution context is available.
    Resolution,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExternalEvaluationOptions {
    /// Recheck the current exact event object when a triggered ability resolves.
    pub triggering_object_current: bool,
    /// If true, treat timing restrictions as satisfied.
    pub ignore_timing: bool,
    /// If true, treat per-turn activation limits as satisfied.
    pub ignore_activation_limits: bool,
    /// The object a static effect is being applied to, when the condition is
    /// evaluated for one recipient ("each creature ... as long as it's not
    /// attacking", "enchanted creature has first strike as long as it's
    /// blocking"). `TargetMatches` binds "it" to this object when there is no
    /// triggering event.
    pub recipient: Option<ObjectId>,
}

/// Whether a condition reads the recipient of the static effect it gates
/// (the object `TargetMatches` binds to outside resolution).
pub fn condition_reads_static_recipient(condition: &Condition) -> bool {
    match condition {
        Condition::TargetMatches(_) => true,
        Condition::Not(inner) => condition_reads_static_recipient(inner),
        Condition::And(left, right) | Condition::Or(left, right) => {
            condition_reads_static_recipient(left) || condition_reads_static_recipient(right)
        }
        _ => false,
    }
}

/// The object an Aura/Equipment/Fortification source is attached to, which is
/// what the "enchanted"/"equipped" tags name outside resolution.
fn external_attached_tag_object(game: &GameState, source: ObjectId, tag: &str) -> Option<ObjectId> {
    if !matches!(tag, "enchanted" | "equipped" | "fortified") {
        return None;
    }
    game.object(source)
        .and_then(|source| source.attached_to.as_ref())
        .and_then(|target| target.object_id())
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExternalEvaluationContext<'a> {
    pub controller: PlayerId,
    pub source: ObjectId,
    /// Player currently being attacked (if evaluation occurs in an attack-defender context).
    pub defending_player: Option<PlayerId>,
    /// Player currently attacking (if different from `controller` in a delegated context).
    pub attacking_player: Option<PlayerId>,
    /// The `FilterContext.source` used when matching ObjectFilters.
    ///
    /// This is intentionally configurable to preserve established semantics:
    /// - Intervening-if checks historically passed `None` so `other` filters do not exclude the source.
    /// - Most other checks should pass `Some(source)`.
    pub filter_source: Option<ObjectId>,
    /// Player bound by an enclosing iteration or attached-object context.
    pub iterated_player: Option<PlayerId>,
    pub triggering_event: Option<&'a TriggerEvent>,
    pub trigger_identity: Option<TriggerIdentity>,
    pub ability_index: Option<usize>,
    pub options: ExternalEvaluationOptions,
}

/// Evaluate a condition outside of effect resolution (trigger checks, activation gating, statics).
pub fn evaluate_condition_external(
    game: &GameState,
    condition: &Condition,
    ctx: &ExternalEvaluationContext<'_>,
) -> bool {
    evaluate_condition_in_context(game, condition, &ConditionContext::external_context(ctx))
        .unwrap_or(false)
}

/// Shared dispatcher for condition evaluation.
pub fn evaluate_condition_with_mode(
    game: &GameState,
    condition: &Condition,
    mode: ConditionEvaluationMode,
    ctx: Option<&ExecutionContext>,
) -> Result<bool, ExecutionError> {
    match mode {
        ConditionEvaluationMode::CastTime { controller, source } => {
            evaluate_condition_cast_time_checked(game, condition, controller, source)
        }
        ConditionEvaluationMode::Resolution => {
            let ctx = ctx.ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "resolution condition evaluation requires execution context".to_string(),
                )
            })?;
            evaluate_condition(game, condition, ctx)
        }
    }
}

/// Evaluate a condition for cast-time decisions.
pub fn evaluate_condition_cast_time(
    game: &GameState,
    condition: &Condition,
    controller: PlayerId,
    source: ObjectId,
) -> bool {
    match evaluate_condition_cast_time_checked(game, condition, controller, source) {
        Ok(value) => value,
        Err(error) => {
            // Legacy bool callers run inside the checked legality / real cast
            // transaction. Preserve unknown until that Result-bearing owner.
            game.record_token_resource_failure(&error);
            false
        }
    }
}

/// Checked cast-time state predicates. Standalone callers must not turn an
/// incomplete continuous world into a completed negative permission answer.
pub fn evaluate_condition_cast_time_checked(
    game: &GameState,
    condition: &Condition,
    controller: PlayerId,
    source: ObjectId,
) -> Result<bool, ExecutionError> {
    let checked = game
        .continuous_query_snapshot()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    evaluate_condition_in_context(
        &checked,
        condition,
        &ConditionContext::cast_time(controller, source),
    )
}

/// Evaluate a condition during effect resolution.
pub fn evaluate_condition_resolution(
    game: &GameState,
    condition: &Condition,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    evaluate_condition_with_mode(
        game,
        condition,
        ConditionEvaluationMode::Resolution,
        Some(ctx),
    )
}

fn condition_objects_for_zone(
    game: &GameState,
    zone: Option<Zone>,
) -> impl Iterator<Item = &crate::object::Object> + '_ {
    let zone = zone.unwrap_or(Zone::Battlefield);
    game.zone_ids(zone).filter_map(|id| game.object(id))
}

fn tagged_object_name_matches_object_set(
    game: &GameState,
    ctx: &ExecutionContext,
    tag: &crate::tag::TagKey,
    filter: &crate::filter::ObjectFilter,
) -> Option<bool> {
    // When `__it__` is not a live loop binding, a same-name constraint inside
    // TaggedObjectMatches represents the comparison set on the right-hand side
    // of a clause such as "it has the same name as a card in your graveyard."
    // Preserve ordinary per-object loop behavior whenever `__it__` is bound.
    if ctx.get_tagged_all(IMPLICIT_IT_TAG).is_some() {
        return None;
    }

    let mut comparison_set = filter.clone();
    let before = comparison_set.tagged_constraints.len();
    comparison_set.tagged_constraints.retain(|constraint| {
        !(constraint.tag.as_str() == IMPLICIT_IT_TAG
            && constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged)
    });
    if comparison_set.tagged_constraints.len() == before {
        return None;
    }

    let tagged = ctx.get_tagged_all(tag.as_str())?;
    let filter_ctx = ctx.filter_context(game);
    Some(tagged.iter().any(|snapshot| {
        condition_objects_for_zone(game, comparison_set.zone).any(|candidate| {
            crate::filter::names_share(
                &snapshot.name,
                snapshot.split_other_half_name(),
                &candidate.name,
                candidate.split_other_half_name(),
            ) && comparison_set.matches(candidate, &filter_ctx, game)
        })
    }))
}

fn condition_object_matches_player_zone(
    game: &GameState,
    obj: &crate::object::Object,
    player_id: PlayerId,
    zone: Option<Zone>,
) -> bool {
    match zone {
        Some(Zone::Battlefield) | None => game.controller_of(obj) == player_id,
        _ => obj.owner == player_id,
    }
}

fn count_distinct_card_types_in_graveyard(game: &GameState, player_id: PlayerId) -> usize {
    use std::collections::HashSet;

    let Some(player_state) = game.player(player_id) else {
        return 0;
    };

    let mut seen = HashSet::new();
    for &object_id in &player_state.graveyard {
        for card_type in game.calculated_card_types(object_id) {
            seen.insert(card_type);
        }
    }
    seen.len()
}

fn count_distinct_matching_powers(
    game: &GameState,
    player_id: PlayerId,
    filter: &crate::target::ObjectFilter,
    filter_ctx: &crate::filter::FilterContext,
) -> usize {
    use std::collections::HashSet;

    let mut seen_powers = HashSet::new();
    for obj in condition_objects_for_zone(game, filter.zone)
        .filter(|obj| condition_object_matches_player_zone(game, obj, player_id, filter.zone))
        .filter(|obj| filter.matches(obj, filter_ctx, game))
    {
        if let Some(power) = game.calculated_power(obj.id).or_else(|| obj.power()) {
            seen_powers.insert(power);
        }
    }
    seen_powers.len()
}

fn player_had_land_enter_battlefield_this_turn(game: &GameState, player_id: PlayerId) -> bool {
    game.turn_store
        .turn_history
        .player_had_land_enter_battlefield_this_turn(player_id)
}

fn player_has_full_party(game: &GameState, player_id: PlayerId) -> bool {
    crate::party::party_size(game, player_id) == 4
}

/// Evaluate a condition with minimal context (for cast-time evaluation).
///
/// This simplified version is used during spell casting to evaluate conditions
/// like `YouControlCommander` before targets are chosen. It handles common
/// conditions that don't require targets or other context-dependent information.
fn resolve_condition_player_simple(
    game: &GameState,
    controller: PlayerId,
    player: &PlayerFilter,
) -> Option<PlayerId> {
    match player {
        PlayerFilter::You => Some(controller),
        PlayerFilter::Specific(id) => Some(*id),
        PlayerFilter::Active => game.active_player_id(),
        PlayerFilter::NotYou => game.players.iter().find_map(|p| {
            if p.id != controller && p.is_in_game() {
                Some(p.id)
            } else {
                None
            }
        }),
        PlayerFilter::Opponent => game.players.iter().find_map(|p| {
            if p.id != controller && p.is_in_game() {
                Some(p.id)
            } else {
                None
            }
        }),
        PlayerFilter::PlayerToYourLeft => {
            game.closest_in_game_player_to_left_matching(controller, |_| true)
        }
        PlayerFilter::PlayerToYourRight => {
            game.closest_in_game_player_to_right_matching(controller, |_| true)
        }
        PlayerFilter::MostLifeTied => {
            let max_life = game
                .players
                .iter()
                .filter(|player| player.is_in_game())
                .map(|player| player.life)
                .max()?;
            game.players.iter().find_map(|player| {
                (player.is_in_game() && player.life == max_life).then_some(player.id)
            })
        }
        PlayerFilter::LowestLifeTied => {
            let min_life = game
                .players
                .iter()
                .filter(|player| player.is_in_game())
                .map(|player| player.life)
                .min()?;
            game.players.iter().find_map(|player| {
                (player.is_in_game() && player.life == min_life).then_some(player.id)
            })
        }
        PlayerFilter::MostCardsInHand => {
            let max_hand = game
                .players
                .iter()
                .filter(|player| player.is_in_game())
                .map(|player| player.hand.len())
                .max()?;
            let leaders = game
                .players
                .iter()
                .filter(|player| player.is_in_game() && player.hand.len() == max_hand)
                .map(|player| player.id)
                .collect::<Vec<_>>();
            match leaders.as_slice() {
                [leader] => Some(*leader),
                _ => None,
            }
        }
        PlayerFilter::CardsInHandAtLeastMoreThanYou { .. }
        | PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. }
        | PlayerFilter::HasMoreLifeThanYou { .. }
        | PlayerFilter::OpponentWithMoreControlledObjectsThan { .. }
        | PlayerFilter::ControlsMost { .. }
        | PlayerFilter::ControlsFewestTied { .. }
        | PlayerFilter::OpponentOf(_)
        | PlayerFilter::MaxSpeed { .. } => {
            let filter_ctx = crate::target::FilterContext::new(controller)
                .with_opponents(
                    game.players
                        .iter()
                        .filter(|p| p.id != controller && p.is_in_game())
                        .map(|p| p.id)
                        .collect(),
                )
                .with_active_player(game.turn.active_player);
            game.players.iter().find_map(|candidate| {
                (candidate.is_in_game()
                    && player_filter_matches_game(player, candidate.id, game, &filter_ctx))
                .then_some(candidate.id)
            })
        }
        PlayerFilter::Any
        | PlayerFilter::CastCardTypeThisTurn(_)
        | PlayerFilter::AttackedBySourceThisTurn
        | PlayerFilter::WasDealtDamageBySourceThisGame { .. }
        | PlayerFilter::LostLifeThisTurn { .. }
        | PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. }
        | PlayerFilter::Target(_)
        | PlayerFilter::AliasedTarget(_)
        | PlayerFilter::Teammate
        | PlayerFilter::Attacking
        | PlayerFilter::Defending
        | PlayerFilter::DamagedPlayer
        | PlayerFilter::EffectController
        | PlayerFilter::ChosenPlayer
        | PlayerFilter::TaggedPlayer(_)
        | PlayerFilter::IteratedPlayer
        | PlayerFilter::TargetPlayerOrControllerOfTarget
        | PlayerFilter::Excluding { .. }
        | PlayerFilter::ControllerOf(_)
        | PlayerFilter::OwnerOf(_)
        | PlayerFilter::AliasedOwnerOf(_)
        | PlayerFilter::AliasedControllerOf(_) => None,
    }
}

fn resolve_condition_player_external(
    game: &GameState,
    ctx: &ExternalEvaluationContext<'_>,
    player: &PlayerFilter,
) -> Option<PlayerId> {
    match player {
        PlayerFilter::IteratedPlayer => ctx.iterated_player,
        PlayerFilter::Defending => ctx
            .defending_player
            .or_else(|| combat_defending_player(game)),
        PlayerFilter::Attacking => Some(ctx.attacking_player.unwrap_or(ctx.controller)),
        _ => resolve_condition_player_simple(game, ctx.controller, player),
    }
}

/// The defending player of the current combat when no event names one, as an
/// activated ability's "only if defending player controls ..." reads it: the
/// one player being attacked, or before attackers are declared in a
/// two-player game, the nonactive player (CR 506.2).
fn combat_defending_player(game: &GameState) -> Option<PlayerId> {
    if game.turn.phase != crate::game_state::Phase::Combat {
        return None;
    }
    if let Some(combat) = game.combat.as_ref() {
        let players = crate::combat_state::defending_players(combat);
        if let [player] = players.as_slice() {
            return Some(*player);
        }
        if !players.is_empty() {
            return None;
        }
    }
    let mut nonactive = game
        .players
        .iter()
        .filter(|player| player.is_in_game() && !game.is_active_player(player.id));
    let player = nonactive.next()?;
    nonactive.next().is_none().then_some(player.id)
}

fn matching_condition_players_simple(
    game: &GameState,
    controller: PlayerId,
    player: &PlayerFilter,
) -> Vec<PlayerId> {
    match player {
        PlayerFilter::Opponent | PlayerFilter::NotYou => game
            .players
            .iter()
            .filter(|p| p.id != controller && p.is_in_game())
            .map(|p| p.id)
            .collect(),
        PlayerFilter::Any => game
            .players
            .iter()
            .filter(|p| p.is_in_game())
            .map(|p| p.id)
            .collect(),
        _ => resolve_condition_player_simple(game, controller, player)
            .into_iter()
            .collect(),
    }
}

/// A read-only copy of the resolution context with "an opponent" bound to
/// `opponent`, used to test a quantified-opponent condition per opponent.
fn an_opponent_probe_context(
    exec: &ExecutionContext,
    opponent: PlayerId,
) -> ExecutionContext<'static> {
    let mut probe = ExecutionContext::new_default(exec.source, exec.controller);
    probe.targets = exec.targets.clone();
    probe.target_assignments = exec.target_assignments.clone();
    probe.target_snapshots = exec.target_snapshots.clone();
    probe.x_value = exec.x_value;
    probe.effect_outcomes = exec.effect_outcomes.clone();
    probe.iteration = exec.iteration.clone();
    probe.combat = exec.combat;
    probe.source_snapshot = exec.source_snapshot.clone();
    probe.tagged_objects = exec.tagged_objects.clone();
    probe.tagged_players = exec.tagged_players.clone();
    probe.triggering_event = exec.triggering_event.clone();
    probe.event_value_amount = exec.event_value_amount;
    probe.optional_costs_paid = exec.optional_costs_paid.clone();
    probe.trigger_identity = exec.trigger_identity;
    probe.ability_index = exec.ability_index;
    probe.set_tagged_players(
        crate::tag::TagKey::from(crate::effects::helpers::AN_OPPONENT_CHOICE_TAG),
        vec![opponent],
    );
    probe
}

/// Evaluate a condition.
fn evaluate_condition(
    game: &GameState,
    condition: &Condition,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    evaluate_condition_in_context(game, condition, &ConditionContext::resolution(ctx))
}

fn matching_snow_mana_was_spent(snapshot: &crate::snapshot::ObjectSnapshot) -> bool {
    use crate::color::Color;
    let spent = &snapshot.snow_mana_spent_to_cast;
    [
        (Color::White, spent.white),
        (Color::Blue, spent.blue),
        (Color::Black, spent.black),
        (Color::Red, spent.red),
        (Color::Green, spent.green),
    ]
    .into_iter()
    .any(|(color, amount)| amount > 0 && snapshot.colors.contains(color))
}

/// One exhaustive interpreter for the condition language. Context adapters own
/// binding and availability policies; each condition's rule is implemented here.
fn evaluate_condition_in_context(
    game: &GameState,
    condition: &Condition,
    ctx: &ConditionContext<'_, '_>,
) -> Result<bool, ExecutionError> {
    let shared = ctx.shared();
    match condition {
        Condition::ItIsNight => Ok(game.is_night),
        Condition::FirstCombatPhaseOfTurn => Ok(game.turn.phase
            == crate::game_state::Phase::Combat
            && game.turn_store.combat_phases_started_this_turn == 1),
        Condition::YouControl(filter) => {
            let filter_ctx = ctx.filter_context(game);

            let has_matching = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| game.controller_of(obj) == ctx.controller)
                .any(|obj| filter.matches(obj, &filter_ctx, game));

            Ok(has_matching)
        }
        Condition::OpponentControls(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let opponents = &filter_ctx.opponents;

            let has_matching = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| opponents.contains(&game.controller_of(obj)))
                .any(|obj| filter.matches(obj, &filter_ctx, game));

            Ok(has_matching)
        } // These history predicates retain the shared core's pre-resolution
          // player bindings, even when an execution context is available.
        Condition::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype } => {
            let players = matching_condition_players_simple(game, shared.controller, player);
            Ok(game
                .turn_store
                .turn_history
                .player_was_dealt_combat_damage_by_creature_subtype_this_turn(&players, *subtype))
        }
        Condition::PlayerControls { player, filter } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    let filter_ctx = ctx.player_filter_context(game, player, player_id);
                    let has_matching = condition_objects_for_zone(game, filter.zone)
                        .filter(|obj| {
                            condition_object_matches_player_zone(game, obj, player_id, filter.zone)
                        })
                        .any(|obj| filter.matches(obj, &filter_ctx, game));
                    Ok(has_matching)
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerOwnsCardNamedInZones {
            player,
            name,
            zones,
        } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    let filter_ctx = ctx.owned_card_filter_context(game, player, player_id);

                    if zones.is_empty() {
                        return Ok(false);
                    }

                    let mut filter = crate::target::ObjectFilter::default().named(name.clone());
                    for zone in zones {
                        filter.zone = Some(*zone);
                        let has_matching = condition_objects_for_zone(game, Some(*zone))
                            .filter(|obj| obj.owner == player_id)
                            .any(|obj| filter.matches(obj, &filter_ctx, game));
                        if !has_matching {
                            return Ok(false);
                        }
                    }

                    Ok(true)
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerHasAtLeast {
            player,
            filter,
            count,
        } => Ok(ctx
            .matching_players(game, player)?
            .into_iter()
            .any(|player_id| {
                let filter_ctx = ctx.player_filter_context(game, player, player_id);
                condition_objects_for_zone(game, filter.zone)
                    .filter(|obj| {
                        condition_object_matches_player_zone(game, obj, player_id, filter.zone)
                    })
                    .filter(|obj| filter.matches(obj, &filter_ctx, game))
                    .count()
                    >= *count as usize
            })),
        Condition::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count } => {
            use crate::types::Subtype;
            use std::collections::HashSet;

            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    let mut seen: HashSet<Subtype> = HashSet::new();
                    for obj in game
                        .battlefield
                        .iter()
                        .filter_map(|&id| game.object(id))
                        .filter(|obj| game.controller_of(obj) == player_id && obj.is_land())
                    {
                        for subtype in game.calculated_subtypes(obj.id) {
                            if matches!(
                                subtype,
                                Subtype::Plains
                                    | Subtype::Island
                                    | Subtype::Swamp
                                    | Subtype::Mountain
                                    | Subtype::Forest
                            ) {
                                seen.insert(subtype);
                            }
                        }
                    }
                    Ok(seen.len() >= *count as usize)
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerHasCardTypesInGraveyardOrMore { player, count } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    Ok(count_distinct_card_types_in_graveyard(game, player_id) >= *count as usize)
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerControlsExactly {
            player,
            filter,
            count,
        } => Ok(ctx
            .matching_players(game, player)?
            .into_iter()
            .any(|player_id| {
                let filter_ctx = ctx.player_filter_context(game, player, player_id);
                condition_objects_for_zone(game, filter.zone)
                    .filter(|object| {
                        condition_object_matches_player_zone(game, object, player_id, filter.zone)
                    })
                    .filter(|object| filter.matches(object, &filter_ctx, game))
                    .count()
                    == *count as usize
            })),
        Condition::PlayerHasAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    let filter_ctx = ctx.player_filter_context(game, player, player_id);
                    let distinct = count_distinct_matching_powers(game, player_id, filter, &filter_ctx);
                    Ok(distinct >= *count as usize)
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerControlsMost { player, filter } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    let count_for = |candidate: PlayerId| {
                        let filter_ctx = ctx.player_filter_context(game, player, candidate);
                        condition_objects_for_zone(game, filter.zone)
                            .filter(|obj| {
                                condition_object_matches_player_zone(game, obj, candidate, filter.zone)
                            })
                            .filter(|obj| filter.matches(obj, &filter_ctx, game))
                            .count()
                    };
                    let current = count_for(player_id);
                    let max_count = game
                        .players
                        .iter()
                        .map(|player| count_for(player.id))
                        .max()
                        .unwrap_or(0);
                    Ok(if ctx.external().is_some() {
                        current >= max_count
                    } else {
                        current == max_count
                    })
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerControlsMoreThanEachOtherPlayer { player, filter } => Ok(ctx
            .matching_players(game, player)?
            .into_iter()
            .any(|player_id| {
                player_controls_more_than_each_other_player(
                    game, ctx.source, player, player_id, filter,
                )
            })),
        Condition::PlayerControlsMoreThanYou { player, filter } => {
            let count_for = |candidate: PlayerId| {
                let filter_ctx = ctx.player_filter_context(game, player, candidate);
                condition_objects_for_zone(game, filter.zone)
                    .filter(|obj| {
                        condition_object_matches_player_zone(game, obj, candidate, filter.zone)
                    })
                    .filter(|obj| filter.matches(obj, &filter_ctx, game))
                    .count()
            };
            Ok(ctx
                .matching_players(game, player)?
                .into_iter()
                .any(|player_id| count_for(player_id) > count_for(ctx.controller)))
        }
        Condition::AnOpponentControlsMoreThanPlayer { player, filter } => Ok(ctx
            .matching_players(game, player)?
            .into_iter()
            .any(|player_id| {
                any_opponent_controls_more_than_player(game, ctx.source, player, player_id, filter)
            })),
        Condition::AnOpponentHasFewerThanPlayer { player, filter } => Ok(ctx
            .matching_players(game, player)?
            .into_iter()
            .any(|player_id| {
                any_opponent_has_fewer_than_player(game, ctx.source, player, player_id, filter)
            })),
        Condition::PlayerLifeAtMostHalfStartingLifeTotal { player } => Ok(ctx
            .matching_players(game, player)?
            .into_iter()
            .any(|player_id| player_life_compares_to_half_starting(game, player_id, true))),
        Condition::PlayerLifeLessThanHalfStartingLifeTotal { player } => Ok(ctx
            .matching_players(game, player)?
            .into_iter()
            .any(|player_id| player_life_compares_to_half_starting(game, player_id, false))),
        Condition::PlayerHasLessLifeThanYou { player } => {
            let life = game.player(ctx.controller).map(|p| p.life);
            if ctx.is_cast_time() && life.is_none() {
                return Ok(false);
            }
            let you_life = life.unwrap_or(0);
            Ok(ctx
                .matching_players(game, player)?
                .into_iter()
                .any(|player_id| {
                    let other = game.player(player_id).map(|p| p.life);
                    (!ctx.is_cast_time() || other.is_some()) && other.unwrap_or(0) < you_life
                }))
        }
        Condition::PlayerHasMoreLifeThanYou { player } => {
            let life = game.player(ctx.controller).map(|p| p.life);
            if ctx.is_cast_time() && life.is_none() {
                return Ok(false);
            }
            let you_life = life.unwrap_or(0);
            Ok(ctx
                .matching_players(game, player)?
                .into_iter()
                .any(|player_id| {
                    let other = game.player(player_id).map(|p| p.life);
                    (!ctx.is_cast_time() || other.is_some()) && other.unwrap_or(0) > you_life
                }))
        }
        Condition::PlayerHasNoOpponentWithMoreLifeThan { player } => Ok(ctx
            .matching_players(game, player)?
            .into_iter()
            .any(|player_id| {
                player_has_no_opponent_with_more_life_than(game, ctx.controller, player_id)
            })),
        Condition::PlayerHasMoreLifeThanEachOtherPlayer { player } => Ok(ctx
            .matching_players(game, player)?
            .into_iter()
            .any(|player_id| player_has_more_life_than_each_other_player(game, player_id))),
        Condition::PlayerWasMonarchAtTurnStart {player} => Ok(ctx.matching_players(game,player)?.into_iter()
            .any(|player|game.turn_store.turn_history.monarch_at_turn_start==Some(player))),
        Condition::PlayerIsMonarch { player } => Ok(ctx
            .matching_players(game, player)?
            .into_iter()
            .any(|player_id| game.is_monarch(player_id))),
        Condition::PlayerHasInitiative { player } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    Ok(game.has_initiative(player_id))
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerHasCitysBlessing { player } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    Ok(game.has_citys_blessing(player_id))
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerHasEnduringStory { player } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    Ok(game.has_enduring_story(player_id))
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerCommittedCrimeThisTurn { player } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    Ok(game
                        .turn_store
                        .turn_history
                        .player_committed_crime_this_turn(player_id))
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerRolledResultThisTurn { player, result } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    Ok(game
                        .turn_store
                        .turn_history
                        .player_rolled_result_this_turn(player_id, *result))
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerCompletedDungeon {
            player,
            dungeon_name,
        } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    Ok(match dungeon_name {
                        Some(name) => game.has_completed_named_dungeon(player_id, name),
                        None => game.has_completed_dungeon(player_id),
                    })
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerCardsInHandOrMore { player, count } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    let hand = game.player(player_id).map(|p| p.hand.len());
                    // External gating uses a signed comparison and requires the player to exist.
                    // Cast/resolution historically compare usize counts, with a missing hand as zero.
                    Ok(if ctx.external().is_some() {
                        hand.is_some_and(|hand| hand as i32 >= *count)
                    } else {
                        hand.unwrap_or(0) >= *count as usize
                    })
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerCardsInHandOrFewer { player, count } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    let hand = game.player(player_id).map(|p| p.hand.len());
                    // External gating uses a signed comparison and requires the player to exist.
                    // Cast/resolution historically compare usize counts, with a missing hand as zero.
                    Ok(if ctx.external().is_some() {
                        hand.is_some_and(|hand| hand as i32 <= *count)
                    } else {
                        hand.unwrap_or(0) <= *count as usize
                    })
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerCardsInHandAtTurnStartOrMore { player, count } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    Ok(player_hand_count_at_turn_start(game, player_id)
                        .map(|hand_count| hand_count >= *count)
                        .unwrap_or(false))
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerCardsInHandAtTurnStartOrFewer { player, count } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    Ok(player_hand_count_at_turn_start(game, player_id)
                        .map(|hand_count| hand_count <= *count)
                        .unwrap_or(false))
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerHasMoreCardsInHandThanYou { player } => {
            let your_hand = game
                .player(ctx.controller)
                .map(|p| p.hand.len())
                .unwrap_or(0);
            Ok(ctx
                .matching_players(game, player)?
                .into_iter()
                .any(|player_id| {
                    game.player(player_id).map(|p| p.hand.len()).unwrap_or(0) > your_hand
                }))
        }
        Condition::PlayerHasMoreCardsInHandThanEachOtherPlayer { player } => Ok(ctx
            .matching_players(game, player)?
            .into_iter()
            .any(|player_id| {
                let hand = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
                game.players
                    .iter()
                    .filter(|candidate| candidate.is_in_game())
                    .all(|candidate| candidate.id == player_id || hand > candidate.hand.len())
            })),
        Condition::PlayerHasPoisonCountersOrMore { player, count } => Ok(ctx
            .matching_players(game, player)?
            .into_iter()
            .any(|player_id| player_poison_counters_or_more(game, player_id, *count))),
        Condition::PlayerHasCountersOrMore {
            player,
            counter_type,
            count,
        } => Ok(ctx
            .matching_players(game, player)?
            .into_iter()
            .any(|player_id| {
                game.player(player_id)
                    .is_some_and(|player| player.counter_count(*counter_type) >= *count)
            })),
        Condition::PlayerCastSpellsThisTurnOrMore { player, count } => {
            let filter_ctx = ctx.spell_history_filter_context(game);
            let player_ids: Vec<PlayerId> = match player {
                PlayerFilter::You => vec![ctx.controller],
                PlayerFilter::Opponent => filter_ctx.opponents,
                PlayerFilter::Specific(id) => vec![*id],
                PlayerFilter::Any => game.players.iter().map(|p| p.id).collect(),
                PlayerFilter::NotYou => game
                    .players
                    .iter()
                    .filter_map(|p| (p.id != ctx.controller).then_some(p.id))
                    .collect(),
                // Surge (CR 702.117a): "you or one of your teammates".
                PlayerFilter::Teammate => game
                    .players
                    .iter()
                    .filter_map(|p| game.are_teammates(ctx.controller, p.id).then_some(p.id))
                    .collect(),
                _ => Vec::new(),
            };
            let cast_count: u32 = player_ids
                .iter()
                .map(|pid| game.turn_store.turn_history.spells_cast_by_player(*pid))
                .sum();
            Ok(cast_count >= *count)
        }
        Condition::PlayerTappedLandForManaThisTurn { player } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    Ok(game
                        .turn_store
                        .turn_history
                        .players_tapped_land_for_mana_this_turn
                        .contains(&player_id))
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerGainedLifeThisTurnOrMore { player, count } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    Ok(game
                        .turn_store
                        .turn_history
                        .total_life_gained_for_players(&[player_id])
                        >= *count)
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::CreatureDiedThisTurnOrMore(count) => Ok(game
            .turn_store
            .turn_history
            .total_creatures_died_this_turn()
            >= *count),
        Condition::CreatureDealtDamageBySourceDiedThisTurn {
            victim,
            damager,
            count,
        } => Ok(
            creatures_dealt_damage_by_source_died_this_turn(game, shared, victim, damager)
                >= *count,
        ),
        Condition::CreatureCardPutIntoYourGraveyardThisTurn => Ok(
            creature_card_was_put_into_your_graveyard_this_turn(game, shared.controller),
        ),
        Condition::PlayerHadLandEnterBattlefieldThisTurn { player } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    Ok(player_had_land_enter_battlefield_this_turn(game, player_id))
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerDescendedThisTurn { player } => {
            for player_id in ctx.matching_players(game, player)? {
                // A quantified player ("an opponent", "a player") matches
                // when any such player satisfies the condition.
                let matched = (|| -> Result<bool, ExecutionError> {
                    Ok(game
                        .turn_store
                        .turn_history
                        .player_descended_count_this_turn(player_id)
                        > 0)
                })()?;
                if matched {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::TargetIsTapped => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            if let Some(crate::effects::ResolvedTarget::Object(id)) = ctx.targets.first() {
                return Ok(game.is_tapped(*id));
            }
            Ok(false)
        }
        Condition::TargetWasKicked => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            for target in &ctx.targets {
                if let crate::effects::ResolvedTarget::Object(id) = target
                    && let Some(obj) = game.object(*id)
                {
                    return Ok(obj.optional_costs_paid.was_kicked());
                }
            }
            Ok(false)
        }
        Condition::ThisSpellWasKicked => {
            if let Some(ctx) = ctx.execution() {
                Ok(resolve_value(game, &Value::WasKicked, ctx)? != 0)
            } else {
                Ok(game
                    .object(ctx.source)
                    .is_some_and(|obj| obj.optional_costs_paid.was_kicked()))
            }
        }
        Condition::ThisSpellEscaped => Ok(source_escaped(game, shared.source)),
        Condition::ThisSpellWasCastFromZone(zone) => {
            if let Some(ctx) = ctx.execution()
                && this_spell_was_cast_from_zone(game, ctx.source, ctx, *zone)
            {
                return Ok(true);
            }
            // Intervening-if checks ("When this creature enters, if you cast it
            // from your hand, ...") run without an execution context, and a
            // normal cast carries no zone in its casting method, so fall back
            // to the recorded cast event.
            Ok(source_was_cast_from_zone(
                game,
                cast_condition_subject(shared.source, shared.triggering_event),
                shared.triggering_event,
                *zone,
            ))
        }
        Condition::ThisSpellWasCastFromNonHand => {
            if let Some(ctx) = ctx.execution() {
                return Ok(this_spell_was_cast_from_non_hand(game, ctx.source, ctx));
            }
            // Intervening-if and static checks ("When this creature enters,
            // if you cast it from anywhere other than your hand, ...") have no
            // casting method; read the recorded cast zone instead.
            Ok(source_was_cast_from_non_hand(
                game,
                cast_condition_subject(shared.source, shared.triggering_event),
                shared.triggering_event,
            ))
        }
        Condition::ThisSpellWasCastAtSorceryTiming => Ok(game
            .object(shared.source)
            .is_some_and(|object| object.optional_costs_paid.was_cast_at_sorcery_timing())),
        Condition::ThisSpellPaidLabel(label) => {
            if let Some(ctx) = ctx.execution() {
                Ok(resolve_value(game, &Value::WasPaidLabel(label.clone()), ctx)? != 0)
            } else {
                Ok(game
                    .object(ctx.source)
                    .is_some_and(|obj| obj.optional_costs_paid.was_paid_label(label.clone())))
            }
        }
        Condition::YouHaveFullParty => Ok(player_has_full_party(game, shared.controller)),
        Condition::TargetSpellCastOrderThisTurn(order) => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            for target in &ctx.targets {
                if let crate::effects::ResolvedTarget::Object(id) = target {
                    let actual = game
                        .turn_store
                        .turn_history
                        .spell_cast_order(*id)
                        .unwrap_or(0);
                    return Ok(actual == *order);
                }
            }
            Ok(false)
        }
        Condition::TargetSpellControllerIsPoisoned => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            for target in &ctx.targets {
                if let crate::effects::ResolvedTarget::Object(id) = target
                    && let Some(obj) = game.object(*id)
                    && let Some(player) = game.player(game.controller_of(obj))
                {
                    return Ok(player.poison_counters > 0);
                }
            }
            Ok(false)
        }
        Condition::TargetSpellManaSpentToCastAtLeast { amount, symbol } => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            for target in &ctx.targets {
                if let crate::effects::ResolvedTarget::Object(id) = target
                    && let Some(obj) = game.object(*id)
                {
                    return Ok(mana_pool_amount(&obj.mana_spent_to_cast, *symbol) >= *amount);
                }
            }
            Ok(false)
        }
        Condition::TriggeringSpellManaSpentToCastAtLeast { amount, symbol } => Ok(
            triggering_spell_mana_spent_at_least(game, shared.triggering_event, *amount, *symbol),
        ),
        Condition::ColoredManaSpentToCastThisSpellAtLeast(amount) => {
            let Some(source_obj) = game.object(shared.source) else {
                return Ok(false);
            };
            Ok(mana_pool_colored_total(&source_obj.mana_spent_to_cast) >= *amount)
        }
        Condition::TriggeringSpellColoredManaSpentToCastAtLeast(amount) => Ok(
            triggering_spell_colored_mana_spent_at_least(game, shared.triggering_event, *amount),
        ),
        Condition::TriggeringSpellWasKicked => {
            Ok(triggering_spell_was_kicked(game, shared.triggering_event))
        }
        Condition::YouControlMoreCreaturesThanTargetSpellController => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            let target_controller = ctx.targets.iter().find_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => {
                    game.object(*id).map(|obj| game.controller_of(obj))
                }
                _ => None,
            });
            let Some(target_controller) = target_controller else {
                return Ok(false);
            };

            let you_count = game
                .battlefield
                .iter()
                .filter(|&&id| {
                    game.object(id).is_some_and(|obj| {
                        game.controller_of(obj) == ctx.controller
                            && game.object_has_card_type(id, crate::types::CardType::Creature)
                    })
                })
                .count();
            let target_count = game
                .battlefield
                .iter()
                .filter(|&&id| {
                    game.object(id).is_some_and(|obj| {
                        game.controller_of(obj) == target_controller
                            && game.object_has_card_type(id, crate::types::CardType::Creature)
                    })
                })
                .count();
            Ok(you_count > target_count)
        }
        Condition::TargetHasGreatestPowerAmongCreatures => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            let target_id = ctx.targets.iter().find_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => Some(*id),
                _ => None,
            });
            let Some(target_id) = target_id else {
                return Ok(false);
            };
            let Some(target_obj) = game.object(target_id) else {
                return Ok(false);
            };
            if !game.object_has_card_type(target_id, crate::types::CardType::Creature) {
                return Ok(false);
            }
            let Some(target_power) = game
                .calculated_power(target_id)
                .or_else(|| target_obj.power())
            else {
                return Ok(false);
            };
            let max_power = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| game.object_has_card_type(obj.id, crate::types::CardType::Creature))
                .filter_map(|obj| game.calculated_power(obj.id).or_else(|| obj.power()))
                .max();
            Ok(max_power.is_some_and(|max| target_power >= max))
        }
        Condition::TargetManaValueLteColorsSpentToCastThisSpell => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            let target_id = ctx.targets.iter().find_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => Some(*id),
                _ => None,
            });
            let Some(target_id) = target_id else {
                return Ok(false);
            };
            let Some(target_obj) = game.object(target_id) else {
                return Ok(false);
            };
            let Some(source_obj) = game.object(ctx.source) else {
                return Ok(false);
            };
            let target_mana_value =
                crate::filter::object_mana_value_for_filter(target_obj).max(0) as u32;
            let colors_spent = [
                source_obj.mana_spent_to_cast.white,
                source_obj.mana_spent_to_cast.blue,
                source_obj.mana_spent_to_cast.black,
                source_obj.mana_spent_to_cast.red,
                source_obj.mana_spent_to_cast.green,
            ]
            .into_iter()
            .filter(|amount| *amount > 0)
            .count() as u32;
            Ok(target_mana_value <= colors_spent)
        }
        Condition::SourceIsTapped => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game.is_tapped(ctx.source))
        }
        Condition::SourceIsSaddled => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game.is_saddled(ctx.source))
        }
        Condition::SourceCrewedByExactly { count, filter } => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(if let Some(exec) = ctx.execution() {
                source_crewed_by_exactly_from_resolution_tags(game, exec, *count, filter)
            } else {
                source_crewed_by_exactly(
                    game,
                    ctx.controller,
                    ctx.source,
                    shared.filter_source,
                    shared.triggering_event,
                    *count,
                    filter,
                )
            })
        }
        Condition::SourceDevouredCreaturesOrMore(count) => {
            if ctx.is_cast_time() {
                return Ok(false);
            }

            Ok(game.devoured_count(ctx.source) >= *count)
        }
        Condition::SourceIsHarnessed => Ok(!ctx.is_cast_time() && game.is_harnessed(ctx.source)),
        Condition::SourceIsMonstrous => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game.is_monstrous(ctx.source))
        }
        Condition::SourceIsFaceDown => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(source_is_face_down_or_alternate_face(game, ctx.source))
        }
        Condition::SourceMatches(filter) => {
            let filter_ctx = game.filter_context_for(shared.controller, Some(shared.source));
            Ok(game
                .object(shared.source)
                .is_some_and(|obj| filter.matches(obj, &filter_ctx, game)))
        }
        Condition::AttachedToSourceMatches(filter) => {
            let filter_ctx = game.filter_context_for(shared.controller, Some(shared.source));
            let retained=ctx.execution().and_then(|execution| execution.source_snapshot.as_ref());
            Ok(crate::effects::helpers::source_attachment_target_with_lki(game,shared.source,retained)
                .and_then(|target| target.object_id())
                .and_then(|id| game.object(id))
                .is_some_and(|object| filter.matches(object, &filter_ctx, game)))
        }
        Condition::AttachmentCount {
            attachment,
            host,
            comparison,
            ..
        } => {
            let filter_ctx = game.filter_context_for(shared.controller, Some(shared.source));
            Ok(attachment_count_condition_matches(
                game,
                shared.source,
                attachment,
                host,
                comparison,
                &filter_ctx,
            ))
        }
        Condition::SourcePowerAtLeast(min_power) => Ok(game
            .calculated_power(shared.source)
            .or_else(|| game.object(shared.source).and_then(|obj| obj.power()))
            .is_some_and(|power| power >= *min_power as i32)),
        Condition::SourceHasCountersAtLeast(count) => Ok(game
            .object(shared.source)
            .map(|obj| obj.counters.values().copied().sum::<u32>() >= *count)
            .unwrap_or(false)),
        Condition::SourceAttackedOrBlockedThisTurn => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game.creature_attacked_this_turn(ctx.source)
                || game.creature_blocked_this_turn(ctx.source))
        }
        Condition::TargetIsAttacking => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            let Some(crate::effects::ResolvedTarget::Object(id)) = ctx.targets.first() else {
                return Ok(false);
            };
            Ok(game
                .combat
                .as_ref()
                .is_some_and(|combat| crate::combat_state::is_attacking(combat, *id)))
        }
        Condition::TargetIsBlocked => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            if let Some(crate::effects::ResolvedTarget::Object(id)) = ctx.targets.first()
                && let Some(combat) = &game.combat
            {
                return Ok(crate::combat_state::is_blocked(combat, *id));
            }
            Ok(false)
        }
        Condition::TaggedObjectMatches(tag, filter) => {
            if let Some(external) = ctx.external() {
                if tag.as_str() == "triggering" {
                    return Ok(triggering_event_object_matches(game, external, filter));
                }
                if tag.as_str() == "damaged" {
                    let recipient = external.triggering_event
                        .and_then(|event| event.downcast::<crate::events::DamageEvent>())
                        .and_then(|damage| match damage.target {
                            crate::events::DamageTarget::Object(object) => Some(object),
                            _ => None,
                        });
                    let filter_ctx = game.filter_context_for(external.controller, external.filter_source);
                    return Ok(recipient.and_then(|id| game.object(id))
                        .is_some_and(|object| filter.matches(object, &filter_ctx, game)));
                }
                // Static and gating checks have no tagged-object bindings, but
                // "enchanted"/"equipped" always name the source's attachment
                // ("as long as equipped creature is legendary").
                return Ok(
                    external_attached_tag_object(game, external.source, tag.as_str())
                        .and_then(|id| game.object(id))
                        .is_some_and(|object| {
                            let filter_ctx =
                                game.filter_context_for(external.controller, external.filter_source);
                            filter.matches(object, &filter_ctx, game)
                        }),
                );
            }
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            if let Some(matches) = tagged_object_name_matches_object_set(game, ctx, tag, filter) {
                return Ok(matches);
            }
            let filter_ctx = ctx.filter_context(game);
            if tag.as_str() == "triggering"
                && let Some(event) = ctx.triggering_event.as_ref()
            {
                return Ok(triggering_event_object_matches_at_resolution(
                    game,
                    event,
                    filter,
                    &filter_ctx,
                ));
            }
            if let Some(tagged) = ctx.get_tagged_all(tag.as_str()) {
                return Ok(tagged.iter().any(|snapshot| {
                    if let Some(current_id) = crate::effects::helpers::resolve_tagged_object_id(game, ctx, snapshot)
                        && let Some(object) = game.object(current_id)
                    {
                        // Current characteristics own an ordinary predicate;
                        // an earlier successful snapshot cannot override them.
                        return filter.matches(object, &filter_ctx, game);
                    }
                    let last_known = game.turn_store.turn_history
                        .source_departure_snapshot(snapshot.object_id).unwrap_or(snapshot);
                    filter.matches_snapshot(last_known, &filter_ctx, game)
                }));
            }

            // Lowering can synthesize a branch-local tag before runtime tagging
            // exists. Preserve the first-target fallback for those tags only.
            let synthetic_tag = tag.as_str().rsplit_once('_').is_some_and(|(head, suffix)| {
                !head.is_empty() && suffix.chars().all(|c| c.is_ascii_digit())
            });
            if !synthetic_tag {
                return Ok(false);
            }

            let Some(crate::effects::ResolvedTarget::Object(id)) = ctx.targets.first() else {
                return Ok(false);
            };
            if let Some(obj) = game.object(*id) {
                return Ok(filter.matches(obj, &filter_ctx, game));
            }
            if let Some(snapshot) = ctx.target_snapshots.get(id) {
                return Ok(filter.matches_snapshot(snapshot, &filter_ctx, game));
            }
            Ok(false)
        }
        Condition::TaggedObjectMatchedLastKnown(tag, filter) => {
            if let Some(external) = ctx.external() {
                return Ok(tag.as_str() == "triggering"
                    && triggering_event_object_matched_last_known(game, external, filter));
            }
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            let filter_ctx = ctx.filter_context(game);
            if tag.as_str() == "triggering"
                && let Some(snapshot) = ctx
                    .triggering_event
                    .as_ref()
                    .and_then(TriggerEvent::snapshot)
            {
                return Ok(
                    triggering_event_object_matched_last_known_with_filter_context(
                        game,
                        snapshot,
                        filter,
                        &filter_ctx,
                    ),
                );
            }
            Ok(ctx.get_tagged_all(tag.as_str()).is_some_and(|tagged| {
                tagged
                    .iter()
                    .any(|snapshot| filter.matches_snapshot(snapshot, &filter_ctx, game))
            }))
        }
        Condition::TaggedObjectIsTopOfLibrary { tag, player } => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let Some(tagged) = ctx.get_tagged_all(tag.as_str()) else {
                return Ok(false);
            };
            Ok(tagged.iter().any(|snapshot| {
                crate::grant_registry::stable_card_is_top_of_library(
                    game,
                    snapshot.stable_id,
                    player_id,
                )
            }))
        }
        Condition::StableObjectIsTopOfLibrary {
            stable_id,
            player,
            library_top_revision,
        } => Ok(
            crate::grant_registry::stable_card_is_top_of_library_at_revision(
                game,
                *stable_id,
                *player,
                *library_top_revision,
            ),
        ),
        Condition::TaggedObjectWasCast(tag) => {
            if let Some(exec) = ctx.execution()
                && exec.get_tagged_all(tag.as_str()).is_some()
            {
                return Ok(tagged_object_was_cast(game, tag, exec));
            }
            // Event-bound references must also work when the intervening-if
            // is checked before an execution frame has been constructed.
            if tag.as_str() == "triggering"
                && let Some(event) = ctx.shared().triggering_event
                && let Some(snapshot) = event.snapshot()
            {
                let mut probe = ExecutionContext::new_default(ctx.source, ctx.controller)
                    .with_triggering_event(event.clone());
                probe.set_tagged_objects(tag.clone(), vec![snapshot.clone()]);
                return Ok(tagged_object_was_cast(game, tag, &probe));
            }
            Ok(false)
        }
        Condition::TaggedObjectIsSoulbondPaired(tag) => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            let tagged_id = ctx
                .get_tagged(tag.as_str())
                .map(|snapshot| snapshot.object_id);
            Ok(tagged_id.is_some_and(|id| game.is_soulbond_paired(id)))
        }
        Condition::EnchantedPermanentAttackedThisTurn => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };
            Ok(game
                .object(ctx.source)
                .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
                .is_some_and(|attached_to| game.creature_attacked_this_turn(attached_to)))
        }
        Condition::EnchantedPermanentAttackedOrBlockedSinceLastUpkeep => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };
            Ok(
                game.enchanted_permanent_attacked_or_blocked_since_last_upkeep(
                    ctx.source,
                    ctx.controller,
                ),
            )
        }
        Condition::SourceBlockedOrBecameBlockedSinceLastUpkeep => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            Ok(game.source_blocked_or_became_blocked_since_last_upkeep(ctx.source, ctx.controller))
        }
        Condition::TargetMatches(filter) => {
            if let Some(ctx) = ctx.external() {
                return Ok({
                    let filter_ctx = condition_filter_context(
                        game,
                        ctx.controller,
                        ctx.source,
                        &PlayerFilter::You,
                        ctx.triggering_event,
                    );
                    let Some(event) = ctx.triggering_event else {
                        // A static condition reads the object its effect is
                        // being applied to ("as long as it's blocking").
                        return Ok(ctx.options.recipient.is_some_and(|recipient| {
                            game.object(recipient)
                                .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
                        }));
                    };
                    if let Some(snapshot) = event.snapshot() {
                        return Ok(filter.matches_snapshot(snapshot, &filter_ctx, game));
                    }
                    event.object_id().is_some_and(|object_id| {
                        game.object(object_id)
                            .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
                    })
                });
            }
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            let filter_ctx = ctx.filter_context(game);
            let Some(crate::effects::ResolvedTarget::Object(id)) = ctx.targets.first() else {
                return Ok(false);
            };
            if let Some(obj) = game.object(*id) {
                return Ok(filter.matches(obj, &filter_ctx, game));
            }
            if let Some(snapshot) = ctx.target_snapshots.get(id) {
                return Ok(filter.matches_snapshot(snapshot, &filter_ctx, game));
            }
            Ok(false)
        }
        Condition::TargetObjectsHaveDifferentColorSets => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            Ok(target_objects_have_different_color_sets(game, ctx))
        }
        Condition::TargetIsSoulbondPaired => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            let target_id = ctx.targets.iter().find_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => Some(*id),
                _ => None,
            });
            Ok(target_id.is_some_and(|id| game.is_soulbond_paired(id)))
        }
        Condition::PlayerTaggedObjectMatches {
            player,
            tag,
            filter,
            mode,
        } => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let Some(tagged) = ctx.get_tagged_all(tag.as_str()) else {
                return Ok(false);
            };
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.iterated_player = Some(player_id);
            for snapshot in tagged {
                if *mode == crate::effect::TaggedObjectMatchMode::CurrentOrLastKnown {
                    let current_id = game
                        .object(snapshot.object_id)
                        .map(|object| object.id)
                        .or_else(|| game.find_object_by_stable_id(snapshot.stable_id));
                    if let Some(current_id) = current_id
                        && let Some(object) = game.object(current_id)
                    {
                        if game.controller_of(object) == player_id
                            && filter.matches(object, &filter_ctx, game)
                        {
                            return Ok(true);
                        }
                        continue;
                    }
                }
                if snapshot.controller != player_id {
                    continue;
                }
                if filter.matches_snapshot(snapshot, &filter_ctx, game) {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerTaggedObjectEnteredBattlefieldThisTurn { player, tag } => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };

            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let Some(tagged) = ctx.get_tagged_all(tag.as_str()) else {
                return Ok(false);
            };
            Ok(tagged.iter().any(|snapshot| {
                game.turn_store
                    .turn_history
                    .object_entered_battlefield_controller_this_turn(snapshot.stable_id)
                    .is_some_and(|entry_controller| entry_controller == player_id)
            }))
        } // Registration limits must not invalidate an already-registered trigger
          // when its condition is checked again during resolution.
        // "For the first time each turn" is part of the trigger event and is
        // decided from the turn's event history when the event is matched
        // (`triggers::check::first_time_this_turn_event`), not by how often
        // this ability has triggered (CR 603.2, 603.2d).
        Condition::FirstTimeThisTurn => Ok(true),
        Condition::SourceFirstCrewedThisTurn => {
            let Some(ctx) = ctx.external() else {
                return Ok(true);
            };
            Ok(source_first_crewed_this_turn(
                game,
                ctx.source,
                ctx.triggering_event,
            ))
        }
        // CR 603.2h: "Do this only once each turn" abilities trigger only if
        // the action hasn't been taken yet this turn when the event happens
        // (checked at match time, when the trigger identity is known). It
        // never stops a triggered ability from resolving; the optional
        // instruction is gated by the resolution's `DoThisLimit` instead.
        Condition::DoThisMaxTimesEachTurn(limit) => {
            let Some(ctx) = ctx.external() else {
                return Ok(true);
            };
            Ok(ctx
                .trigger_identity
                .map(|id| game.do_this_action_count_this_turn(ctx.source, id) < *limit)
                .unwrap_or(true))
        }
        Condition::MaxTimesEachTurn(limit) => {
            let Some(ctx) = ctx.external() else {
                return Ok(true);
            };
            Ok(ctx
                .trigger_identity
                .map(|id| game.trigger_fire_count_this_turn(ctx.source, id) < *limit)
                .unwrap_or(true))
        }
        Condition::TriggeringEventCausedBy { controller, effect_like_only } => Ok(shared.triggering_event
            .and_then(|event| event.cause()).is_some_and(|cause|
                (!*effect_like_only || (cause.cause_type.is_effect_like() && cause.source.is_some()))
                    && cause.source_controller.is_some_and(|actor|
                        crate::filter::player_filter_matches_game(controller, actor, game, &ctx.filter_context(game))))),
        Condition::TriggeringObjectWasEnchanted => Ok(shared
            .triggering_event
            .and_then(|event| event.snapshot())
            .is_some_and(|snapshot| snapshot.was_enchanted)),
        Condition::TriggeringObjectBecameTappedFirstTimeThisTurn => Ok(
            triggering_object_became_tapped_first_time_this_turn(game, shared.triggering_event),
        ),
        Condition::TriggeringObjectHadCountersPutFirstTimeThisTurn => Ok(
            triggering_object_had_counters_put_first_time_this_turn(game, shared.triggering_event),
        ),
        Condition::TriggeringObjectHadToAttackThisCombat => Ok(
            triggering_object_had_to_attack_this_combat(game, shared.triggering_event),
        ),
        Condition::YouWonTriggeringClash => Ok(you_won_triggering_clash(
            shared.triggering_event,
            shared.controller,
        )),
        Condition::TriggeringObjectEnteredTransformed => Ok(triggering_object_entered_transformed(
            game,
            shared.triggering_event,
        )),
        Condition::TriggeringAbilityManaSpentToActivateAtLeast(amount) => Ok(shared
            .triggering_event
            .and_then(|event| event.downcast::<crate::events::AbilityActivatedEvent>())
            .is_some_and(|activation| activation.mana_spent_total >= *amount)),
        Condition::SourceCaseSolved => Ok(game.is_case_solved(shared.source)),
        Condition::SourceClassLevelAtLeast(level) => {
            Ok(game.class_level(shared.source) >= *level)
        }
        Condition::SoulbondPairingPossible => Ok(
            crate::effects::permanents::soulbond_pairing_possible(
                game,
                shared.source,
                shared.controller,
                shared.triggering_event,
            ),
        ),
        Condition::EvolveEnteringCreatureIsLarger => Ok(shared.triggering_event.is_some_and(
            |event| {
                crate::effects::permanents::evolve_entering_creature_is_larger(
                    game,
                    shared.source,
                    event,
                )
            },
        )),
        Condition::TriggeringObjectHadCounters {
            counter_type,
            min_count,
        } => {
            // In a triggered ability "it" is the triggering object. A spell or
            // activated ability has no triggering event: "destroy target
            // creature; if that creature had a counter on it" reads the
            // target's last known information, and "exile this: ... if it had
            // seven or more counters" reads the source's (CR 608.2h).
            let had = |snapshot: &crate::snapshot::ObjectSnapshot| {
                snapshot.counters.get(counter_type).copied().unwrap_or(0) >= *min_count
            };
            if let Some(event) = shared.triggering_event {
                return Ok(event.snapshot().is_some_and(had));
            }
            let Some(execution) = ctx.execution() else {
                return Ok(false);
            };
            let target_snapshot = execution.targets.iter().find_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => game
                    .object(*id)
                    .filter(|object| object.zone == Zone::Battlefield)
                    .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game))
                    .or_else(|| {
                        crate::effects::helpers::latest_zone_change_snapshot_for_object(game, *id)
                    })
                    .or_else(|| execution.target_snapshots.get(id).cloned()),
                crate::effects::ResolvedTarget::Player(_) => None,
            });
            if let Some(snapshot) = target_snapshot {
                return Ok(had(&snapshot));
            }
            let source_snapshot = execution.source_snapshot.clone().or_else(|| {
                game.object(execution.source)
                    .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game))
            });
            Ok(source_snapshot.as_ref().is_some_and(had))
        }
        Condition::ControlCreaturesTotalPowerAtLeast(required_power) => {
            if ctx.is_cast_time() {
                return Ok(false);
            }

            let total_power = game
                .battlefield
                .iter()
                .copied()
                .filter(|&id| {
                    game.object(id).is_some_and(|obj| {
                        game.controller_of(obj) == ctx.controller && game.current_is_creature(id)
                    })
                })
                .map(|id| game.current_power(id).unwrap_or(0).max(0))
                .sum::<i32>();
            Ok(total_power >= *required_power as i32)
        }
        Condition::CardInYourGraveyard {
            card_types,
            subtypes,
        } => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game.player(ctx.controller).is_some_and(|player_state| {
                player_state.graveyard.iter().any(|&card_id| {
                    if game.object(card_id).is_none() {
                        return false;
                    }
                    let card_type_match = card_types.is_empty()
                        || card_types
                            .iter()
                            .any(|card_type| game.current_has_card_type(card_id, *card_type));
                    let subtype_match = subtypes.is_empty()
                        || subtypes
                            .iter()
                            .any(|subtype| game.current_has_subtype(card_id, *subtype));
                    card_type_match && subtype_match
                })
            }))
        }
        Condition::ActivationTiming(timing) => {
            let Some(ctx) = ctx.external() else {
                return Ok(false);
            };
            Ok({
                if ctx.options.ignore_timing {
                    return Ok(true);
                }
                match timing {
                    crate::ability::ActivationTiming::AnyTime
                    | crate::ability::ActivationTiming::AsInstant => true,
                    crate::ability::ActivationTiming::DuringCombat => {
                        matches!(game.turn.phase, crate::game_state::Phase::Combat)
                    }
                    crate::ability::ActivationTiming::SorcerySpeed => {
                        game.is_active_player(ctx.controller)
                            && matches!(
                                game.turn.phase,
                                crate::game_state::Phase::FirstMain
                                    | crate::game_state::Phase::NextMain
                            )
                            && game.stack_is_empty()
                    }
                    crate::ability::ActivationTiming::OncePerTurn => {
                        let Some(ability_index) = ctx.ability_index else {
                            return Ok(false);
                        };
                        game.ability_activation_count_this_turn(ctx.source, ability_index) == 0
                    }
                    crate::ability::ActivationTiming::DuringYourTurn => {
                        game.is_active_player(ctx.controller)
                    }
                    crate::ability::ActivationTiming::DuringOpponentsTurn => {
                        !game.is_active_player(ctx.controller)
                    }
                    crate::ability::ActivationTiming::AnyTimeByEnchantedCreatureController => {
                        game.object(ctx.source).and_then(|object| object.attached_to).and_then(|target| target.object_id())
                            .is_some_and(|host| game.object(host).is_some_and(|object| object.zone == crate::Zone::Battlefield)
                                && game.current_has_card_type(host, crate::CardType::Creature)
                                && game.current_controller(host) == Some(ctx.controller))
                    }
                    crate::ability::ActivationTiming::AnyPlayerDuringTheirTurnBeforeEndStep => {
                        game.is_active_player(ctx.controller)
                            && game.turn.phase != crate::game_state::Phase::Ending
                    }
                    crate::ability::ActivationTiming::DuringSourceOwnersUpkeep => {
                        game.object(ctx.source)
                            .is_some_and(|object| game.is_active_player(object.owner))
                            && game.turn.phase == crate::game_state::Phase::Beginning
                            && game.turn.step == Some(crate::game_state::Step::Upkeep)
                    }
                    crate::ability::ActivationTiming::DuringYourUpkeep => {
                        game.is_active_player(ctx.controller)
                            && game.turn.phase == crate::game_state::Phase::Beginning
                            && game.turn.step == Some(crate::game_state::Step::Upkeep)
                    }
                    crate::ability::ActivationTiming::DuringOpponentsUpkeep => {
                        !game.is_active_player(ctx.controller)
                            && game.turn.phase == crate::game_state::Phase::Beginning
                            && game.turn.step == Some(crate::game_state::Step::Upkeep)
                    }
                    crate::ability::ActivationTiming::DuringAnyUpkeep => {
                        game.turn.phase == crate::game_state::Phase::Beginning
                            && game.turn.step == Some(crate::game_state::Step::Upkeep)
                    }
                    timing => crate::decision::activation_step_window_allows(
                        game,
                        ctx.controller,
                        *timing,
                    ),
                }
            })
        }
        Condition::MaxActivationsPerTurn(limit) => {
            let Some(ctx) = ctx.external() else {
                return Ok(false);
            };
            Ok({
                if ctx.options.ignore_activation_limits {
                    return Ok(true);
                }
                let Some(ability_index) = ctx.ability_index else {
                    return Ok(false);
                };
                let limit = if boast_may_be_activated_an_additional_time(
                    game,
                    ctx.source,
                    ability_index,
                ) {
                    limit.saturating_add(1)
                } else {
                    *limit
                };
                game.ability_activation_count_this_turn(ctx.source, ability_index) < limit
            })
        }
        Condition::MaxActivationsPerObject(limit) => {
            let Some(ctx) = ctx.external() else {
                return Ok(false);
            };
            Ok({
                if ctx.options.ignore_activation_limits {
                    return Ok(true);
                }
                let Some(ability_index) = ctx.ability_index else {
                    return Ok(false);
                };
                let Some(origin) = game.current_characteristics(ctx.source)
                    .and_then(|chars| chars.abilities.origin(ability_index).cloned()) else {
                    return Ok(false);
                };
                game.turn_store.ability_activations_per_object.get(&(ctx.source, origin)).copied().unwrap_or(0) < *limit
            })
        }
        Condition::SourceIsEquipped => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game.object(ctx.source).is_some_and(|source_obj| {
                source_obj.attachments.iter().any(|id| {
                    game.object(*id)
                        .is_some_and(|obj| obj.subtypes.contains(&crate::types::Subtype::Equipment))
                })
            }))
        }
        Condition::SourceIsEnchanted => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game.object(ctx.source).is_some_and(|source_obj| {
                source_obj.attachments.iter().any(|id| {
                    game.object(*id)
                        .is_some_and(|obj| obj.subtypes.contains(&crate::types::Subtype::Aura))
                })
            }))
        }
        Condition::EnchantedPermanentIsCreature => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game
                .object(ctx.source)
                .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
                .is_some_and(|attached| {
                    game.object_has_card_type(attached, crate::types::CardType::Creature)
                }))
        }
        Condition::EnchantedPermanentIsLand => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game
                .object(ctx.source)
                .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
                .is_some_and(|attached| {
                    game.object_has_card_type(attached, crate::types::CardType::Land)
                }))
        }
        Condition::EnchantedPermanentIsEquipment => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game
                .object(ctx.source)
                .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
                .is_some_and(|attached| {
                    game.calculated_subtypes(attached)
                        .contains(&crate::types::Subtype::Equipment)
                }))
        }
        Condition::EnchantedPermanentIsVehicle => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game
                .object(ctx.source)
                .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
                .is_some_and(|attached| {
                    game.calculated_subtypes(attached)
                        .contains(&crate::types::Subtype::Vehicle)
                }))
        }
        Condition::EquippedCreatureTapped => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game
                .object(ctx.source)
                .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
                .is_some_and(|attached| game.is_tapped(attached)))
        }
        Condition::EquippedCreatureUntapped => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game
                .object(ctx.source)
                .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
                .is_some_and(|attached| !game.is_tapped(attached)))
        }
        Condition::EquippedCreatureAttacking => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game
                .object(ctx.source)
                .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
                .is_some_and(|attached| {
                    game.combat
                        .as_ref()
                        .is_some_and(|combat| crate::combat_state::is_attacking(combat, attached))
                }))
        }
        Condition::SourceChosenOption(expected) => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game
                .chosen_named_option(ctx.source)
                .is_some_and(|chosen| chosen.eq_ignore_ascii_case(expected)))
        }
        Condition::SecretChoicesMatch => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };
            Ok(ctx
                .secret_choice_results
                .get(&ctx.source)
                .is_some_and(|result| result.choices_match()))
        }
        Condition::VoteOptionGetsMoreVotes(option) => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };
            Ok(ctx
                .vote_results
                .get(&ctx.source)
                .is_some_and(|result| result.option_gets_more_votes(option)))
        }
        Condition::VoteOptionGetsMoreVotesOrTied(option) => {
            let Some(ctx) = ctx.execution() else {
                return Ok(false);
            };
            Ok(ctx
                .vote_results
                .get(&ctx.source)
                .is_some_and(|result| result.option_gets_more_votes_or_tied(option)))
        }
        Condition::CountComparison {
            count, comparison, ..
        } => {
            if ctx.is_cast_time() {
                let crate::static_abilities::AnthemCountExpression::MatchingFilter(filter) = count else { return Ok(false); };
                if !matches!(filter.zone, None | Some(Zone::Battlefield)) { return Ok(false); }
                let filter_ctx = game.filter_context_for(ctx.controller, Some(ctx.source));
                let exact = game.battlefield.iter().filter(|id| !game.is_phased_out(**id))
                    .filter_map(|id| game.object(*id))
                    .filter(|object| filter.matches(object, &filter_ctx, game)).count();
                let value = crate::events::damage::checked_scalar_count(exact as u128, "cast-time battlefield count")?;
                return Ok(comparison.evaluate(value));
            }
            Ok(
                comparison.evaluate(crate::static_abilities::resolve_anthem_count_expression_checked(
                    count,
                    game,
                    ctx.source,
                    ctx.controller,
                )?),
            )
        }
        Condition::CountParity { count, even, .. } => {
            if ctx.is_cast_time() {
                return Ok(false);
            }

            let value = crate::static_abilities::resolve_anthem_count_expression_checked(
                count,
                game,
                ctx.source,
                ctx.controller,
            )?;
            Ok(value % 2 == if *even { 0 } else { 1 })
        }
        Condition::ValueComparison {
            left,
            operator,
            right,
        } => {
            if let Some(exec) = ctx.execution() {
                let compare = |exec: &ExecutionContext| -> Result<bool, ExecutionError> {
                    compare_resolved_values(game, left, *operator, right, exec)
                };
                match compare(exec) {
                    // "unless an opponent has 10 or less life": a quantified
                    // opponent in a condition is satisfied by any opponent.
                    Err(ExecutionError::UnresolvableValue(message))
                        if message == crate::effects::helpers::AN_OPPONENT_CHOICE_REQUIRED =>
                    {
                        for opponent in
                            crate::effects::helpers::an_opponent_choice_candidates(game, exec)
                        {
                            let probe = an_opponent_probe_context(exec, opponent);
                            if compare(&probe)? {
                                return Ok(true);
                            }
                        }
                        Ok(false)
                    }
                    other => other,
                }
            } else {
                let external = ctx.external();
                Ok(evaluate_value_comparison(
                    game,
                    ctx.controller,
                    ctx.source,
                    left,
                    *operator,
                    right,
                    shared.triggering_event,
                    external.and_then(|c| c.defending_player),
                    external.and_then(|c| c.attacking_player),
                    external.and_then(|c| c.iterated_player),
                    (
                        external.and_then(|c| c.trigger_identity),
                        external.and_then(|c| c.ability_index),
                    ),
                ))
            }
        }
        Condition::ValueIsPrime(value) => {
            if let Some(exec) = ctx.execution() {
                Ok(is_prime_integer(resolve_value(game, value, exec)?))
            } else {
                Ok(evaluate_value_is_prime(
                    game,
                    ctx.controller,
                    ctx.source,
                    value,
                    shared.triggering_event,
                ))
            }
        }
        Condition::OwnsCardExiledWithCounter(counter) => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game.exile.iter().any(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.owner == ctx.controller
                        && obj.counters.get(counter).copied().unwrap_or(0) > 0
                })
            }))
        }
        Condition::SourceAttackedThisTurn => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game.creature_attacked_this_turn(ctx.source))
        }
        Condition::SourceAttackedBattleThisTurn => {
            if ctx.is_cast_time() {
                return Ok(false);
            }

            Ok(game.creature_attacked_battle_this_turn(ctx.source))
        }
        Condition::SourceSuspected => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game.is_suspected(ctx.source))
        }
        Condition::SourceDealtCombatDamageToPlayerThisTurn => {
            Ok(game.source_dealt_combat_damage_to_player_this_turn(shared.source))
        }
        Condition::SourceCameUnderYourControlSinceYourLastUpkeep => {
            Ok(game.object(shared.source).is_some_and(|obj| {
                obj.zone == crate::zone::Zone::Battlefield && !game.is_phased_out(obj.id)
                    && game.controller_of(obj) == shared.controller
                    && game
                        .turn_store
                        .echo_eligible_this_upkeep
                        .contains(&obj.id)
            }))
        }
        Condition::SourceCameUnderYourControlThisTurn => {
            Ok(game.object(shared.source).is_some_and(|obj| {
                game.turn_store
                    .turn_history
                    .object_came_under_controller_this_turn(obj.stable_id, shared.controller)
            }))
        }
        Condition::SourceIsUntapped => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(!game.is_tapped(ctx.source))
        }
        Condition::SourceIsAttacking => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game
                .combat
                .as_ref()
                .is_some_and(|combat| crate::combat_state::is_attacking(combat, ctx.source)))
        }
        Condition::SourceIsBlocking => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game
                .combat
                .as_ref()
                .is_some_and(|combat| crate::combat_state::is_blocking(combat, ctx.source)))
        }
        Condition::SourceIsSoulbondPaired => {
            if ctx.is_cast_time() {
                return Ok(false);
            }
            Ok(game.is_soulbond_paired(ctx.source))
        }
        Condition::SourceSoulbondPartnerMatches(filter) => Ok(game
            .soulbond_partner(ctx.source)
            .and_then(|id| game.object(id))
            .is_some_and(|partner| {
                let filter_ctx = if let Some(exec) = ctx.execution() {
                    exec.filter_context(game)
                } else {
                    FilterContext::new(ctx.controller).with_source(ctx.source)
                };
                filter.matches(partner, &filter_ctx, game)
            })),
        Condition::TurnHistory(condition) => {
            Ok(evaluate_turn_history_condition(game, condition, shared))
        }
        Condition::AllTargetsStillLegal => {
            Ok(ctx.execution().is_none_or(|exec| exec.all_targets_legal))
        }
        Condition::XValueAtLeast(min) => Ok(if let Some(exec) = ctx.execution() {
            exec.x_value.unwrap_or(0) >= *min
        } else if ctx.external().is_some() || ctx.is_cast_time() {
            // X is announced (CR 601.2b) before targets and divisions
            // (CR 601.2c-d), so cast-time choices can already read it.
            game.object(ctx.source)
                .and_then(|object| object.x_value)
                .unwrap_or(0)
                >= *min
        } else {
            false
        }),
        Condition::Custom(_) => Ok(false),
        Condition::LifeTotalOrLess(threshold) => Ok(game
            .player(shared.controller)
            .map(|p| p.life <= *threshold)
            .unwrap_or(false)),
        Condition::LifeTotalOrGreater(threshold) => Ok(game
            .player(shared.controller)
            .map(|p| p.life >= *threshold)
            .unwrap_or(false)),
        Condition::CardsInHandOrMore(threshold) => Ok(game
            .player(shared.controller)
            .map(|p| p.hand.len() as i32 >= *threshold)
            .unwrap_or(false)),
        Condition::YouHaveCardInHandMatching(filter) => Ok(player_has_card_in_hand_matching(
            game,
            shared.controller,
            filter,
            shared.filter_source,
        )),
        Condition::YourTurn => Ok(game.is_active_player(shared.controller)),
        Condition::CurrentTurnIsExtra => Ok(game.turn_store.current_turn_is_extra),
        Condition::SourceControllersMainPhase => Ok(game.is_active_player(shared.controller)
            && matches!(
                game.turn.phase,
                crate::game_state::Phase::FirstMain | crate::game_state::Phase::NextMain
            )),
        Condition::SourceControllersCombatPhase => Ok(game.is_active_player(shared.controller)
            && matches!(game.turn.phase, crate::game_state::Phase::Combat)),
        Condition::SourceControllersEndStep => Ok(game.is_active_player(shared.controller)
            && game.turn.phase == crate::game_state::Phase::Ending),
        Condition::SourceIsRenowned => Ok(game.is_renowned(shared.source)),
        Condition::YourFirstTurnsOfTheGameOrFewer(count) => {
            Ok(game.is_active_player(shared.controller)
                && game.turns_taken_by(shared.controller) <= *count)
        }
        Condition::CreatureDiedThisTurn => Ok(game
            .turn_store
            .turn_history
            .total_creatures_died_this_turn()
            > 0),
        Condition::CastSpellThisTurn => {
            Ok(game.turn_store.turn_history.any_spell_was_cast_this_turn())
        }
        Condition::AttackedThisTurn => Ok(game
            .turn_store
            .turn_history
            .players_attacked_this_turn
            .contains(&shared.controller)),
        Condition::AttackedWithNOrMoreCreaturesThisTurn(count) => Ok(game
            .turn_store
            .turn_history
            .creatures_attacked_by_player_this_turn
            .get(&shared.controller)
            .map_or(0, |creatures| creatures.len()) as u32
            >= *count),
        Condition::AttackedWithTotalPowerAtLeastThisCombat(power) => Ok(
            game.turn.phase == crate::game_state::Phase::Combat
                && game.turn_store.turn_history.declared_attack_power_in_combat(
                    game.turn_store.combat_phases_started_this_turn,
                    shared.controller,
                ) >= i64::from(*power),
        ),
        Condition::OpponentLostLifeThisTurn => {
            let filter_ctx = game.filter_context_for(shared.controller, shared.filter_source);
            Ok(filter_ctx.opponents.iter().any(|opponent| {
                game.turn_store
                    .turn_history
                    .player_lost_life_this_turn(*opponent)
            }))
        }
        Condition::AnyPlayerLostLifeThisTurnOrMore { count } => {
            Ok(game.players.iter().any(|player| {
                player.is_in_game()
                    && game
                        .turn_store
                        .turn_history
                        .total_life_lost_for_players(&[player.id])
                        >= *count
            }))
        }
        Condition::OpponentWasDealtDamageThisTurn => {
            let filter_ctx = game.filter_context_for(shared.controller, shared.filter_source);
            Ok(filter_ctx.opponents.iter().any(|opponent| {
                game.turn_store
                    .turn_history
                    .player_was_dealt_damage_this_turn(*opponent)
            }))
        }
        Condition::OpponentWasDealtDamageThisTurnOrMore(count) => {
            let filter_ctx = game.filter_context_for(shared.controller, shared.filter_source);
            Ok(filter_ctx.opponents.iter().any(|opponent| {
                game.turn_store
                    .turn_history
                    .total_damage_to_player(*opponent)
                    >= *count
            }))
        }
        Condition::PermanentLeftBattlefieldThisTurn => Ok(game
            .turn_store
            .turn_history
            .permanents_left_battlefield_this_turn()
            > 0),
        Condition::NonlandPermanentLeftBattlefieldThisTurn => Ok(game
            .turn_store
            .turn_history
            .nonland_permanents_left_battlefield_this_turn()
            > 0),
        Condition::SpellWasWarpedThisTurn => {
            Ok(game.turn_store.turn_history.spell_was_warped_this_turn())
        }
        Condition::PermanentLeftBattlefieldUnderYourControlThisTurn { .. } => Ok(game
            .turn_store
            .turn_history
            .permanents_left_battlefield_under_controller(shared.controller)
            > 0),
        Condition::ObjectEnteredBattlefieldThisTurn(filter) => Ok(
            object_matching_entered_battlefield_this_turn(game, shared, filter),
        ),
        Condition::ObjectEnteredBattlefieldLastTurn(filter) => Ok(
            object_matching_entered_battlefield_last_turn(game, shared, filter),
        ),
        Condition::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter) => Ok(
            object_matching_was_put_into_graveyard_from_battlefield_this_turn(game, shared, filter),
        ),
        Condition::SourceWasCast => Ok(source_was_cast(
            game,
            shared.source,
            shared.triggering_event,
        )),
        Condition::NoSpellsWereCastLastTurn => Ok(game.turn_store.spells_cast_last_turn_total == 0),
        Condition::SpellsWereCastLastTurnOrMore(count) => {
            Ok(game.turn_store.spells_cast_last_turn_total >= *count)
        }
        Condition::SourceHasNoCounter(counter_type) => Ok(game
            .object(shared.source)
            .map(|obj| obj.counters.get(counter_type).copied().unwrap_or(0) == 0)
            .unwrap_or(false)),
        Condition::SourceHasCounterAtLeast {
            counter_type,
            count,
            ..
        } => Ok(game
            .object(shared.source)
            .map(|obj| obj.counters.get(counter_type).copied().unwrap_or(0) >= *count)
            .unwrap_or(false)),
        Condition::SourceInGraveyardWithCardsAbove { filter, count } => {
            Ok(game.object(shared.source).is_some_and(|source| {
                if source.zone != crate::zone::Zone::Graveyard {
                    return false;
                }
                let Some(graveyard) = game.player(source.owner).map(|player| &player.graveyard)
                else {
                    return false;
                };
                let Some(source_index) = graveyard.iter().position(|id| *id == shared.source)
                else {
                    return false;
                };
                let filter_ctx = game.filter_context_for(shared.controller, Some(shared.source));
                graveyard[source_index + 1..]
                    .iter()
                    .filter(|id| {
                        game.object(**id)
                            .is_some_and(|object| filter.matches(object, &filter_ctx, game))
                    })
                    .count()
                    >= *count as usize
            }))
        }
        Condition::SourceIsInZone(zone) => Ok(game
            .object(shared.source)
            .map(|obj| obj.zone == *zone && !game.is_phased_out(obj.id))
            .unwrap_or(false)),
        Condition::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
            let Some(source_obj) = game.object(shared.source) else {
                return Ok(false);
            };
            Ok(mana_pool_amount(&source_obj.mana_spent_to_cast, *symbol) >= *amount)
        }
Condition::SnowManaOfAnySpellColorSpentToCastThisSpell => {
            Ok(game.object(shared.source).is_some_and(|object| {
                let snapshot =
                    crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        object, game,
                    );
                matching_snow_mana_was_spent(&snapshot)
            }))
        }
Condition::TriggeringSpellSnowManaOfAnySpellColorSpentToCast => {
            Ok(shared.triggering_event
                .and_then(|event| event.downcast::<crate::events::SpellCastEvent>())
                .is_some_and(|cast| {
                    game.object(cast.spell).filter(|object| object.zone == crate::zone::Zone::Stack)
                        .map_or_else(|| cast.snapshot.as_ref().is_some_and(matching_snow_mana_was_spent), |object| {
                            matching_snow_mana_was_spent(&crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(object, game))
                        })
                }))
        },
        Condition::SameColorManaSpentToCastThisSpellAtLeast(amount) => {
            let Some(source_obj) = game.object(shared.source) else {
                return Ok(false);
            };
            let spent = &source_obj.mana_spent_to_cast;
            let most_spent_of_one_color =
                [spent.white, spent.blue, spent.black, spent.red, spent.green]
                    .into_iter()
                    .max()
                    .unwrap_or(0);
            Ok(most_spent_of_one_color >= *amount)
        }
        Condition::ColorsOfManaSpentToCastThisSpellOrMore(amount) => {
            let Some(source_obj) = game.object(shared.source) else {
                return Ok(false);
            };
            let spent = &source_obj.mana_spent_to_cast;
            let distinct_colors = [
                spent.white > 0,
                spent.blue > 0,
                spent.black > 0,
                spent.red > 0,
                spent.green > 0,
            ]
            .into_iter()
            .filter(|present| *present)
            .count() as u32;
            Ok(distinct_colors >= *amount)
        }
        Condition::PlayerGraveyardHasCardsAtLeast { player, count } => Ok(game
            .player(*player)
            .is_some_and(|p| p.graveyard.len() >= *count)),
        Condition::YouChoseAnotherRingBearer => Ok(shared.triggering_event
            .and_then(|event| event.downcast::<crate::events::KeywordActionEvent>())
            .filter(|event| event.action == crate::events::KeywordActionKind::RingTemptsYou && event.player == shared.controller)
            .and_then(|event| event.object_tags.get(ironsmith_core::tag::RING_BEARER_CHOSEN_TAG))
            .is_some_and(|chosen| matches!(chosen.as_slice(), [object] if object.object_id != shared.source))),
        Condition::SourceIsRingBearer { player } => {
            // The designation survives phasing, but the permanent is treated
            // as nonexistent for this current-state predicate (CR 702.26b).
            if game.is_phased_out(shared.source) { return Ok(false); }
            Ok(
                matching_condition_players_simple(game, shared.controller, player)
                    .into_iter()
                    .any(|player_id| game.current_ring_bearer(player_id) == Some(shared.source)),
            )
        }
        Condition::PlayerRingTemptedThisGameOrMore { player, count } => Ok(
            matching_condition_players_simple(game, shared.controller, player)
                .into_iter()
                .any(|player_id| game.ring_temptations(player_id) >= *count),
        ),
        Condition::PlayerRemovedDraftCardMatching {
            player,
            filter,
            with_cards_named,
        } => Ok(
            matching_condition_players_simple(game, shared.controller, player)
                .into_iter()
                .any(|player_id| {
                    game.removed_from_draft_card_matches(
                        player_id,
                        with_cards_named,
                        filter,
                        shared.filter_source,
                    )
                }),
        ),
        Condition::YouControlCommander => {
            if let Some(player) = game.player(shared.controller) {
                let commanders = player.get_commanders();
                for &commander_id in commanders {
                    if game.battlefield.contains(&commander_id)
                        && let Some(obj) = game.object(commander_id)
                        && game.controller_of(obj) == shared.controller
                    {
                        return Ok(true);
                    }
                    for &bf_id in &game.battlefield {
                        if let Some(obj) = game.object(bf_id)
                            && game.controller_of(obj) == shared.controller
                            && obj.stable_id == StableId::from(commander_id)
                        {
                            return Ok(true);
                        }
                    }
                }
            }
            Ok(false)
        }
        Condition::ThisAbilityResolvedThisTurnExactly(count) => {
            Ok(if let Some(ability_index) = shared.ability_index {
                game.activated_ability_resolution_count_this_turn(shared.source, ability_index)
                    == *count
            } else {
                shared.trigger_identity.is_some_and(|trigger_identity| {
                    game.triggered_ability_resolution_count_this_turn(
                        shared.source,
                        trigger_identity,
                    ) == *count
                })
            })
        }
        Condition::Not(inner) => Ok(!evaluate_condition_in_context(game, inner, ctx)?),
        Condition::And(a, b) => Ok(evaluate_condition_in_context(game, a, ctx)?
            && evaluate_condition_in_context(game, b, ctx)?),
        Condition::Or(a, b) => Ok(evaluate_condition_in_context(game, a, ctx)?
            || evaluate_condition_in_context(game, b, ctx)?),
    }
}
#[cfg(test)]
mod context_tests;

/// Birgi, God of Storytelling: "Creatures you control can boast twice during
/// each of your turns rather than once." Raises the CR 702.142a once-per-turn
/// cap for a boast ability whose controller is the active player and controls
/// such a permanent.
fn boast_may_be_activated_an_additional_time(
    game: &GameState,
    source: crate::ids::ObjectId,
    ability_index: usize,
) -> bool {
    let Some(chars) = game.current_characteristics(source) else {
        return false;
    };
    let is_boast = chars
        .abilities
        .as_slice()
        .get(ability_index)
        .is_some_and(|ability| {
            matches!(
                &ability.kind,
                crate::ability::AbilityKind::Activated(activated)
                    if activated.additional_restrictions.iter().any(|restriction| {
                        restriction
                            .strip_prefix("__ironsmith_activation_label:")
                            .is_some_and(|label| label.eq_ignore_ascii_case("Boast"))
                    })
            )
        });
    if !is_boast || !chars.card_types.contains(&crate::types::CardType::Creature) {
        return false;
    }
    let Some(controller) = game.object(source).map(|object| game.controller_of(object)) else {
        return false;
    };
    game.is_active_player(controller)
        && game.battlefield.iter().any(|&id| {
            game.object(id)
                .is_some_and(|object| game.controller_of(object) == controller)
                && game.current_has_static_ability_id(
                    id,
                    crate::static_abilities::StaticAbilityId::BoastTwiceEachTurn,
                )
        })
}

#[cfg(test)]
mod half_starting_life_boundary_tests {
    use super::*;
    #[test]
    fn half_starting_life_compares_before_rounding_without_saturating_doubling() {
        let mut game = GameState::new(vec!["Alice".into()], 41);
        let player = game.players[0].id;
        for (starting, life, strict, inclusive) in [
            (41, 20, true, true),
            (41, 21, false, false),
            (40, 20, false, true),
            (i32::MAX, i32::MAX, false, false),
            (i32::MIN, i32::MIN, true, true),
            (-3, -2, true, true),
            (-3, -1, false, false),
        ] {
            game.player_mut(player).unwrap().starting_life = starting;
            game.write_life_total(player, life);
            assert_eq!(
                player_life_compares_to_half_starting(&game, player, false),
                strict
            );
            assert_eq!(
                player_life_compares_to_half_starting(&game, player, true),
                inclusive
            );
        }
    }

    #[test]
    fn life_comparisons_widen_intermediates_without_clamping_the_predicate() {
        use crate::effect::ValueComparisonOperator as Op;
        let game = GameState::new(vec!["Alice".into()], 20);
        let ctx = ExecutionContext::new_default(ObjectId::from_raw(999), game.players[0].id);
        let difference = Value::absolute_difference(Value::Fixed(i32::MIN), Value::Fixed(i32::MAX));
        assert!(
            !compare_resolved_values(
                &game,
                &difference,
                Op::LessThanOrEqual,
                &Value::Fixed(5),
                &ctx
            )
            .unwrap()
        );
        let upper_threshold =
            Value::Add(Box::new(Value::Fixed(i32::MAX)), Box::new(Value::Fixed(10)));
        assert!(
            !compare_resolved_values(
                &game,
                &Value::Fixed(i32::MAX),
                Op::GreaterThanOrEqual,
                &upper_threshold,
                &ctx
            )
            .unwrap()
        );
        let negative = Value::HalfRoundedDown(Box::new(Value::Fixed(-3)));
        assert!(
            compare_resolved_values(&game, &negative, Op::Equal, &Value::Fixed(-2), &ctx).unwrap()
        );
    }
}

#[cfg(test)]
mod referenced_characteristic_frame_tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::EventCause;
    use crate::filter::{FilterContext, ObjectFilter, ObjectFilterExt as _};
    use crate::ids::CardId;
    use crate::provenance::ProvNodeId;
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;
    fn creature(game: &mut GameState, player: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), "Characteristic subject")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_card(&card, player, Zone::Battlefield)
    }
    #[test]
    fn a_current_recheck_cannot_be_satisfied_by_an_earlier_snapshot_or_a_blinked_incarnation() {
        let mut game = GameState::new(vec!["Alice".into()], 20);
        let a = PlayerId::from_index(0);
        let subject = creature(&mut game, a);
        let before = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(subject).unwrap(),
            &game,
        );
        let mut change = crate::events::ZoneChangeEvent::with_cause(
            subject,
            Zone::Hand,
            Zone::Battlefield,
            EventCause::effect(),
            Some(before.clone()),
        );
        change.destination_snapshots = vec![before.clone()];
        let event = TriggerEvent::new_with_provenance(change, ProvNodeId::default());
        let filter = ObjectFilter {
            power: Some(crate::filter::Comparison::Equal(1)),
            toughness: Some(crate::filter::Comparison::Equal(1)),
            ..Default::default()
        };
        let context = FilterContext::new(a);
        assert!(triggering_event_object_matches_at_resolution(
            &game, &event, &filter, &context
        ));
        game.add_counters(subject, crate::object::CounterType::PlusOnePlusOne, 1)
            .unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(
            triggering_event_object_matches_with_filter_context(&game, &event, &filter, &context),
            "trigger-time snapshot remains 1/1"
        );
        assert!(
            !triggering_event_object_matches_at_resolution(&game, &event, &filter, &context),
            "current 2/2 fails"
        );
        let exiled = game
            .move_object(subject, Zone::Exile, EventCause::effect())
            .unwrap();
        assert!(
            !triggering_event_object_matches_at_resolution(&game, &event, &filter, &context),
            "departure LKI was 2/2"
        );
        let returned = game
            .move_object(exiled, Zone::Battlefield, EventCause::effect())
            .unwrap();
        assert_ne!(returned, subject);
        assert_eq!(game.calculated_power(returned), Some(1));
        assert!(!triggering_event_object_matches_at_resolution(
            &game, &event, &filter, &context
        ));
        assert!(
            filter.matches_snapshot(&before, &context, &game),
            "past-tense condition stays at its captured frame"
        );
    }
    #[test]
    fn effective_vs_base_comparison_uses_one_snapshot_for_growth_and_shrinkage() {
        let mut game = GameState::new(vec!["Alice".into()], 20);
        let a = PlayerId::from_index(0);
        let subject = creature(&mut game, a);
        let mut snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(subject).unwrap(),
            &game,
        );
        let filter = ObjectFilter {
            power_comparison_to_base: Some(crate::effect::ValueComparisonOperator::NotEqual),
            ..Default::default()
        };
        let context = FilterContext::new(a);
        for (power, expected) in [(0, true), (1, false), (2, true)] {
            snapshot.power = Some(power);
            snapshot.base_power = Some(1);
            assert_eq!(
                filter.matches_snapshot(&snapshot, &context, &game),
                expected
            );
        }
    }
    #[test]
    fn attached_aura_controller_comes_from_the_death_snapshot_after_both_objects_leave() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let subject = creature(&mut game, b);
        let aura_card = CardBuilder::new(CardId::new(), "Captured Aura")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![crate::types::Subtype::Aura])
            .build();
        let aura = game.create_object_from_card(&aura_card, a, Zone::Battlefield);
        assert!(
            game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(subject))
        );
        let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(subject).unwrap(),
            &game,
        );
        assert_eq!(snapshot.attachment_snapshots.len(), 1);
        let filter = ObjectFilter {
            with_attached_object: Some(Box::new(
                ObjectFilter::default()
                    .with_subtype(crate::types::Subtype::Aura)
                    .you_control(),
            )),
            ..Default::default()
        };
        let context = FilterContext::new(a);
        game.move_object(subject, Zone::Graveyard, EventCause::effect())
            .unwrap();
        if game.object(aura).is_some() {
            game.move_object(aura, Zone::Graveyard, EventCause::effect())
                .unwrap();
        }
        assert!(filter.matches_snapshot(&snapshot, &context, &game));
        assert!(!filter.matches_snapshot(&snapshot, &FilterContext::new(b), &game));
    }
}

#[cfg(test)]
mod tagged_current_and_departure_condition_tests {
    use super::*;
    use crate::events::combat::{
        AttackEventTarget, CreatureAttackedAndUnblockedEvent, CreatureAttackedEvent,
        CreatureBecameBlockedEvent, CreatureBlockedEvent,
    };
    #[test]
    fn ordinary_tags_use_live_characteristics_or_exact_departure_lki_without_combat_blink_follow() {
        for kind in 0..4 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let card = crate::cards::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                "Tagged combatant",
            )
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(15, 30))
            .build();
            let object = game.create_object_from_definition(&card, alice, Zone::Battlefield);
            let source = game.create_object_from_definition(&card, bob, Zone::Battlefield);
            let snapshot =
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    game.object(object).unwrap(),
                    &game,
                );
            let event = match kind {
                0 => TriggerEvent::new_with_provenance(
                    CreatureAttackedEvent::new(object, AttackEventTarget::Player(bob)),
                    Default::default(),
                ),
                1 => TriggerEvent::new_with_provenance(
                    CreatureAttackedAndUnblockedEvent::new(object, AttackEventTarget::Player(bob)),
                    Default::default(),
                ),
                2 => TriggerEvent::new_with_provenance(
                    CreatureBlockedEvent::new(object, source),
                    Default::default(),
                ),
                _ => TriggerEvent::new_with_provenance(
                    CreatureBecameBlockedEvent::new(object, 1),
                    Default::default(),
                ),
            };
            let mut ctx = ExecutionContext::new_default(source, bob).with_triggering_event(event);
            ctx.tag_object("combatant", snapshot);
            let condition = |power| {
                let mut filter = crate::target::ObjectFilter::creature();
                filter.power = Some(crate::filter::Comparison::Equal(power));
                Condition::TaggedObjectMatches("combatant".into(), filter)
            };
            let effect = crate::effect::Effect::pump(
                -14,
                0,
                crate::target::ChooseSpec::SpecificObject(object),
                crate::effect::Until::EndOfTurn,
            );
            crate::effects::execute_effect(
                &mut game,
                &effect,
                &mut ExecutionContext::new_default(source, bob),
            )
            .unwrap();
            assert!(
                !evaluate_condition_resolution(&game, &condition(15), &ctx).unwrap(),
                "the old matching tag cannot override current power 1"
            );
            assert!(evaluate_condition_resolution(&game, &condition(1), &ctx).unwrap());
            let exiled = game.move_object_by_game_rule(object, Zone::Exile).unwrap();
            let returned = game
                .move_object_by_game_rule(exiled, Zone::Battlefield)
                .unwrap();
            ctx.resolution_object_id_floor = Some(game.new_object_id());
            assert_ne!(returned, object);
            assert_eq!(game.calculated_power(returned), Some(15));
            assert!(
                evaluate_condition_resolution(&game, &condition(1), &ctx).unwrap(),
                "the exact departed incarnation had power 1, not its earlier tagged 15"
            );
            assert!(!evaluate_condition_resolution(&game, &condition(15), &ctx).unwrap());
            assert_eq!(
                crate::effects::helpers::resolve_tagged_object_id(
                    &game,
                    &ctx,
                    &ctx.get_tagged_all("combatant").unwrap()[0]
                ),
                None
            );
        }
    }
}
