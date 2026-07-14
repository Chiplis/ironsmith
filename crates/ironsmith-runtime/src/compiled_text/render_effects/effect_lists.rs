use super::*;

#[path = "effect_list/forced_block_patterns.rs"]
mod forced_block_patterns;
#[path = "effect_list/graveyard_return_compaction.rs"]
mod graveyard_return_compaction;
#[path = "effect_list/helpers_00.rs"]
mod helpers_00;
#[path = "effect_list/helpers_01.rs"]
mod helpers_01;
#[path = "effect_list/helpers_02.rs"]
mod helpers_02;

pub(super) use forced_block_patterns::*;
pub(super) use graveyard_return_compaction::*;
use helpers_00::wrapped_effect_tag;
use helpers_00::*;
use helpers_01::describe_linked_graveyard_choices_then_may_return_bundle as describe_effect_list_linked_graveyard_choices_then_may_return_bundle;
use helpers_01::*;
pub(super) use helpers_01::{
    describe_countered_spell_exile_replacement_followup,
    describe_tagged_die_exile_replacement_followup,
};
pub(super) use helpers_02::render_look_reveal_repeated_choices;
use helpers_02::*;

pub(super) fn structural_unwrap_render_wrappers(effect: &Effect) -> &Effect {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return structural_unwrap_render_wrappers(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return structural_unwrap_render_wrappers(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return structural_unwrap_render_wrappers(&tag_all.effect);
    }
    effect
}

pub(super) fn describe_may_pay_mana_and_discard(may: &crate::effects::MayEffect) -> Option<String> {
    let [pay_effect, discard_effect] = may.effects.as_slice() else {
        return None;
    };
    let pay = structural_unwrap_render_wrappers(pay_effect)
        .downcast_ref::<crate::effects::PayManaEffect>()?;
    let discard = structural_unwrap_render_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if may.decider != Some(PlayerFilter::You)
        || pay.player != ChooseSpec::Player(PlayerFilter::You)
        || discard.player != PlayerFilter::You
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
    {
        return None;
    }
    let count = match discard.count.unhinted() {
        Value::Fixed(1) => "a card".to_string(),
        count => format!("{} cards", describe_value(count)),
    };
    Some(format!(
        "you may pay {} and discard {count}",
        pay.cost.to_oracle()
    ))
}

pub(crate) fn describe_action_and_get_energy_pair(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let energy = structural_unwrap_render_wrappers(second)
        .downcast_ref::<crate::effects::EnergyCountersEffect>()?;
    if energy.player != PlayerFilter::You {
        return None;
    }
    let energy_text = describe_effect(second);
    let energy_amount = energy_text
        .trim()
        .trim_end_matches('.')
        .strip_prefix("you get ")?;

    let first = structural_unwrap_render_wrappers(first);
    if let Some(gain) = first.downcast_ref::<crate::effects::GainLifeEffect>() {
        let actor = choose_spec_player_filter(&gain.player)?;
        if actor != PlayerFilter::You {
            return None;
        }
        return Some(format!(
            "you gain {} and get {energy_amount}",
            describe_life_amount_phrase(&gain.amount)
        ));
    }
    if let Some(draw) = first.downcast_ref::<crate::effects::DrawCardsEffect>() {
        if draw.player != PlayerFilter::You {
            return None;
        }
        return Some(format!(
            "Draw {} and you get {energy_amount}",
            describe_card_count(&draw.count)
        ));
    }

    let first_text = describe_effect(first)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if first_text.is_empty() || first_text.contains(". ") {
        return None;
    }
    if let Some(mill) = first.downcast_ref::<crate::effects::MillEffect>() {
        return Some(if mill.player == PlayerFilter::You {
            format!("{first_text} and get {energy_amount}")
        } else {
            format!("{first_text} and you get {energy_amount}")
        });
    }
    if first
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .is_some_and(|return_to_hand| matches!(return_to_hand.spec.base(), ChooseSpec::Source))
    {
        return Some(format!("{first_text} and you get {energy_amount}"));
    }
    None
}

fn linked_counter_followup_surface(
    put: &crate::effects::PutCountersEffect,
) -> Option<ValueSurfaceHint> {
    [
        ValueSurfaceHint::CounterFollowupThen,
        ValueSurfaceHint::CounterFollowupSeparateSentence,
    ]
    .into_iter()
    .find(|hint| put.amount.has_surface_hint(*hint))
}

fn effect_outer_id(effect: &Effect) -> Option<crate::effect::EffectId> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return Some(with_id.id);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return effect_outer_id(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return effect_outer_id(&tag_all.effect);
    }
    None
}

fn affected_object_characteristic(
    value: &Value,
    expected_id: crate::effect::EffectId,
) -> Option<&'static str> {
    let Value::EffectMetric {
        effect_id,
        source: crate::effect::EffectMetricSource::AffectedObjects,
        metric,
    } = value.unhinted()
    else {
        return None;
    };
    if *effect_id != expected_id {
        return None;
    }
    match metric {
        crate::effect::EffectMetric::FirstPower => Some("power"),
        crate::effect::EffectMetric::FirstToughness => Some("toughness"),
        crate::effect::EffectMetric::FirstManaValue => Some("mana value"),
        _ => None,
    }
}

fn replace_affected_object_characteristic_reference(
    text: &str,
    characteristic: &str,
    antecedent: &str,
) -> String {
    [
        format!("that creature's {characteristic}"),
        format!("that card's {characteristic}"),
        format!("its {characteristic}"),
    ]
    .into_iter()
    .find_map(|surface| {
        text.contains(&surface)
            .then(|| text.replacen(&surface, antecedent, 1))
    })
    .unwrap_or_else(|| text.to_string())
}

fn tagged_characteristic_reference(value: &Value, expected_tag: &TagKey) -> Option<&'static str> {
    let (spec, characteristic) = match value.unhinted() {
        Value::PowerOf(spec) => (spec, "power"),
        Value::ToughnessOf(spec) => (spec, "toughness"),
        Value::ManaValueOf(spec) => (spec, "mana value"),
        _ => return None,
    };
    matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == expected_tag).then_some(characteristic)
}

fn returned_object_reference_noun(spec: &ChooseSpec) -> &'static str {
    let filter = match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
        _ => return "permanent",
    };
    if filter.card_types.contains(&CardType::Creature) {
        "creature"
    } else if filter.card_types.contains(&CardType::Artifact) {
        "artifact"
    } else if filter.card_types.contains(&CardType::Enchantment) {
        "enchantment"
    } else if filter.card_types.contains(&CardType::Planeswalker) {
        "planeswalker"
    } else if filter.card_types.contains(&CardType::Battle) {
        "battle"
    } else if filter.card_types.contains(&CardType::Land) {
        "land"
    } else {
        "permanent"
    }
}

fn describe_linked_counter_followup(effects: &[Effect]) -> Option<String> {
    let effects = match effects {
        [target, tail @ ..]
            if target
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some() =>
        {
            tail
        }
        _ => effects,
    };
    let [first_effect, put_effect] = effects else {
        return None;
    };
    let put = structural_unwrap_render_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed {
        return None;
    }
    let surface = linked_counter_followup_surface(put)?;
    let first_tag = effect_outer_tag(first_effect)?;

    let first = structural_unwrap_render_wrappers(first_effect);
    let mut put_text = describe_effect(put_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if first
        .downcast_ref::<crate::effects::CreateTokenEffect>()
        .is_some()
    {
        if !matches!(put.target.base(), ChooseSpec::Tagged(tag) if tag == first_tag) {
            return None;
        }
    } else if let Some(return_to_hand) = first.downcast_ref::<crate::effects::ReturnToHandEffect>()
    {
        let characteristic = tagged_characteristic_reference(&put.amount, first_tag)?;
        let antecedent = format!(
            "that {}'s {characteristic}",
            returned_object_reference_noun(&return_to_hand.spec)
        );
        put_text = put_text.replacen(&format!("its {characteristic}"), &antecedent, 1);
    } else if let Some(move_to_zone) = first.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        if !move_to_zone_is_plain_exile(move_to_zone) {
            return None;
        }
        let characteristic =
            affected_object_characteristic(&put.amount, effect_outer_id(first_effect)?)?;
        let antecedent = match move_to_zone.target.base() {
            ChooseSpec::Object(filter) if filter.zone == Some(Zone::Graveyard) => {
                format!("the {characteristic} of the card you exiled")
            }
            ChooseSpec::Object(filter) if filter.card_types.as_slice() == [CardType::Creature] => {
                format!("the {characteristic} of the creature exiled this way")
            }
            _ => format!("the {characteristic} of the permanent exiled this way"),
        };
        put_text = replace_affected_object_characteristic_reference(
            &put_text,
            characteristic,
            &antecedent,
        );
    } else if first
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
        .is_some()
    {
        if !choose_spec_is_tagged_object(&put.target, first_tag) {
            return None;
        }
    } else if first
        .downcast_ref::<crate::effects::ExileEffect>()
        .is_some()
    {
        let Value::Count(filter) = put.amount.unhinted() else {
            return None;
        };
        if !matches!(put.target.base(), ChooseSpec::Source)
            || !filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == *first_tag
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            })
        {
            return None;
        }
    } else {
        return None;
    }

    let first_text = describe_effect(first_effect);
    let first_text = first_text.trim().trim_end_matches('.');
    Some(match surface {
        ValueSurfaceHint::CounterFollowupThen => {
            format!("{first_text}, then {}", lowercase_first(&put_text))
        }
        ValueSurfaceHint::CounterFollowupSeparateSentence => {
            format!("{first_text}. {}", capitalize_first(&put_text))
        }
        _ => return None,
    })
}

/// Honor the sentence boundary carried on a counter-placement value after
/// lowering. The split is metadata-driven: effects without the explicit
/// surface hint retain the ordinary coordination and compaction paths.
fn describe_typed_counter_sentence_split(effects: &[Effect]) -> Option<String> {
    let is_sentence_start = |effect: &Effect| {
        structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::PutCountersEffect>()
            .is_some_and(|put| {
                linked_counter_followup_surface(put)
                    == Some(ValueSurfaceHint::CounterFollowupSeparateSentence)
            })
    };

    // Every counter effect produced by one authored sentence carries the
    // same hint. Once a recursively rendered suffix starts at that sentence,
    // do not split its coordinated counter list again.
    if effects
        .first()
        .is_some_and(|effect| is_sentence_start(effect))
    {
        return None;
    }
    let split = effects
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(idx, effect)| is_sentence_start(effect).then_some(idx))?;

    let first = describe_effect_list(&effects[..split]);
    let second = describe_effect_list(&effects[split..]);
    if first.trim().is_empty() || second.trim().is_empty() {
        return None;
    }
    Some(format!(
        "{}. {}",
        first.trim().trim_end_matches('.'),
        capitalize_first(second.trim().trim_end_matches('.'))
    ))
}

fn describe_exile_top_play_then_additional_land(effects: &[Effect]) -> Option<String> {
    let [exile_effect, grant_effect, land_effect] = effects else {
        return None;
    };
    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let grant = structural_unwrap_render_wrappers(grant_effect)
        .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    let land = structural_unwrap_render_wrappers(land_effect)
        .downcast_ref::<crate::effects::AdditionalLandPlaysEffect>()?;
    let prefix = describe_exile_top_then_play(exile, grant, false)?;
    if land.player != PlayerFilter::You || land.duration != Until::EndOfTurn {
        return None;
    }
    let land = capitalize_first(describe_effect(land_effect).trim_end_matches('.'));
    Some(format!("{}. {land}", prefix.trim_end_matches('.')))
}

fn describe_hidden_exile_partition_with_persistent_permission(
    effects: &[Effect],
) -> Option<String> {
    let complete_effects = effects.iter().collect::<Vec<_>>();
    if let Some(compact) =
        describe_target_opponent_look_exile_one_rest_bottom_cast(&complete_effects)
    {
        // Preserve the target declaration while the exact recognizer validates
        // the target/look/remainder relationship. Stripping TargetOnly first
        // leaves the generic renderer unable to distinguish "that library"
        // or the singular tagged card from a broad exiled-card collection.
        return Some(compact);
    }

    let effects = match effects {
        [target, tail @ ..]
            if target
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some() =>
        {
            tail
        }
        _ => effects,
    };
    let [
        look_effect,
        choose_effect,
        exile_effect,
        remainder_effect,
        grant_effect,
    ] = effects
    else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileEffect>()?;
    let remainder = remainder_effect
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    describe_look_at_top_choose_exile_face_down_rest_bottom_then_play_while_exiled(
        look, choose, exile, remainder, grant,
    )
}

fn describe_each_opponent_top_card_hidden_exile_permission(effects: &[Effect]) -> Option<String> {
    let [players_effect, permission_effect] = effects else {
        return None;
    };
    let players = players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if players.filter != PlayerFilter::Opponent
        || players.starting_with_controller
        || players.stop_after_first_happened
    {
        return None;
    }
    let [choose_effect, exile_effect] = players.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.top_only
        || choose.bottom_only
        || choose.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || choose.is_search
        || choose.reveal
    {
        return None;
    }

    let collection_tag = effect_outer_tag(exile_effect)?;
    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileEffect>()?;
    if !exile.face_down
        || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }

    let permission = permission_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if permission.tag != *collection_tag
        || permission.player != PlayerFilter::You
        || permission.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
        || !permission.allow_land
        || permission.mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::Normal
        || permission.while_on_top_of_library
        || permission.filter.is_some()
    {
        return None;
    }

    Some(
        "Exile the top card of each opponent's library face down. You may look at and play those cards for as long as they remain exiled"
            .to_string(),
    )
}

fn describe_exile_all_then_each_player_may_deploy_and_return_exiled(
    effects: &[Effect],
) -> Option<String> {
    let [
        exile_effect,
        players_effect,
        return_effect,
        source_exile_effect,
    ] = effects
    else {
        return None;
    };

    let exiled_tag = effect_outer_tag(exile_effect)?;
    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileEffect>()?;
    if exile.face_down
        || !matches!(exile.spec.base(), ChooseSpec::All(filter) if filter == &ObjectFilter::creature())
    {
        return None;
    }

    let players = structural_unwrap_render_wrappers(players_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if players.filter != PlayerFilter::Any
        || players.starting_with_controller
        || players.stop_after_first_happened
    {
        return None;
    }
    let [may_effect] = players.effects.as_slice() else {
        return None;
    };
    let may = structural_unwrap_render_wrappers(may_effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| decider != &PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let [choose_effect, deploy_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::IteratedPlayer
        || choose.count != ChoiceCount::any_number()
        || choose.count_value.is_some()
        || choose.zone != Some(Zone::Hand)
        || !choose.additional_zones.is_empty()
        || choose.filter.card_types != [CardType::Creature]
        || choose.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
    {
        return None;
    }
    let deploy = structural_unwrap_render_wrappers(deploy_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if deploy.zone != Zone::Battlefield
        || !choose_spec_is_tagged_object(&deploy.target, &choose.tag)
        || deploy.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || deploy.enters_tapped
        || deploy.enters_attacking
        || deploy.enters_face_down
    {
        return None;
    }

    let return_exiled = structural_unwrap_render_wrappers(return_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if return_exiled.zone != Zone::Hand
        || !choose_spec_is_tagged_object(&return_exiled.target, exiled_tag)
        || return_exiled.to_top
    {
        return None;
    }

    let source_exile = structural_unwrap_render_wrappers(source_exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_zone_is_plain_exile(source_exile)
        || !matches!(source_exile.target.base(), ChooseSpec::Source)
    {
        return None;
    }
    let source_exile_text = describe_effect(source_exile_effect);
    let source_exile_text = source_exile_text.trim().trim_end_matches('.');
    let source_exile_text = source_exile_text
        .strip_prefix("You ")
        .or_else(|| source_exile_text.strip_prefix("you "))
        .unwrap_or(source_exile_text);
    if !source_exile_text.to_ascii_lowercase().starts_with("exile ") {
        return None;
    }

    Some(format!(
        "Exile all creatures. Each player may put any number of creature cards from their hand onto the battlefield. Then put all cards exiled this way into their owners' hands. {}",
        capitalize_first(source_exile_text)
    ))
}

fn describe_look_hand_optional_exile_persistent_play_tax(effects: &[Effect]) -> Option<String> {
    fn face_up_exile_spec(effect: &Effect) -> Option<&ChooseSpec> {
        let effect = structural_unwrap_render_wrappers(effect);
        if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
            return (!exile.face_down).then_some(&exile.spec);
        }
        let move_to_zone = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        move_to_zone_is_plain_exile(move_to_zone).then_some(&move_to_zone.target)
    }

    fn single_object_filter(spec: &ChooseSpec) -> Option<&ObjectFilter> {
        match spec {
            ChooseSpec::SurfaceHinted { spec, .. } => single_object_filter(spec),
            ChooseSpec::WithCount(spec, count) if count.is_single() => single_object_filter(spec),
            ChooseSpec::Object(filter) => Some(filter),
            _ => None,
        }
    }

    let [look_effect, may_effect, permission_effect, tax_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if look.reveal || !is_target_opponent_spec(&look.target) {
        return None;
    }
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if !may
        .decider
        .as_ref()
        .is_none_or(|decider| decider == &PlayerFilter::You)
    {
        return None;
    }
    let (exile_tag, filter) = match may.effects.as_slice() {
        [exile_effect] => {
            let exile_tag = structural_effect_tag(exile_effect)
                .cloned()
                .unwrap_or_else(|| TagKey::from(crate::tag::SOURCE_EXILED_TAG));
            (
                exile_tag,
                single_object_filter(face_up_exile_spec(exile_effect)?)?,
            )
        }
        [choose_effect, exile_effect] => {
            let choose = structural_unwrap_render_wrappers(choose_effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let exile_spec = face_up_exile_spec(exile_effect)?;
            if choose.chooser != PlayerFilter::You
                || !choose.count.is_single()
                || choose_primary_zone(choose) != Some(Zone::Hand)
                || !matches!(exile_spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
            {
                return None;
            }
            (choose.tag.clone(), &choose.filter)
        }
        _ => return None,
    };
    if filter.zone != Some(Zone::Hand)
        || !matches!(&filter.owner, None | Some(PlayerFilter::Target(_)))
        || !filter.card_types.is_empty()
        || filter.excluded_card_types != [CardType::Land]
    {
        return None;
    }

    let permission = permission_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if permission.tag != exile_tag
        || permission.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
        || !permission.allow_land
        || permission.allow_any_color_for_cast
        || !matches!(
            &permission.player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag)) if tag == &exile_tag
        )
    {
        return None;
    }

    let tax = tax_effect.downcast_ref::<crate::effects::GrantEffect>()?;
    if tax.duration != crate::grant::GrantDuration::Forever
        || !matches!(&tax.target, ChooseSpec::Tagged(tag) if tag == &exile_tag)
    {
        return None;
    }
    let crate::grant::Grantable::Ability(ability) = &tax.grantable else {
        return None;
    };
    let cost_increase = ability.cost_increase_mana_cost()?;
    if cost_increase.filter.stack_kind != Some(crate::filter::StackObjectKind::Spell)
        || cost_increase.filter.cast_by.is_some()
    {
        return None;
    }

    Some(format!(
        "Look at target opponent's hand. You may exile a nonland card from it. For as long as that card remains exiled, its owner may play it. A spell cast this way costs {} more to cast",
        cost_increase.increase.to_oracle()
    ))
}

fn describe_discard_redraw_mana_value_ladder(effects: &[Effect]) -> Option<String> {
    fn is_artifact_or_creature_filter(filter: &ObjectFilter) -> bool {
        let mut types = filter.card_types.clone();
        for branch in &filter.any_of {
            if branch.card_types.len() != 1 || !branch.any_of.is_empty() {
                return false;
            }
            types.extend(branch.card_types.iter().copied());
        }
        types.len() == 2
            && types.contains(&CardType::Artifact)
            && types.contains(&CardType::Creature)
    }

    let [
        discard_effect,
        draw_effect,
        first,
        second,
        third,
        return_effect,
    ] = effects
    else {
        return None;
    };
    let with_id = discard_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let discard = with_id
        .effect
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    let discarded_tag = discard.tag.as_ref()?;
    if discard.player != PlayerFilter::You
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
        || !discard
            .count
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::AllCardsInHand)
    {
        return None;
    }
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You
        || !value_is_discarded_count_for_effect(&draw.count, with_id.id)
    {
        return None;
    }

    let choices = [
        first.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
        second.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
        third.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
    ];
    let selected_tag = &choices[0].tag;
    for (index, choice) in choices.iter().enumerate() {
        if choice.chooser != PlayerFilter::You
            || choice.count.min != 0
            || choice.count.max != Some(1)
            || &choice.tag != selected_tag
            || choose_primary_zone(choice) != Some(Zone::Graveyard)
            || choice.filter.owner != Some(PlayerFilter::You)
            || !is_artifact_or_creature_filter(&choice.filter)
            || choice.filter.mana_value
                != Some(crate::filter::Comparison::Equal((index + 1) as i32))
            || !object_filter_has_tag(&choice.filter, discarded_tag)
        {
            return None;
        }
    }

    let return_to_battlefield = return_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if return_to_battlefield.zone != Zone::Battlefield
        || return_to_battlefield.enters_tapped
        || return_to_battlefield.enters_attacking
        || !matches!(&return_to_battlefield.target, ChooseSpec::Tagged(tag) if tag == selected_tag)
    {
        return None;
    }

    Some(
        "Discard all the cards in your hand, then draw that many cards. You may choose an artifact or creature card with mana value 1 you discarded this way, then do the same for artifact or creature cards with mana values 2 and 3. Return those cards to the battlefield"
            .to_string(),
    )
}

fn describe_exile_top_choose_one_play_next_turn(effects: &[Effect]) -> Option<String> {
    let [look_effect, move_effect, choose_effect, grant_effect] = effects else {
        return None;
    };
    let look = structural_unwrap_render_wrappers(look_effect)
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let move_to_exile = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let grant = structural_unwrap_render_wrappers(grant_effect)
        .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if look.reveal
        || !tagged_move_to_zone(move_to_exile, &look.tag, Zone::Exile, move_to_exile.to_top)
        || move_to_exile.enters_face_down
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Exile)
        || !choose.additional_zones.is_empty()
        || choose_exact_count(choose) != Some(1)
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || !filter_is_exactly_tagged_in_zone(&choose.filter, &look.tag, Zone::Exile)
        || grant.tag != choose.tag
        || grant.player != PlayerFilter::You
        || grant.allow_any_color_for_cast
        || grant.while_on_top_of_library
        || grant.filter.is_some()
        || grant.cast_pool_is_plural
    {
        return None;
    }
    let duration = match grant.duration {
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn => "Until end of turn",
        crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
            "Until the end of your next turn"
        }
        crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep => "Until your next end step",
        _ => return None,
    };
    let owner = describe_possessive_player_filter(&look.player);
    let verb = if grant.allow_land { "play" } else { "cast" };
    Some(format!(
        "Exile {} from the top of {owner} library, then choose a card exiled this way. {duration}, you may {verb} that card",
        describe_card_count(&look.count)
    ))
}

fn describe_energy_payment_failure_fallback(effects: &[Effect]) -> Option<String> {
    let [payment_effect, fallback_effect] = effects else {
        return None;
    };
    let with_id = payment_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let payment = with_id
        .effect
        .downcast_ref::<crate::effects::PayEnergyEffect>()?;
    let fallback = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if !matches!(payment.player.base(), ChooseSpec::Player(PlayerFilter::You))
        || fallback.condition != with_id.id
        || fallback.predicate != EffectPredicate::DidNotHappen
        || fallback.then.is_empty()
        || !fallback.else_.is_empty()
    {
        return None;
    }

    let payment_text = describe_effect(&with_id.effect);
    let fallback_text = describe_effect_list(&fallback.then);
    let payment_text = payment_text.trim().trim_end_matches('.');
    let fallback_text = fallback_text.trim().trim_end_matches('.');
    (!payment_text.is_empty() && !fallback_text.is_empty()).then(|| {
        format!(
            "{payment_text}. If you can't, {}",
            lowercase_first(fallback_text)
        )
    })
}

pub(super) fn describe_tap_then_put_counters_same_target(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let tap_tag = effect_outer_tag(first)?;
    let tap =
        structural_unwrap_render_wrappers(first).downcast_ref::<crate::effects::TapEffect>()?;
    let count = tap.target.count();
    if !tap.target.is_target() || count.max != Some(1) || count.dynamic_x || count.random {
        return None;
    }

    let put = structural_unwrap_render_wrappers(second)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed
        || put.target_count.is_some()
        || !matches!(put.target.base(), ChooseSpec::Tagged(found) if found == tap_tag)
    {
        return None;
    }

    Some(format!(
        "Tap {} and put {} on it",
        describe_choose_spec(&tap.target),
        describe_put_counter_phrase(&put.amount, put.counter_type)
    ))
}

pub(super) fn describe_choose_tap_conditional_freeze_bundle(effects: &[&Effect]) -> Option<String> {
    let [target_effect, tap_effect, conditional_effect] = effects else {
        return None;
    };
    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    let tap = structural_unwrap_render_wrappers(tap_effect)
        .downcast_ref::<crate::effects::TapEffect>()?;
    if !matches!(tap.target.base(), ChooseSpec::Tagged(tag) if tag == target_tag) {
        return None;
    }
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let Condition::PlayerControls { .. } = &conditional.condition else {
        return None;
    };
    let cant = structural_unwrap_render_wrappers(&conditional.if_true[0])
        .downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Untap(filter) = &cant.restriction else {
        return None;
    };
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && &constraint.tag == target_tag
    }) {
        return None;
    }
    let freeze = describe_untap_restriction_for_subject(
        cant,
        UntapRestrictionSubject::singular("The chosen creature"),
    )?;
    Some(format!(
        "Choose {} and tap it. If {}, {}",
        describe_choose_spec(&target_only.target),
        describe_condition(&conditional.condition),
        lowercase_first(&freeze)
    ))
}

pub(super) fn rendered_action_target(effect: &Effect) -> Option<&ChooseSpec> {
    let action = structural_unwrap_render_wrappers(effect);
    if let Some(apply) = action.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        apply.target_spec.as_ref()
    } else if let Some(destroy) = action.downcast_ref::<crate::effects::DestroyEffect>() {
        Some(&destroy.spec)
    } else if let Some(put) = action.downcast_ref::<crate::effects::PutCountersEffect>() {
        Some(&put.target)
    } else if let Some(tap) = action.downcast_ref::<crate::effects::TapEffect>() {
        Some(&tap.target)
    } else if let Some(untap) = action.downcast_ref::<crate::effects::UntapEffect>() {
        Some(&untap.target)
    } else {
        None
    }
}

pub(super) fn target_specs_select_same_objects(left: &ChooseSpec, right: &ChooseSpec) -> bool {
    use ChooseSpec::{SurfaceHinted, Target, WithCount, WithCountValue};

    match (left, right) {
        (SurfaceHinted { spec, .. }, other) | (other, SurfaceHinted { spec, .. }) => {
            target_specs_select_same_objects(spec, other)
        }
        (Target(left), Target(right)) => target_specs_select_same_objects(left, right),
        (WithCount(left, left_count), WithCount(right, right_count))
        | (WithCountValue(left, left_count, _), WithCount(right, right_count))
        | (WithCount(left, left_count), WithCountValue(right, right_count, _))
        | (WithCountValue(left, left_count, _), WithCountValue(right, right_count, _)) => {
            left_count == right_count && target_specs_select_same_objects(left, right)
        }
        _ => left == right,
    }
}

pub(super) fn describe_redundant_target_only_pair(effects: &[Effect]) -> Option<String> {
    let [target_effect, action_effect] = effects else {
        return None;
    };
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let action_target = rendered_action_target(action_effect)?;
    target_specs_select_same_objects(action_target, &target_only.target)
        .then(|| describe_effect(structural_unwrap_render_wrappers(action_effect)))
}

pub(super) fn describe_kicked_additional_targets_put_counters(
    effects: &[&Effect],
) -> Option<String> {
    let [target_effect, for_each_effect] = effects else {
        return None;
    };
    let target_tag = target_effect
        .downcast_ref::<crate::effects::TaggedEffect>()?
        .tag
        .clone();
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::WithCountValue(target, count, count_value) = &target_only.target else {
        return None;
    };
    if !count.is_dynamic_x() || count.is_up_to_dynamic_x() || count.is_random() {
        return None;
    }
    let Value::Add(left, right) = count_value else {
        return None;
    };
    let counts_one_plus_kicked = matches!(
        (left.as_ref(), right.as_ref()),
        (Value::Fixed(1), Value::KickCount) | (Value::KickCount, Value::Fixed(1))
    );
    if !counts_one_plus_kicked {
        return None;
    }

    let for_each = structural_unwrap_render_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachObject>()?;
    if !for_each.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == target_tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) {
        return None;
    }
    let [put] = for_each.effects.as_slice() else {
        return None;
    };
    let put = structural_unwrap_render_wrappers(put)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed || put.target_count.is_some() || !matches!(put.target, ChooseSpec::Iterated)
    {
        return None;
    }

    let first_target = describe_choose_spec(target);
    let additional_target = first_target
        .strip_prefix("target ")
        .map(|tail| format!("another target {tail}"))
        .unwrap_or_else(|| format!("another {first_target}"));
    Some(format!(
        "Choose {first_target}, then choose {additional_target} for each time this spell was kicked. Put {} on each of them",
        describe_put_counter_phrase(&put.amount, put.counter_type)
    ))
}

pub(super) fn for_each_moves_unchosen_iterated_to_zone(
    effect: &Effect,
    revealed_tag: &crate::TagKey,
    chosen_tag: &crate::TagKey,
    zone: Zone,
) -> bool {
    let Some((_, for_each)) = for_each_tagged_for_compaction(effect) else {
        return false;
    };
    for_each_moves_unselected_to_zone(for_each, revealed_tag.as_str(), chosen_tag.as_str(), zone)
}

pub(super) fn describe_reveal_top_one_hand_gain_mana_value_rest_graveyard(
    effects: &[Effect],
) -> Option<String> {
    let (look_effect, choose_effect, move_effect, gain_effect, rest_effect) = match effects {
        [look, choose, move_effect, gain_effect, rest_effect] => {
            (look, choose, move_effect, gain_effect, rest_effect)
        }
        [look, reveal, choose, move_effect, gain_effect, rest_effect] => {
            let look_view = look.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
            let reveal_view = reveal.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
            if look_view.reveal || reveal_view.tag != look_view.tag {
                return None;
            }
            (look, choose, move_effect, gain_effect, rest_effect)
        }
        _ => return None,
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (_, move_to_hand) = for_each_tagged_for_compaction(move_effect)?;
    let gain = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    let (_, rest) = for_each_tagged_for_compaction(rest_effect)?;

    if look.player != PlayerFilter::You
        || choose.chooser != PlayerFilter::You
        || choose_exact_count(choose) != Some(1)
        || !choose_references_tag(choose, &look.tag)
        || !for_each_moves_tag_to_hand(move_to_hand, choose.tag.as_str())
        || gain.player != ChooseSpec::Player(PlayerFilter::You)
        || !matches!(
            gain.amount.unhinted(),
            Value::ManaValueOf(spec)
                if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        )
        || !for_each_moves_unselected_to_zone(
            rest,
            look.tag.as_str(),
            choose.tag.as_str(),
            Zone::Graveyard,
        )
    {
        return None;
    }

    let (count_text, noun, _) = describe_look_count_and_noun(&look.count);
    Some(format!(
        "Reveal the top {count_text} {noun} of your library and put one of them into your hand. You gain life equal to that card's mana value. Put all other cards revealed this way into your graveyard"
    ))
}

pub(super) fn describe_reveal_top_choice_to_hand_rest_graveyard_structural(
    effects: &[Effect],
) -> Option<String> {
    if effects.len() < 5 {
        return None;
    }
    let look = effects[0].downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let reveal = effects[1].downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if look.player != PlayerFilter::You || look.reveal || reveal.tag != look.tag {
        return None;
    }

    let mut chooses: Vec<&crate::effects::ChooseObjectsEffect> = Vec::new();
    let mut chosen_tag: Option<TagKey> = None;
    let mut idx = 2usize;
    while let Some(choose) = effects
        .get(idx)
        .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
    {
        if choose.chooser != PlayerFilter::You
            || choose.is_search
            || choose.count.min != 0
            || !matches!(choose.count.max, Some(1) | None)
            || !choose_references_tag(choose, &look.tag)
        {
            return None;
        }
        if let Some(existing) = &chosen_tag {
            if choose.tag != *existing {
                return None;
            }
        } else {
            chosen_tag = Some(choose.tag.clone());
        }
        chooses.push(choose);
        idx += 1;
    }
    let chosen_tag = chosen_tag?;
    if chooses.is_empty()
        || effects.len() != idx + 2
        || !for_each_moves_tagged_iterated_to_hand(&effects[idx], &chosen_tag)
        || !for_each_moves_unchosen_iterated_to_zone(
            &effects[idx + 1],
            &look.tag,
            &chosen_tag,
            Zone::Graveyard,
        )
    {
        return None;
    }

    let owner = describe_possessive_player_filter(&look.player);
    let (count_text, noun, _) = describe_look_count_and_noun(&look.count);
    let choice = if let [choose] = chooses.as_slice() {
        if choose.count.is_any_number() {
            format!(
                "any number of {}",
                describe_any_number_filter_from_looked_cards(look, choose)?
            )
        } else if let Some(label) = structural_revealed_choice_label(choose) {
            structural_revealed_choice_phrase(&label)
        } else {
            describe_choose_filter_from_looked_cards(look, choose)?
        }
    } else {
        if chooses.iter().any(|choose| choose.count.max.is_none()) {
            return None;
        }
        chooses
            .iter()
            .map(|choose| {
                structural_revealed_choice_label(choose)
                    .map(|label| structural_revealed_choice_phrase(&label))
            })
            .collect::<Option<Vec<_>>>()?
            .join(" and/or ")
    };
    Some(format!(
        "Reveal the top {count_text} {noun} of {owner} library. You may put {choice} from among them into your hand. Put the rest into your graveyard"
    ))
}

pub(super) fn tagged_apply_continuous_view(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::ApplyContinuousEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let apply = tagged
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    Some((&tagged.tag, apply))
}

pub(super) fn tagged_untap_effect_view(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::UntapEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let untap = tagged
        .effect
        .downcast_ref::<crate::effects::UntapEffect>()?;
    Some((&tagged.tag, untap))
}

pub(super) fn tagged_put_counters_effect_view(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::PutCountersEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let put = tagged
        .effect
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    Some((&tagged.tag, put))
}

pub(super) fn is_target_only_opponent(effect: &Effect) -> bool {
    effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .is_some_and(|target_only| {
            matches!(
                target_only.target.base(),
                ChooseSpec::Player(PlayerFilter::Opponent)
            )
        })
}

pub(super) fn reciprocal_creature_tag_matching(
    effect: &Effect,
    controller: &PlayerFilter,
) -> Option<crate::TagKey> {
    let tag_matching = effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    if tag_matching.zone != Some(Zone::Battlefield)
        || !tag_matching.additional_zones.is_empty()
        || tag_matching.filter.zone != Some(Zone::Battlefield)
        || tag_matching.filter.card_types.as_slice() != [CardType::Creature]
        || tag_matching.filter.controller.as_ref() != Some(controller)
    {
        return None;
    }
    Some(tag_matching.tag.clone())
}

pub(super) fn apply_changes_control_to_effect_controller_for_tag(
    effect: &Effect,
    tag: &crate::TagKey,
) -> bool {
    let Some((_, apply)) = tagged_apply_continuous_view(effect) else {
        return false;
    };
    apply.target == crate::continuous::EffectTarget::Source
        && apply.until == Until::EndOfTurn
        && apply.condition.is_none()
        && apply.modification.is_none()
        && apply.additional_modifications.is_empty()
        && matches!(
            apply.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
        && apply
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_references_tagged_object(spec, tag))
}

pub(super) fn apply_changes_control_to_target_opponent_for_tag(
    effect: &Effect,
    tag: &crate::TagKey,
) -> bool {
    let Some((_, apply)) = tagged_apply_continuous_view(effect) else {
        return false;
    };
    apply.target == crate::continuous::EffectTarget::Source
        && apply.until == Until::EndOfTurn
        && apply.condition.is_none()
        && apply.modification.is_none()
        && apply.additional_modifications.is_empty()
        && matches!(
            apply.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player)]
                if matches!(player, PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Opponent))
        )
        && apply
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_references_tagged_object(spec, tag))
}

pub(super) fn object_filter_references_tag_recursive(
    filter: &ObjectFilter,
    tag: &crate::TagKey,
) -> bool {
    filter_references_tag(filter, tag)
        || filter
            .any_of
            .iter()
            .any(|candidate| object_filter_references_tag_recursive(candidate, tag))
}

pub(super) fn choose_spec_references_tagged_filter_recursive(
    spec: &ChooseSpec,
    tag: &crate::TagKey,
) -> bool {
    match spec.base() {
        ChooseSpec::Tagged(found) => found == tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            object_filter_references_tag_recursive(filter, tag)
        }
        _ => false,
    }
}

pub(super) fn choose_spec_references_both_tags(
    spec: &ChooseSpec,
    first: &crate::TagKey,
    second: &crate::TagKey,
) -> bool {
    choose_spec_references_tagged_filter_recursive(spec, first)
        && choose_spec_references_tagged_filter_recursive(spec, second)
}

pub(super) fn untaps_both_tagged_groups(
    effect: &Effect,
    first: &crate::TagKey,
    second: &crate::TagKey,
) -> bool {
    let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>() else {
        return false;
    };
    choose_spec_references_both_tags(&untap.target, first, second)
}

pub(super) fn tags_both_tagged_groups(
    effect: &Effect,
    first: &crate::TagKey,
    second: &crate::TagKey,
) -> bool {
    let Some(tag_matching) = effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
    else {
        return false;
    };
    tag_matching.zone.is_none()
        && tag_matching.additional_zones.is_empty()
        && object_filter_references_tag_recursive(&tag_matching.filter, first)
        && object_filter_references_tag_recursive(&tag_matching.filter, second)
}

pub(super) fn grants_haste_to_both_tagged_groups(
    effect: &Effect,
    first: &crate::TagKey,
    second: &crate::TagKey,
) -> bool {
    let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() else {
        return false;
    };
    if apply.until != Until::EndOfTurn
        || apply.condition.is_some()
        || !apply.runtime_modifications.is_empty()
        || !apply.additional_modifications.is_empty()
        || !matches!(
            &apply.modification,
            Some(crate::continuous::Modification::AddAbility(ability))
                if ability.id() == crate::static_abilities::StaticAbilityId::Haste
        )
    {
        return false;
    }
    match &apply.target {
        crate::continuous::EffectTarget::Filter(filter) => {
            object_filter_references_tag_recursive(filter, first)
                && object_filter_references_tag_recursive(filter, second)
        }
        _ => false,
    }
}

pub(super) fn describe_reciprocal_creature_control_structural(
    effects: &[Effect],
) -> Option<String> {
    let effects = if let [first, rest @ ..] = effects
        && is_target_only_opponent(first)
    {
        rest
    } else {
        effects
    };
    let [tag_yours, tag_theirs, tail @ ..] = effects else {
        return None;
    };

    let target_opponent = PlayerFilter::Target(Box::new(PlayerFilter::Opponent));
    let your_tag = reciprocal_creature_tag_matching(tag_yours, &PlayerFilter::You)?;
    let their_tag = reciprocal_creature_tag_matching(tag_theirs, &target_opponent)?;
    let valid_control_pair = |control_theirs: &Effect, control_yours: &Effect| {
        apply_changes_control_to_effect_controller_for_tag(control_theirs, &their_tag)
            && apply_changes_control_to_target_opponent_for_tag(control_yours, &your_tag)
    };
    let valid_untap = |tag: Option<&Effect>, untap: &Effect| {
        tag.is_none_or(|tag| tags_both_tagged_groups(tag, &your_tag, &their_tag))
            && untaps_both_tagged_groups(untap, &your_tag, &their_tag)
    };

    let untap_before_control = match tail {
        [control_theirs, control_yours, untap, haste]
            if valid_control_pair(control_theirs, control_yours)
                && valid_untap(None, untap)
                && grants_haste_to_both_tagged_groups(haste, &your_tag, &their_tag) =>
        {
            false
        }
        [control_theirs, control_yours, untap_tag, untap, haste]
            if valid_control_pair(control_theirs, control_yours)
                && valid_untap(Some(untap_tag), untap)
                && grants_haste_to_both_tagged_groups(haste, &your_tag, &their_tag) =>
        {
            false
        }
        [untap, control_theirs, control_yours, haste]
            if valid_untap(None, untap)
                && valid_control_pair(control_theirs, control_yours)
                && grants_haste_to_both_tagged_groups(haste, &your_tag, &their_tag) =>
        {
            true
        }
        [untap_tag, untap, control_theirs, control_yours, haste]
            if valid_untap(Some(untap_tag), untap)
                && valid_control_pair(control_theirs, control_yours)
                && grants_haste_to_both_tagged_groups(haste, &your_tag, &their_tag) =>
        {
            true
        }
        _ => return None,
    };

    Some(if untap_before_control {
        "Untap all creatures you control and all creatures target opponent controls. You and that opponent each gain control of all creatures the other controls until end of turn. Those creatures gain haste until end of turn"
            .to_string()
    } else {
        "You and target opponent each gain control of all creatures the other controls until end of turn. Untap those creatures. Those creatures gain haste until end of turn"
            .to_string()
    })
}

pub(super) fn is_gain_control_until_eot(apply: &crate::effects::ApplyContinuousEffect) -> bool {
    apply.target == crate::continuous::EffectTarget::Source
        && apply.until == Until::EndOfTurn
        && apply.condition.is_none()
        && apply.modification.is_none()
        && apply.additional_modifications.is_empty()
        && matches!(
            apply.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
}

pub(super) fn is_haste_until_eot_for_tag(
    apply: &crate::effects::ApplyContinuousEffect,
    tag: &crate::TagKey,
) -> bool {
    apply.target == crate::continuous::EffectTarget::Source
        && apply.until == Until::EndOfTurn
        && apply.condition.is_none()
        && apply.additional_modifications.is_empty()
        && apply.runtime_modifications.is_empty()
        && apply
            .target_spec
            .as_ref()
            .is_some_and(|target| choose_spec_references_tagged_object(target, tag))
        && matches!(
            &apply.modification,
            Some(crate::continuous::Modification::AddAbility(ability))
                if ability.id() == crate::static_abilities::StaticAbilityId::Haste
        )
}

pub(super) fn gain_control_followup_untap_target_text(target: &str) -> &'static str {
    if target.contains("creature") && !target.contains("artifact or creature") {
        "that creature"
    } else if target.contains("permanent") {
        "that permanent"
    } else {
        "it"
    }
}

fn gain_control_object_reference_tag<'a>(
    controlled_tag: &'a TagKey,
    control: &'a crate::effects::ApplyContinuousEffect,
) -> &'a TagKey {
    match control.target_spec.as_ref().map(ChooseSpec::unhinted) {
        Some(ChooseSpec::Tagged(tag)) => tag,
        _ => controlled_tag,
    }
}

pub(super) fn describe_gain_control_then_untap_structural(effects: &[Effect]) -> Option<String> {
    let [control_effect, untap_effect] = effects else {
        return None;
    };
    if let (Some((controlled_tag, control)), Some((_, untap))) = (
        tagged_apply_continuous_view(control_effect),
        tagged_untap_effect_view(untap_effect),
    ) && is_gain_control_until_eot(control)
    {
        let controlled_object_tag = gain_control_object_reference_tag(controlled_tag, control);
        if !choose_spec_references_tagged_object(&untap.target, controlled_object_tag) {
            return None;
        }
        let target = control
            .target_spec
            .as_ref()
            .map(describe_choose_spec)
            .unwrap_or_else(|| "target creature".to_string());
        let untap_target = gain_control_followup_untap_target_text(&target);
        return Some(format!(
            "Gain control of {target} until end of turn. Untap {untap_target}"
        ));
    }

    let (untapped_tag, untap) = tagged_untap_effect_view(control_effect)?;
    let (_, control) = tagged_apply_continuous_view(untap_effect)?;
    if !is_gain_control_until_eot(control)
        || !control
            .target_spec
            .as_ref()
            .is_some_and(|target| choose_spec_references_tagged_object(target, untapped_tag))
    {
        return None;
    }
    Some(format!(
        "Untap {} and gain control of it until end of turn",
        describe_choose_spec(&untap.target)
    ))
}

pub(super) fn describe_gain_control_untap_haste_structural(effects: &[Effect]) -> Option<String> {
    let [first, second, third] = effects else {
        return None;
    };

    if let (Some((controlled_tag, control)), Some((untapped_tag, untap)), Some((_, haste))) = (
        tagged_apply_continuous_view(first),
        tagged_untap_effect_view(second),
        tagged_apply_continuous_view(third),
    ) && is_gain_control_until_eot(control)
    {
        let controlled_object_tag = gain_control_object_reference_tag(controlled_tag, control);
        if !choose_spec_references_tagged_object(&untap.target, controlled_object_tag)
            || !(is_haste_until_eot_for_tag(haste, untapped_tag)
                || is_haste_until_eot_for_tag(haste, controlled_object_tag))
        {
            return None;
        }
        let target = control
            .target_spec
            .as_ref()
            .map(describe_choose_spec)
            .unwrap_or_else(|| "target creature".to_string());
        let untap_target = gain_control_followup_untap_target_text(&target);
        return Some(format!(
            "Gain control of {target} until end of turn. Untap {untap_target}. It gains haste until end of turn"
        ));
    }

    if let (Some((untapped_tag, untap)), Some((controlled_tag, control)), Some((_, haste))) = (
        tagged_untap_effect_view(first),
        tagged_apply_continuous_view(second),
        tagged_apply_continuous_view(third),
    ) && is_gain_control_until_eot(control)
        && control
            .target_spec
            .as_ref()
            .is_some_and(|target| choose_spec_references_tagged_object(target, untapped_tag))
        && (is_haste_until_eot_for_tag(haste, controlled_tag)
            || is_haste_until_eot_for_tag(haste, untapped_tag))
    {
        let target = describe_choose_spec(&untap.target);
        let followup_subject = match gain_control_followup_untap_target_text(&target) {
            "that creature" => "That creature",
            "that permanent" => "That permanent",
            _ => "It",
        };
        return Some(format!(
            "Untap {target} and gain control of it until end of turn. {followup_subject} gains haste until end of turn"
        ));
    }

    None
}

fn describe_gain_control_untap_haste_clause_structural(effects: &[Effect]) -> Option<String> {
    let [control_effect, untap_effect, haste_effect] = effects else {
        return None;
    };
    let (controlled_tag, control) = tagged_apply_continuous_view(control_effect)?;
    let (untapped_tag, untap) = tagged_untap_effect_view(untap_effect)?;
    let (_, haste) = tagged_apply_continuous_view(haste_effect)?;
    if !is_gain_control_until_eot(control) {
        return None;
    }

    // The comma-joined surface is for a continuation such as Jet's
    // Brainwashing, where an earlier clause has already selected the object
    // and this conditional refers back to it.  A standalone theft effect
    // selects its target here and Oracle keeps control, untap, and haste as
    // separate sentences.
    if !matches!(
        control.target_spec.as_ref().map(ChooseSpec::unhinted),
        Some(ChooseSpec::Tagged(_))
    ) {
        return None;
    }

    let controlled_object_tag = gain_control_object_reference_tag(controlled_tag, control);
    if !choose_spec_references_tagged_object(&untap.target, controlled_object_tag)
        || !(is_haste_until_eot_for_tag(haste, untapped_tag)
            || is_haste_until_eot_for_tag(haste, controlled_object_tag))
    {
        return None;
    }

    let target = control
        .target_spec
        .as_ref()
        .map(describe_choose_spec)
        .unwrap_or_else(|| "target creature".to_string());
    let untap_target = gain_control_followup_untap_target_text(&target);
    Some(format!(
        "Gain control of {target} until end of turn, untap {untap_target}, and it gains haste until end of turn"
    ))
}

pub(super) fn describe_gain_control_counter_untap_haste_structural(
    effects: &[Effect],
) -> Option<String> {
    let [control_effect, counter_effect, untap_effect, haste_effect] = effects else {
        return None;
    };
    let (controlled_tag, control) = tagged_apply_continuous_view(control_effect)?;
    let (_, put) = tagged_put_counters_effect_view(counter_effect)?;
    let (untapped_tag, untap) = tagged_untap_effect_view(untap_effect)?;
    let (_, haste) = tagged_apply_continuous_view(haste_effect)?;
    let controlled_object_tag = gain_control_object_reference_tag(controlled_tag, control);
    if !is_gain_control_until_eot(control)
        || put.distributed
        || put.target_count.is_some()
        || !choose_spec_references_tagged_object(&put.target, controlled_object_tag)
        || !choose_spec_references_tagged_object(&untap.target, controlled_object_tag)
        || !(is_haste_until_eot_for_tag(haste, untapped_tag)
            || is_haste_until_eot_for_tag(haste, controlled_object_tag))
    {
        return None;
    }

    let target = control
        .target_spec
        .as_ref()
        .map(describe_choose_spec)
        .unwrap_or_else(|| "target creature".to_string());
    let final_subject = if gain_control_followup_untap_target_text(&target) == "that creature" {
        "That creature"
    } else {
        "It"
    };
    Some(format!(
        "Gain control of {target} until end of turn. Put {} on it and untap it. {final_subject} gains haste until end of turn",
        describe_put_counter_phrase(&put.amount, put.counter_type)
    ))
}

pub(super) fn describe_put_counters_then_untap_same_target_structural(
    effects: &[Effect],
) -> Option<String> {
    let [counter_effect, untap_effect] = effects else {
        return None;
    };
    let (countered_tag, put) = tagged_put_counters_effect_view(counter_effect)?;
    let (_, untap) = tagged_untap_effect_view(untap_effect)?;
    let count = put.target.count();
    if put.distributed
        || put
            .target_count
            .as_ref()
            .is_some_and(|target_count| target_count != &count)
        || !put.target.is_target()
        || count.max != Some(1)
        || count.dynamic_x
        || count.random
        || !matches!(untap.target.base(), ChooseSpec::Tagged(tag) if tag == countered_tag)
    {
        return None;
    }

    Some(format!(
        "{}. Untap it",
        describe_effect(counter_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_must_block_untap_then_others_cant_block_structural(
    effects: &[Effect],
) -> Option<String> {
    let [must_block_effect, untap_effect, cant_block_effect] = effects else {
        return None;
    };
    let (affected_tag, must_block) = tagged_apply_continuous_view(must_block_effect)?;
    let (_, untap) = tagged_untap_effect_view(untap_effect)?;
    let cant = cant_block_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Block(filter) = &cant.restriction else {
        return None;
    };
    let target = must_block.target_spec.as_ref()?;
    if must_block.target != crate::continuous::EffectTarget::Source
        || must_block.until != Until::EndOfTurn
        || must_block.condition.is_some()
        || !must_block.additional_modifications.is_empty()
        || !must_block.runtime_modifications.is_empty()
        || !matches!(
            &must_block.modification,
            Some(crate::continuous::Modification::AddAbility(ability))
                if ability.id() == crate::static_abilities::StaticAbilityId::MustBlock
        )
        || !matches!(untap.target.base(), ChooseSpec::Tagged(tag) if tag == affected_tag)
        || cant.duration != Until::EndOfTurn
    {
        return None;
    }

    let mut expected_filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
    expected_filter.other = true;
    expected_filter.controller = Some(PlayerFilter::AliasedControllerOf(
        crate::filter::ObjectRef::Tagged(affected_tag.clone()),
    ));
    if filter != &expected_filter {
        return None;
    }

    let target = describe_choose_spec(target);
    Some(format!(
        "{} blocks this turn if able. Untap that creature. Other creatures that player controls can't block this turn",
        capitalize_first(&target)
    ))
}

#[cfg(test)]
mod control_reference_surface_tests {
    use super::*;

    fn gain_control(target: ChooseSpec, tag: &TagKey) -> Effect {
        let mut control = crate::effects::ApplyContinuousEffect::new_runtime(
            crate::continuous::EffectTarget::Source,
            crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
            Until::EndOfTurn,
        );
        control.target_spec = Some(target);
        Effect::new(control).tag(tag.clone())
    }

    fn untap(tag: &TagKey) -> Effect {
        Effect::untap(ChooseSpec::Tagged(tag.clone())).tag("untapped")
    }

    fn untap_tagged_creature(reference_tag: &TagKey, effect_tag: &TagKey) -> Effect {
        let target = ChooseSpec::Object(
            ObjectFilter::creature()
                .in_zone(Zone::Battlefield)
                .match_tagged(
                    reference_tag.clone(),
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                ),
        )
        .with_surface_hint(crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("that creature".to_string()),
        ));
        Effect::untap(target).tag(effect_tag.clone())
    }

    fn grant_haste(tag: &TagKey) -> Effect {
        let mut haste = crate::effects::ApplyContinuousEffect::new(
            crate::continuous::EffectTarget::Source,
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::haste(),
            ),
            Until::EndOfTurn,
        );
        haste.target_spec = Some(ChooseSpec::Tagged(tag.clone()));
        Effect::new(haste).tag("granted")
    }

    fn grant_haste_to_outer_tag(tag: &TagKey) -> Effect {
        let mut haste = crate::effects::ApplyContinuousEffect::new(
            crate::continuous::EffectTarget::Source,
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::haste(),
            ),
            Until::EndOfTurn,
        );
        haste.target_spec = Some(ChooseSpec::Tagged(tag.clone()).with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType("it".to_string()),
            ),
        ));
        Effect::new(haste).tag("granted")
    }

    #[test]
    fn renders_attached_permanent_control_chain_with_sentence_carry() {
        let enchanted = TagKey::from("enchanted");
        let effects = vec![
            Effect::tag_attached_to_source(enchanted.clone()),
            gain_control(
                ChooseSpec::Tagged(enchanted.clone()),
                &TagKey::from("controlled"),
            ),
            untap(&enchanted),
            grant_haste(&enchanted),
        ];

        assert_eq!(
            describe_pre_clause_structural_effect_list(&effects).as_deref(),
            Some(
                "Gain control of enchanted permanent until end of turn. Untap that permanent. It gains haste until end of turn"
            )
        );
    }

    #[test]
    fn standalone_control_untap_haste_preserves_sentence_boundaries() {
        let controlled = TagKey::from("controlled_0");
        let untapped = TagKey::from("untapped_1");
        let effects = vec![
            gain_control(
                ChooseSpec::target(ChooseSpec::Object(
                    ObjectFilter::creature().in_zone(Zone::Battlefield),
                )),
                &controlled,
            ),
            untap_tagged_creature(&controlled, &untapped),
            grant_haste_to_outer_tag(&untapped),
        ];

        assert_eq!(
            describe_effect_clause_list(&effects).as_deref(),
            Some(
                "gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn"
            )
        );
    }

    #[test]
    fn existing_target_control_untap_haste_uses_one_conditional_clause() {
        let selected = TagKey::from("selected");
        let controlled = TagKey::from("controlled");
        let untapped = TagKey::from("untapped");
        let effects = vec![
            gain_control(ChooseSpec::Tagged(selected.clone()), &controlled),
            untap_tagged_creature(&selected, &untapped),
            grant_haste_to_outer_tag(&untapped),
        ];

        assert_eq!(
            describe_effect_clause_list(&effects).as_deref(),
            Some(
                "gain control of it until end of turn, untap it, and it gains haste until end of turn"
            )
        );
    }

    #[test]
    fn renders_control_counter_untap_haste_as_three_sentences() {
        let controlled = TagKey::from("controlled");
        let effects = vec![
            gain_control(
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
                &controlled,
            ),
            Effect::plus_one_counters(Value::Fixed(1), ChooseSpec::Tagged(controlled.clone()))
                .tag("countered"),
            untap(&controlled),
            grant_haste(&controlled),
        ];

        assert_eq!(
            describe_gain_control_counter_untap_haste_structural(&effects).as_deref(),
            Some(
                "Gain control of target creature until end of turn. Put a +1/+1 counter on it and untap it. That creature gains haste until end of turn"
            )
        );
    }

    #[test]
    fn renders_optional_single_counter_target_then_untap_as_separate_sentences() {
        let countered = TagKey::from("countered");
        let target = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::default().with_subtype(Subtype::Elf),
        ))
        .with_count(ChoiceCount::up_to(1));
        let effects = vec![
            Effect::new(
                crate::effects::PutCountersEffect::plus_one_counters(Value::Fixed(1), target)
                    .with_target_count(ChoiceCount::up_to(1)),
            )
            .tag(countered.clone()),
            untap(&countered),
        ];

        assert_eq!(
            describe_put_counters_then_untap_same_target_structural(&effects).as_deref(),
            Some("Put a +1/+1 counter on up to one target Elf. Untap it")
        );
    }

    #[test]
    fn renders_must_block_untap_and_controller_lockout_with_sentence_carry() {
        let affected = TagKey::from("granted");
        let target = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
        ));
        let mut must_block = crate::effects::ApplyContinuousEffect::new(
            crate::continuous::EffectTarget::Source,
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::must_block(),
            ),
            Until::EndOfTurn,
        );
        must_block.target_spec = Some(target);
        let mut other_creatures = ObjectFilter::creature().in_zone(Zone::Battlefield);
        other_creatures.other = true;
        other_creatures.controller = Some(PlayerFilter::AliasedControllerOf(
            crate::filter::ObjectRef::Tagged(affected.clone()),
        ));
        let effects = vec![
            Effect::new(must_block).tag(affected.clone()),
            untap(&affected),
            Effect::cant_until(
                crate::effect::Restriction::block(other_creatures),
                Until::EndOfTurn,
            ),
        ];

        assert_eq!(
            describe_must_block_untap_then_others_cant_block_structural(&effects).as_deref(),
            Some(
                "Target creature an opponent controls blocks this turn if able. Untap that creature. Other creatures that player controls can't block this turn"
            )
        );
    }
}

pub(super) fn choose_spec_has_equipment_filter(spec: &ChooseSpec) -> bool {
    matches!(
        spec.base(),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter)
            if filter.subtypes.contains(&Subtype::Equipment)
    )
}

fn choose_spec_has_aura_filter(spec: &ChooseSpec) -> bool {
    match spec.unhinted() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.subtypes.contains(&Subtype::Aura)
                || filter
                    .any_of
                    .iter()
                    .any(|branch| branch.subtypes.contains(&Subtype::Aura))
        }
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => choose_spec_has_aura_filter(inner),
        _ => false,
    }
}

pub(super) fn is_gain_control_effect(apply: &crate::effects::ApplyContinuousEffect) -> bool {
    apply.target == crate::continuous::EffectTarget::Source
        && apply.condition.is_none()
        && apply.modification.is_none()
        && apply.additional_modifications.is_empty()
        && matches!(
            apply.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
}

pub(super) fn describe_gain_control_create_token_attach_sequence(
    effects: &[Effect],
) -> Option<String> {
    let [gain_effect, create_effect, attach_effect] = effects else {
        return None;
    };

    let (controlled_tag, control) = tagged_apply_continuous_view(gain_effect)?;
    if !is_gain_control_effect(control)
        || !control
            .target_spec
            .as_ref()
            .is_some_and(choose_spec_has_equipment_filter)
    {
        return None;
    }

    let (created_tag, create_token) = tagged_create_token_effect(create_effect)?;
    if create_token.count != Value::Fixed(1) {
        return None;
    }

    let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !matches!(&attach.target, ChooseSpec::Tagged(tag) if tag == created_tag)
        || !choose_spec_has_equipment_filter(&attach.objects)
        || (!choose_spec_references_tagged_object(&attach.objects, controlled_tag)
            && !choose_spec_references_tagged_object(&attach.objects, created_tag))
    {
        return None;
    }

    let gain_text = describe_effect(gain_effect)
        .trim_end_matches('.')
        .to_string();
    let create_text = lowercase_first(describe_effect(create_effect).trim_end_matches('.'));
    Some(format!(
        "{gain_text}, then {create_text} and attach that Equipment to it"
    ))
}

/// Gain control of an Aura and move that same Aura to a legal new host. The
/// Aura qualifier is structural, and runtime attachment already checks the
/// Aura's current enchant restriction, so "it can enchant" is not a guessed
/// surface-only promise.
fn describe_gain_control_aura_then_legal_attach(effects: &[Effect]) -> Option<String> {
    let [gain_effect, attach_effect] = effects else {
        return None;
    };
    let (aura_tag, control) = tagged_apply_continuous_view(gain_effect)?;
    if !is_gain_control_effect(control)
        || !control
            .target_spec
            .as_ref()
            .is_some_and(choose_spec_has_aura_filter)
    {
        return None;
    }
    let attach = structural_unwrap_render_wrappers(attach_effect)
        .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !choose_spec_references_exact_tag(&attach.objects, aura_tag) {
        return None;
    }
    let target = describe_choose_spec(&attach.target);
    let gain = describe_effect(gain_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Some(format!("{gain}, then attach it to {target} it can enchant"))
}

fn choose_spec_is_source(spec: &ChooseSpec) -> bool {
    match spec.unhinted() {
        ChooseSpec::Source => true,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter.source,
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => choose_spec_is_source(inner),
        _ => false,
    }
}

fn effect_produces_attachment_target(effect: &Effect, target_tag: &TagKey) -> bool {
    let Some(producer_tag) = effect_outer_tag(effect) else {
        return false;
    };
    if producer_tag != target_tag {
        return false;
    }
    let producer = structural_unwrap_render_wrappers(effect);
    producer
        .downcast_ref::<crate::effects::CreateTokenEffect>()
        .is_some()
        || producer
            .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
            .is_some()
        || producer
            .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
            .is_some()
        || producer
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|move_effect| move_effect.zone == Zone::Battlefield)
}

fn with_id_sacrifice(effect: &Effect) -> Option<&crate::effects::WithIdEffect> {
    let with_id = effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    sacrifice_view(&with_id.effect)?;
    Some(with_id)
}

/// Join a typed producer/sacrifice with the immediately linked source
/// attachment. Producers preserve the explicit "then" ordering used by token
/// Equipment abilities; linked sacrifice clauses remain a conjunction. This
/// keeps the attachment in the same oracle instruction rather than emitting a
/// misleading new sentence.
fn describe_linked_source_attachment_prefix(effects: &[Effect]) -> Option<String> {
    let [first, second, rest @ ..] = effects else {
        return None;
    };
    let attach = structural_unwrap_render_wrappers(second)
        .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !choose_spec_is_source(&attach.objects) {
        return None;
    }

    let linked_producer = match attach.target.unhinted() {
        ChooseSpec::Tagged(tag) => effect_produces_attachment_target(first, tag),
        _ => false,
    };
    let linked_sacrifice = with_id_sacrifice(first).is_some_and(|with_id| {
        rest.first()
            .and_then(|effect| effect.downcast_ref::<crate::effects::IfEffect>())
            .is_some_and(|if_effect| if_effect.condition == with_id.id)
    });
    if !linked_producer && !linked_sacrifice {
        return None;
    }

    let first = describe_effect(first)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let attach = lowercase_first(describe_effect(second).trim().trim_end_matches('.'));
    let connector = if linked_producer { ", then" } else { " and" };
    let prefix = format!("{first}{connector} {attach}");
    if rest.is_empty() {
        return Some(prefix);
    }
    let suffix = describe_effect_clause_list(rest).unwrap_or_else(|| describe_effect_list(rest));
    Some(format!(
        "{prefix}. {}",
        capitalize_first(suffix.trim().trim_end_matches('.'))
    ))
}

pub(super) fn create_token_attachment_can_compact(
    create_token: &crate::effects::CreateTokenEffect,
) -> bool {
    create_token.count == Value::Fixed(1)
        && matches!(&create_token.controller, PlayerFilter::You)
        && create_token.controller_target.is_none()
        && !create_token.enters_tapped
        && !create_token.enters_attacking
        && !create_token.exile_at_end_of_combat
        && !create_token.sacrifice_at_end_of_combat
        && !create_token.sacrifice_at_next_end_step
        && !create_token.exile_at_next_end_step
        && (create_token.token.card.subtypes.contains(&Subtype::Aura)
            || create_token.token.card.subtypes.contains(&Subtype::Role))
}

pub(super) fn describe_create_token_attached_to_target(
    create_effect: &Effect,
    attach_effect: &Effect,
) -> Option<String> {
    let (created_tag, create_token) = tagged_create_token_effect(create_effect)?;
    let attach = unwrap_for_each_attachment_wrappers(attach_effect)
        .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !create_token_attachment_can_compact(create_token)
        || !choose_spec_references_exact_tag(&attach.objects, created_tag)
    {
        return None;
    }

    let token = with_indefinite_article(&describe_token_blueprint(&create_token.token));
    Some(format!(
        "Create {token} attached to {}",
        describe_choose_spec(&attach.target)
    ))
}

/// Render a tagged single-target selection followed by an until-end-of-turn
/// cast grant on the same tag. Graveyard targets use the compact Oracle surface
/// "You may cast target ... from your graveyard this turn"; other structural
/// uses retain the explicit choose-then-grant form.
pub(super) fn describe_target_card_then_cast_this_turn_structural(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, grant_effect] = effects else {
        return None;
    };
    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if &grant.tag != target_tag
        || grant.player != PlayerFilter::You
        || grant.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || grant.allow_land
        || grant.allow_any_color_for_cast
        || grant.while_on_top_of_library
        || grant.filter.is_some()
        || grant.cast_pool_is_plural
        || choose_spec_is_plural(&target_only.target)
        || choose_spec_allows_multiple(&target_only.target)
    {
        return None;
    }
    let target_text = describe_choose_spec(&target_only.target);
    if target_only.target.is_target()
        && matches!(
            target_only.target.base(),
            ChooseSpec::Object(filter)
                if filter.zone == Some(Zone::Graveyard)
                    && filter.owner == Some(PlayerFilter::You)
        )
    {
        return Some(format!(
            "You may cast {} this turn",
            target_text.replace(" in your graveyard", " from your graveyard")
        ));
    }
    Some(format!(
        "Choose {}. You may cast that card this turn",
        target_text
    ))
}

pub(super) fn describe_choose_top_exile_then_play_structural(effects: &[Effect]) -> Option<String> {
    let [choose_effect, exile_effect, grant_effect] = effects else {
        return None;
    };
    let choose = unwrap_basic_tag_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile =
        unwrap_basic_tag_wrappers(exile_effect).downcast_ref::<crate::effects::ExileEffect>()?;
    let grant = unwrap_basic_tag_wrappers(grant_effect)
        .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Library)
        || choose.filter.owner != Some(PlayerFilter::You)
        || !choose.top_only
        || choose_exact_count(choose) != Some(1)
        || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || exile.face_down
        || (grant.tag != choose.tag && grant.tag.as_str() != crate::tag::SOURCE_EXILED_TAG)
        || grant.player != PlayerFilter::You
        || grant.while_on_top_of_library
        || grant.filter.is_some()
        || grant.cast_pool_is_plural
    {
        return None;
    }

    let verb = if grant.allow_land { "play" } else { "cast" };
    let spell_ref = if grant.allow_land {
        "that spell"
    } else {
        "that card"
    };
    let mana_suffix = grant
        .mana_spend_cast_clause(spell_ref)
        .map(|clause| format!(", and {clause}"))
        .unwrap_or_default();
    let permission = match grant.duration {
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn => {
            format!("You may {verb} that card this turn{mana_suffix}")
        }
        crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
            format!("You may {verb} that card until the end of your next turn{mana_suffix}")
        }
        crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep => {
            format!("You may {verb} that card until your next end step{mana_suffix}")
        }
        _ => return None,
    };
    Some(format!("Exile the top card of your library. {permission}"))
}

pub(super) fn describe_target_modifications_then_exile_top_play(
    effects: &[Effect],
) -> Option<String> {
    let [
        first_modification,
        second_modification,
        exile_effect,
        grant_effect,
    ] = effects
    else {
        return None;
    };
    let modification_text = capitalize_first(&describe_compact_tagged_apply_continuous_pair(
        first_modification,
        second_modification,
    )?);
    let exile = exile_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    let [moved_tag] = exile.moved_tags.as_slice() else {
        return None;
    };
    if exile.count != Value::Fixed(1)
        || exile.player != PlayerFilter::You
        || !exile.accumulated_tags.is_empty()
        || grant.tag != *moved_tag
        || grant.player != PlayerFilter::You
        || grant.allow_any_color_for_cast
        || grant.while_on_top_of_library
        || grant.filter.is_some()
        || grant.cast_pool_is_plural
    {
        return None;
    }

    let duration = match grant.duration {
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn => "this turn",
        crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
            "until the end of your next turn"
        }
        crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep => "until your next end step",
        _ => return None,
    };
    let verb = if grant.allow_land { "play" } else { "cast" };
    let (exile_text, singular) = describe_exile_top_clause(exile, false)?;
    if !singular {
        return None;
    }

    Some(format!(
        "{modification_text}. {exile_text}. You may {verb} it {duration}"
    ))
}

pub(super) fn describe_draw_replacement_exile_top_play(
    player: &PlayerFilter,
    effects: &[Effect],
) -> Option<String> {
    let grant = match effects {
        [choose_effect, exile_effect, grant_effect] => {
            let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let exile = exile_effect.downcast_ref::<crate::effects::ExileEffect>()?;
            let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
            if &choose.chooser != player
                || choose_primary_zone(choose) != Some(Zone::Library)
                || choose.filter.owner.as_ref() != Some(player)
                || !choose.top_only
                || choose_exact_count(choose) != Some(1)
                || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
                || exile.face_down
                || (grant.tag != choose.tag && grant.tag.as_str() != crate::tag::SOURCE_EXILED_TAG)
            {
                return None;
            }
            grant
        }
        [exile_top_effect, grant_effect] => {
            let exile_top =
                exile_top_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
            let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
            let [moved_tag] = exile_top.moved_tags.as_slice() else {
                return None;
            };
            if &exile_top.player != player
                || !matches!(&exile_top.count, Value::Fixed(1))
                || !exile_top.accumulated_tags.is_empty()
                || grant.tag != *moved_tag
                || grant.allow_any_color_for_cast
                || grant.while_on_top_of_library
                || grant.filter.is_some()
                || grant.cast_pool_is_plural
            {
                return None;
            }
            grant
        }
        _ => return None,
    };
    if grant.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || !grant.allow_land
    {
        return None;
    }
    let grants_to_replacement_player = grant.player == *player
        || matches!(
            &grant.player,
            PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(tag))
                if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
        );
    if !grants_to_replacement_player {
        return None;
    }

    let subject = if *player == PlayerFilter::IteratedPlayer {
        "they".to_string()
    } else {
        describe_player_filter(player)
    };
    let possessive = if *player == PlayerFilter::IteratedPlayer {
        "their".to_string()
    } else {
        describe_possessive_player_filter(player)
    };
    let verb = if subject == "they" {
        "exile"
    } else {
        player_verb(&subject, "exile", "exiles")
    };
    Some(format!(
        "{subject} {verb} the top card of {possessive} library. {} may play it this turn",
        capitalize_first(&subject)
    ))
}

pub(super) fn describe_choose_top_exile_then_conditional_cast_structural(
    effects: &[Effect],
) -> Option<String> {
    fn is_nonland_card_filter(filter: &ObjectFilter) -> bool {
        if filter.excluded_card_types.as_slice() != [CardType::Land] {
            return false;
        }
        let mut base = filter.clone();
        base.excluded_card_types.clear();
        base.zone = None;
        base == ObjectFilter::default()
    }

    fn choose_subject(chooser: &PlayerFilter) -> (String, String) {
        if matches!(chooser, PlayerFilter::You) {
            return ("you".to_string(), "your".to_string());
        }
        if matches!(
            chooser,
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
                if tag.as_str() == "triggering"
        ) {
            return ("that player".to_string(), "their".to_string());
        }
        (
            describe_player_filter(chooser),
            describe_possessive_player_filter(chooser),
        )
    }

    let (player, exiled_tag, conditional, allow_source_exiled_alias) = match effects {
        [choose_effect, exile_effect, conditional_effect] => {
            let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let exile = exile_effect.downcast_ref::<crate::effects::ExileEffect>()?;
            let conditional =
                conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
            if choose.is_search
                || choose_primary_zone(choose) != Some(Zone::Library)
                || choose.filter.owner.as_ref() != Some(&choose.chooser)
                || !choose.top_only
                || choose_exact_count(choose) != Some(1)
                || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
                || exile.face_down
            {
                return None;
            }
            (&choose.chooser, &choose.tag, conditional, true)
        }
        [exile_top_effect, conditional_effect] => {
            let exile_top =
                exile_top_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
            let conditional =
                conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
            let [moved_tag] = exile_top.moved_tags.as_slice() else {
                return None;
            };
            if exile_top.count != Value::Fixed(1) || !exile_top.accumulated_tags.is_empty() {
                return None;
            }
            (&exile_top.player, moved_tag, conditional, false)
        }
        _ => return None,
    };
    if !conditional.if_false.is_empty() {
        return None;
    }

    let Condition::TaggedObjectMatches(condition_tag, filter) = &conditional.condition else {
        return None;
    };
    if condition_tag != exiled_tag
        && !(allow_source_exiled_alias && condition_tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    {
        return None;
    }
    if !is_nonland_card_filter(filter) {
        return None;
    }

    let [may_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let may = unwrap_basic_tag_wrappers(may_effect).downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = unwrap_basic_tag_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag != *exiled_tag
        && !(allow_source_exiled_alias && cast.tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    {
        return None;
    }
    if cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
    {
        return None;
    }

    let (subject, possessive) = choose_subject(player);
    let exile_sentence = if subject == "you" {
        "Exile the top card of your library".to_string()
    } else {
        format!(
            "{subject} {} the top card of {possessive} library",
            player_verb(&subject, "exile", "exiles")
        )
    };
    Some(format!(
        "{exile_sentence}. If it's a nonland card, you may cast it without paying its mana cost"
    ))
}

pub(super) fn describe_choose_name_target_mills_conditional_draw(
    effects: &[Effect],
) -> Option<String> {
    let [
        choose_effect,
        target_effect,
        mill_effect,
        conditional_effect,
    ] = effects
    else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseCardNameEffect>()?;
    if choose.chooser != PlayerFilter::You || choose.filter.is_some() {
        return None;
    }
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.target != ChooseSpec::target_player() {
        return None;
    }
    let tagged_mill = mill_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let mill = tagged_mill
        .effect
        .downcast_ref::<crate::effects::MillEffect>()?;
    if mill.count != Value::Fixed(1) || mill.player != PlayerFilter::target_player() {
        return None;
    }
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::TaggedObjectMatches(milled_tag, filter) = &conditional.condition
    else {
        return None;
    };
    if milled_tag != &tagged_mill.tag {
        return None;
    }
    let mut expected_filter = ObjectFilter::default();
    expected_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: choose.tag.clone(),
            relation: crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
        });
    if filter != &expected_filter {
        return None;
    }
    let [draw_two_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let [draw_one_effect] = conditional.if_false.as_slice() else {
        return None;
    };
    let draw_two = draw_two_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let draw_one = draw_one_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw_two.player != PlayerFilter::You
        || draw_two.count != Value::Fixed(2)
        || draw_one.player != PlayerFilter::You
        || draw_one.count != Value::Fixed(1)
    {
        return None;
    }

    Some(
        "Choose a card name, then target player mills a card. If a card with the chosen name was milled this way, draw two cards. Otherwise, draw a card"
            .to_string(),
    )
}

pub(super) fn describe_exile_then_free_cast_while_exiled_structural(
    effects: &[Effect],
) -> Option<String> {
    let [move_effect, grant_play_effect, grant_free_cast_effect] = effects else {
        return None;
    };
    let tag = structural_effect_tag(move_effect)?;
    let move_to_zone = unwrap_structural_effect_tag(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let grant_play = grant_play_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    let grant_free_cast = grant_free_cast_effect
        .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>(
    )?;
    if move_to_zone.zone != Zone::Exile
        || grant_play.tag != *tag
        || grant_free_cast.tag != *tag
        || grant_play.player != grant_free_cast.player
        || grant_play.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
        || grant_free_cast.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
        || grant_play.allow_land
        || grant_play.allow_any_color_for_cast
        || grant_free_cast.zone != Some(Zone::Exile)
        || grant_free_cast.while_on_top_of_library
    {
        return None;
    }
    if !matches!(
        grant_play.player,
        PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(ref owner_tag)) if owner_tag == tag
    ) {
        return None;
    }

    Some(format!(
        "Exile {}. For as long as that card remains exiled, its owner may cast it without paying its mana cost",
        describe_choose_spec(&move_to_zone.target)
    ))
}

pub(super) fn tagged_damage_view(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::DealDamageEffect)> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let damage = tagged
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()?;
        return Some((&tagged.tag, damage));
    }
    None
}

pub(super) fn damage_each_creature_filter_text(filter: &ObjectFilter) -> Option<String> {
    if filter.zone.is_some_and(|zone| zone != Zone::Battlefield)
        || filter.card_types != vec![CardType::Creature]
        || filter.controller.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.any_of.is_empty()
    {
        return None;
    }
    if filter.excluded_static_abilities == vec![crate::static_abilities::StaticAbilityId::Flying] {
        return Some("each creature without flying".to_string());
    }
    if filter.excluded_static_abilities.is_empty() {
        return Some("each creature".to_string());
    }
    None
}

pub(super) fn filter_references_tag(filter: &ObjectFilter, tag: &TagKey) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    | crate::filter::TaggedOpbjectRelation::SameStableId
            )
    })
}

pub(super) fn describe_may_choose_pay_for_each_then_untap_tagged(
    effects: &[&Effect],
) -> Option<String> {
    let [may_effect, if_effect] = effects else {
        return None;
    };
    let with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    let decider = may.decider.as_ref()?;
    let conditional = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if conditional.condition != with_id.id
        || conditional.predicate != crate::effect::EffectPredicate::Happened
        || !conditional.else_.is_empty()
    {
        return None;
    }

    let [choose_effect, for_each_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [pay_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let pay = pay_effect.downcast_ref::<crate::effects::PayManaEffect>()?;
    let [untap_effect] = conditional.then.as_slice() else {
        return None;
    };
    let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;

    if choose.is_search
        || choose.top_only
        || choose.chooser != *decider
        || for_each.tag != choose.tag
    {
        return None;
    }
    let ChooseSpec::Player(pay_player) = &pay.player else {
        return None;
    };
    if pay_player != decider && *pay_player != PlayerFilter::IteratedPlayer {
        return None;
    }
    if !choose_spec_is_tagged_object(&untap.target, &choose.tag) {
        return None;
    }

    let mut selected_filter = choose.filter.clone();
    if selected_filter.controller != Some(decider.clone()) {
        return None;
    }
    selected_filter.controller = None;
    let mut selected = selected_filter.description();
    if let Some(rest) = selected.strip_prefix("a ") {
        selected = rest.to_string();
    }
    if let Some(rest) = selected.strip_prefix("an ") {
        selected = rest.to_string();
    }
    if let Some(rest) = selected.strip_suffix(" on the battlefield") {
        selected = rest.to_string();
    }
    selected = selected.replace("nongreen tapped ", "tapped nongreen ");

    let selection = if choose.count.min == 0 && choose.count.max.is_none() {
        format!("any number of {}", pluralize_noun_phrase(&selected))
    } else {
        describe_choose_selection(choose)
    };
    let chooser = describe_player_filter(decider);
    let controlled_by = if *decider == PlayerFilter::You {
        "you control"
    } else {
        "they control"
    };
    let if_player = if chooser == "that player" {
        "the player"
    } else {
        chooser.as_str()
    };
    let chosen_noun = describe_iterated_object_reference_noun(&choose.filter);
    let chosen_plural = pluralize_noun_phrase(chosen_noun);

    Some(format!(
        "{chooser} may choose {selection} {controlled_by} and pay {} for each \
         {chosen_noun} chosen this way. If {if_player} does, untap those {chosen_plural}",
        pay.cost.to_oracle()
    ))
}

pub(super) fn describe_each_creature_and_player_damage_cant_regenerate_structural(
    effects: &[Effect],
) -> Option<String> {
    let [for_each_effect, for_players_effect, cant_effect] = effects else {
        return None;
    };
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachObject>()?;
    if for_each.effects.len() != 1 {
        return None;
    }
    let (damaged_tag, creature_damage) = tagged_damage_view(&for_each.effects[0])?;
    if !matches!(creature_damage.target, ChooseSpec::Iterated) {
        return None;
    }
    let creature_text = damage_each_creature_filter_text(&for_each.filter)?;

    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Any || for_players.effects.len() != 1 {
        return None;
    }
    let player_damage =
        for_players.effects[0].downcast_ref::<crate::effects::DealDamageEffect>()?;
    if player_damage.amount != creature_damage.amount
        || !matches!(
            player_damage.target,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
    {
        return None;
    }

    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::BeRegenerated(cant_filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != Until::EndOfTurn
        || cant_filter.card_types != vec![CardType::Creature]
        || !filter_references_tag(cant_filter, damaged_tag)
    {
        return None;
    }

    Some(format!(
        "Deal {} damage to {creature_text} and each player. Creatures dealt damage this way can't be regenerated this turn",
        describe_value(&creature_damage.amount)
    ))
}

pub(super) fn describe_choose_color_then_chosen_color_mana(effects: &[&Effect]) -> Option<String> {
    let [choose_effect, mana_effect] = effects else {
        return None;
    };
    let choose_color = choose_effect.downcast_ref::<crate::effects::ChooseColorEffect>()?;
    let add_mana = mana_effect.downcast_ref::<crate::effects::AddManaOfChosenColorEffect>()?;
    if choose_color.chooser != PlayerFilter::You
        || add_mana.player != PlayerFilter::You
        || add_mana.fixed_option.is_some()
    {
        return None;
    }
    if let Value::DistinctPowers(filter) = &add_mana.amount {
        return Some(format!(
            "Choose a color. Add one mana of that color for each different power among {}",
            pluralize_noun_phrase(&describe_for_each_count_filter(filter))
        ));
    }
    None
}

pub(super) fn describe_revealed_cards_opponent_may_put_or_draw(
    effects: &[&Effect],
) -> Option<String> {
    let [look_effect, may_effect, fallback_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if !look.reveal || look.player != PlayerFilter::You {
        return None;
    }

    let with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(
        may.decider.as_ref(),
        Some(PlayerFilter::Target(inner)) if matches!(inner.as_ref(), PlayerFilter::Opponent)
    ) {
        return None;
    }
    let [target_effect, hand_effect] = may.effects.as_slice() else {
        return None;
    };
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !matches!(
        target.target.base(),
        ChooseSpec::Player(PlayerFilter::Opponent)
    ) {
        return None;
    }
    let move_to_hand = hand_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_hand.zone != Zone::Hand
        || !matches!(move_to_hand.target.base(), ChooseSpec::Tagged(tag) if tag == &look.tag)
    {
        return None;
    }

    let if_effect = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::DidNotHappen
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let [graveyard_effect, draw_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let move_to_graveyard = unwrap_basic_tag_wrappers(graveyard_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_graveyard.zone != Zone::Graveyard
        || !matches!(move_to_graveyard.target.base(), ChooseSpec::Tagged(tag) if tag == &look.tag)
    {
        return None;
    }
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You {
        return None;
    }

    let is_single_card = matches!(look.count, Value::Fixed(1));
    let count_text = if is_single_card {
        "card".to_string()
    } else {
        describe_card_count(&look.count)
    };
    let object_text = if is_single_card {
        "that card"
    } else {
        "those cards"
    };
    Some(format!(
        "Reveal the top {count_text} of your library. Target opponent may choose to put {object_text} into your hand. If they don't, put {object_text} into your graveyard and draw {}",
        describe_card_count(&draw.count)
    ))
}

pub(super) fn tagged_exile_effect_tag(effect: &Effect) -> Option<&str> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    tagged
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()
        .map(|_| tagged.tag.as_str())
}

pub(super) fn copied_spell_targets_tag(effect: &Effect, tag: &str) -> bool {
    let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() else {
        return false;
    };
    let Some(with_id) = tagged.effect.downcast_ref::<crate::effects::WithIdEffect>() else {
        return false;
    };
    with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()
        .is_some_and(
            |copy| matches!(&copy.target, ChooseSpec::Tagged(copy_tag) if copy_tag.as_str() == tag),
        )
}

pub(super) fn may_cast_copy_targets_tag(effect: &Effect, tag: &str) -> bool {
    let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() else {
        return false;
    };
    let [cast_effect] = may.effects.as_slice() else {
        return false;
    };
    cast_effect
        .downcast_ref::<crate::effects::CastTaggedEffect>()
        .is_some_and(|cast| cast.as_copy && cast.tag.as_str() == tag)
}

pub(super) fn describe_player_gain_keyword(
    player: &PlayerFilter,
    keyword: &str,
    duration: &Until,
) -> String {
    let subject = describe_player_set_filter(player);
    let verb = match player {
        PlayerFilter::You
        | PlayerFilter::Any
        | PlayerFilter::Opponent
        | PlayerFilter::NotYou
        | PlayerFilter::Teammate => "gain",
        _ => "gains",
    };
    let duration_text = if *duration == Until::EndOfTurn {
        "until end of turn".to_string()
    } else {
        describe_until(duration)
    };
    format!("{subject} {verb} {keyword} {duration_text}")
}

pub(super) fn describe_player_protection_from_everything_pair(
    effects: &[&Effect],
) -> Option<String> {
    let [cant_effect, prevent_effect] = effects else {
        return None;
    };
    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let prevent =
        prevent_effect.downcast_ref::<crate::effects::PreventAllDamageToTargetEffect>()?;
    let crate::effect::Restriction::BeTargetedPlayer(player) = &cant.restriction else {
        return None;
    };
    let same_player = match prevent.target.base() {
        ChooseSpec::Player(prevent_player) => prevent_player == player,
        ChooseSpec::SourceController => player == &PlayerFilter::You,
        _ => false,
    };
    if !same_player
        || prevent.duration != cant.duration
        || prevent.damage_filter != crate::prevention::DamageFilter::all()
        || !prevent.follow_up_effects.is_empty()
    {
        return None;
    }

    Some(describe_player_gain_keyword(
        player,
        "protection from everything",
        &cant.duration,
    ))
}

pub(super) fn numeric_roll_branch_label(predicate: &EffectPredicate) -> Option<String> {
    let EffectPredicate::Value(cmp) = predicate else {
        return None;
    };
    match cmp {
        Comparison::Equal(value) => Some(value.to_string()),
        Comparison::BetweenInclusive(min, max) => Some(format!("{min}—{max}")),
        _ => None,
    }
}

pub(super) fn unwrap_if_effect(effect: &Effect) -> Option<&crate::effects::IfEffect> {
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        return Some(if_effect);
    }
    effect
        .downcast_ref::<crate::effects::WithIdEffect>()?
        .effect
        .downcast_ref::<crate::effects::IfEffect>()
}

fn roll_table_mass_exile_tag(effects: &[Effect]) -> Option<TagKey> {
    effects.iter().rev().find_map(|effect| {
        let tag = effect_outer_tag(effect)?;
        let exile = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ExileEffect>()?;
        matches!(exile.spec.base(), ChooseSpec::All(_)).then(|| tag.clone())
    })
}

fn roll_table_chosen_target(effects: &[Effect]) -> Option<(TagKey, &'static str)> {
    effects.iter().rev().find_map(|effect| {
        let tag = effect_outer_tag(effect)?;
        let target_only = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
        let noun = tagged_reference_noun_from_target(&target_only.target)?;
        Some((tag.clone(), noun))
    })
}

fn describe_roll_branch_damage_to_chosen_target(
    effects: &[Effect],
    chosen_tag: &TagKey,
    chosen_noun: &str,
) -> Option<String> {
    let (damage_effect, trailing) = effects.split_first()?;
    let damage = structural_unwrap_render_wrappers(damage_effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.source_is_combat
        || damage.unpreventable
        || !choose_spec_references_exact_tag(&damage.target, chosen_tag)
    {
        return None;
    }

    let damage_text = format!(
        "Deal {} damage to {chosen_noun}",
        describe_value(&damage.amount)
    );
    if trailing.is_empty() {
        return Some(damage_text);
    }
    let trailing_text = describe_effect_list(trailing);
    let trailing_text = trailing_text.trim().trim_end_matches('.');
    (!trailing_text.is_empty()).then(|| format!("{damage_text}. {trailing_text}"))
}

fn roll_branch_is_each_player_draw(effects: &[Effect]) -> bool {
    let [effect] = effects else {
        return false;
    };
    let Some(for_players) = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()
    else {
        return false;
    };
    let [draw_effect] = for_players.effects.as_slice() else {
        return false;
    };
    let Some(draw) = structural_unwrap_render_wrappers(draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()
    else {
        return false;
    };
    for_players.filter == PlayerFilter::Any && draw.player == PlayerFilter::IteratedPlayer
}

fn describe_controller_draw_roll_branch(effects: &[Effect]) -> Option<String> {
    let [effect] = effects else {
        return None;
    };
    let draw = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You {
        return None;
    }
    let rendered = describe_effect(effect);
    let rendered = rendered.trim().trim_end_matches('.');
    let draw_tail = rendered
        .strip_prefix("Draw ")
        .or_else(|| rendered.strip_prefix("draw "))?;
    Some(format!("You draw {draw_tail}"))
}

fn tagged_battlefield_return_view(effect: &Effect) -> Option<(TagKey, TagKey)> {
    let result_tag = effect_outer_tag(effect)?.clone();
    let moved = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::Tagged(target_tag) = moved.target.base() else {
        return None;
    };
    (moved.zone == Zone::Battlefield
        && moved.battlefield_controller == crate::effects::BattlefieldController::Owner
        && !moved.enters_tapped
        && !moved.enters_face_down)
        .then(|| (result_tag, target_tag.clone()))
}

fn tagged_exile_view(effect: &Effect) -> Option<(TagKey, TagKey)> {
    let result_tag = effect_outer_tag(effect)?.clone();
    let inner = structural_unwrap_render_wrappers(effect);
    if let Some(moved) = inner.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        let ChooseSpec::Tagged(target_tag) = moved.target.base() else {
            return None;
        };
        return (moved.zone == Zone::Exile && !moved.enters_face_down)
            .then(|| (result_tag, target_tag.clone()));
    }
    let exile = inner.downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::Tagged(target_tag) = exile.spec.base() else {
        return None;
    };
    (!exile.face_down).then(|| (result_tag, target_tag.clone()))
}

fn untagged_exile_target(effect: &Effect) -> Option<TagKey> {
    if effect_outer_tag(effect).is_some() {
        return None;
    }
    let inner = structural_unwrap_render_wrappers(effect);
    if let Some(moved) = inner.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        let ChooseSpec::Tagged(target_tag) = moved.target.base() else {
            return None;
        };
        return (moved.zone == Zone::Exile && !moved.enters_face_down).then(|| target_tag.clone());
    }
    let exile = inner.downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::Tagged(target_tag) = exile.spec.base() else {
        return None;
    };
    (!exile.face_down).then(|| target_tag.clone())
}

fn delayed_battlefield_return_target(effect: &Effect) -> Option<TagKey> {
    let schedule = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    schedule
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfEndStepTrigger>()?;
    let [return_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let moved = structural_unwrap_render_wrappers(return_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::Tagged(target_tag) = moved.target.base() else {
        return None;
    };
    (moved.zone == Zone::Battlefield
        && moved.battlefield_controller == crate::effects::BattlefieldController::Owner
        && !moved.enters_tapped
        && !moved.enters_face_down)
        .then(|| target_tag.clone())
}

fn describe_mass_exile_roll_branch(effects: &[Effect], exiled_tag: &TagKey) -> Option<String> {
    if let [schedule] = effects
        && delayed_battlefield_return_target(schedule).as_ref() == Some(exiled_tag)
    {
        return Some(
            "Return those cards to the battlefield under their owner's control at the beginning of the next end step"
                .to_string(),
        );
    }

    let [return_effect, exile_effect, schedule] = effects else {
        return None;
    };
    let (returned_tag, return_target) = tagged_battlefield_return_view(return_effect)?;
    let delayed_target = delayed_battlefield_return_target(schedule)?;
    if return_target != *exiled_tag {
        return None;
    }
    let linked_reexile =
        tagged_exile_view(exile_effect).is_some_and(|(reexiled_tag, exile_target)| {
            exile_target == returned_tag && delayed_target == reexiled_tag
        }) || untagged_exile_target(exile_effect).is_some_and(|exile_target| {
            exile_target == returned_tag && delayed_target.as_str() == crate::tag::SOURCE_EXILED_TAG
        });
    if !linked_reexile {
        return None;
    }
    Some(
        "Return those cards to the battlefield under their owner's control, then exile them again. Return those cards to the battlefield under their owner's control at the beginning of the next end step"
            .to_string(),
    )
}

fn roll_prefix_uses_then(prefix: &str) -> bool {
    ["Choose ", "Exile "]
        .iter()
        .any(|head| prefix.starts_with(head))
}

pub(super) fn describe_roll_die_with_numeric_result_table(effects: &[Effect]) -> Option<String> {
    // Triggered abilities carry an internal snapshot tag before their visible
    // effects. It has no oracle-text surface, so let the same die-table
    // compactor handle both triggered and non-triggered result tables.
    let effects = match effects.split_first() {
        Some((first, rest))
            if first
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            rest
        }
        _ => effects,
    };
    if effects.len() < 2 {
        return None;
    }
    let roll_indices = effects
        .iter()
        .enumerate()
        .filter_map(|(idx, effect)| {
            effect
                .downcast_ref::<crate::effects::WithIdEffect>()?
                .effect
                .downcast_ref::<crate::effects::RollDieEffect>()
                .map(|_| idx)
        })
        .collect::<Vec<_>>();
    let [roll_idx] = roll_indices.as_slice() else {
        return None;
    };
    let roll_idx = *roll_idx;
    let roll_with_id = effects[roll_idx].downcast_ref::<crate::effects::WithIdEffect>()?;
    let branches = effects.get(roll_idx + 1..)?;
    if branches.is_empty() {
        return None;
    }

    if roll_idx == 0
        && let [roll_effect, branch_effect] = effects
        && let Some(if_effect) = unwrap_if_effect(branch_effect)
        && if_effect.condition == roll_with_id.id
        && if_effect.else_.is_empty()
        && let Some(condition) = describe_with_id_if_clause(roll_with_id, if_effect)
    {
        let branch = describe_effect_list(&if_effect.then);
        return Some(format!(
            "{}. {}, {}",
            describe_effect(roll_effect).trim_end_matches('.'),
            condition,
            lowercase_first(branch.trim_end_matches('.'))
        ));
    }

    let prefix_effects = &effects[..roll_idx];
    let mass_exiled_tag = roll_table_mass_exile_tag(prefix_effects);
    let chosen_target = roll_table_chosen_target(prefix_effects);
    let table_contrasts_each_player_with_controller = branches.iter().any(|effect| {
        unwrap_if_effect(effect).is_some_and(|branch| roll_branch_is_each_player_draw(&branch.then))
    });
    let roll_text = describe_effect(&effects[roll_idx])
        .trim()
        .trim_end_matches('.')
        .to_string();
    let header = if prefix_effects.is_empty() {
        roll_text
    } else {
        let prefix = describe_effect_list(prefix_effects);
        let prefix = prefix.trim().trim_end_matches('.');
        if prefix.is_empty() {
            return None;
        }
        if roll_prefix_uses_then(prefix) {
            format!("{prefix}, then {}", lowercase_first(&roll_text))
        } else {
            format!("{prefix}. {roll_text}")
        }
    };
    let header = format!("{}.", header.trim_end_matches('.'));

    let mut lines = vec![header];
    for effect in branches {
        let if_effect = unwrap_if_effect(effect)?;
        if if_effect.condition != roll_with_id.id || !if_effect.else_.is_empty() {
            return None;
        }
        let label = numeric_roll_branch_label(&if_effect.predicate)?;
        let branch = mass_exiled_tag
            .as_ref()
            .and_then(|tag| describe_mass_exile_roll_branch(&if_effect.then, tag))
            .or_else(|| {
                chosen_target.as_ref().and_then(|(tag, noun)| {
                    describe_roll_branch_damage_to_chosen_target(&if_effect.then, tag, noun)
                })
            })
            .or_else(|| {
                table_contrasts_each_player_with_controller
                    .then(|| describe_controller_draw_roll_branch(&if_effect.then))
                    .flatten()
            })
            .unwrap_or_else(|| describe_effect_list(&if_effect.then));
        lines.push(format!("{label} | {}.", branch.trim_end_matches('.')));
    }

    Some(lines.join("\n"))
}

pub(super) fn describe_roll_die_then_scry_result(effects: &[Effect]) -> Option<String> {
    let [roll_effect, scry_effect] = effects else {
        return None;
    };
    let roll_with_id = roll_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let roll = roll_with_id
        .effect
        .downcast_ref::<crate::effects::RollDieEffect>()?;
    let scry = scry_effect.downcast_ref::<crate::effects::ScryEffect>()?;
    if roll.player != scry.player
        || !value_prefers_where_x(&scry.count)
        || !matches!(scry.count.unhinted(), Value::EffectValue(id) if *id == roll_with_id.id)
    {
        return None;
    }

    let scry_text = if scry.player == PlayerFilter::You {
        "Scry X, where X is the result".to_string()
    } else {
        let player = describe_player_filter(&scry.player);
        format!(
            "{player} {} X, where X is the result",
            player_verb(&player, "scry", "scries")
        )
    };
    Some(format!(
        "{}. {scry_text}",
        describe_effect(roll_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_each_opponent_exile_top_then_cast_until_eot_any_color(
    effects: &[Effect],
) -> Option<String> {
    let [for_players_effect, grant_effect] = effects else {
        return None;
    };
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Opponent || for_players.effects.len() != 1 {
        return None;
    }
    let exile_top =
        for_players.effects[0].downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if exile_top.player != PlayerFilter::IteratedPlayer
        || exile_top.count != Value::Fixed(1)
        || !exile_top.moved_tags.is_empty()
        || exile_top.accumulated_tags.len() != 1
    {
        return None;
    }

    let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if grant.tag != exile_top.accumulated_tags[0]
        || grant.player != PlayerFilter::You
        || grant.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || grant.allow_land
        || !grant.allow_any_color_for_cast
        || grant.while_on_top_of_library
        || grant.filter.is_some()
    {
        return None;
    }

    let mana_clause = grant.mana_spend_cast_clause("those spells")?;
    Some(format!(
        "Exile the top card of each opponent's library. Until end of turn, you may cast spells from among those exiled cards, and {mana_clause}"
    ))
}

pub(super) fn describe_group_pump_then_conditional_untap(effects: &[Effect]) -> Option<String> {
    let [pump_effect, conditional_effect] = effects else {
        return None;
    };
    let pump_tag = wrapped_effect_tag(pump_effect)?;
    let pump = unwrap_basic_tag_wrappers(pump_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let target_spec = pump.target_spec.as_ref()?;
    if target_spec.is_target() {
        return None;
    }
    let group_noun = match target_spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter)
            if filter.card_types == [CardType::Creature] =>
        {
            "those creatures"
        }
        _ => return None,
    };

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !matches!(
        &conditional.condition,
        Condition::Not(inner) if matches!(inner.as_ref(), Condition::YourTurn)
    ) || !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
    {
        return None;
    }
    let untap = unwrap_basic_tag_wrappers(&conditional.if_true[0])
        .downcast_ref::<crate::effects::UntapEffect>()?;
    if !choose_spec_references_tagged_object(&untap.target, pump_tag) {
        return None;
    }

    Some(format!(
        "{}. If it's not your turn, untap {group_noun}",
        describe_effect(pump_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_destroy_then_color_conditional(
    destroy_effect: &Effect,
    conditional_effect: &Effect,
) -> Option<String> {
    let tagged_destroy = destroy_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let destroy = tagged_destroy
        .effect
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let crate::effect::Condition::TaggedObjectMatches(condition_tag, filter) =
        &conditional.condition
    else {
        return None;
    };
    if condition_tag != &tagged_destroy.tag {
        return None;
    }
    let colors = filter.colors?;
    let mut color_only = filter.clone();
    color_only.colors = None;
    if color_only != crate::target::ObjectFilter::default() {
        return None;
    }
    let color_text = describe_filter_color_alternatives(colors);
    if color_text.is_empty() {
        return None;
    }
    let noun = destroyed_target_reference_noun(&destroy.spec)?;
    let true_branch = lowercase_first(
        describe_effect_list(&conditional.if_true)
            .trim()
            .trim_end_matches('.'),
    );
    if true_branch.is_empty() {
        return None;
    }
    Some(format!(
        "{}. If that {noun} was {color_text}, {true_branch}",
        describe_effect(destroy_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_filter_color_alternatives(colors: crate::color::ColorSet) -> String {
    let mut names = Vec::new();
    if colors.contains(crate::color::Color::White) {
        names.push("white".to_string());
    }
    if colors.contains(crate::color::Color::Blue) {
        names.push("blue".to_string());
    }
    if colors.contains(crate::color::Color::Black) {
        names.push("black".to_string());
    }
    if colors.contains(crate::color::Color::Red) {
        names.push("red".to_string());
    }
    if colors.contains(crate::color::Color::Green) {
        names.push("green".to_string());
    }
    join_with_or(&names)
}

pub(super) fn destroyed_target_reference_noun(spec: &ChooseSpec) -> Option<&'static str> {
    let target = match spec.unhinted() {
        ChooseSpec::Target(inner) => inner.unhinted(),
        ChooseSpec::WithCount(inner, count) if count.is_single() => match inner.unhinted() {
            ChooseSpec::Target(target) => target.unhinted(),
            other => other,
        },
        _ => return None,
    };
    let ChooseSpec::Object(filter) = target else {
        return None;
    };
    if filter
        .card_types
        .contains(&crate::types::CardType::Creature)
    {
        Some("creature")
    } else if filter
        .card_types
        .contains(&crate::types::CardType::Artifact)
    {
        Some("artifact")
    } else if filter
        .card_types
        .contains(&crate::types::CardType::Enchantment)
    {
        Some("enchantment")
    } else if filter.card_types.contains(&crate::types::CardType::Land) {
        Some("land")
    } else if filter
        .card_types
        .contains(&crate::types::CardType::Planeswalker)
    {
        Some("planeswalker")
    } else {
        Some("permanent")
    }
}

pub(super) fn describe_draw_then_for_players_choose_exile(effects: &[Effect]) -> Option<String> {
    let [draw_effect, for_players_effect] = effects else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You || draw.count != Value::Fixed(1) {
        return None;
    }
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let exile_clause = describe_for_players_choose_then_exile(for_players)?;
    Some(format!("You draw a card. {exile_clause}"))
}

pub(super) fn describe_lose_life_then_endure(effects: &[Effect]) -> Option<String> {
    let [lose_effect, endure_effect] = effects else {
        return None;
    };
    let lose = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if lose.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }
    let choose_mode = endure_effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    let endure = describe_endure_mode(choose_mode)?;
    let amount = endure.strip_prefix("it endures ")?;
    Some(format!(
        "You lose {} life and this creature endures {amount}",
        describe_value(&lose.amount)
    ))
}

pub(super) fn describe_tagged_target_then_conditional_action(effects: &[Effect]) -> Option<String> {
    let [target_effect, conditional_effect] = effects else {
        return None;
    };
    let (tag, target_only) = tagged_target_only_effect(target_effect)?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }

    let target_text = describe_choose_spec(&target_only.target);
    let action_text =
        describe_conditional_action_on_tagged_target(&conditional.if_true[0], tag, &target_text)?;
    let condition_text = describe_condition_for_tagged_target(&conditional.condition, tag)?;
    Some(format!("{action_text} if {condition_text}"))
}

pub(super) fn describe_conditional_action_on_tagged_target(
    effect: &Effect,
    tag: &crate::TagKey,
    target_text: &str,
) -> Option<String> {
    let effect = if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        if tagged.tag != *tag {
            return None;
        }
        tagged.effect.as_ref()
    } else {
        effect
    };

    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_zone.zone == Zone::Exile
    {
        return Some(format!("Exile {target_text}"));
    }
    if effect
        .downcast_ref::<crate::effects::CounterEffect>()
        .is_some()
    {
        return Some(format!("Counter {target_text}"));
    }
    None
}

pub(super) fn describe_condition_for_tagged_target(
    condition: &Condition,
    tag: &crate::TagKey,
) -> Option<String> {
    if let Condition::TaggedObjectMatches(condition_tag, filter) = condition
        && condition_tag == tag
        && let Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) =
            filter.mana_value.as_ref()
    {
        return Some(format!(
            "its mana value is less than or equal to {}",
            describe_value(value)
        ));
    }

    if let Condition::PlayerControls { player, filter } = condition
        && let Some(constraint) = filter.tagged_constraints.iter().find(|constraint| {
            constraint.tag == *tag
                && constraint.relation
                    == crate::filter::TaggedOpbjectRelation::SharesColorWithTagged
        })
    {
        let _ = constraint;
        let mut base = filter.clone();
        base.tagged_constraints.retain(|constraint| {
            !(constraint.tag == *tag
                && constraint.relation
                    == crate::filter::TaggedOpbjectRelation::SharesColorWithTagged)
        });
        base.controller = None;
        let object = with_indefinite_article(strip_indefinite_article(&base.description()));
        let controller = match player {
            PlayerFilter::You => "you control".to_string(),
            PlayerFilter::Opponent => "an opponent controls".to_string(),
            _ => format!(
                "{} {}",
                describe_player_filter(player),
                player_verb(&describe_player_filter(player), "control", "controls")
            ),
        };
        return Some(format!("it shares a color with {object} {controller}"));
    }

    Some(lowercase_first(&describe_condition(condition)))
}

pub(super) fn normalize_each_becomes_plural_surface(text: &str) -> String {
    let Some(rest) = text.strip_prefix("Each ") else {
        return text.to_string();
    };
    let Some((subject, predicate)) = rest.split_once(" becomes ") else {
        return text.to_string();
    };

    let (complement, tail) = predicate
        .split_once(" until ")
        .map(|(complement, tail)| (complement, format!(" until {tail}")))
        .unwrap_or((predicate, String::new()));
    let complement = complement
        .strip_prefix("an ")
        .or_else(|| complement.strip_prefix("a "))
        .map(pluralize_noun_phrase)
        .unwrap_or_else(|| complement.to_string());
    format!(
        "{} become {}{}",
        pluralize_noun_phrase(subject),
        complement,
        tail
    )
}

pub(super) fn describe_continuous_choose_attach_sequence(effects: &[Effect]) -> Option<String> {
    let [continuous_effect, choose_effect, attach_effect] = effects else {
        return None;
    };
    unwrap_basic_tag_wrappers(continuous_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.count.is_single() || choose.chooser != PlayerFilter::You {
        return None;
    }
    let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !matches!(&attach.target, ChooseSpec::Tagged(tag) if tag == &choose.tag) {
        return None;
    }

    let continuous_text = normalize_each_becomes_plural_surface(
        describe_effect(continuous_effect).trim_end_matches('.'),
    );
    let mut choice_text = describe_choose_selection(choose);
    if let Some(base) = choice_text.strip_suffix(" on the battlefield") {
        choice_text = base.to_string();
    }
    Some(format!(
        "{continuous_text}. Choose {choice_text}. {}",
        describe_effect(attach_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_countered_spell_same_name_search_sequence(
    effects: &[Effect],
) -> Option<String> {
    if effects.len() != 4 && effects.len() != 5 {
        return None;
    }
    let _counter =
        unwrap_basic_tag_wrappers(&effects[0]).downcast_ref::<crate::effects::CounterEffect>()?;
    let choose = effects[1].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
    }) {
        return None;
    }
    let for_each = effects[2].downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let shuffle = effects[3].downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let search_text = describe_search_choose_for_each(choose, for_each, Some(shuffle), false)?
        .replace("any number of card with", "any number of cards with")
        .replace("all card with", "all cards with");

    let mut text = format!(
        "{}. {}",
        describe_effect(unwrap_basic_tag_wrappers(&effects[0])),
        search_text
    );
    if effects.len() == 5 {
        let draw = effects[4].downcast_ref::<crate::effects::DrawForEachTaggedMatchingEffect>()?;
        if draw.tag != choose.tag {
            return None;
        }
        text.push_str(", then ");
        text.push_str(&lowercase_first(&describe_effect(&effects[4])));
    }
    Some(text)
}

pub(super) fn describe_counter_and_damage_sequence(effects: &[Effect]) -> Option<String> {
    let [counter_effect, damage_effect] = effects else {
        return None;
    };
    unwrap_basic_tag_wrappers(counter_effect).downcast_ref::<crate::effects::CounterEffect>()?;
    unwrap_basic_tag_wrappers(damage_effect).downcast_ref::<crate::effects::DealDamageEffect>()?;

    let counter_text = describe_effect(unwrap_basic_tag_wrappers(counter_effect));
    let damage_text = lowercase_first(
        describe_effect(unwrap_basic_tag_wrappers(damage_effect))
            .trim_end_matches('.')
            .trim(),
    );
    Some(format!("{counter_text} and {damage_text}"))
}

pub(super) fn describe_put_counters_and_add_mana_sequence(effects: &[Effect]) -> Option<String> {
    let [counter_effect, mana_effect] = effects else {
        return None;
    };
    unwrap_basic_tag_wrappers(counter_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    unwrap_basic_tag_wrappers(mana_effect).downcast_ref::<crate::effects::AddManaEffect>()?;

    let counter_text = describe_effect(unwrap_basic_tag_wrappers(counter_effect));
    let mana_text = lowercase_first(
        describe_effect(unwrap_basic_tag_wrappers(mana_effect))
            .trim_end_matches('.')
            .trim(),
    );
    Some(format!("{counter_text} and {mana_text}"))
}

pub(super) fn describe_destroy_all_groups_then_draw_for_destroyed(
    effects: &[Effect],
) -> Option<String> {
    let (draw_effect, destroy_effects) = effects.split_last()?;
    if destroy_effects.len() < 2 {
        return None;
    }
    let draw =
        unwrap_basic_tag_wrappers(draw_effect).downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You || !is_effect_count_reference(&draw.count, None) {
        return None;
    }

    fn destroy_all_card_type(effect: &Effect) -> Option<CardType> {
        let destroy =
            unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::DestroyEffect>()?;
        let ChooseSpec::All(filter) = destroy.spec.base() else {
            return None;
        };
        [
            CardType::Creature,
            CardType::Enchantment,
            CardType::Artifact,
            CardType::Land,
            CardType::Planeswalker,
            CardType::Battle,
        ]
        .into_iter()
        .find(|card_type| filter_has_only_card_type(filter, *card_type))
    }

    let card_types = destroy_effects
        .iter()
        .map(destroy_all_card_type)
        .collect::<Option<Vec<_>>>()?;
    let mut unique = Vec::new();
    for card_type in card_types {
        if unique.contains(&card_type) {
            return None;
        }
        unique.push(card_type);
    }
    let groups = unique
        .iter()
        .map(|card_type| card_type.plural_name().to_string())
        .collect::<Vec<_>>();

    Some(format!(
        "Destroy all {}. Draw a card for each permanent destroyed this way",
        join_with_and(&groups)
    ))
}

pub(super) fn player_is_controller_reference(player: &PlayerFilter) -> bool {
    matches!(player, PlayerFilter::ControllerOf(_))
}

pub(super) fn player_filters_share_controller_reference(
    left: &PlayerFilter,
    right: &PlayerFilter,
) -> bool {
    left == right || (player_is_controller_reference(left) && player_is_controller_reference(right))
}

pub(super) fn filter_has_only_card_type(filter: &ObjectFilter, card_type: CardType) -> bool {
    filter.card_types.len() == 1
        && filter.card_types.contains(&card_type)
        && filter.all_card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.excluded_subtypes.is_empty()
}

pub(super) fn filter_has_only_card_types(filter: &ObjectFilter, card_types: &[CardType]) -> bool {
    filter.card_types.len() == card_types.len()
        && card_types
            .iter()
            .all(|card_type| filter.card_types.contains(card_type))
        && filter.all_card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.excluded_subtypes.is_empty()
}

pub(super) fn filter_any_of_has_exact_card_types(
    filter: &ObjectFilter,
    zone: Option<Zone>,
    card_types: &[CardType],
) -> bool {
    filter.any_of.len() == card_types.len()
        && card_types.iter().all(|card_type| {
            filter.any_of.iter().any(|branch| {
                (branch.zone == zone
                    || (branch.zone.is_none() && filter.zone == zone)
                    || (zone == Some(Zone::Stack)
                        && branch.stack_kind == Some(StackObjectKind::Spell)))
                    && filter_has_only_card_type(branch, *card_type)
                    && branch.any_of.is_empty()
            })
        })
}

pub(super) fn choose_spec_is_target_instant_or_sorcery_spell(spec: &ChooseSpec) -> bool {
    if !spec.is_target() {
        return false;
    }
    let ChooseSpec::Object(filter) = spec.base() else {
        return false;
    };
    let instant_or_sorcery = [CardType::Instant, CardType::Sorcery];
    let direct_instant_or_sorcery =
        filter.zone == Some(Zone::Stack) && filter_has_only_card_types(filter, &instant_or_sorcery);
    direct_instant_or_sorcery
        || filter_any_of_has_exact_card_types(filter, Some(Zone::Stack), &instant_or_sorcery)
}

pub(super) fn consult_filter_is_instant_or_sorcery_card(filter: &ObjectFilter) -> bool {
    let instant_or_sorcery = [CardType::Instant, CardType::Sorcery];
    filter.zone.is_none() && filter_has_only_card_types(filter, &instant_or_sorcery)
        || filter_any_of_has_exact_card_types(filter, None, &instant_or_sorcery)
}

pub(super) fn describe_countered_spell_controller_consult_cast_shuffle(
    effects: &[Effect],
) -> Option<String> {
    let [counter_effect, consult_effect, may_effect, shuffle_effect] = effects else {
        return None;
    };

    let counter = unwrap_basic_tag_wrappers(counter_effect)
        .downcast_ref::<crate::effects::CounterEffect>()?;
    if !choose_spec_is_target_instant_or_sorcery_spell(&counter.target) {
        return None;
    }

    let consult = unwrap_basic_tag_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !player_is_controller_reference(&consult.player)
        || !matches!(
            &consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
        || !consult_filter_is_instant_or_sorcery_card(&consult.filter)
    {
        return None;
    }

    let may = unwrap_basic_tag_wrappers(may_effect).downcast_ref::<crate::effects::MayEffect>()?;
    if !may
        .decider
        .as_ref()
        .is_some_and(|player| player_filters_share_controller_reference(player, &consult.player))
    {
        return None;
    }
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = unwrap_basic_tag_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag != consult.match_tag
        || !player_filters_share_controller_reference(&cast.player, &consult.player)
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
        || cast.cost_reduction.is_some()
    {
        return None;
    }

    let shuffle = unwrap_basic_tag_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let shuffle_uses_controller =
        player_filters_share_controller_reference(&shuffle.player, &consult.player);
    let shuffle_uses_target_player = matches!(
        &shuffle.player,
        PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Any)
    );
    if !shuffle_uses_controller && !shuffle_uses_target_player {
        return None;
    }

    Some("Counter target instant or sorcery spell. Its controller reveals cards from the top of their library until they reveal an instant or sorcery card. That player may cast that card without paying its mana cost. Then the player shuffles".to_string())
}

pub(super) fn describe_choose_two_tap_then_unattach_equipment_sequence(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, tap_effect, unattach_effect] = effects else {
        return None;
    };
    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    let target_count = target_only.target.count();
    if target_count.min != 2
        || target_count.max != Some(2)
        || target_count.dynamic_x
        || target_count.up_to_x
        || target_count.random
        || !target_only.target.is_target()
    {
        return None;
    }
    let ChooseSpec::Object(target_filter) = target_only.target.base() else {
        return None;
    };
    if !target_filter.card_types.contains(&CardType::Creature) {
        return None;
    }

    let tap = unwrap_basic_tag_wrappers(tap_effect).downcast_ref::<crate::effects::TapEffect>()?;
    if !choose_spec_is_tagged_object(&tap.target, target_tag) {
        return None;
    }

    let unattach = unattach_effect.downcast_ref::<crate::effects::UnattachObjectsEffect>()?;
    if describe_unattach_all_equipment_from_tagged(&unattach.objects).is_none() {
        return None;
    }

    Some(
        "Choose two target creatures. Tap those creatures, then unattach all Equipment from them"
            .to_string(),
    )
}

pub(super) fn tagged_iterated_damage_tag_from_for_each(
    for_each: &crate::effects::ForEachObject,
) -> Option<&crate::TagKey> {
    if !for_each.filter.card_types.contains(&CardType::Creature) {
        return None;
    }
    let [damage_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let (tag, damage) = tagged_damage_view(damage_effect)?;
    if !matches!(damage.target.base(), ChooseSpec::Iterated) {
        return None;
    }
    Some(tag)
}

pub(super) fn describe_damage_each_then_tap_damaged_sequence(effects: &[Effect]) -> Option<String> {
    let (damage_effect, tap_effect) = match effects {
        [damage_effect, tap_effect] => (damage_effect, tap_effect),
        [tag_triggering, damage_effect, tap_effect]
            if tag_triggering
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            (damage_effect, tap_effect)
        }
        _ => return None,
    };

    let for_each =
        unwrap_basic_tag_wrappers(damage_effect).downcast_ref::<crate::effects::ForEachObject>()?;
    let damaged_tag = tagged_iterated_damage_tag_from_for_each(for_each)?;
    let tap = unwrap_basic_tag_wrappers(tap_effect).downcast_ref::<crate::effects::TapEffect>()?;
    if !choose_spec_is_tagged_object(&tap.target, damaged_tag) {
        return None;
    }

    Some(format!(
        "{}. Tap those creatures",
        describe_effect(damage_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_exile_source_and_attacking_nonflying_creature(
    effects: &[Effect],
) -> Option<String> {
    let [source_exile_effect, target_exile_effect] = effects else {
        return None;
    };
    let source_exile = source_exile_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if source_exile.zone != Zone::Exile || !matches!(source_exile.target, ChooseSpec::Source) {
        return None;
    }

    let target_exile = unwrap_basic_tag_wrappers(target_exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if target_exile.zone != Zone::Exile {
        return None;
    }

    let mut expected = ObjectFilter::creature()
        .attacking_player_or_planeswalker_controlled_by(PlayerFilter::You)
        .without_static_ability(crate::static_abilities::StaticAbilityId::Flying);
    expected.attacking = true;
    if !matches!(&target_exile.target, ChooseSpec::Target(inner) if matches!(inner.as_ref(), ChooseSpec::Object(filter) if filter == &expected))
    {
        return None;
    }

    Some("Exile this creature and target creature without flying that's attacking you".to_string())
}

pub(super) fn move_to_zone_is_plain_exile(move_to_zone: &crate::effects::MoveToZoneEffect) -> bool {
    move_to_zone.zone == Zone::Exile
        && move_to_zone.battlefield_controller == crate::effects::BattlefieldController::Preserve
        && !move_to_zone.enters_tapped
        && !move_to_zone.enters_attacking
        && !move_to_zone.enters_face_down
        && !move_to_zone.transfer_exiled_with_source_links
}

pub(super) fn describe_exile_source_and_target(effects: &[Effect]) -> Option<String> {
    let [source_exile_effect, target_exile_effect] = effects else {
        return None;
    };
    let source_exile = unwrap_basic_tag_wrappers(source_exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let target_exile = unwrap_basic_tag_wrappers(target_exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_zone_is_plain_exile(source_exile)
        || !move_to_zone_is_plain_exile(target_exile)
        || !matches!(source_exile.target.base(), ChooseSpec::Source)
        || !target_exile.target.is_target()
    {
        return None;
    }

    Some(format!(
        "Exile {} and {}",
        describe_choose_spec(&source_exile.target),
        describe_choose_spec(&target_exile.target)
    ))
}

pub(super) fn oath_of_ghouls_creature_graveyard_filter(
    filter: &ObjectFilter,
    owner: Option<&PlayerFilter>,
) -> bool {
    filter.zone == Some(Zone::Graveyard)
        && filter.card_types.as_slice() == [CardType::Creature]
        && filter.subtypes.is_empty()
        && filter.owner.as_ref() == owner
}

pub(super) fn describe_oath_of_ghouls_sequence(effects: &[Effect]) -> Option<String> {
    let [conditional_effect] = effects else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let crate::effect::Condition::AnOpponentHasFewerThanPlayer { player, filter } =
        &conditional.condition
    else {
        return None;
    };
    if player != &PlayerFilter::IteratedPlayer
        || !oath_of_ghouls_creature_graveyard_filter(filter, None)
    {
        return None;
    }
    let [may_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.as_ref() != Some(&PlayerFilter::IteratedPlayer) {
        return None;
    }
    let [return_effect] = may.effects.as_slice() else {
        return None;
    };
    let return_from_gy =
        return_effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
    if return_from_gy.random || !exact_count(&return_from_gy.target.count(), 1) {
        return None;
    }
    let ChooseSpec::Object(return_filter) = return_from_gy.target.base() else {
        return None;
    };
    if !oath_of_ghouls_creature_graveyard_filter(return_filter, Some(&PlayerFilter::IteratedPlayer))
    {
        return None;
    }

    Some(
        "That player chooses target player whose graveyard has fewer creature cards in it than their graveyard does and is their opponent. The first player may return a creature card from their graveyard to their hand"
            .to_string(),
    )
}

pub(super) fn describe_gain_life_shuffle_source_and_graveyard(
    effects: &[Effect],
) -> Option<String> {
    let [
        gain_effect,
        move_effect,
        shuffle_effect,
        graveyard_shuffle_effect,
    ] = effects
    else {
        return None;
    };

    let gain = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    if gain.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }

    let move_tag = wrapped_effect_tag(move_effect)?;
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !matches!(move_to_zone.target.base(), ChooseSpec::Source)
        || move_to_zone.zone != Zone::Library
        || move_to_zone.to_top
    {
        return None;
    }

    let shuffle_library = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle_library.target_spec.is_some()
        || !matches!(
            &shuffle_library.player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag)) if tag == move_tag
        )
    {
        return None;
    }

    let graveyard_shuffle = graveyard_shuffle_effect
        .downcast_ref::<crate::effects::ShuffleGraveyardIntoLibraryEffect>()?;
    if graveyard_shuffle.player != PlayerFilter::You {
        return None;
    }

    Some(format!(
        "You gain {} life. Shuffle this permanent and your graveyard into their owner's library",
        describe_value(&gain.amount)
    ))
}

pub(super) fn describe_untap_triggering_then_remove_from_combat(
    effects: &[Effect],
) -> Option<String> {
    let triggering_tag = TagKey::from("triggering");
    let (untap_effect, remove_effect) = match effects {
        [tag_triggering, untap_effect, remove_effect]
            if tag_triggering
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            (untap_effect, remove_effect)
        }
        [untap_effect, remove_effect] => (untap_effect, remove_effect),
        _ => return None,
    };

    let untap = if let Some((_, untap)) = tagged_untap_effect_view(untap_effect) {
        untap
    } else {
        untap_effect.downcast_ref::<crate::effects::UntapEffect>()?
    };
    if !choose_spec_references_exact_tag(&untap.target, &triggering_tag) {
        return None;
    }

    let remove = remove_effect.downcast_ref::<crate::effects::RemoveFromCombatEffect>()?;
    if !choose_spec_references_exact_tag(&remove.spec, &triggering_tag) {
        return None;
    }

    Some("untap it and remove it from combat".to_string())
}

pub(super) fn describe_remove_counter_then_no_counters_conditional(
    effects: &[Effect],
) -> Option<String> {
    let [remove_effect, conditional_effect] = effects else {
        return None;
    };
    let remove = unwrap_basic_tag_wrappers(remove_effect)
        .downcast_ref::<crate::effects::RemoveCountersEffect>()?;
    if !matches!(remove.target.base(), ChooseSpec::Source) {
        return None;
    }
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::SourceHasNoCounter(counter_type) = &conditional.condition else {
        return None;
    };
    if describe_no_more_counters_move_then_each_player_return(conditional).is_some() {
        return None;
    }
    if remove.counter_type != *counter_type
        || !conditional.if_false.is_empty()
        || conditional.if_true.is_empty()
    {
        return None;
    }

    let remove_text = describe_effect(remove_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let mut branch = describe_effect_clause_list(&conditional.if_true)
        .unwrap_or_else(|| describe_effect_list(&conditional.if_true));
    branch = lowercase_first(branch.trim().trim_end_matches('.'));
    branch = branch
        .replace("transform this creature", "transform it")
        .replace("transform this artifact", "transform it")
        .replace("sacrifice this creature", "sacrifice it")
        .replace("sacrifice this artifact", "sacrifice it");

    Some(format!(
        "{remove_text}. Then if it has no {} counters on it, {branch}",
        counter_type.description()
    ))
}

pub(super) fn object_filter_has_tagged_constraint(filter: &ObjectFilter, tag: &TagKey) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    })
}

pub(super) fn choose_spec_has_tagged_constraint(spec: &ChooseSpec, tag: &TagKey) -> bool {
    match spec {
        ChooseSpec::Tagged(candidate) => candidate == tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            object_filter_has_tagged_constraint(filter, tag)
        }
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => choose_spec_has_tagged_constraint(inner, tag),
        ChooseSpec::SurfaceHinted { spec, .. } => choose_spec_has_tagged_constraint(spec, tag),
        _ => false,
    }
}

pub(super) fn aura_attachment_self_subject(filter: &ObjectFilter) -> &'static str {
    if filter.card_types.contains(&CardType::Land)
        || filter.subtypes.iter().any(|subtype| {
            matches!(
                subtype,
                Subtype::Plains
                    | Subtype::Island
                    | Subtype::Swamp
                    | Subtype::Mountain
                    | Subtype::Forest
                    | Subtype::Desert
                    | Subtype::Urzas
                    | Subtype::Cave
                    | Subtype::Gate
                    | Subtype::Locus
                    | Subtype::Town
            )
        })
    {
        "this land"
    } else if filter.card_types.contains(&CardType::Creature) {
        "this creature"
    } else if filter.card_types.contains(&CardType::Artifact) {
        "this artifact"
    } else if filter.card_types.contains(&CardType::Enchantment) {
        "this enchantment"
    } else {
        "this permanent"
    }
}

pub(super) fn move_trailing_tapped_token_surface(text: &str) -> String {
    for prefix in ["Create a ", "Create an "] {
        if let Some(rest) = text.strip_prefix(prefix)
            && let Some(rest) = rest.strip_suffix(", tapped")
        {
            return format!("{prefix}tapped {rest}");
        }
    }
    text.to_string()
}

pub(super) fn describe_return_as_aura_with_granted_abilities(effects: &[Effect]) -> Option<String> {
    let mut idx = 0usize;
    if effects
        .first()
        .and_then(|effect| effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>())
        .is_some()
    {
        idx += 1;
    }

    let choose = effects
        .get(idx)?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.count.min != 1 || choose.count.max != Some(1) || choose.chooser != PlayerFilter::You {
        return None;
    }
    idx += 1;

    let return_effect = effects
        .get(idx)?
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()?;
    let as_aura = return_effect.as_aura.as_ref()?;
    if return_effect.tapped || as_aura.remove_all_abilities {
        return None;
    }
    if !object_filter_has_tagged_constraint(&as_aura.attachment_filter, &choose.tag) {
        return None;
    }
    idx += 1;

    let self_subject = aura_attachment_self_subject(&choose.filter);
    let mut granted_abilities = Vec::new();
    for effect in &effects[idx..] {
        let apply = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
        if apply.until != Until::Forever
            || apply.condition.is_some()
            || !apply.additional_modifications.is_empty()
            || !apply.runtime_modifications.is_empty()
            || !apply
                .target_spec
                .as_ref()
                .is_some_and(|spec| choose_spec_has_tagged_constraint(spec, &choose.tag))
        {
            return None;
        }
        let Some(crate::continuous::Modification::AddAbilityGeneric(ability)) = &apply.modification
        else {
            return None;
        };
        let ability_text = move_trailing_tapped_token_surface(
            &describe_inline_ability_with_self_subject(ability, self_subject),
        );
        granted_abilities.push(ability_text.trim_end_matches('.').to_string());
    }
    if granted_abilities.is_empty() {
        return None;
    }

    let enchant_target = strip_leading_article(&choose.filter.description()).to_string();
    let ability_subject = enchant_target
        .strip_suffix(" you control")
        .unwrap_or(enchant_target.as_str());
    let quoted = granted_abilities
        .iter()
        .enumerate()
        .map(|(idx, ability)| {
            if idx + 1 == granted_abilities.len() && !ability.ends_with('.') {
                format!("'{ability}.'")
            } else {
                format!("'{ability}'")
            }
        })
        .collect::<Vec<_>>();

    Some(format!(
        "Return it to the battlefield. It's an Aura enchantment with enchant {enchant_target} and \"{} has {}\"",
        capitalize_first(&format!("enchanted {ability_subject}")),
        join_with_and(&quoted)
    ))
}

pub(super) fn describe_creature_planeswalker_source_counter_exile_item(
    filter: &ObjectFilter,
) -> Option<String> {
    let Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) = filter.mana_value.as_ref()
    else {
        return None;
    };
    let Value::CountersOnSource(counter_type) = value.unhinted() else {
        return None;
    };
    if filter.card_types.len() != 2
        || !filter.card_types.contains(&CardType::Creature)
        || !filter.card_types.contains(&CardType::Planeswalker)
    {
        return None;
    }

    let mut remaining = filter.clone();
    let zone = remaining.zone.take();
    remaining.card_types.clear();
    remaining.mana_value = None;
    if remaining != ObjectFilter::default() {
        return None;
    }

    let mana_value_clause = format!(
        "with mana value less than or equal to the number of {} counters on it",
        counter_type.description()
    );
    match zone {
        None | Some(Zone::Battlefield) => Some(format!(
            "all creatures and planeswalkers {mana_value_clause}"
        )),
        Some(Zone::Graveyard) => Some(format!(
            "all creature and planeswalker cards in graveyards {mana_value_clause}"
        )),
        _ => None,
    }
}

pub(super) fn describe_mixed_exile_all_list_item(
    exile: &crate::effects::ExileEffect,
) -> Option<String> {
    if exile.face_down {
        return None;
    }
    let ChooseSpec::All(filter) = exile.spec.base() else {
        return None;
    };
    describe_creature_planeswalker_source_counter_exile_item(filter)
        .or_else(|| Some(describe_choose_spec(&exile.spec)))
}

pub(super) fn join_mixed_exile_list_items(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let mut out = items[..items.len() - 1].join(", ");
            out.push_str(", and ");
            out.push_str(&items[items.len() - 1]);
            out
        }
    }
}

pub(super) fn describe_mixed_move_to_exile_then_exile_all_list(
    effects: &[Effect],
) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }
    let first = effects[0].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if first.zone != Zone::Exile || matches!(first.target.base(), ChooseSpec::All(_)) {
        return None;
    }

    let mut items = Vec::with_capacity(effects.len());
    items.push(describe_choose_spec(&first.target));
    for effect in &effects[1..] {
        let exile = effect.downcast_ref::<crate::effects::ExileEffect>()?;
        items.push(describe_mixed_exile_all_list_item(exile)?);
    }

    Some(format!("Exile {}", join_mixed_exile_list_items(&items)))
}

pub(super) fn filter_controls_only_tagged_object(
    filter: &ObjectFilter,
    player: &PlayerFilter,
    tag: &TagKey,
) -> bool {
    let mut stripped = filter.clone();
    if stripped
        .controller
        .as_ref()
        .is_some_and(|controller| controller == player)
    {
        stripped.controller = None;
    }
    let Some(tagged_idx) = stripped.tagged_constraints.iter().position(|constraint| {
        constraint.tag == *tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) else {
        return false;
    };
    if stripped.tagged_constraints.len() != 1 {
        return false;
    }
    stripped.tagged_constraints.remove(tagged_idx);
    stripped == ObjectFilter::default() || stripped == ObjectFilter::creature()
}

pub(super) fn condition_controls_tagged_object(
    condition: &Condition,
    player: &PlayerFilter,
    tag: &TagKey,
) -> bool {
    let Condition::PlayerControls {
        player: condition_player,
        filter,
    } = condition
    else {
        return false;
    };
    condition_player == player && filter_controls_only_tagged_object(filter, player, tag)
}

pub(super) fn condition_does_not_control_tagged_object(
    condition: &Condition,
    player: &PlayerFilter,
    tag: &TagKey,
) -> bool {
    let Condition::Not(inner) = condition else {
        return false;
    };
    condition_controls_tagged_object(inner, player, tag)
}

pub(super) fn describe_triggering_control_draw_else_lose(effects: &[Effect]) -> Option<String> {
    let [tag_effect, draw_conditional_effect, lose_conditional_effect] = effects else {
        return None;
    };
    let tag_triggering = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let tag = &tag_triggering.tag;
    if tag.as_str() != "triggering" {
        return None;
    }

    let draw_conditional =
        draw_conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !draw_conditional.if_false.is_empty()
        || !condition_controls_tagged_object(&draw_conditional.condition, &PlayerFilter::You, tag)
    {
        return None;
    }
    let [draw_effect] = draw_conditional.if_true.as_slice() else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You || draw.count != Value::Fixed(1) {
        return None;
    }

    let lose_conditional =
        lose_conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !lose_conditional.if_false.is_empty()
        || !condition_does_not_control_tagged_object(
            &lose_conditional.condition,
            &PlayerFilter::You,
            tag,
        )
    {
        return None;
    }
    let [lose_effect] = lose_conditional.if_true.as_slice() else {
        return None;
    };
    let lose = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if lose.amount != Value::Fixed(1) {
        return None;
    }
    match &lose.player {
        ChooseSpec::Player(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(
            lose_tag,
        ))) if lose_tag == tag => Some(
            "Draw a card if you control that creature. If you don't control it, its controller loses 1 life"
                .to_string(),
        ),
        _ => None,
    }
}

/// Collection programs whose typed producer, selections, and movement already
/// have an oracle-shaped compactor. These must win before the generic clause
/// renderer considers each effect independently: the marker-safe
/// `ForEachTaggedEffect` fallback is intentionally renderable, but it loses the
/// "from among them" relationship when separated from its producer.
fn describe_typed_collection_selection_program(effects: &[Effect]) -> Option<String> {
    let refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = render_exile_top_then_put_from_among_onto_battlefield(&refs) {
        return Some(compact);
    }

    match effects {
        [milled_effect, choose_effect, move_effect] => {
            let (source_tag, mill) = mill_with_collection_tag(milled_effect)?;
            let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let (_, move_chosen) = for_each_tagged_for_compaction(move_effect)?;
            describe_mill_then_put_milled_cards(source_tag.as_str(), mill, &[choose], move_chosen)
        }
        [
            milled_effect,
            first_choice_effect,
            second_choice_effect,
            move_effect,
        ] => {
            let (source_tag, mill) = mill_with_collection_tag(milled_effect)?;
            let first_choice =
                first_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let second_choice =
                second_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let (_, move_chosen) = for_each_tagged_for_compaction(move_effect)?;
            describe_mill_then_put_milled_cards(
                source_tag.as_str(),
                mill,
                &[first_choice, second_choice],
                move_chosen,
            )
        }
        _ => None,
    }
}

fn describe_typed_collection_selection_prefix(effects: &[Effect]) -> Option<(String, usize)> {
    // Collection procedures are often followed by an independent rider (for
    // example, "You gain 2 life").  The typed producer/selection/move prefix
    // must still render as one procedure instead of leaking Choose/ForEach
    // implementation details merely because a later effect shares the same
    // resolution segment.
    for consumed in [4usize, 3usize] {
        if let Some(prefix) = effects.get(..consumed)
            && let Some(compact) = describe_typed_collection_selection_program(prefix)
        {
            return Some((compact, consumed));
        }
    }
    None
}

fn describe_coordinated_returns_then_discard_and_source_exile(
    effects: &[Effect],
) -> Option<String> {
    let [return_sequence_effect, discard_effect, source_exile_effect] = effects else {
        return None;
    };
    let return_sequence = structural_unwrap_render_wrappers(return_sequence_effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if matches!(
        return_sequence.surface,
        ironsmith_core::SequenceSurface::Sequential
    ) {
        return None;
    }
    let return_effects = return_sequence
        .effects
        .iter()
        .filter(|effect| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_none()
        })
        .collect::<Vec<_>>();
    let (return_text, consumed) = describe_leading_coordinated_graveyard_returns(&return_effects)?;
    if consumed != return_effects.len() {
        return None;
    }

    let discard = structural_unwrap_render_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.player != PlayerFilter::You
        || discard.count != Value::Fixed(1)
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
    {
        return None;
    }

    let source_exile = structural_unwrap_render_wrappers(source_exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_zone_is_plain_exile(source_exile)
        || !matches!(source_exile.target.base(), ChooseSpec::Source)
    {
        return None;
    }
    let rendered_exile = describe_effect(source_exile_effect);
    let rendered_exile = rendered_exile.trim().trim_end_matches('.');
    let rendered_exile = rendered_exile
        .strip_prefix("You ")
        .or_else(|| rendered_exile.strip_prefix("you "))
        .unwrap_or(rendered_exile);
    if !rendered_exile.to_ascii_lowercase().starts_with("exile ") {
        return None;
    }

    Some(format!(
        "{return_text}, then discard a card. {}",
        capitalize_first(rendered_exile)
    ))
}

/// Structural renderings that must run before `describe_effect_clause_list`.
///
/// Resolution-program rendering normally prefers the compact clause renderer,
/// so putting these patterns only in `describe_effect_list` makes them
/// unreachable for ordinary spell and triggered-ability payloads.
fn describe_embedded_suspend_setup_sequence(effects: &[Effect]) -> Option<String> {
    for start in 0..effects.len() {
        let matched = effects
            .get(start..start + 3)
            .and_then(describe_exile_with_counters_then_gain_suspend)
            .map(|text| (text, 3))
            .or_else(|| {
                effects
                    .get(start..start + 2)
                    .and_then(describe_put_counters_then_gain_suspend)
                    .map(|text| (text, 2))
            });
        let Some((compact, consumed)) = matched else {
            continue;
        };

        let mut parts = Vec::new();
        if start > 0 {
            parts.push(
                describe_effect_clause_list(&effects[..start])
                    .unwrap_or_else(|| describe_effect_list(&effects[..start])),
            );
        }
        parts.push(compact);
        if start + consumed < effects.len() {
            parts.push(
                describe_effect_clause_list(&effects[start + consumed..])
                    .unwrap_or_else(|| describe_effect_list(&effects[start + consumed..])),
            );
        }

        return Some(
            parts
                .into_iter()
                .enumerate()
                .filter_map(|(index, part)| {
                    let part = part.trim().trim_end_matches('.');
                    let part = normalize_imperative_you_clause(part);
                    if part.is_empty() {
                        None
                    } else if index == 0 {
                        Some(part)
                    } else {
                        Some(capitalize_first(&part))
                    }
                })
                .collect::<Vec<_>>()
                .join(". "),
        );
    }
    None
}

pub(crate) fn describe_pre_clause_structural_effect_list(effects: &[Effect]) -> Option<String> {
    let raw_effects = effects.iter().collect::<Vec<_>>();

    if let Some(compact) = describe_embedded_suspend_setup_sequence(effects) {
        return Some(compact);
    }

    if let Some((compact, consumed)) = describe_conditional_looked_hand_partition(&raw_effects) {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some((compact, consumed)) =
        describe_look_exile_face_down_rest_graveyard_then_cast(&raw_effects)
    {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some((compact, consumed)) = describe_three_way_looked_card_partition(&raw_effects) {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some((compact, consumed)) = describe_self_look_reorder_then_may_shuffle(&raw_effects) {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some(compact) = describe_chain_copy_effect_list(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_gain_control_counter_untap_haste_structural(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_must_block_untap_then_others_cant_block_structural(effects) {
        return Some(compact);
    }

    if effects.len() >= 3
        && effects[0]
            .downcast_ref::<crate::effects::TagAttachedToSourceEffect>()
            .is_some()
        && let Some(prefix) = describe_gain_control_then_untap_structural(&effects[1..3])
    {
        if effects.len() == 3 {
            return Some(prefix);
        }
        let suffix = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if effects.len() >= 2
        && let Some(prefix) = describe_put_counters_then_untap_same_target_structural(&effects[..2])
    {
        if effects.len() == 2 {
            return Some(prefix);
        }
        let suffix = describe_effect_clause_list(&effects[2..])
            .unwrap_or_else(|| describe_effect_list(&effects[2..]));
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some(compact) = describe_tagged_multi_copy_then_may_retarget(effects) {
        return Some(compact);
    }

    // A targeted damage effect does not make a following explicit "you"
    // discard/draw sequence refer to the damaged player. Keep the sentence
    // boundary and let the typed discard/draw pair retain its own actor.
    if let [damage_effect, tail @ ..] = effects
        && unwrap_basic_tag_wrappers(damage_effect)
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .is_some()
        && let Some(discard_draw) = describe_discard_then_draw_amount_sequence(tail)
    {
        let damage = describe_effect(damage_effect)
            .trim_end_matches('.')
            .to_string();
        return Some(format!(
            "{damage}. {}",
            capitalize_first(discard_draw.trim_end_matches('.'))
        ));
    }

    if let Some(compact) = describe_tagged_forced_block_effect_list(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_coordinated_returns_then_discard_and_source_exile(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_typed_collection_selection_program(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_gain_control_aura_then_legal_attach(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_linked_source_attachment_prefix(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_battlefield_graveyard_return_pair(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_battlefield_graveyard_exile_pair(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_tap_freeze_bundle(&raw_effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_top_choice_to_hand_rest_graveyard_structural(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_become_aura_manifest_then_attach(&raw_effects) {
        return Some(compact);
    }
    if let [producer, for_each] = effects
        && let Some(compact) = describe_result_producer_then_for_each_tagged(producer, for_each)
    {
        return Some(compact);
    }
    if let [
        look_effect,
        reveal_effect,
        choose_effect,
        move_effect,
        rest_effect,
    ] = effects
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_tagged) =
            reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, move_chosen)) = for_each_tagged_for_compaction(move_effect)
        && let Some((_, rest)) = for_each_tagged_for_compaction(rest_effect)
        && let Some(compact) = describe_look_at_top_then_put_matching_to_zone_rest_hand(
            look_at_top,
            Some(reveal_tagged),
            choose,
            move_chosen,
            rest,
        )
    {
        return Some(compact);
    }
    if let [create_effect, draw_effect] = effects
        && created_token_effect(create_effect).is_some()
        && let Some(draw) = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && matches!(draw.count.unhinted(), Value::DistinctNames(_))
    {
        let create_text = describe_effect(create_effect)
            .trim_end_matches('.')
            .to_string();
        let draw_text = lowercase_first(describe_effect(draw_effect).trim_end_matches('.'));
        if !create_text.is_empty() && !draw_text.is_empty() {
            return Some(format!("{create_text}, then {draw_text}"));
        }
    }
    if let [damage_effect, exile_effect, play_effect] = effects
        && let Some(with_id) = damage_effect.downcast_ref::<crate::effects::WithIdEffect>()
        && unwrap_tag_wrappers(&with_id.effect)
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .is_some()
        && let Some(exile) = exile_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
        && matches!(
            exile.count.unhinted(),
            Value::EffectMetric {
                effect_id,
                metric: crate::effect::EffectMetric::ExcessDamage,
                ..
            } if *effect_id == with_id.id
        )
        && let Some(play) = play_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
        && exile.moved_tags.iter().any(|tag| tag == &play.tag)
    {
        let damage_text = describe_effect(damage_effect)
            .trim_end_matches('.')
            .to_string();
        let exile_text = capitalize_first(describe_effect(exile_effect).trim_end_matches('.'));
        let play_text = capitalize_first(describe_effect(play_effect).trim_end_matches('.'));
        if !damage_text.is_empty() && !exile_text.is_empty() && !play_text.is_empty() {
            return Some(format!("{damage_text}. {exile_text}. {play_text}"));
        }
    }

    None
}

fn describe_leading_coordinated_graveyard_returns(effects: &[&Effect]) -> Option<(String, usize)> {
    let mut targets = Vec::new();
    let mut shared_route: Option<(String, String)> = None;
    for effect in effects {
        let Some((target, from, to)) = coordinated_graveyard_to_hand_view(effect) else {
            break;
        };
        if let Some((expected_from, expected_to)) = &shared_route {
            if expected_from != &from || expected_to != &to {
                break;
            }
        } else {
            shared_route = Some((from, to));
        }
        targets.push(target);
    }
    if targets.len() < 2 {
        return None;
    }
    let consumed = targets.len();
    let (from, to) = shared_route?;
    Some((
        format!(
            "Return {} from {from} to {to}",
            join_coordinated_parts(&targets)?
        ),
        consumed,
    ))
}

fn describe_draw_then_additional_draw(effects: &[Effect]) -> Option<String> {
    let [first_effect, additional_effect] = effects else {
        return None;
    };
    let first = structural_unwrap_render_wrappers(first_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let additional = structural_unwrap_render_wrappers(additional_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if first.player != additional.player
        || first
            .count
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalCards)
        || !additional
            .count
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalCards)
    {
        return None;
    }

    let first_text = describe_effect(first_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let additional_text = lowercase_first(
        describe_effect(additional_effect)
            .trim()
            .trim_end_matches('.'),
    );
    (!first_text.is_empty() && !additional_text.is_empty())
        .then(|| format!("{first_text}, then {additional_text}"))
}

#[cfg(test)]
mod coordinated_return_runtime_tests {
    use super::*;

    #[test]
    fn flat_tagged_runtime_returns_compact_before_the_generic_sentence_loop() {
        let returned = |subtype, tag: &str| {
            let target = ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::default()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You)
                    .with_subtype(subtype),
            ))
            .with_count(ChoiceCount::up_to(1));
            Effect::new(crate::effects::ReturnFromGraveyardToHandEffect::new(
                target, false,
            ))
            .tag(tag)
        };
        let effects = vec![
            returned(Subtype::Pirate, "returned_0"),
            returned(Subtype::Vampire, "returned_1"),
            returned(Subtype::Dinosaur, "returned_2"),
            Effect::exile(ChooseSpec::Source),
        ];

        let rendered = describe_effect_list(&effects);
        assert_eq!(
            rendered
                .matches(" from your graveyard to your hand")
                .count(),
            1
        );
        assert!(!rendered.contains(". Return"), "{rendered}");
        assert!(rendered.ends_with(". Exile this card"), "{rendered}");
    }
}

fn describe_each_player_reveal_set_may_move_else_draw(effects: &[Effect]) -> Option<String> {
    let [reveal_effect, may_effect, fallback_effect] = effects else {
        return None;
    };
    let reveal_for_players = reveal_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let (subject, fallback_subject) = match reveal_for_players.filter {
        PlayerFilter::Any => ("Each player", "each player"),
        PlayerFilter::Opponent => ("Each opponent", "each opponent"),
        _ => return None,
    };
    let [reveal_top_effect] = reveal_for_players.effects.as_slice() else {
        return None;
    };
    let reveal_top = reveal_top_effect.downcast_ref::<crate::effects::RevealTopEffect>()?;
    let revealed_tag = reveal_top.tag.as_ref()?;
    if reveal_top.player != PlayerFilter::IteratedPlayer {
        return None;
    }

    let with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.as_ref() != Some(&PlayerFilter::You) {
        return None;
    }
    let [move_effect] = may.effects.as_slice() else {
        return None;
    };
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.to_top
        || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == revealed_tag)
    {
        return None;
    }
    let owners_zone = match move_to_zone.zone {
        Zone::Graveyard => "graveyards",
        Zone::Hand => "hands",
        Zone::Library => "libraries",
        _ => return None,
    };

    let fallback = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if fallback.condition != with_id.id
        || fallback.predicate != EffectPredicate::DidNotHappen
        || !fallback.else_.is_empty()
    {
        return None;
    }
    let [draw_for_players_effect] = fallback.then.as_slice() else {
        return None;
    };
    let draw_for_players =
        draw_for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if draw_for_players.filter != reveal_for_players.filter {
        return None;
    }
    let [draw_effect] = draw_for_players.effects.as_slice() else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::IteratedPlayer {
        return None;
    }

    Some(format!(
        "{subject} reveals the top card of their library. You may put the revealed cards into their owners' {owners_zone}. If you don't, {fallback_subject} draws {}",
        describe_card_count(&draw.count)
    ))
}

fn describe_consult_characteristic_boost_then_all_revealed_bottom(
    effects: &[Effect],
) -> Option<String> {
    let [consult_effect, boost_effect, remainder_effect] = effects else {
        return None;
    };
    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let boost = structural_unwrap_render_wrappers(boost_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || remainder.tag != consult.all_tag
        || remainder.keep_tagged.is_some()
        || remainder.player != consult.player
        || boost.modification.is_some()
        || !boost.additional_modifications.is_empty()
    {
        return None;
    }
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = boost.runtime_modifications.as_slice()
    else {
        return None;
    };
    if !matches!(toughness.unhinted(), Value::Fixed(0))
        || !matches!(
            power.unhinted(),
            Value::ManaValueOf(spec)
                if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
        )
    {
        return None;
    }

    let consult_text = describe_effect(consult_effect);
    let consult_text = if consult.player == PlayerFilter::You {
        capitalize_first(consult_text.strip_prefix("you ").unwrap_or(&consult_text))
    } else {
        capitalize_first(&consult_text)
    };
    let boost_text = capitalize_first(&describe_effect(boost_effect)).replace(
        "where X is its mana value",
        "where X is that card's mana value",
    );
    let remainder_text = capitalize_first(&describe_effect(remainder_effect));
    Some(format!(
        "{}. {}. {}",
        consult_text.trim_end_matches('.'),
        boost_text.trim_end_matches('.'),
        remainder_text.trim_end_matches('.')
    ))
}

fn describe_consult_reflexive_damage_then_all_revealed_bottom(
    effects: &[Effect],
) -> Option<String> {
    let [consult_effect, reflexive_effect, remainder_effect] = effects else {
        return None;
    };
    let with_id = consult_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let consult = structural_unwrap_render_wrappers(&with_id.effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let reflexive = structural_unwrap_render_wrappers(reflexive_effect)
        .downcast_ref::<crate::effects::ReflexiveTriggerEffect>()?;
    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let [damage_effect] = reflexive.effects.as_slice() else {
        return None;
    };
    let damage = structural_unwrap_render_wrappers(damage_effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;

    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || reflexive.condition != with_id.id
        || reflexive.predicate != EffectPredicate::Happened
        || remainder.tag != consult.all_tag
        || remainder.keep_tagged.is_some()
        || remainder.player != consult.player
        || !matches!(damage.target.base(), ChooseSpec::AnyTarget)
        || !matches!(
            damage.amount.unhinted(),
            Value::ManaValueOf(spec)
                if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
        )
        || !matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
    {
        return None;
    }

    let consult_text = describe_effect(consult_effect);
    let consult_text = capitalize_first(consult_text.strip_prefix("you ").unwrap_or(&consult_text));
    let remainder_text = capitalize_first(&describe_effect(remainder_effect));
    let matched_reference = describe_search_selection_with_cards(&consult.filter.description());
    let triggered = "this deals damage equal to that card's mana value to any target";

    Some(format!(
        "{}. {}. When you reveal {matched_reference} this way, {}",
        consult_text.trim_end_matches('.'),
        remainder_text.trim_end_matches('.'),
        triggered
    ))
}

#[cfg(test)]
mod consult_characteristic_cleanup_tests {
    use super::*;

    #[test]
    fn keeps_match_and_full_revealed_collection_references_distinct() {
        let all_tag = TagKey::from("__sentence_helper_revealed_test");
        let match_tag = TagKey::from("__sentence_helper_consult_match_test");
        let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
            PlayerFilter::You,
            crate::effects::consult_helpers::LibraryConsultMode::Reveal,
            ObjectFilter::default().without_type(CardType::Land),
            crate::effects::consult_helpers::ConsultTopOfLibraryStopRule::FirstMatch,
            all_tag.clone(),
            match_tag.clone(),
        ));
        let boost = Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec_runtime(
                ChooseSpec::Source,
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(match_tag)))
                        .with_surface_hint(ValueSurfaceHint::WhereXIs),
                    toughness: Value::Fixed(0),
                },
                Until::EndOfTurn,
            )
            .require_creature_target(),
        );
        let cleanup = Effect::new(
            crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                all_tag,
                None,
                crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses,
                PlayerFilter::You,
            ),
        );

        let rendered = describe_consult_characteristic_boost_then_all_revealed_bottom(&[
            consult, boost, cleanup,
        ])
        .expect("consult/boost/cleanup bundle");
        assert!(rendered.contains("where X is that card's mana value"));
        assert!(
            rendered.contains("Put the revealed cards on the bottom of your library in any order")
        );
    }

    #[test]
    fn restores_cleanup_before_linked_variable_damage_reflexive() {
        let all_tag = TagKey::from("__sentence_helper_revealed_test");
        let match_tag = TagKey::from("__sentence_helper_consult_match_test");
        let consult = Effect::with_id(
            7,
            Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
                PlayerFilter::You,
                crate::effects::consult_helpers::LibraryConsultMode::Reveal,
                ObjectFilter::default().without_type(CardType::Land),
                crate::effects::consult_helpers::ConsultTopOfLibraryStopRule::FirstMatch,
                all_tag.clone(),
                match_tag.clone(),
            )),
        );
        let reflexive = Effect::reflexive_trigger(
            EffectId(7),
            EffectPredicate::Happened,
            vec![Effect::deal_damage(
                Value::ManaValueOf(Box::new(ChooseSpec::Tagged(match_tag))),
                ChooseSpec::AnyTarget,
            )],
            vec![ChooseSpec::AnyTarget],
        );
        let cleanup = Effect::new(
            crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                all_tag,
                None,
                crate::effects::consult_helpers::LibraryBottomOrder::Random,
                PlayerFilter::You,
            ),
        );

        assert_eq!(
            describe_consult_reflexive_damage_then_all_revealed_bottom(&[
                consult, reflexive, cleanup,
            ])
            .as_deref(),
            Some(
                "Reveal cards from the top of your library until you reveal a nonland card. Put the revealed cards on the bottom of your library in a random order. When you reveal a nonland card this way, this deals damage equal to that card's mana value to any target"
            )
        );
    }
}

#[cfg(test)]
mod reveal_set_optional_move_tests {
    use super::*;

    fn for_each_player(effect: Effect) -> Effect {
        Effect::new(crate::effects::ForPlayersEffect {
            filter: PlayerFilter::Any,
            effects: vec![effect],
            starting_with_controller: false,
            stop_after_first_happened: false,
        })
    }

    #[test]
    fn preserves_plural_revealed_collection_across_players() {
        let tag = TagKey::from("revealed_each_player");
        let reveal = for_each_player(Effect::new(crate::effects::RevealTopEffect::tagged(
            PlayerFilter::IteratedPlayer,
            tag.clone(),
        )));
        let may_move = Effect::with_id(
            7,
            Effect::new(crate::effects::MayEffect::new_for_player(
                vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                    ChooseSpec::Tagged(tag),
                    Zone::Graveyard,
                    false,
                ))],
                PlayerFilter::You,
            )),
        );
        let draw = Effect::if_then(
            EffectId(7),
            EffectPredicate::DidNotHappen,
            vec![for_each_player(Effect::new(
                crate::effects::DrawCardsEffect::new(Value::Fixed(1), PlayerFilter::IteratedPlayer),
            ))],
        );

        assert_eq!(
            describe_each_player_reveal_set_may_move_else_draw(&[reveal, may_move, draw])
                .as_deref(),
            Some(
                "Each player reveals the top card of their library. You may put the revealed cards into their owners' graveyards. If you don't, each player draws a card"
            )
        );
    }
}

pub(crate) fn describe_effect_list(effects: &[Effect]) -> String {
    if let Some(compact) = describe_linked_counter_followup(effects) {
        return compact;
    }
    if let Some(compact) = describe_typed_counter_sentence_split(effects) {
        return compact;
    }
    if let Some(compact) = describe_optional_search_battlefield_partition_effects(effects) {
        return compact;
    }
    if let Some(compact) = describe_discard_redraw_mana_value_ladder(effects) {
        return compact;
    }
    if let Some(compact) = describe_look_hand_optional_exile_persistent_play_tax(effects) {
        return compact;
    }
    if let Some(compact) = describe_hidden_exile_partition_with_persistent_permission(effects) {
        return compact;
    }
    if let Some(compact) = describe_each_opponent_top_card_hidden_exile_permission(effects) {
        return compact;
    }
    if let Some(compact) = describe_exile_all_then_each_player_may_deploy_and_return_exiled(effects)
    {
        return compact;
    }
    if let Some(compact) = describe_exile_two_creatures_then_controller_consults(effects) {
        return compact;
    }
    if let Some(compact) = describe_exile_top_play_then_additional_land(effects) {
        return compact;
    }
    if let Some(compact) = describe_exile_top_choose_one_play_next_turn(effects) {
        return compact;
    }
    if let Some(compact) = describe_each_player_reveal_set_may_move_else_draw(effects) {
        return compact;
    }
    if let Some(compact) = describe_consult_characteristic_boost_then_all_revealed_bottom(effects) {
        return compact;
    }
    if let Some(compact) = describe_consult_reflexive_damage_then_all_revealed_bottom(effects) {
        return compact;
    }
    if let Some(compact) = describe_energy_payment_failure_fallback(effects) {
        return compact;
    }
    if let Some(compact) = describe_draw_then_additional_draw(effects) {
        return compact;
    }
    if let [first, second] = effects
        && let Some(compact) = describe_action_and_get_energy_pair(first, second)
    {
        return compact;
    }
    if let Some(compact) = describe_milled_creatures_returned_then_animated(effects) {
        return compact;
    }
    if let Some(compact) = describe_returned_object_set_to_enchantment(effects) {
        return compact;
    }
    if let Some(compact) = describe_bulk_battlefield_move_then_grant_decayed(effects) {
        return compact;
    }
    let same_name_refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = describe_choose_name_reveal_hand_discard_named_bundle(&same_name_refs) {
        return compact;
    }
    if let Some(compact) = describe_same_name_reference_search_bundle(&same_name_refs) {
        return compact;
    }
    if let Some((compact, consumed)) = describe_linked_target_set_followup_prefix(effects)
        .or_else(|| describe_same_name_exile_then_investigate_prefix(effects))
        .or_else(|| describe_target_same_name_action_fanout_prefix(effects))
    {
        if consumed == effects.len() {
            return compact;
        }
        let suffix = describe_effect_list(&effects[consumed..]);
        return format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        );
    }
    let raw_effects = effects.iter().collect::<Vec<_>>();
    include!("effect_list/raw_patterns.rs");
    let preserve_target_only_players = effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::ForPlayersEffect>()
            .is_some_and(|for_players| for_players.filter == PlayerFilter::target_player())
    });
    let preserve_target_only_references = effects.iter().any(effect_references_target_player);
    let has_non_target_only = effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_none()
    });
    let filtered = effects
        .iter()
        .filter(|effect| {
            if let Some(target_only) = structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                && effects.iter().any(|candidate| {
                    !std::ptr::eq(*effect, candidate)
                        && rendered_action_target(candidate).is_some_and(|action_target| {
                            target_specs_select_same_objects(action_target, &target_only.target)
                        })
                })
            {
                return false;
            }
            if !(has_non_target_only
                && effect
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()
                    .is_some())
            {
                return true;
            }

            if preserve_target_only_players
                && effect
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()
                    .is_some_and(|target_only| {
                        matches!(
                            target_only.target,
                            ChooseSpec::Player(_) | ChooseSpec::WithCount(_, _)
                        )
                    })
            {
                return true;
            }

            if preserve_target_only_references
                && effect
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()
                    .is_some_and(|target_only| choose_spec_is_player_choice(&target_only.target))
            {
                return true;
            }

            if effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some_and(|target_only| {
                    choose_spec_contains_hand_advantage_player_filter(&target_only.target)
                })
            {
                return true;
            }

            false
        })
        .collect::<Vec<_>>();

    if let Some(compact) = describe_coordinated_controller_opponent_bundle(&filtered) {
        return compact;
    }

    include!("effect_list/filtered_patterns.rs");
    include!("effect_list/bundle_patterns.rs");
    let mut parts = Vec::new();
    let mut idx = 0usize;
    while idx < filtered.len() {
        if let Some((compact, consumed)) =
            describe_leading_coordinated_graveyard_returns(&filtered[idx..])
        {
            parts.push(compact);
            idx += consumed;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(compact) =
                describe_choose_then_return_from_graveyard(filtered[idx], filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        include!("effect_list/loop_patterns_early.rs");
        include!("effect_list/loop_patterns_late.rs");
    }
    let text = parts.join(". ");
    if let Some(compact) = normalize_haunting_echoes_text(&text) {
        return compact;
    }
    cleanup_decompiled_text(&text)
}

fn describe_bulk_battlefield_move_then_grant_decayed(effects: &[Effect]) -> Option<String> {
    let [move_effect, grant_effect] = effects else {
        return None;
    };
    let moved_tag = effect_outer_tag(move_effect)?;
    let return_all = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()?;
    let apply = tagged_apply_continuous_effect(grant_effect)?;
    if !apply_continuous_is_forever_tagged(apply, moved_tag)
        || !apply_continuous_grants_decayed(apply)
    {
        return None;
    }

    Some(format!(
        "{}. They gain decayed",
        describe_return_all_to_battlefield_effect(return_all)
    ))
}

pub(super) fn describe_turn_start_hand_condition_effects(effects: &[Effect]) -> Option<String> {
    let [first_effect, second_effect] = effects else {
        return None;
    };
    let first = first_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let second = second_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !first.if_false.is_empty()
        || !second.if_false.is_empty()
        || first.if_true.len() != 1
        || second.if_true.len() != 1
    {
        return None;
    }
    let Condition::PlayerCardsInHandAtTurnStartOrFewer {
        player: first_player,
        count: 0,
    } = &first.condition
    else {
        return None;
    };
    let Condition::PlayerCardsInHandAtTurnStartOrMore {
        player: second_player,
        count: 1,
    } = &second.condition
    else {
        return None;
    };
    if first_player != second_player {
        return None;
    }

    let first_text = lowercase_first(describe_effect(&first.if_true[0]).trim_end_matches('.'));
    let first_condition = lowercase_first(&describe_condition(&first.condition));
    let second_condition = lowercase_first(&describe_condition(&second.condition))
        .replace(" at the beginning of this turn", "");
    let second_text = lowercase_first(describe_effect(&second.if_true[0]).trim_end_matches('.'));
    Some(format!(
        "{first_text} if {first_condition}. If {second_condition}, {second_text}"
    ))
}

pub(super) fn describe_vote_with_received_vote_followups(effects: &[Effect]) -> Option<String> {
    let [first, rest @ ..] = effects else {
        return None;
    };
    first.downcast_ref::<crate::effects::VoteEffect>()?;
    if rest.is_empty() {
        return None;
    }
    let structurally_received_vote_followups = rest.iter().all(|effect| {
        if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
            return for_players.effects.len() == 1
                && for_players.effects[0]
                    .downcast_ref::<crate::effects::RepeatEffectsEffect>()
                    .is_some_and(|repeat| {
                        repeat.count == Value::PlayerVoteCount(PlayerFilter::IteratedPlayer)
                    });
        }
        effect
            .downcast_ref::<crate::effects::RepeatEffectsEffect>()
            .is_some_and(|repeat| matches!(repeat.count, Value::PlayerVoteCount(_)))
    });
    let rendered = effects
        .iter()
        .map(describe_effect)
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>();
    let rendered_received_vote_followups = rendered
        .iter()
        .skip(1)
        .all(|text| text.starts_with("For each vote ") && text.contains(" received,"));
    if !structurally_received_vote_followups && !rendered_received_vote_followups {
        return None;
    }
    Some(rendered.join(". "))
}

pub(super) fn normalize_haunting_echoes_text(text: &str) -> Option<String> {
    let expected = concat!(
        "Exile all nonland cards in target player's graveyard or nonbasic cards in target player's graveyard. ",
        "For each card in exile, you search that player's library for any number with the same name as that object card. ",
        "For each tagged 'searched' object, Exile the tagged object 'searched'. ",
        "Target player shuffles"
    );
    if text == expected {
        return Some(
            "Exile all cards from target player's graveyard other than basic land cards. For each card exiled this way, search that player's library for all cards with the same name as that card and exile them. Then that player shuffles"
                .to_string(),
        );
    }
    None
}

pub(super) fn describe_linked_graveyard_choices_then_may_return_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [first_choose_effect, second_choose_effect, may_effect] = filtered else {
        return None;
    };
    let first_choose = first_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let second_choose =
        second_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [move_effect] = may.effects.as_slice() else {
        return None;
    };
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;

    if first_choose.is_search
        || second_choose.is_search
        || first_choose.replace_tagged_objects
        || second_choose.replace_tagged_objects
        || first_choose.tag != second_choose.tag
        || choose_exact_count(first_choose) != Some(1)
        || choose_exact_count(second_choose) != Some(1)
        || choose_primary_zone(first_choose) != Some(Zone::Graveyard)
        || choose_primary_zone(second_choose) != Some(Zone::Graveyard)
        || !matches!(
            &second_choose.chooser,
            PlayerFilter::AliasedOwnerOf(crate::filter::ObjectRef::Tagged(tag))
                | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Tagged(tag))
                if tag == &first_choose.tag
        )
        || !move_to_battlefield_uses_chosen_tag(move_to_zone, first_choose.tag.as_str())
    {
        return None;
    }

    let describe_choose_clause = |choose: &crate::effects::ChooseObjectsEffect,
                                  capitalize_subject: bool| {
        let chooser = describe_player_filter(&choose.chooser);
        let chosen = describe_choose_selection(choose);
        let location = describe_choose_zone_location(choose, "graveyard");
        if chooser == "you" {
            return format!("Choose {chosen} {location}");
        }
        let subject = if capitalize_subject {
            capitalize_first(&chooser)
        } else {
            chooser.clone()
        };
        let choose_verb = player_verb(&chooser, "choose", "chooses");
        format!("{subject} {choose_verb} {chosen} {location}")
    };

    let first_clause = describe_choose_clause(first_choose, true);
    let second_clause = describe_choose_clause(second_choose, false);
    let tapped_suffix = if move_to_zone.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_to_zone.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => " under their owners' control",
        crate::effects::BattlefieldController::You => " under your control",
    };
    let decider = may
        .decider
        .as_ref()
        .map(describe_player_filter)
        .unwrap_or_else(|| "you".to_string());
    let may_clause = if decider == "you" {
        format!("You may return those cards to the battlefield{tapped_suffix}{controller_suffix}")
    } else {
        let may_verb = player_verb(&decider, "may", "may");
        format!(
            "{} {may_verb} return those cards to the battlefield{tapped_suffix}{controller_suffix}",
            capitalize_first(&decider)
        )
    };

    Some(format!(
        "{first_clause}, then {second_clause}. {may_clause}"
    ))
}

pub(super) fn describe_graveyard_mana_ladder_return_clause_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [first_choose, second_choose, third_choose, return_effect] = filtered else {
        return None;
    };
    let chooses = [
        first_choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
        second_choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
        third_choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
    ];
    for (idx, choose) in chooses.iter().enumerate() {
        if choose.chooser != PlayerFilter::You
            || choose_exact_count(choose) != Some(1)
            || choose_primary_zone(choose) != Some(Zone::Graveyard)
            || choose.filter.owner != Some(PlayerFilter::You)
            || choose.filter.card_types != vec![CardType::Creature]
            || choose.filter.mana_value != Some(crate::filter::Comparison::Equal((idx + 1) as i32))
        {
            return None;
        }
    }
    let return_to_battlefield = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>(
    )?;
    if return_to_battlefield.tapped
        || (!matches!(&return_to_battlefield.target, ChooseSpec::Tagged(tag) if tag == &chooses[0].tag)
            && !matches!(&return_to_battlefield.target, ChooseSpec::Iterated))
    {
        return None;
    }
    Some(
        "Choose a creature card with mana value 1 in your graveyard, then do the same for creature cards with mana value 2 and 3. Return those cards to the battlefield."
            .to_string(),
    )
}

pub(super) fn describe_reveal_power_cards_for_mana_clause_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [choose_effect, reveal_effect, mana_effect] = filtered else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let mana = mana_effect.downcast_ref::<crate::effects::AddScaledManaEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.count.min != 0
        || choose.count.max.is_some()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || reveal.tag != choose.tag
        || mana.player != PlayerFilter::You
        || mana.mana != vec![crate::mana::ManaSymbol::Green]
        || !matches!(&mana.amount, Value::Count(filter) if object_filter_has_tag(filter, &choose.tag))
    {
        return None;
    }
    let mut selection = choose.filter.clone();
    selection.zone = None;
    selection.owner = None;
    selection.controller = None;
    selection.tagged_constraints.clear();
    let mut selection = pluralize_noun_phrase(&selection.description());
    if let Some(rest) = selection.strip_prefix("creatures ") {
        selection = format!("creature cards {rest}");
    } else if selection == "creatures" {
        selection = "creature cards".to_string();
    } else if !selection.contains("card") {
        selection.push_str(" cards");
    }
    Some(format!(
        "Reveal any number of {selection} from your hand. Add {{G}} for each card revealed this way"
    ))
}

pub(super) fn describe_chosen_creatures_blessing_additional_combat_clause(
    effects: &[Effect],
) -> Option<String> {
    if let [
        target_effect,
        tag_matching_effect,
        untap_effect,
        for_each_effect,
        grant_effect,
        additional_effect,
        cant_effect,
    ] = effects
    {
        let targeted = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
        let target_only = targeted
            .effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
        let target_count = target_only.target.count();
        let tag_matching =
            tag_matching_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
        let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;
        let for_each = for_each_effect
            .downcast_ref::<crate::effects::ForEachObject>()
            .or_else(|| {
                for_each_effect
                    .downcast_ref::<crate::effects::TaggedEffect>()
                    .and_then(|tagged| {
                        tagged
                            .effect
                            .downcast_ref::<crate::effects::ForEachObject>()
                    })
            })?;
        let additional =
            additional_effect.downcast_ref::<crate::effects::AdditionalPhasesEffect>()?;
        let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
        if target_count.min != 2
            || target_count.max != Some(2)
            || !describe_choose_spec(&target_only.target).contains("target creatures")
            || !object_filter_has_tag(&tag_matching.filter, &targeted.tag)
            || !matches!(
                &untap.target,
                ChooseSpec::All(filter)
                    if object_filter_has_tag(filter, &tag_matching.tag)
                        || object_filter_has_tag(filter, &targeted.tag)
            )
        {
            return None;
        }
        let blessing =
            describe_for_each_chosen_put_counters_then_gain_keywords(for_each, grant_effect)?;
        let combat =
            describe_additional_combat_then_chosen_attack_or_block_restriction(additional, cant)?;
        return Some(format!(
            "Choose two target creatures. Untap them. {blessing}. {combat}"
        ));
    }

    if let [
        target_effect,
        untap_effect,
        for_each_effect,
        grant_effect,
        additional_effect,
        cant_effect,
    ] = effects
        && let Some(targeted) = target_effect.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(target_only) = targeted
            .effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(untap) = untap_effect.downcast_ref::<crate::effects::UntapEffect>()
        && let Some(for_each) = for_each_effect
            .downcast_ref::<crate::effects::ForEachObject>()
            .or_else(|| {
                for_each_effect
                    .downcast_ref::<crate::effects::TaggedEffect>()
                    .and_then(|tagged| {
                        tagged
                            .effect
                            .downcast_ref::<crate::effects::ForEachObject>()
                    })
            })
        && let Some(additional) =
            additional_effect.downcast_ref::<crate::effects::AdditionalPhasesEffect>()
        && let Some(cant) = cant_effect.downcast_ref::<crate::effects::CantEffect>()
    {
        let target_count = target_only.target.count();
        if target_count.min != 2
            || target_count.max != Some(2)
            || !describe_choose_spec(&target_only.target).contains("target creatures")
            || !matches!(
                &untap.target,
                ChooseSpec::All(filter) if object_filter_has_tag(filter, &targeted.tag)
            )
        {
            // fall through to other six-effect shapes below
        } else if let Some(blessing) =
            describe_for_each_chosen_put_counters_then_gain_keywords(for_each, grant_effect)
            && let Some(combat) =
                describe_additional_combat_then_chosen_attack_or_block_restriction(additional, cant)
        {
            return Some(format!(
                "Choose two target creatures. Untap them. {blessing}. {combat}"
            ));
        }
    }

    let [
        choose_effect,
        untap_effect,
        for_each_effect,
        grant_effect,
        additional_effect,
        cant_effect,
    ] = effects
    else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachObject>()?;
    let additional = additional_effect.downcast_ref::<crate::effects::AdditionalPhasesEffect>()?;
    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.count.min != 2
        || choose.count.max != Some(2)
        || !choose.filter.card_types.contains(&CardType::Creature)
        || !matches!(&untap.target, ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }
    let blessing =
        describe_for_each_chosen_put_counters_then_gain_keywords(for_each, grant_effect)?;
    let combat =
        describe_additional_combat_then_chosen_attack_or_block_restriction(additional, cant)?;
    Some(format!(
        "Choose two target creatures. Untap them. {blessing}. {combat}"
    ))
}

pub(super) fn clause_effects_have_typed_sentence_boundaries(effects: &[&Effect]) -> bool {
    match effects {
        [first, second] => {
            if let Some(add_mana) = unwrap_basic_tag_wrappers(first)
                .downcast_ref::<crate::effects::AddManaOfAnyColorEffect>()
                && add_mana.amount == Value::Fixed(1)
                && add_mana.player == PlayerFilter::You
                && !add_mana.distinct_colors
                && let Some(damage) = unwrap_basic_tag_wrappers(second)
                    .downcast_ref::<crate::effects::DealDamageEffect>()
                && matches!(damage.amount, Value::Fixed(_))
                && damage.target == ChooseSpec::SourceController
                && !damage.source_is_combat
                && !damage.unpreventable
            {
                return true;
            }

            if unwrap_basic_tag_wrappers(first)
                .downcast_ref::<crate::effects::AddManaOfAnyColorEffect>()
                .is_some()
                && let Some(cant) =
                    unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::CantEffect>()
                && matches!(&cant.restriction, crate::effect::Restriction::Untap(filter) if filter.source)
            {
                return true;
            }

            if let Some(tag) = effect_outer_tag(first)
                && unwrap_basic_tag_wrappers(first)
                    .downcast_ref::<crate::effects::ApplyContinuousEffect>()
                    .is_some()
                && let Some(remove) = unwrap_basic_tag_wrappers(second)
                    .downcast_ref::<crate::effects::RemoveUpToAnyCountersEffect>()
                && matches!(&remove.target, ChooseSpec::Tagged(found) if found == tag)
                && matches!(
                    &remove.max_count,
                    Value::CountersOn(spec, None)
                        if matches!(spec.as_ref(), ChooseSpec::Tagged(found) if found == tag)
                )
            {
                return true;
            }

            if let Some(tag) = effect_outer_tag(first)
                && let Some(return_all) = unwrap_basic_tag_wrappers(first)
                    .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>(
                )
                && return_all.face_down
                && let Some(apply) = unwrap_basic_tag_wrappers(second)
                    .downcast_ref::<crate::effects::ApplyContinuousEffect>()
                && apply_continuous_targets_tag(apply, tag)
            {
                return true;
            }

            if let Some(tag) = effect_outer_tag(first)
                && unwrap_basic_tag_wrappers(first)
                    .downcast_ref::<crate::effects::ApplyContinuousEffect>()
                    .is_some()
                && let Some(untap) =
                    unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::UntapEffect>()
                && choose_spec_references_tagged_object(&untap.target, tag)
            {
                return true;
            }

            false
        }
        [reduction, exile_top, grant_play] => {
            reduction
                .downcast_ref::<crate::effects::GrantNextSpellCostReductionEffect>()
                .is_some()
                && exile_top
                    .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
                    .is_some()
                && grant_play
                    .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
                    .is_some()
        }
        [counter_effect, color_effect, subtype_effect, ability_effect] => {
            let Some(tag) = effect_outer_tag(counter_effect) else {
                return false;
            };
            if unwrap_basic_tag_wrappers(counter_effect)
                .downcast_ref::<crate::effects::PutCountersEffect>()
                .is_none()
            {
                return false;
            }
            let Some(color_apply) = tagged_apply_continuous_effect(color_effect) else {
                return false;
            };
            let Some(subtype_apply) = tagged_apply_continuous_effect(subtype_effect) else {
                return false;
            };
            let Some(ability_apply) = tagged_apply_continuous_effect(ability_effect) else {
                return false;
            };
            apply_continuous_targets_tag(color_apply, tag)
                && apply_continuous_targets_tag(subtype_apply, tag)
                && apply_continuous_targets_tag(ability_apply, tag)
                && color_apply.until == subtype_apply.until
                && color_apply.until == ability_apply.until
                && matches!(
                    &color_apply.modification,
                    Some(crate::continuous::Modification::SetColors(_))
                )
                && matches!(
                    &subtype_apply.modification,
                    Some(crate::continuous::Modification::AddSubtypes(_))
                )
                && matches!(
                    &ability_apply.modification,
                    Some(crate::continuous::Modification::AddAbility(_))
                )
        }
        _ => false,
    }
}

fn describe_optional_look_then_reveal_top_rest_bottom(effects: &[Effect]) -> Option<String> {
    let [with_id_effect, if_effect] = effects else {
        return None;
    };
    let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::You)
    {
        return None;
    }
    let [look_effect] = may.effects.as_slice() else {
        return None;
    };
    let look_at_top = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let if_effect = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let [choose_effect, reveal_effect, move_effect, remainder_effect] = if_effect.then.as_slice()
    else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let (_, move_to_top) = for_each_tagged_for_compaction(move_effect)?;
    let remainder = remainder_effect
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if look_at_top.player != PlayerFilter::You
        || choose.chooser != PlayerFilter::You
        || choose.is_search
        || !choose_references_tag(choose, &look_at_top.tag)
        || !for_each_reveals_tag(reveal, choose.tag.as_str())
        || !for_each_moves_tag_to_library_top(move_to_top, choose.tag.as_str())
        || remainder.tag != look_at_top.tag
        || remainder.keep_tagged.as_ref() != Some(&choose.tag)
        || remainder.player != look_at_top.player
    {
        return None;
    }

    let selection = if choose.count.is_any_number() {
        format!(
            "any number of {}",
            describe_any_number_filter_from_looked_cards(look_at_top, choose)?
        )
    } else if choose.count == ChoiceCount::up_to(1) {
        let single = describe_choose_filter_from_looked_cards(look_at_top, choose)?;
        format!("up to one {}", strip_indefinite_article(&single))
    } else {
        describe_counted_choose_filter_from_looked_cards(look_at_top, choose)?
    };
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let (count_text, noun, where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let selected_reference = if choose.count.max == Some(1) {
        "that card"
    } else {
        "those cards"
    };
    let selected_order = if choose.count.max == Some(1) {
        ""
    } else {
        " in any order"
    };
    let remainder_order = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    };

    Some(format!(
        "You may look at the top {count_text} {noun} of {owner} library{where_clause}. If you do, reveal {selection} from among them, then put {selected_reference} on top of {owner} library{selected_order} and the rest on the bottom of {owner} library{remainder_order}"
    ))
}

fn describe_looked_hand_rest_bottom_clause(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    reveal_top: Option<&crate::effects::RevealTaggedEffect>,
    choose: &crate::effects::ChooseObjectsEffect,
    move_effect: &Effect,
    remainder: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    if reveal_top.is_some_and(|reveal| reveal.tag != look_at_top.tag)
        || choose.is_search
        || !choose_references_tag(choose, &look_at_top.tag)
        || remainder.tag != look_at_top.tag
        || remainder.keep_tagged.as_ref() != Some(&choose.tag)
        || remainder.player != look_at_top.player
    {
        return None;
    }
    let (_, move_to_hand) = for_each_tagged_for_compaction(move_effect)?;
    if !for_each_moves_tag_to_hand(move_to_hand, choose.tag.as_str()) {
        return None;
    }

    let selection = if choose.count.is_any_number() {
        format!(
            "any number of {}",
            describe_any_number_filter_from_looked_cards(look_at_top, choose)?
        )
    } else {
        describe_counted_choose_filter_from_looked_cards(look_at_top, choose)?
    };
    let mandatory = choose.count.min > 0
        && choose.count.max == Some(choose.count.min)
        && !choose.count.dynamic_x
        && choose.search_mode != SearchSelectionMode::Optional;
    let actor = if mandatory {
        "Put".to_string()
    } else if choose.chooser == PlayerFilter::You {
        "You may put".to_string()
    } else {
        let chooser = capitalize_first(&describe_player_filter(&choose.chooser));
        format!("{chooser} may put")
    };
    let opener = if look_at_top.reveal || reveal_top.is_some() {
        "Reveal"
    } else {
        "Look at"
    };
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand = describe_possessive_player_filter(&choose.chooser);
    let (count_text, noun, where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let order = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    };

    Some(format!(
        "{opener} the top {count_text} {noun} of {owner} library{where_clause}. {actor} {selection} from among them into {hand} hand and the rest on the bottom of {owner} library{order}"
    ))
}

fn shuffle_matches_looked_library(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> bool {
    shuffle.player == look_at_top.player && shuffle.target_spec.is_none()
}

fn describe_looked_battlefield_then_shuffle(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    move_effect: &Effect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> Option<String> {
    if choose.is_search
        || !choose_references_tag(choose, &look_at_top.tag)
        || !shuffle_matches_looked_library(look_at_top, shuffle)
    {
        return None;
    }
    let (_, for_each) = for_each_tagged_for_compaction(move_effect)?;
    if for_each.tag != choose.tag || for_each.effects.len() != 1 {
        return None;
    }
    let move_to_zone = unwrap_basic_tag_wrappers(&for_each.effects[0])
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || !matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    {
        return None;
    }

    let selection = describe_looked_battlefield_selection(choose)?;
    let actor = if choose.count.min == 0 {
        if choose.chooser == PlayerFilter::You {
            "You may put".to_string()
        } else {
            format!(
                "{} may put",
                capitalize_first(&describe_player_filter(&choose.chooser))
            )
        }
    } else {
        "Put".to_string()
    };
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let (count_text, noun, where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let opener = if look_at_top.reveal {
        "Reveal"
    } else {
        "Look at"
    };
    let entry_state = describe_battlefield_entry_state_for_looked_move(move_to_zone);
    Some(format!(
        "{opener} the top {count_text} {noun} of {owner} library{where_clause}. {actor} {selection} from among them onto the battlefield{entry_state}. Then shuffle"
    ))
}

fn effect_reveals_looked_choice(effect: &Effect, tag: &crate::TagKey) -> bool {
    if let Some(reveal) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::RevealTaggedEffect>()
    {
        return reveal.tag == *tag;
    }
    effect
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()
        .is_some_and(|for_each| for_each_reveals_tag(for_each, tag.as_str()))
}

fn effect_moves_looked_choice_to_hand(effect: &Effect, tag: &crate::TagKey) -> bool {
    if let Some(move_to_zone) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
    {
        return move_to_zone.zone == Zone::Hand
            && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if found == tag);
    }
    for_each_tagged_for_compaction(effect)
        .is_some_and(|(_, for_each)| for_each_moves_tag_to_hand(for_each, tag.as_str()))
}

fn describe_looked_reveal_hand_then_shuffle(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    reveal_effect: &Effect,
    move_effect: &Effect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> Option<String> {
    if look_at_top.reveal
        || choose.is_search
        || !choose_references_tag(choose, &look_at_top.tag)
        || !effect_reveals_looked_choice(reveal_effect, &choose.tag)
        || !effect_moves_looked_choice_to_hand(move_effect, &choose.tag)
        || !shuffle_matches_looked_library(look_at_top, shuffle)
    {
        return None;
    }
    let selection = describe_counted_choose_filter_from_looked_cards(look_at_top, choose)?;
    let (selection, where_clause) = selection
        .split_once(", where X is ")
        .map(|(head, basis)| (head.to_string(), format!(", where X is {basis}")))
        .unwrap_or((selection, String::new()));
    let reveal_actor = if choose.count.min == 0 {
        if choose.chooser == PlayerFilter::You {
            "You may reveal".to_string()
        } else {
            format!(
                "{} may reveal",
                capitalize_first(&describe_player_filter(&choose.chooser))
            )
        }
    } else {
        "Reveal".to_string()
    };
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand = describe_possessive_player_filter(&choose.chooser);
    let (count_text, noun, look_where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library{look_where_clause}. {reveal_actor} {selection} from among them{where_clause}. Put the revealed cards into {hand} hand, then shuffle"
    ))
}

fn looked_granted_ability_text(effect: &Effect, chosen_tag: &crate::TagKey) -> Option<String> {
    let apply = unwrap_basic_tag_wrappers(effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !matches!(apply.until, Until::Forever)
        || apply.condition.is_some()
        || !apply.runtime_modifications.is_empty()
        || !matches!(
            apply.target_spec.as_ref(),
            Some(spec) if choose_spec_references_tagged_object(spec, chosen_tag)
        )
    {
        return None;
    }
    let grants_ability = |modification: &crate::continuous::Modification| {
        matches!(
            modification,
            crate::continuous::Modification::AddAbility(_)
                | crate::continuous::Modification::AddAbilityGeneric(_)
        )
    };
    if !apply.modification.as_ref().is_some_and(grants_ability)
        || !apply.additional_modifications.iter().all(grants_ability)
    {
        return None;
    }
    let mut text = describe_effect(effect).trim_end_matches('.').to_string();
    for subject in ["That object", "That card", "The chosen card"] {
        if let Some(rest) = text.strip_prefix(subject) {
            text = format!("It{rest}");
            break;
        }
    }
    Some(capitalize_first(&text))
}

fn describe_looked_battlefield_grant_then_remainder(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    reveal_top: Option<&crate::effects::RevealTaggedEffect>,
    choose: &crate::effects::ChooseObjectsEffect,
    move_effect: &Effect,
    grant_effect: &Effect,
    remainder: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    let base = describe_look_at_top_choose_battlefield_rest_bottom(
        look_at_top,
        reveal_top,
        choose,
        move_effect,
        remainder,
    )?;
    let grant = looked_granted_ability_text(grant_effect, &choose.tag)?;
    let (put_selected, rest) = base.split_once(". Put the rest")?;
    Some(format!("{put_selected}. {grant}. Then put the rest{rest}"))
}

fn describe_looked_battlefield_rest_then_reflexive(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    reveal_top: Option<&crate::effects::RevealTaggedEffect>,
    choose: &crate::effects::ChooseObjectsEffect,
    move_effect: &Effect,
    reflexive: &crate::effects::ReflexiveTriggerEffect,
    remainder: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    let (with_id, for_each) = for_each_tagged_for_compaction(move_effect)?;
    let with_id = with_id?;
    if reflexive.condition != with_id.id || for_each.tag != choose.tag {
        return None;
    }
    let EffectPredicate::AffectedObjectMatchesCardType {
        card_type,
        negated: false,
    } = reflexive.predicate
    else {
        return None;
    };
    let move_to_zone = for_each.effects.first().and_then(|effect| {
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
    })?;
    if move_to_zone.zone != Zone::Battlefield
        || !matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    {
        return None;
    }
    let base = describe_look_at_top_choose_battlefield_rest_bottom(
        look_at_top,
        reveal_top,
        choose,
        move_effect,
        remainder,
    )?;
    let type_word = describe_card_type_word_local(card_type);
    let mut triggered = lowercase_first(&describe_effect_list(&reflexive.effects));
    let affected_subject = format!("a {type_word} ");
    if let Some(rest) = triggered.strip_prefix(&affected_subject) {
        triggered = format!("it {rest}");
    }
    Some(format!(
        "{base}. When a {type_word} is put onto the battlefield this way, {triggered}"
    ))
}

/// Looked-card compactors normally run from `describe_effect_list`, but
/// resolution programs prefer `describe_effect_clause_list`. Keep the shared
/// card-pool routing shapes at that earlier dispatch point so their tagged
/// implementation details do not leak into compiled rules text.
fn describe_looked_cards_clause_prefix(effects: &[Effect]) -> Option<(String, usize)> {
    let hidden_prefix = effects
        .iter()
        .take_while(|effect| {
            effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some()
                || effect
                    .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
                    .is_some()
                || effect
                    .downcast_ref::<crate::effects::TagTriggeringBlockersEffect>()
                    .is_some()
        })
        .count();
    let visible = effects.get(hidden_prefix..)?;

    fn optional_choice_and_move(
        effect: &Effect,
    ) -> Option<(crate::effects::ChooseObjectsEffect, &Effect)> {
        let may = effect.downcast_ref::<crate::effects::MayEffect>()?;
        let [choose_effect, move_effect] = may.effects.as_slice() else {
            return None;
        };
        let mut choose = choose_effect
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()?
            .clone();
        if may
            .decider
            .as_ref()
            .is_some_and(|decider| *decider != choose.chooser)
        {
            return None;
        }
        // The outer May is the optionality.  Existing looked-card renderers
        // derive "may put" from the choice count, so reflect that without
        // mutating the runtime effect.
        choose.count.min = 0;
        Some((choose, move_effect))
    }

    if let [look_effect, may_effect, shuffle_effect, ..] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some((choose, move_effect)) = optional_choice_and_move(may_effect)
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) =
            describe_looked_battlefield_then_shuffle(look_at_top, &choose, move_effect, shuffle)
    {
        return Some((compact, hidden_prefix + 3));
    }

    if let [look_effect, may_effect, grant_effect, remainder_effect, ..] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some((choose, move_effect)) = optional_choice_and_move(may_effect)
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
        && let Some(compact) = describe_looked_battlefield_grant_then_remainder(
            look_at_top,
            None,
            &choose,
            move_effect,
            grant_effect,
            remainder,
        )
    {
        return Some((compact, hidden_prefix + 4));
    }

    if let [look_effect, may_effect, remainder_effect, ..] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some((choose, move_effect)) = optional_choice_and_move(may_effect)
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
    {
        if let Some(compact) = describe_looked_hand_rest_bottom_clause(
            look_at_top,
            None,
            &choose,
            move_effect,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 3));
        }
        if let Some(compact) = describe_look_at_top_choose_battlefield_rest_bottom(
            look_at_top,
            None,
            &choose,
            move_effect,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 3));
        }
    }

    if let [
        look_effect,
        choose_effect,
        reveal_effect,
        move_effect,
        shuffle_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) = describe_looked_reveal_hand_then_shuffle(
            look_at_top,
            choose,
            reveal_effect,
            move_effect,
            shuffle,
        )
    {
        return Some((compact, hidden_prefix + 5));
    }

    if let [look_effect, choose_effect, move_effect, shuffle_effect, ..] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) =
            describe_looked_battlefield_then_shuffle(look_at_top, choose, move_effect, shuffle)
    {
        return Some((compact, hidden_prefix + 4));
    }

    if let [
        look_effect,
        choose_effect,
        move_effect,
        reflexive_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(reflexive) =
            reflexive_effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
        && let Some(compact) = describe_looked_battlefield_rest_then_reflexive(
            look_at_top,
            None,
            choose,
            move_effect,
            reflexive,
            remainder,
        )
    {
        return Some((compact, hidden_prefix + 5));
    }

    if let [
        look_effect,
        reveal_top_effect,
        choose_effect,
        move_effect,
        reflexive_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_top) =
            reveal_top_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(reflexive) =
            reflexive_effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
        && let Some(compact) = describe_looked_battlefield_rest_then_reflexive(
            look_at_top,
            Some(reveal_top),
            choose,
            move_effect,
            reflexive,
            remainder,
        )
    {
        return Some((compact, hidden_prefix + 6));
    }

    if let [
        look_effect,
        choose_effect,
        move_effect,
        grant_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
        && let Some(compact) = describe_looked_battlefield_grant_then_remainder(
            look_at_top,
            None,
            choose,
            move_effect,
            grant_effect,
            remainder,
        )
    {
        return Some((compact, hidden_prefix + 5));
    }

    if let [
        look_effect,
        reveal_top_effect,
        choose_effect,
        move_effect,
        grant_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_top) =
            reveal_top_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
        && let Some(compact) = describe_looked_battlefield_grant_then_remainder(
            look_at_top,
            Some(reveal_top),
            choose,
            move_effect,
            grant_effect,
            remainder,
        )
    {
        return Some((compact, hidden_prefix + 6));
    }

    if let [
        look_effect,
        reveal_top_effect,
        hand_choose_effect,
        hand_move_effect,
        matching_choose_effect,
        matching_move_effect,
        rest_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_top) =
            reveal_top_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(hand_choose) =
            hand_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(matching_choose) =
            matching_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(rest) = rest_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
        && let Some(compact) = describe_looked_one_hand_then_matching_to_zone_rest_graveyard(
            look_at_top,
            Some(reveal_top),
            hand_choose,
            hand_move_effect,
            matching_choose,
            matching_move_effect,
            rest,
        )
    {
        return Some((compact, hidden_prefix + 7));
    }
    if let [
        look_effect,
        hand_choose_effect,
        hand_move_effect,
        matching_choose_effect,
        matching_move_effect,
        rest_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(hand_choose) =
            hand_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(matching_choose) =
            matching_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(rest) = rest_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
        && let Some(compact) = describe_looked_one_hand_then_matching_to_zone_rest_graveyard(
            look_at_top,
            None,
            hand_choose,
            hand_move_effect,
            matching_choose,
            matching_move_effect,
            rest,
        )
    {
        return Some((compact, hidden_prefix + 6));
    }

    if let [
        look_effect,
        battlefield_choose_effect,
        battlefield_move_effect,
        if_not_moved_effect,
        rest_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(battlefield_choose) =
            battlefield_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((battlefield_move_id, battlefield_move)) =
            for_each_tagged_for_compaction(battlefield_move_effect)
        && let Some(if_not_moved) = if_not_moved_effect.downcast_ref::<crate::effects::IfEffect>()
        && let Some(rest) = rest_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
        && let Some(compact) = describe_look_at_top_then_may_put_battlefield_else_hand_rest_bottom(
            look_at_top,
            battlefield_choose,
            battlefield_move_id,
            battlefield_move,
            if_not_moved,
            rest,
        )
    {
        return Some((compact, hidden_prefix + 5));
    }

    if let [
        look_effect,
        reveal_top_effect,
        choose_effect,
        move_effect,
        rest_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_top) =
            reveal_top_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(rest) = rest_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
        && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(move_effect)
        && let Some(compact) = describe_look_at_top_then_put_into_hand_rest_graveyard(
            look_at_top,
            Some(reveal_top),
            choose,
            None,
            move_to_hand,
            rest,
        )
    {
        return Some((compact, hidden_prefix + 5));
    }

    if let [
        look_effect,
        reveal_top_effect,
        choose_effect,
        move_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_top) =
            reveal_top_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
    {
        if let Some(compact) = describe_looked_hand_rest_bottom_clause(
            look_at_top,
            Some(reveal_top),
            choose,
            move_effect,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 5));
        }
        if let Some(compact) = describe_look_at_top_choose_battlefield_rest_bottom(
            look_at_top,
            Some(reveal_top),
            choose,
            move_effect,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 5));
        }
        if let Some((_, move_chosen)) = for_each_tagged_for_compaction(move_effect)
            && let Some(compact) = describe_look_at_top_then_put_any_matching_to_zone_rest_bottom(
                look_at_top,
                Some(reveal_top),
                choose,
                move_chosen,
                remainder,
            )
        {
            return Some((compact, hidden_prefix + 5));
        }
    }

    if let [
        look_effect,
        choose_effect,
        reveal_effect,
        move_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(reveal) = reveal_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
        && let Some((_, move_chosen)) = for_each_tagged_for_compaction(move_effect)
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
    {
        if let Some(compact) = describe_look_at_top_then_reveal_put_on_top_rest_bottom(
            look_at_top,
            choose,
            reveal,
            move_chosen,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 5));
        }
        if let Some(compact) = describe_look_at_top_then_reveal_put_into_hand_rest_bottom(
            look_at_top,
            choose,
            Some(reveal),
            move_chosen,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 5));
        }
    }

    if let [
        look_effect,
        choose_effect,
        move_effect,
        remainder_effect,
        ..,
    ] = visible
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(remainder) = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
    {
        if let Some(compact) =
            describe_looked_up_to_one_top_rest_bottom(look_at_top, choose, move_effect, remainder)
        {
            return Some((compact, hidden_prefix + 4));
        }
        if let Some(compact) = describe_looked_hand_rest_bottom_clause(
            look_at_top,
            None,
            choose,
            move_effect,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 4));
        }
        if let Some(compact) = describe_look_at_top_choose_battlefield_rest_bottom(
            look_at_top,
            None,
            choose,
            move_effect,
            remainder,
        ) {
            return Some((compact, hidden_prefix + 4));
        }
        if let Some((_, move_chosen)) = for_each_tagged_for_compaction(move_effect)
            && let Some(compact) = describe_look_at_top_then_put_any_matching_to_zone_rest_bottom(
                look_at_top,
                None,
                choose,
                move_chosen,
                remainder,
            )
        {
            return Some((compact, hidden_prefix + 4));
        }
    }

    None
}

pub(crate) fn describe_effect_clause_list(effects: &[Effect]) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }
    if let Some(compact) = describe_linked_counter_followup(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_typed_counter_sentence_split(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_optional_search_battlefield_partition_effects(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_discard_redraw_mana_value_ladder(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_look_hand_optional_exile_persistent_play_tax(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_hidden_exile_partition_with_persistent_permission(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_each_opponent_top_card_hidden_exile_permission(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_exile_all_then_each_player_may_deploy_and_return_exiled(effects)
    {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_exile_top_play_then_additional_land(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_exile_two_creatures_then_controller_consults(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_exile_top_then_search_to_hand_and_shuffle(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_two_target_players_each_search_to_top(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_search_reveal_nested_may_move_else_hand(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_bulk_battlefield_move_then_grant_decayed(effects) {
        return Some(lowercase_first(&compact));
    }
    // Trigger and spell resolution normally enters through the clause-list
    // renderer. Recognize the complete reveal-until partition before the
    // generic clause joiner exposes its internal tagged iterations as "for
    // each of those objects" / "unless it's a permanent" scaffolding.
    if effects.len() >= 3 {
        let consult_refs = effects[..3].iter().collect::<Vec<_>>();
        if let Some(compact) = render_consult_reveal_put_battlefield_rest_graveyard(&consult_refs) {
            let compact = lowercase_first(&compact);
            if effects.len() == 3 {
                return Some(compact);
            }
            let suffix = describe_effect_clause_list(&effects[3..])
                .unwrap_or_else(|| describe_effect_list(&effects[3..]));
            return Some(format!(
                "{}. {}",
                compact.trim_end_matches('.'),
                capitalize_first(suffix.trim_end_matches('.'))
            ));
        }
    }
    if let Some(compact) = describe_exile_top_choose_one_play_next_turn(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_each_player_reveal_set_may_move_else_draw(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_consult_characteristic_boost_then_all_revealed_bottom(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_consult_reflexive_damage_then_all_revealed_bottom(effects) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_energy_payment_failure_fallback(effects) {
        return Some(lowercase_first(&compact));
    }
    if let [first, second] = effects
        && let Some(compact) = describe_action_and_get_energy_pair(first, second)
    {
        return Some(lowercase_first(&compact));
    }

    if let Some(compact) = describe_milled_creatures_returned_then_animated(effects) {
        return Some(lowercase_first(&compact));
    }

    // Spell and ability resolution prefers the clause-list renderer, so run
    // compound target-plus-linked-set prefixes here before the generic pair
    // renderer consumes only their first two visible effects. This preserves
    // the semantic union tag for follow-up clauses such as "those creatures",
    // "with that name", and event-count references.
    if let Some((compact, consumed)) = describe_linked_target_set_followup_prefix(effects)
        .or_else(|| describe_same_name_exile_then_investigate_prefix(effects))
        .or_else(|| describe_target_same_name_action_fanout_prefix(effects))
    {
        let compact = lowercase_first(&compact);
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some(compact) = describe_returned_object_set_to_enchantment(effects) {
        return Some(lowercase_first(&compact));
    }

    if let Some(compact) = describe_optional_look_then_reveal_top_rest_bottom(effects) {
        return Some(compact);
    }

    if let Some((compact, consumed)) = describe_typed_collection_selection_prefix(effects) {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    if let Some((compact, consumed)) = describe_looked_cards_clause_prefix(effects) {
        if consumed == effects.len() {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[consumed..])
            .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    let early_refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = describe_look_hand_choose_then_discard(&early_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_player_damage_then_same_player_discards(&early_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_target_player_sacrifice_then_gain_toughness(&early_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_reveal_hand_then_gain_for_that_players_hand(&early_refs) {
        return Some(lowercase_first(&compact));
    }
    // Preserve the typed revealed-card pool through its selection, movement,
    // and remainder disposition before the generic target-player reveal
    // prefix splits the program into unrelated clauses.
    if effects.len() >= 5
        && let Some(compact) =
            describe_target_player_reveal_top_may_put_matching_rest_bottom(&effects[..5])
    {
        let compact = lowercase_first(&compact);
        if effects.len() == 5 {
            return Some(compact);
        }
        let suffix = describe_effect_clause_list(&effects[5..])
            .unwrap_or_else(|| describe_effect_list(&effects[5..]));
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }
    if effects.len() >= 3
        && structural_unwrap_render_wrappers(&effects[0])
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_some()
        && let Some(observation_prefix) =
            describe_target_player_reveal_top(&effects[0], &effects[1])
    {
        let observed_refs = effects[1..].iter().collect::<Vec<_>>();
        if let Some((mut compact, consumed)) =
            describe_immediate_observation_conditionals(&observed_refs)
        {
            if let Some((_, remainder)) = compact.split_once(". ") {
                compact = format!("{observation_prefix}. {remainder}");
            }
            let consumed = consumed + 1;
            if consumed == effects.len() {
                return Some(lowercase_first(&compact));
            }
            let suffix = describe_effect_clause_list(&effects[consumed..])
                .unwrap_or_else(|| describe_effect_list(&effects[consumed..]));
            return Some(format!(
                "{}. {}",
                lowercase_first(compact.trim_end_matches('.')),
                capitalize_first(suffix.trim_end_matches('.'))
            ));
        }
    }
    if effects.len() >= 2
        && let Some(prefix) = describe_target_player_reveal_top(&effects[0], &effects[1])
    {
        if effects.len() == 2 {
            return Some(lowercase_first(&prefix));
        }
        let suffix = describe_effect_clause_list(&effects[2..])
            .unwrap_or_else(|| describe_effect_list(&effects[2..]));
        return Some(format!(
            "{}. {}",
            lowercase_first(&prefix),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    // Resolution programs prefer the clause-list renderer over
    // `describe_effect_list`, so structural prefixes that only live in the
    // latter never get a chance to run for ordinary spell and ability text.
    // Match the full control/untap/haste bundle before its two-effect prefix,
    // or the haste grant loses the shared object reference and conjunction.
    if effects.len() >= 3
        && let Some(bundle) = describe_gain_control_untap_haste_clause_structural(&effects[..3])
    {
        let bundle = lowercase_first(&bundle);
        if effects.len() == 3 {
            return Some(bundle);
        }
        let suffix = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        return Some(format!(
            "{}. {}",
            bundle.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }
    if effects.len() >= 3
        && let Some(bundle) = describe_gain_control_untap_haste_structural(&effects[..3])
    {
        let bundle = lowercase_first(&bundle);
        if effects.len() == 3 {
            return Some(bundle);
        }
        let suffix = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        return Some(format!(
            "{}. {}",
            bundle.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    // Keep the reusable two-effect control/untap recognizer at the real
    // dispatch point after longer structural bundles have declined.
    if effects.len() >= 2
        && let Some(prefix) = describe_gain_control_then_untap_structural(&effects[..2])
    {
        let prefix = lowercase_first(&prefix);
        if effects.len() == 2 {
            return Some(prefix);
        }
        let suffix = describe_effect_clause_list(&effects[2..])
            .unwrap_or_else(|| describe_effect_list(&effects[2..]));
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }

    let bundle_refs = effects.iter().collect::<Vec<_>>();
    let visible_refs = bundle_refs
        .iter()
        .copied()
        .filter(|effect| {
            effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_none()
                && effect
                    .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
                    .is_none()
                && effect
                    .downcast_ref::<crate::effects::TagTriggeringBlockersEffect>()
                    .is_none()
        })
        .collect::<Vec<_>>();
    if let Some(compact) = describe_same_name_reference_search_bundle(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_single_hand_reveal_same_name_search(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_target_card_same_name_extraction(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_target_creature_damage_then_destroy_attached(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_destroy_target_creature_then_owner_gains(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let [first, second] = visible_refs.as_slice()
        && let Some(compact) = describe_target_continuous_fanout_pair(first, second)
            .or_else(|| describe_target_prevention_fanout_pair(first, second))
    {
        return Some(lowercase_first(&compact));
    }
    if let [first, second] = visible_refs.as_slice()
        && let Some(compact) = describe_target_creature_damage_fanout_pair(first, second)
    {
        return Some(lowercase_first(&compact));
    }
    if visible_refs.len() >= 2
        && let Some(compact) =
            describe_target_same_name_action_fanout_pair(visible_refs[0], visible_refs[1])
    {
        let compact = lowercase_first(&compact);
        if visible_refs.len() == 2 {
            return Some(compact);
        }
        let suffix = visible_refs[2..]
            .iter()
            .map(|effect| describe_effect(effect).trim_end_matches('.').to_string())
            .collect::<Vec<_>>()
            .join(". ");
        return Some(format!(
            "{}. {}",
            compact.trim_end_matches('.'),
            capitalize_first(suffix.trim_end_matches('.'))
        ));
    }
    if let Some(compact) = describe_look_hand_choose_then_discard(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_target_player_look_top_may_move_that_card(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_target_player_consult_exile_shuffle_may_cast(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    if let Some(compact) = describe_choose_name_reveal_discard_failure_draw_bundle(&visible_refs) {
        return Some(lowercase_first(&compact));
    }
    let search_sequence_refs = if matches!(
        visible_refs.first(),
        Some(effect) if effect.downcast_ref::<crate::effects::TargetOnlyEffect>().is_some()
    ) {
        &visible_refs[1..]
    } else {
        visible_refs.as_slice()
    };
    if let [sequence_effect, shuffle_effect] = search_sequence_refs
        && let Some(sequence) = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) = describe_search_sequence_then_shuffle(sequence, shuffle)
    {
        return Some(lowercase_first(&compact));
    }
    if visible_refs.len() >= 2
        && let Some(compact) = describe_source_exile_with_counters_pair(
            visible_refs[visible_refs.len() - 2],
            visible_refs[visible_refs.len() - 1],
        )
    {
        let compact = lowercase_first(&compact);
        if visible_refs.len() == 2 {
            return Some(compact);
        }
        let prefix_effects = &effects[..effects.len() - 2];
        let prefix = describe_effect_clause_list(prefix_effects)
            .unwrap_or_else(|| lowercase_first(&describe_effect_list(prefix_effects)));
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(&compact)
        ));
    }
    if let [choose_effect, move_effect, shuffle_effect, cast_effect] = visible_refs.as_slice()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) =
            describe_search_choose_then_exile_and_cast(choose, move_effect, shuffle, cast_effect)
    {
        return Some(cleanup_decompiled_text(&lowercase_first(&compact)));
    }
    if let [choose_effect, cast_effect, shuffle_effect] = search_sequence_refs
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) =
            describe_search_choose_then_cast_then_shuffle(choose, cast_effect, shuffle)
    {
        return Some(cleanup_decompiled_text(&lowercase_first(&compact)));
    }
    if let Some(compact) = describe_choose_each_basic_land_type_then_destroy(&visible_refs) {
        return Some(compact);
    }
    if let Some(compact) =
        describe_may_cast_target_graveyard_spell_then_exile_replacement(&visible_refs)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_then_same_player_discards(&visible_refs) {
        return Some(compact);
    }
    if let Some((compact, consumed)) =
        describe_same_referenced_player_action_sequence(&visible_refs)
        && consumed == visible_refs.len()
    {
        return Some(lowercase_first(&compact));
    }
    if let [for_players_effect, destroy_effect] = visible_refs.as_slice()
        && let Some(for_players) =
            for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()
        && let Some(destroy) = unwrap_basic_tag_wrappers(destroy_effect)
            .downcast_ref::<crate::effects::DestroyEffect>()
        && let Some(compact) =
            describe_for_players_may_choose_then_destroy_chosen(for_players, destroy)
    {
        return Some(compact);
    }
    if let [choose_effect, reveal_effect, move_effect, shuffle_effect] = visible_refs.as_slice()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(reveal) = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(move_to_zone) = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) =
            describe_search_choose_then_move(choose, Some(reveal), move_to_zone, Some(shuffle))
    {
        return Some(cleanup_decompiled_text(&lowercase_first(&compact)));
    }
    if let Some(compact) = describe_look_exile_one_rest_bottom_cast_else_hand(&bundle_refs) {
        return Some(compact);
    }
    if let [for_players_effect, look_effect, grant_effect] = bundle_refs.as_slice()
        && let Some(for_players) =
            for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()
        && let Some(look) = look_effect.downcast_ref::<crate::effects::LookAtObjectsEffect>()
        && let Some(grant) = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
        && let Some(compact) =
            describe_for_players_bottom_library_exile_then_look_cast(for_players, look, grant)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_exile_target_search_same_name_exile_shuffle_bundle(&bundle_refs)
    {
        return Some(compact);
    }
    let is_reference_search_bundle = match effects {
        [exile, for_each, shuffle] => {
            exile
                .downcast_ref::<crate::effects::TaggedEffect>()
                .is_some()
                && (for_each
                    .downcast_ref::<crate::effects::ForEachObject>()
                    .is_some()
                    || for_each
                        .downcast_ref::<crate::effects::ForEachTaggedEffect>()
                        .is_some())
                && shuffle
                    .downcast_ref::<crate::effects::ShuffleLibraryEffect>()
                    .is_some()
        }
        [look, choose, exile, for_each, shuffle] => {
            look.downcast_ref::<crate::effects::LookAtHandEffect>()
                .is_some()
                && choose
                    .downcast_ref::<crate::effects::ChooseObjectsEffect>()
                    .is_some()
                && exile
                    .downcast_ref::<crate::effects::MoveToZoneEffect>()
                    .is_some()
                && (for_each
                    .downcast_ref::<crate::effects::ForEachObject>()
                    .is_some()
                    || for_each
                        .downcast_ref::<crate::effects::ForEachTaggedEffect>()
                        .is_some())
                && shuffle
                    .downcast_ref::<crate::effects::ShuffleLibraryEffect>()
                    .is_some()
        }
        _ => false,
    };
    if is_reference_search_bundle {
        let compact = describe_effect_list(effects);
        if compact.starts_with(
            "Exile all cards from target player's graveyard other than basic land cards",
        ) || compact.starts_with(
            "Target opponent reveals their hand. Choose up to X nonland cards from it and exile them",
        ) {
            return Some(compact);
        }
    }
    if let Some(compact) = describe_reveal_hand_choose_graveyard_exile_bundle(&bundle_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_name_reveal_hand_discard_named_bundle(&bundle_refs) {
        return Some(compact);
    }
    if let Some(reveal_line) =
        describe_choose_hand_then_reveal_chosen_pair(&effects[0], &effects[1])
    {
        if effects.len() == 2 {
            return Some(reveal_line);
        }
        let rest = describe_effect_clause_list(&effects[2..])
            .unwrap_or_else(|| describe_effect_list(&effects[2..]));
        if !rest.trim().is_empty() {
            return Some(format!(
                "{reveal_line}. {}",
                capitalize_first(rest.trim_end_matches('.'))
            ));
        }
        return Some(reveal_line);
    }

    if let [may_effect, shuffle_effect] = effects
        && let Some(may) = may_effect.downcast_ref::<crate::effects::MayEffect>()
        && may.decider.is_none()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(compact) = describe_may_search_choose_for_each_with_shuffle(may, shuffle)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_destroy_all_groups_then_draw_for_destroyed(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_return_as_aura_with_granted_abilities(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_exile_source_and_target(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_target_permanent_shuffle_reveal_permanent_card(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_color_target_and_shared_color_protection(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_target_and_shared_color_inline_ability_grant(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_look_reorder_then_may_shuffle(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_target_modifications_then_exile_top_play(effects) {
        return Some(compact);
    }

    let effect_refs = effects.iter().collect::<Vec<_>>();
    if effects.len() >= 4
        && let Some(look_at_top) = effects[0].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = effects[1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(remainder) =
            effects[3].downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
        && let Some(compact) = describe_look_at_top_choose_battlefield_rest_bottom(
            look_at_top,
            None,
            choose,
            &effects[2],
            remainder,
        )
    {
        if effects.len() == 4 {
            return Some(compact);
        }
        let rest = describe_effect_clause_list(&effects[4..])
            .unwrap_or_else(|| describe_effect_list(&effects[4..]));
        return Some(format!(
            "{compact}. {}",
            capitalize_first(rest.trim_end_matches('.'))
        ));
    }
    if effects.len() >= 5
        && let Some(compact) =
            describe_for_players_choose_move_then_characteristics(&effect_refs[..5])
    {
        if effects.len() == 5 {
            return Some(compact);
        }
        let rest = describe_effect_clause_list(&effects[5..])
            .unwrap_or_else(|| describe_effect_list(&effects[5..]));
        return Some(format!(
            "{compact}. {}",
            capitalize_first(rest.trim_end_matches('.'))
        ));
    }
    if effects.len() >= 3
        && let Some(compact) =
            describe_consult_may_cast_remainder_bottom_sequence(&effect_refs[..3])
    {
        if effects.len() == 3 {
            return Some(compact);
        }
        let rest = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        return Some(format!(
            "{compact}. {}",
            capitalize_first(rest.trim_end_matches('.'))
        ));
    }
    if effects.len() >= 3
        && let Some(compact) =
            describe_consult_exile_may_cast_rest_bottom_sequence(&effect_refs[..3])
    {
        if effects.len() == 3 {
            return Some(compact);
        }
        let rest = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        return Some(format!(
            "{compact}. {}",
            capitalize_first(rest.trim_end_matches('.'))
        ));
    }
    if effects.len() > 3
        && let Some(prefix) = describe_choose_top_exile_then_play_structural(&effects[..3])
    {
        let rest = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        return Some(format!(
            "{prefix}. Then {}",
            lowercase_first(rest.trim_end_matches('.'))
        ));
    }
    if let Some(compact) = describe_choose_top_exile_then_play_structural(effects) {
        return Some(compact);
    }
    if effects.len() > 3
        && let Some(suffix) =
            describe_choose_top_exile_then_play_structural(&effects[effects.len() - 3..])
    {
        let prefix = describe_effect_clause_list(&effects[..effects.len() - 3])
            .unwrap_or_else(|| describe_effect_list(&effects[..effects.len() - 3]));
        return Some(format!("{}. {suffix}", prefix.trim_end_matches('.')));
    }
    if effects.len() >= 3
        && let Some(exile_top) =
            effects[0].downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
        && let Some(choose) = effects[1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(grant_play) = effects[2].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
        && let Some(prefix) = describe_exile_top_choose_one_then_play(exile_top, choose, grant_play)
    {
        if effects.len() == 3 {
            return Some(prefix);
        }
        let rest = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| describe_effect_list(&effects[3..]));
        return Some(format!(
            "{prefix}. Then {}",
            lowercase_first(rest.trim_end_matches('.'))
        ));
    }
    if let Some(compact) =
        describe_sacrifice_return_from_graveyard_then_exile_source_bundle(effects)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_chosen_creatures_blessing_additional_combat_clause(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_power_cards_for_mana_clause_bundle(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_gain_life_shuffle_source_and_graveyard(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_untap_triggering_then_remove_from_combat(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_remove_counter_then_no_counters_conditional(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_linked_graveyard_choices_then_may_return_bundle(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_graveyard_mana_ladder_return_clause_bundle(&effect_refs) {
        return Some(compact);
    }
    if let [first, second] = effects
        && let Some(compact) = describe_put_counters_then_untap_them(first, second)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_return_then_color_subtype_addition_compact(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_countered_spell_same_name_search_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_countered_spell_controller_consult_cast_shuffle(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_damage_each_then_tap_damaged_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_exile_source_and_attacking_nonflying_creature(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_exile_source_and_target(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_two_tap_then_unattach_equipment_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_sacrifice_then_sacrificed_conditional_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_gain_control_create_token_attach_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_create_token_then_grant_same_tag(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_moved_object_haste_delayed_cleanup(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_pump_all_then_grant_same_filter(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_put_counters_then_grant_same_filter(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_draw_count_then_grant_same_filter(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_continuous_choose_attach_sequence(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_return_each_subtype_card_from_your_graveyard(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_random_choose_then_destroy_rest(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_search_two_split_hand_graveyard_sequence(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_choose_two_filters_then_discard(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_discard_reveal_hand_choose_discard_chosen(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_color_reveal_hand_discard_that_color(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_target_player_choose_hand_top_library_any_order(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_hand_choose_then_library_placement(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_then_gain_for_that_players_hand(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_choose_graveyard_or_hand_exile(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_choose_discard_then_scry(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_choose_discard_then_adventure_move(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_hand_choose_gain_toughness_then_discard(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_look_hand_choose_then_discard_or_exile(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_player_protection_from_everything_pair(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_color_then_chosen_color_mana(&effect_refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_power_damage_exchange_clause(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_tagged_for_each_then_apply_continuous(&effect_refs) {
        return Some(compact);
    }
    if effects.len() >= 4
        && let Some(look_at_top) = effects[0].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = effects[1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(&effects[2])
        && let Some((_, rest)) = for_each_tagged_for_compaction(&effects[3])
        && let Some(compact) = describe_look_at_top_then_put_into_hand_rest_graveyard(
            look_at_top,
            None,
            choose,
            None,
            move_to_hand,
            rest,
        )
    {
        if effects.len() == 4 {
            return Some(compact);
        }
        let rest = describe_effect_clause_list(&effects[4..])
            .unwrap_or_else(|| describe_effect_list(&effects[4..]));
        return Some(format!("{compact}. {}", capitalize_first(&rest)));
    }
    if effects.len() >= 4
        && let Some(look_at_top) = effects[0].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = effects[1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, move_chosen)) = for_each_tagged_for_compaction(&effects[2])
        && let Some((_, rest)) = for_each_tagged_for_compaction(&effects[3])
        && let Some(compact) = describe_look_at_top_then_put_matching_to_zone_rest_hand(
            look_at_top,
            None,
            choose,
            move_chosen,
            rest,
        )
    {
        if effects.len() == 4 {
            return Some(compact);
        }
        let rest = describe_effect_clause_list(&effects[4..])
            .unwrap_or_else(|| describe_effect_list(&effects[4..]));
        return Some(format!("{compact}. {}", capitalize_first(&rest)));
    }
    if let Some(compact) = describe_choose_two_move_one_put_counters_on_other(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_same_controller_sacrifice_one_return_other(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_exiled_cards_exile_library_put_chosen_on_top(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_two_sacrifice_one_return_other(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_sacrifice_power_damage_each(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_return_from_graveyard_with_counters(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_move_to_battlefield_with_additional_counters(effects) {
        return Some(compact);
    }
    if let [destroy_effect, search_effect, shuffle_effect] = effects
        && let Some(compact) =
            describe_destroy_then_search_target_opponent_to_graveyard_then_shuffle(
                destroy_effect,
                search_effect,
                shuffle_effect,
            )
    {
        return Some(compact);
    }

    if effects.len() >= 2
        && let Some(first) = describe_target_then_look_at_tagged_object(&effect_refs[..2])
    {
        if effects.len() == 2 {
            return Some(first);
        }
        let rest = describe_effect_clause_list(&effects[2..])
            .unwrap_or_else(|| lowercase_first(&describe_effect_list(&effects[2..])));
        return Some(format!("{first}. {rest}"));
    }

    if effects.len() >= 3
        && let Some(hand) = effects[0].downcast_ref::<crate::effects::LookAtHandEffect>()
        && !hand.reveal
        && hand.target == ChooseSpec::target_player()
        && let Some(top) = effects[1].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && !top.reveal
        && top.count == Value::Fixed(1)
        && top.player == PlayerFilter::target_player()
        && let Some(objects) = effects[2].downcast_ref::<crate::effects::LookAtObjectsEffect>()
        && objects.viewer == PlayerFilter::You
        && objects.subject == PlayerFilter::target_player()
        && objects.filter
            == ObjectFilter::creature()
                .face_down()
                .controlled_by(PlayerFilter::target_player())
    {
        let first = "look at target player's hand, the top card of that player's library, and any face-down creatures they control";
        if effects.len() == 3 {
            return Some(first.to_string());
        }
        let rest = describe_effect_clause_list(&effects[3..])
            .unwrap_or_else(|| lowercase_first(&describe_effect_list(&effects[3..])));
        return Some(format!("{first}. {rest}"));
    }

    if let Some(compact) = describe_structural_multisentence_effect_list(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_leading_selection_then_draw_sequence(effects) {
        return Some(compact);
    }

    // "you and that player each gain that much life" — joint-subject life
    // gain pair (see the matching compaction in describe_effect_list).
    if let [first, second] = effects
        && let Some(first_gain) =
            unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::GainLifeEffect>()
        && let Some(second_gain) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::GainLifeEffect>()
        && first_gain.amount == second_gain.amount
        && matches!(&first_gain.player, ChooseSpec::Player(PlayerFilter::You))
        && let ChooseSpec::Player(second_player) = &second_gain.player
        && *second_player != PlayerFilter::You
    {
        let other = match second_player {
            PlayerFilter::DamagedPlayer | PlayerFilter::TaggedPlayer(_) => {
                "that player".to_string()
            }
            other => describe_player_filter(other),
        };
        return Some(format!(
            "you and {other} each gain {}",
            describe_life_amount_phrase(&first_gain.amount)
        ));
    }

    let compact = describe_effect_list(effects);
    let compact_trimmed = compact.trim();
    if compact_trimmed.starts_with("Exile the bottom card of ")
        && compact_trimmed.contains("For as long as those cards remain exiled")
    {
        return Some(cleanup_decompiled_text(&lowercase_first(
            compact_trimmed.trim_end_matches('.'),
        )));
    }
    if compact_trimmed
        == "Reveal the top card of your library and put that card into your hand. You lose life equal to that card's mana value"
    {
        return Some(cleanup_decompiled_text(&lowercase_first(compact_trimmed)));
    }
    if clause_effects_have_typed_sentence_boundaries(&visible_refs) {
        return Some(cleanup_decompiled_text(&lowercase_first(
            compact_trimmed.trim_end_matches('.'),
        )));
    }
    if !compact_trimmed.is_empty()
        && !compact_trimmed.contains(". ")
        && !compact_trimmed.contains(": ")
        && !compact_trimmed.starts_with("If ")
        && !compact_trimmed.starts_with("When ")
        && !compact_trimmed.starts_with("Whenever ")
        && !compact_trimmed.starts_with("At ")
        && !compact_trimmed.starts_with("Choose ")
    {
        let normalized = normalize_imperative_you_clause(compact_trimmed.trim_end_matches('.'));
        return Some(cleanup_decompiled_text(&lowercase_first(&normalized)));
    }
    if !compact_trimmed.is_empty()
        && compact_trimmed.contains(". That ")
        && compact_trimmed.contains(" in addition to its other colors and types")
        && !compact_trimmed.starts_with("If ")
        && !compact_trimmed.starts_with("When ")
        && !compact_trimmed.starts_with("Whenever ")
        && !compact_trimmed.starts_with("At ")
    {
        return Some(cleanup_decompiled_text(
            compact_trimmed.trim_end_matches('.'),
        ));
    }
    if !compact_trimmed.is_empty()
        && compact_trimmed.contains(" until ")
        && compact_trimmed.contains(". Put ")
        && compact_trimmed.contains(" and the rest on the bottom of ")
        && !compact_trimmed.starts_with("If ")
        && !compact_trimmed.starts_with("When ")
        && !compact_trimmed.starts_with("Whenever ")
        && !compact_trimmed.starts_with("At ")
    {
        return Some(cleanup_decompiled_text(&lowercase_first(
            compact_trimmed.trim_end_matches('.'),
        )));
    }

    if let Some(compact) = describe_roll_die_then_scry_result(effects) {
        return Some(compact);
    }

    // Per-effect rendering that surfaces internal tag scaffolding is never
    // oracle-faithful; when the compaction-aware multi-sentence render in
    // describe_effect_list avoided that scaffolding, bail so callers use it.
    let compact_has_scaffolding =
        compact_trimmed.contains("tagged cards") || compact_trimmed.contains("tagged '");
    let mut parts = Vec::with_capacity(effects.len());
    let mut effect_idx = 0usize;
    while effect_idx < effects.len() {
        let effect = &effects[effect_idx];
        if effect_idx + 1 < effects.len()
            && let Some(joint) =
                describe_choose_then_return_from_graveyard(effect, &effects[effect_idx + 1])
        {
            let joint = joint
                .strip_prefix("you ")
                .map(normalize_you_verb_phrase)
                .unwrap_or(joint);
            parts.push(lowercase_first(joint.trim_end_matches('.')));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(joint) =
                describe_action_and_get_energy_pair(effect, &effects[effect_idx + 1])
        {
            parts.push(lowercase_first(&joint));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(joint) =
                describe_same_actor_gain_then_draw(effect, &effects[effect_idx + 1])
        {
            parts.push(lowercase_first(&joint));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(joint) =
                describe_same_actor_draw_then_gain(effect, &effects[effect_idx + 1])
        {
            parts.push(lowercase_first(&joint));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(joint) = describe_joint_subject_pair(effect, &effects[effect_idx + 1])
        {
            parts.push(lowercase_first(&joint));
            effect_idx += 2;
            continue;
        }
        if effect_idx + 1 < effects.len()
            && let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(sacrifice) = sacrifice_view(&effects[effect_idx + 1])
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice)
        {
            let compact = compact
                .strip_prefix("you ")
                .map(normalize_you_verb_phrase)
                .unwrap_or(compact);
            parts.push(lowercase_first(compact.trim_end_matches('.')));
            effect_idx += 2;
            continue;
        }
        let remaining = effects[effect_idx..].iter().collect::<Vec<_>>();
        if let Some((joint, consumed)) =
            describe_longest_conjoined_counter_or_draw_sequence(&remaining)
        {
            parts.push(lowercase_first(&joint));
            effect_idx += consumed;
            continue;
        }
        let rendered = describe_effect(effect);
        let trimmed = rendered.trim();
        if trimmed.is_empty()
            || trimmed.contains(". ")
            || trimmed.contains(": ")
            || trimmed.starts_with("If ")
            || trimmed.starts_with("When ")
            || trimmed.starts_with("Whenever ")
            || trimmed.starts_with("At ")
            || trimmed.starts_with("Choose ")
            || (!compact_has_scaffolding
                && (trimmed.contains("tagged cards") || trimmed.contains("tagged '")))
        {
            return None;
        }
        let normalized = normalize_imperative_you_clause(trimmed.trim_end_matches('.'));
        parts.push(lowercase_first(&normalized));
        effect_idx += 1;
    }

    let last = parts.pop()?;
    let body = if parts.is_empty() {
        last
    } else {
        format!("{}, then {last}", parts.join(", "))
    };
    Some(cleanup_decompiled_text(&body))
}

fn describe_exile_two_creatures_then_controller_consults(effects: &[Effect]) -> Option<String> {
    let [exile_effect, iteration_effect] = effects else {
        return None;
    };
    let exiled_tag =
        tagged_exile_exact_target_type(exile_effect, crate::types::CardType::Creature, 2)?;
    let for_each = iteration_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if for_each.tag.as_str() != exiled_tag && for_each.tag.as_str() != crate::tag::SOURCE_EXILED_TAG
    {
        return None;
    }
    if !matches!(
        consult_reveal_put_battlefield_then_shuffle_selection(for_each).as_deref(),
        Some("creature" | "creature card")
    ) {
        return None;
    }

    Some("Exile two target creatures. For each of those creatures, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then shuffles the rest into their library".to_string())
}

fn describe_exile_top_then_search_to_hand_and_shuffle(effects: &[Effect]) -> Option<String> {
    let [exile_effect, search_effect, move_effect, shuffle_effect] = effects else {
        return None;
    };
    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if exile.player != PlayerFilter::You {
        return None;
    }
    let search = structural_unwrap_render_wrappers(search_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !search.is_search
        || search.chooser != PlayerFilter::You
        || search.zone != Some(Zone::Library)
        || search.count.min != 1
        || search.count.max != Some(1)
    {
        return None;
    }
    let move_to_hand = downcast_search_split_move_to_zone(move_effect)?;
    if !search_split_move_to_zone_uses_tag(move_to_hand, search.tag.as_str(), Zone::Hand) {
        return None;
    }
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle.player != PlayerFilter::You || shuffle.target_spec.is_some() {
        return None;
    }

    let exile_text = describe_effect(exile_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let search_text = lowercase_first(describe_effect(search_effect).trim().trim_end_matches('.'));
    Some(format!(
        "{exile_text}, then {search_text}. Put that card into your hand, then shuffle"
    ))
}

fn describe_two_target_players_each_search_to_top(effects: &[Effect]) -> Option<String> {
    let [target_effect, per_player_effect] = effects else {
        return None;
    };
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::WithCount(target, count) = &target_only.target else {
        return None;
    };
    if count.min != 2 || count.max != Some(2) {
        return None;
    }
    let ChooseSpec::Target(target) = target.as_ref() else {
        return None;
    };
    if !matches!(target.as_ref(), ChooseSpec::Player(PlayerFilter::Any)) {
        return None;
    }

    let for_players = structural_unwrap_render_wrappers(per_player_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::target_player() || for_players.effects.len() != 1 {
        return None;
    }
    let search = structural_unwrap_render_wrappers(&for_players.effects[0])
        .downcast_ref::<crate::effects::SearchLibraryEffect>()?;
    if search.destination != Zone::Library
        || search.chooser != PlayerFilter::IteratedPlayer
        || search.player != PlayerFilter::IteratedPlayer
        || search.library_position_from_top != Some(Value::Fixed(1))
    {
        return None;
    }
    let rendered_search = describe_effect(&for_players.effects[0]);
    let action = rendered_search
        .trim()
        .trim_end_matches('.')
        .strip_prefix("That player ")
        .or_else(|| {
            rendered_search
                .trim()
                .trim_end_matches('.')
                .strip_prefix("that player ")
        })?;
    Some(format!("Choose two target players. Each of them {action}"))
}

fn describe_search_reveal_nested_may_move_else_hand(effects: &[Effect]) -> Option<String> {
    let [
        search_effect,
        reveal_effect,
        conditional_effect,
        shuffle_effect,
    ] = effects
    else {
        return None;
    };
    let search = structural_unwrap_render_wrappers(search_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !search.is_search
        || search.zone != Some(Zone::Library)
        || search.count.min != 1
        || search.count.max != Some(1)
    {
        return None;
    }
    let reveal = structural_unwrap_render_wrappers(reveal_effect)
        .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if reveal.tag != search.tag {
        return None;
    }
    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let [with_id_effect, declined_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = structural_unwrap_render_wrappers(&with_id.effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::You)
    {
        return None;
    }
    let [battlefield_effect] = may.effects.as_slice() else {
        return None;
    };
    let battlefield_move = move_to_zone_from_effect(battlefield_effect)?;
    if battlefield_move.zone != Zone::Battlefield
        || !matches!(battlefield_move.target.base(), ChooseSpec::Tagged(tag) if tag == &search.tag)
    {
        return None;
    }
    let declined = structural_unwrap_render_wrappers(declined_effect)
        .downcast_ref::<crate::effects::IfEffect>()?;
    if declined.condition != with_id.id
        || declined.predicate != crate::effect::EffectPredicate::DidNotHappen
        || !declined.else_.is_empty()
    {
        return None;
    }
    let [declined_hand_effect] = declined.then.as_slice() else {
        return None;
    };
    let declined_hand = move_to_zone_from_effect(declined_hand_effect)?;
    let [otherwise_hand_effect] = conditional.if_false.as_slice() else {
        return None;
    };
    let otherwise_hand = move_to_zone_from_effect(otherwise_hand_effect)?;
    let is_searched_card_to_hand = |move_to_zone: &crate::effects::MoveToZoneEffect| {
        move_to_zone.zone == Zone::Hand
            && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &search.tag)
    };
    if !is_searched_card_to_hand(declined_hand) || !is_searched_card_to_hand(otherwise_hand) {
        return None;
    }
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle.player != PlayerFilter::You || shuffle.target_spec.is_some() {
        return None;
    }

    let rendered_search = describe_effect(search_effect);
    let search_text = rendered_search
        .trim()
        .trim_end_matches('.')
        .strip_prefix("You ")
        .or_else(|| {
            rendered_search
                .trim()
                .trim_end_matches('.')
                .strip_prefix("you ")
        })?
        .to_string();
    let condition = describe_condition(&conditional.condition);
    let tapped = if battlefield_move.enters_tapped {
        " tapped"
    } else {
        ""
    };
    Some(format!(
        "{}. You may put that card onto the battlefield{tapped} if {condition}. Otherwise, put that card into your hand. Then shuffle",
        capitalize_first(&format!("{search_text} and reveal it"))
    ))
}
