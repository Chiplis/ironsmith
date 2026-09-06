//! Library-viewing procedures composed statement by statement.
//!
//! "Look at the top four cards of your library. You may reveal a creature card
//! from among them and put it into your hand. Put the rest on the bottom of
//! your library in a random order." is three statements. The first binds the
//! group of cards looked at; the second selects from that group and binds the
//! selection; the third disposes of what was not selected. Each statement is
//! recognized on its own sentence. What links them is the group an earlier
//! statement bound, which the sentence loop carries between consecutive
//! sentences; a sentence that is none of these statements ends the procedure.
//!
//! The sequence registry used to recognize each such wording as one named
//! program. These statements produce the same effects those programs did, one
//! sentence at a time, so a program the registry no longer names still compiles
//! to the same text. Where two programs spelled the same selection differently
//! depending on what followed it, the selection is read when its sentence
//! arrives and spelled when the remainder does ([`selections`]).

#[path = "looked_procedure/partitions.rs"]
mod partitions;
#[path = "looked_procedure/revealed.rs"]
mod revealed;
#[path = "looked_procedure/conditionals.rs"]
mod conditionals;
#[path = "looked_procedure/singles.rs"]
mod singles;
#[path = "looked_procedure/selections.rs"]
mod selections;

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, PlayerAst, TargetAst, Value, ConditionalEffectAst, PermissionEffectAst,
};
use crate::grammar::effects::triple_sequence_shapes as triple_grammar;
use crate::grammar::sentence_markers;
use crate::lexer::OwnedLexToken;
use crate::tag::TagKey;
use crate::util::helper_tag_for_tokens;

/// How the view statement is spelled as effects. A reveal is one effect when
/// the selection reads the revealed group, and a look followed by revealing
/// the looked group when a hand selection or a same-sentence disposition
/// follows, as the registry programs these statements replace spelled them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewStyle {
    Look,
    RevealTop,
    LookThenRevealTagged,
    /// A later statement's recognizer produced the view itself.
    Absorbed,
}

/// The group of cards a view statement bound, and what later statements have
/// done with it so far.
pub(super) struct ViewedGroup {
    /// The looked-at (or revealed) cards.
    tag: TagKey,
    /// Whose library they came from, and how many.
    owner: PlayerAst,
    count: Value,
    /// Whether the view statement revealed rather than looked.
    revealed: bool,
    view_style: ViewStyle,
    /// The view sentence's tokens: the group's tag is minted from them, and
    /// one follow-up recognizer reads the view and its follow-up as a clause.
    view_tokens: Vec<OwnedLexToken>,
    /// The most recent selection out of the group, if any.
    selected: Option<TagKey>,
    /// A selection whose spelling waits for the remainder statement.
    pending: Option<selections::PendingSelection>,
    /// Who disposes of the remainder: the selecting player for a reveal
    /// selection, the library's owner otherwise.
    remainder_player: PlayerAst,
    /// The view opened under "if you do, ..." and the whole procedure is
    /// gated on that result.
    gated: bool,
    /// The view was optional ("You may look at the top four cards"): the
    /// statements over the group happen only if it was done.
    optional: bool,
    /// A card was exiled from the group and a statement granting its cast or
    /// play (while exiled, this turn, or now) is awaited.
    awaiting_permission: Option<TagKey>,
    /// Statements already read together with the previous sentence; the
    /// next sentence is consumed as them.
    pending_statements: std::collections::VecDeque<Vec<EffectAst>>,
    /// The effects after the view, in statement order.
    effects: Vec<EffectAst>,
    /// The sentences consumed so far, as a range into the document's
    /// sentences, for a "where X is" binding that spans the procedure.
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn remainder_owner(owner: PlayerAst) -> PlayerAst {
    match owner {
        PlayerAst::Target | PlayerAst::TargetOpponent => PlayerAst::That,
        player => player,
    }
}

fn it() -> TargetAst {
    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None)
}

/// Whether the sentence after a view continues the procedure. A view claims
/// its sentence only when something follows that selects from or disposes of
/// the group; a lone "Look at the top card of your library." stays with the
/// sentence grammar.
fn continues(
    view: &SentenceInput,
    next: &SentenceInput,
    rest: &[SentenceInput],
    revealed: bool,
    owner: PlayerAst,
    count: &Value,
) -> bool {
    let following = rest.first();
    let mut combined = view.lowered().to_vec();
    combined.extend_from_slice(next.lowered());
    partitions::face_down_exile_clause(&combined).is_some()
        // A selection whose filter is unreadable still opens the procedure: the
        // statement is then a committed error rather than a guess.
        || !matches!(selections::selection_shape(next, owner), Ok(None))
        || triple_grammar::parse_looked_remainder_shape(next.lexed()).is_some()
        || selections::same_sentence_shape(next).is_some()
        || partitions::partition_shape(next, revealed).is_some()
        || partitions::counted_hand_remainder_shape(next, revealed, owner).is_some()
        || partitions::singleton_hand_disposition(next, revealed).is_some()
        || partitions::three_way_disposition(next, revealed, count).is_some()
        || partitions::reveal_top_followup(next, revealed, owner).is_some()
        || partitions::exile_selection_shape(next).is_some()
        || partitions::exact_one_to_graveyard_shape(next, revealed, owner).is_some()
        || partitions::optional_reveal_top_shape(next).is_some()
        || matches!(
            selections::cast_from_among_shape(next, owner),
            Ok(Some(_))
        )
        || (revealed
            && crate::grammar::effects::looked_card_shapes::parse_revealed_card_choice_shape(
                next.lowered(),
            )
            .is_some())
        || (revealed
            && crate::grammar::effects::looked_card_shapes::parse_opponent_revealed_card_selection_shape(
                crate::lexer::trim_lexed_commas(next.lowered()),
            )
            .is_some())
        || revealed::opponent_exile_then_hand_shape(owner, revealed, next, following)
        || singles::same_name_battlefield_shape(next, owner, revealed)
        || singles::optional_top_shape(next, following, owner, revealed)
        || singles::reveal_put_top_shape(next)
        || singles::reveal_to_hand_then_shuffle_shape(next, following, revealed)
        || singles::hand_bottom_exile_split_shape(next, following)
        || matches!(
            singles::battlefield_or_hand_split_shape(next, rest),
            Ok(Some(_))
        )
        || matches!(
            singles::exile_one_cast_else_hand_shape(next, rest, revealed),
            Ok(Some(_))
        )
        || singles::kicked_hand_count_shape(next, rest).is_some()
        || singles::reveal_then_your_turn_shape(next, rest, revealed)
        || singles::reveal_then_bargain_shape(next, rest, revealed)
        || singles::nonhand_replacement_shape(next, rest, revealed)
        || conditionals::conditional_hand_counts_shape(next, rest, revealed).is_some()
        || singles::any_number_revealed_land_split_shape(next, rest, revealed)
        || conditionals::reveal_selection_land_creature_split_shape(next, rest, revealed)
}

/// Open a procedure at a view sentence ("Look at the top N cards of your
/// library", "Reveal the top N cards of your library", either under "if you
/// do,"), when the next sentence continues it.
pub(super) fn open(sentences: &[SentenceInput], sentence_idx: usize) -> Option<ViewedGroup> {
    if let Some(group) = partitions::open_exiled_face_down(sentences, sentence_idx) {
        return Some(group);
    }
    let sentence = sentences.get(sentence_idx)?;
    let next = sentences.get(sentence_idx + 1)?;
    let first_tokens = crate::lexer::trim_lexed_commas(sentence.lowered());
    let first_tokens = crate::util::strip_leading_token_words_any(first_tokens, &["then"]);
    let (first_tokens, optional) = match first_tokens {
        [you, may, rest @ ..] if you.is_word("you") && may.is_word("may") => (rest, true),
        tokens => (tokens, false),
    };
    let (view_tokens, gated) =
        if let Some(followup) = sentence_markers::parse_conditional_followup_tokens(first_tokens) {
            if followup.actor != sentence_markers::ConditionalFollowupActor::You {
                return None;
            }
            (crate::lexer::trim_lexed_commas(followup.tail_tokens), true)
        } else {
            (first_tokens, false)
        };
    let (owner, count, revealed) =
        super::looked_cards_family::parse_top_cards_view_sentence(view_tokens)?;
    if optional && (revealed || gated) {
        return None;
    }
    // A view that already carries its own counted face-down exile is one
    // clause, not a view awaiting statements.
    if partitions::face_down_exile_clause(sentence.lowered()).is_some() {
        return None;
    }
    let rest = sentences.get(sentence_idx + 2..).unwrap_or(&[]);
    if !continues(sentence, next, rest, revealed, owner, &count) {
        return None;
    }
    let tag = helper_tag_for_tokens(
        sentence.lowered(),
        if revealed { "revealed" } else { "looked" },
    );
    Some(ViewedGroup {
        tag: tag.key.clone(),
        owner,
        count,
        revealed,
        view_style: if revealed {
            ViewStyle::RevealTop
        } else {
            ViewStyle::Look
        },
        view_tokens: sentence.lowered().to_vec(),
        selected: None,
        pending: None,
        remainder_player: remainder_owner(owner),
        gated,
        optional,
        awaiting_permission: None,
        pending_statements: std::collections::VecDeque::new(),
        effects: Vec::new(),
        first_sentence: sentence_idx,
        consumed: 1,
    })
}

/// Continue an open procedure with the next sentence. Returns false, leaving
/// the group untouched, when the sentence is not one of its statements.
pub(super) fn continue_with(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> Result<bool, CardTextError> {
    let following = rest.first();
    if let Some(statements) = group.pending_statements.pop_front() {
        group.effects.extend(statements);
        group.consumed += 1;
        return Ok(true);
    }
    if let Some(exiled) = group.awaiting_permission.clone()
        && let Some(permission) = partitions::exiled_permission(sentence, exiled)
    {
        group.awaiting_permission = None;
        group.effects.push(permission);
        group.consumed += 1;
        return Ok(true);
    }
    // Statements read together with the sentences after them come before the
    // single-sentence partitions, which would otherwise claim their first
    // sentence alone.
    if group.selected.is_none() && group.effects.is_empty() {
        if singles::exile_one_cast_else_hand(group, sentence, rest)?
            || singles::kicked_hand_count(group, sentence, rest)
            || singles::reveal_then_your_turn(group, sentence, rest)?
            || singles::reveal_then_bargain(group, sentence, rest)?
            || singles::nonhand_replacement(group, sentence, rest)
            || conditionals::conditional_hand_counts(group, sentence, rest)
            || conditionals::reveal_selection_land_creature_split(group, sentence, rest)
            || singles::any_number_revealed_land_split(group, sentence, rest)
        {
            group.consumed += 1;
            return Ok(true);
        }
    }
    if group.selected.is_none() && group.effects.is_empty() {
        if partitions::first_statement(group, sentence) {
            group.consumed += 1;
            return Ok(true);
        }
    }
    if group.selected.is_none() && partitions::exile_selection(group, sentence)? {
        group.consumed += 1;
        return Ok(true);
    }
    if group.selected.is_none() && group.effects.is_empty() {
        if revealed::revealed_choice(group, sentence)
            || revealed::opponent_selection(group, sentence)?
            || revealed::opponent_exile_then_hand(group, sentence, following)
            || singles::same_name_battlefield(group, sentence)
            || singles::optional_top(group, sentence, following)
            || singles::reveal_put_top(group, sentence)
            || singles::reveal_to_hand_then_shuffle(group, sentence, following)?
            || singles::hand_bottom_exile_split(group, sentence, following)
            || singles::battlefield_or_hand_split(group, sentence, rest)?
        {
            group.consumed += 1;
            return Ok(true);
        }
    }
    if group.selected.is_some()
        && (conditionals::conditional_remainder(group, sentence, rest)
            || singles::entry_counter_condition(group, sentence))
    {
        group.consumed += 1;
        return Ok(true);
    }
    if group.selected.is_some()
        && (revealed::chosen_partition(group, sentence)
            || revealed::chosen_move_followup(group, sentence)?)
    {
        group.consumed += 1;
        return Ok(true);
    }
    if group.selected.is_none() && selections::cast_from_among(group, sentence)? {
        group.consumed += 1;
        return Ok(true);
    }
    if group.selected.is_some() && partitions::shuffle_statement(group, sentence) {
        group.consumed += 1;
        return Ok(true);
    }
    if group.selected.is_none()
        && let Some((action, remainder)) = selections::same_sentence_shape(sentence)
    {
        if selections::select_with_remainder(group, sentence, action, remainder) {
            group.consumed += 1;
            return Ok(true);
        }
        return Ok(false);
    }
    if group.pending.is_none()
        && let Some(selection) = selections::selection_shape(sentence, group.owner)?
    {
        if !selections::select(group, sentence, selection) {
            return Ok(false);
        }
        group.consumed += 1;
        return Ok(true);
    }
    if group.selected.is_some()
        && let Some(remainder) = triple_grammar::parse_looked_remainder_shape(sentence.lexed())
    {
        selections::spell_remainder(group, sentence, remainder);
        group.consumed += 1;
        return Ok(true);
    }
    Ok(false)
}

/// Close the procedure: the view spelled the way its statements need, then
/// their effects, gated when the view was.
pub(super) fn finish(mut group: ViewedGroup) -> Vec<EffectAst> {
    selections::spell_pending(&mut group, None);
    let mut effects = match group.view_style {
        ViewStyle::Look => vec![EffectAst::subject_verb_look_at_top_cards(
            group.owner,
            group.count,
            crate::tag::TagRef::of(group.tag.clone()),
        )],
        ViewStyle::RevealTop => vec![EffectAst::subject_verb_reveal_top_cards(
            group.owner,
            group.count,
            crate::tag::TagRef::of(group.tag.clone()),
        )],
        ViewStyle::LookThenRevealTagged => vec![
            EffectAst::subject_verb_look_at_top_cards(group.owner, group.count, crate::tag::TagRef::of(group.tag.clone())),
            EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(group.tag.clone())),
        ],
        ViewStyle::Absorbed => Vec::new(),
    };
    if group.optional {
        return vec![
            EffectAst::Permissions(PermissionEffectAst::May { effects }),
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: group.effects,
            }),
        ];
    }
    effects.extend(group.effects);
    if group.gated {
        vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        })]
    } else {
        effects
    }
}
