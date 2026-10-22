use super::*;

pub fn parse_exile_dynamic_top_library_shape(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Option<ExileLibraryCardsShape> {
    let tokens = trim_commas(tokens);
    // "exile a number of cards from the top of your library equal to the
    // number of differently named tokens you control" (Neriv): the counted
    // noun phrase reads like "cards ... equal to <value>" once the "a number
    // of" quantifier is set aside; the trailing "equal to" must supply it.
    let (tokens, a_number_of) =
        match primitives::parse_prefix(tokens, primitives::phrase(&["a", "number", "of"])) {
            Some((_, rest))
                if TokenWordView::new(rest)
                    .word_refs()
                    .windows(2)
                    .any(|window| window == ["equal", "to"]) =>
            {
                (rest, true)
            }
            _ => (tokens, false),
        };
    let shape = parse_exile_dynamic_top_library_count_shape(tokens, default_player)?;
    Some(if a_number_of {
        ExileLibraryCardsShape {
            count: shape.count.with_surface_hint(ValueSurfaceHint::ANumberOf),
            ..shape
        }
    } else {
        shape
    })
}

fn parse_exile_dynamic_top_library_count_shape(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Option<ExileLibraryCardsShape> {
    // Oracle also places the count basis before the library source:
    // "cards equal to its power from the top of its owner's library".
    // In that exact possessive-owner shape, both occurrences of "its" refer
    // to the same prior event object. Keep that LKI identity explicit rather
    // than letting the generic value parser bind the power to the ability
    // source.
    if let Some((_, after_equal)) =
        primitives::parse_prefix(tokens, (card_word, primitives::phrase(&["equal", "to"])))
        && let Some((count_tokens, position_tokens)) =
            primitives::split_lexed_once_on_separator(after_equal, || primitives::kw("from").void())
        && let Some(owner_tokens) = strip_position_and_of(trim_commas(position_tokens), "top")
    {
        let player = library_player(trim_commas(owner_tokens), default_player, false)?;
        let count_tokens = trim_commas(count_tokens);
        let (mut count, used) = values::parse_value_prefix_lexed(count_tokens)?;
        if used == count_tokens.len() {
            let count_words = TokenWordView::new(count_tokens).word_refs();
            if player == ExileLibraryPlayerShape::Player(PlayerAst::ItsOwner)
                && crate::word_primitives::parse_sequence_complete(&count_words, &["its", "power"])
                && matches!(count.unhinted(), Value::SourcePower | Value::PowerOf(_))
            {
                count = Value::PowerOf(Box::new(ChooseSpec::Tagged(
                    (crate::tag::CompilerReferenceTag::Triggering.bind()).into(),
                )));
            } else if crate::word_primitives::parse_sequence_complete(
                &count_words,
                &["its", "power"],
            ) && matches!(count.unhinted(), Value::SourcePower | Value::PowerOf(_))
            {
                count = Value::PowerOf(Box::new(crate::util::source_choose_spec_for_surface(
                    crate::target::SourceReferenceSurface::ThisPermanentType("it".to_string()),
                )));
            }
            return Some(ExileLibraryCardsShape {
                player,
                count: count.with_surface_hint(ValueSurfaceHint::EqualTo),
                face_down: false,
            });
        }
    }
    if let Some((_, after_from)) = primitives::parse_prefix(
        tokens,
        (
            alt((primitives::kw("a"), primitives::kw("one"))),
            card_word,
            primitives::kw("from"),
        ),
    ) && let Some(owner_and_count) = strip_position_and_of(trim_commas(after_from), "top")
        && let Some((owner_tokens, count_basis_tokens)) =
            primitives::split_lexed_once_on_separator(owner_and_count, || {
                primitives::phrase(&["for", "each"]).void()
            })
    {
        let mut owner_tokens = trim_commas(owner_tokens);
        let mut face_down = false;
        for suffix in [&["face", "down"][..], &["face-down"][..], &["facedown"][..]] {
            if let Some(core) = primitives::strip_lexed_suffix_phrase(owner_tokens, suffix) {
                owner_tokens = trim_commas(core);
                face_down = true;
                break;
            }
        }
        let player = library_player(owner_tokens, default_player, false)?;
        let count_basis_words = TokenWordView::new(trim_commas(count_basis_tokens)).word_refs();
        let mut count_words = Vec::with_capacity(count_basis_words.len() + 2);
        count_words.extend(["for", "each"]);
        count_words.extend(count_basis_words);
        let (count, used) = parse_for_each_count_value_words(&count_words)?;
        if used == count_words.len() {
            return Some(ExileLibraryCardsShape {
                player,
                count: count.with_surface_hint(ValueSurfaceHint::ForEach),
                face_down,
            });
        }
    }
    if let Some((_, after_cards)) = primitives::parse_prefix(tokens, card_word)
        && let Some((before_equal, after_equal)) =
            primitives::split_lexed_once_on_separator(after_cards, || {
                primitives::phrase(&["equal", "to"]).void()
            })
        && let Some((_, position_tokens)) =
            primitives::parse_prefix(trim_commas(before_equal), primitives::kw("from"))
        && let Some(owner_tokens) = strip_position_and_of(trim_commas(position_tokens), "top")
    {
        let player = library_player(trim_commas(owner_tokens), default_player, false)?;
        let (count, used) = values::parse_value_prefix_lexed(trim_commas(after_equal))?;
        if used == trim_commas(after_equal).len() {
            return Some(ExileLibraryCardsShape {
                player,
                count: count.with_surface_hint(ValueSurfaceHint::EqualTo),
                face_down: false,
            });
        }
    }
    let (count, after_from) = if let Some((_, after_from)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["that", "many"]),
            card_word,
            primitives::kw("from"),
        ),
    ) {
        // "shuffle your library, then exile that many cards from the top"
        // (Vault 112: Sadistic Simulation): a bare "the top" names the
        // library of the clause's own player, the one just shuffled.
        let bare_top = TokenWordView::new(trim_commas(after_from)).word_refs();
        if matches!(bare_top.as_slice(), ["the", "top"] | ["top"]) {
            return Some(ExileLibraryCardsShape {
                player: ExileLibraryPlayerShape::Player(default_player),
                count: Value::EventValue(EventValueSpec::Amount),
                face_down: false,
            });
        }
        (Value::EventValue(EventValueSpec::Amount), after_from)
    } else {
        let (_, after_cards) = primitives::parse_prefix(tokens, card_word)?;
        let (count_tokens, after_from) =
            primitives::split_lexed_once_on_separator(after_cards, || {
                primitives::kw("from").void()
            })?;
        if TokenWordView::new(count_tokens).is_empty() {
            return None;
        }
        let count = values::parse_add_mana_equal_amount_value_lexed(trim_commas(count_tokens))?;
        (count, after_from)
    };
    let owner_tokens = strip_position_and_of(trim_commas(after_from), "top")?;
    let player = library_player(trim_commas(owner_tokens), default_player, false)?;
    Some(ExileLibraryCardsShape {
        player,
        count,
        face_down: false,
    })
}

pub fn parse_exile_top_library_shape(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Option<ExileLibraryCardsShape> {
    let tokens = trim_commas(tokens);
    // "target opponent exiles the top half of their library, rounded up"
    // (Ulamog, the Defiler): a count computed from the owner's library size.
    let half_words = TokenWordView::new(tokens).word_refs();
    let half_words = half_words.strip_prefix(&["the"]).unwrap_or(&half_words);
    if let ["top", "half", "of", owner, "library", rounding @ ..] = half_words {
        let (player, library_owner) = match (*owner, default_player) {
            ("your", _) => (PlayerAst::You, crate::target::PlayerFilter::You),
            ("their", PlayerAst::Target) => (PlayerAst::Target, crate::target::PlayerFilter::target_player()),
            ("their", PlayerAst::TargetOpponent) => {
                (PlayerAst::TargetOpponent, crate::target::PlayerFilter::target_opponent())
            }
            ("their", PlayerAst::That) => (PlayerAst::That, crate::target::PlayerFilter::IteratedPlayer),
            _ => return None,
        };
        let base = Value::CardsInLibrary(library_owner);
        let count = match rounding {
            ["rounded", "down"] => Value::HalfRoundedDown(Box::new(base)),
            ["rounded", "up"] => Value::HalfRoundedDown(Box::new(Value::Add(
                Box::new(base),
                Box::new(Value::Fixed(1)),
            ))),
            _ => return None,
        };
        return Some(ExileLibraryCardsShape {
            player: ExileLibraryPlayerShape::Player(player),
            count,
            face_down: false,
        });
    }
    let (count, player) = if let Some((count, _implicit, owner_tokens)) =
        parse_position_count_and_owner(tokens, "top")
    {
        (count, library_player(owner_tokens, default_player, true)?)
    } else {
        let (count, _implicit) = parse_position_count_without_owner(tokens, "top")?;
        (count, ExileLibraryPlayerShape::Player(default_player))
    };
    Some(ExileLibraryCardsShape {
        player,
        count,
        face_down: false,
    })
}

pub fn parse_exile_bottom_library_shape(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Option<ExileLibraryCardsShape> {
    let tokens = trim_commas(tokens);
    // "exile that many cards from the bottom of your library" (The Temporal
    // Anchor): the triggering event's amount, from the bottom.
    if let Some((_, after_from)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["that", "many"]),
            card_word,
            primitives::kw("from"),
        ),
    ) && let Some(owner_tokens) = strip_position_and_of(trim_commas(after_from), "bottom")
    {
        let player = library_player(trim_commas(owner_tokens), default_player, false)?;
        return Some(ExileLibraryCardsShape {
            player,
            count: Value::EventValue(EventValueSpec::Amount),
            face_down: false,
        });
    }
    let (count, _, owner_tokens) = parse_position_count_and_owner(tokens, "bottom")?;
    if count != Value::Fixed(1) {
        return None;
    }
    let player = library_player(owner_tokens, default_player, true)?;
    Some(ExileLibraryCardsShape {
        player,
        count,
        face_down: false,
    })
}
