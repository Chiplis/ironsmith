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
    let source_kind = alt((
        primitives::kw("enchantment").value("enchantment"),
        primitives::kw("artifact").value("artifact"),
        primitives::kw("creature").value("creature"),
        primitives::kw("permanent").value("permanent"),
        primitives::kw("card").value("card"),
        primitives::kw("land").value("land"),
    ))
    .parse_next(input)?;
    Ok((
        owned_by_you,
        SourceExiledReference {
            surface: ironsmith_core::SourceReferenceSurface::ThisPermanentType(format!(
                "this {source_kind}"
            )),
        },
    ))
}
