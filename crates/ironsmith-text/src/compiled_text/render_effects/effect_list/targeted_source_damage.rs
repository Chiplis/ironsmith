use super::*;

/// "Target creature you control deals damage equal to its power to target
/// creature an opponent controls" (CR 120.3, 608.2b): the first target is
/// chosen as a target of the spell and then becomes the source of the damage.
/// Lowering announces that target on its own and executes the damage with the
/// same object as its source; both steps may carry outcome receipts.
pub(in crate::compiled_text) fn describe_targeted_source_characteristic_damage(
    target_effect: &Effect,
    damage_effect: &Effect,
) -> Option<String> {
    // The announced target's receipt tag names that same object later.
    let target_tag = target_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| tagged.tag.clone());
    let target_effect = target_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map_or(target_effect, |tagged| tagged.effect.as_ref());
    let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let damage_effect = damage_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .filter(|tagged| !tagged.outcome_only)
        .map_or(damage_effect, |tagged| tagged.effect.as_ref());
    let with_source = damage_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    let damage = with_source
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if with_source.source != target_only.target
        || !target_only.target.is_target()
        || !target_only.target.is_single()
        || damage.source_is_combat
        || damage.unpreventable
        || damage.excess_to_controller.is_some()
    {
        return None;
    }
    let targets_itself = matches!(damage.target.base(), ChooseSpec::Source)
        || matches!(damage.target.base(), ChooseSpec::Tagged(tag) if Some(tag) == target_tag.as_ref());
    let characteristic = match damage.amount.unhinted() {
        Value::PowerOf(spec) if matches!(spec.unhinted(), ChooseSpec::Source) => Some("power"),
        Value::ToughnessOf(spec) if matches!(spec.unhinted(), ChooseSpec::Source) => Some("toughness"),
        // "Target creature deals 6 damage to itself" (Asmoranomardicadaistinaculdacar).
        Value::Fixed(_) if targets_itself => None,
        _ => return None,
    };
    let recipient = if targets_itself {
        "itself".to_string()
    } else if matches!(
        damage.target.unhinted(),
        ChooseSpec::Player(PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag)))
            if Some(tag) == target_tag.as_ref()
    ) {
        // "target creature an opponent controls deals damage equal to its
        // power to that player" (Dong Zhou): the damaged player is the
        // opponent the target clause named.
        match target_only.target.base() {
            ChooseSpec::Object(filter)
                if matches!(filter.controller, Some(PlayerFilter::Opponent | PlayerFilter::NotYou)) =>
            {
                "that player".to_string()
            }
            _ => "its controller".to_string(),
        }
    } else {
        describe_damage_target(&damage.target)
    };
    let subject = capitalize_first(&describe_choose_spec(&target_only.target));
    Some(match characteristic {
        Some(characteristic) => {
            format!("{subject} deals damage equal to its {characteristic} to {recipient}")
        }
        None => format!("{subject} deals {} damage to {recipient}", describe_value(&damage.amount)),
    })
}

/// "Put a menace counter on a creature you control" (Blood Curdle): the
/// controller chooses exactly one object as the instruction resolves (an
/// untargeted choice, CR 608.2d) and the counters go on it. Lowering records
/// the choice and the counter placement as two steps.
pub(in crate::compiled_text) fn describe_choose_one_then_put_counter(
    choose_effect: &Effect,
    put_effect: &Effect,
) -> Option<String> {
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let put = put_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .filter(|tagged| !tagged.outcome_only)
        .map_or(put_effect, |tagged| tagged.effect.as_ref())
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if choose.is_search
        || choose.reveal
        || !matches!(choose.chooser, PlayerFilter::You | PlayerFilter::IteratedPlayer)
        || (choose.chooser == PlayerFilter::IteratedPlayer
            && choose.filter.controller != Some(PlayerFilter::IteratedPlayer))
        || !choose.count.is_single()
        || choose.count.min != 1
        || choose.count.random
        || choose.count.dynamic_x
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || !matches!(&put.target, ChooseSpec::Tagged(tag) if *tag == choose.tag)
        || put.target_count.is_some()
        || put.distributed
        || put.maximum_total.is_some()
        || put.completion_action.is_some()
    {
        return None;
    }
    // "Put X +1/+1 counters on a commander creature you control, where X is
    // the power of the creature exiled this way" (Astarion's Thirst): an
    // authored where-X amount keeps its trailing definition.
    let (counter, where_x) = match put.amount.unhinted() {
        Value::Fixed(_) => (describe_put_counter_phrase(&put.amount, put.counter_type), None),
        _ => {
            let (counter, where_x) =
                describe_counter_count_with_where_x(&put.amount, put.counter_type)?;
            (counter, Some(where_x))
        }
    };
    let where_suffix = where_x.map_or(String::new(), |where_x| format!(", where X is {where_x}"));
    // "each player may put two +1/+1 counters on a creature they control"
    // (Agitator Ant, Orzhov Advokist): inside a per-player iteration the
    // iterated player chooses one of their own permanents; the enclosing
    // player loop supplies the subject.
    if choose.chooser == PlayerFilter::IteratedPlayer {
        let mut objects = choose.filter.clone();
        objects.controller = None;
        objects.zone = None;
        return Some(format!(
            "Put {counter} on {} they control{where_suffix}",
            ensure_indefinite_article(&objects.description())
        ));
    }
    let selection = describe_choose_selection(choose);
    Some(format!("Put {counter} on {selection}{where_suffix}"))
}

/// "Distribute three +1/+1 counters among any number of creatures you
/// control" (Vastwood Hydra, Crashing Wave, Invoke Justice): the recipients
/// are an untargeted choice made as the instruction resolves (CR 608.2d),
/// and the distributed placement covers exactly that chosen set.
pub(in crate::compiled_text) fn describe_choose_then_distribute_counters(
    choose_effect: &Effect,
    put_effect: &Effect,
) -> Option<String> {
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let put = structural_unwrap_render_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if choose.is_search
        || choose.reveal
        || choose.chooser != PlayerFilter::You
        || choose.count.random
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || !matches!(&put.target, ChooseSpec::Tagged(tag) if *tag == choose.tag)
        || !put.distributed
        || put.required_recipient.is_some()
        || put.target_count.is_some()
        || put.maximum_total.is_some()
        || put.completion_action.is_some()
    {
        return None;
    }
    let recipients = describe_choose_selection(choose);
    if let Some((counter_text, where_x)) =
        describe_counter_count_with_where_x(&put.amount, put.counter_type)
    {
        return Some(format!("Distribute {counter_text} among {recipients}, where X is {where_x}"));
    }
    if matches!(put.amount.unhinted(), Value::CountersOnSource(_) | Value::CountersOn(..)) {
        return Some(format!(
            "Distribute a number of {} counters equal to {} among {recipients}",
            describe_counter_type(put.counter_type),
            describe_value(&put.amount),
        ));
    }
    Some(format!(
        "Distribute {} among {recipients}",
        describe_put_counter_phrase(&put.amount, put.counter_type),
    ))
}

/// "Turn a face-down creature you control face up" (Ugin's Mastery): an
/// untargeted single choice made on resolution (CR 608.2d), then the turn
/// face up of exactly that object, recorded by lowering as two steps.
pub(in crate::compiled_text) fn describe_choose_one_then_turn_face_up(
    choose_effect: &Effect,
    turn_effect: &Effect,
) -> Option<String> {
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let turn = structural_unwrap_render_wrappers(turn_effect)
        .downcast_ref::<crate::effects::TurnFaceUpEffect>()?;
    if choose.is_search
        || choose.reveal
        || choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose.count.min != 1
        || choose.count.random
        || choose.count.dynamic_x
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || !matches!(turn.target.base(), ChooseSpec::Tagged(tag) if *tag == choose.tag)
    {
        return None;
    }
    Some(format!("Turn {} face up", describe_choose_selection(choose)))
}

/// "Put a card you own from outside the game into your hand" (Ring of
/// Ma'rûf, Burning Wish family): the owned card chosen from outside the game
/// (CR 400.11b), then moved to hand, recorded by lowering as two steps.
pub(in crate::compiled_text) fn describe_choose_outside_game_then_put_into_hand(
    choose_effect: &Effect,
    move_effect: &Effect,
) -> Option<String> {
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let movement = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if choose.is_search
        || choose.reveal
        || choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose.count.min != 1
        || choose.count.random
        || choose.count.dynamic_x
        || choose.filter.zone != Some(Zone::OutsideGame)
        || choose.filter.owner != Some(PlayerFilter::You)
        || movement.zone != Zone::Hand
        || !matches!(movement.target.base(), ChooseSpec::Tagged(tag) if *tag == choose.tag)
    {
        return None;
    }
    let mut card = choose.filter.clone();
    card.zone = None;
    card.owner = None;
    let noun = if card == ObjectFilter::default() {
        "a card".to_string()
    } else {
        with_indefinite_article(&format!("{} card", strip_indefinite_article(&card.description())))
    };
    Some(format!("Put {noun} you own from outside the game into your hand"))
}

/// "Cloak a card from your hand" (Vannifar, Evolved Enigma): an untargeted
/// single choice made on resolution (CR 608.2d) whose only consumer is the
/// following manifest or cloak reads as that action on the chosen card.
pub(in crate::compiled_text) fn describe_choose_one_then_manifest_from_hand(
    choose_effect: &Effect,
    manifest_effect: &Effect,
) -> Option<String> {
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let manifest = structural_unwrap_render_wrappers(manifest_effect)
        .downcast_ref::<crate::effects::ManifestObjectsEffect>()?;
    if choose.is_search
        || choose.reveal
        || choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose.count.min != 1
        || choose.count.random
        || choose.count.dynamic_x
        || !choose.additional_zones.is_empty()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || !matches!(manifest.target.base(), ChooseSpec::Tagged(tag) if *tag == choose.tag)
        || manifest.controller != PlayerFilter::You
        || manifest.descriptive
        || manifest.tapped
        || manifest.shuffle
        || manifest.under_owner_control
        || manifest.exile_cast_permission.is_some()
    {
        return None;
    }
    let verb = if manifest.cloak { "Cloak" } else { "Manifest" };
    // The chosen card's origin is part of the action ("Cloak a card from
    // your hand"); the bare choice selection drops it.
    let mut card = choose.filter.clone();
    card.zone = None;
    if card.owner == Some(PlayerFilter::You) {
        card.owner = None;
    }
    if card == ObjectFilter::default() {
        return Some(format!("{verb} a card from your hand"));
    }
    Some(format!("{verb} {}", describe_choose_selection(choose)))
}


/// "Tezzeret deals X damage to each opponent, where X is the number of
/// artifacts you control. You gain X life." (Tezzeret, Master of the
/// Bridge): the second sentence reuses the X its predecessor defined, so the
/// shared where-X basis is stated once.
pub(in crate::compiled_text) fn describe_where_x_damage_then_gain_x_life(
    damage_effect: &Effect,
    gain_effect: &Effect,
) -> Option<String> {
    fn damage_amount(effect: &Effect) -> Option<&Value> {
        let effect = structural_unwrap_render_wrappers(effect);
        if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
            return Some(&damage.amount);
        }
        if let Some(each) = effect.downcast_ref::<crate::effects::ForPlayersEffect>()
            && let [only] = each.effects.as_slice()
        {
            return damage_amount(only);
        }
        if let Some(with_source) =
            effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        {
            return damage_amount(&with_source.effect);
        }
        None
    }
    let gain = structural_unwrap_render_wrappers(gain_effect)
        .downcast_ref::<crate::effects::GainLifeEffect>()?;
    let amount = damage_amount(damage_effect)?;
    if !matches!(gain.player.base(), ChooseSpec::Player(PlayerFilter::You))
        || !gain
            .amount
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs)
        || !amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs)
        || amount.unhinted() != gain.amount.unhinted()
    {
        return None;
    }
    let damage = describe_effect(damage_effect);
    let damage = damage.trim().trim_end_matches('.');
    if !damage.contains(", where X is ") {
        return None;
    }
    Some(format!("{damage}. You gain X life"))
}


/// "Put all cards with the chosen name from among them into your hand and
/// the rest into your graveyard" (Tamiyo, Collector of Tales): the revealed
/// cards matching the chosen name are tagged and moved to hand, and every
/// other revealed card goes to its graveyard as "the rest".
pub(in crate::compiled_text) fn describe_chosen_name_matches_to_hand_rest_graveyard(
    effects: &[Effect],
) -> Option<String> {
    let [tag_matching, to_hand, rest] = effects else {
        return None;
    };
    let tag_matching = structural_unwrap_render_wrappers(tag_matching)
        .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let [revealed_tag] = tag_matching.source_tags.as_slice() else {
        return None;
    };
    let [name_constraint] = tag_matching.filter.tagged_constraints.as_slice() else {
        return None;
    };
    if name_constraint.relation != crate::filter::TaggedOpbjectRelation::SameNameAsTagged {
        return None;
    }
    let mut residual = tag_matching.filter.clone();
    residual.tagged_constraints.clear();
    residual.match_current_state = false;
    residual.zone = None;
    if residual != ObjectFilter::default() {
        return None;
    }
    let to_hand = structural_unwrap_render_wrappers(to_hand)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [hand_move] = to_hand.effects.as_slice() else {
        return None;
    };
    let hand_move = structural_unwrap_render_wrappers(hand_move)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if to_hand.tag != tag_matching.tag
        || !matches!(hand_move.target.base(), ChooseSpec::Iterated)
        || hand_move.zone != Zone::Hand
    {
        return None;
    }
    let rest = structural_unwrap_render_wrappers(rest)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [conditional] = rest.effects.as_slice() else {
        return None;
    };
    let conditional = structural_unwrap_render_wrappers(conditional)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::TaggedObjectMatches(iterated, excluded) = &conditional.condition else {
        return None;
    };
    let [excluded_constraint] = excluded.tagged_constraints.as_slice() else {
        return None;
    };
    let [graveyard_move] = conditional.if_false.as_slice() else {
        return None;
    };
    let graveyard_move = structural_unwrap_render_wrappers(graveyard_move)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if &rest.tag != revealed_tag
        || iterated.as_str() != "__it__"
        || !conditional.if_true.is_empty()
        || excluded_constraint.tag != tag_matching.tag
        || excluded_constraint.relation != crate::filter::TaggedOpbjectRelation::SameStableId
        || !matches!(graveyard_move.target.base(), ChooseSpec::Iterated)
        || graveyard_move.zone != Zone::Graveyard
    {
        return None;
    }
    Some(
        "Put all cards with the chosen name from among them into your hand and the rest into your graveyard"
            .to_string(),
    )
}

/// "For each different power among creatures on the battlefield, choose a
/// creature with that power. Destroy each creature not chosen this way."
/// (Celestial Judgment): one creature per distinct power, chosen by you, then
/// every creature outside that chosen set is destroyed.
pub(in crate::compiled_text) fn describe_choose_one_per_distinct_power_then_destroy_rest(
    choose_effect: &Effect,
    destroy_effect: &Effect,
) -> Option<String> {
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let destroy = destroy_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map_or(destroy_effect, |tagged| tagged.effect.as_ref())
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    fn normalized(filter: &ObjectFilter) -> ObjectFilter {
        let mut rest = filter.clone();
        rest.zone = None;
        rest.union_surface = Default::default();
        rest
    }
    let creature = normalized(&ObjectFilter::creature());
    if choose.is_search
        || choose.reveal
        || choose.chooser != PlayerFilter::You
        || !choose.filter.distinct_powers
        || choose.count.random
        || !choose.additional_zones.is_empty()
    {
        return None;
    }
    let mut chosen = normalized(&choose.filter);
    chosen.distinct_powers = false;
    if chosen != creature {
        return None;
    }
    let Some(Value::DistinctPowers(pool)) = choose.count_value.as_ref().map(|value| value.unhinted())
    else {
        return None;
    };
    if normalized(pool) != creature {
        return None;
    }
    let ChooseSpec::All(destroyed) = destroy.spec.base() else {
        return None;
    };
    let mut rest = normalized(destroyed);
    let [constraint] = rest.tagged_constraints.as_slice() else {
        return None;
    };
    if constraint.tag != choose.tag
        || constraint.relation != crate::target::TaggedOpbjectRelation::IsNotTaggedObject
    {
        return None;
    }
    rest.tagged_constraints.clear();
    if rest != creature {
        return None;
    }
    Some(
        "For each different power among creatures on the battlefield, choose a creature with that power. Destroy each creature not chosen this way"
            .to_string(),
    )
}


/// "If the player discards a card this way, they draw a card. If the player
/// doesn't discard a card this way, they mill a card." (Chains of
/// Mephistopheles): the did-not-happen branch on the same prior result as an
/// authored "<actor> <verb>s ... this way" branch states the negation of that
/// same result surface.
pub(in crate::compiled_text) fn describe_prior_result_branch_then_negated_branch(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let first_if = structural_unwrap_render_wrappers(first)
        .downcast_ref::<crate::effects::IfEffect>()?;
    let second_if = structural_unwrap_render_wrappers(second)
        .downcast_ref::<crate::effects::IfEffect>()?;
    let EffectPredicate::PriorEffectResult(surface) = &first_if.predicate else {
        return None;
    };
    if surface.negated
        || !matches!(second_if.predicate, EffectPredicate::DidNotHappen)
        || second_if.condition != first_if.condition
        || first_if.then.is_empty()
        || second_if.then.is_empty()
        || !first_if.else_.is_empty()
        || !second_if.else_.is_empty()
        || first_if.per_player_result
        || second_if.per_player_result
    {
        return None;
    }
    let mut negated = surface.clone();
    negated.negated = true;
    let negated_text = describe_effect_predicate(&EffectPredicate::PriorEffectResult(negated));
    if negated_text.starts_with("not (") {
        return None;
    }
    let first_text = describe_effect(first);
    let first_text = first_text.trim().trim_end_matches('.');
    let then = describe_effect_list(&second_if.then);
    let then = then.trim().trim_end_matches('.');
    if first_text.is_empty() || then.is_empty() {
        return None;
    }
    Some(format!("{first_text}. If {negated_text}, {}", lowercase_first(then)))
}


/// "{7}: Hold for Ransom's controller sacrifices it and draws a card." (Hold
/// for Ransom): inside an ability an Aura grants, the granting source's
/// controller sacrifices that source (CR 701.21a) and the same player draws.
pub(in crate::compiled_text) fn describe_granting_source_controller_sacrifices_and_draws(
    sacrifice_effect: &Effect,
    draw_effect: &Effect,
) -> Option<String> {
    let with_source = structural_unwrap_render_wrappers(sacrifice_effect)
        .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    let ChooseSpec::Tagged(source_tag) = with_source.source.base() else {
        return None;
    };
    if source_tag.as_str() != crate::tag::GRANTING_SOURCE_TAG {
        return None;
    }
    let sacrifice = structural_unwrap_render_wrappers(&with_source.effect)
        .downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(sacrifice.target.base(), ChooseSpec::Source) || sacrifice.player.is_some() {
        return None;
    }
    let draw = structural_unwrap_render_wrappers(draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if !matches!(
        &draw.player,
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag)) if tag == source_tag
    ) {
        return None;
    }
    let draws = match &draw.count {
        Value::Fixed(1) => "draws a card".to_string(),
        Value::Fixed(count) if *count > 1 => format!(
            "draws {} cards",
            small_number_word(*count as u32).unwrap_or_else(|| count.to_string())
        ),
        _ => return None,
    };
    let source = describe_choose_spec(&with_source.source);
    let source = if source == crate::compiled_text::GRANTING_SOURCE_RENDER_SURFACE {
        source
    } else {
        crate::compiled_text::normalize_common::title_case_card_name_fragment(source.trim())
    };
    Some(format!(
        "{}'s controller sacrifices it and {draws}",
        capitalize_first(&source)
    ))
}

/// "Put a flying counter on any creature you control if a creature card in
/// your graveyard has flying. Repeat this process for first strike, ..., and
/// vigilance. Then put a +1/+1 counter on Kathril for each counter put on a
/// creature this way." (Kathril, Aspect Warper): one conditional placement
/// per keyword, identical except for that keyword, followed by a count of
/// the counters the whole process placed.
pub(in crate::compiled_text) fn describe_repeated_keyword_counter_process(
    effects: &[Effect],
) -> Option<String> {
    let [process_effect, follow_up] = effects else {
        return None;
    };
    let with_id = process_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let sequence = with_id.effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.effects.len() < 3 {
        return None;
    }
    let mut shared: Option<(ObjectFilter, ObjectFilter)> = None;
    let mut keywords = Vec::new();
    for arm in &sequence.effects {
        let conditional = arm.downcast_ref::<crate::effects::ConditionalEffect>()?;
        if !conditional.if_false.is_empty() {
            return None;
        }
        let Condition::PlayerControls { player: PlayerFilter::You, filter } = &conditional.condition
        else {
            return None;
        };
        let [choose_effect, put_effect] = conditional.if_true.as_slice() else {
            return None;
        };
        let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        let put = put_effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
        if choose.chooser != PlayerFilter::You
            || choose.is_search
            || !choose.count.is_single()
            || choose.count.min != 1
            || put.amount.unhinted() != &Value::Fixed(1)
            || !matches!(&put.target, ChooseSpec::Tagged(tag) if *tag == choose.tag)
            || filter.static_abilities.len() != 1
        {
            return None;
        }
        let mut condition_shape = filter.clone();
        condition_shape.static_abilities.clear();
        match &shared {
            None => shared = Some((condition_shape, choose.filter.clone())),
            Some((condition, chosen)) => {
                if *condition != condition_shape || *chosen != choose.filter {
                    return None;
                }
            }
        }
        keywords.push(describe_counter_type(put.counter_type));
    }
    let follow = follow_up.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if !matches!(
        follow.amount.unhinted(),
        Value::PriorEffectMetric { effect_id, .. } if *effect_id == with_id.id
    ) {
        return None;
    }
    let (condition, chosen) = shared?;
    if condition.zone != Some(Zone::Graveyard)
        || condition.card_types.as_slice() != [CardType::Creature]
        || chosen.controller != Some(PlayerFilter::You)
        || chosen.card_types.as_slice() != [CardType::Creature]
    {
        return None;
    }
    let (first, rest) = keywords.split_first()?;
    let rest = match rest {
        [] => return None,
        [only] => only.clone(),
        [head @ .., last] => format!("{}, and {last}", head.join(", ")),
    };
    let follow_text = describe_effect(follow_up);
    Some(format!(
        "Put {} counter on any creature you control if a creature card in your graveyard has {first}. Repeat this process for {rest}. Then {}",
        with_indefinite_article(first),
        lowercase_first(follow_text.trim().trim_end_matches('.'))
    ))
}


/// "Gain control of target nonland permanent an opponent controls. That
/// opponent draws three cards." (Rules of Acquisition): the draw names the
/// target's controller as tagged before the control change, which the
/// target filter fixes as an opponent.
pub(in crate::compiled_text) fn describe_control_target_then_former_opponent_draws(
    control_effect: &Effect,
    draw_effect: &Effect,
) -> Option<String> {
    let tagged = control_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let apply = structural_unwrap_render_wrappers(&tagged.effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.target != crate::continuous::EffectTarget::Source
        || apply.modification.is_some()
        || !apply.additional_modifications.is_empty()
        || apply.until != Until::Forever
        || apply.condition.is_some()
        || !matches!(
            apply.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
    {
        return None;
    }
    let spec = apply.target_spec.as_ref()?;
    if !spec.is_target() {
        return None;
    }
    let ChooseSpec::Object(filter) = spec.base() else {
        return None;
    };
    if filter.controller != Some(PlayerFilter::Opponent) {
        return None;
    }
    let draw = structural_unwrap_render_wrappers(draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if !matches!(
        &draw.player,
        PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Tagged(tag)) if tag == &tagged.tag
    ) || draw.count.has_surface_hint(ValueSurfaceHint::ForEach)
    {
        return None;
    }
    let control = describe_effect(control_effect);
    let control = control.trim().trim_end_matches('.');
    if control.is_empty() || control.contains(". ") {
        return None;
    }
    Some(format!(
        "{control}. That opponent draws {}",
        describe_card_count(&draw.count)
    ))
}


/// "Tap target creature defending player controls. If that creature has
/// greater power than this creature, put a number of +1/+1 counters on this
/// creature equal to the difference." (Conformer Shuriken): the condition
/// compares the tapped target with the source, and the counter amount is
/// that comparison's difference.
pub(in crate::compiled_text) fn describe_tap_target_then_greater_power_difference_counters(
    tap_effect: &Effect,
    conditional_effect: &Effect,
) -> Option<String> {
    let tagged = tap_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let tap = tagged.effect.downcast_ref::<crate::effects::TapEffect>()?;
    if !tap.target.is_target() || !tap.target.is_single() {
        return None;
    }
    let ChooseSpec::Object(target_filter) = tap.target.base() else {
        return None;
    };
    if target_filter.card_types.as_slice() != [CardType::Creature] {
        return None;
    }
    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::TaggedObjectMatches(tag, filter) = &conditional.condition else {
        return None;
    };
    if tag != &tagged.tag || !conditional.if_false.is_empty() {
        return None;
    }
    let mut rest = filter.clone();
    let Some(crate::filter::Comparison::GreaterThanExpr(threshold)) = rest.power.take() else {
        return None;
    };
    if *threshold != Value::SourcePower || rest != ObjectFilter::default() {
        return None;
    }
    let [put] = conditional.if_true.as_slice() else {
        return None;
    };
    let put = structural_unwrap_render_wrappers(put)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    // The difference hint may sit under another hint layer (the counter
    // follow-up sentence surface wraps the comparison difference).
    let mut hinted = &put.amount;
    let difference_hint = loop {
        if hinted.has_surface_hint(ValueSurfaceHint::Difference) {
            break true;
        }
        match hinted {
            Value::SurfaceHinted { value, .. } => hinted = value.as_ref(),
            _ => break false,
        }
    };
    if put.counter_type != crate::object::CounterType::PlusOnePlusOne
        || put.distributed
        || put.target_count.is_some()
        || !matches!(put.target.unhinted(), ChooseSpec::Source)
        || !difference_hint
    {
        return None;
    }
    let tap_text = describe_effect(tap_effect);
    let tap_text = tap_text.trim().trim_end_matches('.');
    if tap_text.is_empty() || tap_text.contains(". ") {
        return None;
    }
    let subject = describe_choose_spec(&put.target);
    let subject = subject.trim();
    if subject.is_empty() {
        return None;
    }
    Some(format!(
        "{tap_text}. If that creature has greater power than {subject}, put a number of +1/+1 counters on {subject} equal to the difference"
    ))
}


/// "Target creature you control deals damage equal to its power to each
/// other creature and each opponent" (Chandra's Ignition): the announced
/// target is the source of a fanout over the other creatures and of a
/// per-opponent damage step that repeats the same target. Lowering splits
/// the opponents into their own `ForPlayers` iteration.
pub(in crate::compiled_text) fn describe_targeted_source_power_damage_to_each_and_each_opponent(
    effects: &[Effect],
) -> Option<String> {
    fn power_damage<'a>(
        effect: &'a Effect,
        source: &ChooseSpec,
    ) -> Option<&'a crate::effects::DealDamageEffect> {
        let with_source = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
        if with_source.source != *source {
            return None;
        }
        let damage = structural_unwrap_render_wrappers(with_source.effect.as_ref())
            .downcast_ref::<crate::effects::DealDamageEffect>()?;
        (!damage.source_is_combat
            && !damage.unpreventable
            && damage.excess_to_controller.is_none()
            && matches!(damage.amount.unhinted(),
                Value::PowerOf(spec) if matches!(spec.unhinted(), ChooseSpec::Source)))
        .then_some(damage)
    }
    let [target_effect, group_effect, opponents_effect] = effects else {
        return None;
    };
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !target_only.target.is_target()
        || !target_only.target.is_single()
        || target_only.chooser.is_some()
    {
        return None;
    }
    let group = power_damage(group_effect, &target_only.target)?;
    if !matches!(group.target.unhinted(), ChooseSpec::All(_)) {
        return None;
    }
    let for_players = structural_unwrap_render_wrappers(opponents_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Opponent {
        return None;
    }
    let [repeat_target, opponent_damage] = for_players.effects.as_slice() else {
        return None;
    };
    let repeated = structural_unwrap_render_wrappers(repeat_target)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if repeated.target != target_only.target {
        return None;
    }
    let opponent = power_damage(opponent_damage, &target_only.target)?;
    if !matches!(opponent.target.unhinted(), ChooseSpec::Player(PlayerFilter::IteratedPlayer)) {
        return None;
    }
    let recipients = describe_damage_target(&group.target);
    let recipients = recipients.trim();
    if !recipients.starts_with("each ") || recipients.contains('.') {
        return None;
    }
    let subject = capitalize_first(&describe_choose_spec(&target_only.target));
    Some(format!(
        "{subject} deals damage equal to its power to {recipients} and each opponent"
    ))
}


/// The negation of an authored prior-result branch condition ("that player
/// doesn't discard a card this way"), stated by a later did-not-happen branch
/// on the same result (Chains of Mephistopheles).
pub(in crate::compiled_text) fn negated_prior_result_branch_condition(
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    let EffectPredicate::PriorEffectResult(surface) = &if_effect.predicate else {
        return None;
    };
    if surface.negated || !if_effect.else_.is_empty() || if_effect.per_player_result {
        return None;
    }
    let mut negated = surface.clone();
    negated.negated = true;
    let text = describe_effect_predicate(&EffectPredicate::PriorEffectResult(negated));
    (!text.starts_with("not (") && !text.trim().is_empty()).then_some(text)
}

/// "Exile cards from the top of your library until you exile a nonland card.
/// That nonland card gains "..."" (Q, Stacking the Deck): an ability grant to
/// the consult's matched card names that card by the consult's filter.
pub(in crate::compiled_text) fn describe_consult_then_grant_to_match(
    consult_effect: &Effect,
    grant_effect: &Effect,
) -> Option<String> {
    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let grant = structural_unwrap_render_wrappers(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !matches!(
        grant.target_spec.as_ref().map(ChooseSpec::unhinted),
        Some(ChooseSpec::Tagged(tag)) if *tag == consult.match_tag
    ) || !matches!(
        grant.modification,
        Some(crate::continuous::Modification::AddAbilityGeneric(_))
    ) || !grant.additional_modifications.is_empty()
    {
        return None;
    }
    let consult_text = describe_effect(consult_effect);
    let consult_text = consult_text.trim().trim_end_matches('.');
    let grant_text = describe_effect(grant_effect);
    let grant_text = grant_text.trim();
    let rest = grant_text
        .strip_prefix("It gains ")
        .or_else(|| grant_text.strip_prefix("it gains "))?;
    let noun = consult.filter.description();
    let noun = strip_indefinite_article(noun.trim());
    if consult_text.is_empty() || consult_text.contains(". ") || noun.is_empty() || !noun.ends_with("card") {
        return None;
    }
    Some(format!("{consult_text}. That {noun} gains {rest}"))
}

/// "Target opponent whose turn it is may cast a spell from their hand
/// without paying its mana cost. If they do, you copy that spell. You may
/// choose new targets for the copy. If they don't, you draw a card." (Cmdr.
/// Troi, Outgoing Empath): the declared player is the offered caster; the
/// copy and the fallback branch on whether that player cast the spell.
pub(in crate::compiled_text) fn describe_target_player_free_cast_copy_or_fallback(
    effects: &[Effect],
) -> Option<String> {
    let [declare_effect, offer_effect, copy_effect, fallback_effect] = effects else {
        return None;
    };
    let declare = structural_unwrap_render_wrappers(declare_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::Target(inner) = &declare.target else {
        return None;
    };
    let ChooseSpec::Player(declared) = inner.as_ref() else {
        return None;
    };
    if declare.chooser.is_some() {
        return None;
    }
    let caster = PlayerFilter::AliasedTarget(Box::new(declared.clone()));
    let offer = offer_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = offer.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.as_ref() != Some(&caster) {
        return None;
    }
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast_tagged = cast_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let cast = cast_tagged
        .effect
        .downcast_ref::<crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect>()?;
    let mut nonland = ObjectFilter::default();
    nonland.excluded_card_types = vec![CardType::Land];
    if cast.player != caster
        || cast.zone_owner != caster
        || cast.zone != Zone::Hand
        || cast.payment != ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost
        || cast.filter != nonland
    {
        return None;
    }
    let copy_branch = structural_unwrap_render_wrappers(copy_effect)
        .downcast_ref::<crate::effects::IfEffect>()?;
    let fallback = structural_unwrap_render_wrappers(fallback_effect)
        .downcast_ref::<crate::effects::IfEffect>()?;
    if copy_branch.condition != offer.id
        || copy_branch.predicate != crate::effect::EffectPredicate::Happened
        || !copy_branch.else_.is_empty()
        || fallback.condition != offer.id
        || fallback.predicate != crate::effect::EffectPredicate::DidNotHappen
        || !fallback.else_.is_empty()
        || fallback.then.is_empty()
    {
        return None;
    }
    let [copy, retarget] = copy_branch.then.as_slice() else {
        return None;
    };
    let copy_with_id = copy
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map_or(copy, |tagged| tagged.effect.as_ref())
        .downcast_ref::<crate::effects::WithIdEffect>()?;
    let copy_spell = copy_with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    let retarget = retarget.downcast_ref::<crate::effects::ChooseNewTargetsEffect>()?;
    if copy_spell.copier != PlayerFilter::You
        || !matches!(copy_spell.count.unhinted(), Value::Fixed(1))
        || !matches!(copy_spell.target.base(), ChooseSpec::Tagged(tag) if *tag == cast_tagged.tag)
        || retarget.from_effect != copy_with_id.id
        || !retarget.may
        || !matches!(retarget.chooser, None | Some(PlayerFilter::You))
    {
        return None;
    }
    let fallback_text = describe_effect_list(&fallback.then);
    let fallback_text = lowercase_first(fallback_text.trim().trim_end_matches('.'));
    if fallback_text.is_empty() || fallback_text.contains(". ") {
        return None;
    }
    // The fallback is yours: "If they don't, you draw a card."
    let fallback_text = if fallback_text.starts_with("draw ") {
        format!("you {fallback_text}")
    } else {
        fallback_text
    };
    let player = describe_player_filter(declared);
    Some(format!(
        "Target {} may cast a spell from their hand without paying its mana cost. If they do, you copy that spell. You may choose new targets for the copy. If they don't, {fallback_text}",
        strip_leading_article(&player)
    ))
}

/// "target opponent exiles the top card of their library, a card at random
/// from their graveyard, and a card at random from their hand"
/// (Crabomination): one declared player's coordinated exile of one card from
/// each of their zones. Lowering nests the members in coordinated sequences
/// and repeats the player's target declaration; both name one player.
pub(in crate::compiled_text) fn describe_declared_player_exiles_card_from_each_zone(
    effects: &[Effect],
) -> Option<String> {
    fn flatten<'a>(effects: &'a [Effect], out: &mut Vec<&'a Effect>) {
        for effect in effects {
            match effect.downcast_ref::<crate::effects::SequenceEffect>() {
                Some(sequence)
                    if sequence.surface == ironsmith_core::SequenceSurface::Coordinated
                        && sequence.result_label.is_none() =>
                {
                    flatten(&sequence.effects, out)
                }
                _ => out.push(effect),
            }
        }
    }
    let mut flat = Vec::new();
    flatten(effects, &mut flat);
    let mut declared: Option<&PlayerFilter> = None;
    let mut items: Vec<String> = Vec::new();
    let mut index = 0;
    while let Some(effect) = flat.get(index).copied() {
        let effect = structural_unwrap_render_wrappers(effect);
        if let Some(target_only) = effect.downcast_ref::<crate::effects::TargetOnlyEffect>() {
            let ChooseSpec::Target(inner) = &target_only.target else {
                return None;
            };
            let ChooseSpec::Player(player) = inner.as_ref() else {
                return None;
            };
            if target_only.chooser.is_some() || declared.is_some_and(|known| known != player) {
                return None;
            }
            declared = Some(player);
            index += 1;
            continue;
        }
        let player = declared?;
        let actor = PlayerFilter::Target(Box::new(player.clone()));
        let aliased = PlayerFilter::AliasedTarget(Box::new(player.clone()));
        if let Some(exile_top) = effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>() {
            if exile_top.player != actor
                || exile_top.surface
                    != Some(ironsmith_core::ExileTopLibrarySurface::LibraryOwnerAsActor)
                || exile_top.face_down
                || !matches!(exile_top.count.unhinted(), Value::Fixed(1))
            {
                return None;
            }
            items.push("the top card of their library".to_string());
            index += 1;
            continue;
        }
        let choose = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        let exile = structural_unwrap_render_wrappers(flat.get(index + 1).copied()?)
            .downcast_ref::<crate::effects::ExileEffect>()?;
        if choose.is_search
            || choose.chooser != actor
            || choose.count.min != 1
            || choose.count.max != Some(1)
            || !choose.count.random
            || exile.face_down
            || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if *tag == choose.tag)
        {
            return None;
        }
        let mut rest = choose.filter.clone();
        let owner = rest.owner.take()?;
        let zone = match rest.zone.take()? {
            Zone::Graveyard => "graveyard",
            Zone::Hand => "hand",
            _ => return None,
        };
        if (owner != actor && owner != aliased) || rest != ObjectFilter::default() {
            return None;
        }
        items.push(format!("a card at random from their {zone}"));
        index += 2;
    }
    if items.len() < 2 {
        return None;
    }
    let player = describe_player_filter(declared?);
    Some(format!(
        "Target {} exiles {}",
        strip_leading_article(&player),
        join_with_and(&items)
    ))
}

/// "Whenever this creature becomes tapped, it and other creatures you
/// control that share a creature type with it each get +2/+0 and gain
/// undying until end of turn." (Haunted One): lowering splits the compound
/// subject into the referenced object's coordinated pump and grant, then the
/// same pump and grant over the other creatures related to that object. The
/// identical modifications and durations, the `other` bit, and the filter's
/// tagged relation to the first half's object prove the shared predicate.
pub(in crate::compiled_text) fn describe_tagged_and_related_others_shared_pump_grant(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    fn coordinated_pump_grant(
        effect: &Effect,
    ) -> Option<(
        &crate::effects::ApplyContinuousEffect,
        &crate::effects::ApplyContinuousEffect,
    )> {
        let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
        if sequence.surface != ironsmith_core::SequenceSurface::Coordinated {
            return None;
        }
        let [pump, grant] = sequence.effects.as_slice() else {
            return None;
        };
        let pump = unwrap_basic_tag_wrappers(pump)
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
        let grant = unwrap_basic_tag_wrappers(grant)
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
        let shaped = pump.modification.is_none()
            && pump.additional_modifications.is_empty()
            && !pump.runtime_modifications.is_empty()
            && grant.modification.is_some()
            && grant.runtime_modifications.is_empty()
            && pump.until == Until::EndOfTurn
            && grant.until == Until::EndOfTurn
            && pump.condition.is_none()
            && grant.condition.is_none();
        shaped.then_some((pump, grant))
    }

    let (first_pump, first_grant) = coordinated_pump_grant(first)?;
    let (other_pump, other_grant) = coordinated_pump_grant(second)?;
    let tag = match (
        first_pump.target_spec.as_ref()?.unhinted(),
        first_grant.target_spec.as_ref()?.unhinted(),
    ) {
        (ChooseSpec::Tagged(pump_tag), ChooseSpec::Tagged(grant_tag)) if pump_tag == grant_tag => {
            pump_tag.clone()
        }
        _ => return None,
    };
    let crate::continuous::EffectTarget::Filter(filter) = &other_pump.target else {
        return None;
    };
    if other_grant.target != other_pump.target
        || other_pump.target_spec.is_some()
        || other_grant.target_spec.is_some()
        || !filter.other
        || !filter.tagged_constraints.iter().any(|constraint| constraint.tag == tag)
        // `Effect` equality is executor identity (`Arc::ptr_eq`); the two
        // halves' granted abilities are lowered independently, so compare
        // their typed structure instead.
        || format!("{:?}", other_pump.runtime_modifications)
            != format!("{:?}", first_pump.runtime_modifications)
        || format!("{:?}", other_grant.modification) != format!("{:?}", first_grant.modification)
        || format!("{:?}", other_grant.additional_modifications)
            != format!("{:?}", first_grant.additional_modifications)
    {
        return None;
    }

    let first_text = describe_effect(first);
    let first_text = first_text.trim().trim_end_matches('.');
    let (subject, _) = first_text.split_once(" gets ")?;
    let other_text = lowercase_first(describe_effect(second).trim().trim_end_matches('.'));
    let (other_subject, predicate) = other_text.split_once(" get ")?;
    if subject.is_empty()
        || subject.contains(". ")
        || other_subject.is_empty()
        || predicate.contains(". ")
        || first_text.contains(". ")
    {
        return None;
    }
    let other_subject = other_subject.replace(" that shares ", " that share ");
    let predicate = predicate.replacen(" and gains ", " and gain ", 1);
    Some(format!("{subject} and {other_subject} each get {predicate}"))
}


/// The whole resolution of "Whenever this creature becomes tapped, it and
/// other creatures you control that share a creature type with it each get
/// +2/+0 and gain undying until end of turn." (Haunted One): the triggering
/// object's tag, then the two coordinated pump-and-grant halves.
pub(in crate::compiled_text) fn describe_triggering_tagged_and_related_others_pump_grant(
    effects: &[Effect],
) -> Option<String> {
    let rest = match effects {
        [tag, rest @ ..]
            if tag
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            rest
        }
        _ => effects,
    };
    let [first, second] = rest else {
        return None;
    };
    describe_tagged_and_related_others_shared_pump_grant(first, second)
}


/// "If the player discards a card this way, they draw a card." (Chains of
/// Mephistopheles): an authored definite "the player" result actor, whose
/// consequence names the same player by pronoun.
pub(in crate::compiled_text) fn prior_result_definite_player(
    if_effect: &crate::effects::IfEffect,
) -> bool {
    matches!(
        &if_effect.predicate,
        EffectPredicate::PriorEffectResult(surface)
            if surface.definite_player_surface
                && surface.actor == crate::effect::PriorEffectResultActor::ThatPlayer
    )
}

/// The consequence of a definite-actor result branch: "that player draws a
/// card" reads "they draw a card".
pub(in crate::compiled_text) fn definite_player_pronoun_followup(followup: &str) -> String {
    let trimmed = followup.trim_start();
    match trimmed
        .strip_prefix("that player ")
        .or_else(|| trimmed.strip_prefix("That player "))
    {
        Some(rest) => format!(
            "they {}",
            crate::compiled_text::normalize_common::normalize_you_verb_phrase(rest)
        ),
        None => followup.to_string(),
    }
}


/// "When this Aura enters attached to a creature, each other nontoken
/// creature you control becomes a copy of that creature." (Infinite
/// Reflection): the host captured by the attachment tag is the trigger's
/// "that <type>" anaphor, and the set excluding exactly that host is
/// "each other <set>".
pub(in crate::compiled_text) fn describe_entered_attached_host_copy(
    tag_effect: &Effect,
    apply_effect: &Effect,
) -> Option<String> {
    let tag_attached = tag_effect.downcast_ref::<crate::effects::TagAttachedToSourceEffect>()?;
    let host = tag_attached.entered_attached_host_surface?;
    let apply = apply_effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.until != Until::Forever
        || apply.condition.is_some()
        || apply.modification.is_some()
        || !apply.additional_modifications.is_empty()
        || apply.source_reference_surface.is_some()
        || apply.set_quantifier_surface.is_some()
        || apply.clause_surface.is_some()
    {
        return None;
    }
    let [crate::effects::continuous::RuntimeModification::CopyOf {
        source,
        preserve_source_abilities: false,
        name_override: None,
        add_supertypes,
        copy_exception_surface: None,
        ..
    }] = apply.runtime_modifications.as_slice()
    else {
        return None;
    };
    if !add_supertypes.is_empty() {
        return None;
    }
    let names_host = |filter: &ObjectFilter, is_tagged: bool| {
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == tag_attached.tag
                && if is_tagged {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                } else {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                }
        })
    };
    let ChooseSpec::Object(source_filter) = source.base() else {
        return None;
    };
    if !names_host(source_filter, true) {
        return None;
    }
    let crate::continuous::EffectTarget::Filter(filter) = &apply.target else {
        return None;
    };
    if filter.other || !names_host(filter, false) {
        return None;
    }
    let mut base = filter.clone();
    base.tagged_constraints.retain(|constraint| {
        !(constraint.tag == tag_attached.tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject)
    });
    let subject = strip_indefinite_article(&base.description()).to_string();
    Some(format!(
        "Each other {subject} becomes a copy of that {}",
        host.name()
    ))
}
