//! Render a named player choice together with every option-scoped action.
//! The option tags, rather than card names or mode text, bind each consumer.
use super::*;

fn joint_action(effects: &[Effect]) -> Option<String> {
    let [first, second] = effects else { return None; };
    if let (Some(you), Some(that_player)) = (
        first.downcast_ref::<crate::effects::DrawCardsEffect>(),
        second.downcast_ref::<crate::effects::DrawCardsEffect>(),
    ) {
        let mut normalized_that_player = that_player.clone();
        normalized_that_player.player = PlayerFilter::You;
        if you.player == PlayerFilter::You
            && that_player.player == PlayerFilter::IteratedPlayer
            && you == &normalized_that_player
        {
            return Some(format!(
                "you and that player each draw {}",
                describe_card_count(&you.count)
            ));
        }
    }
    let you = structural_unwrap_render_wrappers(first)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    let that_player = structural_unwrap_render_wrappers(second)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if you.controller != PlayerFilter::You
        || that_player.controller != PlayerFilter::IteratedPlayer
    {
        return None;
    }
    let mut first_shape = you.clone();
    let mut second_shape = that_player.clone();
    first_shape.controller = PlayerFilter::You;
    second_shape.controller = PlayerFilter::You;
    first_shape.actor_surface_explicit = false;
    second_shape.actor_surface_explicit = false;
    // Separate creation instructions receive distinct synthetic token IDs.
    // All copiable characteristics, text roles, and executable entry flags
    // still have to match before describing them as one joint action.
    second_shape.token.card.id = first_shape.token.card.id;
    // Token blueprints also carry per-instance ability ids (a Treasure's mana
    // ability), so fall back to comparing the rendered creations with the
    // creator normalized.
    if first_shape != second_shape
        && describe_effect(&Effect::new(first_shape.clone()))
            != describe_effect(&Effect::new(second_shape.clone()))
    {
        return None;
    }
    let token = describe_effect(first).strip_prefix("Create ")?.to_string();
    Some(format!("you and that player each create {token}"))
}

fn option_action(
    choice: &crate::effects::ChoosePlayerOptionEffect,
    option: &str,
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.starting_with_controller || for_players.stop_after_first_happened {
        return None;
    }
    let person_noun = matches!(choice.chooser, crate::effects::PlayerOptionChooser::Controller)
        && choice.participants == PlayerFilter::Any;
    let named_subject = person_noun.then(|| format!("Each {option}"));
    if let Some(subject) = named_subject.as_deref()
        && describe_for_players_subject(&for_players.filter) == Some(subject)
        && let Some(action) = choose_one_then_tap_rest(for_players)
    {
        return Some(format!("{subject} {action}"));
    }
    if let Some(action) = joint_action(&for_players.effects) {
        return Some(format!("For each player who chose {option}, {action}"));
    }
    if let Some(subject) = named_subject.as_deref()
        && describe_for_players_subject(&for_players.filter) == Some(subject)
    {
        if let Some(rendered) = describe_for_players_simple_iterated_action(for_players)
            .or_else(|| describe_for_players_iterated_action_sequence(for_players))
        {
            return Some(rendered);
        }
    }
    let [effect] = for_players.effects.as_slice() else { return None; };
    // "puts a counter on each creature they control" may lower either as one
    // all-filter put or as a per-object iteration putting on the iterated one.
    let (put, filter) = if let Some(put) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
        let ChooseSpec::All(filter) = put.target.base() else { return None; };
        (put, filter)
    } else {
        let each = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ForEachObject>()?;
        let [inner] = each.effects.as_slice() else { return None; };
        let put = inner.downcast_ref::<crate::effects::PutCountersEffect>()?;
        if !matches!(put.target.base(), ChooseSpec::Iterated) {
            return None;
        }
        (put, &each.filter)
    };
    if put.target_count.is_some()
        || put.distributed
        || put.required_recipient.is_some()
        || put.maximum_total.is_some()
        || put.completion_action.is_some()
        || put.counter_kind_from_trigger
        || filter.controller != Some(PlayerFilter::IteratedPlayer)
        || filter.zone != Some(Zone::Battlefield)
    {
        return None;
    }
    let mut generic_filter = filter.clone();
    generic_filter.controller = None;
    // The object noun can carry authored pronoun/article presentation from
    // "each creature they control" without changing the affected set.
    generic_filter.union_surface = Default::default();
    if generic_filter != ObjectFilter::default()
        .with_type(*filter.card_types.first()?)
        .in_zone(Zone::Battlefield)
        || filter.card_types.len() != 1
    {
        return None;
    }
    let subject = named_subject?;
    if describe_for_players_subject(&for_players.filter) != Some(subject.as_str()) {
        return None;
    }
    let action = normalize_third_person_verb_phrase(&lowercase_first(&describe_effect(effect)));
    let action = rewrite_iterated_player_references(&action);
    Some(format!("{subject} {action}"))
}

/// "chooses one untapped creature they control, then taps the rest": the
/// iterated player's single choice, then a tap of the same population minus
/// the chosen object (proved by the negative tag constraint).
fn choose_one_then_tap_rest(for_players: &crate::effects::ForPlayersEffect) -> Option<String> {
    let [only] = for_players.effects.as_slice() else { return None; };
    let sequence = structural_unwrap_render_wrappers(only)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    let (choose_effect, tap_effect) = match sequence.effects.as_slice() {
        [choose, tag, tap]
            if structural_unwrap_render_wrappers(tag)
                .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
                .is_some() =>
        {
            (choose, tap)
        }
        [choose, tap] => (choose, tap),
        _ => return None,
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let tap = structural_unwrap_render_wrappers(tap_effect)
        .downcast_ref::<crate::effects::TapEffect>()?;
    if choose.chooser != PlayerFilter::IteratedPlayer
        || !choose.count.is_single()
        || choose.is_search
        || choose.reveal
        || choose.filter.controller != Some(PlayerFilter::IteratedPlayer)
        || tap.actor.is_some()
    {
        return None;
    }
    let ChooseSpec::All(rest) = tap.target.base() else { return None; };
    let [constraint] = rest.tagged_constraints.as_slice() else { return None; };
    if constraint.relation != crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
        || constraint.tag != choose.tag
        || rest.card_types != choose.filter.card_types
        || rest.subtypes != choose.filter.subtypes
        || rest.controller != choose.filter.controller
        || rest.untapped != choose.filter.untapped
    {
        return None;
    }
    let selection = describe_choose_selection(choose)
        .replace(" that player controls", " they control");
    Some(format!("chooses {selection}, then taps the rest"))
}

pub(in crate::compiled_text::render_effects) fn describe(effects: &[Effect]) -> Option<String> {
    let [choice_effect, rest @ ..] = effects else { return None; };
    let choice = choice_effect.downcast_ref::<crate::effects::ChoosePlayerOptionEffect>()?;
    let plain_choice = crate::effects::ChoosePlayerOptionEffect::new(
        choice.participants.clone(),
        choice.options.clone(),
        choice.chooser,
    );
    if choice.options.len() < 2
        || choice.options.len() != rest.len()
        || choice != &plain_choice
        || !matches!(choice.participants, PlayerFilter::Any | PlayerFilter::Opponent)
    {
        return None;
    }
    let mut sentences = vec![describe_effect(choice_effect)];
    for (option, effect) in choice.options.iter().zip(rest) {
        let for_players = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ForPlayersEffect>()?;
        if for_players.filter
            != PlayerFilter::TaggedPlayer(crate::effects::player_option_choice_tag(option))
        {
            return None;
        }
        sentences.push(option_action(choice, option, for_players)?);
    }
    Some(sentences.join(". "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_player_options_keep_all_scoped_actions() {
        let text = "For each player, choose friend or foe. Each friend puts a +1/+1 counter on each creature they control. Each foe draws a card.";
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Option Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .expect("option procedure compiles");
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(rendered.contains("Each friend puts a +1/+1 counter"), "{rendered}");
        assert!(rendered.contains("Each foe draws a card"), "{rendered}");
        assert!(!rendered.contains("for each that player"), "{rendered}");
    }

    #[test]
    fn option_compactor_rejects_wrong_group_tag_or_extra_effect() {
        let options = vec!["alpha".to_string(), "beta".to_string()];
        let choice = Effect::new(crate::effects::ChoosePlayerOptionEffect::new(
            PlayerFilter::Opponent,
            options.clone(),
            crate::effects::PlayerOptionChooser::Participant,
        ));
        let scoped_draw = |option: &str| Effect::new(crate::effects::ForPlayersEffect::new(
            PlayerFilter::TaggedPlayer(crate::effects::player_option_choice_tag(option)),
            vec![
                Effect::new(crate::effects::DrawCardsEffect::you(1)),
                Effect::new(crate::effects::DrawCardsEffect::new(1, PlayerFilter::IteratedPlayer)),
            ],
        ));
        let exact = vec![choice, scoped_draw("alpha"), scoped_draw("beta")];
        assert!(describe(&exact).is_some());
        let mut wrong_tag = exact.clone();
        wrong_tag[2] = scoped_draw("gamma");
        assert!(describe(&wrong_tag).is_none());
        let mut extra = exact;
        extra.push(Effect::gain_life(1));
        assert!(describe(&extra).is_none());
    }

    #[test]
    fn participant_options_preserve_joint_actors() {
        let text = "Each opponent chooses money or secrets. For each player who chose money, you and that player each create a Treasure token. For each player who chose secrets, you and that player each draw a card.";
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Joint Option Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .expect("joint option procedure compiles");
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(rendered.contains("you and that player each create a Treasure token"), "{rendered}");
        assert!(rendered.contains("you and that player each draw a card"), "{rendered}");
    }
}
