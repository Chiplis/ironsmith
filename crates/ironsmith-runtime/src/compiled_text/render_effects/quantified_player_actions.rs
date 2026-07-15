use super::*;

pub(super) fn describe_each_player_shuffle_hand_then_draw(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Any {
        return None;
    }
    let [shuffle_effect, draw_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let shuffle_effect = shuffle_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| tagged.effect.as_ref())
        .unwrap_or(shuffle_effect);
    let with_id = shuffle_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let shuffle = with_id
        .effect
        .downcast_ref::<crate::effects::ShuffleObjectsIntoLibraryEffect>()?;
    if shuffle.player != PlayerFilter::IteratedPlayer || shuffle.owner_library_destination {
        return None;
    }
    let filter = match shuffle.target.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
        _ => return None,
    };
    if filter.zone != Some(Zone::Hand)
        || filter.owner != Some(PlayerFilter::IteratedPlayer)
        || filter.controller.is_some()
    {
        return None;
    }
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::IteratedPlayer
        || !draw.count.has_surface_hint(ValueSurfaceHint::ThatManyCards)
        || !matches!(draw.count.unhinted(), Value::EffectValue(id) if *id == with_id.id)
    {
        return None;
    }
    Some(
        "Each player shuffles the cards from their hand into their library, then draws that many cards"
            .to_string(),
    )
}

/// Render sibling actions performed by the same quantified opponent as one
/// coordinated list. The runtime program remains unchanged; this only retains
/// the shared subject that was present in the parsed player loop.
pub(super) fn describe_for_players_coordinated_actions(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Opponent {
        return None;
    }
    if let Some(compact) = describe_opponent_discard_lose_exile_top(for_players) {
        return Some(compact);
    }
    if let Some(compact) = describe_for_players_repeated_sacrifice(for_players) {
        return Some(compact);
    }

    let rendered = describe_for_players_iterated_action_sequence(for_players)?;
    let Some((prefix, last)) = rendered.rsplit_once(", then ") else {
        return Some(rendered);
    };
    let subject = describe_for_players_subject(&for_players.filter)?;
    let action_prefix = prefix.strip_prefix(&format!("{subject} "))?;
    let conjunction = if action_prefix.contains(", ") {
        ", and "
    } else {
        " and "
    };
    Some(format!("{prefix}{conjunction}{last}"))
}

/// Keep a quantified participant as the actor when a prior action determines
/// how many permanents that participant sacrifices. The runtime keeps the
/// repeated choice explicit, while the authored surface places the count after
/// the sacrifice clause ("for each card discarded this way").
fn describe_for_players_repeated_sacrifice(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let [repeat_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let repeat = repeat_effect.downcast_ref::<crate::effects::RepeatEffectsEffect>()?;
    let basis = describe_turn_history_for_each_basis(&repeat.count)?;
    let [choose_effect, sacrifice_effect] = repeat.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view(sacrifice_effect)?;
    let inner = describe_choose_then_sacrifice(choose, sacrifice)?;
    let subject = describe_for_players_subject(&for_players.filter)?;
    let subject_lower = lowercase_first(subject);
    let action = iterated_player_action_phrase(&inner, subject, &subject_lower)?;
    Some(format!("{subject} {action} for each {basis}"))
}

/// Join a controller/source action with an adjacent opponent action without
/// losing either actor. Only typed, simple action families are accepted.
pub(super) fn describe_coordinated_controller_opponent_bundle(
    effects: &[&Effect],
) -> Option<String> {
    match effects {
        [controller_effect, opponent_effect] => {
            if let Some(opponent) = opponent_action_text(opponent_effect) {
                return coordinate_controller_then_opponents(controller_effect, &opponent);
            }
            let opponent = opponent_action_text(controller_effect)?;
            let controller = controller_imperative_phrase(opponent_effect)?;
            Some(format!("{opponent} and you {controller}"))
        }
        [opponent_effect, first_controller, second_controller] => {
            let opponent = opponent_action_text(opponent_effect)?;
            let first = controller_imperative_phrase(first_controller)?;
            let second = controller_imperative_phrase(second_controller)?;
            Some(format!("{opponent}. You {first} and {second}"))
        }
        _ => None,
    }
}

fn opponent_action_text(effect: &Effect) -> Option<String> {
    let for_players =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Opponent {
        return None;
    }
    describe_for_players_coordinated_actions(for_players)
        .or_else(|| describe_for_players_simple_iterated_action(for_players))
}

enum ControllerActionPhrase {
    Imperative(String),
    Source(String),
}

fn coordinate_controller_then_opponents(effect: &Effect, opponent: &str) -> Option<String> {
    match controller_action_phrase(effect)? {
        ControllerActionPhrase::Imperative(first) => Some(format!(
            "{} and {}",
            capitalize_first(&first),
            lowercase_first(opponent)
        )),
        ControllerActionPhrase::Source(first) => {
            coordinate_repeated_source_subject(&first, opponent)
                .or_else(|| Some(format!("{first} and {}", lowercase_first(opponent))))
        }
    }
}

fn controller_imperative_phrase(effect: &Effect) -> Option<String> {
    match controller_action_phrase(effect)? {
        ControllerActionPhrase::Imperative(text) => Some(text),
        ControllerActionPhrase::Source(_) => None,
    }
}

fn controller_action_phrase(effect: &Effect) -> Option<ControllerActionPhrase> {
    let effect = unwrap_basic_tag_wrappers(effect);
    let imperative = if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
        draw.player == PlayerFilter::You
    } else if let Some(gain) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
        gain.player == ChooseSpec::Player(PlayerFilter::You)
    } else if let Some(create) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
        create.controller == PlayerFilter::You
    } else if let Some(put) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
        matches!(put.target.base(), ChooseSpec::Source)
    } else if let Some(return_to_hand) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
    {
        return_to_hand.graveyard_player_surface == Some(PlayerFilter::You)
            && return_to_hand.destination_player_surface == Some(PlayerFilter::You)
    } else {
        false
    };
    if imperative {
        let rendered = lowercase_first(describe_effect(effect).trim_end_matches('.'));
        let rendered = rendered.strip_prefix("you ").unwrap_or(&rendered);
        return Some(ControllerActionPhrase::Imperative(
            normalize_you_verb_phrase(rendered),
        ));
    }

    let apply = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.target != crate::continuous::EffectTarget::Source {
        return None;
    }
    Some(ControllerActionPhrase::Source(
        describe_effect(effect).trim_end_matches('.').to_string(),
    ))
}

fn coordinate_repeated_source_subject(first: &str, second: &str) -> Option<String> {
    const SOURCE_SUBJECTS: &[&str] = &[
        "This spell",
        "This card",
        "This permanent",
        "This artifact",
        "This creature",
        "This enchantment",
        "This planeswalker",
        "This land",
    ];
    for subject in SOURCE_SUBJECTS {
        let Some(first_body) = first
            .strip_prefix(subject)
            .and_then(|text| text.strip_prefix(' '))
        else {
            continue;
        };
        let lower_subject = subject.to_ascii_lowercase();
        let Some(second_body) = second
            .strip_prefix(subject)
            .or_else(|| second.strip_prefix(&lower_subject))
            .and_then(|text| text.strip_prefix(' '))
        else {
            continue;
        };
        return Some(format!("{subject} {first_body} and {second_body}"));
    }
    None
}

fn describe_opponent_discard_lose_exile_top(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let [discard_effect, lose_effect, exile_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let discard = unwrap_basic_tag_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    let lose =
        unwrap_basic_tag_wrappers(lose_effect).downcast_ref::<crate::effects::LoseLifeEffect>()?;
    let exile = unwrap_basic_tag_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let discarded_tag = discard.tag.as_ref()?;
    let exile_is_same_player = exile.player == PlayerFilter::IteratedPlayer
        || matches!(
            &exile.player,
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
                if tag == discarded_tag
        );
    if discard.player != PlayerFilter::IteratedPlayer
        || discard.random
        || discard.any_number
        || lose.player != ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        || !exile_is_same_player
    {
        return None;
    }

    let top = match exile.count.unhinted() {
        Value::Fixed(1) => "the top card".to_string(),
        count => format!("the top {} cards", describe_value(count)),
    };
    Some(format!(
        "Each opponent discards {}, loses {}, and exiles {top} of their library",
        describe_discard_count(&discard.count, discard.card_filter.as_ref()),
        describe_life_amount_phrase(&lose.amount),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_same_opponent_discard_and_life_loss() {
        let for_players = crate::effects::ForPlayersEffect::new(
            PlayerFilter::Opponent,
            vec![
                Effect::new(crate::effects::DiscardEffect::new(
                    1,
                    PlayerFilter::IteratedPlayer,
                    false,
                )),
                Effect::new(crate::effects::LoseLifeEffect::with_filter(
                    2,
                    PlayerFilter::IteratedPlayer,
                )),
            ],
        );
        assert_eq!(
            describe_for_players_coordinated_actions(&for_players).as_deref(),
            Some("Each opponent discards a card and loses 2 life")
        );
    }

    #[test]
    fn coordinates_controller_draw_with_opponent_life_loss() {
        let draw = Effect::new(crate::effects::DrawCardsEffect::you(1));
        let opponents = Effect::new(crate::effects::ForPlayersEffect::new(
            PlayerFilter::Opponent,
            vec![Effect::new(crate::effects::LoseLifeEffect::with_filter(
                1,
                PlayerFilter::IteratedPlayer,
            ))],
        ));
        assert_eq!(
            describe_coordinated_controller_opponent_bundle(&[&draw, &opponents]).as_deref(),
            Some("Draw a card and each opponent loses 1 life")
        );
    }

    #[test]
    fn preserves_for_each_surface_for_controller_relative_opponent_loss() {
        let for_players = crate::effects::ForPlayersEffect::new(
            PlayerFilter::Opponent,
            vec![Effect::new(crate::effects::LoseLifeEffect::with_filter(
                Value::Count(ObjectFilter::creature().controlled_by(PlayerFilter::You))
                    .with_surface_hint(ValueSurfaceHint::ForEach),
                PlayerFilter::IteratedPlayer,
            ))],
        );

        assert_eq!(
            describe_for_players_simple_iterated_action(&for_players).as_deref(),
            Some("Each opponent loses 1 life for each creature you control")
        );
    }

    #[test]
    fn preserves_for_each_surface_and_multiplier_for_iterated_player_loss() {
        let for_players = crate::effects::ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::new(crate::effects::LoseLifeEffect::with_filter(
                Value::CountScaled(
                    ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                    2,
                )
                .with_surface_hint(ValueSurfaceHint::ForEach),
                PlayerFilter::IteratedPlayer,
            ))],
        );

        assert_eq!(
            describe_for_players_simple_iterated_action(&for_players).as_deref(),
            Some("Each player loses 2 life for each creature they control")
        );
    }

    #[test]
    fn preserves_for_each_surface_for_iterated_player_graveyard_loss() {
        let for_players = crate::effects::ForPlayersEffect::new(
            PlayerFilter::Opponent,
            vec![Effect::new(crate::effects::LoseLifeEffect::with_filter(
                Value::Count(
                    ObjectFilter::default()
                        .in_zone(Zone::Graveyard)
                        .owned_by(PlayerFilter::IteratedPlayer)
                        .with_type(CardType::Creature),
                )
                .with_surface_hint(ValueSurfaceHint::ForEach),
                PlayerFilter::IteratedPlayer,
            ))],
        );

        assert_eq!(
            describe_for_players_simple_iterated_action(&for_players).as_deref(),
            Some("Each opponent loses 1 life for each creature card in their graveyard")
        );
    }
}
