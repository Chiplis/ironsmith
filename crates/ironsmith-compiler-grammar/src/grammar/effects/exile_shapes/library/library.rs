use super::*;

pub fn parse_exile_dynamic_top_library_shape(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Option<ExileLibraryCardsShape> {
    let tokens = trim_commas(tokens);
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
                    crate::tag::CompilerReferenceTag::Triggering.bind(),
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
