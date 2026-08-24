use super::*;

pub fn parse_protection_choice_shape(tokens: &[OwnedLexToken]) -> Option<ProtectionChoiceShape> {
    let fixed_option = alt((
        primitives::phrase(&["colorless", "or", "from"]).value((true, false)),
        primitives::phrase(&["artifacts", "or", "from"]).value((false, true)),
        winnow::combinator::empty.value((false, false)),
    ));
    let choice = alt((
        primitives::phrase(&["the", "card", "type", "of", "your", "choice"])
            .value((ProtectionChoiceChooserShape::You, true)),
        primitives::phrase(&["the", "color", "of", "your", "choice"])
            .value((ProtectionChoiceChooserShape::You, false)),
        primitives::phrase(&["the", "color", "of", "its", "controller's", "choice"])
            .value((ProtectionChoiceChooserShape::TargetController, false)),
        primitives::phrase(&["the", "color", "of", "its", "controllers", "choice"])
            .value((ProtectionChoiceChooserShape::TargetController, false)),
    ));
    primitives::parse_all(
        tokens,
        (
            primitives::phrase(&["protection", "from"]),
            fixed_option,
            choice,
            primitives::phrase(&["until", "end", "of", "turn"]),
            primitives::sentence_end(),
        )
            .map(
                |(
                    _,
                    (includes_colorless, includes_artifacts),
                    (chooser, chooses_card_type),
                    _,
                    _,
                )| {
                    ProtectionChoiceShape {
                        includes_colorless,
                        includes_artifacts,
                        chooses_card_type,
                        chooser,
                    }
                },
            ),
        "protection choice shape",
    )
    .ok()
}

pub fn strip_optional_you_choice_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::parse_prefix(tokens, primitives::kw("you"))
        .map(|(_, rest)| rest)
        .unwrap_or(tokens)
}

pub(super) fn target_phrase_excludes_chooser_controller(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        primitives::phrase(&["another", "player", "controls"])
    })
    .is_some()
}

pub fn parse_choose_target_shape(tokens: &[OwnedLexToken]) -> Option<ChooseTargetShape<'_>> {
    let (chooser, tail) = if let Some((_, tail)) = primitives::parse_prefix(
        tokens,
        (
            primitives::kw("you"),
            alt((primitives::kw("choose"), primitives::kw("chooses"))),
        ),
    ) {
        (ChooseTargetChooserShape::AbilityController, tail)
    } else if let Some((_, tail)) = primitives::parse_prefix(
        tokens,
        (
            primitives::kw("that"),
            alt((primitives::kw("opponent"), primitives::kw("opponents"))),
            alt((primitives::kw("choose"), primitives::kw("chooses"))),
        ),
    ) {
        (ChooseTargetChooserShape::ThatOpponent, tail)
    } else if let Some((_, tail)) = primitives::parse_prefix(
        tokens,
        (
            primitives::kw("its"),
            primitives::kw("controller"),
            alt((primitives::kw("choose"), primitives::kw("chooses"))),
        ),
    ) {
        (ChooseTargetChooserShape::ItsController, tail)
    } else {
        let (_, tail) = primitives::parse_prefix(
            tokens,
            alt((primitives::kw("choose"), primitives::kw("chooses"))),
        )?;
        (ChooseTargetChooserShape::AbilityController, tail)
    };
    // A target count is part of the target phrase, so `target` is not always
    // immediately adjacent to `choose` ("choose up to one target ...",
    // "choose any number of target ...").  Reuse the shared target-indicator
    // grammar instead of letting those forms fall through to resolution-time
    // object choice.
    super::super::super::super::activation_restrictions::parse_target_indicator_tokens(tail)?;
    let target_tokens = trim_lexed_commas(tail);
    Some(ChooseTargetShape {
        target_tokens,
        chooser,
        excludes_chooser_controller: target_phrase_excludes_chooser_controller(target_tokens),
    })
}

pub fn parse_embedded_choose_target_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChooseTargetShape<'_>> {
    let (choose_idx, _, target_tokens) = primitives::find_prefix(tokens, || {
        (
            alt((primitives::kw("choose"), primitives::kw("chooses"))),
            peek(primitives::kw("target")),
        )
            .void()
    })?;
    let chooser_tokens = trim_lexed_commas(&tokens[..choose_idx]);
    let chooser = if exact(chooser_tokens, primitives::kw("you").void()) {
        ChooseTargetChooserShape::AbilityController
    } else if exact(
        chooser_tokens,
        primitives::phrase(&["that", "opponent"]).void(),
    ) {
        ChooseTargetChooserShape::ThatOpponent
    } else if exact(
        chooser_tokens,
        primitives::phrase(&["its", "controller"]).void(),
    ) {
        ChooseTargetChooserShape::ItsController
    } else {
        ChooseTargetChooserShape::Unresolved
    };
    let target_tokens = trim_lexed_commas(target_tokens);
    Some(ChooseTargetShape {
        target_tokens,
        chooser,
        excludes_chooser_controller: target_phrase_excludes_chooser_controller(target_tokens),
    })
}
