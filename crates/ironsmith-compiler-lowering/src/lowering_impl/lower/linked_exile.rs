//! Definition-local links for executable scalar references (CR 607).
use crate::ability::AbilityKind;
use crate::cards::CardDefinition;
use crate::effect::{Effect, Value};
use crate::resolution::ResolutionProgram;
use crate::target::ChooseSpec;
use sha2::{Digest, Sha256};

fn program(ability: &crate::ability::Ability) -> Option<&ResolutionProgram> {
    match &ability.kind {
        AbilityKind::Triggered(ability) => Some(&ability.effects),
        AbilityKind::Activated(ability) => Some(&ability.effects),
        _ => None,
    }
}

fn scalar_consumer(value: &Value) -> bool {
    match value {
        Value::PowerOf(spec) | Value::ToughnessOf(spec) | Value::ManaValueOf(spec) =>
            matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == ironsmith_core::SOURCE_EXILED_TAG),
        Value::SurfaceHinted { value, .. } | Value::Scaled(value, _) |
        Value::DividedRoundedDown(value, _) | Value::HalfRoundedDown(value) => scalar_consumer(value),
        Value::Add(left, right) | Value::Min(left, right) => scalar_consumer(left) || scalar_consumer(right),
        _ => false,
    }
}

fn inspect(effect: &Effect, producers: &mut usize, compatible: &mut usize, consumes: &mut bool) {
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileUntilEffect>() {
        *producers += 1;
        if exile.duration == ironsmith_core::ExileUntilDuration::SourceLeavesBattlefield
            && exile.leave_watcher.is_none()
        {
            *compatible += 1;
        }
    } else if effect.downcast_ref::<crate::effects::ExileEffect>().is_some()
        || effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>().is_some()
        || effect.downcast_ref::<crate::effects::cards::ImprintFromHandEffect>().is_some()
        || effect.downcast_ref::<crate::effects::HauntExileEffect>().is_some()
        || effect.downcast_ref::<crate::effects::ExileTaggedWhenSourceLeavesEffect>().is_some()
        || effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|effect| effect.zone == crate::zone::Zone::Exile)
    {
        *producers += 1;
    }
    crate::compile_support::visit_direct_nested_effect_values(effect, &mut |value| {
        *consumes |= scalar_consumer(value);
    });
    effect.visit_child_effects(&mut |child| inspect(child, producers, compatible, consumes));
}

/// Stamp only a proven unambiguous executable pair. Unknown, multi-producer,
/// static, and independently authored native bodies keep absent metadata and
/// cannot use the scalar reader. Run after all program-rebuilding finalizers.
pub(super) fn bind_scalar_linked_exile(definition: &mut CardDefinition) {
    // Until reference analysis exports explicit multi-pair relationships, a
    // third ability scope (including a static producer or a second pair) is
    // not evidence that these two abilities are linked. Nonmana costs may
    // themselves exile objects and need a separate typed producer inventory.
    if definition.abilities.len() != 2 || definition.abilities.iter().any(|ability| {
        match &ability.kind {
            AbilityKind::Triggered(_) => false,
            AbilityKind::Activated(ability) => ability.mana_cost.has_non_mana_costs()
                || ability.mana_cost.dynamic_mana_cost().is_some()
                || ability.mana_cost.as_one_of().is_some(),
            _ => true,
        }
    }) { return; }
    let mut producer = None;
    let mut consumers = Vec::new();
    let mut total = 0;
    let mut compatible_total = 0;
    for (slot, ability) in definition.abilities.iter().enumerate() {
        let Some(program) = program(ability) else { continue; };
        let (mut count, mut compatible, mut consumes) = (0, 0, false);
        for effect in program.all_effects() {
            inspect(effect, &mut count, &mut compatible, &mut consumes);
        }
        total += count;
        compatible_total += compatible;
        if compatible == 1 { producer = Some(slot); }
        if consumes { consumers.push(slot); }
    }
    let Some(producer) = producer else { return; };
    if total != 1 || compatible_total != 1 || consumers.len() != 1 || consumers[0] == producer { return; }
    // Hash typed executable definitions, never a caller's local card number,
    // physical host or Debug rendering. Pair compatibility above uses only
    // typed effects and values; presentation fields never decide membership.
    let Ok(bytes) = serde_json::to_vec(&definition.abilities) else { return; };
    let pair = ironsmith_core::LinkedExilePair {
        definition: ironsmith_core::LinkedExileDefinition(Sha256::digest(bytes).into()),
        pair: producer as u32,
    };
    for (slot, ability) in definition.abilities.iter_mut().enumerate() {
        if slot != producer && !consumers.contains(&slot) { continue; }
        let effects = match &mut ability.kind {
            AbilityKind::Triggered(ability) => &mut ability.effects,
            AbilityKind::Activated(ability) => &mut ability.effects,
            _ => continue,
        };
        effects.linked_exile_pair = Some(pair);
    }
}

#[derive(Clone, Copy)]
enum StaticExileProducer { FaceUpLibrary, FaceDownLibrary, FaceDownHandChoice, FaceUpHandUntilSourceLeaves }

/// Inventory one complete typed producer, including the private selection's
/// exact dataflow. No wrapper, extra action, label or tag spelling proves a pair.
fn static_exile_producer(program: &ResolutionProgram) -> Option<StaticExileProducer> {
    if program.segments.len() != 1 || !program.segments[0].self_replacements.is_empty() { return None; }
    let effects = program.all_effects();
    if effects.len() == 1
        && let Some(exile) = effects[0].downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
    {
        return Some(if exile.face_down { StaticExileProducer::FaceDownLibrary } else { StaticExileProducer::FaceUpLibrary });
    }
    if effects.len() == 1
        && let Some(players) = crate::compile_support::effect_without_result_tags(effects[0])
            .downcast_ref::<crate::effects::ForPlayersEffect<Effect>>()
        && players.filter == crate::target::PlayerFilter::Opponent
        && !players.starting_with_controller && !players.sequential && !players.stop_after_first_happened
        && players.effects.len() == 1
        && let Some(exile) = crate::compile_support::effect_without_result_tags(&players.effects[0])
            .downcast_ref::<crate::effects::ExileUntilEffect>()
        && exile.duration == ironsmith_core::ExileUntilDuration::SourceLeavesBattlefield
        && exile.leave_watcher.is_none() && !exile.face_down && !exile.explicit_return_surface
        && exile.return_zone == crate::zone::Zone::Battlefield
        && !exile.spec.is_target() && exile.spec.count().is_single() && !exile.spec.count().is_random()
        && exile.spec.count_value().is_none()
        && let ChooseSpec::Object(filter) = exile.spec.base()
    {
        let mut plain = filter.clone();
        if plain.zone.take() == Some(crate::zone::Zone::Hand)
            && plain.owner.take() == Some(crate::target::PlayerFilter::IteratedPlayer)
            && plain == crate::target::ObjectFilter::default()
        { return Some(StaticExileProducer::FaceUpHandUntilSourceLeaves); }
    }
    if effects.len() != 2 { return None; }
    let choice = effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile = effects[1].downcast_ref::<crate::effects::ExileEffect>()?;
    if choice.zone != Some(crate::zone::Zone::Hand) || !choice.additional_zones.is_empty()
        || !choice.count.is_single() || choice.count.is_random()
        || choice.count_value.is_some() || choice.aggregate_constraint.is_some()
        || choice.is_search || choice.reveal || choice.top_only || choice.bottom_only
        || choice.remember_as_chosen_object
        || !exile.face_down || exile.source_controller_may_look || exile.turn_face_up
        || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choice.tag)
    { return None; }
    let mut filter = choice.filter.clone();
    if filter.owner.as_ref() != Some(&choice.chooser) { return None; }
    filter.owner = None;
    if filter != crate::target::ObjectFilter::default() { return None; }
    Some(StaticExileProducer::FaceDownHandChoice)
}

/// Prove one typed exile trigger and one static whole-pool play permission.
/// Leaf evasion keywords are unrelated scopes; any other ability, executable
/// producer, rider, or level/copy scope stays unbound pending typed analysis.
/// This is deliberately separate from the scalar binder's stricter contract.
pub(super) fn bind_static_linked_exile(definition: &mut CardDefinition) {
    use ironsmith_core::{Grantable, StaticAbilityId, StaticAbilityPayload};
    if definition.spell_effect.is_some() || !definition.alternative_casts.is_empty()
        || !definition.optional_costs.is_empty()
        || definition.additional_cost.has_non_mana_costs()
        || definition.additional_cost.dynamic_mana_cost().is_some()
        || definition.additional_cost.as_one_of().is_some()
    { return; }
    let mut producer = None;
    let mut consumer = None;
    for (slot, ability) in definition.abilities.iter().enumerate() {
        match &ability.kind {
            AbilityKind::Triggered(trigger) => {
                if producer.is_some() { return; }
                let Some(kind) = static_exile_producer(&trigger.effects) else { return; };
                producer = Some((slot, kind));
            }
            AbilityKind::Static(ability) => match &ability.payload {
                StaticAbilityPayload::None
                    if matches!(ability.id, Some(StaticAbilityId::Flying | StaticAbilityId::Menace)) => {}
                // These independent player rules contain no producer,
                // executable program, or source-exiled reference. Their own
                // typed runtime owners retain draw/cast behavior unchanged.
                StaticAbilityPayload::PlayerSkipsDrawStep { player: crate::target::PlayerFilter::You } => {}
                StaticAbilityPayload::RuleRestriction {
                    restriction: crate::effect::Restriction::CastMoreThanOneSpellEachTurn(crate::target::PlayerFilter::You, filter),
                    additional_restrictions, ..
                } if filter == &crate::target::ObjectFilter::default() && additional_restrictions.is_empty() => {}
                StaticAbilityPayload::Grants(spec)
                    if spec.requires_linked_exile_pair
                        && matches!(spec.grantable, Grantable::PlayFrom)
                        && spec.zone == crate::zone::Zone::Exile
                        && spec.additional_zones.is_empty()
                        && spec.beneficiary == crate::target::PlayerFilter::You
                        && spec.usage_limit.is_none() && spec.max_plays.is_none()
                        && spec.cast_this_way_grants.is_empty()
                        && spec.permanent_this_way_grants.is_empty()
                        && spec.on_use_effects.is_empty()
                        && spec.cast_this_way_filter.is_none()
                        && !spec.top_card_only && !spec.instant_timing && !spec.may_look_at_top => {
                    if consumer.is_some() { return; }
                    let mut filter = spec.filter.clone();
                    filter.zone = None;
                    if filter.tagged_constraints.len() != 1
                        || filter.tagged_constraints[0].tag.as_str() != ironsmith_core::SOURCE_EXILED_TAG
                        || filter.tagged_constraints[0].relation != crate::target::TaggedOpbjectRelation::IsTaggedObject
                    { return; }
                    filter.tagged_constraints.clear();
                    if filter != crate::target::ObjectFilter::default() { return; }
                    consumer = Some((slot, spec.may_look_at_linked_exile));
                }
                _ => return,
            },
            _ => return,
        }
    }
    let (Some((producer, producer_kind)), Some((consumer, may_look))) = (producer, consumer) else { return; };
    if matches!(producer_kind, StaticExileProducer::FaceDownLibrary | StaticExileProducer::FaceDownHandChoice) && !may_look { return; }
    if matches!(producer_kind, StaticExileProducer::FaceDownHandChoice)
        && let AbilityKind::Triggered(ability) = &mut definition.abilities[producer].kind
    {
        let mut segments = ability.effects.segments.clone();
        let Some(exile) = segments[0].default_effects[1].downcast_ref::<crate::effects::ExileEffect>() else { return; };
        let mut exile = exile.clone();
        // Choosing one's hand card is not permission to inspect the new
        // face-down exile. The separate active static reader owns entitlement.
        exile.exclude_prior_zone_viewers = true;
        segments[0].default_effects[1] = Effect::new(exile);
        ability.effects.replace_segments(segments);
    }
    let Ok(bytes) = serde_json::to_vec(&definition.abilities) else { return; };
    let pair = ironsmith_core::LinkedExilePair {
        definition: ironsmith_core::LinkedExileDefinition(Sha256::digest(bytes).into()),
        pair: producer as u32,
    };
    if let AbilityKind::Triggered(ability) = &mut definition.abilities[producer].kind {
        ability.effects.linked_exile_pair = Some(pair);
    }
    if let AbilityKind::Static(ability) = &mut definition.abilities[consumer].kind
        && let StaticAbilityPayload::Grants(spec) = &mut ability.payload
    {
        spec.linked_exile_pair = Some(pair);
    }
}


fn simple_linked_activation(ability: &crate::ability::ActivatedAbility) -> bool {
    ability.choices.is_empty()
        && ability.mana_cost.as_all().is_some_and(|costs| costs.iter().all(|cost|
            matches!(cost, crate::costs::Cost::Mana(_) | crate::costs::Cost::Tap)))
        && ability.effects.segments.len() == 1
        && ability.effects.segments[0].self_replacements.is_empty()
}

fn draw_then_private_hand_exile(program: &ResolutionProgram) -> bool {
    let effects = program.all_effects();
    if effects.len() != 3 { return false; }
    let Some(draw) = crate::compile_support::effect_without_result_tags(effects[0])
        .downcast_ref::<crate::effects::DrawCardsEffect>() else { return false; };
    if draw.count != Value::Fixed(1) || draw.player != crate::target::PlayerFilter::You { return false; }
    let hand = ResolutionProgram::from_effects(effects[1..].iter().map(|effect| (*effect).clone()).collect());
    matches!(static_exile_producer(&hand), Some(StaticExileProducer::FaceDownHandChoice))
        && effects[1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .is_some_and(|choice| choice.chooser == crate::target::PlayerFilter::You)
}

fn single_paired_return(program: &ResolutionProgram) -> bool {
    let effects = program.all_effects();
    if effects.len() != 1 { return false; }
    let Some(returned) = crate::compile_support::effect_without_result_tags(effects[0])
        .downcast_ref::<crate::effects::ReturnToHandEffect>() else { return false; };
    if returned.spec.is_target() || !returned.spec.count().is_single()
        || !matches!(returned.spec.unhinted(), ChooseSpec::WithCount(_, _)) { return false; }
    let ChooseSpec::Object(filter) = returned.spec.base() else { return false; };
    if filter.zone != Some(crate::zone::Zone::Exile) || filter.tagged_constraints.len() != 1
        || filter.tagged_constraints[0].tag.as_str() != ironsmith_core::SOURCE_EXILED_TAG
        || filter.tagged_constraints[0].relation != crate::target::TaggedOpbjectRelation::IsTaggedObject { return false; }
    let mut filter = filter.clone(); filter.zone = None; filter.tagged_constraints.clear();
    filter == crate::target::ObjectFilter::default()
}

/// One typed draw/hand-exile producer, one inspector, and one singular return
/// form a complete linked family. Costs are limited to mana and tapping the
/// source; no extra producer, alternative scope or executable child is ignored.
pub(super) fn bind_private_return_linked_exile(definition: &mut CardDefinition) {
    use ironsmith_core::StaticAbilityPayload;
    if definition.abilities.len() != 3 || definition.spell_effect.is_some()
        || !definition.alternative_casts.is_empty() || !definition.optional_costs.is_empty()
        || definition.additional_cost.has_non_mana_costs()
        || definition.additional_cost.dynamic_mana_cost().is_some()
        || definition.additional_cost.as_one_of().is_some() { return; }
    let (mut producer, mut consumer, mut inspector) = (None, None, None);
    for (slot, ability) in definition.abilities.iter().enumerate() {
        match &ability.kind {
            AbilityKind::Static(ability) if matches!(ability.payload, StaticAbilityPayload::LookAtSourceExiledCards { .. }) => {
                if inspector.replace(slot).is_some() { return; }
            }
            AbilityKind::Activated(ability) if simple_linked_activation(ability) => {
                if draw_then_private_hand_exile(&ability.effects) {
                    if producer.replace(slot).is_some() { return; }
                } else if single_paired_return(&ability.effects) {
                    if consumer.replace(slot).is_some() { return; }
                } else { return; }
            }
            _ => return,
        }
    }
    let (Some(producer), Some(consumer), Some(inspector)) = (producer, consumer, inspector) else { return; };
    if let AbilityKind::Activated(ability) = &mut definition.abilities[producer].kind {
        let mut segments = ability.effects.segments.clone();
        let Some(exile) = segments[0].default_effects[2].downcast_ref::<crate::effects::ExileEffect>() else { return; };
        let mut exile = exile.clone(); exile.exclude_prior_zone_viewers = true;
        segments[0].default_effects[2] = Effect::new(exile); ability.effects.replace_segments(segments);
    }
    let Ok(bytes) = serde_json::to_vec(&definition.abilities) else { return; };
    let pair = ironsmith_core::LinkedExilePair {
        definition: ironsmith_core::LinkedExileDefinition(Sha256::digest(bytes).into()), pair: producer as u32,
    };
    for slot in [producer, consumer] {
        if let AbilityKind::Activated(ability) = &mut definition.abilities[slot].kind { ability.effects.linked_exile_pair = Some(pair); }
    }
    if let AbilityKind::Static(ability) = &mut definition.abilities[inspector].kind
        && let StaticAbilityPayload::LookAtSourceExiledCards { pair: member, .. } = &mut ability.payload { *member = Some(pair); }
}

fn class_level_activation(ability: &crate::ability::Ability, level: u32) -> bool {
    let AbilityKind::Activated(activated) = &ability.kind else { return false; };
    if activated.keyword != Some(ironsmith_core::ActivatedAbilityKeyword::ClassLevel(level))
        || activated.timing != crate::ability::ActivationTiming::SorcerySpeed
        || !activated.choices.is_empty() || activated.is_loyalty_ability
        || !activated.activation_restrictions.is_empty() || activated.activation_condition.is_some()
        || activated.mana_output.is_some() || !activated.mana_usage_restrictions.is_empty()
        || !activated.mana_cost.as_all().is_some_and(|costs| !costs.is_empty()
            && costs.iter().all(|cost| matches!(cost, crate::costs::Cost::Mana(_))))
        || activated.effects.segments.len() != 1 || !activated.effects.segments[0].self_replacements.is_empty()
    { return false; }
    let effects = activated.effects.all_effects();
    if effects.len() != 1 { return false; }
    let Some(put) = crate::compile_support::effect_without_result_tags(effects[0])
        .downcast_ref::<crate::effects::PutCountersEffect>() else { return false; };
    put.counter_type == crate::CounterType::Level && put.amount == Value::Fixed(1)
        && put.target == ChooseSpec::Source && put.target_count.is_none() && !put.distributed
        && put.maximum_total.is_none() && put.completion_action.is_none()
}

fn class_private_library_producer(program: &ResolutionProgram) -> bool {
    if program.segments.len() != 1 || !program.segments[0].self_replacements.is_empty() { return false; }
    let effects = program.all_effects();
    if effects.len() != 2 { return false; }
    let Some(exile) = crate::compile_support::effect_without_result_tags(effects[0])
        .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>() else { return false; };
    let Some(look) = crate::compile_support::effect_without_result_tags(effects[1])
        .downcast_ref::<crate::effects::LookAtObjectsEffect>() else { return false; };
    if !exile.face_down || exile.count != Value::Fixed(1) || !exile.accumulated_tags.is_empty()
        || exile.moved_tags.len() != 1 || !look.permit_while_exiled
        || look.viewer != crate::target::PlayerFilter::You || look.subject != crate::target::PlayerFilter::You
    { return false; }
    let mut pool = look.filter.clone();
    if pool.tagged_constraints.len() != 1
        || pool.tagged_constraints[0].tag != exile.moved_tags[0]
        || pool.tagged_constraints[0].relation != crate::target::TaggedOpbjectRelation::IsTaggedObject
    { return false; }
    pool.tagged_constraints.clear();
    if pool.zone.take() != Some(crate::zone::Zone::Exile) { return false; }
    pool == crate::target::ObjectFilter::default()
}

/// A complete Class definition proves where the base producer and final
/// reader live. Only this reader may project its generated level wrapper back
/// to the enclosing rules-text acquisition. Other effect/level grants retain
/// their independent owners.
pub(super) fn bind_class_linked_exile(definition: &mut CardDefinition) {
    use ironsmith_core::{Grantable, StaticAbilityId, StaticAbilityPayload};
    if !definition.card.subtypes.contains(&crate::types::Subtype::Class)
        || definition.abilities.len() != 5 || definition.spell_effect.is_some()
        || !definition.alternative_casts.is_empty() || !definition.optional_costs.is_empty()
        || definition.additional_cost.has_non_mana_costs()
        || definition.additional_cost.dynamic_mana_cost().is_some()
        || definition.additional_cost.as_one_of().is_some()
        || definition.abilities.iter().any(|ability| ability.functional_zones != [crate::zone::Zone::Battlefield])
    { return; }
    let AbilityKind::Triggered(trigger) = &definition.abilities[0].kind else { return; };
    if !class_private_library_producer(&trigger.effects)
        || !class_level_activation(&definition.abilities[1], 2)
        || !class_level_activation(&definition.abilities[3], 3) { return; }
    let AbilityKind::Static(menace) = &definition.abilities[2].kind else { return; };
    let StaticAbilityPayload::GrantObjectAbilityForFilter(grant) = &menace.payload else { return; };
    if grant.filter != crate::target::ObjectFilter::creature().you_control()
        || !grant.additional_abilities.is_empty() || grant.condition.is_some()
        || grant.ability.functional_zones != [crate::zone::Zone::Battlefield]
        || !matches!(&grant.ability.kind, AbilityKind::Static(ability)
            if ability.id == Some(StaticAbilityId::Menace) && matches!(ability.payload, StaticAbilityPayload::None))
    { return; }
    let AbilityKind::Static(reader) = &definition.abilities[4].kind else { return; };
    let StaticAbilityPayload::Grants(spec) = &reader.payload else { return; };
    if !spec.requires_linked_exile_pair || !matches!(spec.grantable, Grantable::PlayFrom)
        || spec.zone != crate::zone::Zone::Exile || !spec.additional_zones.is_empty()
        || spec.beneficiary != crate::target::PlayerFilter::You
        || spec.usage_limit.is_some() || spec.max_plays.is_some()
        || !spec.cast_this_way_grants.is_empty() || !spec.permanent_this_way_grants.is_empty()
        || !spec.on_use_effects.is_empty() || spec.cast_this_way_filter.is_some()
        || spec.top_card_only || spec.instant_timing || spec.may_look_at_top || spec.may_look_at_linked_exile
        || spec.cast_mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::AnyColor
    { return; }
    let mut pool = spec.filter.clone();
    if pool.zone.take() != Some(crate::zone::Zone::Exile) || pool.tagged_constraints.len() != 1
        || pool.tagged_constraints[0].tag.as_str() != ironsmith_core::SOURCE_EXILED_TAG
        || pool.tagged_constraints[0].relation != crate::target::TaggedOpbjectRelation::IsTaggedObject { return; }
    pool.tagged_constraints.clear();
    if pool != crate::target::ObjectFilter::default() { return; }
    if let AbilityKind::Static(reader) = &mut definition.abilities[4].kind
        && let StaticAbilityPayload::Grants(spec) = &mut reader.payload { spec.linked_exile_class_level = Some(3); }
    let Ok(bytes) = serde_json::to_vec(&definition.abilities) else { return; };
    let pair = ironsmith_core::LinkedExilePair {
        definition: ironsmith_core::LinkedExileDefinition(Sha256::digest(bytes).into()), pair: 0,
    };
    if let AbilityKind::Triggered(trigger) = &mut definition.abilities[0].kind { trigger.effects.linked_exile_pair = Some(pair); }
    if let AbilityKind::Static(reader) = &mut definition.abilities[4].kind
        && let StaticAbilityPayload::Grants(spec) = &mut reader.payload { spec.linked_exile_pair = Some(pair); }
}
