use super::*;

/// Preserve the shared player subject for the common life-change/token pair.
///
/// The generic coordinated fallback intentionally turns a leading `You ...`
/// clause into an imperative. That is correct for many resolving spell
/// instructions, but it produces the grammatically mixed
/// `lose 1 life and you create ...` when the second action retains its
/// explicit actor surface. Prove that both executable actions belong to the
/// controller before rendering the subject once.
pub(in crate::compiled_text) fn describe_you_life_change_and_create_token(
    effects: &[Effect],
) -> Option<String> {
    let [life_root, create_root] = effects else {
        return None;
    };

    let life_effect = structural_unwrap_render_wrappers(life_root);
    let life_is_yours = life_effect
        .downcast_ref::<crate::effects::GainLifeEffect>()
        .is_some_and(|gain| gain.player == ChooseSpec::Player(PlayerFilter::You))
        || life_effect
            .downcast_ref::<crate::effects::LoseLifeEffect>()
            .is_some_and(|lose| lose.player == ChooseSpec::Player(PlayerFilter::You));
    if !life_is_yours {
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

    let life = describe_effect(life_root);
    let life = life.trim().trim_end_matches('.');
    let create = describe_effect(create_root);
    let create = create.trim().trim_end_matches('.');
    if !matches!(life.split_once(' '), Some(("You" | "you", _))) {
        return None;
    }
    let create = create
        .strip_prefix("You ")
        .or_else(|| create.strip_prefix("you "))?;
    if !create.starts_with("create ") {
        return None;
    }

    Some(format!("{} and {create}", capitalize_first(life)))
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
            describe_you_life_change_and_create_token(&[lose.clone(), create.clone()]),
            Some("You lose 1 life and create a Treasure token".to_string())
        );

        let other_player = Effect::new(crate::effects::LoseLifeEffect {
            amount: Value::Fixed(1),
            player: ChooseSpec::Player(PlayerFilter::Opponent),
        });
        assert_eq!(
            describe_you_life_change_and_create_token(&[other_player, create]),
            None
        );
    }
}
