use super::*;

pub(super) fn exact_self_reference(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        trim_shape_edges(tokens),
        (
            alt((
                primitives::kw("it").void(),
                primitives::kw("this").void(),
                (
                    primitives::kw("this"),
                    alt((
                        primitives::kw("creature"),
                        primitives::kw("land"),
                        primitives::kw("permanent"),
                    )),
                )
                    .void(),
            )),
            primitives::sentence_end(),
        )
            .void(),
        "transform self reference",
    )
    .is_ok()
}

pub fn parse_transform_target_shape(tokens: &[OwnedLexToken]) -> TransformTargetShape<'_> {
    let tokens = trim_shape_edges(tokens);
    if tokens.is_empty() {
        return TransformTargetShape::ImplicitSource;
    }
    if let Some((_, filter_tokens)) =
        primitives::parse_prefix(tokens, alt((primitives::kw("all"), primitives::kw("each"))))
    {
        return TransformTargetShape::EachObject {
            filter_tokens: trim_shape_edges(filter_tokens),
        };
    }
    let words = primitives::TokenWordView::new(tokens).to_word_refs();
    if exact_self_reference(tokens) {
        return TransformTargetShape::Source {
            surface: this_source_surface_for_words(&words),
        };
    }
    if let Some(surface) =
        source_reference_surface_for_words(&words).or_else(|| this_source_surface_for_words(&words))
    {
        return TransformTargetShape::Source {
            surface: Some(surface),
        };
    }
    let fallback_to_source = words.len() <= 3
        && !words.iter().any(|word| {
            matches!(
                *word,
                "target" | "another" | "other" | "each" | "all" | "that" | "those"
            )
        });
    TransformTargetShape::Target {
        target_tokens: tokens,
        fallback_to_source,
    }
}

pub fn source_spec_for_reference(source: CounterReferenceSource) -> crate::ChooseSpec {
    match source {
        CounterReferenceSource::TaggedIt => {
            crate::ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.key())
        }
        CounterReferenceSource::Source => crate::ChooseSpec::Source,
    }
}

pub fn player_filter_for_half_reference(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Active => Some(PlayerFilter::Active),
        PlayerAst::Any => Some(PlayerFilter::Any),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::PlayerToYourLeft => Some(PlayerFilter::PlayerToYourLeft),
        PlayerAst::PlayerToYourRight => Some(PlayerFilter::PlayerToYourRight),
        PlayerAst::NotYou => Some(PlayerFilter::NotYou),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Chosen => Some(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::MostCardsInHand => Some(PlayerFilter::MostCardsInHand),
        PlayerAst::MostLifeTied => Some(PlayerFilter::MostLifeTied),
        PlayerAst::LowestLifeTied => Some(PlayerFilter::LowestLifeTied),
        PlayerAst::ThatPlayerOrTargetController
        | PlayerAst::ItsController
        | PlayerAst::ItsOwner
        | PlayerAst::Enchanted => None,
        PlayerAst::TriggeringSourceController => Some(PlayerFilter::ControllerOf(
            crate::filter::ObjectRef::tagged("triggering_source"),
        )),
    }
}
