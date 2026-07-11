use super::*;

fn object_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        alt((
            primitives::kw("ability"),
            primitives::kw("abilitys"),
            primitives::kw("card"),
            primitives::kw("cards"),
            primitives::kw("creature"),
            primitives::kw("creatures"),
            primitives::kw("object"),
            primitives::kw("objects"),
        )),
        alt((
            primitives::kw("permanent"),
            primitives::kw("permanents"),
            primitives::kw("planeswalker"),
            primitives::kw("planeswalkers"),
            primitives::kw("source"),
            primitives::kw("sources"),
            primitives::kw("spell"),
            primitives::kw("spells"),
        )),
    ))
    .void()
    .parse_next(input)
}

fn static_player<'a>(input: &mut LexStream<'a>) -> WResult<PlayerAst> {
    alt((
        primitives::kw("you").value(PlayerAst::You),
        (
            primitives::kw("target"),
            alt((primitives::kw("opponent"), primitives::kw("opponents"))),
        )
            .value(PlayerAst::TargetOpponent),
        (
            primitives::kw("target"),
            alt((primitives::kw("player"), primitives::kw("players"))),
        )
            .value(PlayerAst::Target),
        (
            primitives::kw("any"),
            alt((primitives::kw("player"), primitives::kw("players"))),
        )
            .value(PlayerAst::Any),
        primitives::kw("they").value(PlayerAst::That),
        (
            primitives::kw("defending"),
            alt((primitives::kw("player"), primitives::kw("players"))),
        )
            .value(PlayerAst::Defending),
        (
            primitives::kw("that"),
            alt((primitives::kw("player"), primitives::kw("players"))),
        )
            .value(PlayerAst::That),
        alt((
            primitives::phrase(&["its", "controller"]),
            primitives::phrase(&["their", "controller"]),
        ))
        .value(PlayerAst::ItsController),
        alt((
            primitives::phrase(&["its", "owner"]),
            primitives::phrase(&["their", "owner"]),
        ))
        .value(PlayerAst::ItsOwner),
    ))
    .parse_next(input)
}

fn dynamic_player<'a>(input: &mut LexStream<'a>) -> WResult<PlayerAst> {
    let mut target_or_controller = input.clone();
    if primitives::phrase(&["that", "player", "or", "that"])
        .parse_next(&mut target_or_controller)
        .is_ok()
        && object_noun.parse_next(&mut target_or_controller).is_ok()
        && alt((primitives::kw("controller"), primitives::kw("controllers")))
            .parse_next(&mut target_or_controller)
            .is_ok()
    {
        *input = target_or_controller;
        return Ok(PlayerAst::ThatPlayerOrTargetController);
    }
    primitives::kw("that").parse_next(input)?;
    object_noun.parse_next(input)?;
    let owner = alt((
        alt((primitives::kw("controller"), primitives::kw("controllers"))).value(false),
        alt((primitives::kw("owner"), primitives::kw("owners"))).value(true),
    ))
    .parse_next(input)?;
    if !owner {
        let mut or_player = input.clone();
        if primitives::phrase(&["or", "that", "player"])
            .parse_next(&mut or_player)
            .is_ok()
        {
            *input = or_player;
            return Ok(PlayerAst::ThatPlayerOrTargetController);
        }
    }
    Ok(if owner {
        PlayerAst::ItsOwner
    } else {
        PlayerAst::ItsController
    })
}

pub(crate) fn parse_delayed_player_prefix_words(
    words: &[&str],
    static_must_be_exact: bool,
) -> Option<(PlayerAst, usize)> {
    let tokens = words_to_tokens(words);
    let tokens = trimmed(&tokens);
    if let Some((player, rest)) = primitives::parse_prefix(tokens, static_player) {
        if !static_must_be_exact || trimmed(rest).is_empty() {
            return Some((player, tokens.len().saturating_sub(rest.len())));
        }
    }
    let (player, rest) = primitives::parse_prefix(tokens, dynamic_player)?;
    Some((player, tokens.len().saturating_sub(rest.len())))
}

pub(crate) fn parse_delayed_upkeep_payment_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedUpkeepPaymentShape<'_>> {
    let tokens = trimmed(tokens);
    let (_, mana_tokens) = primitives::parse_prefix(
        tokens,
        alt((
            (
                primitives::phrase(&["at", "the", "beginning", "of", "your", "next", "upkeep"]),
                opt(primitives::comma()),
                primitives::kw("pay"),
            )
                .void(),
            (
                primitives::phrase(&["at", "the", "beginning", "of", "the", "next", "upkeep"]),
                opt(primitives::comma()),
                primitives::kw("pay"),
            )
                .void(),
        )),
    )?;
    let mana_tokens = trimmed(mana_tokens);
    (!mana_tokens.is_empty()).then_some(DelayedUpkeepPaymentShape { mana_tokens })
}

pub(crate) fn parse_next_end_step_prefix_remainder(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let (_, remainder) = primitives::parse_prefix(
        trimmed(tokens),
        alt((
            primitives::phrase(&["at", "the", "beginning", "of", "the", "next", "end", "step"]),
            primitives::phrase(&["at", "the", "beginning", "of", "next", "end", "step"]),
        )),
    )?;
    Some(trimmed(remainder))
}

pub(crate) fn split_delayed_payment_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedPaymentActionSplit<'_>> {
    let tokens = trimmed(tokens);
    let (action_start, _, _) = primitives::find_prefix(tokens, || {
        alt((primitives::kw("pay"), primitives::kw("pays")))
    })?;
    Some(DelayedPaymentActionSplit {
        player_tokens: trimmed(&tokens[..action_start]),
        action_tokens: trimmed(&tokens[action_start..]),
    })
}

pub(crate) fn parse_implicit_become_subject_shape(
    tokens: &[OwnedLexToken],
) -> Option<ImplicitBecomeSubjectShape<'_>> {
    let tokens = trimmed(tokens);
    let (kind, remainder_tokens) = primitives::parse_prefix(
        tokens,
        alt((
            alt((
                primitives::phrase(&["this", "permanent"]),
                primitives::phrase(&["this", "creature"]),
                primitives::phrase(&["this", "land"]),
                primitives::kw("this").void(),
            ))
            .value(ImplicitBecomeSubjectKind::Source),
            alt((
                primitives::phrase(&["each", "of", "them"]),
                alt((
                    primitives::kw("they're"),
                    primitives::kw("they’re"),
                    primitives::kw("theyre"),
                ))
                .void(),
                primitives::phrase(&["they", "are"]),
                primitives::kw("they").void(),
                alt((
                    primitives::kw("it's"),
                    primitives::kw("it’s"),
                    primitives::kw("its"),
                ))
                .void(),
                primitives::kw("it").void(),
            ))
            .value(ImplicitBecomeSubjectKind::Tagged),
        )),
    )?;
    Some(ImplicitBecomeSubjectShape {
        kind,
        remainder_tokens: trimmed(remainder_tokens),
    })
}

pub(crate) fn is_known_fallback_marker_shape(tokens: &[OwnedLexToken]) -> bool {
    delayed_starts_any_shape(tokens, KNOWN_FALLBACK_MARKER_PREFIXES)
}

pub(crate) fn parse_delayed_timing_marker_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedTimingMarkerShape> {
    let patterns: &'static [(&'static [&'static str], DelayedTimingStepShape, PlayerAst)] = &[
        (
            &["at", "the", "beginning", "of", "your", "next", "upkeep"],
            DelayedTimingStepShape::Upkeep,
            PlayerAst::You,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "upkeep",
                "step",
            ],
            DelayedTimingStepShape::Upkeep,
            PlayerAst::You,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "draw",
                "step",
            ],
            DelayedTimingStepShape::DrawStep,
            PlayerAst::You,
        ),
        (
            &["at", "the", "beginning", "of", "their", "next", "upkeep"],
            DelayedTimingStepShape::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "their",
                "next",
                "upkeep",
                "step",
            ],
            DelayedTimingStepShape::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "their",
                "next",
                "draw",
                "step",
            ],
            DelayedTimingStepShape::DrawStep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "that",
                "players",
                "next",
                "upkeep",
            ],
            DelayedTimingStepShape::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "that",
                "players",
                "next",
                "upkeep",
                "step",
            ],
            DelayedTimingStepShape::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "that",
                "players",
                "next",
                "draw",
                "step",
            ],
            DelayedTimingStepShape::DrawStep,
            PlayerAst::That,
        ),
    ];
    let mut best: Option<(
        usize,
        &'static [&'static str],
        DelayedTimingStepShape,
        PlayerAst,
    )> = None;
    for &(phrase, step, player) in patterns {
        let Some((token_start, _, _)) = primitives::find_prefix(tokens, || semantic_phrase(phrase))
        else {
            continue;
        };
        if best.is_none_or(|(best_start, _, _, _)| token_start < best_start) {
            best = Some((token_start, phrase, step, player));
        }
    }
    let (token_start, phrase, step, player) = best?;
    let mut start_word = 0usize;
    for token in &tokens[..token_start] {
        if token.as_word().is_some() {
            start_word += 1;
        }
    }
    Some(DelayedTimingMarkerShape {
        start_word,
        end_word: start_word + phrase.len(),
        step,
        player,
    })
}
