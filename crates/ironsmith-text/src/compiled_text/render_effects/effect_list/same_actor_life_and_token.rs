use super::*;

/// Preserve the shared player subject for the player-action/token pair.
///
/// The generic coordinated fallback intentionally turns a leading `You ...`
/// clause into an imperative. That is correct for many resolving spell
/// instructions, but it produces the grammatically mixed
/// `lose 1 life and you create ...` when the second action retains its
/// explicit actor surface. Prove that both executable actions belong to the
/// controller before rendering the subject once.
pub(in crate::compiled_text) fn describe_you_action_and_create_token(
    effects: &[Effect],
) -> Option<String> {
    let [action_root, create_root] = effects else {
        return None;
    };

    let action_effect = structural_unwrap_render_wrappers(action_root);
    let action_is_yours = action_effect
        .downcast_ref::<crate::effects::GainLifeEffect>()
        .is_some_and(|gain| gain.player == ChooseSpec::Player(PlayerFilter::You))
        || action_effect
            .downcast_ref::<crate::effects::LoseLifeEffect>()
            .is_some_and(|lose| lose.player == ChooseSpec::Player(PlayerFilter::You))
        || action_effect
            .downcast_ref::<crate::effects::TakeInitiativeEffect>()
            .is_some_and(|initiative| initiative.player == PlayerFilter::You);
    if !action_is_yours {
        return None;
    }

    let create = structural_unwrap_render_wrappers(create_root)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if create.controller != PlayerFilter::You
        || create.controller_target.is_some()
        || !create.actor_surface_explicit
    {
        return None;
    }

    let action = describe_effect(action_root);
    let action = action.trim().trim_end_matches('.');
    let create = describe_effect(create_root);
    let create = create.trim().trim_end_matches('.');
    let action = action
        .strip_prefix("You ")
        .or_else(|| action.strip_prefix("you "))
        .unwrap_or(action);
    if !action.starts_with("gain ") && !action.starts_with("lose ") && action != "take the initiative" {
        return None;
    }
    let create = create
        .strip_prefix("You ")
        .or_else(|| create.strip_prefix("you "))?;
    if !create.starts_with("create ") {
        return None;
    }

    Some(format!("You {action} and {create}"))
}

/// Preserve one explicit controller subject across a life change and a
/// library-owner exile. `LibraryOwnerAsActor` proves that the second action
/// was authored as "you exile" rather than as an imperative instruction.
pub(in crate::compiled_text) fn describe_you_life_change_and_exile_top(
    effects: &[Effect],
) -> Option<String> {
    let [life_root, exile_root] = effects else {
        return None;
    };
    let life = structural_unwrap_render_wrappers(life_root)
        .downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if life.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }
    let exile = structural_unwrap_render_wrappers(exile_root)
        .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if exile.player != PlayerFilter::You
        || exile.surface != Some(ironsmith_core::ExileTopLibrarySurface::LibraryOwnerAsActor)
    {
        return None;
    }

    let life_text = describe_effect(life_root);
    let life_action = strip_you_action(&life_text)?;
    if !life_action.starts_with("lose ") {
        return None;
    }
    let exile_text = describe_effect(exile_root);
    let exile_action = strip_you_action(&exile_text)?;
    let exile_action = normalize_you_verb_phrase(exile_action);
    if !exile_action.starts_with("exile ") {
        return None;
    }
    Some(format!("You {life_action} and {exile_action}"))
}

/// Render a coordinated list of simple token creations with one explicit
/// non-controller actor only once. The common controller value proves that
/// every list item belongs to the same player; the single-clause surface
/// guard prevents token definitions with follow-up sentences from being
/// folded into the list.
pub(in crate::compiled_text) fn describe_shared_actor_token_creation_list(
    effects: &[Effect],
) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }
    let mut shared_controller = None;
    let mut shared_subject = None;
    let mut token_items = Vec::with_capacity(effects.len());
    for effect in effects {
        let create = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::CreateTokenEffect>()?;
        if create.controller_target.is_some() {
            return None;
        }
        if let Some(controller) = &shared_controller {
            if controller != &create.controller {
                return None;
            }
        } else {
            shared_controller = Some(create.controller.clone());
        }

        let rendered = describe_effect(effect);
        let rendered = rendered.trim().trim_end_matches('.');
        if rendered.contains(". ") {
            return None;
        }
        let (subject, item) = if let Some(item) = rendered.strip_prefix("Create ") {
            ("Create".to_string(), item)
        } else if let Some(item) = rendered.strip_prefix("You create ") {
            ("You create".to_string(), item)
        } else {
            let (subject, item) = rendered.split_once(" creates ")?;
            (format!("{subject} creates"), item)
        };
        if subject.is_empty() || item.is_empty() {
            return None;
        }
        if let Some(expected) = &shared_subject {
            if expected != &subject {
                return None;
            }
        } else {
            shared_subject = Some(subject.to_string());
        }
        token_items.push(item.to_string());
    }

    Some(format!(
        "{} {}",
        shared_subject?,
        join_with_and(&token_items)
    ))
}

fn strip_you_action(text: &str) -> Option<&str> {
    let text = text.trim().trim_end_matches('.');
    if text.is_empty() || text.contains(". ") {
        return None;
    }
    text.strip_prefix("You ")
        .or_else(|| text.strip_prefix("you "))
}

/// Preserve an explicitly authored shared `you` subject across a coordinated
/// list of actions.
///
/// Lowering keeps the actors on the individual effects. The broad sequence
/// renderer normally turns a leading `You ...` instruction into an
/// imperative, which produces mixed clauses such as `draw ..., gain ..., and
/// you create ...`. The typed player fields prove that every action has the
/// same actor before rendering that subject once.
pub(in crate::compiled_text) fn describe_explicit_you_action_sequence(
    effects: &[Effect],
) -> Option<String> {
    let simple_actions = || -> Option<Vec<String>> {
        if effects.len() < 3 { return None; }
        let mut actions = Vec::new();
        let mut explicit_actor = false;
        for root in effects {
            let effect = structural_unwrap_render_wrappers(root);
            let is_yours = if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
                draw.player == PlayerFilter::You
            } else if let Some(gain) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
                gain.player == ChooseSpec::Player(PlayerFilter::You)
            } else if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
                lose.player == ChooseSpec::Player(PlayerFilter::You)
            } else if let Some(initiative) = effect.downcast_ref::<crate::effects::TakeInitiativeEffect>() {
                initiative.player == PlayerFilter::You
            } else if let Some(create) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
                explicit_actor |= create.actor_surface_explicit;
                create.controller == PlayerFilter::You && create.controller_target.is_none()
            } else {
                return None;
            };
            if !is_yours { return None; }
            let text = describe_effect(root);
            actions.push(strip_you_action(&text)?.to_string());
        }
        explicit_actor.then_some(actions)
    };
    if let Some(actions) = simple_actions() {
        return Some(format!("You {}", join_with_and(&actions)));
    }

    let [discard_root, lose_root, choose_root, sacrifice_root] = effects else {
        return None;
    };
    let discard = structural_unwrap_render_wrappers(discard_root)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.player != PlayerFilter::You
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
    {
        return None;
    }
    let lose = structural_unwrap_render_wrappers(lose_root)
        .downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if lose.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }
    let choose = structural_unwrap_render_wrappers(choose_root)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view_unwrapped(sacrifice_root)?;
    let sacrifice = describe_choose_then_sacrifice(choose, sacrifice)?;

    let discard_text = describe_effect(discard_root);
    let discard = strip_you_action(&discard_text)?;
    let lose_text = describe_effect(lose_root);
    let lose = strip_you_action(&lose_text)?;
    let sacrifice = strip_you_action(&sacrifice)?;
    if !discard.starts_with("discard ")
        || !lose.starts_with("lose ")
        || !sacrifice.starts_with("sacrifice ")
    {
        return None;
    }
    Some(format!("You {discard}, {lose}, and {sacrifice}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_actor_life_and_token_subject_is_rendered_once() {
        let lose = Effect::new(crate::effects::LoseLifeEffect {
            amount: Value::Fixed(1),
            player: ChooseSpec::Player(PlayerFilter::You),
        });
        let mut create = crate::effects::CreateTokenEffect::new(
            crate::cards::tokens::treasure_token_definition(),
            1,
            PlayerFilter::You,
        );
        create.actor_surface_explicit = true;
        let create = Effect::new(create);

        assert_eq!(
            describe_you_action_and_create_token(&[lose.clone(), create.clone()]),
            Some("You lose 1 life and create a Treasure token".to_string())
        );

        let gain = Effect::new(crate::effects::GainLifeEffect::you(2));
        assert_eq!(
            describe_you_action_and_create_token(&[gain, create.clone()]),
            Some("You gain 2 life and create a Treasure token".to_string())
        );

        let other_player = Effect::new(crate::effects::LoseLifeEffect {
            amount: Value::Fixed(1),
            player: ChooseSpec::Player(PlayerFilter::Opponent),
        });
        assert_eq!(
            describe_you_action_and_create_token(&[other_player, create]),
            None
        );
    }

    #[test]
    fn same_actor_life_and_library_exile_subject_is_rendered_once() {
        let lose = Effect::new(crate::effects::LoseLifeEffect::you(2));
        let exile = Effect::new(
            crate::effects::ExileTopOfLibraryEffect::new(1, PlayerFilter::You)
                .with_surface(ironsmith_core::ExileTopLibrarySurface::LibraryOwnerAsActor),
        );

        assert_eq!(
            describe_you_life_change_and_exile_top(&[lose.clone(), exile.clone()]),
            Some("You lose 2 life and exile the top card of your library".to_string())
        );
        assert_eq!(
            describe_effect(&Effect::new(crate::effects::SequenceEffect::coordinated(
                vec![lose.clone(), exile.clone()],
            ))),
            "You lose 2 life and exile the top card of your library"
        );

        let changed = Effect::new(
            crate::effects::ExileTopOfLibraryEffect::new(1, PlayerFilter::Opponent)
                .with_surface(ironsmith_core::ExileTopLibrarySurface::LibraryOwnerAsActor),
        );
        assert_eq!(
            describe_you_life_change_and_exile_top(&[lose, changed]),
            None
        );
    }

    #[test]
    fn shared_noncontroller_token_actor_is_rendered_once() {
        fn create(token: crate::CardDefinition, controller: PlayerFilter) -> Effect {
            Effect::new(crate::effects::CreateTokenEffect::new(token, 1, controller))
        }

        let enchanted = PlayerFilter::TaggedPlayer(TagKey::from("enchanted"));
        let clue = create(
            crate::cards::tokens::clue_token_definition(),
            enchanted.clone(),
        );
        let food = create(
            crate::cards::tokens::food_token_definition(),
            enchanted.clone(),
        );
        let junk = create(crate::cards::tokens::junk_token_definition(), enchanted);
        assert_eq!(
            describe_shared_actor_token_creation_list(&[clue.clone(), food.clone(), junk.clone(),]),
            Some(
                "Enchanted player creates a Clue token, a Food token, and a Junk token".to_string()
            )
        );
        assert_eq!(
            describe_effect(&Effect::new(crate::effects::SequenceEffect::coordinated(
                vec![clue.clone(), food.clone(), junk.clone()],
            ))),
            "Enchanted player creates a Clue token, a Food token, and a Junk token"
        );

        let changed = create(
            crate::cards::tokens::junk_token_definition(),
            PlayerFilter::Opponent,
        );
        assert!(describe_shared_actor_token_creation_list(&[clue, food, changed]).is_none());
    }

    #[test]
    fn longer_explicit_action_lists_require_every_action_to_share_the_actor() {
        for changed in 0..6 {
            let player = |slot| if changed == slot { PlayerFilter::Opponent } else { PlayerFilter::You };
            let mut create = crate::effects::CreateTokenEffect::new(
                crate::cards::tokens::treasure_token_definition(), 1, player(4));
            create.actor_surface_explicit = changed != 5;
            let effects = vec![
                Effect::new(crate::effects::TakeInitiativeEffect::new(player(1))),
                Effect::new(crate::effects::GainLifeEffect { amount: Value::Fixed(3), player: ChooseSpec::Player(player(2)) }),
                Effect::new(crate::effects::DrawCardsEffect::new(1, player(3))),
                Effect::new(create),
            ];
            assert_eq!(describe_explicit_you_action_sequence(&effects), if changed == 0 {
                Some("You take the initiative, gain 3 life, draw a card, and create a Treasure token".to_string())
            } else { None });
        }
    }

    #[test]
    fn explicit_three_action_subject_requires_the_same_player() {
        let draw = Effect::new(crate::effects::DrawCardsEffect::you(1));
        let gain = Effect::new(crate::effects::GainLifeEffect::you(2));
        let mut create = crate::effects::CreateTokenEffect::new(
            crate::cards::tokens::treasure_token_definition(),
            1,
            PlayerFilter::You,
        );
        create.actor_surface_explicit = true;
        let create = Effect::new(create);

        assert_eq!(
            describe_explicit_you_action_sequence(&[draw.clone(), gain, create.clone(),]),
            Some("You draw a card, gain 2 life, and create a Treasure token".to_string())
        );

        let other_draw = Effect::new(crate::effects::DrawCardsEffect::new(
            1,
            PlayerFilter::Opponent,
        ));
        assert_eq!(
            describe_explicit_you_action_sequence(&[other_draw, draw, create]),
            None
        );
    }
}

/// Pair two independently tagged creations only when their permanent size
/// assignments agree. Render each complete bundle under the same controller
/// before comparing, so token instance IDs cannot affect the shared wording.
pub(in crate::compiled_text) fn describe_shared_dynamic_token_pair(
    effects: &[Effect],
) -> Option<String> {
    let [first, first_pt, second, second_pt] = effects else {
        return None;
    };
    let first_tagged = first.downcast_ref::<crate::effects::TaggedEffect>()?;
    let second_tagged = second.downcast_ref::<crate::effects::TaggedEffect>()?;
    let first_create = first_tagged
        .effect
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    let second_create = second_tagged
        .effect
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    let first_set = unwrap_basic_tag_wrappers(first_pt)
        .downcast_ref::<crate::effects::SetBasePowerToughnessEffect>()?;
    let second_set = unwrap_basic_tag_wrappers(second_pt)
        .downcast_ref::<crate::effects::SetBasePowerToughnessEffect>()?;
    if first_create.controller != PlayerFilter::You
        || first_create.controller_target.is_some()
        || second_create.controller_target.is_some()
        || first_set.power != second_set.power
        || first_set.toughness != second_set.toughness
        || first_create.count != second_create.count
    {
        return None;
    }
    let other = match &second_create.controller {
        PlayerFilter::TaggedPlayer(_) | PlayerFilter::ChosenPlayer => "that player",
        PlayerFilter::Target(inner) if **inner == PlayerFilter::Opponent => "target opponent",
        PlayerFilter::Target(inner) if **inner == PlayerFilter::Any => "target player",
        _ => return None,
    };
    let first_rendered = describe_create_token_then_set_base_pt_bundle(&[first, first_pt])?;
    let mut normalized_create = second_create.clone();
    normalized_create.controller = PlayerFilter::You;
    let mut normalized_tagged = second_tagged.clone();
    *normalized_tagged.effect = Effect::new(normalized_create);
    let normalized = Effect::new(normalized_tagged);
    let second_rendered = describe_create_token_then_set_base_pt_bundle(&[&normalized, second_pt])?;
    if first_rendered != second_rendered || first_rendered.contains('.') {
        return None;
    }
    Some(format!(
        "You and {other} each create {}",
        first_rendered.strip_prefix("Create ")?
    ))
}
