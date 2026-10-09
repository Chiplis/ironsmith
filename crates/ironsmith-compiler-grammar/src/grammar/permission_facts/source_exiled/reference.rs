use super::*;

pub fn parse_spell_from_source_exiled_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SpellFromSourceExiledFact<'_>> {
    let ((kind, reference), tail_tokens) =
        primitives::parse_prefix(tokens, parse_spell_from_source_exiled_lexed)?;
    Some(SpellFromSourceExiledFact {
        kind,
        reference,
        tail_tokens,
    })
}

pub fn parse_spells_from_source_exiled_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SpellsFromSourceExiledFact<'_>> {
    let (scope_start, _, after_cards) =
        primitives::find_prefix(tokens, || primitives::phrase(&["from", "among", "cards"]))?;
    let subject_tokens = trim_lexed_commas(&tokens[..scope_start]);
    if subject_tokens.is_empty() {
        return None;
    }
    let ((owned_by_you, reference), tail_tokens) =
        primitives::parse_prefix(after_cards, parse_source_exiled_tail_lexed)?;
    Some(SpellsFromSourceExiledFact {
        subject_tokens,
        owned_by_you,
        reference,
        tail_tokens,
    })
}

pub(super) fn parse_spell_from_source_exiled_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(SourceExiledSpellKind, SourceExiledReference)> {
    primitives::kw("a").parse_next(input)?;
    let kind = alt((
        primitives::phrase(&["creature", "spell"]).value(SourceExiledSpellKind::Creature),
        primitives::kw("spell").value(SourceExiledSpellKind::Any),
    ))
    .parse_next(input)?;
    primitives::phrase(&["from", "among", "cards"]).parse_next(input)?;
    let (_, reference) = parse_source_exiled_tail_lexed.parse_next(input)?;
    Ok((kind, reference))
}

pub(super) fn parse_source_exiled_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(bool, SourceExiledReference)> {
    let owned_by_you = opt(primitives::phrase(&["you", "own"]))
        .parse_next(input)?
        .is_some();
    primitives::phrase(&["exiled", "with", "this"]).parse_next(input)?;
    let source_kind = opt(alt((
        primitives::kw("enchantment").value("enchantment"),
        primitives::kw("class").value("Class"),
        primitives::kw("artifact").value("artifact"),
        primitives::kw("creature").value("creature"),
        primitives::kw("permanent").value("permanent"),
        primitives::kw("card").value("card"),
        primitives::kw("land").value("land"),
    )))
    .parse_next(input)?;
    Ok((
        owned_by_you,
        SourceExiledReference {
            surface: ironsmith_core::SourceReferenceSurface::ThisPermanentType(
                source_kind.map_or_else(|| "this".to_string(), |kind| format!("this {kind}")),
            ),
        },
    ))
}

pub fn parse_cards_from_source_exiled_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(SourceExiledReference, &[OwnedLexToken])> {
    let (_, rest) = primitives::parse_prefix(tokens, primitives::kw("cards"))?;
    let ((owned_by_you, reference), tail) =
        primitives::parse_prefix(rest, parse_source_exiled_tail_lexed).or_else(|| {
            let (_, tail) =
                primitives::parse_prefix(rest, primitives::phrase(&["exiled", "with", "this"]))?;
            Some((
                (
                    false,
                    SourceExiledReference {
                        surface: ironsmith_core::SourceReferenceSurface::ThisPermanentType(
                            "this source".to_string(),
                        ),
                    },
                ),
                tail,
            ))
        })?;
    if owned_by_you {
        return None;
    }
    Some((reference, tail))
}

/// "cards you own exiled with this artifact" (Kayla's Music Box): the
/// source-linked pool narrowed to cards the permission's player owns.
pub fn parse_owned_cards_from_source_exiled_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(SourceExiledReference, &[OwnedLexToken])> {
    let (_, rest) = primitives::parse_prefix(tokens, primitives::kw("cards"))?;
    let ((owned_by_you, reference), tail) =
        primitives::parse_prefix(rest, parse_source_exiled_tail_lexed)?;
    owned_by_you.then_some((reference, tail))
}

/// A complete static land-and-spell permission over one source-linked pool.
/// Leave durations, price riders and narrower spell subjects to their owners.
pub fn parse_play_lands_and_spells_from_source_exiled_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SourceExiledReference> {
    let (_, rest) = primitives::parse_prefix(tokens, primitives::phrase(&[
        "you", "may", "play", "lands", "and", "cast", "spells", "from", "among", "cards",
    ]))?;
    let ((owned_by_you, reference), tail) =
        primitives::parse_prefix(rest, parse_source_exiled_tail_lexed)?;
    if owned_by_you || primitives::probe_all(tail, primitives::sentence_end(),
        "source-linked land and spell permission").is_none() { return None; }
    Some(reference)
}

/// The look and play clauses share the same source-linked antecedent. The
/// grammar owns that relationship; lowering receives explicit inspection.
pub fn parse_look_and_play_source_exiled_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SourceExiledReference> {
    let (_, rest) = primitives::parse_prefix(tokens,
        primitives::phrase(&["you", "may", "look", "at"]))?;
    let (_, rest) = primitives::parse_prefix(rest,
        (opt(primitives::kw("the")), primitives::kw("cards")))?;
    let ((owned_by_you, reference), tail) =
        primitives::parse_prefix(rest, parse_source_exiled_tail_lexed)?;
    if owned_by_you { return None; }
    primitives::probe_all(tail, (opt(primitives::comma()), primitives::phrase(&[
        "and", "you", "may", "play", "lands", "and", "cast", "spells", "from", "among", "those", "cards",
    ]), primitives::sentence_end()).void(), "paired exile inspection and play permission")?;
    Some(reference)
}


pub fn parse_look_source_exiled_tokens(tokens: &[OwnedLexToken]) -> Option<SourceExiledReference> {
    let (_, rest) = primitives::parse_prefix(tokens, primitives::phrase(&["you", "may", "look", "at", "cards"]))?;
    let ((owned_by_you, reference), tail) = primitives::parse_prefix(rest, parse_source_exiled_tail_lexed)?;
    if owned_by_you { return None; }
    primitives::probe_all(tail, primitives::sentence_end(), "standalone paired exile inspection")?;
    Some(reference)
}

/// The conditional rider is part of the same static source-pool permission.
pub fn parse_play_source_exiled_with_mana_tokens(tokens: &[OwnedLexToken])
    -> Option<(SourceExiledReference, ironsmith_core::value_model::ManaSpendMode)> {
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    let [permission, rider] = sentences.as_slice() else { return None; };
    let reference = parse_play_lands_and_spells_from_source_exiled_tokens(permission)?;
    let mode = super::super::tagged_surface::parse_cast_this_way_mana_rider_tokens(rider)?;
    Some((reference, mode))
}

/// A complete same-sentence rider over the same source-linked card pool.
pub fn parse_play_source_exiled_inline_mana_tokens(tokens: &[OwnedLexToken])
    -> Option<(SourceExiledReference, ironsmith_core::value_model::ManaSpendMode)> {
    use super::super::tagged_surface::{parse_allow_any_color_for_cast_suffix_tokens, ManaSpendCastReference};
    let suffix = parse_allow_any_color_for_cast_suffix_tokens(tokens)?;
    if suffix.reference != ManaSpendCastReference::ThoseSpells { return None; }
    let permission = trim_lexed_commas(suffix.body_tokens);
    let reference = if let Some(reference) = parse_play_lands_and_spells_from_source_exiled_tokens(permission) {
        reference
    } else {
        let (_, rest) = primitives::parse_prefix(permission, primitives::phrase(&["you", "may", "play"]))?;
        let (reference, tail) = parse_cards_from_source_exiled_tokens(rest)?;
        primitives::probe_all(tail, primitives::sentence_end(), "complete source-linked card permission")?;
        reference
    };
    Some((reference, suffix.mana_spend_mode))
}
