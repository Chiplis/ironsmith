use super::*;

fn describe_each_player_mill_then_reanimate_as_artifact(effects: &[Effect]) -> Option<String> {
    let [mill_players_effect, move_effect, type_effect] = effects else {
        return None;
    };
    let mill_players = mill_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if mill_players.filter != PlayerFilter::Any
        || mill_players.starting_with_controller
        || mill_players.stop_after_first_happened
    {
        return None;
    }
    let [mill_effect] = mill_players.effects.as_slice() else {
        return None;
    };
    let mill = structural_unwrap_render_wrappers(mill_effect)
        .downcast_ref::<crate::effects::MillEffect>()?;
    if mill.player != PlayerFilter::IteratedPlayer || mill.count != Value::Fixed(2) {
        return None;
    }

    let move_tag = direct_wrapped_effect_tag(move_effect)?;
    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::WithCount(target, count) = &move_to_zone.target else {
        return None;
    };
    let ChooseSpec::Object(filter) = target.as_ref() else {
        return None;
    };
    if count.min != 1
        || count.max != Some(1)
        || filter.zone != Some(Zone::Graveyard)
        || filter.owner.is_some()
        || filter.card_types.as_slice() != [CardType::Creature]
        || move_to_zone.zone != Zone::Battlefield
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::You
    {
        return None;
    }

    let apply = structural_unwrap_render_wrappers(type_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.until != Until::Forever
        || apply.condition.is_some()
        || !apply.additional_modifications.is_empty()
        || !apply.runtime_modifications.is_empty()
        || !matches!(
            apply.target_spec.as_ref().map(ChooseSpec::base),
            Some(ChooseSpec::Tagged(tag)) if tag == move_tag
        )
        || !matches!(
            &apply.modification,
            Some(crate::continuous::Modification::AddCardTypes(types))
                if types.as_slice() == [CardType::Artifact]
        )
    {
        return None;
    }

    Some(
        "Each player mills two cards. Then you put a creature card from a graveyard onto the battlefield under your control. It's an artifact in addition to its other types"
            .to_string(),
    )
}

fn player_is_immediately_chosen_opponent(
    player: &PlayerFilter,
    choose: &crate::effects::ChoosePlayerEffect,
) -> bool {
    matches!(
        player,
        PlayerFilter::TaggedPlayer(tag)
            if tag == &choose.tag || tag.as_str() == "__it__"
    )
}

fn tag_matching_untap_filter<'a>(
    tag_effect: &'a Effect,
    untap_effect: &Effect,
) -> Option<&'a ObjectFilter> {
    let tagged = tag_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;
    let same_set = match untap.target.base() {
        ChooseSpec::All(filter) | ChooseSpec::Object(filter) => filter == &tagged.filter,
        ChooseSpec::Tagged(tag) => tag == &tagged.tag,
        _ => choose_spec_references_tagged_filter_recursive(&untap.target, &tagged.tag),
    };
    (same_set
        && tagged.zone.is_none()
        && tagged.additional_zones.is_empty()
        && tagged.filter.zone == Some(Zone::Battlefield))
    .then_some(&tagged.filter)
}

fn describe_choose_opponent_joint_nonland_untap(effects: &[Effect]) -> Option<String> {
    let [choose_effect, sequence_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.filter != PlayerFilter::Opponent
        || choose.random
    {
        return None;
    }
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let [your_tag, your_untap, their_tag, their_untap] = sequence.effects.as_slice() else {
        return None;
    };
    let your_filter = tag_matching_untap_filter(your_tag, your_untap)?;
    let their_filter = tag_matching_untap_filter(their_tag, their_untap)?;
    if your_filter.controller != Some(PlayerFilter::You)
        || !their_filter
            .controller
            .as_ref()
            .is_some_and(|player| player_is_immediately_chosen_opponent(player, choose))
        || !is_nonland_permanent_filter(your_filter)
        || !is_nonland_permanent_filter(their_filter)
    {
        return None;
    }
    let mut your_base = your_filter.clone();
    let mut their_base = their_filter.clone();
    your_base.controller = None;
    their_base.controller = None;
    if your_base != their_base {
        return None;
    }
    Some("Choose an opponent. Untap all nonland permanents you control and all nonland permanents that player controls".to_string())
}

fn normalize_correlated_action_base(text: &str) -> Option<String> {
    let text = text.trim().trim_end_matches('.');
    if text.is_empty() || text.contains(". ") {
        return None;
    }
    let text = text
        .strip_prefix("that player ")
        .or_else(|| text.strip_prefix("That player "))
        .or_else(|| text.strip_prefix("you "))
        .or_else(|| text.strip_prefix("You "))
        .unwrap_or(text);
    let normalized = normalize_you_verb_phrase(text);
    let normalized = [
        ("creates ", "create "),
        ("copies ", "copy "),
        ("exiles ", "exile "),
        ("destroys ", "destroy "),
        ("taps ", "tap "),
        ("untaps ", "untap "),
        ("fights ", "fight "),
    ]
    .into_iter()
    .find_map(|(from, to)| {
        normalized
            .strip_prefix(from)
            .map(|rest| format!("{to}{rest}"))
    })
    .unwrap_or(normalized);
    Some(lowercase_first(&normalized))
}

fn join_correlated_clauses(mut clauses: Vec<String>) -> Option<String> {
    match clauses.as_mut_slice() {
        [] => None,
        [only] => Some(std::mem::take(only)),
        [first, second] => Some(format!("{first} and {second}")),
        _ => {
            let last = clauses.pop()?;
            Some(format!("{}, and {last}", clauses.join(", ")))
        }
    }
}

fn describe_correlated_player_action(filter: &PlayerFilter, effects: &[Effect]) -> Option<String> {
    if let Some(action) = describe_for_players_may_action(filter, effects)
        && let Some(normalized) = normalize_correlated_action_base(&action)
    {
        return Some(normalized);
    }

    let clauses = effects
        .iter()
        .map(|effect| normalize_correlated_action_base(&describe_effect(effect)))
        .collect::<Option<Vec<_>>>()?;
    join_correlated_clauses(clauses)
}

fn correlated_followup_is_controller_action(effect: &Effect) -> bool {
    let effect = structural_unwrap_render_wrappers(effect);
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
        return draw.player == PlayerFilter::You;
    }
    if let Some(gain) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
        return gain.player == ChooseSpec::Player(PlayerFilter::You);
    }
    if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
        return lose.player == ChooseSpec::Player(PlayerFilter::You);
    }
    if let Some(pay) = effect.downcast_ref::<crate::effects::PayLifeEffect>() {
        return pay.player == ChooseSpec::Player(PlayerFilter::You);
    }
    if let Some(create) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
        return create.controller == PlayerFilter::You;
    }
    if let Some(create) = effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>() {
        return create.controller == PlayerFilter::You;
    }
    false
}

pub(in crate::compiled_text) fn describe_distinct_power_choice_destroy_complement(
    effects: &[Effect],
) -> Option<String> {
    let [repeat_effect, destroy_effect] = effects else {
        return None;
    };
    let repeat = repeat_effect.downcast_ref::<crate::effects::RepeatEffectsEffect>()?;
    let Value::DistinctPowers(power_filter) = repeat.count.unhinted() else {
        return None;
    };
    let [choice_effect] = repeat.effects.as_slice() else {
        return None;
    };
    let choice = choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choice.count.is_single() || &choice.filter != power_filter {
        return None;
    }

    let destroy = structural_unwrap_render_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(destroy_filter) = destroy.spec.base() else {
        return None;
    };
    if destroy_filter.controller.is_some() || destroy_filter.owner.is_some() {
        return None;
    }
    let [constraint] = destroy_filter.tagged_constraints.as_slice() else {
        return None;
    };
    if constraint.tag != choice.tag
        || constraint.relation != crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
    {
        return None;
    }
    let mut destroy_base = destroy_filter.clone();
    destroy_base.tagged_constraints.clear();
    if &destroy_base != power_filter {
        return None;
    }

    let among = describe_count_filter_value_subject(power_filter);
    let mut item_filter = power_filter.clone();
    item_filter.zone = None;
    let item = strip_indefinite_article(&item_filter.description())
        .trim()
        .to_string();
    Some(format!(
        "For each different power among {among}, choose {} with that power. Destroy each {item} not chosen this way",
        with_indefinite_article(&item)
    ))
}

/// Render a direct choice followed by a quantified participant choice and a
/// destroy instruction consuming the complement of their shared accumulated
/// tag. The tag equality proves that both producer groups contribute to the
/// same chosen set; filter equality is checked after removing only the
/// chooser-relative controller and that exact tag constraint.
pub(in crate::compiled_text) fn describe_direct_then_players_choose_destroy_complement(
    effects: &[Effect],
) -> Option<String> {
    let [direct_effect, participant_effect, destroy_effect] = effects else {
        return None;
    };
    let direct = structural_unwrap_render_wrappers(direct_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let participants = structural_unwrap_render_wrappers(participant_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let [participant_effect] = participants.effects.as_slice() else {
        return None;
    };
    let participant = structural_unwrap_render_wrappers(participant_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let destroy = structural_unwrap_render_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;

    if participants.starting_with_controller
        || participants.stop_after_first_happened
        || participants.filter != PlayerFilter::NotYou
        || direct.chooser != PlayerFilter::You
        || participant.chooser != PlayerFilter::IteratedPlayer
        || !direct.count.is_single()
        || !participant.count.is_single()
        || direct.count_value.is_some()
        || participant.count_value.is_some()
        || direct.aggregate_constraint.is_some()
        || participant.aggregate_constraint.is_some()
        || direct.is_search
        || participant.is_search
        || direct.tag != participant.tag
        || direct.filter.controller != Some(PlayerFilter::NotYou)
        || participant.filter.controller
            != Some(PlayerFilter::excluding(
                PlayerFilter::Any,
                PlayerFilter::IteratedPlayer,
            ))
        || direct.filter.owner.is_some()
        || participant.filter.owner.is_some()
        || !direct.filter.tagged_constraints.is_empty()
    {
        return None;
    }

    let ChooseSpec::All(destroy_filter) = destroy.spec.base() else {
        return None;
    };
    if destroy_filter.controller.is_some() || destroy_filter.owner.is_some() {
        return None;
    }
    let shared_tag = &direct.tag;
    let [participant_constraint] = participant.filter.tagged_constraints.as_slice() else {
        return None;
    };
    let [destroy_constraint] = destroy_filter.tagged_constraints.as_slice() else {
        return None;
    };
    if [participant_constraint, destroy_constraint]
        .iter()
        .any(|constraint| {
            constraint.tag != *shared_tag
                || constraint.relation != crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
        })
    {
        return None;
    }

    let normalize_kind = |filter: &ObjectFilter| {
        let mut kind = filter.clone();
        kind.zone = None;
        kind.controller = None;
        kind.owner = None;
        kind.other = false;
        kind.tagged_constraints
            .retain(|constraint| constraint.tag != *shared_tag);
        kind
    };
    let direct_kind = normalize_kind(&direct.filter);
    let participant_kind = normalize_kind(&participant.filter);
    let destroy_kind = normalize_kind(destroy_filter);
    if direct_kind != participant_kind || direct_kind != destroy_kind {
        return None;
    }

    let participant_subject = lowercase_first(describe_for_players_subject(&participants.filter)?);
    let participant_item =
        with_indefinite_article(strip_indefinite_article(&participant_kind.description()));
    let destroyed = pluralize_noun_phrase(strip_leading_article(&destroy_kind.description()));
    Some(format!(
        "Choose {}, then {participant_subject} chooses {participant_item} they don't control that hasn't been chosen this way. Destroy all other {destroyed}",
        describe_choose_selection(direct),
    ))
}

/// Render a player-partitioned producer, a per-player repeat driven by that
/// producer's typed result, and a final consumer of the complement of the
/// accumulated chosen set. Every prose relationship here is backed by an
/// effect id, player partition, action metric, or shared tag.
pub(in crate::compiled_text) fn describe_partitioned_tap_choice_destroy_complement(
    effects: &[Effect],
) -> Option<String> {
    let [producer_effect, participant_effect, destroy_effect] = effects else {
        return None;
    };

    let producer = producer_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let producer_players = producer
        .effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let [may_effect] = producer_players.effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [tap_effect] = may.effects.as_slice() else {
        return None;
    };
    let tap = structural_unwrap_render_wrappers(tap_effect)
        .downcast_ref::<crate::effects::TapEffect>()?;
    let ChooseSpec::WithCount(tap_target, tap_count) = &tap.target else {
        return None;
    };
    let ChooseSpec::Object(tap_filter) = tap_target.as_ref() else {
        return None;
    };
    if producer_players.filter != PlayerFilter::Any
        || producer_players.starting_with_controller
        || producer_players.stop_after_first_happened
        || may.decider != Some(PlayerFilter::IteratedPlayer)
        || may.fallback != crate::decision::FallbackStrategy::Decline
        || !tap_count.is_any_number()
        || tap_filter.zone != Some(Zone::Battlefield)
        || tap_filter.controller != Some(PlayerFilter::IteratedPlayer)
        || !tap_filter.untapped
    {
        return None;
    }

    let participant_players = structural_unwrap_render_wrappers(participant_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let [repeat_effect] = participant_players.effects.as_slice() else {
        return None;
    };
    let repeat = structural_unwrap_render_wrappers(repeat_effect)
        .downcast_ref::<crate::effects::RepeatEffectsEffect>()?;
    let Value::PriorEffectMetric { effect_id, query } = repeat.count.unhinted() else {
        return None;
    };
    let [choose_effect] = repeat.effects.as_slice() else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if participant_players.filter != producer_players.filter
        || participant_players.starting_with_controller
        || participant_players.stop_after_first_happened
        || !repeat.count.has_surface_hint(ValueSurfaceHint::ForEach)
        || *effect_id != producer.id
        || query.source != crate::effect::EffectMetricSource::AffectedObjects
        || query.metric != crate::effect::EffectMetric::Count
        || query.player != Some(PlayerFilter::IteratedPlayer)
        || query.action != Some(crate::effect::PriorEffectAction::Tapped)
        || query.filter.is_none()
        || choose.chooser != PlayerFilter::IteratedPlayer
        || choose.zone != Some(Zone::Battlefield)
        || !choose.additional_zones.is_empty()
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.count.dynamic_x
        || choose.count.random
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
    {
        return None;
    }

    let destroy = structural_unwrap_render_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(destroy_filter) = destroy.spec.base() else {
        return None;
    };
    let [constraint] = destroy_filter.tagged_constraints.as_slice() else {
        return None;
    };
    if constraint.tag != choose.tag
        || constraint.relation != crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
    {
        return None;
    }

    let normalize_choice_kind = |filter: &ObjectFilter, clear_tag: bool| {
        let mut kind = filter.clone();
        kind.zone = None;
        kind.controller = None;
        kind.owner = None;
        if clear_tag {
            kind.tagged_constraints.clear();
        }
        kind
    };
    let choice_kind = normalize_choice_kind(&choose.filter, false);
    let destroy_kind = normalize_choice_kind(destroy_filter, true);
    if choice_kind != destroy_kind {
        return None;
    }

    let mut tappable_kind = tap_filter.clone();
    tappable_kind.zone = None;
    tappable_kind.controller = None;
    tappable_kind.untapped = false;
    let tappable = pluralize_noun_phrase(strip_indefinite_article(&tappable_kind.description()));
    let subject = describe_for_players_subject(&producer_players.filter)?;
    let basis = describe_prior_effect_metric_basis(query, false);
    let selection = describe_choose_selection(choose);
    let destroyed = pluralize_noun_phrase(strip_indefinite_article(&destroy_kind.description()));
    Some(format!(
        "{subject} may tap any number of untapped {tappable} they control. For each {basis}, that player chooses {selection}. Then destroy all {destroyed} that weren't chosen this way by any player"
    ))
}

fn describe_exile_collection_play_any_type_then_exile_source(effects: &[Effect]) -> Option<String> {
    let [exile_effect, permission_effect, source_exile_effect] = effects else {
        return None;
    };
    let tagged_exile = exile_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let exile = tagged_exile
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()?;
    if exile.face_down || !matches!(exile.spec.base(), ChooseSpec::All(_)) {
        return None;
    }

    let for_each = permission_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if for_each.tag != tagged_exile.tag {
        return None;
    }
    let [grant_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if grant.tag.as_str() != "__it__"
        || grant.player != PlayerFilter::You
        || grant.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
        || !grant.allow_land
        || grant.mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::AnyType
        || grant.while_on_top_of_library
        || grant.filter.is_some()
    {
        return None;
    }

    let source_exile = structural_unwrap_render_wrappers(source_exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if source_exile.zone != Zone::Exile || !matches!(source_exile.target.base(), ChooseSpec::Source)
    {
        return None;
    }

    let exile_text = capitalize_first(describe_effect(exile_effect).trim_end_matches('.'));
    let source_exile_text =
        capitalize_first(describe_effect(source_exile_effect).trim_end_matches('.'));
    Some(format!(
        "{exile_text}. For each card exiled this way, you may play that card for as long as it remains exiled, and mana of any type can be spent to cast that spell. {source_exile_text}"
    ))
}

fn describe_correlated_followup(effects: &[Effect]) -> Option<String> {
    let clauses = effects
        .iter()
        .map(|effect| {
            let rendered = describe_effect(effect);
            let rendered = rendered.trim().trim_end_matches('.');
            if rendered.is_empty() || rendered.contains(". ") {
                return None;
            }
            if let Some(rest) = rendered
                .strip_prefix("That player ")
                .or_else(|| rendered.strip_prefix("that player "))
            {
                return Some(format!("that player {rest}"));
            }
            if let Some(rest) = rendered
                .strip_prefix("You ")
                .or_else(|| rendered.strip_prefix("you "))
            {
                return Some(format!("you {rest}"));
            }
            let rendered = lowercase_first(rendered);
            if correlated_followup_is_controller_action(effect) {
                Some(format!("you {rendered}"))
            } else {
                Some(rendered)
            }
        })
        .collect::<Option<Vec<_>>>()?;
    join_correlated_clauses(clauses)
}

fn correlated_third_person_action(action: &str) -> String {
    let normalized = normalize_third_person_verb_phrase(action);
    [
        ("create ", "creates "),
        ("copy ", "copies "),
        ("exile ", "exiles "),
        ("destroy ", "destroys "),
        ("tap ", "taps "),
        ("untap ", "untaps "),
        ("fight ", "fights "),
    ]
    .into_iter()
    .find_map(|(from, to)| {
        normalized
            .strip_prefix(from)
            .map(|rest| format!("{to}{rest}"))
    })
    .unwrap_or(normalized)
}

/// Preserve the per-player result partition represented by an inner
/// `WithId -> If` pair. Rendering the body as an ordinary effect list turns
/// this into one singular "if" and erases how many opponents succeeded or
/// failed their own instruction.
pub(super) fn describe_for_players_correlated_result_loop(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if !matches!(
        for_players.filter,
        PlayerFilter::Opponent | PlayerFilter::Any
    ) || for_players.starting_with_controller
        || for_players.stop_after_first_happened
    {
        return None;
    }
    let (choose, antecedent_effect, conditional_effect) = match for_players.effects.as_slice() {
        [antecedent_effect, conditional_effect] => (None, antecedent_effect, conditional_effect),
        [choose_effect, antecedent_effect, conditional_effect] => (
            Some(choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?),
            antecedent_effect,
            conditional_effect,
        ),
        _ => return None,
    };
    let with_id = antecedent_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if conditional.condition != with_id.id
        || !conditional.else_.is_empty()
        || conditional.then.is_empty()
        || !matches!(
            conditional.predicate,
            EffectPredicate::DidNotHappen | EffectPredicate::Chosen
        )
    {
        return None;
    }

    let (optional, antecedent) = if choose.is_none() {
        if let Some(may) = with_id.effect.downcast_ref::<crate::effects::MayEffect>() {
            if may
                .decider
                .as_ref()
                .is_some_and(|decider| *decider != PlayerFilter::IteratedPlayer)
            {
                return None;
            }
            (true, may.effects.as_slice())
        } else {
            (false, std::slice::from_ref(with_id.effect.as_ref()))
        }
    } else {
        (false, std::slice::from_ref(with_id.effect.as_ref()))
    };

    let action = if let Some(choose) = choose {
        let sacrifice = sacrifice_view(&with_id.effect)?;
        let compact = describe_choose_then_sacrifice(choose, sacrifice)?;
        normalize_correlated_action_base(&compact)?
    } else {
        describe_correlated_player_action(&for_players.filter, antecedent)?
    };
    let followup = if optional && conditional.predicate == EffectPredicate::DidNotHappen {
        describe_for_players_didnt_followup(&conditional.then)
            .filter(|text| !text.contains(". "))
            .map(|text| lowercase_first(&text))
            .or_else(|| describe_correlated_followup(&conditional.then))?
    } else {
        describe_correlated_followup(&conditional.then)?
    };

    let quantified_player = if for_players.filter == PlayerFilter::Opponent {
        "opponent"
    } else {
        "player"
    };
    let first = if optional {
        format!("Each {quantified_player} may {action}")
    } else {
        format!(
            "Each {quantified_player} {}",
            correlated_third_person_action(&action)
        )
    };
    let relative = match (optional, &conditional.predicate) {
        (true, EffectPredicate::DidNotHappen) => "doesn't",
        (false, EffectPredicate::DidNotHappen) => "can't",
        (true, EffectPredicate::Chosen) => "does",
        _ => return None,
    };
    if choose.is_some()
        && let Some(action) = followup
            .strip_prefix("that player ")
            .or_else(|| followup.strip_prefix("That player "))
    {
        return Some(format!(
            "{first}. Each {quantified_player} who {relative} {action}"
        ));
    }
    Some(format!(
        "{first}. For each {quantified_player} who {relative}, {followup}"
    ))
}

/// Render a parser-lowered sacrifice choice together with the branch that is
/// explicitly keyed to that sacrifice's result ID. Treating the three effects
/// independently exposes the internal ID and loses the player who succeeded
/// or failed to sacrifice.
pub(super) fn describe_choose_sacrifice_result_sequence(
    choose: &crate::effects::ChooseObjectsEffect,
    with_id: &crate::effects::WithIdEffect,
    conditional: &crate::effects::IfEffect,
) -> Option<String> {
    if conditional.condition != with_id.id
        || !conditional.else_.is_empty()
        || conditional.then.is_empty()
    {
        return None;
    }
    let sacrifice = sacrifice_view(&with_id.effect)?;
    let setup = describe_choose_then_sacrifice(choose, sacrifice)?;
    let subject = describe_player_filter(sacrifice.player);
    let who = if subject == "you" { "you" } else { "they" };
    let condition = match conditional.predicate {
        EffectPredicate::DidNotHappen => format!("If {who} can't"),
        EffectPredicate::Happened | EffectPredicate::Chosen => {
            if who == "you" {
                "If you do".to_string()
            } else {
                "If they do".to_string()
            }
        }
        _ => return None,
    };
    let followup = describe_correlated_followup(&conditional.then)
        .unwrap_or_else(|| describe_result_branch_effect_list(&conditional.then));
    let mut followup = lowercase_first(followup.trim().trim_end_matches('.'));
    if who == "they"
        && let Some(rest) = followup
            .strip_prefix("that player ")
            .or_else(|| followup.strip_prefix("the defending player "))
    {
        followup = format!("they {}", normalize_you_verb_phrase(rest));
    }
    (!followup.is_empty()).then(|| format!("{setup}. {condition}, {followup}"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BattlefieldGraveyardReturnScope {
    Target,
    All,
}

fn battlefield_graveyard_return_view(
    effect: &Effect,
) -> Option<(BattlefieldGraveyardReturnScope, Zone)> {
    let effect = structural_unwrap_render_wrappers(effect);
    let spec =
        if let Some(return_to_hand) = effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
            if return_to_hand.destination_player_surface.is_some() {
                return None;
            }
            &return_to_hand.spec
        } else if let Some(return_from_graveyard) =
            effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
        {
            if return_from_graveyard.random
                || return_from_graveyard.graveyard_player_surface.is_some()
                || return_from_graveyard.destination_player_surface.is_some()
            {
                return None;
            }
            &return_from_graveyard.target
        } else {
            return None;
        };

    let (scope, filter) = if spec.is_target() {
        let ChooseSpec::Object(filter) = spec.base() else {
            return None;
        };
        (BattlefieldGraveyardReturnScope::Target, filter)
    } else {
        let ChooseSpec::All(filter) = spec.base() else {
            return None;
        };
        (BattlefieldGraveyardReturnScope::All, filter)
    };
    let zone = filter.zone?;
    if !matches!(zone, Zone::Battlefield | Zone::Graveyard) {
        return None;
    }
    let mut normalized = filter.clone();
    normalized.zone = Some(Zone::Battlefield);
    (normalized == ObjectFilter::creature()).then_some((scope, zone))
}

/// Preserve the two explicit domains when one return instruction addresses
/// battlefield creatures and the other addresses creature cards in graveyards.
pub(super) fn describe_battlefield_graveyard_return_pair(effects: &[Effect]) -> Option<String> {
    let [first, second] = effects else {
        return None;
    };
    let (first_scope, first_zone) = battlefield_graveyard_return_view(first)?;
    let (second_scope, second_zone) = battlefield_graveyard_return_view(second)?;
    if first_scope != second_scope
        || first_zone == second_zone
        || !matches!(
            (first_zone, second_zone),
            (Zone::Battlefield, Zone::Graveyard) | (Zone::Graveyard, Zone::Battlefield)
        )
    {
        return None;
    }

    let describe_subject = |zone| match (first_scope, zone) {
        (BattlefieldGraveyardReturnScope::Target, Zone::Battlefield) => {
            "target creature on the battlefield"
        }
        (BattlefieldGraveyardReturnScope::Target, Zone::Graveyard) => {
            "target creature card from a graveyard"
        }
        (BattlefieldGraveyardReturnScope::All, Zone::Battlefield) => {
            "all creatures on the battlefield"
        }
        (BattlefieldGraveyardReturnScope::All, Zone::Graveyard) => {
            "all creature cards in graveyards"
        }
        _ => unreachable!("return view only admits battlefield and graveyard zones"),
    };
    Some(format!(
        "Return {} and {} to their owners' hands",
        describe_subject(first_zone),
        describe_subject(second_zone)
    ))
}

fn simple_creature_planeswalker_mana_limit(filter: &ObjectFilter, zone: Zone) -> Option<i32> {
    if filter.zone != Some(zone)
        || filter.card_types.len() != 2
        || !filter.card_types.contains(&CardType::Creature)
        || !filter.card_types.contains(&CardType::Planeswalker)
    {
        return None;
    }
    let ironsmith_core::FilterComparison::LessThanOrEqual(limit) = filter.mana_value.as_ref()?
    else {
        return None;
    };
    let mut plain = filter.clone();
    plain.zone = None;
    plain.card_types.clear();
    plain.mana_value = None;
    (plain == ObjectFilter::default()).then_some(*limit)
}

fn plain_exile_all_filter(effect: &Effect) -> Option<&ObjectFilter> {
    let exile =
        structural_unwrap_render_wrappers(effect).downcast_ref::<crate::effects::ExileEffect>()?;
    if exile.face_down {
        return None;
    }
    let ChooseSpec::All(filter) = exile.spec.base() else {
        return None;
    };
    Some(filter)
}

/// A battlefield/graveyard exile pair needs both provenance phrases; neither
/// clause can rely on the other domain's default object-filter wording.
pub(super) fn describe_battlefield_graveyard_exile_pair(effects: &[Effect]) -> Option<String> {
    let [first, second] = effects else {
        return None;
    };
    let first = plain_exile_all_filter(first)?;
    let second = plain_exile_all_filter(second)?;
    let (battlefield, graveyard) = match (first.zone, second.zone) {
        (Some(Zone::Battlefield), Some(Zone::Graveyard)) => (first, second),
        (Some(Zone::Graveyard), Some(Zone::Battlefield)) => (second, first),
        _ => return None,
    };
    let battlefield_limit =
        simple_creature_planeswalker_mana_limit(battlefield, Zone::Battlefield)?;
    let graveyard_limit = simple_creature_planeswalker_mana_limit(graveyard, Zone::Graveyard)?;
    if battlefield_limit != graveyard_limit {
        return None;
    }
    Some(format!(
        "Exile all creatures and planeswalkers with mana value {battlefield_limit} or less from the battlefield and all creature and planeswalker cards with mana value {battlefield_limit} or less from all graveyards"
    ))
}

pub(in crate::compiled_text) fn exact_single_target_object_filter(
    spec: &ChooseSpec,
) -> Option<&ObjectFilter> {
    if !spec.is_target() || !spec.count().is_single() {
        return None;
    }
    let ChooseSpec::Object(filter) = spec.base() else {
        return None;
    };
    Some(filter)
}

fn exact_target_creature_filter(spec: &ChooseSpec) -> Option<&ObjectFilter> {
    let filter = exact_single_target_object_filter(spec)?;
    (filter.card_types.as_slice() == [CardType::Creature]
        && filter.zone == Some(Zone::Battlefield)
        && filter.controller.is_none()
        && filter.owner.is_none()
        && filter.any_of.is_empty()
        && filter.tagged_constraints.is_empty())
    .then_some(filter)
}

fn effect_outer_tag_through_damage_source(effect: &Effect) -> Option<&TagKey> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return effect_outer_tag_through_damage_source(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Some(&tagged.tag);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return Some(&tag_all.tag);
    }
    effect
        .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        .and_then(|with_source| effect_outer_tag_through_damage_source(&with_source.effect))
}

fn for_each_with_source_view(
    effect: &Effect,
) -> Option<(Option<&ChooseSpec>, &crate::effects::ForEachObject)> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return for_each_with_source_view(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return for_each_with_source_view(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return for_each_with_source_view(&tag_all.effect);
    }
    if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() {
        let for_each = unwrap_basic_tag_wrappers(&with_source.effect)
            .downcast_ref::<crate::effects::ForEachObject>()?;
        return Some((Some(&with_source.source), for_each));
    }
    effect
        .downcast_ref::<crate::effects::ForEachObject>()
        .map(|for_each| (None, for_each))
}

fn damage_sources_are_same(first: Option<&ChooseSpec>, second: Option<&ChooseSpec>) -> bool {
    match (first, second) {
        (None, None) => true,
        (Some(first), Some(second)) => first.unhinted() == second.unhinted(),
        (None, Some(source)) | (Some(source), None) => {
            matches!(source.base(), ChooseSpec::Source)
        }
    }
}

fn with_linked_mechanic_label(
    relation: crate::filter::TaggedOpbjectRelation,
    text: String,
) -> String {
    if relation == crate::filter::TaggedOpbjectRelation::SharesColorWithTagged {
        format!("Radiance — {text}")
    } else {
        text
    }
}

/// A typed target-creature damage followed by an equal damage fanout whose
/// filter refers back to that exact target. This is the runtime shape shared
/// by radiance damage and same-name damage spells.
pub(super) fn describe_target_creature_damage_fanout_pair(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let target_tag = effect_outer_tag_through_damage_source(first)?;
    let (first_source, first_damage) = damage_with_source_view(first)?;
    let target_filter = exact_target_creature_filter(&first_damage.target)?;

    let (fanout_outer_source, for_each) = for_each_with_source_view(second)?;
    let [fanout_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let (fanout_inner_source, fanout_damage) = damage_with_source_view(fanout_effect)?;
    let fanout_source = compatible_damage_sources(fanout_outer_source, fanout_inner_source)?;
    if !matches!(fanout_damage.target.base(), ChooseSpec::Iterated)
        || fanout_damage.amount != first_damage.amount
        || fanout_damage.source_is_combat != first_damage.source_is_combat
        || fanout_damage.unpreventable != first_damage.unpreventable
        || !damage_sources_are_same(first_source, fanout_source)
    {
        return None;
    }

    let relation = for_each
        .filter
        .tagged_constraints
        .iter()
        .find_map(|constraint| {
            (constraint.tag == *target_tag
                && matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                        | crate::filter::TaggedOpbjectRelation::SharesColorWithTagged
                ))
            .then_some(constraint.relation)
        })?;
    let has_exclusion = for_each.filter.other
        || for_each.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *target_tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
        });
    if !has_exclusion
        || for_each.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag != *target_tag
                || !matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                        | crate::filter::TaggedOpbjectRelation::SharesColorWithTagged
                        | crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                )
        })
    {
        return None;
    }

    let mut fanout_base = for_each.filter.clone();
    fanout_base.other = false;
    fanout_base.tagged_constraints.clear();
    if &fanout_base != target_filter {
        return None;
    }

    let suffix = match relation {
        crate::filter::TaggedOpbjectRelation::SameNameAsTagged => {
            "each other creature with the same name as that creature"
        }
        crate::filter::TaggedOpbjectRelation::SharesColorWithTagged => {
            "each other creature that shares a color with it"
        }
        _ => return None,
    };
    Some(with_linked_mechanic_label(
        relation,
        format!(
            "{} and {suffix}",
            describe_effect(first).trim().trim_end_matches('.')
        ),
    ))
}

fn linked_fanout_relation(
    filter: &ObjectFilter,
    target_tag: &TagKey,
) -> Option<crate::filter::TaggedOpbjectRelation> {
    let mut relations = filter.tagged_constraints.iter().filter_map(|constraint| {
        (constraint.tag == *target_tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                    | crate::filter::TaggedOpbjectRelation::SharesColorWithTagged
            ))
        .then_some(constraint.relation)
    });
    let relation = relations.next()?;
    relations.next().is_none().then_some(relation)
}

fn linked_fanout_subject(
    filter: &ObjectFilter,
    target_filter: &ObjectFilter,
    target_tag: &TagKey,
) -> Option<String> {
    let relation = linked_fanout_relation(filter, target_tag)?;
    let excludes_target = filter.other
        || filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *target_tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
        });
    if !excludes_target {
        return None;
    }
    let mut base = filter.clone();
    base.other = false;
    base.tagged_constraints.clear();
    if &base != target_filter {
        return None;
    }
    let mut subject = describe_choose_spec(&ChooseSpec::All(filter.clone()));
    if relation == crate::filter::TaggedOpbjectRelation::SharesColorWithTagged {
        if let Some(rest) = subject.strip_prefix("all other ") {
            subject = format!("each other {rest}");
        }
    } else {
        subject = subject.replace(
            "with the same name as it",
            &format!(
                "with the same name as that {}",
                same_name_reference_noun(target_filter)
            ),
        );
    }
    Some(subject)
}

/// Two continuous modifications with one target and one linked fanout are a
/// single compound subject, not sequential modifications.
pub(super) fn describe_target_continuous_fanout_pair(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let target_tag = effect_outer_tag(first)?;
    let first_apply = structural_unwrap_render_wrappers(first)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let second_apply = structural_unwrap_render_wrappers(second)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let target_spec = first_apply.target_spec.as_ref()?;
    let target_filter = exact_single_target_object_filter(target_spec)?;
    let crate::continuous::EffectTarget::Filter(fanout_filter) = &second_apply.target else {
        return None;
    };
    if first_apply.modification != second_apply.modification
        || first_apply.additional_modifications != second_apply.additional_modifications
        || first_apply.runtime_modifications != second_apply.runtime_modifications
        || first_apply.until != second_apply.until
        || first_apply.condition != second_apply.condition
        || first_apply.source_type != second_apply.source_type
        || first_apply.source_reference_surface != second_apply.source_reference_surface
        || first_apply.type_retention_surface != second_apply.type_retention_surface
        || first_apply.animation_pt_surface != second_apply.animation_pt_surface
        || first_apply.animation_duration_surface != second_apply.animation_duration_surface
        || first_apply.resolve_set_pt_values_at_resolution
            != second_apply.resolve_set_pt_values_at_resolution
    {
        return None;
    }
    let relation = linked_fanout_relation(fanout_filter, target_tag)?;
    let fanout_subject = linked_fanout_subject(fanout_filter, target_filter, target_tag)?;
    let target_subject = describe_choose_spec(target_spec);
    let rendered = describe_effect(first);
    let (_, action) = rendered.split_once(&target_subject)?;
    let action = action
        .strip_prefix(" gets ")
        .map(|tail| format!("get {tail}"))
        .or_else(|| {
            action
                .strip_prefix(" gains ")
                .map(|tail| format!("gain {tail}"))
        })
        .unwrap_or_else(|| action.trim_start().to_string());
    Some(with_linked_mechanic_label(
        relation,
        format!(
            "{} and {fanout_subject} {action}",
            capitalize_first(&target_subject)
        ),
    ))
}

/// Prevention shields over a target and its linked fanout share the same
/// damage clause; substitute the structurally verified compound subject.
pub(super) fn describe_target_prevention_fanout_pair(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let target_tag = effect_outer_tag(first)?;
    let first_prevent = structural_unwrap_render_wrappers(first)
        .downcast_ref::<crate::effects::PreventDamageEffect>()?;
    let for_each = structural_unwrap_render_wrappers(second)
        .downcast_ref::<crate::effects::ForEachObject>()?;
    let [inner] = for_each.effects.as_slice() else {
        return None;
    };
    let second_prevent = structural_unwrap_render_wrappers(inner)
        .downcast_ref::<crate::effects::PreventDamageEffect>()?;
    let target_filter = exact_single_target_object_filter(&first_prevent.target)?;
    if second_prevent.target != ChooseSpec::Iterated
        || first_prevent.amount != second_prevent.amount
        || first_prevent.duration != second_prevent.duration
        || first_prevent.damage_filter != second_prevent.damage_filter
        || first_prevent.follow_up_effects != second_prevent.follow_up_effects
        || first_prevent.source_of_your_choice != second_prevent.source_of_your_choice
        || first_prevent.protect_you_and_permanents_you_control
            != second_prevent.protect_you_and_permanents_you_control
    {
        return None;
    }
    let relation = linked_fanout_relation(&for_each.filter, target_tag)?;
    let fanout_subject = linked_fanout_subject(&for_each.filter, target_filter, target_tag)?;
    let target_subject = describe_choose_spec(&first_prevent.target);
    Some(with_linked_mechanic_label(
        relation,
        describe_effect(first).replacen(
            &target_subject,
            &format!("{target_subject} and {fanout_subject}"),
            1,
        ),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SameNameFanoutAction {
    Destroy,
    Exile,
    ExileUntilSourceLeaves,
    ReturnToHand,
    ReturnToBattlefield { tapped: bool },
    Untap,
}

fn same_name_fanout_action_prefix(action: SameNameFanoutAction) -> &'static str {
    match action {
        SameNameFanoutAction::Destroy => "Destroy ",
        SameNameFanoutAction::Exile | SameNameFanoutAction::ExileUntilSourceLeaves => "Exile ",
        SameNameFanoutAction::ReturnToHand | SameNameFanoutAction::ReturnToBattlefield { .. } => {
            "Return "
        }
        SameNameFanoutAction::Untap => "Untap ",
    }
}

fn same_name_fanout_target_action_view(
    effect: &Effect,
) -> Option<(SameNameFanoutAction, &ChooseSpec)> {
    let effect = structural_unwrap_render_wrappers(effect);
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        return Some((SameNameFanoutAction::Destroy, &destroy.spec));
    }
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
        return (!exile.face_down).then_some((SameNameFanoutAction::Exile, &exile.spec));
    }
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileUntilEffect>() {
        return (!exile.face_down
            && exile.duration == crate::effects::ExileUntilDuration::SourceLeavesBattlefield
            && exile.return_zone == Zone::Battlefield)
            .then_some((SameNameFanoutAction::ExileUntilSourceLeaves, &exile.spec));
    }
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return (move_to_zone.zone == Zone::Exile && !move_to_zone.enters_face_down)
            .then_some((SameNameFanoutAction::Exile, &move_to_zone.target));
    }
    if let Some(return_to_hand) = effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        return Some((SameNameFanoutAction::ReturnToHand, &return_to_hand.spec));
    }
    if let Some(return_to_hand) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
    {
        return (!return_to_hand.random)
            .then_some((SameNameFanoutAction::ReturnToHand, &return_to_hand.target));
    }
    if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>() {
        return Some((SameNameFanoutAction::Untap, &untap.target));
    }
    effect
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
        .and_then(|return_to_battlefield| {
            return_to_battlefield.as_aura.is_none().then_some((
                SameNameFanoutAction::ReturnToBattlefield {
                    tapped: return_to_battlefield.tapped,
                },
                &return_to_battlefield.target,
            ))
        })
}

fn same_name_fanout_all_filter(spec: &ChooseSpec) -> Option<&ObjectFilter> {
    match spec.base() {
        ChooseSpec::All(filter) => Some(filter),
        _ => None,
    }
}

fn same_name_fanout_all_action_view(
    effect: &Effect,
) -> Option<(SameNameFanoutAction, &ObjectFilter)> {
    let effect = structural_unwrap_render_wrappers(effect);
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        return Some((
            SameNameFanoutAction::Destroy,
            same_name_fanout_all_filter(&destroy.spec)?,
        ));
    }
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
        return (!exile.face_down)
            .then(|| same_name_fanout_all_filter(&exile.spec))
            .flatten()
            .map(|filter| (SameNameFanoutAction::Exile, filter));
    }
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileUntilEffect>() {
        return (!exile.face_down
            && exile.duration == crate::effects::ExileUntilDuration::SourceLeavesBattlefield
            && exile.return_zone == Zone::Battlefield)
            .then(|| same_name_fanout_all_filter(&exile.spec))
            .flatten()
            .map(|filter| (SameNameFanoutAction::ExileUntilSourceLeaves, filter));
    }
    if let Some(return_to_hand) = effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        return Some((
            SameNameFanoutAction::ReturnToHand,
            same_name_fanout_all_filter(&return_to_hand.spec)?,
        ));
    }
    if let Some(return_to_hand) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
    {
        return (!return_to_hand.random)
            .then(|| same_name_fanout_all_filter(&return_to_hand.target))
            .flatten()
            .map(|filter| (SameNameFanoutAction::ReturnToHand, filter));
    }
    if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>() {
        return same_name_fanout_all_filter(&untap.target)
            .map(|filter| (SameNameFanoutAction::Untap, filter));
    }
    effect
        .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
        .and_then(|return_all| {
            (!return_all.face_down
                && return_all.battlefield_controller
                    == crate::effects::BattlefieldController::Owner)
                .then_some((
                    SameNameFanoutAction::ReturnToBattlefield {
                        tapped: return_all.tapped,
                    },
                    &return_all.filter,
                ))
        })
}

fn same_name_reference_noun(filter: &ObjectFilter) -> &'static str {
    if matches!(
        filter.zone,
        Some(
            Zone::Graveyard
                | Zone::Hand
                | Zone::Library
                | Zone::Exile
                | Zone::Command
                | Zone::OutsideGame
        )
    ) {
        return "card";
    }
    match filter.card_types.as_slice() {
        [CardType::Artifact] => "artifact",
        [CardType::Creature] => "creature",
        [CardType::Enchantment] => "enchantment",
        [CardType::Land] => "land",
        [CardType::Planeswalker] => "planeswalker",
        [CardType::Battle] => "battle",
        _ => "permanent",
    }
}

fn shared_same_name_action_suffix<'a>(first: &'a str, second: &str) -> Option<&'a str> {
    for marker in [" until ", " from ", " to "] {
        let Some(index) = first.find(marker) else {
            continue;
        };
        let suffix = &first[index..];
        if second.ends_with(suffix) {
            return Some(suffix);
        }
    }
    None
}

/// A target action followed by the same action over every other object with
/// that target's name. The shared tag is the semantic link; rendering does not
/// infer the antecedent from adjacent prose.
pub(super) fn describe_target_same_name_action_fanout_pair(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let target_tag = effect_outer_tag(first)?;
    let (first_action, first_spec) = same_name_fanout_target_action_view(first)?;
    let (second_action, fanout_filter) = same_name_fanout_all_action_view(second)?;
    if first_action != second_action {
        return None;
    }
    let target_filter = exact_single_target_object_filter(first_spec)?;

    let relation = fanout_filter
        .tagged_constraints
        .iter()
        .filter_map(|constraint| {
            (constraint.tag == *target_tag
                && matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                        | crate::filter::TaggedOpbjectRelation::SharesColorWithTagged
                ))
            .then_some(constraint.relation)
        })
        .collect::<Vec<_>>();
    let tagged_exclusions = fanout_filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.tag == *target_tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
        })
        .count();
    if relation.len() != 1
        || (!fanout_filter.other && tagged_exclusions != 1)
        || tagged_exclusions > 1
        || fanout_filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag != *target_tag
                || !matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                        | crate::filter::TaggedOpbjectRelation::SharesColorWithTagged
                        | crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                        | crate::filter::TaggedOpbjectRelation::SameControllerAsTagged
                )
        })
    {
        return None;
    }

    let relation = relation[0];
    let finish = |text| with_linked_mechanic_label(relation, text);
    let action_prefix = same_name_fanout_action_prefix(first_action);
    let first_text = describe_effect(first)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let second_text = describe_effect(second)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let mut second_tail = second_text.strip_prefix(action_prefix)?.to_string();
    if let Some(rest) = second_tail.strip_prefix("all another ") {
        second_tail = format!("all other {rest}");
    }
    if let Some(rest) = second_tail.strip_prefix("all other card ") {
        second_tail = format!("all other cards {rest}");
    }
    let reference_noun = same_name_reference_noun(target_filter);
    let same_name_phrase = format!("with the same name as that {reference_noun}");
    if relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged {
        second_tail = second_tail.replace("with the same name as it", &same_name_phrase);
        if fanout_filter.zone == Some(Zone::Graveyard)
            && fanout_filter.owner.is_none()
            && let Some(without_zone) = second_tail.strip_suffix(" in all graveyards")
            && let Some((subject, after_name)) = without_zone.split_once(&same_name_phrase)
        {
            second_tail = format!(
                "{} from graveyards {same_name_phrase}{after_name}",
                subject.trim_end()
            );
        }
    } else if let Some(rest) = second_tail.strip_prefix("all other ") {
        second_tail = format!("each other {rest}");
    }

    let has_same_controller = fanout_filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *target_tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::SameControllerAsTagged
    });
    if relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
        && has_same_controller
        && let Some((base, after_name)) = second_tail.split_once(&same_name_phrase)
        && let Some(rest) = after_name.strip_prefix(" controlled by its controller")
    {
        let controller_clause = if target_filter.controller.is_some() {
            "that player controls"
        } else {
            "its controller controls"
        };
        second_tail = format!(
            "{} {controller_clause} {same_name_phrase}{rest}",
            base.trim_end()
        );
    }

    if matches!(
        first_action,
        SameNameFanoutAction::ReturnToBattlefield { .. }
    ) {
        second_tail = second_tail
            .strip_suffix(" under their owners' control")
            .unwrap_or(&second_tail)
            .to_string();
        if target_filter.zone == Some(Zone::Graveyard)
            && fanout_filter.zone == Some(Zone::Graveyard)
            && target_filter.owner == fanout_filter.owner
            && let Some(owner) = target_filter.owner.as_ref()
        {
            let graveyard = format!("{} graveyard", describe_possessive_player_filter(owner));
            let first_marker = format!(" from {graveyard}");
            let second_marker = format!(" in {graveyard}");
            if let Some((first_stem, first_suffix)) = first_text.split_once(&first_marker)
                && let Some((second_stem, second_suffix)) = second_tail.split_once(&second_marker)
                && first_suffix == second_suffix
            {
                return Some(finish(format!(
                    "{first_stem} and {second_stem}{first_marker}{first_suffix}"
                )));
            }
        }
    }

    if let Some(suffix) = shared_same_name_action_suffix(&first_text, &second_tail) {
        let first_stem = first_text.strip_suffix(suffix)?;
        let second_stem = second_tail.strip_suffix(suffix)?;
        return Some(finish(format!("{first_stem} and {second_stem}{suffix}")));
    }
    Some(finish(format!("{first_text} and {second_tail}")))
}

fn filter_has_tag_relation_recursive(
    filter: &ObjectFilter,
    tag: &TagKey,
    relation: crate::filter::TaggedOpbjectRelation,
) -> bool {
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag == *tag && constraint.relation == relation)
        || filter
            .any_of
            .iter()
            .any(|branch| filter_has_tag_relation_recursive(branch, tag, relation))
}

struct LinkedActionFanoutPrefix<'a> {
    action: SameNameFanoutAction,
    target_tag: &'a TagKey,
    target_filter: &'a ObjectFilter,
    fanout_filter: &'a ObjectFilter,
    relation: crate::filter::TaggedOpbjectRelation,
    text: String,
    consumed: usize,
}

fn linked_action_fanout_prefix_view(effects: &[Effect]) -> Option<LinkedActionFanoutPrefix<'_>> {
    let first = effects.first()?;
    let target_tag = effect_outer_tag(first)?;
    let (action, target_spec) = same_name_fanout_target_action_view(first)?;
    let target_filter = exact_single_target_object_filter(target_spec)?;

    let (fanout, fanout_filter, consumed) = if let Some((fanout_action, filter)) =
        effects.get(1).and_then(same_name_fanout_all_action_view)
    {
        (fanout_action, filter, 2)
    } else {
        let capture = effects
            .get(1)?
            .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
        let (fanout_action, filter) = effects.get(2).and_then(same_name_fanout_all_action_view)?;
        if capture.filter != *filter
            || capture.zone.is_some_and(|zone| Some(zone) != filter.zone)
            || !capture.additional_zones.is_empty()
        {
            return None;
        }
        (fanout_action, filter, 3)
    };
    if action != fanout {
        return None;
    }
    let relation = linked_fanout_relation(fanout_filter, target_tag)?;
    let fanout_effect = effects.get(consumed - 1)?;
    let text = describe_target_same_name_action_fanout_pair(first, fanout_effect)?;
    Some(LinkedActionFanoutPrefix {
        action,
        target_tag,
        target_filter,
        fanout_filter,
        relation,
        text,
        consumed,
    })
}

fn linked_group_view<'a>(
    effect: &'a Effect,
    prefix: &LinkedActionFanoutPrefix<'_>,
) -> Option<&'a crate::effects::TagMatchingObjectsEffect> {
    let group = effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    if !group.additional_zones.is_empty()
        || !filter_has_tag_relation_recursive(
            &group.filter,
            prefix.target_tag,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        )
        || !filter_has_tag_relation_recursive(&group.filter, prefix.target_tag, prefix.relation)
    {
        return None;
    }
    Some(group)
}

fn describe_linked_group_continuous_followup(
    effect: &Effect,
    group: &crate::effects::TagMatchingObjectsEffect,
) -> Option<String> {
    let apply = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.target_spec.is_some() {
        return None;
    }
    let crate::continuous::EffectTarget::Filter(filter) = &apply.target else {
        return None;
    };
    if filter.tagged_constraints.len() != 1
        || !filter_has_tag_relation_recursive(
            filter,
            &group.tag,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        )
    {
        return None;
    }
    let mut base = filter.clone();
    base.tagged_constraints.clear();
    let mut group_base = group.filter.clone();
    group_base.tagged_constraints.clear();
    group_base.any_of.clear();
    group_base.other = false;
    if base != group_base {
        return None;
    }

    let rendered = describe_effect(effect);
    let rendered = rendered.trim().trim_end_matches('.');
    let pronoun = if base.card_types.as_slice() == [CardType::Creature] {
        "Those creatures"
    } else if base.zone.is_some_and(|zone| zone != Zone::Battlefield) {
        "Those cards"
    } else {
        "Those permanents"
    };
    for (singular, plural) in [
        (" gets ", " get "),
        (" gains ", " gain "),
        (" becomes ", " become "),
        (" is ", " are "),
    ] {
        if let Some((_, tail)) = rendered.split_once(singular) {
            return Some(format!("{pronoun}{plural}{tail}"));
        }
    }
    None
}

fn describe_linked_same_name_third_group(
    effect: &Effect,
    prefix: &LinkedActionFanoutPrefix<'_>,
    group: &crate::effects::TagMatchingObjectsEffect,
) -> Option<String> {
    if prefix.relation != crate::filter::TaggedOpbjectRelation::SameNameAsTagged {
        return None;
    }
    let (action, filter) = same_name_fanout_all_action_view(effect)?;
    if action != prefix.action
        || filter.other
        || filter.tagged_constraints.len() != 1
        || !filter_has_tag_relation_recursive(
            filter,
            &group.tag,
            crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
        )
    {
        return None;
    }
    let action_prefix = same_name_fanout_action_prefix(action);
    let third = describe_effect(effect);
    let third = third
        .trim()
        .trim_end_matches('.')
        .strip_prefix(action_prefix)?
        .replace("with the same name as it", "with that name")
        .replace("with the same name as that object", "with that name")
        .replace("with the same name as that card", "with that name");
    let (first, second) = prefix.text.split_once(" and all other ")?;
    Some(format!("{first}, all other {second}, and {third}"))
}

fn controller_of_tagged_player(player: &PlayerFilter, tag: &TagKey) -> bool {
    matches!(
        player,
        PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Tagged(found))
            if found == tag
    )
}

fn describe_linked_same_name_hand_graveyard_exile(
    look_effect: &Effect,
    exile_effect: &Effect,
    prefix: &LinkedActionFanoutPrefix<'_>,
    group: &crate::effects::TagMatchingObjectsEffect,
) -> Option<String> {
    if prefix.action != SameNameFanoutAction::Exile
        || prefix.relation != crate::filter::TaggedOpbjectRelation::SameNameAsTagged
    {
        return None;
    }
    let look = structural_unwrap_render_wrappers(look_effect)
        .downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let ChooseSpec::Player(player) = look.target.base() else {
        return None;
    };
    if !look.reveal || !controller_of_tagged_player(player, prefix.target_tag) {
        return None;
    }
    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::All(filter) = exile.spec.base() else {
        return None;
    };
    let owner_matches = filter
        .owner
        .as_ref()
        .is_some_and(|owner| controller_of_tagged_player(owner, prefix.target_tag));
    let zones = filter
        .any_of
        .iter()
        .filter_map(|branch| branch.zone)
        .collect::<Vec<_>>();
    if exile.face_down
        || !owner_matches
        || zones.as_slice() != [Zone::Hand, Zone::Graveyard]
        || filter.tagged_constraints.len() != 1
        || !filter_has_tag_relation_recursive(
            filter,
            &group.tag,
            crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
        )
    {
        return None;
    }

    let primary = if prefix.target_filter.card_types.as_slice() == [CardType::Creature]
        && prefix.target_filter.controller == Some(PlayerFilter::Opponent)
        && let Some(crate::filter::Comparison::LessThanOrEqual(limit)) =
            prefix.target_filter.mana_value.as_ref()
        && prefix
            .fanout_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag == *prefix.target_tag
                    && constraint.relation
                        == crate::filter::TaggedOpbjectRelation::SameControllerAsTagged
            }) {
        format!(
            "Exile target creature an opponent controls with mana value {limit} or less and all other creatures that player controls with the same name as that creature"
        )
    } else {
        prefix.text.clone()
    };
    Some(format!(
        "{primary}. Then that player reveals their hand and exiles all cards with that name from their hand and graveyard"
    ))
}

/// Join a target plus same-name fanout with a second same-name fanout in a
/// different zone. The intermediate collection tag is the proof that the
/// final group has the same name as the original target and its first fanout.
fn describe_target_same_name_action_second_zone(effects: &[Effect]) -> Option<(String, usize)> {
    let prefix = linked_action_fanout_prefix_view(effects)?;
    if prefix.relation != crate::filter::TaggedOpbjectRelation::SameNameAsTagged {
        return None;
    }
    let group = linked_group_view(effects.get(prefix.consumed)?, &prefix)?;
    let second_idx = prefix.consumed + 1;
    let second_fanout = effects.get(second_idx)?;
    let (second_action, second_filter) = same_name_fanout_all_action_view(second_fanout)?;
    if second_action != prefix.action
        || prefix.target_filter.zone != prefix.fanout_filter.zone
        || group.zone != prefix.fanout_filter.zone
        || second_filter.zone.is_none()
        || second_filter.zone == group.zone
        || second_filter.other
        || second_filter.tagged_constraints.len() != 1
        || !filter_has_tag_relation_recursive(
            second_filter,
            &group.tag,
            crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
        )
    {
        return None;
    }
    describe_linked_same_name_third_group(second_fanout, &prefix, group)
        .map(|text| (text, second_idx + 1))
}

pub(super) fn describe_linked_target_set_followup_prefix(
    effects: &[Effect],
) -> Option<(String, usize)> {
    if let Some(compact) = describe_target_same_name_action_second_zone(effects) {
        return Some(compact);
    }
    let prefix = linked_action_fanout_prefix_view(effects)?;
    let group = linked_group_view(effects.get(prefix.consumed)?, &prefix)?;
    let followup_idx = prefix.consumed + 1;

    if let Some(text) = effects
        .get(followup_idx)
        .and_then(|effect| describe_linked_group_continuous_followup(effect, group))
    {
        return Some((format!("{}. {text}", prefix.text), followup_idx + 1));
    }
    if let Some(text) = effects
        .get(followup_idx)
        .and_then(|effect| describe_linked_same_name_third_group(effect, &prefix, group))
    {
        return Some((text, followup_idx + 1));
    }
    let look = effects.get(followup_idx)?;
    let exile = effects.get(followup_idx + 1)?;
    describe_linked_same_name_hand_graveyard_exile(look, exile, &prefix, group)
        .map(|text| (text, followup_idx + 2))
}

pub(super) fn describe_same_name_exile_then_investigate_prefix(
    effects: &[Effect],
) -> Option<(String, usize)> {
    let prefix = linked_action_fanout_prefix_view(effects)?;
    if prefix.action != SameNameFanoutAction::Exile
        || prefix.relation != crate::filter::TaggedOpbjectRelation::SameNameAsTagged
    {
        return None;
    }
    let investigate = structural_unwrap_render_wrappers(effects.get(prefix.consumed)?)
        .downcast_ref::<crate::effects::InvestigateEffect>()?;
    let Value::Count(filter) = &investigate.count else {
        return None;
    };
    if filter.zone != Some(Zone::Exile)
        || filter.card_types.as_slice() != [CardType::Creature]
        || !filter.nontoken
        || filter.tagged_constraints.len() != 1
        || !filter_has_tag_relation_recursive(
            filter,
            prefix.target_tag,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        )
        || !controller_of_tagged_player(&investigate.player, prefix.target_tag)
    {
        return None;
    }
    Some((
        format!(
            "{}. That player investigates for each nontoken creature exiled this way",
            prefix.text
        ),
        prefix.consumed + 1,
    ))
}

pub(super) fn describe_target_same_name_action_fanout_prefix(
    effects: &[Effect],
) -> Option<(String, usize)> {
    let [first, second, ..] = effects else {
        return None;
    };
    if let Some(text) = describe_target_same_name_action_fanout_pair(first, second) {
        return Some((text, 2));
    }
    let target_only = structural_unwrap_render_wrappers(first)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let action = effects.get(1)?;
    let fanout = effects.get(2)?;
    let (_, action_spec) = same_name_fanout_target_action_view(action)?;
    if !target_specs_select_same_objects(&target_only.target, action_spec) {
        return None;
    }
    describe_target_same_name_action_fanout_pair(action, fanout).map(|text| (text, 3))
}

/// Damage a single target creature, then destroy a typed set of objects
/// attached to that exact target (Blastfire Bolt / Turn to Slag family).
pub(super) fn describe_target_creature_damage_then_destroy_attached(
    effects: &[&Effect],
) -> Option<String> {
    let [damage_effect, destroy_effect] = effects else {
        return None;
    };
    let target_tag = effect_outer_tag(damage_effect)?;
    let damage = damage_effect_view(damage_effect)?;
    exact_target_creature_filter(&damage.target)?;

    let destroy = unwrap_basic_tag_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(attached_filter) = destroy.spec.base() else {
        return None;
    };
    let matching = attached_filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.tag == *target_tag
                && constraint.relation
                    == crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
        })
        .count();
    if matching != 1
        || attached_filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag != *target_tag
                || constraint.relation
                    != crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
        })
    {
        return None;
    }

    let mut attachment_kind = attached_filter.clone();
    attachment_kind.tagged_constraints.clear();
    attachment_kind.zone = None;
    let attachment_description = attachment_kind.description();
    let description = strip_indefinite_article(&attachment_description);
    if description.is_empty() || description == "permanent" {
        return None;
    }
    let plural = pluralize_noun_phrase(description);
    Some(format!(
        "{}. Destroy all {plural} attached to that creature",
        describe_effect(damage_effect).trim().trim_end_matches('.')
    ))
}

/// Destroy a tagged target creature, then have that exact object's owner gain
/// life. The target tag is the semantic link; no textual pronoun guessing is
/// involved.
pub(super) fn describe_destroy_target_creature_then_owner_gains(
    effects: &[&Effect],
) -> Option<String> {
    let [destroy_effect, gain_effect] = effects else {
        return None;
    };
    let target_tag = effect_outer_tag(destroy_effect)?;
    let destroy = unwrap_basic_tag_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    exact_target_creature_filter(&destroy.spec)?;

    let gain =
        unwrap_basic_tag_wrappers(gain_effect).downcast_ref::<crate::effects::GainLifeEffect>()?;
    if !matches!(
        gain.player.base(),
        ChooseSpec::Player(PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag)))
            if tag == target_tag
    ) {
        return None;
    }

    Some(format!(
        "{}. Its owner gains {}",
        describe_effect(destroy_effect).trim().trim_end_matches('.'),
        describe_life_amount_phrase(&gain.amount)
    ))
}

fn effect_moves_exact_tag_to_hand(effect: &Effect, tag: &TagKey) -> bool {
    let effect = unwrap_basic_tag_wrappers(effect);
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return move_to_zone.zone == Zone::Hand
            && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if found == tag);
    }
    if let Some(return_to_hand) = effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        return return_to_hand_uses_chosen_tag(return_to_hand, tag.as_str());
    }
    effect
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()
        .is_some_and(|for_each| for_each_moves_tag_to_hand(for_each, tag.as_str()))
}

fn filter_is_plain_same_name_reference(
    filter: &ObjectFilter,
    reference_tag: &TagKey,
    owner: Option<&PlayerFilter>,
) -> bool {
    if filter.owner.as_ref() != owner {
        return false;
    }
    let matching = filter.tagged_constraints.iter().filter(|constraint| {
        constraint.tag == *reference_tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
    });
    if matching.count() != 1 || filter.tagged_constraints.len() != 1 {
        return false;
    }
    let mut base = filter.clone();
    base.owner = None;
    base.zone = None;
    base.tagged_constraints.clear();
    base == ObjectFilter::default()
}

fn describe_same_name_search_reference_setup(effect: &Effect) -> Option<(&TagKey, String)> {
    if let Some((tag, target_only)) = tagged_target_only_effect(effect) {
        exact_single_target_object_filter(&target_only.target)?;
        return Some((tag, describe_choose_spec(&target_only.target)));
    }

    let choose = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.is_search || choose.replace_tagged_objects || choose_exact_count(choose) != Some(1) {
        return None;
    }
    Some((&choose.tag, describe_choose_selection(choose)))
}

fn same_name_search_move_for_tag<'a>(
    effect: &'a Effect,
    tag: &TagKey,
) -> Option<&'a crate::effects::MoveToZoneEffect> {
    let effect = structural_unwrap_render_wrappers(effect);
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if found == tag)
            .then_some(move_to_zone);
    }
    let for_each = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if for_each.tag != *tag {
        return None;
    }
    let [inner] = for_each.effects.as_slice() else {
        return None;
    };
    let move_to_zone = structural_unwrap_render_wrappers(inner)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if found == tag)
        .then_some(move_to_zone)
}

fn same_name_search_for_each_consumes_tag(effect: &Effect, tag: &TagKey) -> bool {
    let Some(for_each) = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()
    else {
        return false;
    };
    if for_each.tag != *tag {
        return false;
    }
    let [inner] = for_each.effects.as_slice() else {
        return false;
    };
    let inner = structural_unwrap_render_wrappers(inner);
    let consumes = |spec: &ChooseSpec| {
        matches!(spec.base(), ChooseSpec::Iterated)
            || matches!(spec.base(), ChooseSpec::Tagged(found) if found == tag)
    };
    inner
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .is_some_and(|move_to_zone| consumes(&move_to_zone.target))
        || inner
            .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
            .is_some_and(|put| consumes(&put.target))
}

fn render_same_name_reference_search_tail(
    tail: &[&Effect],
    reference_tag: &TagKey,
) -> Option<String> {
    match tail {
        [search_effect] => {
            let search = structural_unwrap_render_wrappers(search_effect)
                .downcast_ref::<crate::effects::SearchLibraryEffect>()?;
            if search.chooser != PlayerFilter::You
                || search.player != PlayerFilter::You
                || search.library_position_from_top.is_some()
                || !filter_is_plain_same_name_reference(
                    &search.filter,
                    reference_tag,
                    search.filter.owner.as_ref(),
                )
            {
                return None;
            }
            Some(describe_effect(search_effect))
        }
        [choose_effect, reveal_effect, move_effect, shuffle_effect] => {
            let choose = structural_unwrap_render_wrappers(choose_effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let reveal = structural_unwrap_render_wrappers(reveal_effect)
                .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
            let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
                .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
            if !choose.is_search
                || choose.chooser != PlayerFilter::You
                || choose_search_zones(choose)? != [Zone::Library]
                || !filter_is_plain_same_name_reference(
                    &choose.filter,
                    reference_tag,
                    choose.filter.owner.as_ref(),
                )
            {
                return None;
            }
            if let Some(move_to_zone) = same_name_search_move_for_tag(move_effect, &choose.tag) {
                describe_search_choose_then_move(choose, Some(reveal), move_to_zone, Some(shuffle))
            } else if same_name_search_for_each_consumes_tag(move_effect, &choose.tag) {
                Some(describe_effect_list(
                    &tail.iter().map(|effect| (*effect).clone()).collect::<Vec<_>>(),
                ))
            } else {
                None
            }
        }
        [choose_effect, move_effect, shuffle_effect] => {
            let choose = structural_unwrap_render_wrappers(choose_effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
                .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
            if !choose.is_search
                || choose.chooser != PlayerFilter::You
                || choose_search_zones(choose)? != [Zone::Library]
                || !filter_is_plain_same_name_reference(
                    &choose.filter,
                    reference_tag,
                    choose.filter.owner.as_ref(),
                )
            {
                return None;
            }
            if let Some(move_to_zone) = same_name_search_move_for_tag(move_effect, &choose.tag) {
                describe_search_choose_then_move(choose, None, move_to_zone, Some(shuffle))
            } else if same_name_search_for_each_consumes_tag(move_effect, &choose.tag) {
                Some(describe_effect_list(
                    &tail.iter().map(|effect| (*effect).clone()).collect::<Vec<_>>(),
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// A same-name library search whose comparison object is selected only to
/// establish the semantic reference. Fold that setup selection into the
/// search's filter phrase rather than printing it as an independent action.
pub(in crate::compiled_text) fn describe_same_name_reference_search_bundle(
    effects: &[&Effect],
) -> Option<String> {
    let [setup_effect, tail @ ..] = effects else {
        return None;
    };
    let (reference_tag, reference) = describe_same_name_search_reference_setup(setup_effect)?;
    let rendered = render_same_name_reference_search_tail(tail, reference_tag)?;
    for old_reference in [
        "with the same name as it",
        "with the same name as that object",
        "with the same name as that card",
        "with the same name as that creature",
        "with the same name as that permanent",
        "with the same name as that spell",
    ] {
        if rendered.contains(old_reference) {
            return Some(rendered.replacen(
                old_reference,
                &format!("with the same name as {reference}"),
                1,
            ));
        }
    }
    None
}

/// Reveal one card from your hand, then search for the card with the same
/// name. The first choice's tag proves the latter reference noun is "card".
pub(in crate::compiled_text) fn describe_single_hand_reveal_same_name_search(
    effects: &[&Effect],
) -> Option<String> {
    let [hand_choose_effect, hand_reveal_effect, tail @ ..] = effects else {
        return None;
    };
    let hand_choose = hand_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let hand_reveal = unwrap_basic_tag_wrappers(hand_reveal_effect)
        .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if hand_choose.is_search
        || hand_choose.chooser != PlayerFilter::You
        || choose_exact_count(hand_choose) != Some(1)
        || choose_primary_zone(hand_choose) != Some(Zone::Hand)
        || hand_choose.filter.owner.as_ref() != Some(&PlayerFilter::You)
        || hand_reveal.tag != hand_choose.tag
    {
        return None;
    }

    match tail {
        [search_effect] => {
            let search = unwrap_basic_tag_wrappers(search_effect)
                .downcast_ref::<crate::effects::SearchLibraryEffect>()?;
            if search.destination != Zone::Hand
                || search.chooser != PlayerFilter::You
                || search.player != PlayerFilter::You
                || !search.reveal
                || search.library_position_from_top.is_some()
                || !filter_is_plain_same_name_reference(
                    &search.filter,
                    &hand_choose.tag,
                    Some(&PlayerFilter::You),
                )
            {
                return None;
            }
        }
        [
            search_effect,
            search_reveal_effect,
            move_effect,
            shuffle_effect,
        ] => {
            let search = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let search_reveal = unwrap_basic_tag_wrappers(search_reveal_effect)
                .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
            let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
            if !search.is_search
                || search.chooser != PlayerFilter::You
                || choose_exact_count(search) != Some(1)
                || choose_search_zones(search)? != [Zone::Library]
                || !filter_is_plain_same_name_reference(&search.filter, &hand_choose.tag, None)
                || search_reveal.tag != search.tag
                || !effect_moves_exact_tag_to_hand(move_effect, &search.tag)
                || shuffle.player != PlayerFilter::You
            {
                return None;
            }
        }
        _ => return None,
    }

    let selection = describe_choose_selection(hand_choose);
    if !selection.contains("card") {
        return None;
    }
    Some(format!(
        "Reveal {selection} in your hand. Search your library for a card with the same name as that card, reveal it, put it into your hand, then shuffle"
    ))
}

/// A reveal instruction is commonly lowered as an internal choose followed by
/// `RevealTagged`. Keep the choice as execution machinery while rendering the
/// authored reveal action. Exact count, hand zone, owner, chooser, and tag are
/// all proved before the setup is hidden.
pub(in crate::compiled_text) fn describe_single_hand_reveal_setup(
    effects: &[&Effect],
) -> Option<String> {
    let [choose_effect, reveal_effect] = effects else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = structural_unwrap_render_wrappers(reveal_effect)
        .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_exact_count(choose) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose.filter.owner.as_ref() != Some(&PlayerFilter::You)
        || reveal.tag != choose.tag
    {
        return None;
    }
    let selection = describe_choose_selection(choose);
    selection
        .contains("card")
        .then(|| format!("Reveal {selection} from your hand"))
}

fn describe_search_exile_shuffle_tail_view<'a>(
    effects: &[&'a Effect],
) -> Option<(&'a PlayerFilter, String)> {
    let [choose_effect, for_each_effect, shuffle_effect] = effects else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let search_owner = choose.filter.owner.as_ref()?;
    if !choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_search_zones(choose)? != [Zone::Library]
    {
        return None;
    }
    let mut plain_filter = choose.filter.clone();
    plain_filter.zone = None;
    plain_filter.owner = None;
    if plain_filter != ObjectFilter::default() {
        return None;
    }

    let for_each = structural_unwrap_render_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [move_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let move_uses_search_tag = matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
        || matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag);
    if for_each.tag != choose.tag || move_to_zone.zone != Zone::Exile || !move_uses_search_tag {
        return None;
    }

    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !same_search_player_filter(&shuffle.player, search_owner) {
        return None;
    }

    let tail = [
        (*choose_effect).clone(),
        (*for_each_effect).clone(),
        (*shuffle_effect).clone(),
    ];
    Some((search_owner, describe_effect_list(&tail)))
}

/// Fold an internal target declaration into a library-search instruction when
/// the target, search owner, tagged exile consumer, and final shuffle all name
/// the same player/object set.
pub(in crate::compiled_text) fn describe_target_player_search_exile_shuffle_bundle(
    effects: &[&Effect],
) -> Option<String> {
    let [target_effect, tail @ ..] = effects else {
        return None;
    };
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::Player(target_player) = target_only.target.base() else {
        return None;
    };
    if !target_only.target.is_target() || !target_only.target.count().is_single() {
        return None;
    }

    let (search_owner, rendered) = describe_search_exile_shuffle_tail_view(tail)?;
    if !same_search_player_filter(target_player, search_owner) {
        return None;
    }
    Some(rendered)
}

fn basic_land_exception_graveyard_base(filter: &ObjectFilter) -> Option<ObjectFilter> {
    if filter.any_of.len() != 2 {
        return None;
    }
    let normalize_branch = |branch: &ObjectFilter| -> Option<(ObjectFilter, bool, bool)> {
        let mut base = branch.clone();
        let excludes_land =
            base.excluded_card_types == [CardType::Land] && base.excluded_supertypes.is_empty();
        let excludes_basic =
            base.excluded_card_types.is_empty() && base.excluded_supertypes == [Supertype::Basic];
        if !excludes_land && !excludes_basic {
            return None;
        }
        base.excluded_card_types.clear();
        base.excluded_supertypes.clear();
        Some((base, excludes_land, excludes_basic))
    };
    let (left, left_land, left_basic) = normalize_branch(&filter.any_of[0])?;
    let (right, right_land, right_basic) = normalize_branch(&filter.any_of[1])?;
    if left != right || !((left_land && right_basic) || (left_basic && right_land)) {
        return None;
    }
    if left.zone != Some(Zone::Graveyard)
        || left.owner.is_some()
        || left.controller.is_some()
        || left != ObjectFilter::default().in_zone(Zone::Graveyard)
    {
        return None;
    }
    Some(left)
}

/// Choose a nonbasic-land card in a graveyard, search all of its owner's
/// zones for cards sharing its name, exile the chosen set, then shuffle.
/// This covers both all-matching and optional-selection extraction effects.
pub(super) fn describe_target_card_same_name_extraction(effects: &[&Effect]) -> Option<String> {
    let (core, draw_effect) = match effects.len() {
        4 => (effects, None),
        5 => (&effects[..4], Some(effects[4])),
        _ => return None,
    };
    let [target_effect, search_effect, exile_effect, shuffle_effect] = core else {
        return None;
    };
    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    let target_filter = exact_single_target_object_filter(&target_only.target)?;
    basic_land_exception_graveyard_base(target_filter)?;

    let search = structural_unwrap_render_wrappers(search_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let owner = search.filter.owner.as_ref()?;
    let is_selected_card_owner = |player: &PlayerFilter| {
        matches!(
            player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag))
                | PlayerFilter::AliasedOwnerOf(crate::filter::ObjectRef::Tagged(tag))
                if tag == target_tag
        ) || matches!(
            player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target)
                | PlayerFilter::AliasedOwnerOf(crate::filter::ObjectRef::Target)
        )
    };
    if !search.is_search
        || search.chooser != PlayerFilter::You
        || choose_search_zones(search)? != [Zone::Graveyard, Zone::Hand, Zone::Library]
        // This exact four-effect family contains one declared target: the
        // tagged card above. Accept both lowering forms for its owner, but
        // still require the same-name constraint to reference that exact tag.
        || !is_selected_card_owner(owner)
        || !filter_is_plain_same_name_reference(&search.filter, target_tag, Some(owner))
    {
        return None;
    }
    let for_each = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [move_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let move_to_exile = downcast_search_split_move_to_zone(move_effect)?;
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if for_each.tag != search.tag
        || !search_split_move_to_zone_uses_tag(move_to_exile, search.tag.as_str(), Zone::Exile)
        || !is_selected_card_owner(&shuffle.player)
    {
        return None;
    }

    let selection = match search.search_mode {
        SearchSelectionMode::AllMatching => "all cards",
        SearchSelectionMode::Optional => "any number of cards",
        SearchSelectionMode::Exact if search.count.min == 0 && search.count.max.is_none() => {
            "all cards"
        }
        _ => return None,
    };
    let prefix = format!(
        "Choose target card in a graveyard other than a basic land card. Search its owner's graveyard, hand, and library for {selection} with the same name as that card and exile them"
    );
    if let Some(draw_effect) = draw_effect {
        if !same_name_extraction_hand_draw_matches(draw_effect, &search.tag, owner) {
            return None;
        }
        Some(format!(
            "{prefix}. That player shuffles, then draws a card for each card exiled from their hand this way"
        ))
    } else {
        Some(format!("{prefix}. Then that player shuffles"))
    }
}

pub(crate) fn describe_target_player_exile_hand_delayed_return(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, exile_effect, schedule_effect] = effects else {
        return None;
    };
    let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let selected_player = choose_spec_player_filter(&target_only.target)?;
    if !matches!(selected_player, PlayerFilter::Target(_)) {
        return None;
    }

    let tagged_exile = exile_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let exile = tagged_exile
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::All(hand_filter) = &exile.spec else {
        return None;
    };
    let mut plain_filter = hand_filter.clone();
    let hand_owner = plain_filter.owner.take()?;
    plain_filter.zone = None;
    if !exile.face_down
        || hand_filter.zone != Some(Zone::Hand)
        || plain_filter != ObjectFilter::default()
        || !player_filters_refer_to_same_player(&selected_player, &hand_owner)
    {
        return None;
    }

    let schedule =
        schedule_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    let end_step = schedule
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfEndStepTrigger>()?;
    let expected_delayed_player =
        PlayerFilter::AliasedOwnerOf(crate::filter::ObjectRef::Tagged(tagged_exile.tag.clone()));
    if !schedule.one_shot
        || !schedule.start_next_turn
        || schedule.until_end_of_turn
        || end_step.player != expected_delayed_player
    {
        return None;
    }

    let delayed = schedule.effects.flattened_default_effects();
    let [return_effect] = delayed else {
        return None;
    };
    let return_to_hand = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    if !matches!(
        return_to_hand.spec.base(),
        ChooseSpec::Tagged(tag) if tag == &tagged_exile.tag
    ) || return_to_hand.destination_player_surface.as_ref() != Some(&expected_delayed_player)
    {
        return None;
    }

    let player = describe_player_filter(&selected_player);
    Some(format!(
        "{} {} all cards from their hand face down. At the beginning of the end step of that player's next turn, that player returns those cards to their hand",
        capitalize_first(&player),
        player_verb(&player, "exile", "exiles"),
    ))
}

pub(super) fn choose_search_zones(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<Vec<Zone>> {
    let primary_zone = choose.filter.zone.or(choose.zone)?;
    let mut zones = vec![primary_zone];
    for zone in &choose.additional_zones {
        if !zones.contains(zone) {
            zones.push(*zone);
        }
    }
    Some(zones)
}

pub(super) fn search_split_filter_is_tagged_as(filter: &ObjectFilter, tag: &str) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == tag
    })
}

pub(super) fn downcast_search_split_move_to_zone(
    effect: &Effect,
) -> Option<&crate::effects::MoveToZoneEffect> {
    unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
}

pub(super) fn search_split_move_to_zone_uses_tag(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &str,
    zone: Zone,
) -> bool {
    move_to_zone.zone == zone
        && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if found.as_str() == tag)
}

/// "Exile target X. Search its controller's graveyard, hand, and library for
/// all cards / any number of cards with the same name as that X and exile
/// them. Then that player shuffles." (Eradicate, Splinter, Sowing Salt,
/// Scour, Crumble to Dust).
pub(super) fn describe_exile_target_search_same_name_exile_shuffle_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let (core, draw_effect) = match filtered.len() {
        4 => (filtered, None),
        5 => (&filtered[..4], Some(filtered[4])),
        _ => return None,
    };
    let [exile_effect, search_effect, for_each_effect, shuffle_effect] = core else {
        return None;
    };
    let exile_tag = wrapped_effect_tag(exile_effect)?;
    let exile = downcast_search_split_move_to_zone(exile_effect)?;
    let search = structural_unwrap_render_wrappers(search_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = structural_unwrap_render_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if exile.zone != Zone::Exile || !exile.target.is_target() {
        return None;
    }
    let ChooseSpec::Object(exiled_filter) = exile.target.base() else {
        return None;
    };
    let search_owner = search.filter.owner.as_ref()?;
    if !search.is_search
        || search.chooser != PlayerFilter::You
        || choose_search_zones(search)? != vec![Zone::Graveyard, Zone::Hand, Zone::Library]
        || search.count.min != 0
        || search.count.max.is_some()
        || !player_is_controller_of_produced_target(search_owner, exile_tag)
        || !search.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                && constraint.tag == *exile_tag
        })
    {
        return None;
    }
    let [move_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let move_to_exile = downcast_search_split_move_to_zone(move_effect)?;
    if for_each.tag != search.tag
        || !search_split_move_to_zone_uses_tag(move_to_exile, search.tag.as_str(), Zone::Exile)
        || !same_search_player_filter(&shuffle.player, search_owner)
    {
        return None;
    }

    let selection = match search.search_mode {
        SearchSelectionMode::Optional => "any number of cards",
        SearchSelectionMode::AllMatching | SearchSelectionMode::Exact => "all cards",
    };
    let noun = match exiled_filter.card_types.as_slice() {
        [card_type] => card_type.selection_name(),
        _ => "permanent",
    };
    let prefix = format!(
        "Exile {}. Search its controller's graveyard, hand, and library for {selection} with the same name as that {noun} and exile them",
        describe_choose_spec(&exile.target)
    );
    if let Some(draw_effect) = draw_effect {
        if !same_name_extraction_hand_draw_matches(draw_effect, &search.tag, search_owner) {
            return None;
        }
        Some(format!(
            "{prefix}. That player shuffles, then draws a card for each card exiled from their hand this way."
        ))
    } else {
        Some(format!("{prefix}. Then that player shuffles."))
    }
}

pub(super) fn describe_reveal_hand_choose_graveyard_exile_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [
        look_effect,
        hand_choose_effect,
        graveyard_choose_effect,
        exile_effect,
    ] = filtered
    else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let hand_choose = hand_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let graveyard_choose =
        graveyard_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile = downcast_search_split_move_to_zone(exile_effect)?;

    if !look.reveal
        || !matches!(
            look.target.base(),
            ChooseSpec::Player(PlayerFilter::Opponent)
        )
        || hand_choose.chooser != PlayerFilter::You
        || graveyard_choose.chooser != PlayerFilter::You
        || choose_exact_count(hand_choose) != Some(1)
        || choose_exact_count(graveyard_choose) != Some(1)
        || choose_primary_zone(hand_choose) != Some(Zone::Hand)
        || choose_primary_zone(graveyard_choose) != Some(Zone::Graveyard)
        || !hand_choose.filter.owner.as_ref().is_some_and(|owner| {
            player_filters_refer_to_same_player(owner, &PlayerFilter::target_opponent())
        })
        || !graveyard_choose.filter.owner.as_ref().is_some_and(|owner| {
            player_filters_refer_to_same_player(owner, &PlayerFilter::target_opponent())
        })
        || hand_choose.filter.card_types != graveyard_choose.filter.card_types
        || !search_split_move_to_zone_uses_tag(exile, hand_choose.tag.as_str(), Zone::Exile)
        || hand_choose.tag != graveyard_choose.tag
    {
        return None;
    }

    let mut display_filter = hand_choose.filter.clone();
    display_filter.zone = None;
    display_filter.owner = None;
    display_filter.controller = None;
    let mut display_description = display_filter.description();
    if !display_description.contains("card") {
        display_description.push_str(" card");
    }
    let choice_text = with_indefinite_article(&display_description);

    Some(format!(
        "Target opponent reveals their hand. You choose {choice_text} from it, then choose {choice_text} from their graveyard. Exile the chosen cards."
    ))
}

pub(super) fn describe_choose_card_name_selection(
    choose_name: &crate::effects::ChooseCardNameEffect,
) -> String {
    if let Some(filter) = &choose_name.filter {
        let mut filter_text = strip_leading_article(&filter.description()).to_string();
        if filter.card_types.is_empty() {
            // Card names are card properties: the filter's default
            // battlefield noun ("permanent") reads wrong here.
            filter_text = filter_text
                .replace("permanents", "cards")
                .replace("permanent", "card");
        }
        if !filter_text.to_ascii_lowercase().contains("card") {
            filter_text.push_str(" card");
        }
        with_indefinite_article(&filter_text)
    } else {
        "a card".to_string()
    }
}

/// "Choose a <kind> card name. <Player> reveals their hand and discards all
/// cards with that name." (Cabal Therapy).
pub(super) fn describe_choose_name_reveal_hand_discard_named_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let filtered = match filtered {
        [target, tail @ ..]
            if target
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some() =>
        {
            tail
        }
        _ => filtered,
    };
    let [choose_name_effect, look_effect, discard_effect] = filtered else {
        return None;
    };
    let choose_name = choose_name_effect.downcast_ref::<crate::effects::ChooseCardNameEffect>()?;
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;

    if !look.reveal
        || discard.random
        || discard.any_number
        || choose_name.chooser != PlayerFilter::You
    {
        return None;
    }
    let card_filter = discard.card_filter.as_ref()?;
    if card_filter.name.as_deref() != Some("{chosen name}") {
        return None;
    }
    // The revealed hand and the discard must belong to the same player.
    let look_player = choose_spec_player_filter(&look.target)?;
    if !player_filters_refer_to_same_player(&look_player, &discard.player) {
        return None;
    }

    // A count derived from the same chosen-name filter means the whole
    // matching subset, not an arbitrary numeric discard. Preserve that
    // relationship in the rendered text instead of expanding it as
    // "discards X cards named ...".
    let discard_count = if matches!(
        discard.count.unhinted(),
        Value::Count(filter) if filter.name.as_deref() == Some("{chosen name}")
    ) {
        "all cards with that name".to_string()
    } else {
        describe_discard_count(&discard.count, Some(card_filter))
    };
    let player = describe_player_filter(&look_player);
    let reveal_verb = player_verb(&player, "reveal", "reveals");
    let discard_verb = player_verb(&player, "discard", "discards");
    let hand = if player == "you" {
        "your hand"
    } else {
        "their hand"
    };
    Some(format!(
        "Choose {} name, then {player} {reveal_verb} {hand} and {discard_verb} {discard_count}",
        describe_choose_card_name_selection(choose_name),
    ))
}

/// Preserve the discard result boundary used by "If they can't" instead of
/// folding the reveal and result-producing discard into one action.
pub(super) fn describe_choose_name_reveal_discard_failure_draw_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [
        choose_name_effect,
        look_effect,
        discard_effect,
        failure_effect,
    ] = filtered
    else {
        return None;
    };
    let choose_name = choose_name_effect.downcast_ref::<crate::effects::ChooseCardNameEffect>()?;
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let with_id = discard_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let discard = with_id
        .effect
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    let failure = failure_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [draw_effect] = failure.then.as_slice() else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let looked_player = choose_spec_player_filter(&look.target)?;
    if choose_name.chooser != PlayerFilter::You
        || !look.reveal
        || discard.count != Value::Fixed(1)
        || discard.random
        || discard.any_number
        || discard
            .card_filter
            .as_ref()
            .and_then(|filter| filter.name.as_deref())
            != Some("{chosen name}")
        || !player_filters_refer_to_same_player(&looked_player, &discard.player)
        || failure.condition != with_id.id
        || failure.predicate != EffectPredicate::DidNotHappen
        || !failure.else_.is_empty()
        || draw.player != PlayerFilter::You
        || draw.count != Value::Fixed(1)
    {
        return None;
    }

    let player = describe_player_filter(&looked_player);
    let reveal_verb = player_verb(&player, "reveal", "reveals");
    Some(format!(
        "Choose {} name. {} {reveal_verb} their hand. That player discards a card with that name. If they can't, you draw a card",
        describe_choose_card_name_selection(choose_name),
        capitalize_first(&player),
    ))
}

/// "Reveal any number of <kind> cards in your hand" — the parser models this
/// as choosing any number of matching cards in hand, then revealing the
/// chosen cards (Scent of Cinder and friends).
pub(super) fn describe_choose_hand_then_reveal_chosen_pair(
    choose_effect: &Effect,
    reveal_effect: &Effect,
) -> Option<String> {
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = unwrap_basic_tag_wrappers(reveal_effect)
        .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if reveal.tag != choose.tag
        || choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose.filter.owner != Some(PlayerFilter::You)
        || choose.count.min != 0
        || choose.count.max.is_some()
        || choose.count.dynamic_x
        || choose.count.random
    {
        return None;
    }

    let mut display_filter = choose.filter.clone();
    display_filter.zone = None;
    display_filter.owner = None;
    let mut selection = strip_indefinite_article(&display_filter.description()).to_string();
    if choose.filter.card_types.is_empty() {
        // The cards live in hand: the filter's default battlefield noun
        // ("permanent") reads wrong here.
        selection = selection
            .replace("permanents", "cards")
            .replace("permanent", "card");
    }
    Some(format!(
        "Reveal any number of {} in your hand",
        pluralize_hand_card_selection(&selection)
    ))
}

pub(super) fn pluralize_hand_card_selection(selection: &str) -> String {
    let plural = pluralize_noun_phrase(selection);
    if plural.contains("card") {
        return plural;
    }
    for (plural_type, card_type) in [
        ("creatures", "creature"),
        ("artifacts", "artifact"),
        ("enchantments", "enchantment"),
        ("lands", "land"),
        ("planeswalkers", "planeswalker"),
        ("battles", "battle"),
        ("instants", "instant"),
        ("sorceries", "sorcery"),
        ("permanents", "permanent"),
    ] {
        if plural == plural_type {
            return format!("{card_type} cards");
        }
        if let Some(rest) = plural.strip_prefix(&format!("{plural_type} ")) {
            return format!("{card_type} cards {rest}");
        }
    }
    format!("{plural} cards")
}

pub(super) fn search_split_effect_moves_chosen_to_hand(effect: &Effect, chosen_tag: &str) -> bool {
    if let Some(hand_move) = downcast_search_split_move_to_zone(effect) {
        return search_split_move_to_zone_uses_tag(hand_move, chosen_tag, Zone::Hand);
    }
    unwrap_basic_tag_wrappers(effect)
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .is_some_and(|return_to_hand| {
            matches!(
                return_to_hand.spec.base(),
                ChooseSpec::Tagged(found) if found.as_str() == chosen_tag
            )
        })
}

pub(super) fn search_split_effect_moves_unselected_to_zone(
    effect: &Effect,
    source_tag: &str,
    chosen_tag: &str,
    zone: Zone,
) -> bool {
    for_each_tagged_for_compaction(effect).is_some_and(|(_, for_each)| {
        for_each_moves_unselected_to_zone(for_each, source_tag, chosen_tag, zone)
    })
}

fn describe_search_two_split_battlefield_hand_sequence(effects: &[&Effect]) -> Option<String> {
    let (core, trailing_scry) = match effects.len() {
        6 => (effects, None),
        7 => (
            &effects[..6],
            Some(effects[6].downcast_ref::<crate::effects::ScryEffect>()?),
        ),
        _ => return None,
    };
    let [
        search_effect,
        reveal_effect,
        choose_effect,
        battlefield_effect,
        hand_effect,
        shuffle_effect,
    ] = core
    else {
        return None;
    };
    let search = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (_, battlefield_each) = for_each_tagged_for_compaction(battlefield_effect)?;
    let [put_effect] = battlefield_each.effects.as_slice() else {
        return None;
    };
    let put = unwrap_basic_tag_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if !search.is_search
        || search.search_mode != crate::effect::SearchSelectionMode::Optional
        || search.count.min != 0
        || search.count.max != Some(2)
        || search.count_value.is_some()
        || search.chooser != PlayerFilter::You
        || search.filter.owner != Some(PlayerFilter::You)
        || choose_search_zones(search)? != vec![Zone::Library]
        || reveal.tag != search.tag
        || choose.is_search
        || choose.count.min != 1
        || choose.count.max != Some(1)
        || choose.count_value.is_some()
        || choose.chooser != search.chooser
        || !choose_search_zones(choose)?.contains(&Zone::Library)
        || !search_split_filter_is_tagged_as(&choose.filter, search.tag.as_str())
        || battlefield_each.tag != choose.tag
        || !matches!(put.target.base(), ChooseSpec::Iterated)
        || !put.tapped
        || put.controller != PlayerFilter::You
        || !search_split_effect_moves_unselected_to_zone(
            hand_effect,
            search.tag.as_str(),
            choose.tag.as_str(),
            Zone::Hand,
        )
        || shuffle.player != PlayerFilter::You
        || trailing_scry.is_some_and(|scry| scry.player != PlayerFilter::You)
    {
        return None;
    }

    let mut display_filter = search.filter.clone();
    display_filter.zone = None;
    display_filter.owner = None;
    let is_basic_land_gate_union = display_filter.any_of.len() == 2
        && display_filter.union_surface.connective()
            == crate::filter::ObjectFilterUnionConnective::AndOr
        && display_filter.any_of.iter().any(|branch| {
            branch.card_types.contains(&CardType::Land)
                && branch.supertypes.contains(&Supertype::Basic)
        })
        && display_filter
            .any_of
            .iter()
            .any(|branch| branch.subtypes.contains(&Subtype::Gate));
    let selection = if is_basic_land_gate_union {
        "up to two basic land cards and/or Gate cards".to_string()
    } else if display_filter == ObjectFilter::default() {
        "up to two cards".to_string()
    } else {
        let filter_text =
            describe_nonbattlefield_card_filter_without_zone(&display_filter, Zone::Library);
        let filter_text = filter_text
            .strip_suffix(" card")
            .unwrap_or(&filter_text)
            .trim();
        describe_search_selection_with_cards(&format!("up to two {filter_text}"))
    };

    if let Some(scry) = trailing_scry {
        return Some(format!(
            "Search your library for {selection}, reveal those cards, and put one onto the battlefield tapped and the other into your hand. Shuffle, then scry {}",
            describe_value(&scry.count)
        ));
    }
    Some(format!(
        "Search your library for {selection}, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle"
    ))
}

pub(in crate::compiled_text) fn describe_search_two_split_hand_graveyard_sequence(
    effects: &[&Effect],
) -> Option<String> {
    if let Some(compact) = describe_search_two_split_battlefield_hand_sequence(effects) {
        return Some(compact);
    }
    let [
        search_effect,
        choose_effect,
        hand_effect,
        graveyard_effect,
        shuffle_effect,
    ] = effects
    else {
        return None;
    };
    let search = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if !search.is_search
        || choose.is_search
        || search.count.min != 2
        || search.count.max != Some(2)
        || search.count_value.is_some()
        || choose.count.min != 1
        || choose.count.max != Some(1)
        || choose.count_value.is_some()
        || search.chooser != choose.chooser
        || shuffle.player != search.chooser
        || choose_search_zones(search)? != vec![Zone::Library]
        || !choose_search_zones(choose)?.contains(&Zone::Library)
        || !search_split_filter_is_tagged_as(&choose.filter, search.tag.as_str())
        || !search_split_effect_moves_chosen_to_hand(hand_effect, choose.tag.as_str())
        || !search_split_effect_moves_unselected_to_zone(
            graveyard_effect,
            search.tag.as_str(),
            choose.tag.as_str(),
            Zone::Graveyard,
        )
    {
        return None;
    }

    if search.chooser == PlayerFilter::You {
        return Some(
            "Search your library for two cards. Put one into your hand and the other into your graveyard. Then shuffle"
                .to_string(),
        );
    }

    let player = describe_player_filter(&search.chooser);
    let capitalized = capitalize_first(&player);
    let possessive = describe_possessive_player_filter(&search.chooser);
    let shuffle_verb = player_verb(&player, "shuffle", "shuffles");
    Some(format!(
        "{capitalized} searches {possessive} library for two cards. Put one into {possessive} hand and the other into {possessive} graveyard. Then {player} {shuffle_verb}"
    ))
}

pub(super) fn normalize_search_descriptor_for_origin(
    descriptor: &str,
    searched_library: bool,
) -> String {
    let mut descriptor = descriptor.trim().to_string();
    if searched_library {
        for phrase in [
            " in your library",
            " in target opponent's library",
            " in target player's library",
            " in that player's library",
            " in their library",
            " in library",
            " in the library",
        ] {
            descriptor = descriptor.replace(phrase, "");
        }
    }
    descriptor = descriptor.replace("permanent you own named ", "card you own named ");
    descriptor = descriptor.replace("permanent named ", "card named ");
    descriptor = descriptor.replace("card you own named ", "card named ");
    descriptor
}

pub(super) fn describe_search_selection_from_filter_text(
    choose: &crate::effects::ChooseObjectsEffect,
    filter_text: &str,
) -> String {
    let filter_text = filter_text.trim();
    let where_clause = describe_runtime_choice_where_clause(choose).unwrap_or_default();
    let filter_is_generic_card = filter_text.eq_ignore_ascii_case("card");
    let simple_land_subtype = (choose.filter.card_types.as_slice() == [CardType::Land]
        && choose.filter.subtypes.len() == 1)
        .then(|| {
            let subtype = choose.filter.subtypes[0];
            let mut remainder = choose.filter.clone();
            remainder.zone = None;
            remainder.owner = None;
            remainder.card_types.clear();
            remainder.subtypes.clear();
            (remainder == ObjectFilter::default()).then_some(subtype)
        })
        .flatten();

    if choose.count.max == Some(1) {
        if let Some(subtype) = simple_land_subtype {
            return format!("a {subtype} card");
        }
        return if filter_is_generic_card {
            "a card".to_string()
        } else {
            with_indefinite_article(filter_text)
        };
    }

    if let Some(runtime_count) = describe_runtime_choice_count(choose) {
        if let Some(subtype) = simple_land_subtype {
            return format!("{runtime_count} {subtype} cards{where_clause}");
        }
        return if filter_is_generic_card {
            format!("{runtime_count} cards{where_clause}")
        } else {
            format!("{runtime_count} {filter_text}{where_clause}")
        };
    }

    let count_text = describe_choice_count(&choose.count);
    if filter_is_generic_card {
        if count_text == "all" {
            "all cards".to_string()
        } else if count_text == "any number of" {
            "any number of cards".to_string()
        } else {
            format!("{count_text} cards")
        }
    } else {
        format!("{count_text} {filter_text}")
    }
}

pub(super) fn describe_search_selection_with_cards_preserving_where(selection: &str) -> String {
    if let Some((head, tail)) = selection.split_once(", where X is ") {
        return format!(
            "{}, where X is {}",
            describe_search_selection_with_cards(head),
            tail
        );
    }
    describe_search_selection_with_cards(selection)
}

pub(super) fn for_each_subject_reference_phrase(subject: &str) -> &'static str {
    let lower = subject.to_ascii_lowercase();
    if lower.contains("creature") {
        "that creature"
    } else if lower.contains("permanent") {
        "that permanent"
    } else if lower.contains("artifact") {
        "that artifact"
    } else if lower.contains("enchantment") {
        "that enchantment"
    } else if lower.contains("land") {
        "that land"
    } else if lower.contains("spell") {
        "that spell"
    } else if lower.contains("card") {
        "that card"
    } else {
        "that object"
    }
}

pub(super) fn describe_stack_object_copy_target(target: &ChooseSpec) -> String {
    match target {
        ChooseSpec::Source => "this spell".to_string(),
        ChooseSpec::Tagged(tag) if matches!(tag.as_str(), "triggering" | "__it__" | "it") => {
            "that spell".to_string()
        }
        ChooseSpec::All(filter) => {
            if let Some(abilities) = describe_all_activated_and_triggered_abilities(filter) {
                return abilities;
            }
            describe_choose_spec(target)
        }
        _ => {
            let described = describe_choose_spec(target);
            if described == "it" {
                "that spell".to_string()
            } else {
                described
            }
        }
    }
}

fn describe_all_activated_and_triggered_abilities(filter: &ObjectFilter) -> Option<String> {
    if filter.zone != Some(Zone::Stack) || filter.any_of.len() != 2 {
        return None;
    }
    let mut kinds = filter
        .any_of
        .iter()
        .map(|branch| {
            let mut base = branch.clone();
            let kind = base.stack_kind.take()?;
            base.zone = None;
            (base == ObjectFilter::default()).then_some(kind)
        })
        .collect::<Option<Vec<_>>>()?;
    kinds.sort_by_key(|kind| match kind {
        StackObjectKind::ActivatedAbility => 0,
        StackObjectKind::TriggeredAbility => 1,
        _ => 2,
    });
    if kinds
        != [
            StackObjectKind::ActivatedAbility,
            StackObjectKind::TriggeredAbility,
        ]
    {
        return None;
    }

    let mut outer = filter.clone();
    let controller = outer.controller.take();
    let other = std::mem::take(&mut outer.other);
    outer.zone = None;
    outer.any_of.clear();
    if outer != ObjectFilter::default() {
        return None;
    }
    let controller = match controller {
        Some(PlayerFilter::You) => " you control",
        Some(PlayerFilter::NotYou) => " you don't control",
        Some(PlayerFilter::Opponent) => " an opponent controls",
        None => "",
        _ => return None,
    };
    Some(format!(
        "all {}activated and triggered abilities{controller}",
        if other { "other " } else { "" }
    ))
}

pub(super) fn describe_counter_all_stack_abilities(target: &ChooseSpec) -> Option<&'static str> {
    let ChooseSpec::All(filter) = target else {
        return None;
    };
    if filter.zone != Some(Zone::Stack)
        || filter.controller != Some(PlayerFilter::Opponent)
        || filter.stack_kind != Some(StackObjectKind::Ability)
    {
        return None;
    }

    let mut base = filter.clone();
    base.zone = None;
    base.controller = None;
    base.stack_kind = None;
    (base == ObjectFilter::default()).then_some("all abilities your opponents control")
}

pub(super) fn copy_target_player_candidate_text(filter: &PlayerFilter, plural: bool) -> String {
    match (filter, plural) {
        (PlayerFilter::Any, false) => "player".to_string(),
        (PlayerFilter::Any, true) => "players".to_string(),
        (PlayerFilter::Opponent, false) => "opponent".to_string(),
        (PlayerFilter::Opponent, true) => "opponents".to_string(),
        (PlayerFilter::You, _) => "you".to_string(),
        (_, false) => strip_leading_article(&describe_player_filter(filter)).to_string(),
        (_, true) => pluralize_noun_phrase(&describe_player_filter(filter)),
    }
}

pub(super) fn describe_copy_target_candidates(
    object_filter: Option<&ObjectFilter>,
    player_filter: Option<&PlayerFilter>,
    plural: bool,
) -> String {
    let object_text = object_filter.map(|filter| {
        let description = strip_leading_article(&filter.description()).to_string();
        if plural {
            pluralize_noun_phrase(&description)
        } else {
            description
        }
    });
    let player_text = player_filter.map(|filter| copy_target_player_candidate_text(filter, plural));

    match (object_text, player_text, plural) {
        (Some(object), Some(player), false) => format!("{object} or {player}"),
        (Some(object), Some(player), true) => format!("{object} and {player}"),
        (Some(object), None, _) => object,
        (None, Some(player), _) => player,
        (None, None, false) => "target".to_string(),
        (None, None, true) => "targets".to_string(),
    }
}

pub(super) fn describe_copy_spell_for_each_target(
    effect: &crate::effects::CopySpellForEachTargetEffect,
) -> String {
    let stack_object = describe_stack_object_copy_target(&effect.target);
    let candidate = describe_copy_target_candidates(
        effect.object_filter.as_ref(),
        effect.player_filter.as_ref(),
        false,
    );
    let candidate = if effect.exclude_current_targets {
        format!("other {candidate}")
    } else {
        candidate
    };
    let plural_candidate = describe_copy_target_candidates(
        effect.object_filter.as_ref(),
        effect.player_filter.as_ref(),
        true,
    );

    let mut text = format!(
        "Copy {stack_object} for each {candidate} {stack_object} could target. Each copy targets a different one of those {plural_candidate}"
    );
    if effect
        .removed_supertypes
        .contains(&crate::types::Supertype::Legendary)
    {
        text.push_str(". The copies aren't legendary");
    }
    text
}

pub(super) fn copy_spell_from_effect(effect: &Effect) -> Option<&crate::effects::CopySpellEffect> {
    if let Some(copy_spell) = effect.downcast_ref::<crate::effects::CopySpellEffect>() {
        return Some(copy_spell);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return copy_spell_from_effect(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return copy_spell_from_effect(&tagged.effect);
    }
    None
}

pub(super) fn describe_draw_count_for_each_phrase(count: &Value) -> Option<String> {
    match count {
        Value::SurfaceHinted { value, hints } => {
            if hints.contains(&ValueSurfaceHint::EqualTo) {
                return None;
            }
            let phrase = if hints.contains(&ValueSurfaceHint::ThatManyCards) {
                Some("that many cards".to_string())
            } else if hints.contains(&ValueSurfaceHint::CardsDiscardedThisWay) {
                Some("a card for each card discarded this way".to_string())
            } else if hints.contains(&ValueSurfaceHint::CardsExiledThisWay) {
                Some("a card for each card exiled this way".to_string())
            } else if hints.contains(&ValueSurfaceHint::PermanentsSacrificedThisWay) {
                Some("a card for each permanent sacrificed this way".to_string())
            } else {
                describe_draw_count_for_each_phrase(value)
            };
            phrase.map(|phrase| {
                if hints.contains(&ValueSurfaceHint::AdditionalCards) {
                    additionalize_card_count_phrase(&phrase)
                } else {
                    phrase
                }
            })
        }
        Value::Count(filter) => Some(format!(
            "a card for each {}",
            describe_for_each_filter(filter)
        )),
        Value::CreaturesDiedThisTurn => {
            Some("a card for each creature that died this turn".to_string())
        }
        Value::CreaturesDiedThisTurnControlledBy(controller) => {
            let suffix = match controller {
                PlayerFilter::You => "under your control this turn".to_string(),
                PlayerFilter::Opponent => "under an opponent's control this turn".to_string(),
                PlayerFilter::Any => "this turn".to_string(),
                other => format!(
                    "under {} control this turn",
                    describe_possessive_player_filter(other)
                ),
            };
            Some(format!("a card for each creature that died {suffix}"))
        }
        Value::SpellsCastThisTurn(spell_caster) => Some(format!(
            "a card for each {}",
            describe_spells_cast_this_turn_each(spell_caster)
        )),
        Value::KickCount => Some("a card for each time this spell was kicked".to_string()),
        Value::SpellsCastThisTurnMatching {
            player: spell_caster,
            filter,
            exclude_source,
        } => {
            let base = describe_for_each_filter(filter);
            let prefix = if *exclude_source && !base.starts_with("other ") {
                "other "
            } else {
                ""
            };
            let tail = match spell_caster {
                PlayerFilter::You => "you've cast this turn".to_string(),
                PlayerFilter::Opponent => "an opponent has cast this turn".to_string(),
                PlayerFilter::Any => "cast this turn".to_string(),
                other => format!(
                    "cast this turn by {}",
                    strip_leading_article(&describe_player_filter(other))
                ),
            };
            Some(format!("a card for each {prefix}{base} {tail}"))
        }
        Value::PlayerCounters(counter_player, counter_type) => Some(format!(
            "a card for each {} counter {}",
            describe_counter_type(*counter_type),
            describe_player_counter_holder(counter_player)
        )),
        Value::CountersOnSource(counter_type) => Some(format!(
            "a card for each {} counter on this permanent",
            describe_counter_type(*counter_type)
        )),
        Value::CountersOn(spec, Some(counter_type)) => Some(format!(
            "a card for each {} counter on {}",
            describe_counter_type(*counter_type),
            describe_choose_spec(spec)
        )),
        Value::CountersOn(spec, None) => Some(format!(
            "a card for each counter on {}",
            describe_choose_spec(spec)
        )),
        Value::BasicLandTypesAmong(filter) => Some(format!(
            "a card for each {}",
            describe_basic_land_types_among(filter)
        )),
        Value::CreatureTypesAmong(filter) => Some(format!(
            "a card for each creature type among {}",
            describe_count_filter_value_subject(filter)
        )),
        Value::CardTypesAmong(filter) => Some(format!(
            "a card for each card type among {}",
            describe_count_filter_value_subject(filter)
        )),
        Value::ColorsAmong(filter) => {
            Some(format!("a card for each {}", describe_colors_among(filter)))
        }
        _ => None,
    }
}

pub(super) fn describe_for_players_vote_received_repeat(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let [effect] = for_players.effects.as_slice() else {
        return None;
    };
    let repeat = effect.downcast_ref::<crate::effects::RepeatEffectsEffect>()?;
    if repeat.count != Value::PlayerVoteCount(PlayerFilter::IteratedPlayer) {
        return None;
    }

    let player = match for_players.filter {
        PlayerFilter::Opponent => "an opponent".to_string(),
        PlayerFilter::You => "you".to_string(),
        PlayerFilter::Any => "a player".to_string(),
        _ => {
            strip_leading_article(&describe_for_each_player_filter(&for_players.filter)).to_string()
        }
    };
    let repeated = describe_damage_and_controlled_damage_pair(&repeat.effects)
        .unwrap_or_else(|| describe_effect_list(&repeat.effects));
    let repeated = lowercase_first(repeated.trim().trim_end_matches('.'));
    Some(format!("For each vote {player} received, {repeated}"))
}

pub(super) fn describe_damage_and_controlled_damage_pair(effects: &[Effect]) -> Option<String> {
    fn source_damage(
        effect: &Effect,
    ) -> Option<(Option<&ChooseSpec>, &crate::effects::DealDamageEffect)> {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return source_damage(&tagged.effect);
        }
        if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
            && let Some(damage) = with_source
                .effect
                .downcast_ref::<crate::effects::DealDamageEffect>()
        {
            return Some((Some(&with_source.source), damage));
        }
        effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .map(|damage| (None, damage))
    }

    fn source_for_each(
        effect: &Effect,
    ) -> Option<(Option<&ChooseSpec>, &crate::effects::ForEachObject)> {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return source_for_each(&tagged.effect);
        }
        if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
            && let Some(for_each) = with_source
                .effect
                .downcast_ref::<crate::effects::ForEachObject>()
        {
            return Some((Some(&with_source.source), for_each));
        }
        effect
            .downcast_ref::<crate::effects::ForEachObject>()
            .map(|for_each| (None, for_each))
    }

    let [first, second] = effects else {
        return None;
    };
    let (source, player_damage) = source_damage(first)?;
    if !matches!(
        player_damage.target,
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) {
        return None;
    }
    let (for_each_source, for_each) = source_for_each(second)?;
    let [inner] = for_each.effects.as_slice() else {
        return None;
    };
    let (inner_source, object_damage) = source_damage(inner)?;
    if object_damage.amount != player_damage.amount
        || !matches!(object_damage.target, ChooseSpec::Iterated)
    {
        return None;
    }
    let mut objects = describe_each_controlled_by_iterated(&for_each.filter)?;
    objects = objects.replace(" they control", " that player controls");
    let amount = describe_damage_amount_clause(&player_damage.amount).0;
    if let Some(subject) = source
        .or(for_each_source)
        .or(inner_source)
        .map(describe_choose_spec)
    {
        return Some(format!(
            "{subject} deals {amount} to that player and {objects}"
        ));
    }
    Some(format!("Deal {amount} to that player and {objects}"))
}

pub(super) fn tagged_copy_spell_from_effect(
    effect: &Effect,
) -> Option<(&crate::TagKey, &crate::effects::CopySpellEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let copy_spell = copy_spell_from_effect(&tagged.effect)?;
    Some((&tagged.tag, copy_spell))
}

pub(super) fn retarget_fixed_spec_uses_chosen_tag(
    spec: &ChooseSpec,
    chosen_tag: &crate::TagKey,
) -> bool {
    match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag == *chosen_tag
            })
        }
        ChooseSpec::Tagged(tag) => tag == chosen_tag,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            retarget_fixed_spec_uses_chosen_tag(inner, chosen_tag)
        }
        _ => false,
    }
}

pub(super) fn copy_retarget_reference_noun(filter: &ObjectFilter) -> &'static str {
    if filter.card_types.contains(&CardType::Creature) {
        "creature"
    } else if filter.card_types.contains(&CardType::Artifact) {
        "artifact"
    } else if filter.card_types.contains(&CardType::Enchantment) {
        "enchantment"
    } else if filter.card_types.contains(&CardType::Land) {
        "land"
    } else if filter.zone == Some(Zone::Battlefield) {
        "permanent"
    } else {
        "object"
    }
}

pub(super) fn describe_choose_copy_spell_and_retarget_copy_to_chosen(
    effects: &[&Effect],
) -> Option<String> {
    let [choose_effect, copy_effect, retarget_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.is_search || !choose.count.is_single() {
        return None;
    }

    let (copied_tag, copy_spell) = tagged_copy_spell_from_effect(copy_effect)?;
    if copy_spell.count != Value::Fixed(1)
        || !copy_spell.removed_supertypes.is_empty()
        || copy_spell.copier != choose.chooser
    {
        return None;
    }
    let copied_spell_text = describe_stack_object_copy_target(&copy_spell.target);
    if copied_spell_text != "that spell" && copied_spell_text != "this spell" {
        return None;
    }

    let retarget = retarget_effect.downcast_ref::<crate::effects::RetargetStackObjectEffect>()?;
    if retarget.chooser != choose.chooser
        || retarget.require_change
        || retarget.new_target_restriction.is_some()
        || !matches!(&retarget.target, ChooseSpec::Tagged(tag) if tag == copied_tag)
    {
        return None;
    }
    let crate::effects::RetargetMode::OneToFixed(fixed_spec) = &retarget.mode else {
        return None;
    };
    if !retarget_fixed_spec_uses_chosen_tag(fixed_spec, &choose.tag) {
        return None;
    }

    let chooser = describe_player_filter(&choose.chooser);
    let choose_verb = player_verb(&chooser, "choose", "chooses");
    let noun = copy_retarget_reference_noun(&choose.filter);
    let plural_noun = pluralize_noun_phrase(noun);
    let copy_verb = if copy_spell.copier == PlayerFilter::You {
        "Copy".to_string()
    } else {
        let copier = describe_player_filter(&copy_spell.copier);
        format!(
            "{} {}",
            capitalize_first(&copier),
            player_verb(&copier, "copy", "copies")
        )
    };
    Some(format!(
        "{chooser} {choose_verb} one of those {plural_noun}. {copy_verb} {copied_spell_text}. The copy targets the chosen {noun}"
    ))
}

pub(super) fn describe_tagged_multi_copy_then_may_retarget(effects: &[Effect]) -> Option<String> {
    let [copy_effect, may_effect] = effects else {
        return None;
    };
    let tagged = copy_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let with_id = tagged
        .effect
        .downcast_ref::<crate::effects::WithIdEffect>()?;
    let copy = with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    if matches!(copy.count.unhinted(), Value::Fixed(1)) {
        return None;
    }
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(may.decider, None | Some(PlayerFilter::You)) {
        return None;
    }
    let [retarget_effect] = may.effects.as_slice() else {
        return None;
    };
    let retarget = retarget_effect.downcast_ref::<crate::effects::RetargetStackObjectEffect>()?;
    if retarget.chooser != PlayerFilter::You
        || !matches!(retarget.mode, crate::effects::RetargetMode::All)
        || retarget.require_change
        || retarget.new_target_restriction.is_some()
        || !matches!(&retarget.target, ChooseSpec::Tagged(tag) if tag == &tagged.tag)
    {
        return None;
    }
    let copy_text = describe_effect(copy_effect)
        .trim_end_matches('.')
        .to_string();
    Some(format!(
        "{copy_text}. You may choose new targets for the copies"
    ))
}

pub(super) fn describe_phase_in_out_pair(first: &Effect, second: &Effect) -> Option<String> {
    let phase_in = first.downcast_ref::<crate::effects::PhaseInEffect>()?;
    let phase_out = second.downcast_ref::<crate::effects::PhaseOutEffect>()?;
    let ChooseSpec::All(phase_in_filter) = phase_in.spec.base() else {
        return None;
    };
    let ChooseSpec::All(phase_out_filter) = phase_out.spec.base() else {
        return None;
    };
    let phase_in_is_all_creatures = phase_in_filter.card_types == vec![CardType::Creature]
        && phase_in_filter.subtypes.is_empty()
        && phase_in_filter.static_abilities.is_empty();
    let phase_out_is_creatures_with_phasing = phase_out_filter.card_types
        == vec![CardType::Creature]
        && phase_out_filter.subtypes.is_empty()
        && phase_out_filter
            .static_abilities
            .contains(&crate::static_abilities::StaticAbilityId::Phasing);
    if phase_in_is_all_creatures && phase_out_is_creatures_with_phasing {
        Some(
            "Simultaneously, all phased-out creatures phase in and all creatures with phasing phase out"
                .to_string(),
        )
    } else {
        None
    }
}

pub(super) fn describe_for_players_target_return_unless_draw(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Opponent || for_players.effects.len() != 2 {
        return None;
    }
    let targeted = for_players.effects[0].downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = targeted
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let unless_action =
        for_players.effects[1].downcast_ref::<crate::effects::UnlessActionEffect>()?;
    if unless_action.effects.len() != 1 || unless_action.alternative.len() != 1 {
        return None;
    }
    let returned = unless_action.effects[0].downcast_ref::<crate::effects::TaggedEffect>()?;
    let return_to_hand = returned
        .effect
        .downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    if !matches!(return_to_hand.spec.base(), ChooseSpec::Tagged(tag) if tag == &targeted.tag) {
        return None;
    }
    if !matches!(
        &unless_action.player,
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag)) if tag == &targeted.tag
    ) {
        return None;
    }
    let draw = unless_action.alternative[0].downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You || draw.count != Value::Fixed(1) {
        return None;
    }
    let target_text = describe_choose_spec(&target_only.target);
    let returned_text = for_each_subject_reference_phrase(&target_text);
    Some(format!(
        "For each opponent, choose {target_text}, then return {returned_text} to its owner's hand unless its controller has you draw a card"
    ))
}

pub(super) fn choose_spec_contains_hand_advantage_player_filter(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_contains_hand_advantage_player_filter(inner)
        }
        ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            player_filter_contains_hand_advantage_filter(filter)
        }
        _ => false,
    }
}

pub(super) fn choose_spec_is_player_choice(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_is_player_choice(inner)
        }
        ChooseSpec::Player(_) | ChooseSpec::PlayerOrPlaneswalker(_) => true,
        _ => false,
    }
}

pub(super) fn player_filter_references_target_player(filter: &PlayerFilter) -> bool {
    match filter {
        PlayerFilter::Target(_) => true,
        PlayerFilter::Excluding { base, excluded } => {
            player_filter_references_target_player(base)
                || player_filter_references_target_player(excluded)
        }
        _ => false,
    }
}

pub(super) fn object_filter_references_target_player(filter: &ObjectFilter) -> bool {
    filter
        .controller
        .as_ref()
        .is_some_and(player_filter_references_target_player)
        || filter
            .owner
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .cast_by
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .targets_player
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .targets_only_player
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .attached_to_player
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .entered_battlefield_controller
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .attached_to_object
            .as_deref()
            .is_some_and(object_filter_references_target_player)
        || filter
            .dealt_damage_to_player_this_turn
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .any_of
            .iter()
            .any(object_filter_references_target_player)
}

pub(super) fn choose_spec_references_target_player(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_target_player(inner)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            object_filter_references_target_player(filter)
        }
        ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            player_filter_references_target_player(filter)
        }
        _ => false,
    }
}

pub(super) fn value_references_target_player(value: &Value) -> bool {
    match value {
        Value::SurfaceHinted { value, .. }
        | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => value_references_target_player(value),
        Value::Add(left, right) | Value::Min(left, right) => {
            value_references_target_player(left) || value_references_target_player(right)
        }
        Value::Count(filter)
        | Value::CountScaled(filter, _)
        | Value::GreatestCount(filter)
        | Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestToughness(filter)
        | Value::GreatestManaValue(filter)
        | Value::LeastPower(filter)
        | Value::LeastToughness(filter)
        | Value::LeastManaValue(filter)
        | Value::BasicLandTypesAmong(filter)
        | Value::CreatureTypesAmong(filter)
        | Value::CardTypesAmong(filter)
        | Value::ColorsAmong(filter)
        | Value::DistinctNames(filter)
        | Value::DistinctPowers(filter) => object_filter_references_target_player(filter),
        Value::StaticAbilitiesAmong { filter, .. } => {
            object_filter_references_target_player(filter)
        }
        Value::CreaturesDiedThisTurnControlledBy(player)
        | Value::CountPlayers(player)
        | Value::PartySize(player)
        | Value::LifeTotal(player)
        | Value::LifeTotalAsTurnBegan(player)
        | Value::LifeTotalDifference(player)
        | Value::UnspentMana(player)
        | Value::Speed(player)
        | Value::StartingLifeTotal(player)
        | Value::HalfLifeTotalRoundedUp(player)
        | Value::HalfLifeTotalRoundedDown(player)
        | Value::HalfStartingLifeTotalRoundedUp(player)
        | Value::HalfStartingLifeTotalRoundedDown(player)
        | Value::CardsInHand(player)
        | Value::CardsInLibrary(player)
        | Value::DevotionToChosenColor(player)
        | Value::LifeGainedThisTurn(player)
        | Value::LifeLostThisTurn(player)
        | Value::CardsDiscardedThisTurn(player)
        | Value::DamageDealtToPlayersThisTurn(player)
        | Value::NoncombatDamageDealtToPlayersThisTurn(player)
        | Value::MaxCardsDrawnThisTurn(player)
        | Value::MaxDiceRolledThisTurn(player)
        | Value::LandsEnteredBattlefieldThisTurn(player)
        | Value::MaxCardsInHand(player)
        | Value::CardsInGraveyard(player)
        | Value::SpellsCastThisTurn(player)
        | Value::SpellsCastBeforeThisTurn(player)
        | Value::CommanderCastCount(player)
        | Value::CardTypesInGraveyard(player) => player_filter_references_target_player(player),
        Value::NoncombatDamageDealtBySourcesControlledThisTurn { player, .. }
        | Value::Devotion { player, .. } => player_filter_references_target_player(player),
        Value::SpellsCastThisTurnMatching { player, filter, .. } => {
            player_filter_references_target_player(player)
                || object_filter_references_target_player(filter)
        }
        Value::PowerOf(spec) | Value::ToughnessOf(spec) | Value::ManaValueOf(spec) => {
            choose_spec_references_target_player(spec)
        }
        _ => false,
    }
}

pub(super) fn effect_references_target_player(effect: &Effect) -> bool {
    let effect = unwrap_basic_tag_wrappers(effect);
    if let Some(energy) = effect.downcast_ref::<crate::effects::EnergyCountersEffect>() {
        return value_references_target_player(&energy.count);
    }
    if let Some(ticket) = effect.downcast_ref::<crate::effects::TicketCountersEffect>() {
        return value_references_target_player(&ticket.count);
    }
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
        return value_references_target_player(&draw.count);
    }
    if let Some(gain_life) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
        return value_references_target_player(&gain_life.amount);
    }
    if let Some(lose_life) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
        return value_references_target_player(&lose_life.amount);
    }
    if let Some(pay_life) = effect.downcast_ref::<crate::effects::PayLifeEffect>() {
        return value_references_target_player(&pay_life.amount);
    }
    false
}

pub(super) fn controlled_filter_suffix(player: &PlayerFilter, verb: &str) -> String {
    match player {
        PlayerFilter::You => format!("you {verb}"),
        PlayerFilter::NotYou => format!("you don't {verb}"),
        PlayerFilter::Opponent => format!("an opponent {verb}s"),
        PlayerFilter::Any => format!("a player {verb}s"),
        PlayerFilter::Defending => format!("defending player {verb}s"),
        PlayerFilter::Attacking => format!("attacking player {verb}s"),
        PlayerFilter::DamagedPlayer
        | PlayerFilter::Specific(_)
        | PlayerFilter::Target(_)
        | PlayerFilter::IteratedPlayer
        | PlayerFilter::TaggedPlayer(_)
        | PlayerFilter::ChosenPlayer => format!("that player {verb}s"),
        other => format!("{} {verb}s", describe_player_filter(other)),
    }
}

pub(super) fn insert_filter_suffix_before_qualifier(subject: &str, suffix: &str) -> String {
    for marker in [" without ", " with ", " named ", " not named "] {
        if let Some((head, tail)) = subject.split_once(marker) {
            return format!("{head} {suffix}{marker}{tail}");
        }
    }
    format!("{subject} {suffix}")
}

pub(super) fn describe_plural_block_restriction_subject(filter: &ObjectFilter) -> Option<String> {
    if filter.card_types.as_slice() != [CardType::Creature] || filter.source {
        return None;
    }
    let mut bare = filter.clone();
    let controller = bare.controller.take();
    let owner = bare.owner.take();
    let mut subject = pluralize_noun_phrase(strip_indefinite_article(&bare.description()));
    if let Some(controller) = controller.as_ref() {
        let suffix = controlled_filter_suffix(controller, "control");
        subject = insert_filter_suffix_before_qualifier(&subject, &suffix);
    } else if let Some(owner) = owner.as_ref() {
        let suffix = controlled_filter_suffix(owner, "own");
        subject = insert_filter_suffix_before_qualifier(&subject, &suffix);
    }
    Some(capitalize_first(&subject))
}

pub(super) fn describe_destroy_then_temporary_cant_attack_block(
    destroy_effect: &Effect,
    cant_effect: &Effect,
) -> Option<String> {
    let destroy = destroy_effect.downcast_ref::<crate::effects::DestroyEffect>()?;
    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    if cant.duration != Until::EndOfTurn {
        return None;
    }
    let ChooseSpec::Object(destroy_filter) = destroy.spec.base() else {
        return None;
    };
    let destroy_controller = destroy_filter.controller.as_ref()?;
    let (restriction_filter, restriction_text) = match &cant.restriction {
        crate::effect::Restriction::Attack(filter) => (filter, "can't attack this turn"),
        crate::effect::Restriction::Block(filter) => (filter, "can't block this turn"),
        crate::effect::Restriction::AttackOrBlock(filter) => {
            (filter, "can't attack or block this turn")
        }
        _ => return None,
    };
    if restriction_filter.controller.as_ref() != Some(destroy_controller) {
        return None;
    }
    let mut subject = describe_plural_block_restriction_subject(restriction_filter)?;
    match destroy_controller {
        PlayerFilter::Defending
        | PlayerFilter::Attacking
        | PlayerFilter::Target(_)
        | PlayerFilter::AliasedTarget(_) => {
            subject = subject
                .replace("Defending player controls", "that player controls")
                .replace("defending player controls", "that player controls")
                .replace("Attacking player controls", "that player controls")
                .replace("attacking player controls", "that player controls");
        }
        _ => {}
    }
    Some(format!(
        "{}, and {} {restriction_text}",
        describe_effect(destroy_effect).trim_end_matches('.'),
        lowercase_first(&subject)
    ))
}

pub(super) fn player_filter_contains_hand_advantage_filter(filter: &PlayerFilter) -> bool {
    match filter {
        PlayerFilter::CardsInHandAtLeastMoreThanYou { .. }
        | PlayerFilter::HasMoreLifeThanYou { .. } => true,
        PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner) => {
            player_filter_contains_hand_advantage_filter(inner)
        }
        PlayerFilter::Excluding { base, excluded } => {
            player_filter_contains_hand_advantage_filter(base)
                || player_filter_contains_hand_advantage_filter(excluded)
        }
        _ => false,
    }
}

pub(super) fn describe_for_each_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            let base_description = describe_player_filter(base);
            let base_text = strip_leading_article(&base_description);
            if *count == 1 {
                format!("{base_text} who has more cards in hand than you")
            } else {
                let count_text = small_number_word(*count).unwrap_or_else(|| count.to_string());
                format!("{base_text} who has at least {count_text} more cards in hand than you")
            }
        }
        _ => describe_player_filter(filter),
    }
}

pub(super) fn describe_next_end_step_cleanup_timing(player: &PlayerFilter) -> String {
    match player {
        PlayerFilter::Any => "the next end step".to_string(),
        PlayerFilter::You => "your next end step".to_string(),
        other => format!("{} next end step", describe_possessive_player_filter(other)),
    }
}

pub(super) fn describe_choose_each_basic_land_type_then_destroy(
    effects: &[&Effect],
) -> Option<String> {
    let [plains, island, swamp, mountain, forest, destroy] = effects else {
        return None;
    };
    let expected_subtypes = [
        Subtype::Plains,
        Subtype::Island,
        Subtype::Swamp,
        Subtype::Mountain,
        Subtype::Forest,
    ];
    let mut tag: Option<&str> = None;
    for (effect, subtype) in [plains, island, swamp, mountain, forest]
        .into_iter()
        .zip(expected_subtypes)
    {
        let choose = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        if !choose.count.is_single()
            || choose.is_search
            || choose.top_only
            || choose_primary_zone(choose) != Some(Zone::Battlefield)
            || choose.filter.card_types != vec![CardType::Land]
            || choose.filter.subtypes != vec![subtype]
        {
            return None;
        }
        if let Some(existing_tag) = tag {
            if existing_tag != choose.tag.as_str() {
                return None;
            }
        } else {
            tag = Some(choose.tag.as_str());
        }
    }

    let tag = tag?;
    let destroy = destroy.downcast_ref::<crate::effects::DestroyEffect>()?;
    let destroys_tagged_lands = match &destroy.spec {
        ChooseSpec::Tagged(found) => found.as_str() == tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.card_types == vec![CardType::Land]
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == tag
                        && matches!(
                            constraint.relation,
                            crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        )
                })
        }
        _ => false,
    };
    destroys_tagged_lands
        .then_some("Choose a land of each basic land type, then destroy those lands".to_string())
}

pub(super) fn describe_distributed_damage_target(target: &ChooseSpec) -> String {
    match target {
        ChooseSpec::WithCount(inner, count)
            if matches!(inner.as_ref(), ChooseSpec::AnyTarget)
                && count.min == 1
                && count.max == Some(2) =>
        {
            "one or two targets".to_string()
        }
        ChooseSpec::WithCount(inner, count)
            if matches!(inner.as_ref(), ChooseSpec::AnyTarget)
                && count.min == 1
                && count.max == Some(3) =>
        {
            "one, two, or three targets".to_string()
        }
        ChooseSpec::WithCount(inner, count)
            if count.is_any_number()
                && matches!(inner.base(), ChooseSpec::Object(filter) if filter.tagged_constraints.iter().any(|constraint| {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        && is_implicit_reference_tag(constraint.tag.as_str())
                })) =>
        {
            let ChooseSpec::Object(filter) = inner.base() else {
                unreachable!();
            };
            let mut bare = filter.clone();
            bare.zone = None;
            bare.tagged_constraints.retain(|constraint| {
                !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && is_implicit_reference_tag(constraint.tag.as_str()))
            });
            let noun = strip_indefinite_article(&bare.description()).to_string();
            format!("any number of those {}", pluralize_noun_phrase(&noun))
        }
        ChooseSpec::WithCount(inner, count) if !inner.is_target() => {
            describe_choose_spec(&ChooseSpec::target(inner.as_ref().clone()).with_count(*count))
        }
        _ => describe_choose_spec(target),
    }
}

pub(super) fn describe_distributed_damage_amount(value: &Value) -> String {
    if let Value::ManaValueOf(spec) = value
        && matches!(spec.as_ref(), ChooseSpec::Tagged(tag) if tag.as_str().starts_with("unattach_cost_"))
    {
        return "that Equipment's mana value".to_string();
    }
    describe_value(value)
}

pub(super) fn describe_for_each_tagged_shuffle_into_owner_library(
    for_each: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    if for_each.effects.len() != 2 {
        return None;
    }
    let move_to_zone = for_each.effects[0].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Library
        || move_to_zone.to_top
        || !matches!(move_to_zone.target, ChooseSpec::Iterated)
    {
        return None;
    }
    let shuffle = for_each.effects[1].downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !matches!(
        &shuffle.player,
        PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag)) if tag == &for_each.tag
    ) {
        return None;
    }
    Some("Its owner shuffles it into their library".to_string())
}

pub(super) fn describe_source_and_blocked_creatures_top_library_shuffle(
    for_each: &crate::effects::ForEachObject,
) -> Option<String> {
    let [move_effect, shuffle_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let [source_filter, blocked_filter] = for_each.filter.any_of.as_slice() else {
        return None;
    };
    let mut expected_blocked_filter = ObjectFilter::creature();
    expected_blocked_filter.blocked_by_source = true;
    if source_filter != &ObjectFilter::source() || blocked_filter != &expected_blocked_filter {
        return None;
    }
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Library
        || !move_to_zone.to_top
        || !matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    {
        return None;
    }
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !matches!(
        &shuffle.player,
        PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag)) if tag.as_str() == "__it__"
    ) {
        return None;
    }
    Some(
        "Put this creature and each creature it's blocking on top of their owners' libraries, then those players shuffle"
            .to_string(),
    )
}

pub(super) fn describe_source_owner_shuffle_then_reveal_named_to_battlefield(
    effects: &[&Effect],
) -> Option<String> {
    fn unwrap_effect(effect: &Effect) -> &Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return unwrap_effect(&tagged.effect);
        }
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            return unwrap_effect(&tag_all.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return unwrap_effect(&with_id.effect);
        }
        effect
    }

    fn is_owner_of_source_target(player: &PlayerFilter) -> bool {
        matches!(
            player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target)
                | PlayerFilter::AliasedOwnerOf(crate::filter::ObjectRef::Target)
        )
    }

    let [
        shuffle_effect,
        consult_effect,
        move_effect,
        remainder_effect,
    ] = effects
    else {
        return None;
    };
    let shuffle = unwrap_effect(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleObjectsIntoLibraryEffect>()?;
    if !matches!(shuffle.target.base(), ChooseSpec::Source)
        || !is_owner_of_source_target(&shuffle.player)
    {
        return None;
    }

    let consult = consult_effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !is_owner_of_source_target(&consult.player)
    {
        return None;
    }
    match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => {}
        _ => return None,
    }
    let card_name =
        super::costs_and_triggers::title_case_card_name_fragment(consult.filter.name.as_ref()?);

    let move_to_zone =
        unwrap_effect(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
    {
        return None;
    }

    let remainder =
        unwrap_effect(remainder_effect).downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let standard_complement = for_each_moves_unselected_to_zone(
        remainder,
        consult.all_tag.as_str(),
        consult.match_tag.as_str(),
        Zone::Graveyard,
    );
    let iterated_subject_complement = (|| {
        if remainder.tag != consult.all_tag {
            return None;
        }
        let [conditional_effect] = remainder.effects.as_slice() else {
            return None;
        };
        let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
        if !conditional.if_true.is_empty() || conditional.if_false.len() != 1 {
            return None;
        }
        let crate::ConditionExpr::TaggedObjectMatches(tag, filter) = &conditional.condition else {
            return None;
        };
        if tag.as_str() != "__it__"
            || !filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == consult.match_tag
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::SameStableId
            })
        {
            return None;
        }
        let graveyard = unwrap_effect(&conditional.if_false[0])
            .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        (graveyard.zone == Zone::Graveyard
            && !graveyard.to_top
            && matches!(graveyard.target.base(), ChooseSpec::Iterated))
        .then_some(())
    })()
    .is_some();
    if !standard_complement && !iterated_subject_complement {
        return None;
    }

    Some(format!(
        "This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named {card_name} is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard"
    ))
}

pub(super) fn filter_has_same_name_tag(filter: &ObjectFilter, tag: &TagKey) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
    })
}

pub(super) fn describe_choose_name_exile_top_consult_hand_rest_exile(
    effects: &[&Effect],
) -> Option<String> {
    let [
        choose_name_effect,
        exile_top_effect,
        consult_effect,
        move_effect,
        remainder_effect,
    ] = effects
    else {
        return None;
    };

    let choose_name = choose_name_effect.downcast_ref::<crate::effects::ChooseCardNameEffect>()?;
    if choose_name.chooser != PlayerFilter::You {
        return None;
    }

    let exile_top = exile_top_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if exile_top.player != PlayerFilter::You {
        return None;
    }

    let consult = consult_effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !filter_has_same_name_tag(&consult.filter, &choose_name.tag)
    {
        return None;
    }
    match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => {}
        _ => return None,
    }

    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Hand
        || move_to_zone.to_top
        || !matches!(
            &move_to_zone.target,
            ChooseSpec::Tagged(tag) if tag == &consult.match_tag
        )
    {
        return None;
    }

    let remainder = remainder_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if remainder.tag != consult.all_tag {
        return None;
    }
    let [conditional_effect] = remainder.effects.as_slice() else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let condition_ok = matches!(
        &conditional.condition,
        crate::ConditionExpr::TaggedObjectMatches(tag, filter)
            if tag == &consult.match_tag
                && *filter
                    == ObjectFilter::default()
                        .same_stable_id_as_tagged(crate::tag::TagKey::from("__it__"))
    ) || matches!(
        &conditional.condition,
        crate::ConditionExpr::TaggedObjectMatches(tag, filter)
            if tag.as_str() == "__it__"
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag == consult.match_tag
                        && constraint.relation
                            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                })
    );
    if !condition_ok || !conditional.if_true.is_empty() || conditional.if_false.len() != 1 {
        return None;
    }

    let exile_remainder = unwrap_basic_tag_wrappers(&conditional.if_false[0])
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile_remainder.zone != Zone::Exile
        || !exile_remainder.to_top
        || !matches!(&exile_remainder.target, ChooseSpec::Iterated)
    {
        return None;
    }

    let count_text = match exile_top.count.unhinted() {
        Value::Fixed(count) if *count >= 0 => {
            small_number_word(*count as u32).unwrap_or_else(|| count.to_string())
        }
        _ => describe_value(&exile_top.count),
    };
    let card_noun = match exile_top.count.unhinted() {
        Value::Fixed(1) => "card",
        _ => "cards",
    };

    Some(format!(
        "Choose a card name. Exile the top {count_text} {card_noun} of your library, then reveal cards from the top of your library until you reveal a card with the chosen name. Put that card into your hand and exile all other cards revealed this way"
    ))
}

pub(crate) fn describe_chosen_name_consult_after_top_exile_effects(
    effects: &[Effect],
) -> Option<String> {
    let refs = effects.iter().collect::<Vec<_>>();
    describe_choose_name_exile_top_consult_hand_rest_exile(&refs)
}

pub(crate) fn describe_reveal_hand_choose_discard_then_random_effects(
    effects: &[Effect],
) -> Option<String> {
    let [
        look_effect,
        choose_effect,
        discard_chosen_effect,
        discard_random_effect,
    ] = effects
    else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if !look.reveal {
        return None;
    }

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose
            .filter
            .owner
            .as_ref()
            .is_none_or(|owner| describe_player_filter(owner) != describe_choose_spec(&look.target))
    {
        return None;
    }

    let discard_chosen = discard_chosen_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    let revealer = describe_choose_spec(&look.target);
    if discard_chosen.count != Value::Fixed(1)
        || discard_chosen.random
        || discard_chosen.any_number
        || describe_player_filter(&discard_chosen.player) != revealer
        || !discard_chosen.card_filter.as_ref().is_some_and(|filter| {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag == choose.tag
            })
        })
    {
        return None;
    }

    let discard_random = discard_random_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard_random.count != Value::Fixed(1)
        || !discard_random.random
        || discard_random.any_number
        || discard_random.card_filter.is_some()
        || describe_player_filter(&discard_random.player) != revealer
    {
        return None;
    }

    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    let hand = if revealer == "you" {
        "your hand"
    } else {
        "their hand"
    };
    let mut selection = choose.filter.description();
    for suffix in [
        format!(" in {revealer}'s hand"),
        " in their hand".to_string(),
        " in your hand".to_string(),
        " in hand".to_string(),
    ] {
        if let Some(rest) = selection.strip_suffix(&suffix) {
            selection = rest.trim().to_string();
            break;
        }
    }
    let selection = with_indefinite_article(&selection);
    let discard_subject = if revealer == "you" {
        "You"
    } else {
        "That player"
    };
    let discard_verb = player_verb(&discard_subject.to_ascii_lowercase(), "discard", "discards");

    Some(format!(
        "{} {} {hand}. You choose {selection} from it. {discard_subject} {discard_verb} that card, then {discard_verb} a card at random",
        capitalize_first(&revealer),
        reveal_verb
    ))
}

pub(crate) fn describe_choose_sacrifice_then_source_damage_effects(
    effects: &[Effect],
) -> Option<String> {
    let [choose_effect, sacrifice_effect, damage_effect] = effects else {
        return None;
    };

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view(sacrifice_effect)?;
    describe_choose_then_sacrifice(choose, sacrifice)?;

    let damage = damage_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.source_is_combat
        || damage.unpreventable
        || !matches!(damage.target, ChooseSpec::SourceController)
    {
        return None;
    }

    let mut sacrificed_filter = choose.filter.clone();
    sacrificed_filter.zone = None;
    if sacrificed_filter.controller == Some(PlayerFilter::You) {
        sacrificed_filter.controller = None;
    }
    let sacrificed =
        with_indefinite_article(strip_leading_article(&sacrificed_filter.description()));
    let damage_text = lowercase_first(&describe_effect(damage_effect));

    Some(format!("Sacrifice {sacrificed} and {damage_text}"))
}

pub(super) fn normalize_reflexive_sacrifice_setup(setup: String) -> String {
    if let Some(rest) = setup.strip_prefix("you sacrifice ") {
        format!("Sacrifice {rest}")
    } else {
        capitalize_first(&setup)
    }
}

pub(super) fn describe_reflexive_sacrifice_condition(
    predicate: &EffectPredicate,
) -> Option<String> {
    match predicate {
        EffectPredicate::Happened => Some("When you do".to_string()),
        EffectPredicate::HappenedNotReplaced => {
            Some("When you do and it isn't replaced".to_string())
        }
        _ => None,
    }
}

pub(super) fn describe_counted_reflexive_sacrifice_condition(
    predicate: &EffectPredicate,
    choose: &crate::effects::ChooseObjectsEffect,
    sacrifice: SacrificeView<'_>,
) -> Option<String> {
    if predicate == &EffectPredicate::Happened
        && choose.chooser == PlayerFilter::You
        && sacrifice.player == &PlayerFilter::You
        && (choose.count.dynamic_x || choose.count.max.map_or(true, |max| max > 1))
    {
        let sacrificed = pluralize_noun_phrase(&describe_sacrifice_choice_kind(choose));
        return Some(format!(
            "When you sacrifice one or more {sacrificed} this way"
        ));
    }

    describe_reflexive_sacrifice_condition(predicate)
}

pub(super) fn rewrite_sacrificed_reflexive_value_references(text: &str) -> String {
    text.replace(
        "where X is its toughness",
        "where X is the sacrificed creature's toughness",
    )
    .replace(
        "where X is its power",
        "where X is the sacrificed creature's power",
    )
    .replace(
        "where X is its mana value",
        "where X is the sacrificed creature's mana value",
    )
}

pub(super) fn describe_choose_sacrifice_then_reflexive_trigger_effects(
    effects: &[Effect],
) -> Option<String> {
    let [choose_effect, sacrifice_effect, reflexive_effect] = effects else {
        return None;
    };

    describe_choose_sacrifice_then_reflexive_trigger(
        choose_effect,
        sacrifice_effect,
        reflexive_effect,
    )
}

pub(super) fn describe_choose_sacrifice_then_reflexive_trigger_refs(
    effects: &[&Effect],
) -> Option<String> {
    let [choose_effect, sacrifice_effect, reflexive_effect] = effects else {
        return None;
    };

    describe_choose_sacrifice_then_reflexive_trigger(
        choose_effect,
        sacrifice_effect,
        reflexive_effect,
    )
}

pub(super) fn describe_choose_sacrifice_then_reflexive_trigger(
    choose_effect: &Effect,
    sacrifice_effect: &Effect,
    reflexive_effect: &Effect,
) -> Option<String> {
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let with_id = sacrifice_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let sacrifice = sacrifice_view(&with_id.effect)?;
    let setup =
        normalize_reflexive_sacrifice_setup(describe_choose_then_sacrifice(choose, sacrifice)?);

    let reflexive = reflexive_effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>()?;
    if reflexive.condition != with_id.id {
        return None;
    }
    let condition =
        describe_counted_reflexive_sacrifice_condition(&reflexive.predicate, choose, sacrifice)?;
    let triggered = lowercase_first(&describe_result_branch_effect_list(&reflexive.effects));
    let triggered = rewrite_sacrificed_reflexive_value_references(&triggered);

    Some(format!("{setup}. {condition}, {triggered}"))
}

pub(super) fn describe_add_mana_then_conditional_consult_hand_bottom(
    effects: &[&Effect],
) -> Option<String> {
    let [mana_effect, conditional_effect] = effects else {
        return None;
    };
    if mana_effect
        .downcast_ref::<crate::effects::AddManaOfAnyColorEffect>()
        .is_none()
    {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 3 {
        return None;
    }

    let consult =
        conditional.if_true[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
    {
        return None;
    }

    let move_to_hand = conditional.if_true[1].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_hand.zone != Zone::Hand || move_to_hand.to_top {
        return None;
    }
    if !matches!(
        &move_to_hand.target,
        ChooseSpec::Tagged(tag) if tag == &consult.match_tag
    ) {
        return None;
    }

    let remainder = conditional.if_true[2]
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if remainder.player != PlayerFilter::You
        || remainder.tag != consult.all_tag
        || remainder.keep_tagged.as_ref() != Some(&consult.match_tag)
    {
        return None;
    }

    let selection = describe_search_selection_with_cards(&consult.filter.description());
    let stop_text = match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch => selection,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => selection,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) => format!(
            "{} {}",
            describe_value(count),
            pluralize_noun_phrase(&selection)
        ),
    };
    let order_text = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => {
            " in an order chosen by you"
        }
    };
    let mana_text = cleanup_decompiled_text(&describe_effect(mana_effect))
        .trim_end_matches('.')
        .to_string();

    Some(format!(
        "{mana_text}. Then if {}, reveal cards from the top of your library until you reveal {stop_text}. Put that card into your hand and the rest on the bottom of your library{order_text}",
        describe_condition(&conditional.condition)
    ))
}

pub(super) fn describe_choose_then_put_counter_on_each(effects: &[&Effect]) -> Option<String> {
    let [choose_effect, for_each_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = unwrap_basic_tag_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachObject>()?;
    let [put_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let put = unwrap_basic_tag_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if !matches!(put.target, ChooseSpec::Iterated)
        || put.target_count.is_some()
        || put.distributed
        || put.amount != Value::Fixed(1)
    {
        return None;
    }
    if !for_each.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == choose.tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    }) {
        return None;
    }

    fn normalize_relative_clause_plural(selection: String) -> String {
        let mut normalized = selection;
        for (singular, plural) in [
            ("artifact", "artifacts"),
            ("battle", "battles"),
            ("card", "cards"),
            ("creature", "creatures"),
            ("enchantment", "enchantments"),
            ("land", "lands"),
            ("permanent", "permanents"),
            ("planeswalker", "planeswalkers"),
            ("spell", "spells"),
        ] {
            normalized = normalized.replace(
                &format!(" {singular} you don't controls"),
                &format!(" {plural} you don't control"),
            );
            normalized = normalized.replace(
                &format!(" {singular} you controls"),
                &format!(" {plural} you control"),
            );
        }
        normalized
    }

    let selection = normalize_relative_clause_plural(describe_choose_selection(choose));
    let counter = describe_put_counter_phrase(&put.amount, put.counter_type);
    let chooser = describe_player_filter(&choose.chooser);
    if choose.chooser == PlayerFilter::You {
        Some(format!(
            "Choose {selection} and put {counter} on each of them"
        ))
    } else {
        Some(format!(
            "{} {} {selection} and put {counter} on each of them",
            chooser,
            player_verb(&chooser, "choose", "chooses")
        ))
    }
}

pub(super) fn describe_tagged_effect_then_put_counter_on_each(
    effects: &[Effect],
) -> Option<String> {
    let [tagged_effect, for_each_effect] = effects else {
        return None;
    };
    let tagged = tagged_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .is_some()
    {
        // Pure choose/target effects carry their own surface (e.g. kicked target
        // clauses); let the structural renderers preserve it.
        return None;
    }
    let suffix = describe_put_counter_on_each_tagged_suffix(&tagged.tag, for_each_effect)?;

    Some(format!("{}. {suffix}", describe_effect(&tagged.effect)))
}

/// Preserve the sentence boundary when a whole graveyard is exiled and the
/// source then gets counters for cards in that newly exiled set. The count's
/// tagged constraint is the semantic link between the two instructions.
pub(super) fn describe_graveyard_exile_then_source_counters(effects: &[Effect]) -> Option<String> {
    let (exile_effect, counter_effect) = match effects {
        [exile_effect, counter_effect] => (exile_effect, counter_effect),
        [target_effect, exile_effect, counter_effect]
            if structural_unwrap_render_wrappers(target_effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some() =>
        {
            (exile_effect, counter_effect)
        }
        _ => return None,
    };
    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileEffect>()?;
    if exile.face_down {
        return None;
    }
    let ChooseSpec::All(graveyard_filter) = exile.spec.base() else {
        return None;
    };
    if graveyard_filter.zone != Some(Zone::Graveyard)
        || !matches!(
            graveyard_filter.owner.as_ref(),
            Some(PlayerFilter::Target(_))
        )
    {
        return None;
    }
    let mut plain_graveyard = graveyard_filter.clone();
    plain_graveyard.zone = None;
    plain_graveyard.owner = None;
    plain_graveyard.single_graveyard = false;
    if plain_graveyard != ObjectFilter::default() {
        return None;
    }

    let put = structural_unwrap_render_wrappers(counter_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if !matches!(put.target.base(), ChooseSpec::Source)
        || put.target_count.is_some()
        || put.distributed
    {
        return None;
    }
    let Value::Count(count_filter) = put.amount.unhinted() else {
        return None;
    };
    if !matches!(count_filter.zone, None | Some(Zone::Exile)) {
        return None;
    }
    let producer_tag = effect_outer_tag_through_damage_source(exile_effect);
    let counts_this_exile = count_filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && (producer_tag == Some(&constraint.tag)
                || constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    });
    if !counts_this_exile {
        return None;
    }

    let exile_clause = describe_effect(exile_effect);
    let counter_clause = describe_effect(counter_effect);
    Some(format!(
        "{}. {}",
        exile_clause.trim_end_matches('.'),
        capitalize_first(counter_clause.trim_end_matches('.'))
    ))
}

fn describe_put_counter_on_each_tagged_suffix(
    tag: &crate::TagKey,
    for_each_effect: &Effect,
) -> Option<String> {
    let for_each = unwrap_basic_tag_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachObject>()?;
    let [put_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let put = unwrap_basic_tag_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if !matches!(put.target, ChooseSpec::Iterated) || put.target_count.is_some() || put.distributed
    {
        return None;
    }
    if !for_each.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    }) {
        return None;
    }

    Some(format!(
        "Put {} on each of them",
        describe_put_counter_phrase(&put.amount, put.counter_type)
    ))
}

fn describe_distributed_damage_reciprocal_sources(effects: &[Effect]) -> Option<String> {
    let [distributed_effect, reciprocal_effect] = effects else {
        return None;
    };
    let tagged = distributed_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let distributed = tagged
        .effect
        .downcast_ref::<crate::effects::DealDistributedDamageEffect>()?;
    let target_filter = match distributed.target.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
        _ => return None,
    };
    let for_each = reciprocal_effect.downcast_ref::<crate::effects::ForEachObject>()?;
    let [constraint] = for_each.filter.tagged_constraints.as_slice() else {
        return None;
    };
    if constraint.tag != tagged.tag
        || constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    {
        return None;
    }
    let mut untagged_filter = for_each.filter.clone();
    untagged_filter.tagged_constraints.clear();
    if &untagged_filter != target_filter {
        return None;
    }
    let [execute_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let execute = execute_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    if !matches!(execute.source.base(), ChooseSpec::Iterated) {
        return None;
    }
    let damage = execute
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.source_is_combat
        || damage.unpreventable
        || !matches!(
            damage.amount.unhinted(),
            Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Iterated)
        )
    {
        return None;
    }

    let mut noun_filter = target_filter.clone();
    noun_filter.zone = None;
    let noun = pluralize_noun_phrase(strip_leading_article(&noun_filter.description()));
    Some(format!(
        "{}. Each of those {noun} deals damage equal to its power to {}",
        describe_effect(&tagged.effect).trim_end_matches('.'),
        describe_choose_spec(&damage.target)
    ))
}

fn tagged_coordinated_sequence(
    effect: &Effect,
) -> Option<(&crate::TagKey, &crate::effects::SequenceEffect)> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return tagged_coordinated_sequence(&with_id.effect);
    }
    let (tag, inner) = if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        (&tagged.tag, tagged.effect.as_ref())
    } else if let Some(tagged) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        (&tagged.tag, tagged.effect.as_ref())
    } else {
        return None;
    };
    let sequence = inner.downcast_ref::<crate::effects::SequenceEffect>()?;
    (sequence.surface == ironsmith_core::SequenceSurface::Coordinated).then_some((tag, sequence))
}

fn discard_or_sacrifice_choice(
    effect: &Effect,
) -> Option<(&PlayerFilter, &crate::effects::DiscardEffect, String)> {
    let choice = effect.downcast_ref::<crate::effects::UnlessActionEffect>()?;
    let [discard_effect] = choice.effects.as_slice() else {
        return None;
    };
    let discard = structural_unwrap_render_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.player != choice.player || discard.random || discard.any_number {
        return None;
    }

    let [choose_effect, sacrifice_effect] = choice.alternative.as_slice() else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view_unwrapped(sacrifice_effect)?;
    if choose.chooser != choice.player || sacrifice.player != &choice.player {
        return None;
    }
    let sacrifice_text = describe_choose_then_sacrifice(choose, sacrifice)?;
    let player = describe_player_filter(&choice.player);
    let verb = player_verb(&player, "sacrifice", "sacrifices");
    let prefix = format!("{player} {verb} ");
    let action = sacrifice_text.strip_prefix(&prefix)?;
    let action = action.strip_suffix(" of their choice").unwrap_or(action);
    Some((&choice.player, discard, action.to_string()))
}

fn joint_player_surface(player: &PlayerFilter) -> String {
    match player {
        PlayerFilter::Defending => "defending player".to_string(),
        PlayerFilter::Attacking => "attacking player".to_string(),
        _ => describe_player_filter(player),
    }
}

fn tagged_graveyard_count_subject(value: &Value, tag: &crate::TagKey) -> Option<String> {
    let Value::Count(filter) = value.unhinted() else {
        return None;
    };
    if filter.zone != Some(Zone::Graveyard)
        || filter.controller.is_some()
        || filter.owner.is_some()
        || filter.single_graveyard
        || filter.tagged_constraints.len() != 1
        || !filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return None;
    }

    let mut subject_filter = filter.clone();
    subject_filter.zone = None;
    subject_filter.tagged_constraints.clear();
    let subject = strip_leading_article(&subject_filter.description())
        .trim()
        .to_string();
    (!subject.is_empty() && !subject.contains("tagged object")).then_some(subject)
}

/// Compact a shared, affected-object-tagged pair of player choices followed by
/// a count of cards those choices put into graveyards. The shared tag is what
/// proves the final "this way" relationship.
fn describe_joint_discard_or_sacrifice_then_draw(effects: &[Effect]) -> Option<String> {
    let [choice_effect, draw_effect] = effects else {
        return None;
    };
    let (affected_tag, sequence) = tagged_coordinated_sequence(choice_effect)?;
    let [first_choice, second_choice] = sequence.effects.as_slice() else {
        return None;
    };
    let (first_player, first_discard, first_sacrifice) = discard_or_sacrifice_choice(first_choice)?;
    let (second_player, second_discard, second_sacrifice) =
        discard_or_sacrifice_choice(second_choice)?;
    if first_player == second_player
        || first_discard.count != second_discard.count
        || first_discard.card_filter != second_discard.card_filter
        || first_sacrifice != second_sacrifice
    {
        return None;
    }

    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let counted_subject = tagged_graveyard_count_subject(&draw.count, affected_tag)?;
    let choice_subject = capitalize_first(&format!(
        "{} and {}",
        joint_player_surface(first_player),
        joint_player_surface(second_player)
    ));
    let discarded =
        describe_discard_count(&first_discard.count, first_discard.card_filter.as_ref());
    let draw_player = describe_player_filter(&draw.player);
    let draw_verb = player_verb(&draw_player, "draw", "draws");
    Some(format!(
        "{choice_subject} each discard {discarded} or sacrifice {first_sacrifice}. {} {draw_verb} a card for each {counted_subject} put into a graveyard this way",
        capitalize_first(&draw_player)
    ))
}

pub(super) fn describe_target_player_reveal_top_may_put_matching_rest_bottom(
    effects: &[Effect],
) -> Option<String> {
    let [
        target_effect,
        reveal_effect,
        choose_effect,
        move_effect,
        remainder_effect,
    ] = effects
    else {
        return None;
    };
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let selected_player = choose_spec_player_filter(&target.target)?;
    if !matches!(selected_player, PlayerFilter::Target(_)) {
        return None;
    }

    let reveal = reveal_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = move_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [move_one] = for_each.effects.as_slice() else {
        return None;
    };
    let move_one = structural_unwrap_render_wrappers(move_one)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let remainder = remainder_effect
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;

    if !reveal.reveal
        || !player_filters_refer_to_same_player(&selected_player, &reveal.player)
        || choose.chooser != PlayerFilter::You
        || choose.count.min != 0
        || choose.count.max != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == reveal.tag
        })
        || for_each.tag != choose.tag
        || !matches!(move_one.target.base(), ChooseSpec::Iterated)
        || move_one.zone != Zone::Battlefield
        || move_one.enters_tapped
        || move_one.enters_attacking
        || move_one.enters_face_down
        || move_one.battlefield_controller != crate::effects::BattlefieldController::You
        || move_one.verb_surface != ironsmith_core::MoveToZoneVerbSurface::Put
        || remainder.tag != reveal.tag
        || remainder.keep_tagged.as_ref() != Some(&choose.tag)
        || remainder.order != crate::effects::consult_helpers::LibraryBottomOrder::Random
        || !player_filters_refer_to_same_player(&selected_player, &remainder.player)
    {
        return None;
    }

    let mut display_filter = choose.filter.clone();
    display_filter.zone = None;
    display_filter.owner = None;
    display_filter.tagged_constraints.clear();
    let selection = with_indefinite_article(&describe_nonbattlefield_card_filter_without_zone(
        &display_filter,
        Zone::Library,
    ));
    let player = capitalize_first(&describe_player_filter(&selected_player));
    let reveal_count = match reveal.count.unhinted() {
        Value::Fixed(1) => "the top card".to_string(),
        count => format!("the top {} cards", describe_value(count)),
    };

    Some(format!(
        "{player} reveals {reveal_count} of their library. You may put {selection} from among them onto the battlefield under your control. That player puts the rest on the bottom of their library in a random order"
    ))
}

fn describes_first_strike_grant_to_tag(effect: &Effect, tag: &crate::TagKey) -> bool {
    let Some(apply) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::ApplyContinuousEffect>()
    else {
        return false;
    };
    if apply.until != Until::EndOfTurn
        || apply.condition.is_some()
        || !apply.additional_modifications.is_empty()
        || !apply.runtime_modifications.is_empty()
        || !matches!(apply.target_spec.as_ref().and_then(choose_spec_tag), Some(target) if target == tag)
    {
        return false;
    }
    match &apply.modification {
        Some(crate::continuous::Modification::AddAbility(ability)) => {
            ability.id() == crate::static_abilities::StaticAbilityId::FirstStrike
        }
        Some(crate::continuous::Modification::AddAbilityGeneric(ability)) => matches!(
            &ability.kind,
            crate::ability::AbilityKind::Static(ability)
                if ability.id() == crate::static_abilities::StaticAbilityId::FirstStrike
        ),
        _ => false,
    }
}

fn describe_linked_attack_group_first_strike_reward(effects: &[Effect]) -> Option<String> {
    let [capture_effect, grant_effect, draw_effect, schedule_effect] = effects else {
        return None;
    };
    let capture = capture_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    if capture.tag.as_str() != ironsmith_core::ATTACKING_GROUP_TAG
        || !capture.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == capture.tag
        })
        || !describes_first_strike_grant_to_tag(grant_effect, &capture.tag)
    {
        return None;
    }
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You || !matches!(draw.count.unhinted(), Value::Fixed(1)) {
        return None;
    }
    let schedule =
        schedule_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    let damage = schedule
        .trigger
        .downcast_ref::<crate::triggers::DealsCombatDamageToPlayerTrigger>()?;
    if schedule.one_shot
        || schedule.start_next_turn
        || !schedule.until_end_of_combat
        || schedule.target_tag.as_ref() != Some(&capture.tag)
        || damage.player != PlayerFilter::Any
    {
        return None;
    }
    let delayed = schedule.effects.flattened_default_effects();
    let [counter_effect] = delayed else {
        return None;
    };
    let counters = unwrap_basic_tag_wrappers(counter_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    let counter_target_tag = choose_spec_tag(&counters.target)?;
    if counters.counter_type != crate::object::CounterType::PlusOnePlusOne
        || !matches!(counters.amount.unhinted(), Value::Fixed(1))
        || counters.distributed
        || counter_target_tag == &capture.tag
    {
        return None;
    }

    Some(
        "those creatures gain first strike until end of turn, then draw a card. Whenever either of those creatures deals combat damage to a player this combat, put a +1/+1 counter on it"
            .to_string(),
    )
}

fn tagged_move_to_zone<'a>(
    effect: &'a Effect,
    tag: &TagKey,
    zone: Zone,
) -> Option<&'a crate::effects::MoveToZoneEffect> {
    let move_to_zone =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let target_matches = match move_to_zone.target.base() {
        ChooseSpec::Tagged(candidate) => candidate == tag,
        ChooseSpec::All(filter) => {
            let matching_constraints = filter
                .tagged_constraints
                .iter()
                .filter(|constraint| {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        && constraint.tag == *tag
                })
                .count();
            let mut remainder = filter.clone();
            remainder.zone = None;
            remainder.tagged_constraints.clear();
            matching_constraints == 1
                && filter.tagged_constraints.len() == 1
                && remainder == ObjectFilter::default()
        }
        _ => false,
    };
    (move_to_zone.zone == zone && target_matches).then_some(move_to_zone)
}

pub(super) fn describe_search_reveal_conditional_battlefield_or_hand(
    effects: &[Effect],
) -> Option<String> {
    let [
        choose_effect,
        reveal_effect,
        conditional_effect,
        shuffle_effect,
    ] = effects
    else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.is_search
        || choose.reveal
        || choose.count != crate::effect::ChoiceCount::exactly(1)
        || choose.zone != Some(Zone::Library)
        || !choose.additional_zones.is_empty()
        || choose.filter.zone != Some(Zone::Library)
        || choose.chooser != PlayerFilter::You
    {
        return None;
    }
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if reveal.tag != choose.tag {
        return None;
    }
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::PlayerControls { player, filter } = &conditional.condition else {
        return None;
    };
    if *player != PlayerFilter::You
        || filter.zone != Some(Zone::Battlefield)
        || filter.controller != Some(PlayerFilter::You)
        || !filter.card_types.contains(&CardType::Land)
    {
        return None;
    }
    let controlled_name = filter.name.as_ref()?;
    let [with_id_effect, declined_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [battlefield_effect] = may.effects.as_slice() else {
        return None;
    };
    let battlefield_move = tagged_move_to_zone(battlefield_effect, &choose.tag, Zone::Battlefield)?;
    if !battlefield_move.enters_tapped {
        return None;
    }
    let declined = declined_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if declined.condition != with_id.id
        || declined.predicate != crate::effect::EffectPredicate::DidNotHappen
        || !declined.else_.is_empty()
    {
        return None;
    }
    let [declined_move] = declined.then.as_slice() else {
        return None;
    };
    tagged_move_to_zone(declined_move, &choose.tag, Zone::Hand)?;
    let [otherwise_move] = conditional.if_false.as_slice() else {
        return None;
    };
    tagged_move_to_zone(otherwise_move, &choose.tag, Zone::Hand)?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle.player != PlayerFilter::You || shuffle.target_spec.is_some() {
        return None;
    }

    let rendered_search = describe_effect(choose_effect);
    let search = rendered_search
        .trim_end_matches('.')
        .strip_prefix("You ")
        .or_else(|| rendered_search.trim_end_matches('.').strip_prefix("you "))?
        .to_string();
    let named = super::costs_and_triggers::title_case_card_name_fragment(controlled_name);
    Some(format!(
        "{}. You may put that card onto the battlefield tapped if you control a land named {named}. Otherwise, put that card into your hand. Then shuffle",
        capitalize_first(&format!("{search} and reveal it"))
    ))
}

fn source_exiled_tag(tag: &TagKey) -> bool {
    tag.as_str() == crate::tag::SOURCE_EXILED_TAG
}

fn plain_creature_card_condition(filter: &ObjectFilter) -> bool {
    if filter.card_types.as_slice() != [CardType::Creature] {
        return false;
    }
    let mut base = filter.clone();
    base.card_types.clear();
    base == ObjectFilter::default()
}

pub(super) fn describe_turn_source_exiled_face_up_then_lose_mana_value(
    effects: &[Effect],
) -> Option<String> {
    let [turn_effect, conditional_effect] = effects else {
        return None;
    };
    let turn = structural_unwrap_render_wrappers(turn_effect)
        .downcast_ref::<crate::effects::TurnFaceUpEffect>()?;
    let ChooseSpec::Tagged(turn_tag) = turn.target.base() else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::TaggedObjectMatches(condition_tag, filter) =
        &conditional.condition
    else {
        return None;
    };
    let [lose_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let lose = structural_unwrap_render_wrappers(lose_effect)
        .downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if !source_exiled_tag(turn_tag)
        || condition_tag != turn_tag
        || !plain_creature_card_condition(filter)
        || !conditional.if_false.is_empty()
        || !matches!(lose.player.base(), ChooseSpec::Player(PlayerFilter::You))
        || !value_is_source_exiled_mana_value(&lose.amount)
    {
        return None;
    }
    Some(
        "Turn the exiled card face up. If it's a creature card, you lose life equal to its mana value"
            .to_string(),
    )
}

pub(super) fn describe_source_exiled_creature_may_battlefield_else_hand(
    effects: &[Effect],
) -> Option<String> {
    if let [creature_conditional_effect] = effects {
        let creature_conditional =
            creature_conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
        let crate::effect::Condition::TaggedObjectMatches(source_tag, creature_filter) =
            &creature_conditional.condition
        else {
            return None;
        };
        let [with_id_effect, declined_effect] = creature_conditional.if_true.as_slice() else {
            return None;
        };
        let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
        let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
        let [battlefield_effect] = may.effects.as_slice() else {
            return None;
        };
        let battlefield = tagged_move_to_zone(battlefield_effect, source_tag, Zone::Battlefield)?;

        let declined = declined_effect.downcast_ref::<crate::effects::IfEffect>()?;
        let [declined_hand_effect] = declined.then.as_slice() else {
            return None;
        };
        let declined_hand = tagged_move_to_zone(declined_hand_effect, source_tag, Zone::Hand)?;
        let [noncreature_hand_effect] = creature_conditional.if_false.as_slice() else {
            return None;
        };
        let noncreature_hand =
            tagged_move_to_zone(noncreature_hand_effect, source_tag, Zone::Hand)?;

        if !source_exiled_tag(source_tag)
            || !plain_creature_card_condition(creature_filter)
            || may.decider != Some(PlayerFilter::You)
            || battlefield.enters_tapped
            || battlefield.enters_face_down
            || declined.condition != with_id.id
            || declined.predicate != crate::effect::EffectPredicate::WasDeclined
            || !declined.else_.is_empty()
            || declined_hand.enters_tapped
            || declined_hand.enters_face_down
            || noncreature_hand.enters_tapped
            || noncreature_hand.enters_face_down
        {
            return None;
        }

        return Some(
            "You may put the exiled card onto the battlefield if it's a creature card. If you don't put it onto the battlefield, put it into its owner's hand"
                .to_string(),
        );
    }

    let [creature_conditional_effect, fallback_conditional_effect] = effects else {
        return None;
    };
    let creature_conditional =
        creature_conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::TaggedObjectMatches(source_tag, creature_filter) =
        &creature_conditional.condition
    else {
        return None;
    };
    let [may_effect] = creature_conditional.if_true.as_slice() else {
        return None;
    };
    let may = structural_unwrap_render_wrappers(may_effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    let [battlefield_effect] = may.effects.as_slice() else {
        return None;
    };
    let battlefield = tagged_move_to_zone(battlefield_effect, source_tag, Zone::Battlefield)?;

    let fallback =
        fallback_conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::Not(inner) = &fallback.condition else {
        return None;
    };
    let crate::effect::Condition::PlayerTaggedObjectMatches {
        player,
        tag: fallback_tag,
        filter: battlefield_filter,
    } = inner.as_ref()
    else {
        return None;
    };
    let [hand_effect] = fallback.if_true.as_slice() else {
        return None;
    };
    let hand = tagged_move_to_zone(hand_effect, source_tag, Zone::Hand)?;

    if !source_exiled_tag(source_tag)
        || fallback_tag != source_tag
        || !plain_creature_card_condition(creature_filter)
        || !creature_conditional.if_false.is_empty()
        || may.decider != Some(PlayerFilter::You)
        || battlefield.enters_tapped
        || battlefield.enters_face_down
        || *player != PlayerFilter::You
        || battlefield_filter.zone != Some(Zone::Battlefield)
        || !fallback.if_false.is_empty()
        || hand.enters_tapped
        || hand.enters_face_down
    {
        return None;
    }

    Some(
        "You may put the exiled card onto the battlefield if it's a creature card. If you don't put it onto the battlefield, put it into its owner's hand"
            .to_string(),
    )
}

/// Preserve the matched collection's typed noun when a variable-count
/// library consult moves that whole collection in a follow-up sentence. The
/// shared tag proves that the moved objects are exactly the cards satisfying
/// the consult filter; the plural surface proves this is a collection rather
/// than the ordinary single-match "that card" shape.
fn describe_consult_then_move_matched_collection(effects: &[Effect]) -> Option<String> {
    let [consult_effect, move_effect] = effects else {
        return None;
    };
    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) = &consult.stop_rule else {
        return None;
    };
    if consult.player != PlayerFilter::You
        || consult.max_exposed.is_some()
        || matches!(count.unhinted(), Value::Fixed(1))
        || !move_to_zone.target_plural_surface
        || move_to_zone.actor_surface.is_some()
        || move_to_zone.verb_surface != ironsmith_core::MoveToZoneVerbSurface::Put
        || !matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag == &consult.match_tag
        )
    {
        return None;
    }

    let rendered_consult = describe_effect(consult_effect);
    let consult_text = rendered_consult
        .trim()
        .trim_end_matches('.')
        .strip_prefix("you ")
        .or_else(|| {
            rendered_consult
                .trim()
                .trim_end_matches('.')
                .strip_prefix("You ")
        })?;
    let selection = describe_library_consult_selection_with_cards(&consult.filter);
    let moved_reference = format!("those {}", pluralize_noun_phrase(&selection));
    let rendered_move = describe_effect(&Effect::new(move_to_zone.clone()));
    let move_text = rendered_move.trim().trim_end_matches('.');
    let move_tail = move_text
        .strip_prefix("Put them")
        .or_else(|| move_text.strip_prefix("put them"))
        .or_else(|| move_text.strip_prefix("Put those cards"))
        .or_else(|| move_text.strip_prefix("put those cards"))?;

    Some(format!(
        "{}. Put {moved_reference}{move_tail}",
        capitalize_first(consult_text)
    ))
}

/// Render a dynamic library search followed by moving the complete searched
/// collection and shuffling.  Two equivalent lowering shapes exist: newer
/// search sentences move the tagged collection directly, while older comma
/// chains iterate the tag inside a `SequenceEffect`.  Keeping the renderer
/// keyed to the shared tag and dynamic count avoids treating an arbitrary
/// search/move pair as the authored "those cards" collection.
fn describe_dynamic_search_move_collection(effects: &[Effect]) -> Option<String> {
    fn normalize_collection_reference(mut text: String, comma_join: bool) -> String {
        text = text
            .replace(". Put them", ". Put those cards")
            .replace(", put them", ", put those cards")
            .replace(", Put them", ", put those cards");
        if comma_join {
            text = text
                .replace(". Put those cards", ", put those cards")
                .replace(". put those cards", ", put those cards");
        }
        text
    }

    fn dynamic_collection_search(choose: &crate::effects::ChooseObjectsEffect) -> bool {
        choose.is_search
            && choose.count.dynamic_x
            && choose.count_value.is_some()
            && !choose.count.is_single()
            && choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library))
    }

    if let [choose_effect, move_effect, shuffle_effect] = effects {
        let choose = structural_unwrap_render_wrappers(choose_effect)
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        let move_to_zone = structural_unwrap_render_wrappers(move_effect)
            .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
            .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
        if dynamic_collection_search(choose) {
            let compact =
                describe_search_choose_then_move(choose, None, move_to_zone, Some(shuffle))?;
            return Some(normalize_collection_reference(compact, false));
        }
    }

    let sequence = match effects {
        [sequence_effect] => structural_unwrap_render_wrappers(sequence_effect)
            .downcast_ref::<crate::effects::SequenceEffect>()?,
        _ => return None,
    };
    let [choose_effect, move_effect, shuffle_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !dynamic_collection_search(choose) {
        return None;
    }
    let compact = describe_search_choose_for_each(choose, for_each, Some(shuffle), false)?;
    Some(normalize_collection_reference(compact, true))
}

/// Render searches accumulated across a per-object loop as one linked
/// collection. This is the executable shape for Oracle's "for each ... search
/// ... Put those cards" wording: every inner search appends to the same tag,
/// and the move happens once after the loop.
fn describe_iterated_same_name_search_collection(effects: &[Effect]) -> Option<String> {
    let [for_each_effect, move_effect, shuffle_effect] = effects else {
        return None;
    };
    let for_each = structural_unwrap_render_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachObject>()?;
    let [inner] = for_each.effects.as_slice() else {
        return None;
    };
    let (search, optional) = if let Some(search) = structural_unwrap_render_wrappers(inner)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>(
    ) {
        (search, false)
    } else {
        let may =
            structural_unwrap_render_wrappers(inner).downcast_ref::<crate::effects::MayEffect>()?;
        let [search_effect] = may.effects.as_slice() else {
            return None;
        };
        (
            structural_unwrap_render_wrappers(search_effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
            true,
        )
    };
    if !search.is_search
        || choose_search_zones(search)? != vec![Zone::Library]
        || !search.count.is_single()
        || !search.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "__it__"
                && constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
        })
    {
        return None;
    }

    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &search.tag)
        || move_to_zone.enters_face_down
        || move_to_zone.enters_attacking
    {
        return None;
    }
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    let iterated_noun = if for_each.filter.card_types.len() == 1 {
        describe_iterated_object_reference_noun(&for_each.filter)
    } else if matches!(for_each.filter.zone, Some(Zone::Battlefield) | None) {
        "permanent"
    } else {
        "card"
    };
    let subject = describe_for_each_object_filter_subject(&for_each.filter);
    let mut search_text = describe_effect(&Effect::new(search.clone()));
    search_text = lowercase_first(search_text.trim().trim_end_matches('.'));
    search_text = search_text
        .replace(
            "with the same name as that card",
            &format!("with the same name as that {iterated_noun}"),
        )
        .replace(
            "with the same name as that object",
            &format!("with the same name as that {iterated_noun}"),
        );

    if matches!(
        &for_each.filter.controller,
        Some(PlayerFilter::Target(inner)) if **inner == PlayerFilter::Opponent
    ) {
        search_text = search_text
            .replace("target opponent's library", "that player's library")
            .replace("the target opponent's library", "that player's library");
    }
    if optional && !search_text.starts_with("you may ") {
        search_text = if let Some(rest) = search_text.strip_prefix("you ") {
            format!("you may {rest}")
        } else {
            format!("you may {search_text}")
        };
    }

    let controller = match move_to_zone.battlefield_controller {
        crate::effects::BattlefieldController::You => " under your control",
        crate::effects::BattlefieldController::Owner => " under its owner's control",
        crate::effects::BattlefieldController::Preserve => "",
    };
    let tapped = if move_to_zone.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let shuffle_text = match &shuffle.player {
        PlayerFilter::You => "then shuffle".to_string(),
        PlayerFilter::Target(_)
        | PlayerFilter::AliasedTarget(_)
        | PlayerFilter::ControllerOf(_)
        | PlayerFilter::AliasedControllerOf(_) => "then that player shuffles".to_string(),
        _ => format!(
            "then {}",
            lowercase_first(describe_effect(shuffle_effect).trim().trim_end_matches('.'))
        ),
    };

    Some(format!(
        "For each {subject}, {search_text}. Put those cards onto the battlefield{tapped}{controller}, {shuffle_text}"
    ))
}

fn describe_for_players_choose_move_then_subtypes(effects: &[Effect]) -> Option<String> {
    let [for_players_effect, move_effect, subtype_effect] = effects else {
        return None;
    };
    let for_players = structural_unwrap_render_wrappers(for_players_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let [choose_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let (base, chosen_tag) = if let Some(choose) = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>(
    ) {
        (
            describe_for_players_choose_then_move_to_battlefield(for_players, move_to_zone)?,
            &choose.tag,
        )
    } else {
        // "Choose up to one target ... for each player" is an authored
        // target declaration rather than an ordinary ChooseObjects effect.
        // The direct tag joins that declaration to the later collection move
        // and type-changing effect just as it does for non-target choices.
        let target_only = structural_unwrap_render_wrappers(choose_effect)
            .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
        let chosen_tag = direct_wrapped_effect_tag(choose_effect)?;
        let ChooseSpec::Object(filter) = target_only.target.base() else {
            return None;
        };
        let count = target_only.target.count();
        if for_players.filter != PlayerFilter::Any
            || for_players.starting_with_controller
            || for_players.stop_after_first_happened
            || !target_only.explicit_declaration
            || !target_only.target.is_target()
            || count.min != 0
            || count.max != Some(1)
            || filter.zone != Some(Zone::Graveyard)
            || filter.owner != Some(PlayerFilter::IteratedPlayer)
            || filter.card_types.as_slice() != [CardType::Creature]
            || move_to_zone.zone != Zone::Battlefield
            || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::You
            || !move_to_battlefield_uses_chosen_tag(move_to_zone, chosen_tag.as_str())
        {
            return None;
        }
        let choice = describe_effect(for_players_effect)
            .trim()
            .trim_end_matches('.')
            .to_string();
        let tapped = if move_to_zone.enters_tapped {
            " tapped"
        } else {
            ""
        };
        let attacking = if move_to_zone.enters_attacking {
            " and attacking"
        } else {
            ""
        };
        (
            format!(
                "{choice}. Put those cards onto the battlefield{tapped}{attacking} under your control"
            ),
            chosen_tag,
        )
    };
    let apply = structural_unwrap_render_wrappers(subtype_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !apply_continuous_is_forever_tagged(apply, chosen_tag)
        || !apply.additional_modifications.is_empty()
        || !apply.runtime_modifications.is_empty()
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddSubtypes(subtypes)) = &apply.modification else {
        return None;
    };
    if subtypes.is_empty() {
        return None;
    }
    let subtype_words = pluralize_noun_phrase(
        &subtypes
            .iter()
            .map(|subtype| subtype.display_name())
            .collect::<Vec<_>>()
            .join(" "),
    );
    Some(format!(
        "{base}. They're {subtype_words} in addition to their other types"
    ))
}

/// Render a collection search grouped by the controllers of objects exiled by
/// the first effect. The two controller loops prove both the search actor and
/// the set of players that shuffle; the shared search tag proves the single
/// intervening move is the union of all cards found this way.
fn describe_controller_grouped_exile_search_collection(effects: &[Effect]) -> Option<String> {
    let [
        exile_effect,
        search_loop_effect,
        move_effect,
        shuffle_loop_effect,
    ] = effects
    else {
        return None;
    };
    let exile_tag = direct_wrapped_effect_tag(exile_effect)?;
    let exile = structural_unwrap_render_wrappers(exile_effect);
    let exile_filter =
        if let Some(move_to_zone) = exile.downcast_ref::<crate::effects::MoveToZoneEffect>() {
            if move_to_zone.zone != Zone::Exile {
                return None;
            }
            match move_to_zone.target.base() {
                ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
                _ => return None,
            }
        } else if let Some(exile) = exile.downcast_ref::<crate::effects::ExileEffect>() {
            match exile.spec.base() {
                ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
                _ => return None,
            }
        } else {
            return None;
        };
    if exile_filter.card_types.as_slice() != [CardType::Creature]
        || exile_filter.controller != Some(PlayerFilter::NotYou)
    {
        return None;
    }

    let search_loop = structural_unwrap_render_wrappers(search_loop_effect)
        .downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()?;
    let [search_effect] = search_loop.effects.as_slice() else {
        return None;
    };
    let search = structural_unwrap_render_wrappers(search_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if &search_loop.tag != exile_tag
        || !search.is_search
        || search.zone != Some(Zone::Library)
        || !search.additional_zones.is_empty()
        || !search.count.is_up_to_dynamic_x()
        || search.count_value.as_ref().map(Value::unhinted) != Some(&Value::TaggedCount)
        || search.search_mode != crate::effect::SearchSelectionMode::Optional
        || search.chooser != PlayerFilter::IteratedPlayer
        || search.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || search.filter.card_types.as_slice() != [CardType::Land]
        || search.filter.supertypes.as_slice() != [Supertype::Basic]
    {
        return None;
    }

    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || !move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &search.tag)
    {
        return None;
    }

    let shuffle_loop = structural_unwrap_render_wrappers(shuffle_loop_effect)
        .downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()?;
    let [shuffle_effect] = shuffle_loop.effects.as_slice() else {
        return None;
    };
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if &shuffle_loop.tag != exile_tag
        || shuffle.player != PlayerFilter::IteratedPlayer
        || shuffle.target_spec.is_some()
    {
        return None;
    }

    let exile_clause = describe_effect(exile_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Some(format!(
        "{exile_clause}. For each creature exiled this way, its controller searches their library for a basic land card. Those players put those cards onto the battlefield tapped, then shuffle"
    ))
}

fn describe_chosen_type_consult_move_matches_shuffle_remainder(
    effects: &[Effect],
) -> Option<String> {
    let [
        choose_type_effect,
        consult_effect,
        move_effect,
        shuffle_effect,
    ] = effects
    else {
        return None;
    };
    let choose_type = structural_unwrap_render_wrappers(choose_type_effect)
        .downcast_ref::<crate::effects::ChooseCreatureTypeEffect>()?;
    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) = &consult.stop_rule else {
        return None;
    };
    let Value::Count(count_filter) = count.unhinted() else {
        return None;
    };
    if choose_type.chooser != PlayerFilter::You
        || consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || consult.max_exposed.is_some()
        || consult.filter.card_types.as_slice() != [CardType::Creature]
        || !consult.filter.chosen_creature_type
        || count_filter.zone != Some(Zone::Battlefield)
        || count_filter.controller != Some(PlayerFilter::You)
        || count_filter.card_types.as_slice() != [CardType::Creature]
        || !count_filter.chosen_creature_type
        || move_to_zone.zone != Zone::Battlefield
        || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
        || move_to_zone.enters_tapped
        || move_to_zone.enters_face_down
        || move_to_zone.enters_attacking
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || shuffle.player != PlayerFilter::You
    {
        return None;
    }

    let rendered_consult = describe_effect(consult_effect);
    let consult_text = rendered_consult
        .trim()
        .trim_end_matches('.')
        .strip_prefix("You ")
        .or_else(|| {
            rendered_consult
                .trim()
                .trim_end_matches('.')
                .strip_prefix("you ")
        })?;
    Some(format!(
        "Choose a creature type. {}. Put those cards onto the battlefield, then shuffle the rest of the revealed cards into your library",
        capitalize_first(consult_text)
    ))
}

fn describe_ordered_choose_all_then_relative_pump(effects: &[Effect]) -> Option<String> {
    let [choose_effect, loop_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose
        .count_value
        .as_ref()
        .is_some_and(|value| value.has_surface_hint(ValueSurfaceHint::ChooseAllInOrder))
    {
        return None;
    }
    let loop_effect = loop_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if loop_effect.tag != choose.tag {
        return None;
    }
    let [pump_effect] = loop_effect.effects.as_slice() else {
        return None;
    };
    let pump = pump_effect.downcast_ref::<crate::effects::ModifyPowerToughnessForEachEffect>()?;
    if !pump
        .count
        .has_surface_hint(ValueSurfaceHint::CreaturesChosenBeforeIt)
    {
        return None;
    }

    let selection = describe_choose_selection(choose);
    let chosen_noun = if choose.filter.card_types == [CardType::Creature] {
        "creatures"
    } else if choose.filter.card_types == [CardType::Artifact] {
        "artifacts"
    } else if choose.filter.card_types == [CardType::Enchantment] {
        "enchantments"
    } else if choose.filter.card_types == [CardType::Land] {
        "lands"
    } else {
        "objects"
    };
    let each_text = describe_create_for_each_count(&pump.count)?;
    let additional = if pump
        .count
        .has_surface_hint(ValueSurfaceHint::AdditionalPowerToughnessModifier)
    {
        "an additional "
    } else {
        ""
    };
    Some(format!(
        "Choose {selection}. Each of those {chosen_noun} gets {additional}{}/{} {} for each {each_text}",
        describe_signed_i32(pump.power_per),
        describe_signed_i32(pump.toughness_per),
        describe_until(&pump.duration),
    ))
}

fn quantified_player_subject(filter: &PlayerFilter) -> String {
    describe_for_players_subject(filter)
        .map(str::to_string)
        .unwrap_or_else(|| {
            let described = describe_for_each_player_filter(filter);
            format!("Each {}", strip_leading_article(&described))
        })
}

fn strip_shared_where_x_clause(text: &str) -> &str {
    text.split_once(", where X is ")
        .map_or(text, |(head, _)| head)
        .trim_end_matches('.')
}

/// A `ForPlayersEffect` is the executable form of an authored quantified
/// player subject. Keep that subject on the action instead of exposing the
/// implementation loop as "for each ..., that player ...".
fn describe_quantified_player_damage(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.starting_with_controller || for_players.stop_after_first_happened {
        return None;
    }
    let [effect] = for_players.effects.as_slice() else {
        return None;
    };
    let damage = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(
        damage.target.base(),
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) {
        return None;
    }

    let subject = lowercase_first(&quantified_player_subject(&for_players.filter));
    let rendered = describe_effect(effect);
    let rendered = rendered.trim().trim_end_matches('.');
    let source = rendered.split_once(" deals ").map(|(source, _)| source);
    if source.is_none() && !rendered.starts_with("Deal ") {
        return None;
    }

    let (amount, where_x) = describe_damage_amount_clause(&damage.amount);
    let (amount, where_x) = match damage.amount.unhinted() {
        Value::Devotion { .. }
        | Value::DevotionToChosenColor(_)
        | Value::BasicLandTypesAmong(_) => (
            format!(
                "damage equal to {}",
                describe_value(damage.amount.unhinted())
            ),
            None,
        ),
        _ => (amount, where_x),
    };
    let damage = source.map_or_else(
        || format!("Deal {amount} to {subject}"),
        |source| format!("{source} deals {amount} to {subject}"),
    );
    Some(format!(
        "{damage}{}",
        where_x.map_or_else(String::new, |basis| format!(", where X is {basis}"))
    ))
}

fn damage_amount_as_life(amount: &str) -> Option<String> {
    amount
        .strip_suffix(" damage")
        .map(|head| format!("{head} life"))
        .or_else(|| {
            amount
                .strip_prefix("damage equal to ")
                .map(|tail| format!("life equal to {tail}"))
        })
}

fn describe_quantified_damage_then_controller_gain(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.starting_with_controller || for_players.stop_after_first_happened {
        return None;
    }
    let [damage_effect, gain_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let damage = structural_unwrap_render_wrappers(damage_effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    let gain = structural_unwrap_render_wrappers(gain_effect)
        .downcast_ref::<crate::effects::GainLifeEffect>()?;
    if damage.amount != gain.amount
        || !matches!(
            damage.target.base(),
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
        || !matches!(gain.player.base(), ChooseSpec::Player(PlayerFilter::You))
    {
        return None;
    }

    let subject = lowercase_first(&quantified_player_subject(&for_players.filter));
    let rendered = describe_effect(damage_effect);
    let rendered = rendered.trim().trim_end_matches('.');
    let source = rendered.split_once(" deals ").map(|(source, _)| source);
    if source.is_none() && !rendered.starts_with("Deal ") {
        return None;
    }
    let (damage_amount, where_x) = describe_damage_amount_clause(&damage.amount);
    let life_amount = damage_amount_as_life(&damage_amount)?;
    let damage = source.map_or_else(
        || format!("Deal {damage_amount} to {subject}"),
        |source| format!("{source} deals {damage_amount} to {subject}"),
    );
    Some(format!(
        "{damage} and you gain {life_amount}{}",
        where_x.map_or_else(String::new, |basis| format!(", where X is {basis}"))
    ))
}

fn describe_quantified_nested_shared_action(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.starting_with_controller || for_players.stop_after_first_happened {
        return None;
    }
    let [first_effect, nested_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let nested = nested_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if nested.filter != for_players.filter
        || nested.starting_with_controller
        || nested.stop_after_first_happened
    {
        return None;
    }
    let [nested_damage_effect] = nested.effects.as_slice() else {
        return None;
    };
    let nested_damage = structural_unwrap_render_wrappers(nested_damage_effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(
        nested_damage.target.base(),
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) {
        return None;
    }

    let create = structural_unwrap_render_wrappers(first_effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if create.controller != PlayerFilter::IteratedPlayer || create.count != nested_damage.amount {
        return None;
    }
    let single_action = crate::effects::ForPlayersEffect::new(
        for_players.filter.clone(),
        vec![first_effect.clone()],
    );
    let first = describe_for_players_simple_iterated_action(&single_action)?;
    let second = describe_quantified_player_damage(nested)?;
    let where_x = describe_where_x_basis(&create.count)?;
    Some(format!(
        "{} and {}, where X is {where_x}",
        strip_shared_where_x_clause(&first),
        lowercase_first(strip_shared_where_x_clause(&second))
    ))
}

fn relative_iterated_player_condition(condition: &Condition) -> Option<String> {
    if let Some(relative) = describe_player_relative_condition(condition) {
        return Some(relative);
    }
    let described = describe_condition(condition);
    let relative = described
        .strip_prefix("that player ")
        .or_else(|| described.strip_prefix("That player "))?;
    Some(
        relative
            .replace("that player's", "their")
            .replace("That player's", "their"),
    )
}

fn describe_quantified_player_conditional(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.starting_with_controller || for_players.stop_after_first_happened {
        return None;
    }
    let [conditional_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.if_true.is_empty() || !conditional.if_false.is_empty() {
        return None;
    }
    let relative = relative_iterated_player_condition(&conditional.condition)?;
    let branch = crate::effects::ForPlayersEffect::new(
        for_players.filter.clone(),
        conditional.if_true.clone(),
    );
    let rendered = describe_for_players_simple_iterated_action(&branch)
        .or_else(|| describe_for_players_iterated_action_sequence(&branch))?;
    let subject = quantified_player_subject(&for_players.filter);
    let action = rendered.strip_prefix(&format!("{subject} "))?;
    Some(format!("{subject} who {relative} {action}"))
}

fn describe_quantified_player_life_total(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.starting_with_controller || for_players.stop_after_first_happened {
        return None;
    }
    let [effect] = for_players.effects.as_slice() else {
        return None;
    };
    let set_life = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::SetLifeTotalEffect>()?;
    if set_life.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    let subject = quantified_player_subject(&for_players.filter);
    let possessive = if subject == "Each player" {
        "Each player's".to_string()
    } else if subject == "Each opponent" {
        "Each opponent's".to_string()
    } else {
        format!("{subject}'s")
    };
    Some(format!(
        "{possessive} life total becomes {}",
        describe_value(&set_life.amount)
    ))
}

fn describe_quantified_player_effect(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    describe_for_players_may_happened_sequence(for_players)
        .or_else(|| describe_quantified_damage_then_controller_gain(for_players))
        .or_else(|| describe_quantified_nested_shared_action(for_players))
        .or_else(|| describe_quantified_player_damage(for_players))
        .or_else(|| describe_quantified_player_conditional(for_players))
        .or_else(|| describe_quantified_player_life_total(for_players))
        .or_else(|| describe_for_players_simple_iterated_action(for_players))
        .or_else(|| describe_for_players_iterated_action_sequence(for_players))
}

fn describe_parley_reveal_repeat_draw(effects: &[Effect]) -> Option<String> {
    let [reveal_effect, repeat_effect, draw_effect] = effects else {
        return None;
    };
    let reveal_with_id = reveal_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let reveal_players = reveal_with_id
        .effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let [reveal_top_effect] = reveal_players.effects.as_slice() else {
        return None;
    };
    let reveal_top = reveal_top_effect.downcast_ref::<crate::effects::RevealTopEffect>()?;
    let repeat = repeat_effect.downcast_ref::<crate::effects::RepeatEffectsEffect>()?;
    let draw_players = draw_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let [draw_cards_effect] = draw_players.effects.as_slice() else {
        return None;
    };
    let draw = draw_cards_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if reveal_players.filter != PlayerFilter::Any
        || reveal_players.starting_with_controller
        || reveal_players.stop_after_first_happened
        || reveal_top.player != PlayerFilter::IteratedPlayer
        || !matches!(
            repeat.count.unhinted(),
            Value::PriorEffectMetric { effect_id, .. } if *effect_id == reveal_with_id.id
        )
        || !repeat
            .count
            .has_surface_hint(ValueSurfaceHint::CardsRevealedThisWay)
        || draw_players.filter != PlayerFilter::Any
        || draw_players.starting_with_controller
        || draw_players.stop_after_first_happened
        || draw.player != PlayerFilter::IteratedPlayer
        || draw.count != Value::Fixed(1)
    {
        return None;
    }

    let reveal = describe_quantified_player_effect(reveal_players)?;
    let mut repeated = describe_effect(repeat_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    for (without_actor, with_actor) in [
        (", create ", ", you create "),
        (", investigate", ", you investigate"),
    ] {
        repeated = repeated.replace(without_actor, with_actor);
    }
    repeated = repeated.replace(
        ", each attacking creature you control gets ",
        ", attacking creatures you control get ",
    );
    let draw = describe_quantified_player_effect(draw_players)?;
    Some(format!(
        "{}. {}. Then {}",
        reveal.trim_end_matches('.'),
        capitalize_first(&repeated),
        lowercase_first(draw.trim_end_matches('.'))
    ))
}

fn describe_fade_from_history_shape(effects: &[Effect]) -> Option<String> {
    let [for_players_effect, destroy_effect] = effects else {
        return None;
    };
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let qualified = describe_quantified_player_conditional(for_players)?;
    let destroy = structural_unwrap_render_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(filter) = destroy.spec.base() else {
        return None;
    };
    if filter.card_types.len() != 2
        || !filter.card_types.contains(&CardType::Artifact)
        || !filter.card_types.contains(&CardType::Enchantment)
    {
        return None;
    }
    let destroyed = describe_effect(destroy_effect)
        .replace("artifacts or enchantments", "artifacts and enchantments");
    Some(format!(
        "{}. Then {}",
        qualified.trim_end_matches('.'),
        lowercase_first(destroyed.trim_end_matches('.'))
    ))
}

fn describe_worldfire_shape(effects: &[Effect]) -> Option<String> {
    let [
        battlefield_exile_effect,
        hand_graveyard_exile_effect,
        life_effect,
    ] = effects
    else {
        return None;
    };
    let battlefield_filter = plain_exile_all_filter(battlefield_exile_effect)?;
    if battlefield_filter.zone != Some(Zone::Battlefield) || battlefield_filter.card_types.len() < 5
    {
        return None;
    }
    let hand_graveyard_filter = plain_exile_all_filter(hand_graveyard_exile_effect)?;
    if hand_graveyard_filter.any_of.len() != 2
        || ![Zone::Hand, Zone::Graveyard].iter().all(|zone| {
            hand_graveyard_filter
                .any_of
                .iter()
                .any(|part| part.zone == Some(*zone))
        })
    {
        return None;
    }
    let life_players = life_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let life = describe_quantified_player_life_total(life_players)?;
    Some(format!(
        "{}. Exile all cards from all hands and graveyards. {life}",
        describe_effect(battlefield_exile_effect)
            .trim()
            .trim_end_matches('.')
    ))
}

pub(in crate::compiled_text) fn describe_structural_multisentence_effect_list(
    effects: &[Effect],
) -> Option<String> {
    if let [first, rest @ ..] = effects
        && first
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    {
        return describe_structural_multisentence_effect_list(rest);
    }

    if let Some(compact) = describe_parley_reveal_repeat_draw(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_fade_from_history_shape(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_worldfire_shape(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_partitioned_tap_choice_destroy_complement(effects) {
        return Some(compact);
    }

    if let [for_players_effect] = effects
        && let Some(for_players) =
            for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()
        && let Some(compact) = describe_quantified_player_effect(for_players)
    {
        return Some(compact);
    }
    // The pile split and the pile-choice sacrifice are one printed procedure
    // even when lowering emits them as two sibling per-opponent loops.
    if let [split_effect, choice_effect] = effects
        && let Some(split_for_players) =
            split_effect.downcast_ref::<crate::effects::ForPlayersEffect>()
        && let Some(choice_for_players) =
            choice_effect.downcast_ref::<crate::effects::ForPlayersEffect>()
        && let Some(compact) = super::costs_and_triggers::describe_for_players_split_piles_then_choose_sacrifice_pair(
            split_for_players,
            choice_for_players,
        )
    {
        return Some(compact);
    }
    if let [first, rest @ ..] = effects
        && first
            .downcast_ref::<crate::effects::TagTriggeringBlockersEffect>()
            .is_some()
    {
        return describe_structural_multisentence_effect_list(rest);
    }
    if let [first, rest @ ..] = effects
        && first
            .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
            .is_some()
    {
        return describe_structural_multisentence_effect_list(rest);
    }

    if let Some(compact) = describe_ordered_choose_all_then_relative_pump(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_each_player_mill_then_reanimate_as_artifact(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_linked_attack_group_first_strike_reward(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_turn_source_exiled_face_up_then_lose_mana_value(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_source_exiled_creature_may_battlefield_else_hand(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_consult_then_move_matched_collection(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_dynamic_search_move_collection(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_iterated_same_name_search_collection(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_for_players_choose_move_then_subtypes(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_controller_grouped_exile_search_collection(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_chosen_type_consult_move_matches_shuffle_remainder(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_search_reveal_conditional_battlefield_or_hand(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_joint_discard_or_sacrifice_then_draw(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_target_player_reveal_top_may_put_matching_rest_bottom(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_distributed_damage_reciprocal_sources(effects) {
        return Some(compact);
    }

    if let [choose_effect, first_draw_effect, second_draw_effect] = effects
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChoosePlayerEffect>()
        && choose.chooser == PlayerFilter::You
        && choose.filter == PlayerFilter::Opponent
        && !choose.random
        && let Some(first_draw) =
            first_draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && let Some(second_draw) =
            second_draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && first_draw.count == second_draw.count
        && first_draw.player == PlayerFilter::You
        && player_is_immediately_chosen_opponent(&second_draw.player, choose)
    {
        return Some(format!(
            "Choose an opponent. You and that player each draw {}",
            describe_card_count(&first_draw.count)
        ));
    }

    if let Some(compact) = describe_choose_opponent_joint_nonland_untap(effects) {
        return Some(compact);
    }

    if let [choose_effect, joint_effect, followup_effect] = effects
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChoosePlayerEffect>()
        && choose.chooser == PlayerFilter::You
        && choose.filter == PlayerFilter::Opponent
        && !choose.random
        && let Some(sequence) = joint_effect.downcast_ref::<crate::effects::SequenceEffect>()
        && let Some(joint) = describe_coordinated_sequence(sequence)
        && joint.starts_with("You and that player each sacrifice ")
    {
        let followup = describe_effect(followup_effect);
        if followup.starts_with("Each player who sacrificed ") {
            return Some(format!(
                "Choose an opponent. {}. {}",
                joint.trim_end_matches('.'),
                followup.trim_end_matches('.')
            ));
        }
    }

    if let [
        choose_player_effect,
        first_choose,
        first_return,
        second_choose,
        second_return,
    ] = effects
        && let Some(choose_player) =
            choose_player_effect.downcast_ref::<crate::effects::ChoosePlayerEffect>()
        && choose_player.chooser == PlayerFilter::You
        && choose_player.filter == PlayerFilter::Opponent
        && !choose_player.random
        && let Some(second_choice) = structural_unwrap_render_wrappers(second_choose)
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && player_is_immediately_chosen_opponent(&second_choice.chooser, choose_player)
        && let Some(first) = describe_choose_then_return_from_graveyard(first_choose, first_return)
        && let Some(second) =
            describe_choose_then_return_from_graveyard(second_choose, second_return)
        && let Some(first) = first.strip_prefix("you return ")
        && second.starts_with("that player returns ")
    {
        return Some(format!("Choose an opponent. Return {first}, then {second}"));
    }

    if let Some(compact) = describe_discard_then_draw_amount_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_leading_effect_then_pump_and_grant_same_filter(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_distinct_power_choice_destroy_complement(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_exile_collection_play_any_type_then_exile_source(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_looked_card_split_destinations_structural(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_each_opponent_exile_top_then_cast_until_eot_any_color(effects) {
        return Some(compact);
    }

    let refs = effects.iter().collect::<Vec<_>>();
    if let Some((compact, consumed)) = render_look_reveal_repeated_choices(&refs)
        && consumed == effects.len()
    {
        return Some(compact);
    }
    if let Some(compact) = describe_exile_creatures_consult_that_many_battlefield_shuffle(&refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_top_opponent_exiles_rest_hand_then_may_cast(effects) {
        return Some(compact);
    }
    if let Some(compact) =
        describe_destroy_land_then_controller_reveals_until_land_graveyard(effects)
    {
        return Some(compact);
    }
    if let Some(compact) =
        describe_each_player_mill_exile_milled_creatures_create_power_token(effects)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_exile_all_creatures_each_player_fractal_power_counters(effects)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_draw_reveal_discard_nonland(effects) {
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
    if let Some(compact) = describe_discard_then_draw_for_discarded(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_for_players_may_discard_then_draw_if_discarded(effects) {
        return Some(compact);
    }
    if let [choose_effect, look_effect, reveal_effect, distribute_effect] = effects
        && let Some(choose_name) =
            choose_effect.downcast_ref::<crate::effects::ChooseCardNameEffect>()
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_tagged) =
            reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some((_, distribute)) = for_each_tagged_for_compaction(distribute_effect)
        && let Some(compact) = describe_choose_name_then_reveal_matching_hand_rest_graveyard(
            choose_name,
            look_at_top,
            reveal_tagged,
            distribute,
        )
    {
        return Some(compact);
    }
    if let [with_id_effect, if_effect] = effects
        && let Some(with_id) = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()
        && let Some(if_effect) = if_effect.downcast_ref::<crate::effects::IfEffect>()
        && let Some(compact) =
            describe_may_tagged_mill_then_if_do_put_milled_cards(with_id, if_effect)
    {
        return Some(compact);
    }
    if let [first, second, third, fourth] = effects
        && let Some((source_tag, mill)) = mill_with_collection_tag(first)
        && let Some(first_choice) = second.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(second_choice) = third.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, move_chosen)) = for_each_tagged_for_compaction(fourth)
        && let Some(compact) = describe_mill_then_put_milled_cards(
            source_tag.as_str(),
            mill,
            &[first_choice, second_choice],
            move_chosen,
        )
    {
        return Some(compact);
    }
    if let [first, second, third, fourth] = effects
        && let Some(tagged_mill) = first.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(mill) = tagged_mill
            .effect
            .downcast_ref::<crate::effects::MillEffect>()
        && let Some(choose) = second.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((Some(move_to_hand_with_id), move_to_hand)) =
            for_each_tagged_for_compaction(third)
        && let Some(if_effect) = fourth.downcast_ref::<crate::effects::IfEffect>()
        && let Some(compact) = describe_tagged_mill_then_put_milled_card_into_hand_with_fallback(
            tagged_mill,
            mill,
            choose,
            move_to_hand_with_id,
            move_to_hand,
            if_effect,
        )
    {
        return Some(compact);
    }
    if let [first, second, third] = effects
        && let Some((source_tag, mill)) = mill_with_collection_tag(first)
        && let Some(choose) = second.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, move_chosen)) = for_each_tagged_for_compaction(third)
        && let Some(compact) =
            describe_mill_then_put_milled_cards(source_tag.as_str(), mill, &[choose], move_chosen)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_top_one_hand_gain_mana_value_rest_graveyard(effects) {
        return Some(compact);
    }

    fn early_effect_tag(effect: &Effect) -> Option<&crate::TagKey> {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return Some(&tagged.tag);
        }
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            return Some(&tag_all.tag);
        }
        None
    }

    fn early_create_token(effect: &Effect) -> Option<&crate::effects::CreateTokenEffect> {
        unwrap_tag_wrapped_effect(effect).downcast_ref()
    }

    fn early_set_base_pt(effect: &Effect) -> Option<&crate::effects::SetBasePowerToughnessEffect> {
        unwrap_tag_wrapped_effect(effect).downcast_ref()
    }

    fn early_clean_count_subject(filter: &ObjectFilter) -> String {
        let mut subject = describe_count_filter_value_subject(filter);
        for suffix in [
            " in exile",
            " in all graveyards",
            " in a graveyard",
            " in graveyard",
            " on the battlefield",
        ] {
            if let Some(stripped) = subject.strip_suffix(suffix) {
                subject = stripped.to_string();
                break;
            }
        }
        subject
    }

    fn early_prior_count_subject(effect: &Effect) -> Option<(String, &'static str)> {
        let effect = unwrap_basic_tag_wrappers(effect);
        if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
            if let ChooseSpec::All(filter) | ChooseSpec::Object(filter) = destroy.spec.base() {
                return Some((early_clean_count_subject(filter), "destroyed"));
            }
        }
        if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
            if let ChooseSpec::All(filter) | ChooseSpec::Object(filter) = exile.spec.base() {
                return Some((early_clean_count_subject(filter), "exiled"));
            }
        }
        None
    }

    fn early_dynamic_token_phrase(
        create_effect: &Effect,
        set_pt_effect: &Effect,
        where_x: String,
    ) -> Option<String> {
        let create = early_create_token(create_effect)?;
        let set_pt = early_set_base_pt(set_pt_effect)?;
        let created_tag = early_effect_tag(create_effect)?;
        let Value::Fixed(count) = create.count.unhinted() else {
            return None;
        };
        if *count < 1
            || create.enters_attacking
            || set_pt.duration != Until::Forever
            || !matches!(&set_pt.target, ChooseSpec::Tagged(tag) if tag == created_tag)
        {
            return None;
        }
        let power_fixed = matches!(set_pt.power.unhinted(), Value::Fixed(_));
        let toughness_fixed = matches!(set_pt.toughness.unhinted(), Value::Fixed(_));
        let dynamic_pt = match (power_fixed, toughness_fixed) {
            (false, false) if set_pt.power.unhinted() == set_pt.toughness.unhinted() => {
                "X/X".to_string()
            }
            (false, true) => format!("X/{}", describe_value(&set_pt.toughness)),
            (true, false) => format!("{}/X", describe_value(&set_pt.power)),
            _ => return None,
        };
        let blueprint = describe_token_blueprint(&create.token);
        let mut token_phrase = blueprint.replacen("0/0 ", &format!("{dynamic_pt} "), 1);
        if token_phrase == blueprint {
            return None;
        }
        if create.enters_tapped {
            token_phrase = format!("tapped {token_phrase}");
        }
        let token_object = if *count == 1 {
            with_indefinite_article(&token_phrase)
        } else {
            format!(
                "{} {}",
                describe_object_count(&create.count),
                pluralize_token_phrase(&token_phrase)
            )
        };
        let controller_suffix = if create.controller == PlayerFilter::You {
            String::new()
        } else {
            format!(
                " under {} control",
                describe_possessive_player_filter(&create.controller)
            )
        };
        let pronoun = if *count == 1 { "it" } else { "them" };
        let mut text = format!("create {token_object}{controller_suffix}, where X is {where_x}");
        if create.sacrifice_at_end_of_combat {
            text.push_str(&format!(". Sacrifice {pronoun} at end of combat"));
        }
        if create.sacrifice_at_next_end_step {
            text.push_str(&format!(
                ". Sacrifice {pronoun} at the beginning of the next end step"
            ));
        }
        if create.exile_at_end_of_combat {
            text.push_str(&format!(". Exile {pronoun} at end of combat"));
        }
        if create.exile_at_next_end_step {
            text.push_str(&format!(
                ". Exile {pronoun} at the beginning of the next end step"
            ));
        }
        Some(text)
    }

    fn early_prior_effect_dynamic_count_token_bundle(effects: &[&Effect]) -> Option<String> {
        let [prior_effect, create_effect, set_pt_effect] = effects else {
            return None;
        };
        let with_id = prior_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
        if !is_effect_count_reference(&early_set_base_pt(set_pt_effect)?.power, Some(with_id.id)) {
            return None;
        }
        let (subject, action) = early_prior_count_subject(&with_id.effect)?;
        let token_text = early_dynamic_token_phrase(
            create_effect,
            set_pt_effect,
            format!("the number of {subject} {action} this way"),
        )?;
        Some(format!(
            "{}, then {token_text}",
            describe_effect(prior_effect).trim_end_matches('.')
        ))
    }

    fn early_create_token_then_set_base_pt_bundle(effects: &[&Effect]) -> Option<String> {
        let [create_effect, set_pt_effect] = effects else {
            return None;
        };
        let set_pt = early_set_base_pt(set_pt_effect)?;
        let basis = if matches!(set_pt.power.unhinted(), Value::Fixed(_)) {
            &set_pt.toughness
        } else {
            &set_pt.power
        };
        let where_x = describe_where_x_basis(basis)?;
        early_dynamic_token_phrase(create_effect, set_pt_effect, where_x)
            .map(|text| capitalize_first(&text))
    }

    if refs.len() >= 2
        && let Some(token_text) = early_create_token_then_set_base_pt_bundle(&refs[..2])
    {
        if effects.len() == 3
            && let Some(tag) = early_effect_tag(refs[0])
            && let Some(suffix) = describe_put_counter_on_each_tagged_suffix(tag, refs[2])
        {
            return Some(format!("{token_text}. {suffix}"));
        }
        if effects.len() == 2 {
            return Some(token_text);
        }
        let mut rest = describe_effect_list(&effects[2..]);
        if let Some((_, token_basis)) = token_text.rsplit_once(", where X is ")
            && !token_basis.contains('.')
        {
            let repeated_suffix = format!(", where X is {token_basis}");
            if let Some(stripped) = rest.strip_suffix(&repeated_suffix) {
                rest = stripped.to_string();
            }
        }
        return Some(format!(
            "{token_text}. {}",
            capitalize_first(rest.trim_end_matches('.'))
        ));
    }

    if let Some(compact) = early_prior_effect_dynamic_count_token_bundle(&refs) {
        return Some(compact);
    }
    if refs.len() == 3
        && let Some(token_text) = early_create_token_then_set_base_pt_bundle(&refs[1..])
    {
        return Some(format!(
            "{}. {token_text}",
            describe_effect(refs[0]).trim_end_matches('.')
        ));
    }
    if let [
        look_effect,
        choose_effect,
        reveal_effect,
        move_effect,
        rest_effect,
    ] = effects
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, reveal)) = for_each_tagged_for_compaction(reveal_effect)
        && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(move_effect)
        && let Some((_, rest)) = for_each_tagged_for_compaction(rest_effect)
        && let Some(compact) = describe_look_at_top_then_put_into_hand_rest_graveyard(
            look_at_top,
            None,
            choose,
            Some(reveal),
            move_to_hand,
            rest,
        )
    {
        return Some(compact);
    }
    if let [look_effect, choose_effect, move_effect, rest_effect] = effects
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(move_effect)
        && let Some((_, rest)) = for_each_tagged_for_compaction(rest_effect)
        && let Some(compact) = describe_look_at_top_then_put_into_hand_rest_graveyard(
            look_at_top,
            None,
            choose,
            None,
            move_to_hand,
            rest,
        )
    {
        return Some(compact);
    }
    if let Some(compact) = describe_player_protection_from_everything_pair(&refs) {
        return Some(compact);
    }

    if let Some(compact) = describe_draw_then_for_players_choose_exile(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_untap_attacking_then_additional_combat(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_double_power_then_grant_same_filter(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_pump_all_then_grant_same_filter(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_put_counters_then_grant_same_filter(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_targeted_named_vote_conditional_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_source_exiled_named_vote_conditional_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_secret_vote_voter_choice_control_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_secret_named_vote_followup_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_hybrid_named_vote_per_vote_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_council_dilemma_named_vote_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_secret_choice_match_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_each_player_repeat_pay_life_tokens_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_sacrificed_object_conditional_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_exile_target_and_attached_objects(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_countered_spell_exile_with_counters_gain_suspend(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_put_counters_then_gain_suspend(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_graveyard_exile_then_source_counters(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_tagged_effect_then_put_counter_on_each(effects) {
        return Some(compact);
    }

    if effects.len() == 3 {
        if let Some(compact) = describe_choose_sacrifice_then_gain_life_for_sacrificed(&refs) {
            return Some(compact);
        }
        if let Some(compact) = describe_choose_sacrifice_then_draw_for_sacrificed(&refs) {
            return Some(compact);
        }
        if let Some(compact) = describe_discard_hand_add_mana_draw_sequence(&refs) {
            return Some(compact);
        }
        if let Some(compact) = describe_planeswalk_chaos_vote_sequence(&refs) {
            return Some(compact);
        }
        if let Some(compact) = describe_named_vote_conditional_sequence(&refs) {
            return Some(compact);
        }
    }
    if effects.len() == 4
        && let Some(compact) = describe_choose_sacrifice_then_return_from_graveyard(&refs)
    {
        return Some(compact);
    }

    if let Some(compact) = describe_counter_unless_then_controller_discards(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_counter_unless_then_kick_count_draw(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_return_to_hand_then_owner_discards(effects) {
        return Some(compact);
    }

    describe_source_exiled_graveyard_token_sacrifice_structural(effects)
        .or_else(|| describe_roll_choose_destroy_create_structural(effects))
        .or_else(|| describe_roll_choose_draw_then_may_cast_structural(effects))
        .or_else(|| describe_draw_then_conditional_discard_unless_structural(effects))
        .or_else(|| describe_draw_discard_then_conditional_untap_structural(effects))
        .or_else(|| describe_draw_discard_then_create_structural(effects))
        .or_else(|| describe_reveal_top_choice_to_hand_rest_graveyard_structural(effects))
        .or_else(|| describe_reciprocal_creature_control_structural(effects))
        .or_else(|| describe_gain_control_untap_haste_structural(effects))
        .or_else(|| describe_exile_then_free_cast_while_exiled_structural(effects))
        .or_else(|| describe_choose_top_exile_then_conditional_cast_structural(effects))
        .or_else(|| describe_choose_top_exile_then_play_structural(effects))
        .or_else(|| describe_target_card_then_cast_this_turn_structural(effects))
        .or_else(|| describe_choose_name_target_mills_conditional_draw(effects))
        .or_else(|| describe_each_creature_and_player_damage_cant_regenerate_structural(effects))
}

pub(super) fn describe_source_exiled_graveyard_token_sacrifice_structural(
    effects: &[Effect],
) -> Option<String> {
    let [move_effect, create_effect, sacrifice_effect] = effects else {
        return None;
    };
    let with_id = move_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let move_to_zone = with_id
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Graveyard || move_to_zone.to_top {
        return None;
    }
    let ChooseSpec::All(filter) = move_to_zone.target.base() else {
        return None;
    };
    if !is_source_exiled_cards_filter(filter) {
        return None;
    }
    let create = create_effect.downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if !is_effect_count_reference(&create.count, Some(with_id.id)) {
        return None;
    }
    let sacrifice = sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(sacrifice.target, ChooseSpec::Source) {
        return None;
    }

    let token_blueprint = describe_token_blueprint(&create.token);
    let create_text = describe_create_token_action(
        &format!("a {token_blueprint} for each card put into a graveyard this way"),
        &create.controller,
        create.actor_surface_explicit,
    );
    Some(format!(
        "Put each card exiled with this artifact into its owner's graveyard, then {}. Sacrifice this artifact.",
        lowercase_first(&create_text)
    ))
}

pub(super) fn keyword_label_from_static_ability_id(
    ability: crate::static_abilities::StaticAbilityId,
) -> Option<&'static str> {
    Some(match ability {
        crate::static_abilities::StaticAbilityId::Flying => "flying",
        crate::static_abilities::StaticAbilityId::FirstStrike => "first strike",
        crate::static_abilities::StaticAbilityId::DoubleStrike => "double strike",
        crate::static_abilities::StaticAbilityId::Deathtouch => "deathtouch",
        crate::static_abilities::StaticAbilityId::Haste => "haste",
        crate::static_abilities::StaticAbilityId::Hexproof => "hexproof",
        crate::static_abilities::StaticAbilityId::Indestructible => "indestructible",
        crate::static_abilities::StaticAbilityId::Lifelink => "lifelink",
        crate::static_abilities::StaticAbilityId::Menace => "menace",
        crate::static_abilities::StaticAbilityId::Reach => "reach",
        crate::static_abilities::StaticAbilityId::Trample => "trample",
        crate::static_abilities::StaticAbilityId::Vigilance => "vigilance",
        _ => return None,
    })
}

pub(super) fn describe_double_power_then_grant_same_filter(effects: &[Effect]) -> Option<String> {
    let [for_each_effect, grant_effect] = effects else {
        return None;
    };
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachObject>()?;
    let [pump_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let pump = pump_effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if pump.until != Until::EndOfTurn
        || pump.condition.is_some()
        || pump.modification.is_some()
        || !pump.additional_modifications.is_empty()
        || !matches!(pump.target_spec.as_ref(), Some(ChooseSpec::Iterated))
    {
        return None;
    }
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = pump.runtime_modifications.as_slice()
    else {
        return None;
    };
    if !matches!(power, Value::PowerOf(spec) if matches!(spec.as_ref(), ChooseSpec::Iterated))
        || !matches!(toughness, Value::Fixed(0))
    {
        return None;
    }

    let grant = grant_effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if grant.until != Until::EndOfTurn
        || grant.condition.is_some()
        || !grant.runtime_modifications.is_empty()
        || !grant.additional_modifications.is_empty()
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
        return None;
    };
    let filter = match (&grant.target, grant.target_spec.as_ref()) {
        (crate::continuous::EffectTarget::Filter(filter), _) => filter,
        (_, Some(ChooseSpec::Object(filter))) => filter,
        _ => return None,
    };
    if filter != &for_each.filter {
        return None;
    }

    let ability_text = keyword_label_from_static_ability_id(ability.id())?;
    let description = for_each.filter.description();
    let filter_text = strip_indefinite_article(&description);
    let pronoun = if for_each.filter.card_types.contains(&CardType::Creature) {
        "Those creatures"
    } else {
        "Those objects"
    };
    Some(format!(
        "Double the power of each {filter_text} until end of turn. {pronoun} gain {ability_text} until end of turn"
    ))
}

pub(super) fn apply_continuous_filter(
    effect: &crate::effects::ApplyContinuousEffect,
) -> Option<&ObjectFilter> {
    match (&effect.target, effect.target_spec.as_ref()) {
        (crate::continuous::EffectTarget::Filter(filter), None) => Some(filter),
        (
            crate::continuous::EffectTarget::Filter(filter),
            Some(ChooseSpec::Object(spec_filter)),
        ) if filter == spec_filter => Some(filter),
        (_, Some(ChooseSpec::Object(filter))) => Some(filter),
        _ => None,
    }
}

pub(super) fn describe_pump_all_then_grant_same_filter(effects: &[Effect]) -> Option<String> {
    let [first_effect, second_effect] = effects else {
        return None;
    };
    let first = unwrap_basic_tag_wrappers(first_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let second = unwrap_basic_tag_wrappers(second_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let (pump, grant) = if !first.runtime_modifications.is_empty() {
        (first, second)
    } else {
        (second, first)
    };
    if pump.until != grant.until
        || pump.condition.is_some()
        || grant.condition.is_some()
        || pump.modification.is_some()
        || !pump.additional_modifications.is_empty()
        || !grant.additional_modifications.is_empty()
        || !grant.runtime_modifications.is_empty()
    {
        return None;
    }
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = pump.runtime_modifications.as_slice()
    else {
        return None;
    };
    let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
        return None;
    };
    let pump_filter = apply_continuous_filter(pump)?;
    let grant_filter = apply_continuous_filter(grant)?;
    if pump_filter != grant_filter {
        return None;
    }

    let ability_text = keyword_label_from_static_ability_id(ability.id())?;
    let subject = capitalize_first(&pluralize_noun_phrase(&pump_filter.description()));
    if power.unhinted() == toughness.unhinted()
        && value_prefers_where_x(power)
        && let Some(where_x) = describe_where_x_basis(power)
    {
        return Some(format!(
            "{subject} get +X/+X and gain {ability_text} {}, where X is {where_x}",
            describe_until(&pump.until)
        ));
    }
    Some(format!(
        "{subject} get {}/{} and gain {ability_text} {}",
        describe_signed_value(power),
        describe_toughness_delta_with_power_context(power, toughness),
        describe_until(&pump.until)
    ))
}

pub(super) fn describe_leading_effect_then_pump_and_grant_same_filter(
    effects: &[Effect],
) -> Option<String> {
    let [leading, _, _] = effects else {
        return None;
    };
    let suffix = describe_pump_all_then_grant_same_filter(&effects[1..])?;
    let leading = capitalize_first(describe_effect(leading).trim_end_matches('.'));
    Some(format!("{leading}. {suffix}"))
}

pub(super) fn describe_put_counters_then_grant_same_filter(effects: &[Effect]) -> Option<String> {
    let [put_effect, grant_effect] = effects else {
        return None;
    };
    let grant = unwrap_basic_tag_wrappers(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if grant.condition.is_some()
        || !grant.runtime_modifications.is_empty()
        || !grant.additional_modifications.is_empty()
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
        return None;
    };
    let ability_text = keyword_label_from_static_ability_id(ability.id())?;
    let until = describe_until(&grant.until);
    let duration = (!until.is_empty())
        .then(|| format!(" {until}"))
        .unwrap_or_default();

    if let Some((put_text, put_filter, put_tag)) = put_counters_each_filter_view(put_effect) {
        let grant_matches_countered_group = if let Some(put_tag) = put_tag {
            grant
                .target_spec
                .as_ref()
                .is_some_and(|spec| choose_spec_references_exact_tag(spec, put_tag))
        } else {
            apply_continuous_filter(grant).is_some_and(|grant_filter| grant_filter == put_filter)
        };
        if !grant_matches_countered_group {
            return None;
        }

        let subject = if put_filter.card_types.contains(&CardType::Creature) {
            "Those creatures"
        } else {
            "Those permanents"
        };
        return Some(format!(
            "{put_text}. {subject} gain {ability_text}{duration}"
        ));
    }

    None
}

/// Render a counter-followup conjunction only when the parser
/// preserved the authored `and` as a coordinated sequence. Exact affected-set
/// provenance is necessary to select the pronoun, but it cannot by itself
/// distinguish a conjunction from adjacent sentences such as Ajani Goldmane's
/// "Put ... . Those creatures gain ...".
pub(super) fn describe_coordinated_put_counters_then_grant_same_filter(
    effects: &[Effect],
) -> Option<String> {
    let [put_effect, grant_effect] = effects else {
        return None;
    };
    let grant = unwrap_basic_tag_wrappers(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if grant.condition.is_some()
        || !grant.runtime_modifications.is_empty()
        || !grant.additional_modifications.is_empty()
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
        return None;
    };
    let ability_text = keyword_label_from_static_ability_id(ability.id())?;
    let until = describe_until(&grant.until);
    let duration = (!until.is_empty())
        .then(|| format!(" {until}"))
        .unwrap_or_default();

    if let Some((put_text, _, put_tag)) = put_counters_each_filter_view(put_effect) {
        let put_tag = put_tag?;
        if !grant
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_references_exact_tag(spec, put_tag))
        {
            return None;
        }
        return Some(format!("{put_text} and they gain {ability_text}{duration}"));
    }

    let put = structural_unwrap_render_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed || put.target_count.is_some() {
        return None;
    }
    let grant_target = grant.target_spec.as_ref()?;
    let same_direct_target = target_specs_select_same_objects(&put.target, grant_target);
    let same_affected_tag = wrapped_effect_tag(put_effect)
        .is_some_and(|tag| choose_spec_references_exact_tag(grant_target, tag));
    let singular_counter_target = matches!(
        put.target.unhinted(),
        ChooseSpec::Source | ChooseSpec::Target(_) | ChooseSpec::Iterated
    ) || matches!(
        put.target.unhinted(),
        ChooseSpec::Tagged(tag) if tag.as_str() == "triggering" || tag.as_str() == "__it__"
    );
    if !(singular_counter_target && (same_direct_target || same_affected_tag)) {
        return None;
    }

    let put_text = describe_effect(put_effect)
        .trim_end_matches('.')
        .to_string();
    Some(format!("{put_text} and it gains {ability_text}{duration}"))
}

pub(super) fn describe_create_token_then_grant_same_tag(effects: &[Effect]) -> Option<String> {
    let [create_effect, grant_effect] = effects else {
        return None;
    };
    let (created_tag, create) = tagged_create_token_effect(create_effect)?;
    let grant = unwrap_basic_tag_wrappers(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if grant.condition.is_some()
        || !grant.runtime_modifications.is_empty()
        || !grant.additional_modifications.is_empty()
        || !grant
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_references_exact_tag(spec, created_tag))
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
        return None;
    };
    let mut create_text = describe_effect(create_effect)
        .trim_end_matches('.')
        .to_string();
    if let Value::Fixed(count) = create.count.unhinted()
        && let Some(count_word) = number_word(*count)
    {
        create_text = create_text
            .replace(
                &format!("Create {count} "),
                &format!("Create {count_word} "),
            )
            .replace(
                &format!("creates {count} "),
                &format!("creates {count_word} "),
            );
    }
    if ability.id() == crate::static_abilities::StaticAbilityId::Unblockable
        && grant.until == Until::Forever
    {
        return Some(format!(
            "{create_text} with \"This token can't be blocked\""
        ));
    }

    if grant.until != Until::EndOfTurn {
        return None;
    }
    let ability_text = keyword_label_from_static_ability_id(ability.id())?;
    let token_subject = if matches!(create.count.unhinted(), Value::Fixed(1)) {
        "that token gains"
    } else {
        "those tokens gain"
    };
    Some(format!(
        "{create_text}, and {token_subject} {ability_text} {}",
        describe_until(&grant.until)
    ))
}

pub(super) fn choose_spec_tag(spec: &ChooseSpec) -> Option<&crate::TagKey> {
    match spec.base() {
        ChooseSpec::Tagged(tag) => Some(tag),
        _ => None,
    }
}

pub(super) fn find_choice_filter_for_tag(
    effect: &Effect,
    tag: &crate::TagKey,
) -> Option<ObjectFilter> {
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && choose.tag == *tag
    {
        return Some(choose.filter.clone());
    }

    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_choice_filter_for_tag(child, tag);
        }
    });
    found
}

pub(super) fn find_battlefield_move_source_tag(
    effect: &Effect,
    moved_tag: &crate::TagKey,
) -> Option<crate::TagKey> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
        && tagged.tag == *moved_tag
    {
        if let Some(move_to_zone) = tagged
            .effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && move_to_zone.zone == Zone::Battlefield
        {
            return choose_spec_tag(&move_to_zone.target).cloned();
        }
        if let Some(put_onto_battlefield) = tagged
            .effect
            .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
        {
            return choose_spec_tag(&put_onto_battlefield.target).cloned();
        }
    }

    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_battlefield_move_source_tag(child, moved_tag);
        }
    });
    found
}

pub(super) fn find_reference_filter_for_tag(
    effects: &[Effect],
    tag: &crate::TagKey,
) -> Option<ObjectFilter> {
    for effect in effects {
        if let Some(filter) = find_choice_filter_for_tag(effect, tag) {
            return Some(filter);
        }
    }
    for effect in effects {
        let source_tag = find_battlefield_move_source_tag(effect, tag)?;
        for effect in effects {
            if let Some(filter) = find_choice_filter_for_tag(effect, &source_tag) {
                return Some(filter);
            }
        }
    }
    None
}

pub(super) fn demonstrative_subject_for_filter(filter: &ObjectFilter) -> Option<String> {
    if filter.subtypes.len() == 1 {
        return Some(format!("That {}", filter.subtypes[0]));
    }
    if filter.card_types.len() == 1 {
        return Some(format!(
            "That {}",
            filter.card_types[0].name().to_ascii_lowercase()
        ));
    }
    if filter.card_types.contains(&CardType::Creature) {
        return Some("That creature".to_string());
    }
    if filter.card_types.contains(&CardType::Artifact) {
        return Some("That artifact".to_string());
    }
    None
}

pub(super) fn tagged_haste_grant(effect: &Effect) -> Option<(&crate::TagKey, &Until)> {
    let apply = unwrap_basic_tag_wrappers(effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.condition.is_some()
        || !apply.runtime_modifications.is_empty()
        || !apply.additional_modifications.is_empty()
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &apply.modification else {
        return None;
    };
    if ability.id() != crate::static_abilities::StaticAbilityId::Haste {
        return None;
    }
    let Some(ChooseSpec::Tagged(tag)) = apply.target_spec.as_ref() else {
        return None;
    };
    Some((tag, &apply.until))
}

pub(super) fn delayed_next_end_step_cleanup(
    effect: &Effect,
    tag: &crate::TagKey,
) -> Option<&'static str> {
    let schedule = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    if !schedule.one_shot
        || schedule.start_next_turn
        || schedule.until_end_of_turn
        || !schedule
            .trigger
            .display()
            .to_ascii_lowercase()
            .contains("end step")
    {
        return None;
    }
    let delayed = schedule.effects.flattened_default_effects();
    let [cleanup] = delayed else {
        return None;
    };
    if let Some(sacrifice) = cleanup.downcast_ref::<crate::effects::SacrificeTargetEffect>()
        && matches!(choose_spec_tag(&sacrifice.target), Some(candidate) if candidate == tag)
    {
        return Some("Sacrifice it at the beginning of the next end step");
    }
    if let Some(move_to_zone) =
        unwrap_basic_tag_wrappers(cleanup).downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_zone.zone == Zone::Exile
        && matches!(choose_spec_tag(&move_to_zone.target), Some(candidate) if candidate == tag)
    {
        return Some("Exile it at the beginning of the next end step");
    }
    None
}

fn tagged_created_copy_is_plural(effect: &Effect, tag: &crate::TagKey) -> bool {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return tagged_created_copy_is_plural(&with_id.effect, tag);
    }
    let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() else {
        return false;
    };
    if &tagged.tag != tag {
        return false;
    }
    let Some(create) = unwrap_basic_tag_wrappers(&tagged.effect)
        .downcast_ref::<crate::effects::CreateTokenCopyEffect>()
    else {
        return false;
    };
    !matches!(create.count.unhinted(), Value::Fixed(1))
}

pub(super) fn describe_moved_object_haste_delayed_cleanup(effects: &[Effect]) -> Option<String> {
    if effects.len() < 3 {
        return None;
    }
    let grant_idx = effects.len() - 2;
    let cleanup_idx = effects.len() - 1;
    let prefix_effects = &effects[..grant_idx];
    let (tag, duration) = tagged_haste_grant(&effects[grant_idx])?;
    let cleanup = delayed_next_end_step_cleanup(&effects[cleanup_idx], tag)?;
    let filter = find_reference_filter_for_tag(prefix_effects, tag)?;
    let _ = demonstrative_subject_for_filter(&filter)?;
    let duration_text = match duration {
        Until::Forever => "",
        Until::EndOfTurn => " until end of turn",
        _ => return None,
    };
    let prefix = describe_effect_list(prefix_effects)
        .replace(
            "put it onto the battlefield",
            "put that card onto the battlefield",
        )
        .trim_end_matches('.')
        .to_string();
    let plural_created_tokens = prefix_effects
        .iter()
        .any(|effect| tagged_created_copy_is_plural(effect, tag));
    let (subject, cleanup) = if plural_created_tokens {
        ("They", cleanup.replacen(" it ", " them ", 1))
    } else {
        ("It", cleanup.to_string())
    };
    Some(format!(
        "{prefix}. {subject} gain{} haste{duration_text}. {cleanup}",
        if plural_created_tokens { "" } else { "s" }
    ))
}

pub(crate) fn describe_draw_count_then_grant_same_filter(effects: &[Effect]) -> Option<String> {
    let [draw_effect, grant_effect] = effects else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let Value::Count(draw_filter) = draw.count.unhinted() else {
        return None;
    };
    let grant = unwrap_basic_tag_wrappers(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if grant.condition.is_some()
        || !grant.runtime_modifications.is_empty()
        || !grant.additional_modifications.is_empty()
    {
        return None;
    }
    let crate::continuous::EffectTarget::Filter(grant_filter) = &grant.target else {
        return None;
    };
    if grant_filter != draw_filter {
        return None;
    }
    if let Some(target_spec) = &grant.target_spec
        && !matches!(target_spec, ChooseSpec::Object(filter) if filter == draw_filter)
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
        return None;
    };
    let ability_text = keyword_label_from_static_ability_id(ability.id())?;
    let subject = if draw_filter.card_types.contains(&CardType::Creature) {
        "Those creatures"
    } else {
        "Those permanents"
    };
    Some(format!(
        "{}. {subject} gain {ability_text} {}",
        describe_effect(draw_effect).trim_end_matches('.'),
        describe_until(&grant.until)
    ))
}

pub(super) fn describe_sacrificed_tagged_condition(condition: &Condition) -> Option<String> {
    let Condition::TaggedObjectMatches(tag, filter) = condition else {
        return None;
    };
    if filter.additional_cost_object_surface().is_some() {
        // An explicit "the sacrificed permanent was ..." predicate carries
        // its authored noun/action on the filter. Preserve that provenance
        // before the legacy event-tag fallback canonicalizes the same runtime
        // relation as "an artifact is sacrificed this way."
        return describe_sacrifice_cost_object_condition(tag, filter);
    }
    if !tag.as_str().starts_with("sacrificed_") {
        return None;
    }

    let mut filter = filter.clone();
    filter.zone = None;
    let subject = with_indefinite_article(strip_indefinite_article(&filter.description()));
    Some(format!("{subject} is sacrificed this way"))
}

pub(super) fn describe_exile_target_and_attached_objects(effects: &[Effect]) -> Option<String> {
    let [target_effect, attached_exile_effect, target_exile_effect] = effects else {
        return None;
    };
    let tagged_target = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = tagged_target
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let target_exile = target_exile_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if target_exile.zone != Zone::Exile
        || !matches!(&target_exile.target, ChooseSpec::Tagged(tag) if tag == &tagged_target.tag)
    {
        return None;
    }

    let attached_exile = attached_exile_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| tagged.effect.as_ref())
        .unwrap_or(attached_exile_effect)
        .downcast_ref::<crate::effects::ExileEffect>()?;
    if attached_exile.face_down {
        return None;
    }
    let ChooseSpec::All(attached_filter) = &attached_exile.spec else {
        return None;
    };
    let matching_constraints = attached_filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
                && constraint.tag == tagged_target.tag
        })
        .count();
    if matching_constraints != 1 {
        return None;
    }

    let mut described_filter = attached_filter.clone();
    described_filter.tagged_constraints.retain(|constraint| {
        !(constraint.relation == crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
            && constraint.tag == tagged_target.tag)
    });
    if described_filter == ObjectFilter::default() {
        return None;
    }

    let target_text = describe_choose_spec(&target_only.target);
    let attached_text = described_filter.description();
    let attachment_reference = if target_only.target.count().is_single() {
        "it"
    } else {
        "them"
    };
    Some(format!(
        "Exile {target_text} and all {attached_text} attached to {attachment_reference}"
    ))
}

pub(super) fn describe_sacrificed_object_conditional_sequence(
    effects: &[Effect],
) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }
    let mut parts = Vec::with_capacity(effects.len());
    for effect in effects {
        let conditional = effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
        if !conditional.if_false.is_empty() || conditional.if_true.is_empty() {
            return None;
        }
        let condition_text = describe_condition(&conditional.condition);
        if !condition_text.starts_with("the sacrificed ") {
            return None;
        }
        let rendered = describe_effect(effect);
        let trimmed = rendered.trim().trim_end_matches('.');
        if trimmed.is_empty()
            || trimmed.contains(". ")
            || trimmed.contains(": ")
            || trimmed.starts_with("If ")
            || trimmed.starts_with("When ")
            || trimmed.starts_with("Whenever ")
            || trimmed.starts_with("At ")
        {
            return None;
        }
        parts.push(trimmed.to_string());
    }
    Some(parts.join(". "))
}

pub(super) fn describe_sacrifice_then_sacrificed_conditional_sequence(
    effects: &[Effect],
) -> Option<String> {
    let (conditional_effect, prior_effects) = effects.split_last()?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::TaggedObjectMatches(tag, _) = &conditional.condition else {
        return None;
    };
    if !tag.as_str().starts_with("sacrificed_") {
        return None;
    }
    let has_matching_sacrifice = prior_effects.iter().any(|effect| {
        sacrifice_view_unwrapped(effect)
            .is_some_and(|view| filter_is_exactly_tagged(view.filter, tag))
    });
    if !has_matching_sacrifice {
        return None;
    }

    let prefix = describe_effect_list(prior_effects);
    let conditional_text = describe_effect(conditional_effect);
    let prefix = prefix.trim().trim_end_matches('.');
    let conditional_text = conditional_text.trim().trim_end_matches('.');
    if prefix.is_empty() || conditional_text.is_empty() || !conditional_text.starts_with("If ") {
        return None;
    }
    Some(format!("{prefix}. {conditional_text}"))
}

pub(super) fn unwrap_with_id(effect: &Effect) -> (&Effect, Option<crate::effect::EffectId>) {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return (&with_id.effect, Some(with_id.id));
    }
    (effect, None)
}

pub(super) fn describe_each_player_repeat_pay_life_tokens_sequence(
    effects: &[Effect],
) -> Option<String> {
    let [repeat_effect, token_effect] = effects else {
        return None;
    };
    let (repeat_unwrapped, repeat_id) = unwrap_with_id(repeat_effect);
    let repeat = repeat_unwrapped.downcast_ref::<crate::effects::RepeatProcessEffect>()?;
    if !matches!(repeat.predicate, crate::effect::EffectPredicate::Happened) {
        return None;
    }
    let [pay_players_effect] = repeat.effects.as_slice() else {
        return None;
    };
    let (pay_players_unwrapped, _) = unwrap_with_id(pay_players_effect);
    let pay_players = pay_players_unwrapped.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if pay_players.filter != PlayerFilter::Any || !pay_players.starting_with_controller {
        return None;
    }
    let [pay_life_effect] = pay_players.effects.as_slice() else {
        return None;
    };
    let pay_life = pay_life_effect.downcast_ref::<crate::effects::PayAnyLifeEffect>()?;
    if pay_life.min_amount != 0
        || pay_life.player != ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    {
        return None;
    }

    let token_players = token_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if token_players.filter != PlayerFilter::Any {
        return None;
    }
    let [create_effect] = token_players.effects.as_slice() else {
        return None;
    };
    let create = unwrap_basic_tag_wrappers(create_effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if create.controller != PlayerFilter::IteratedPlayer
        || !create.token.card.is_token
        || create.token.card.name != "Rat"
    {
        return None;
    }
    if !create
        .token
        .card
        .card_types
        .contains(&crate::types::CardType::Creature)
        || !create
            .token
            .card
            .subtypes
            .contains(&crate::types::Subtype::Rat)
        || create.token.card.color_indicator != Some(crate::color::ColorSet::BLACK)
        || !create.token.card.power_toughness.is_some_and(|pt| {
            matches!(pt.power, crate::card::PtValue::Fixed(1))
                && matches!(pt.toughness, crate::card::PtValue::Fixed(1))
        })
    {
        return None;
    }
    let Value::EffectMetric {
        effect_id,
        source: crate::effect::EffectMetricSource::Outcome,
        metric: crate::effect::EffectMetric::IteratedPlayerCount,
    } = &create.count
    else {
        return None;
    };
    if repeat_id != Some(*effect_id) {
        return None;
    }

    Some("Starting with you, each player may pay any amount of life. Repeat this process until no one pays life. Each player creates a 1/1 black Rat creature token for each 1 life they paid this way".to_string())
}

pub(super) fn describe_reveal_top_to_hand_then_lose_mana_value_effects(
    effects: &[Effect],
) -> Option<String> {
    let [reveal_effect, move_effect, lose_effect] = effects else {
        return None;
    };
    let reveal = unwrap_basic_tag_wrappers(reveal_effect)
        .downcast_ref::<crate::effects::RevealTopEffect>()?;
    if reveal.player != PlayerFilter::You {
        return None;
    }
    let tag = reveal.tag.as_ref()?;
    let move_effect = unwrap_basic_tag_wrappers(move_effect);
    let moves_tag_to_hand = move_effect
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .is_some_and(|return_to_hand| {
            matches!(return_to_hand.spec.base(), ChooseSpec::Tagged(found) if found == tag)
        })
        || move_effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|move_to_zone| {
                move_to_zone.zone == Zone::Hand
                    && matches!(
                        move_to_zone.target.base(),
                        ChooseSpec::Tagged(found) if found == tag
                    )
            });
    if !moves_tag_to_hand {
        return None;
    }
    let lose_life =
        unwrap_basic_tag_wrappers(lose_effect).downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if lose_life.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }
    if !matches!(
        &lose_life.amount,
        Value::ManaValueOf(spec)
            if matches!(spec.base(), ChooseSpec::Tagged(found) if found == tag)
    ) {
        return None;
    }
    Some(
        "Reveal the top card of your library and put that card into your hand. You lose life equal to that card's mana value"
            .to_string(),
    )
}

pub(super) fn is_all_attacking_creatures(spec: &ChooseSpec) -> bool {
    let ChooseSpec::All(filter) = spec.base() else {
        return false;
    };
    if !filter.attacking {
        return false;
    }
    let mut base = filter.clone();
    base.attacking = false;
    base == ObjectFilter::creature()
}

pub(super) fn describe_untap_attacking_then_additional_combat(
    effects: &[Effect],
) -> Option<String> {
    let [untap_effect, phases_effect] = effects else {
        return None;
    };
    let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;
    if !is_all_attacking_creatures(&untap.target) {
        return None;
    }
    let additional_phases =
        phases_effect.downcast_ref::<crate::effects::AdditionalPhasesEffect>()?;
    if additional_phases.phases != [crate::effects::AdditionalPhase::Combat] {
        return None;
    }
    Some(
        "Untap each attacking creature. After this phase, there is an additional combat phase"
            .to_string(),
    )
}

pub(super) fn describe_counter_unless_then_kick_count_draw(effects: &[Effect]) -> Option<String> {
    let [unless_effect, draw_effect] = effects else {
        return None;
    };
    let unless_pays = unwrap_structural_effect_tag(unless_effect)
        .downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    let [_counter] = unless_pays.effects.as_slice() else {
        return None;
    };
    unless_pays.effects[0].downcast_ref::<crate::effects::CounterEffect>()?;
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.count != Value::KickCount {
        return None;
    }

    let counter_text = describe_effect(unless_effect)
        .trim_end_matches('.')
        .to_string();
    let draw_text = if draw.player == PlayerFilter::You {
        "Draw a card for each time this spell was kicked".to_string()
    } else {
        describe_draw_for_each(draw)?
            .trim_end_matches('.')
            .to_string()
    };
    Some(format!("{counter_text}. {draw_text}"))
}

pub(super) fn describe_counter_unless_then_controller_discards(
    effects: &[Effect],
) -> Option<String> {
    let [unless_effect, discard_effect] = effects else {
        return None;
    };
    let countered_tag = structural_effect_tag(unless_effect)?.clone();
    let unless_pays = unwrap_structural_effect_tag(unless_effect)
        .downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    let [counter_effect] = unless_pays.effects.as_slice() else {
        return None;
    };
    counter_effect.downcast_ref::<crate::effects::CounterEffect>()?;

    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.count != Value::Fixed(1)
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
        || discard.player
            != PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(countered_tag))
    {
        return None;
    }

    let counter_text = describe_effect(unless_effect)
        .trim_end_matches('.')
        .to_string();
    Some(format!("{counter_text}. That player discards a card."))
}

pub(super) fn describe_return_to_hand_then_owner_discards(effects: &[Effect]) -> Option<String> {
    let [return_effect, discard_effect] = effects else {
        return None;
    };
    let returned_tag = structural_effect_tag(return_effect)?;
    unwrap_structural_effect_tag(return_effect)
        .downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.count != Value::Fixed(1)
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
        || discard.player
            != PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(returned_tag.clone()))
    {
        return None;
    }

    let return_text = describe_effect(return_effect)
        .trim_end_matches('.')
        .to_string();
    Some(format!("{return_text}, then that player discards a card."))
}

pub(super) fn structural_effect_tag(effect: &Effect) -> Option<&crate::TagKey> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Some(&tagged.tag);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return Some(&tag_all.tag);
    }
    None
}

pub(super) fn unwrap_structural_effect_tag(effect: &Effect) -> &Effect {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_structural_effect_tag(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return unwrap_structural_effect_tag(&tag_all.effect);
    }
    effect
}

pub(super) fn describe_roll_choose_destroy_create_structural(effects: &[Effect]) -> Option<String> {
    let [roll_effect, destroy_effect, create_effect] = effects else {
        return None;
    };
    let with_id = roll_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    with_id
        .effect
        .downcast_ref::<crate::effects::RollDiceChooseResultEffect>()?;

    fn unwrap_tags(effect: &Effect) -> &Effect {
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            return unwrap_tags(&tag_all.effect);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return unwrap_tags(&tagged.effect);
        }
        effect
    }

    let destroy = unwrap_tags(destroy_effect).downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(filter) = &destroy.spec else {
        return None;
    };
    if filter.card_types.as_slice() != [CardType::Creature] {
        return None;
    }
    let Some(crate::filter::Comparison::GreaterThanOrEqualExpr(value)) = &filter.power else {
        return None;
    };
    if !matches!(value.unhinted(), Value::EffectValue(id) if *id == with_id.id) {
        return None;
    }

    let create = unwrap_tags(create_effect).downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if !matches!(
        create.count.unhinted(),
        Value::EffectMetric {
            effect_id,
            metric: crate::effect::EffectMetric::OtherNumber,
            ..
        } if *effect_id == with_id.id
    ) {
        return None;
    }

    let roll_text = describe_effect(roll_effect)
        .trim_end_matches('.')
        .to_string();
    let destroy_text = describe_effect(destroy_effect)
        .trim_end_matches('.')
        .to_string();
    let create_text = lowercase_first(describe_effect(create_effect).trim_end_matches('.'));
    Some(format!("{roll_text}. {destroy_text}. Then {create_text}."))
}

pub(super) fn describe_roll_choose_draw_then_may_cast_structural(
    effects: &[Effect],
) -> Option<String> {
    let [roll_effect, draw_effect, may_effect] = effects else {
        return None;
    };
    let with_id = roll_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    with_id
        .effect
        .downcast_ref::<crate::effects::RollDiceChooseResultEffect>()?;

    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You
        || !matches!(draw.count.unhinted(), Value::EffectValue(id) if *id == with_id.id)
    {
        return None;
    }

    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    cast_effect
        .downcast_ref::<crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect>()?;

    let roll_text = describe_effect(roll_effect)
        .trim_end_matches('.')
        .to_string();
    let may_text = lowercase_first(describe_effect(cast_effect).trim_end_matches('.'));
    Some(format!(
        "{roll_text}. Draw cards equal to that result. Then {may_text}."
    ))
}

pub(super) fn describe_draw_discard_then_create_structural(effects: &[Effect]) -> Option<String> {
    let (draw_effect, discard_effect, lose_effect, create_effect) = match effects {
        [draw_effect, discard_effect, create_effect] => {
            (draw_effect, discard_effect, None, create_effect)
        }
        [draw_effect, discard_effect, lose_effect, create_effect] => (
            draw_effect,
            discard_effect,
            Some(lose_effect),
            create_effect,
        ),
        _ => return None,
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    let mut draw_discard = describe_draw_then_discard(draw, discard)?;
    if let Some(lose_effect) = lose_effect {
        let lose = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()?;
        if lose.player != ChooseSpec::Player(draw.player.clone()) {
            return None;
        }
        draw_discard.push_str(" and ");
        draw_discard.push_str(&lowercase_first(
            describe_effect(lose_effect).trim_end_matches('.'),
        ));
    }
    let create = describe_effect(create_effect);
    Some(format!("{}. {}", capitalize_first(&draw_discard), create))
}

pub(super) fn describe_draw_then_conditional_discard_unless_structural(
    effects: &[Effect],
) -> Option<String> {
    let [draw_effect, conditional_effect] = effects else {
        return None;
    };
    let draw = unwrap_structural_effect_tag(draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let conditional = unwrap_structural_effect_tag(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let Condition::Not(unless_condition) = &conditional.condition else {
        return None;
    };
    let [discard_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let discard = unwrap_structural_effect_tag(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    let draw_discard = describe_draw_then_discard(draw, discard)?;
    let condition_text = describe_condition(unless_condition);
    (!condition_text.trim().is_empty()).then(|| format!("{draw_discard} unless {condition_text}"))
}

pub(super) fn describe_draw_discard_then_conditional_untap_structural(
    effects: &[Effect],
) -> Option<String> {
    let (draw_effect, discard_effect, conditional_effect) = match effects {
        [draw_effect, discard_effect, conditional_effect] => {
            (draw_effect, discard_effect, conditional_effect)
        }
        [
            target_effect,
            draw_effect,
            discard_effect,
            conditional_effect,
        ] => {
            target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            (draw_effect, discard_effect, conditional_effect)
        }
        _ => return None,
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let discard = unwrap_structural_effect_tag(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    let discard_tag = structural_effect_tag(discard_effect).or(discard.tag.as_ref())?;
    let draw_discard = describe_draw_then_discard(draw, discard)?;

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let Condition::PlayerTaggedObjectMatches {
        player,
        tag,
        filter,
    } = &conditional.condition
    else {
        return None;
    };
    if tag != discard_tag || player != &draw.player {
        return None;
    }
    let [untap_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;
    if !matches!(untap.target.unhinted(), ChooseSpec::Source) {
        return None;
    }

    let subject = if *player == PlayerFilter::You {
        "you".to_string()
    } else {
        "that player".to_string()
    };
    let object_text = describe_player_tagged_object_text(tag, filter);
    let untap_text = lowercase_first(describe_effect(untap_effect).trim_end_matches('.'));
    Some(format!(
        "{}. If {subject} discards {object_text} this way, {untap_text}.",
        capitalize_first(&draw_discard)
    ))
}

pub(super) fn join_or_list(items: &[String]) -> Option<String> {
    match items {
        [] => None,
        [one] => Some(one.clone()),
        [first, second] => Some(format!("{first} or {second}")),
        _ => {
            let (last, rest) = items.split_last()?;
            Some(format!("{}, or {last}", rest.join(", ")))
        }
    }
}

pub(super) fn structural_revealed_choice_label(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    if looked_filter_has_only_card_type_structure(&choose.filter)
        && looked_filter_is_creature_land_union(&choose.filter)
    {
        return Some("creature or land card".to_string());
    }

    if looked_filter_has_only_card_type_structure(&choose.filter)
        && choose.filter.card_types.len() == 1
        && choose.filter.any_of.is_empty()
    {
        return Some(format!(
            "{} card",
            describe_card_type_word_local(choose.filter.card_types[0])
        ));
    }

    if looked_filter_has_only_card_type_structure(&choose.filter)
        && choose.filter.card_types.is_empty()
        && !choose.filter.any_of.is_empty()
    {
        let mut type_words = Vec::new();
        for candidate in &choose.filter.any_of {
            if candidate.card_types.len() != 1
                || !candidate.all_card_types.is_empty()
                || !candidate.subtypes.is_empty()
                || !candidate.static_abilities.is_empty()
                || !candidate.any_of.is_empty()
            {
                return None;
            }
            type_words.push(describe_card_type_word_local(candidate.card_types[0]).to_string());
        }
        return Some(format!("{} card", join_or_list(&type_words)?));
    }

    None
}

pub(super) fn structural_revealed_choice_phrase(label: &str) -> String {
    with_indefinite_article(label)
}

pub(super) fn choose_references_tag(
    choose: &crate::effects::ChooseObjectsEffect,
    tag: &crate::TagKey,
) -> bool {
    choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    })
}

pub(super) fn for_each_moves_tagged_iterated_to_hand(effect: &Effect, tag: &crate::TagKey) -> bool {
    let Some((_, for_each)) = for_each_tagged_for_compaction(effect) else {
        return false;
    };
    if for_each.tag != *tag || for_each.effects.len() != 1 {
        return false;
    }
    let inner = structural_unwrap_render_wrappers(&for_each.effects[0]);
    matches!(
        inner.downcast_ref::<crate::effects::MoveToZoneEffect>(),
        Some(move_to_zone)
            if move_to_zone.zone == Zone::Hand
                && !move_to_zone.to_top
                && matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    ) || matches!(
        inner.downcast_ref::<crate::effects::ReturnToHandEffect>(),
        Some(return_to_hand) if matches!(return_to_hand.spec.base(), ChooseSpec::Iterated)
    )
}
