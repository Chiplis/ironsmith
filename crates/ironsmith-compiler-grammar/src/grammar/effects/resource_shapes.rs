use winnow::combinator::{alt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::PlayerAst;
use crate::effect::Value;
use crate::grammar::{primitives, values};
use crate::lexer::{LexStream, LexedClause, OwnedLexToken, TokenWordView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLookObjectKind {
    FaceDownCreature,
    FaceDownPermanent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLookHandFollowup {
    None,
    TopCard,
    TopCardAndFaceDownCreatures,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceLookShape<'a> {
    PlayTaggedWhileExiled,
    Hand {
        player: PlayerAst,
        surface_tokens: &'a [OwnedLexToken],
        followup: ResourceLookHandFollowup,
    },
    EachPlayerHand,
    Object {
        kind: ResourceLookObjectKind,
        surface_tokens: &'a [OwnedLexToken],
    },
    TopCards {
        player: PlayerAst,
        count: Value,
    },
    EachPlayerTopCards {
        count: Value,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceShuffleShape {
    TaggedIntoLibrary { player: PlayerAst, to_bottom: bool },
    ShuffleLibrary { player: PlayerAst },
    SimpleLibrary,
}

#[derive(Debug, Clone, Copy)]
pub struct ResourceChosenNameTargetShape<'a> {
    pub base_tokens: &'a [OwnedLexToken],
    pub chosen_name_source: ironsmith_core::ChosenNameSourceSurface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DestinationPlayer {
    Default,
    You,
    DefaultOrController,
    That,
    ItsOwner,
}

fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    LexedClause::new(tokens).trimmed().tokens()
}

fn sentence_finished(tokens: &[OwnedLexToken]) -> bool {
    let tokens = trimmed(tokens);
    tokens.is_empty()
        || primitives::parse_all(tokens, primitives::sentence_end(), "resource sentence end")
            .is_ok()
}

/// Matches the global game action authored as “reverse the game's turn
/// order.” Apostrophe tokenization can expose the possessive as either
/// `game s` or `games`, so both normalized forms are accepted exactly.
pub fn parse_resource_reverse_turn_order_shape(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        TokenWordView::new(trimmed(tokens)).word_refs().as_slice(),
        ["the", "game", "s", "turn", "order"]
            | ["the", "games", "turn", "order"]
            | ["the", "game's", "turn", "order"]
    )
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    any.verify(move |token: &&OwnedLexToken| {
        token.is_word(expected)
            || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
    })
    .void()
}

fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn article<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("the"),
        primitives::kw("a"),
        primitives::kw("an"),
    ))
    .void()
    .parse_next(input)
}

fn card_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("card"), primitives::kw("cards")))
        .void()
        .parse_next(input)
}

fn library_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("library"), primitives::kw("libraries")))
        .void()
        .parse_next(input)
}

fn hand_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("hand"), primitives::kw("hands")))
        .void()
        .parse_next(input)
}

fn object_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        alt((
            primitives::kw("artifact"),
            primitives::kw("card"),
            primitives::kw("creature"),
        )),
        alt((
            primitives::kw("enchantment"),
            primitives::kw("permanent"),
            primitives::kw("source"),
        )),
    ))
    .void()
    .parse_next(input)
}

fn tagged_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["it"]),
        primitives::phrase(&["them"]),
        primitives::phrase(&["that", "card"]),
        primitives::phrase(&["those", "cards"]),
    ))
    .void()
    .parse_next(input)
}

fn exact_unit<'a>(
    tokens: &'a [OwnedLexToken],
    parser: fn(&mut LexStream<'a>) -> WResult<()>,
) -> bool {
    primitives::parse_prefix(trimmed(tokens), parser)
        .is_some_and(|(_, rest)| trimmed(rest).is_empty())
}

fn strip_articles(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    tokens = trimmed(tokens);
    while let Some(((), rest)) = primitives::parse_prefix(tokens, article) {
        tokens = trimmed(rest);
    }
    tokens
}

fn is_article_token(token: &OwnedLexToken) -> bool {
    exact_unit(std::slice::from_ref(token), article)
}

fn without_articles(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    tokens
        .iter()
        .filter(|token| !is_article_token(token))
        .cloned()
        .collect()
}

fn all_abilities<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["all", "abilities"]),
        primitives::phrase(&["all", "other", "abilities"]),
    ))
    .void()
    .parse_next(input)
}

pub fn parse_resource_all_abilities_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_unit(tokens, all_abilities)
}

fn all_unspent_mana<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["all", "unspent", "mana"])
        .void()
        .parse_next(input)
}

pub fn parse_resource_all_unspent_mana_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_unit(tokens, all_unspent_mana)
}

fn note_life_total<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["your", "life", "total"])
        .void()
        .parse_next(input)
}

pub fn parse_resource_note_life_total_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_unit(tokens, note_life_total)
}

fn take_extra_turn<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["an", "extra", "turn", "after", "this", "one"])
        .void()
        .parse_next(input)
}

pub fn parse_resource_take_extra_turn_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_unit(tokens, take_extra_turn)
}

fn proliferate_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["time"]),
        primitives::phrase(&["times"]),
        primitives::phrase(&["instead"]),
        primitives::phrase(&["time", "instead"]),
        primitives::phrase(&["times", "instead"]),
    ))
    .void()
    .parse_next(input)
}

pub fn parse_resource_proliferate_tail_shape(tokens: &[OwnedLexToken]) -> bool {
    trimmed(tokens).is_empty() || exact_unit(tokens, proliferate_tail)
}

fn reorder_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["as", "you", "choose"])
        .void()
        .parse_next(input)
}

pub fn parse_resource_reorder_tail_shape(tokens: &[OwnedLexToken]) -> bool {
    trimmed(tokens).is_empty() || exact_unit(tokens, reorder_tail)
}

fn it_or_them<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("it"), primitives::kw("them")))
        .void()
        .parse_next(input)
}

pub fn parse_resource_tagged_reference_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_unit(tokens, it_or_them)
}

fn hand_owner<'a>(input: &mut LexStream<'a>) -> WResult<PlayerAst> {
    let player = alt((
        primitives::kw("your").value(PlayerAst::You),
        alt((
            semantic_phrase(&["each", "player"]),
            semantic_phrase(&["each", "players"]),
        ))
        .value(PlayerAst::Any),
        primitives::kw("their").value(PlayerAst::That),
        alt((
            semantic_phrase(&["that", "player"]),
            semantic_phrase(&["that", "players"]),
        ))
        .value(PlayerAst::That),
        alt((
            semantic_phrase(&["target", "player"]),
            semantic_phrase(&["target", "players"]),
        ))
        .value(PlayerAst::Target),
        alt((
            semantic_phrase(&["target", "opponent"]),
            semantic_phrase(&["target", "opponents"]),
        ))
        .value(PlayerAst::TargetOpponent),
        alt((semantic_kw("opponent"), semantic_kw("opponents"))).value(PlayerAst::Opponent),
        semantic_phrase(&["his", "or", "her"]).value(PlayerAst::That),
    ))
    .parse_next(input)?;
    hand_noun.parse_next(input)?;
    Ok(player)
}

fn library_owner<'a>(input: &mut LexStream<'a>) -> WResult<PlayerAst> {
    let player = alt((
        primitives::kw("your").value(PlayerAst::You),
        alt((
            semantic_phrase(&["each", "player"]),
            semantic_phrase(&["each", "players"]),
        ))
        .value(PlayerAst::Any),
        primitives::kw("their").value(PlayerAst::That),
        alt((
            semantic_phrase(&["that", "player"]),
            semantic_phrase(&["that", "players"]),
        ))
        .value(PlayerAst::That),
        alt((
            semantic_phrase(&["target", "player"]),
            semantic_phrase(&["target", "players"]),
        ))
        .value(PlayerAst::Target),
        alt((
            semantic_phrase(&["target", "opponent"]),
            semantic_phrase(&["target", "opponents"]),
        ))
        .value(PlayerAst::TargetOpponent),
        alt((
            semantic_phrase(&["its", "owner"]),
            semantic_phrase(&["its", "owners"]),
        ))
        .value(PlayerAst::ItsOwner),
        semantic_phrase(&["his", "or", "her"]).value(PlayerAst::That),
    ))
    .parse_next(input)?;
    library_noun.parse_next(input)?;
    Ok(player)
}

fn top_card_same_player<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        semantic_phrase(&["the", "top", "card", "of", "that", "player", "library"]),
        semantic_phrase(&["the", "top", "card", "of", "that", "players", "library"]),
        semantic_phrase(&["top", "card", "of", "that", "player", "library"]),
        semantic_phrase(&["top", "card", "of", "that", "players", "library"]),
        semantic_phrase(&["the", "top", "card", "of", "their", "library"]),
        semantic_phrase(&["top", "card", "of", "their", "library"]),
    ))
    .void()
    .parse_next(input)
}

fn face_down<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("face-down").void(),
        primitives::kw("facedown").void(),
        primitives::phrase(&["face", "down"]),
    ))
    .parse_next(input)
}

fn face_down_same_player_creatures<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        face_down,
        alt((
            primitives::phrase(&["creatures", "they", "control"]),
            primitives::phrase(&["creature", "they", "control"]),
            primitives::phrase(&["creatures", "that", "player", "controls"]),
            primitives::phrase(&["creatures", "that", "players", "control"]),
            primitives::phrase(&["creature", "that", "player", "controls"]),
            primitives::phrase(&["creature", "that", "players", "control"]),
        )),
    )
        .void()
        .parse_next(input)
}

fn parse_hand_followup(tokens: &[OwnedLexToken]) -> Option<ResourceLookHandFollowup> {
    let ((), mut rest) = primitives::parse_prefix(trimmed(tokens), top_card_same_player)?;
    rest = trimmed(rest);
    if sentence_finished(rest) {
        return Some(ResourceLookHandFollowup::TopCard);
    }
    if let Some(((), after_and)) = primitives::parse_prefix(rest, |input: &mut LexStream<'_>| {
        primitives::kw("and").void().parse_next(input)
    }) {
        rest = trimmed(after_and);
    }
    if let Some(((), after_choice)) = primitives::parse_prefix(rest, |input: &mut LexStream<'_>| {
        alt((primitives::kw("any"), primitives::kw("all")))
            .void()
            .parse_next(input)
    }) {
        rest = trimmed(after_choice);
    }
    let ((), rest) = primitives::parse_prefix(rest, face_down_same_player_creatures)?;
    sentence_finished(rest).then_some(ResourceLookHandFollowup::TopCardAndFaceDownCreatures)
}

fn play_tagged_while_exiled<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "and", "play", "those", "cards", "for", "as", "long", "as", "they", "remain", "exiled",
    ])
    .void()
    .parse_next(input)
}

fn face_down_target<'a>(input: &mut LexStream<'a>) -> WResult<ResourceLookObjectKind> {
    (
        primitives::kw("target"),
        face_down,
        alt((
            alt((primitives::kw("creature"), primitives::kw("creatures")))
                .value(ResourceLookObjectKind::FaceDownCreature),
            alt((primitives::kw("permanent"), primitives::kw("permanents")))
                .value(ResourceLookObjectKind::FaceDownPermanent),
        )),
    )
        .map(|(_, _, kind)| kind)
        .parse_next(input)
}

fn count_before_top_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        card_noun,
        winnow::combinator::opt(primitives::kw("from")),
        repeat::<_, _, (), _, _>(0.., article),
    )
        .void()
        .parse_next(input)
}

fn parse_top_count<'a>(
    before_top: &'a [OwnedLexToken],
    after_top: &'a [OwnedLexToken],
) -> Option<(Value, &'a [OwnedLexToken])> {
    if let Some((value, used)) = values::parse_value_prefix_lexed(before_top)
        && exact_unit(&before_top[used..], count_before_top_tail)
    {
        return Some((value, trimmed(after_top)));
    }

    let after_top = trimmed(after_top);
    if let Some(((), rest)) = primitives::parse_prefix(after_top, card_noun) {
        return Some((Value::Fixed(1), trimmed(rest)));
    }
    let (value, used) = values::parse_value_prefix_lexed(after_top)?;
    let ((), rest) = primitives::parse_prefix(&after_top[used..], card_noun)?;
    Some((value, trimmed(rest)))
}

pub fn parse_resource_look_shape<'a>(
    tokens: &'a [OwnedLexToken],
    subject_player: Option<PlayerAst>,
) -> Option<ResourceLookShape<'a>> {
    let mut clause = trimmed(tokens);
    if let Some(((), rest)) = primitives::parse_prefix(clause, |input: &mut LexStream<'_>| {
        primitives::kw("at").void().parse_next(input)
    }) {
        clause = trimmed(rest);
    }

    if exact_unit(clause, play_tagged_while_exiled) {
        return Some(ResourceLookShape::PlayTaggedWhileExiled);
    }

    let hand_surface = strip_articles(clause);
    if let Some((player, rest)) = primitives::parse_prefix(hand_surface, hand_owner) {
        let rest = trimmed(rest);
        if matches!(player, PlayerAst::Any) {
            return sentence_finished(rest).then_some(ResourceLookShape::EachPlayerHand);
        }
        let followup = if sentence_finished(rest) {
            ResourceLookHandFollowup::None
        } else {
            parse_hand_followup(rest)?
        };
        return Some(ResourceLookShape::Hand {
            player,
            surface_tokens: hand_surface,
            followup,
        });
    }

    if let Some((kind, rest)) = primitives::parse_prefix(hand_surface, face_down_target)
        && sentence_finished(rest)
    {
        return Some(ResourceLookShape::Object {
            kind,
            surface_tokens: hand_surface,
        });
    }

    let (top_idx, (), after_top) =
        primitives::find_prefix(clause, || primitives::kw("top").void())?;
    let (count, rest) = parse_top_count(&clause[..top_idx], after_top)?;
    let ((), owner_surface) = primitives::parse_prefix(rest, |input: &mut LexStream<'_>| {
        primitives::kw("of").void().parse_next(input)
    })?;
    let owner_surface = strip_articles(owner_surface);
    let (player, owner_rest) =
        if let Some((player, rest)) = primitives::parse_prefix(owner_surface, library_owner) {
            (player, rest)
        } else if trimmed(owner_surface).is_empty() {
            (subject_player?, owner_surface)
        } else {
            return None;
        };
    let owner_rest = trimmed(owner_rest);
    let count = if sentence_finished(owner_rest) {
        count
    } else {
        if count != Value::X {
            return None;
        }
        let (_, value_tokens) =
            primitives::parse_prefix(owner_rest, |input: &mut LexStream<'_>| {
                primitives::phrase(&["where", "x", "is"])
                    .void()
                    .parse_next(input)
            })?;
        super::looked_card_shapes::parse_where_x_value(trimmed(value_tokens))?
    };
    if matches!(player, PlayerAst::Any) {
        Some(ResourceLookShape::EachPlayerTopCards { count })
    } else {
        Some(ResourceLookShape::TopCards { player, count })
    }
}

fn ordinal_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["from", "top"])
        .void()
        .parse_next(input)
}

fn beneath_top<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["just", "beneath", "top"]),
        primitives::phrase(&["beneath", "top"]),
    ))
    .void()
    .parse_next(input)
}

fn that_library_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["of", "that", "library"])
        .void()
        .parse_next(input)
}

pub fn parse_resource_library_position_shape(tokens: &[OwnedLexToken]) -> Option<Value> {
    let (_, (), after_library) = primitives::find_prefix(tokens, || library_noun)?;
    let normalized = without_articles(trimmed(after_library));
    if normalized.is_empty() {
        return None;
    }

    let view = TokenWordView::new(&normalized);
    let words = view.word_refs();
    if let Some((position, used_words)) = ironsmith_core::parse_ordinal_words(&words) {
        let used_tokens = view
            .token_start_indices()
            .get(used_words)
            .copied()
            .unwrap_or(normalized.len());
        if exact_unit(&normalized[used_tokens..], ordinal_tail) {
            return Some(Value::Fixed(position as i32));
        }
    }

    let ((), amount_tokens) = primitives::parse_prefix(&normalized, beneath_top)?;
    let amount_tokens = trimmed(amount_tokens);
    let (amount, used) = values::parse_value_prefix_lexed(amount_tokens)?;
    let ((), tail) = primitives::parse_prefix(&amount_tokens[used..], card_noun)?;
    if !exact_unit(tail, that_library_tail) {
        return None;
    }
    Some(Value::Add(Box::new(amount), Box::new(Value::Fixed(1))))
}

fn destination_owner<'a>(input: &mut LexStream<'a>) -> WResult<DestinationPlayer> {
    alt((
        primitives::kw("your").value(DestinationPlayer::You),
        primitives::kw("their").value(DestinationPlayer::DefaultOrController),
        alt((
            semantic_phrase(&["that", "player"]),
            semantic_phrase(&["that", "players"]),
        ))
        .value(DestinationPlayer::That),
        alt((
            semantic_phrase(&["its", "owner"]),
            semantic_phrase(&["its", "owners"]),
        ))
        .value(DestinationPlayer::ItsOwner),
        semantic_phrase(&["his", "or", "her"]).value(DestinationPlayer::DefaultOrController),
        winnow::combinator::empty.value(DestinationPlayer::Default),
    ))
    .parse_next(input)
}

fn destination<'a>(input: &mut LexStream<'a>) -> WResult<DestinationPlayer> {
    let player = destination_owner.parse_next(input)?;
    library_noun.parse_next(input)?;
    Ok(player)
}

fn resolve_destination(player: DestinationPlayer, default: PlayerAst) -> PlayerAst {
    match player {
        DestinationPlayer::Default => default,
        DestinationPlayer::You => PlayerAst::You,
        DestinationPlayer::DefaultOrController => {
            if matches!(default, PlayerAst::Implicit) {
                PlayerAst::ItsController
            } else {
                default
            }
        }
        DestinationPlayer::That => PlayerAst::That,
        DestinationPlayer::ItsOwner => PlayerAst::ItsOwner,
    }
}

fn source_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        alt((
            semantic_phrase(&["from", "your", "graveyard"]),
            semantic_phrase(&["from", "your", "graveyards"]),
            semantic_phrase(&["from", "their", "graveyard"]),
            semantic_phrase(&["from", "their", "graveyards"]),
        )),
        alt((
            semantic_phrase(&["from", "that", "player", "graveyard"]),
            semantic_phrase(&["from", "that", "player", "graveyards"]),
            semantic_phrase(&["from", "that", "players", "graveyard"]),
            semantic_phrase(&["from", "that", "players", "graveyards"]),
        )),
        alt((
            semantic_phrase(&["from", "its", "owner", "graveyard"]),
            semantic_phrase(&["from", "its", "owner", "graveyards"]),
            semantic_phrase(&["from", "its", "owners", "graveyard"]),
            semantic_phrase(&["from", "its", "owners", "graveyards"]),
        )),
        alt((
            semantic_phrase(&["from", "his", "or", "her", "graveyard"]),
            semantic_phrase(&["from", "his", "or", "her", "graveyards"]),
            semantic_phrase(&["from", "graveyard"]),
            semantic_phrase(&["from", "graveyards"]),
        )),
    ))
    .void()
    .parse_next(input)
}

fn supported_source_tail(tokens: &[OwnedLexToken]) -> bool {
    trimmed(tokens).is_empty() || exact_unit(tokens, source_tail)
}

fn the_rest<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["the", "rest"])
        .void()
        .parse_next(input)
}

fn all_other<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["all", "other"])
        .void()
        .parse_next(input)
}

fn revealed_or_exiled<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("revealed"), primitives::kw("exiled")))
        .void()
        .parse_next(input)
}

fn consult_remainder(tokens: &[OwnedLexToken]) -> bool {
    if primitives::parse_prefix(trimmed(tokens), the_rest).is_some() {
        return true;
    }
    let Some(((), rest)) = primitives::parse_prefix(trimmed(tokens), all_other) else {
        return false;
    };
    primitives::find_prefix(rest, || card_noun).is_some()
        && primitives::find_prefix(rest, || revealed_or_exiled).is_some()
}

fn required_shuffle_markers(tokens: &[OwnedLexToken]) -> bool {
    ["graveyard", "cards", "card", "into", "from"]
        .iter()
        .all(|word| primitives::find_prefix(tokens, || primitives::kw(word)).is_some())
}

fn tagged_into_their_library<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        tagged_reference,
        primitives::kw("into"),
        primitives::kw("their"),
        library_noun,
    )
        .void()
        .parse_next(input)
}

#[cfg(test)]
#[path = "resource_shapes/tests.rs"]
mod tests;

#[path = "resource_shapes/choice.rs"]
mod choice_programs;
use choice_programs::chosen_name_tail;
pub use choice_programs::parse_resource_chosen_name_target_shape;
#[path = "resource_shapes/library.rs"]
mod library_programs;
pub use library_programs::parse_resource_shuffle_shape;
