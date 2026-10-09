//! Render the capture/partition/choice topology, never compiler tag names.
use super::*;
use super::helpers_00::*;

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

pub(super) fn describe(effects: &[&Effect]) -> Option<String> {
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
