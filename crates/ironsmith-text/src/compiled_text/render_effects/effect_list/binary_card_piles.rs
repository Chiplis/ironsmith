//! Render the capture/partition/choice topology, never compiler tag names.
use super::*;
use super::helpers_00::*;


/// Grouping cards is also recorded as a turn-history fact. The record has
/// no printed clause, but it is part of the executable pile program and must
/// be present with its exact one-event, untagged shape.
fn is_single_pile_grouping_record(effect: &Effect) -> bool {
    effect
        .downcast_ref::<crate::effects::EmitKeywordActionEffect>()
        .is_some_and(|record| {
            *record == crate::effects::EmitKeywordActionEffect::new(
                crate::events::KeywordActionKind::GroupIntoPiles,
                1,
            )
        })
}

/// A target opponent privately partitions a looked-at library collection;
/// the controller exiles one complete pile and searches only the other.
/// Every producer/consumer tag and the search's scoped owner must agree.
pub(super) fn describe_face_down_library_search(effects: &[Effect]) -> Option<String> {
    let [gate_effect] = effects else { return None; };
    let gate = gate_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::ValueComparison { left, operator, right } = &gate.condition else {
        return None;
    };
    let (Value::CardsInLibrary(PlayerFilter::You),
        crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        Value::Fixed(minimum)) = (left, operator, right) else { return None; };
    if *minimum < 1 || !gate.if_false.is_empty()
        || gate.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || gate.capture_condition_result
    { return None; }
    let [target, look, split, complement, pick, grouped] = gate.if_true.as_slice() else { return None; };
    if !is_single_pile_grouping_record(grouped) { return None; }
    let target = target.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let expected_target = crate::effects::TargetOnlyEffect::new(
        ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Opponent)),
    );
    if *target != expected_target { return None; }
    let look = look.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let Value::Fixed(count) = look.count else { return None; };
    if count < 1 || *look != crate::effects::LookAtTopCardsEffect::new(
        PlayerFilter::You, Value::Fixed(count), look.tag.clone(),
    ).viewed_by(PlayerFilter::target_opponent()) { return None; }
    let captured = ObjectFilter::tagged(look.tag.clone())
        .in_zone(Zone::Library)
        .match_tagged(look.tag.clone(), crate::filter::TaggedOpbjectRelation::SameObjectId);
    let split = split.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let expected_split = crate::effects::ChooseObjectsEffect::new(
        captured.clone(), crate::effect::ChoiceCount::any_number(),
        PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent)), split.tag.clone(),
    ).in_zone(Zone::Library);
    if *split != expected_split { return None; }
    let complement = complement.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let expected_complement = crate::effects::TagMatchingObjectsEffect::new(
        captured.not_tagged(split.tag.clone()), complement.tag.clone(),
    ).in_zone(Zone::Library);
    if *complement != expected_complement { return None; }
    let pick = pick.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    let expected_pick = crate::effects::ChooseModeEffect::choose_one(pick.modes.clone())
        .with_chooser(PlayerFilter::You);
    if *pick != expected_pick || pick.modes.len() != 2 { return None; }
    let mut selected = None;
    for (index, (mode, (exiled, searched))) in pick.modes.iter().zip([
        (&split.tag, &complement.tag), (&complement.tag, &split.tag),
    ]).enumerate() {
        if mode.source_text != format!("Exile face-down pile {}", index + 1) { return None; }
        let [exile, search, hand, shuffle] = mode.effects.as_slice() else { return None; };
        let capture = |tag: &crate::tag::TagKey| ObjectFilter::tagged(tag.clone())
            .in_zone(Zone::Library)
            .match_tagged(tag.clone(), crate::filter::TaggedOpbjectRelation::SameObjectId);
        let exile = exile.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        let mut expected_exile = crate::effects::MoveToZoneEffect::new(
            ChooseSpec::All(capture(exiled)), Zone::Exile, false,
        ).with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put);
        expected_exile.target_plural_surface = true;
        if *exile != expected_exile { return None; }
        let search = search.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        let expected_search = crate::effects::ChooseObjectsEffect::new(
            capture(searched).owned_by(PlayerFilter::You),
            crate::effect::ChoiceCount::exactly(1), PlayerFilter::You, search.tag.clone(),
        ).in_zone(Zone::Library).as_search();
        if *search != expected_search { return None; }
        if selected.as_ref().is_some_and(|tag| tag != &search.tag) { return None; }
        selected = Some(search.tag.clone());
        let hand = hand.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        let expected_hand = crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(search.tag.clone()), Zone::Hand, false,
        ).with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put);
        if *hand != expected_hand { return None; }
        let shuffle = shuffle.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
        if *shuffle != crate::effects::ShuffleLibraryEffect::you() { return None; }
    }
    let minimum = number_word(*minimum).unwrap_or_else(|| minimum.to_string());
    let count = number_word(count).unwrap_or_else(|| count.to_string());
    Some(format!(
        "If your library has {minimum} or more cards in it, target opponent looks at the top {count} cards of your library and separates them into two face-down piles. Exile one of those piles. Search the other pile for a card, put it into your hand, then shuffle the rest of that pile into your library"
    ))
}

fn captured(filter: &ObjectFilter, tag: &crate::tag::TagKey) -> bool {
    captured_in(filter, tag, Zone::Library)
}
fn captured_in(filter: &ObjectFilter, tag: &crate::tag::TagKey, zone: Zone) -> bool {
    filter.zone == Some(zone)
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *tag && constraint.relation == crate::target::TaggedOpbjectRelation::SameObjectId
        })
}

fn move_is(effect: &Effect, tag: &crate::tag::TagKey, destination: Zone) -> bool {
    let Some(moved) = downcast_move_to_zone(effect) else { return false; };
    let ChooseSpec::All(filter) = moved.target.base() else { return false; };
    captured(filter, tag) && moved.zone == destination && !moved.to_top
}

fn partition_move_is(effect: &Effect, chosen: &crate::tag::TagKey, other: &crate::tag::TagKey, source: Zone) -> bool {
    let Some(moved) = downcast_move_to_zone(effect) else { return false; };
    let ChooseSpec::All(filter) = moved.target.base() else { return false; };
    moved.zone == Zone::Graveyard && filter.zone == Some(source)
        && moved.tagged_destinations == vec![(chosen.clone(), Zone::Hand), (other.clone(), Zone::Graveyard)]
        && filter.any_of.len() == 2
        && captured_in(&filter.any_of[0], chosen, source)
        && captured_in(&filter.any_of[1], other, source)
}

/// "Separate all creature cards in your graveyard into two piles. Exile the
/// pile of an opponent's choice and return the other to the battlefield."
fn describe_graveyard_partition(effects: &[&Effect]) -> Option<String> {
    let [pool, split, complement, opponent, pick] = effects else { return None; };
    let pool = unwrap_tag_wrappers(pool).downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let split = unwrap_tag_wrappers(split).downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let complement =
        unwrap_tag_wrappers(complement).downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let opponent = unwrap_tag_wrappers(opponent).downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
    let pick = unwrap_tag_wrappers(pick).downcast_ref::<crate::effects::ChooseModeEffect>()?;
    let [card_type] = pool.filter.card_types.as_slice() else { return None; };
    if pool.filter.zone != Some(Zone::Graveyard)
        || split.chooser != PlayerFilter::You
        || !split.count.is_any_number()
        || !captured_in(&split.filter, &pool.tag, Zone::Graveyard)
        || !captured_in(&complement.filter, &pool.tag, Zone::Graveyard)
        || opponent.filter != PlayerFilter::Opponent
        || pick.modes.len() != 2
    {
        return None;
    }
    let destinations_match = pick.modes.iter().all(|mode| {
        let moves: Vec<_> = mode
            .effects
            .iter()
            .filter_map(|effect| downcast_move_to_zone(effect))
            .map(|moved| moved.zone)
            .collect();
        moves == [Zone::Exile, Zone::Battlefield]
    });
    if !destinations_match {
        return None;
    }
    Some(format!(
        "Separate all {} cards in your graveyard into two piles. Exile the pile of an opponent's choice and return the other to the battlefield",
        describe_card_type_word_local(*card_type)
    ))
}

/// A looked-at library collection is divided into face-down and face-up
/// exiled piles; the defending player picks which tagged pile goes to hand.
pub(super) fn describe_looked_exile_defender_choice(effects: &[&Effect]) -> Option<String> {
    let [look, split, complement, down, up, pick, grouped] = effects else { return None; };
    if !is_single_pile_grouping_record(grouped) { return None; }
    let look = unwrap_tag_wrappers(look).downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let split = unwrap_tag_wrappers(split).downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let complement = unwrap_tag_wrappers(complement).downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let has = |filter: &ObjectFilter, tag: &crate::TagKey, relation: crate::filter::TaggedOpbjectRelation| {
        filter.tagged_constraints.iter().any(|constraint| constraint.tag == *tag && constraint.relation == relation)
    };
    // Field-level proof (not whole-struct equality, which drifts whenever a
    // presentation field is added): the split is an any-number private pick
    // from exactly the looked-at cards and the complement is the rest.
    if look.player != PlayerFilter::You || look.viewer != PlayerFilter::You || look.reveal
        || split.chooser != PlayerFilter::You || !split.count.is_any_number() || split.is_search
        || split.filter.zone != Some(Zone::Library)
        || !has(&split.filter, &look.tag, crate::filter::TaggedOpbjectRelation::IsTaggedObject)
        || !has(&complement.filter, &look.tag, crate::filter::TaggedOpbjectRelation::IsTaggedObject)
        || !has(&complement.filter, &split.tag, crate::filter::TaggedOpbjectRelation::IsNotTaggedObject)
    { return None; }
    for (effect, tag, face_down) in [(down, &split.tag, true), (up, &complement.tag, false)] {
        let exile = unwrap_tag_wrappers(effect).downcast_ref::<crate::effects::ExileEffect>()?;
        if exile.face_down != face_down
            || !matches!(&exile.spec, ChooseSpec::Tagged(exiled) if exiled == tag)
        { return None; }
    }
    let pick = unwrap_tag_wrappers(pick).downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if pick.chooser != Some(PlayerFilter::Defending) || pick.modes.len() != 2
        || !pick.common_prefix_effects.is_empty()
    { return None; }
    for (mode, tag) in pick.modes.iter().zip([&split.tag, &complement.tag]) {
        let [move_effect] = mode.effects.as_slice() else { return None; };
        let moved = downcast_move_to_zone(move_effect)?;
        let ChooseSpec::All(filter) = moved.target.base() else { return None; };
        if moved.zone != Zone::Hand || filter.zone != Some(Zone::Exile)
            || !has(filter, tag, crate::filter::TaggedOpbjectRelation::IsTaggedObject)
        { return None; }
    }
    let Value::Fixed(count) = look.count else { return None; };
    let count = number_word(count).unwrap_or_else(|| count.to_string());
    Some(format!(
        "Look at the top {count} cards of your library. Exile any number of them in a face-down pile and the rest in a face-up pile. Defending player chooses one of those piles. Put that pile into your hand"
    ))
}

pub(super) fn describe(effects: &[&Effect]) -> Option<String> {
    if let Some(text) = describe_looked_exile_defender_choice(effects) { return Some(text); }
    if let Some(text) = describe_exiled_partition_free_plays(effects) { return Some(text); }
    if let Some(text) = describe_graveyard_partition(effects) { return Some(text); }
    if let Some(text) = describe_exile(effects) { return Some(text); }
    if let Some(text) = describe_face_down_then_face_up_exile(effects) { return Some(text); }
    let effects = if effects.first().is_some_and(|effect|
        effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>().is_some())
    { &effects[1..] } else { effects };
    let [look, split, complement, rest @ ..] = effects else { return None; };
    let look = unwrap_tag_wrappers(look).downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let split = unwrap_tag_wrappers(split).downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let complement = unwrap_tag_wrappers(complement).downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    if look.player != PlayerFilter::You || !split.count.is_any_number()
        || split.is_search || !captured(&split.filter, &look.tag)
        || !captured(&complement.filter, &look.tag)
        || !complement.filter.tagged_constraints.iter().any(|constraint|
            constraint.tag == split.tag
                && constraint.relation == crate::target::TaggedOpbjectRelation::IsNotTaggedObject)
    { return None; }
    let rest = if !look.reveal {
        let (reveal, rest) = rest.split_first()?;
        if unwrap_tag_wrappers(reveal).downcast_ref::<crate::effects::RevealTaggedEffect>()?.tag != complement.tag {
            return None;
        }
        rest
    } else { rest };
    let self_split = split.chooser == PlayerFilter::You;
    let (pick, remaining) = if self_split {
        if !look.reveal && look.viewer != PlayerFilter::You { return None; }
        let [opponent, pick, rest @ ..] = rest else { return None; };
        let opponent = unwrap_tag_wrappers(opponent).downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
        let pick = unwrap_tag_wrappers(pick).downcast_ref::<crate::effects::ChooseModeEffect>()?;
        if opponent.chooser != PlayerFilter::You || opponent.filter != PlayerFilter::Opponent
            || opponent.random || pick.chooser != Some(PlayerFilter::TaggedPlayer(opponent.tag.clone()))
        { return None; }
        (pick, rest)
    } else {
        if look.reveal || look.viewer != PlayerFilter::target_opponent()
            || split.chooser != PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent)) { return None; }
        let (pick, rest) = rest.split_first()?;
        let pick = unwrap_tag_wrappers(pick).downcast_ref::<crate::effects::ChooseModeEffect>()?;
        if pick.chooser != Some(PlayerFilter::You) { return None; }
        (pick, rest)
    };
    if pick.modes.len() != 2 { return None; }
    let mut one_card = None;
    for (mode, (chosen, other)) in pick.modes.iter().zip([
        (&split.tag, &complement.tag), (&complement.tag, &split.tag),
    ]) {
        let single = match mode.effects.as_slice() {
            [moved] if partition_move_is(moved, chosen, other, Zone::Library) => false,
            [select, capture, hand, bottom] => {
                let select = unwrap_tag_wrappers(select).downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
                let capture = unwrap_tag_wrappers(capture).downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
                let bottom_move = downcast_move_to_zone(bottom)?;
                if select.chooser != PlayerFilter::You || select.count != crate::effect::ChoiceCount::exactly(1)
                    || !captured(&select.filter, chosen) || !captured(&capture.filter, &look.tag)
                    || !capture.filter.tagged_constraints.iter().any(|constraint|
                        constraint.tag == select.tag
                            && constraint.relation == crate::target::TaggedOpbjectRelation::IsNotTaggedObject)
                    || !move_is(hand, &select.tag, Zone::Hand)
                    || !move_is(bottom, &capture.tag, Zone::Library)
                    || bottom_move.library_order != Some(crate::effects::LibraryPlacementOrder::ChosenBy(PlayerFilter::You))
                { return None; }
                true
            }
            _ => return None,
        };
        if one_card.is_some_and(|previous| previous != single) { return None; }
        one_card = Some(single);
    }
    let count = match &look.count {
        Value::Fixed(count) => number_word(*count).unwrap_or_else(|| count.to_string()),
        Value::Add(left, right) if matches!(left.as_ref(), Value::X) => {
            let Value::Fixed(extra) = right.as_ref() else { return None; };
            format!("X plus {}", number_word(*extra).unwrap_or_else(|| extra.to_string()))
        }
        _ => return None,
    };
    let first = if look.reveal { "Reveal" } else if self_split { "Look at" } else { "Target opponent looks at" };
    let partition = if look.reveal { "two piles" } else { "a face-down pile and a face-up pile" };
    let verb = if self_split { "separate" } else { "separates" };
    let mut text = format!("{first} the top {count} cards of your library and {verb} them into {partition}.");
    if self_split { text.push_str(" An opponent chooses one of those piles."); }
    if one_card == Some(true) {
        text.push_str(" Put a card from the chosen pile into your hand, then put all other cards revealed this way on the bottom of your library in any order.");
    } else {
        text.push_str(if self_split {
            " Put that pile into your hand and the other into your graveyard."
        } else { " Put one pile into your hand and the other into your graveyard." });
    }
    for effect in remaining {
        text.push(' ');
        text.push_str(describe_effect(effect).trim_end_matches('.'));
        text.push('.');
    }
    Some(text)
}

/// "Exile the top four cards of your library in a face-down pile, then exile
/// the top four cards of your library in a face-up pile. An opponent chooses
/// one of those piles. Put that pile into your graveyard. Look at the cards in
/// the other pile. You may cast a spell from among them without paying its
/// mana cost. Put the rest into your hand."
fn describe_face_down_then_face_up_exile(effects: &[&Effect]) -> Option<String> {
    let [first, second, capture_first, capture_second, opponent, pick, rest @ ..] = effects else { return None; };
    let first = unwrap_tag_wrappers(first).downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let second = unwrap_tag_wrappers(second).downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let first_tag = first.moved_tags.first()?;
    let second_tag = second.moved_tags.first()?;
    if !first.face_down || second.face_down { return None; }
    for (producer, capture, tag) in [(first, capture_first, first_tag), (second, capture_second, second_tag)] {
        let capture = unwrap_tag_wrappers(capture).downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
        if producer.player != PlayerFilter::You || producer.moved_tags.len() != 1
            || !producer.accumulated_tags.is_empty()
            || capture.tag != *tag || !captured_in(&capture.filter, tag, Zone::Exile)
        { return None; }
    }
    let opponent = unwrap_tag_wrappers(opponent).downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
    let pick = unwrap_tag_wrappers(pick).downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if opponent.chooser != PlayerFilter::You || opponent.filter != PlayerFilter::Opponent
        || opponent.random || pick.chooser != Some(PlayerFilter::TaggedPlayer(opponent.tag.clone()))
        || pick.modes.len() != 2 { return None; }
    for (mode, (chosen, other)) in pick.modes.iter().zip([(first_tag, second_tag), (second_tag, first_tag)]) {
        let [to_graveyard, _look, _choose, cast, to_hand] = mode.effects.as_slice() else { return None; };
        let to_graveyard = downcast_move_to_zone(to_graveyard)?;
        let to_hand = downcast_move_to_zone(to_hand)?;
        let (ChooseSpec::All(chosen_filter), ChooseSpec::All(other_filter)) =
            (to_graveyard.target.base(), to_hand.target.base()) else { return None; };
        if to_graveyard.zone != Zone::Graveyard || to_hand.zone != Zone::Hand
            || !captured_in(chosen_filter, chosen, Zone::Exile)
            || !captured_in(other_filter, other, Zone::Exile)
            || unwrap_tag_wrappers(cast).downcast_ref::<crate::effects::CastTaggedEffect>().is_none()
        { return None; }
    }
    let Value::Fixed(first) = first.count else { return None; };
    let Value::Fixed(second) = second.count else { return None; };
    let first = number_word(first).unwrap_or_else(|| first.to_string());
    let second = number_word(second).unwrap_or_else(|| second.to_string());
    let mut text = format!("Exile the top {first} cards of your library in a face-down pile, then exile the top {second} cards of your library in a face-up pile. An opponent chooses one of those piles. Put that pile into your graveyard. Look at the cards in the other pile. You may cast a spell from among them without paying its mana cost. Put the rest into your hand.");
    for effect in rest {
        text.push(' ');
        text.push_str(describe_effect(effect).trim_end_matches('.'));
        text.push('.');
    }
    Some(text)
}

fn describe_exile(effects: &[&Effect]) -> Option<String> {
    let [first, second, capture_first, look_first, capture_second, look_second, expose, opponent, pick, rest @ ..] = effects else { return None; };
    let first = unwrap_tag_wrappers(first).downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let second = unwrap_tag_wrappers(second).downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let first_tag = first.moved_tags.first()?;
    let second_tag = second.moved_tags.first()?;
    for (producer, capture, look, tag) in [
        (first, capture_first, look_first, first_tag), (second, capture_second, look_second, second_tag),
    ] {
        let capture = unwrap_tag_wrappers(capture).downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
        let look = unwrap_tag_wrappers(look).downcast_ref::<crate::effects::LookAtObjectsEffect>()?;
        if producer.player != PlayerFilter::You || !producer.face_down
            || producer.moved_tags.len() != 1 || !producer.accumulated_tags.is_empty()
            || capture.tag != *tag || !captured_in(&capture.filter, tag, Zone::Exile)
            || look.viewer != PlayerFilter::You || !captured_in(&look.filter, tag, Zone::Exile)
        { return None; }
    }
    let expose = unwrap_tag_wrappers(expose).downcast_ref::<crate::effects::ChooseModeEffect>()?;
    let opponent = unwrap_tag_wrappers(opponent).downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
    let pick = unwrap_tag_wrappers(pick).downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if expose.chooser != Some(PlayerFilter::You) || expose.modes.len() != 2
        || opponent.chooser != PlayerFilter::You || opponent.filter != PlayerFilter::Opponent
        || opponent.random || pick.chooser != Some(PlayerFilter::TaggedPlayer(opponent.tag.clone()))
        || pick.modes.len() != 2 { return None; }
    for (mode, tag) in expose.modes.iter().zip([first_tag, second_tag]) {
        let [turn] = mode.effects.as_slice() else { return None; };
        let turn = unwrap_tag_wrappers(turn).downcast_ref::<crate::effects::TurnFaceUpEffect>()?;
        if turn.target.base() != &ChooseSpec::Tagged(tag.clone()) { return None; }
    }
    for (mode, (chosen, other)) in pick.modes.iter().zip([(first_tag, second_tag), (second_tag, first_tag)]) {
        let [moved] = mode.effects.as_slice() else { return None; };
        if !partition_move_is(moved, chosen, other, Zone::Exile) { return None; }
    }
    let Value::Fixed(first) = first.count else { return None; };
    let Value::Fixed(second) = second.count else { return None; };
    let first = number_word(first).unwrap_or_else(|| first.to_string());
    let second = number_word(second).unwrap_or_else(|| second.to_string());
    let mut text = format!("Exile the top {first} cards of your library in a face-down pile, then exile the top {second} cards of your library in another face-down pile. Look at the cards in each pile, then turn a pile of your choice face up. An opponent chooses one of those piles. Put that pile into your hand and the other into your graveyard.");
    for effect in rest {
        text.push(' ');
        text.push_str(describe_effect(effect).trim_end_matches('.'));
        text.push('.');
    }
    Some(text)
}

/// A finite exiled pool is partitioned by an opponent, then the controller
/// repeatedly selects and immediately plays one card from the chosen pile.
fn describe_exiled_partition_free_plays(effects: &[&Effect]) -> Option<String> {
    let [exile, opponent, partition, complement, pick] = effects else { return None; };
    let exile = unwrap_tag_wrappers(exile).downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let opponent = unwrap_tag_wrappers(opponent).downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
    let partition = unwrap_tag_wrappers(partition).downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let complement = unwrap_tag_wrappers(complement).downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let pick = unwrap_tag_wrappers(pick).downcast_ref::<crate::effects::ChooseModeEffect>()?;
    let [pool] = exile.moved_tags.as_slice() else { return None; };
    let [split] = partition.effects.as_slice() else { return None; };
    let split = unwrap_tag_wrappers(split).downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if exile.player != PlayerFilter::You || exile.face_down || !exile.accumulated_tags.is_empty()
        || opponent.chooser != PlayerFilter::You || opponent.filter != PlayerFilter::Opponent || opponent.random
        || partition.filter != PlayerFilter::TaggedPlayer(opponent.tag.clone())
        || split.chooser != PlayerFilter::IteratedPlayer || !split.count.is_any_number() || split.is_search
        || !captured_in(&split.filter, pool, Zone::Exile)
        || !captured_in(&complement.filter, pool, Zone::Exile)
        || !complement.filter.tagged_constraints.iter().any(|constraint|
            constraint.tag == split.tag && constraint.relation == crate::target::TaggedOpbjectRelation::IsNotTaggedObject)
        || pick.chooser != Some(PlayerFilter::You) || pick.modes.len() != 2
    { return None; }
    for (mode, pile) in pick.modes.iter().zip([&split.tag, &complement.tag]) {
        let [repeat] = mode.effects.as_slice() else { return None; };
        let repeat = unwrap_tag_wrappers(repeat).downcast_ref::<crate::effects::RepeatProcessEffect>()?;
        let [choose, cast] = repeat.effects.as_slice() else { return None; };
        let choose = unwrap_tag_wrappers(choose).downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        let with_id = cast.downcast_ref::<crate::effects::WithIdEffect>()?;
        let cast = unwrap_tag_wrappers(&with_id.effect).downcast_ref::<crate::effects::CastTaggedEffect>()?;
        if choose.chooser != PlayerFilter::You || choose.count != crate::effect::ChoiceCount::up_to(1)
            || choose.is_search || !captured_in(&choose.filter, pile, Zone::Exile)
            || cast.tag != choose.tag || cast.player != PlayerFilter::You || !cast.allow_land
            || !cast.without_paying_mana_cost || cast.as_copy || cast.copy_named_definition
            || cast.alternative_cost.is_some() || cast.alternative_payment.is_some()
            || cast.additional_mana_cost.is_some() || cast.cost_reduction.is_some()
            || cast.caster_land_mana_abilities_only || cast.control_caster_while_spell_resolves
            || repeat.condition != with_id.id || repeat.predicate != EffectPredicate::Happened
            || repeat.max_iterations.is_some() || !repeat.choice_history.is_empty()
        { return None; }
    }
    let count = describe_value(&exile.count);
    Some(format!("Exile the top {count} cards of your library. An opponent separates those cards into two piles. You may play lands and cast spells from one of those piles. If you cast a spell this way, you cast it without paying its mana cost"))
}

#[cfg(test)]
mod exiled_partition_free_play_tests {
    #[test]
    fn repeated_pile_plays_preserve_the_free_cast_price() {
        for count in [5, 7] {
            let definition = crate::CardDefinitionBuilder::new(crate::CardId::new(), "Pile spell")
                .card_types(vec![crate::types::CardType::Sorcery])
                .parse_text(&format!("Exile the top {count} cards of your library. An opponent separates those cards into two piles. You may play lands and cast spells from one of those piles. If you cast a spell this way, you cast it without paying its mana cost."))
                .expect("exiled partition compiles");
            let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
            assert!(text.contains("An opponent separates those cards into two piles"), "{text}");
            assert!(text.contains("You may play lands and cast spells from one of those piles"), "{text}");
            assert!(text.contains("without paying its mana cost"), "{text}");
            assert!(!text.contains("binary_") && !text.contains("tagged object"), "{text}");
        }
    }
}

#[cfg(test)]
mod looked_exiled_defender_choice_tests {
    use super::*;
    #[test]
    fn defender_chooses_the_exiled_pile_and_counted_loss_is_per_event() {
        let oracle = "Whenever this creature attacks, look at the top three cards of your library. Exile any number of them in a face-down pile and the rest in a face-up pile. Defending player chooses one of those piles. Put that pile into your hand.\nAt the beginning of each end step, each opponent loses 2 life for each spell or ability that caused one or more players to guess or to group cards or permanents into a pile this turn.";
        let definition = crate::CardDefinitionBuilder::new(crate::CardId::new(), "Pile attacker")
            .card_types(vec![crate::types::CardType::Creature])
            .parse_text(oracle)
            .expect("tagged exile piles compile");
        let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
            panic!("expected attacking trigger");
        };
        let effects = triggered.effects.flattened_default_effects().iter().collect::<Vec<_>>();
        assert!(describe_looked_exile_defender_choice(&effects).is_some());
        assert!(describe_looked_exile_defender_choice(&effects[..effects.len() - 1]).is_none(),
            "missing grouping record must not receive the compact surface");
        let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(text.contains("Exile any number of them in a face-down pile and the rest in a face-up pile"), "{text}");
        assert!(text.contains("Defending player chooses one of those piles. Put that pile into your hand"), "{text}");
        assert!(text.contains("opponent loses 2 life for each spell or ability"), "{text}");
    }
}

#[cfg(test)]
mod face_down_library_search_tests {
    use super::*;

    #[test]
    fn tagged_other_pile_search_renders_and_requires_its_search_scope() {
        let oracle = "{3}: If your library has ten or more cards in it, target opponent looks at the top ten cards of your library and separates them into two face-down piles. Exile one of those piles. Search the other pile for a card, put it into your hand, then shuffle the rest of that pile into your library.";
        let definition = crate::CardDefinitionBuilder::new(crate::CardId::new(), "Library pile probe")
            .card_types(vec![crate::types::CardType::Artifact])
            .parse_text(oracle).expect("library pile procedure compiles");
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert_eq!(rendered, oracle);
        let crate::ability::AbilityKind::Activated(activated) = &definition.abilities[0].kind else {
            panic!("expected activated ability");
        };
        let mut effects = activated.effects.segments[0].default_effects.clone();
        assert!(describe_face_down_library_search(&effects).is_some());
        let gate = effects[0].downcast_ref::<crate::effects::ConditionalEffect>().unwrap();
        let mut gate = gate.clone();
        let grouped = gate.if_true[5]
            .downcast_ref::<crate::effects::EmitKeywordActionEffect>()
            .expect("turn-history grouping record");
        let mut altered_grouped = grouped.clone();
        altered_grouped.amount = 2;
        let mut wrong_group = gate.clone();
        wrong_group.if_true[5] = Effect::new(altered_grouped);
        assert!(describe_face_down_library_search(&[Effect::new(wrong_group)]).is_none(),
            "different grouping event count must not receive the compact surface");
        let pick = gate.if_true[4].downcast_ref::<crate::effects::ChooseModeEffect>().unwrap();
        let mut pick = pick.clone();
        let search = pick.modes[0].effects[1].downcast_ref::<crate::effects::ChooseObjectsEffect>().unwrap();
        let mut search = search.clone();
        search.is_search = false;
        pick.modes[0].effects[1] = Effect::new(search);
        gate.if_true[4] = Effect::new(pick);
        effects[0] = Effect::new(gate);
        assert!(describe_face_down_library_search(&effects).is_none());
    }
}
