use super::*;

use crate::grammar::leaf;
use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::token::any;

#[path = "followup_shapes/regeneration.rs"]
mod regeneration;
pub use regeneration::*;
#[path = "followup_shapes/player_and_library.rs"]
mod player_and_library;
pub use player_and_library::*;
#[path = "followup_shapes/counter_linked_land.rs"]
mod counter_linked_land;
pub use counter_linked_land::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateMorePriorTokensShape<'a> {
    pub predicate_tokens: &'a [OwnedLexToken],
    pub count: u32,
    pub instead: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalFollowupKind {
    WhenMilledThisWay,
    IfNoOneDoes,
    IfYouWin,
    IfYouWinClash,
    IfYouWinFlip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalFollowupShape<'a> {
    pub kind: ConditionalFollowupKind,
    pub continuation_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TokenReminderFollowupFacts {
    pub lifecycle_head: bool,
    pub delayed_pronoun_lifecycle: bool,
    pub pronoun_trigger_prefix: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovedObjectEntryFollowupShape {
    /// Index in the original token slice of the authored `gains` verb.  The
    /// caller can prepend the leading object pronoun and feed the resulting
    /// clause through the ordinary typed ability-grant parser.
    pub grant_verb_token_idx: usize,
}

/// Recognize a result sentence that modifies the exact object just moved to
/// the battlefield: `It enters tapped and attacking and gains ... until end
/// of turn.`  This is deliberately only the grammatical boundary; the
/// follow-up dispatcher still has to prove an immediately preceding optional
/// single-object battlefield move before it may bind the pronoun.
pub fn parse_moved_object_entry_followup_shape(
    tokens: &[OwnedLexToken],
) -> Option<MovedObjectEntryFollowupShape> {
    let words_with_indices = crate::lexer::parser_token_word_positions(tokens);
    let words = words_with_indices
        .iter()
        .map(|(_, word)| *word)
        .collect::<Vec<_>>();
    if !crate::word_primitives::parse_sequence_prefix(
        &words,
        &["it", "enters", "tapped", "and", "attacking", "and", "gains"],
    ) || words.len() <= 10
        || !crate::word_primitives::parse_sequence_suffix(&words, &["until", "end", "of", "turn"])
    {
        return None;
    }
    Some(MovedObjectEntryFollowupShape {
        grant_verb_token_idx: words_with_indices[6].0,
    })
}

fn marker_anywhere<'a, O, P>(tokens: &'a [OwnedLexToken], parser: P) -> bool
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut input = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), parser)
        .parse_next(&mut input)
        .is_ok()
}

#[path = "followup_shapes/turn_skip.rs"]
mod turn_skip;
pub use turn_skip::*;

pub fn is_temporary_land_animation_sentence(tokens: &[OwnedLexToken]) -> bool {
    marker_anywhere(
        tokens,
        alt((primitives::kw("become"), primitives::kw("becomes"))),
    ) && marker_anywhere(
        tokens,
        alt((primitives::kw("creature"), primitives::kw("creatures"))),
    ) && marker_anywhere(tokens, primitives::phrase(&["until", "end", "of", "turn"]))
}

fn create_or_put<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("create"), primitives::kw("put")))
        .void()
        .parse_next(input)
}

fn create_more_tail<'a>(input: &mut LexStream<'a>) -> WResult<bool> {
    alt((
        (primitives::kw("instead"), primitives::sentence_end()).value(true),
        (
            primitives::phrase(&["onto", "the", "battlefield"]),
            opt(primitives::kw("instead")),
            primitives::sentence_end(),
        )
            .map(|(_, instead, _)| instead.is_some()),
        primitives::sentence_end().value(false),
    ))
    .parse_next(input)
}

fn parse_create_more_prior_tokens_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CreateMorePriorTokensShape<'a>> {
    let predicate_tokens = repeat_till(1.., any.void(), peek(create_or_put))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    create_or_put.parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::phrase(&["of", "those", "tokens"]).parse_next(input)?;
    let instead = create_more_tail.parse_next(input)?;
    Ok(CreateMorePriorTokensShape {
        predicate_tokens,
        count,
        instead,
    })
}

/// Does this sentence restate an action as a replacement of the preceding one?
///
/// An ability-word ladder writes the replacement in full ("Morbid — Create
/// three 2/2 green Wolf creature tokens instead if a creature died this turn")
/// instead of referring back to the objects it replaces.
pub fn is_instead_replacement_sentence(tokens: &[OwnedLexToken]) -> bool {
    let tokens = super::split_labeled_effect_prefix_lexed(tokens).unwrap_or(tokens);
    let words = crate::lexer::parser_token_word_refs(tokens);
    // An ordinary replacement effect states its own event first ("If a card
    // would be put into a graveyard, exile it instead") and is a complete
    // statement. Only a restated action — one that opens with the action
    // itself — replaces what the preceding sentence did.
    if words
        .first()
        .is_some_and(|word| matches!(*word, "if" | "when" | "whenever" | "at" | "as"))
    {
        return false;
    }
    let Some(instead) = words.iter().rposition(|word| *word == "instead") else {
        return false;
    };
    if instead == 0 {
        return false;
    }
    // A trigger header anywhere in the sentence means this line states its own
    // ability (an ability word may precede it, as in "Opus — Whenever ...");
    // a restatement carries only the action it replaces.
    if words[..instead]
        .iter()
        .any(|word| matches!(*word, "if" | "when" | "whenever"))
    {
        return false;
    }
    matches!(words.get(instead + 1..), Some([]) | None)
        || words.get(instead + 1).is_some_and(|word| *word == "if")
}

pub fn parse_create_more_prior_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CreateMorePriorTokensShape<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_create_more_prior_tokens_lexed,
        "create more prior tokens",
    )
}

#[cfg(test)]
#[path = "followup_shapes/tests.rs"]
mod tests;

#[path = "followup_shapes/object_action.rs"]
mod object_action_programs;
pub use object_action_programs::token_reminder_followup_facts;
#[path = "followup_shapes/trigger.rs"]
mod trigger_programs;
use trigger_programs::pronoun_trigger_prefix;
#[path = "followup_shapes/resource.rs"]
mod resource_programs;
use resource_programs::lifecycle_head;
#[path = "followup_shapes/combat.rs"]
mod combat_programs;
pub use combat_programs::is_anaphoric_damage_self_replacement;
#[path = "followup_shapes/condition.rs"]
mod condition_programs;
pub use condition_programs::parse_conditional_followup;
use condition_programs::{conditional_followup_prefix, parse_conditional_followup_lexed};
