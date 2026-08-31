use super::*;

fn vote_target_noun<'a>(input: &mut LexStream<'a>) -> ModalResult<()> {
    alt((
        alt((
            primitives::kw("artifact"),
            primitives::kw("artifacts"),
            primitives::kw("battle"),
            primitives::kw("battles"),
            primitives::kw("card"),
        ))
        .void(),
        alt((
            primitives::kw("cards"),
            primitives::kw("creature"),
            primitives::kw("creatures"),
            primitives::kw("enchantment"),
            primitives::kw("enchantments"),
        ))
        .void(),
        alt((
            primitives::kw("land"),
            primitives::kw("lands"),
            primitives::kw("permanent"),
            primitives::kw("permanents"),
            primitives::kw("planeswalker"),
        ))
        .void(),
        alt((
            primitives::kw("planeswalkers"),
            primitives::kw("player"),
            primitives::kw("players"),
            primitives::kw("spell"),
            primitives::kw("spells"),
        ))
        .void(),
    ))
    .parse_next(input)
}

fn vote_target_prefix<'a>(input: &mut LexStream<'a>) -> ModalResult<()> {
    alt((
        primitives::phrase(&["up", "to"]).void(),
        primitives::kw("target").void(),
        primitives::kw("another").void(),
        primitives::kw("other").void(),
        primitives::kw("a").void(),
        primitives::kw("an").void(),
    ))
    .parse_next(input)
}

pub fn vote_options_tokens_look_like_target_choice(tokens: &[OwnedLexToken]) -> bool {
    if primitives::parse_prefix(tokens, vote_target_prefix).is_some() {
        return true;
    }
    let mut input = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), vote_target_noun)
        .parse_next(&mut input)
        .is_ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedVoteOptionEffectsShape<'a> {
    pub option_tokens: &'a [OwnedLexToken],
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VotedAgainstYouEffectsShape<'a> {
    pub effect_tokens: &'a [OwnedLexToken],
}

pub fn parse_voted_against_you_effects_shape(
    tokens: &[OwnedLexToken],
) -> Option<VotedAgainstYouEffectsShape<'_>> {
    // This shape is also used from a trigger payload whose sentence boundary
    // has already been consumed. Parse the typed relational prefix and hand
    // the remaining action clause back to ordinary effect parsing instead of
    // requiring a second synthetic sentence terminator.
    let (_, rest) = primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["each", "opponent", "who", "voted", "for", "a", "choice"]),
            // Quantified player lowering scopes the action to an iterated
            // player by rewriting `each opponent` to this contextual subject
            // before sentence dispatch. Retain the typed vote predicate when
            // receiving that equivalent internal surface.
            primitives::phrase(&["that", "players", "who", "voted", "for", "a", "choice"]),
        ))
        .void(),
    )?;
    let (_, rest) = primitives::parse_prefix(rest, primitives::kw("you").void())?;
    let (_, rest) = primitives::parse_prefix(
        rest,
        alt((
            alt((primitives::kw("didn't"), primitives::kw("didnt"))).void(),
            primitives::phrase(&["did", "not"]).void(),
            primitives::phrase(&["didn", "t"]).void(),
        )),
    )?;
    let (_, effect_tokens) =
        primitives::parse_prefix(rest, primitives::phrase(&["vote", "for"]).void())?;
    let effect_tokens = crate::lexer::trim_lexed_commas(effect_tokens);
    (!effect_tokens.is_empty()).then_some(VotedAgainstYouEffectsShape { effect_tokens })
}

fn vote_word<'a>(input: &mut LexStream<'a>) -> ModalResult<()> {
    alt((primitives::kw("vote"), primitives::kw("votes")))
        .void()
        .parse_next(input)
}

fn parse_named_vote_option_effects_lexed<'a>(
    input: &mut LexStream<'a>,
) -> ModalResult<NamedVoteOptionEffectsShape<'a>> {
    opt(primitives::kw("then")).parse_next(input)?;
    primitives::phrase(&["for", "each"]).parse_next(input)?;
    let option_tokens = repeat_till(1.., any.void(), peek(vote_word))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    vote_word.parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    let effect_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(NamedVoteOptionEffectsShape {
        option_tokens,
        effect_tokens,
    })
}

pub fn parse_named_vote_option_effects_shape(
    tokens: &[OwnedLexToken],
) -> Option<NamedVoteOptionEffectsShape<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_named_vote_option_effects_lexed,
        "named vote option effects",
    )
}
