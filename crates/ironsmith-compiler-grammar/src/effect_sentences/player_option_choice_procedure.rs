//! Per-player named choices and the instructions that read them.
//!
//! "For each player, choose friend or foe. Each friend <does A>. Each foe
//! <does B>." (Battlebond's friend-or-foe cycle), "Each opponent chooses fame
//! or fortune. For each player who chose fame, <A>. For each player who chose
//! fortune, <B>." (Seize the Spotlight), "each opponent chooses money,
//! friends, or secrets. For each player who chose money, …" (Master of
//! Ceremonies).
//!
//! The opening statement makes one named choice per participating player —
//! by the controller for every participant ("For each player, choose …"), or
//! by each participant ("Each opponent chooses …"). These are not votes
//! (CR 701.38). The following statements quantify over the players who ended
//! up with one option: "each <option>" and "each player who chose <option>".
//! Each such statement is the ordinary "each player …" statement restricted
//! to that option's player set, so the participant-scoped readings (actor,
//! "their", "that player") are the established ones.
//!
//! The procedure only opens when a following statement reads one of the
//! options, so an unrelated "choose X or Y" sentence never becomes a
//! per-player choice.

use winnow::Parser as _;
use winnow::combinator::{alt, opt};

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{CardTextError, EffectAst, ForEachEffectAst, TextSpan};
use ironsmith_core::{ChoosePlayerOptionEffect, PlayerOptionChooser, player_option_choice_tag};
use crate::grammar::primitives;
use crate::lexer::OwnedLexToken;
use crate::target::PlayerFilter;

pub(super) struct PlayerOptionChoiceGroup {
    options: Vec<String>,
    effects: Vec<EffectAst>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn trim(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    crate::util::trim_edge_punctuation_tokens(tokens)
}

/// Words that start an object or quantity phrase rather than a named option.
const NON_OPTION_WORDS: &[&str] = &[
    "a", "an", "the", "one", "two", "three", "target", "each", "any", "up", "to", "x",
    "another", "other", "that", "this", "those", "them", "it", "number", "color", "card",
    "cards", "creature", "creatures", "player", "players", "opponent", "permanent",
];

/// "<word>[, <word>]* [,] or <word>": two or more single-word named options.
fn named_options(tokens: &[OwnedLexToken]) -> Option<Vec<String>> {
    let tokens = trim(tokens);
    let mut options = Vec::new();
    let mut expect_option = true;
    let mut saw_or = false;
    for token in tokens {
        if expect_option {
            let word = token.as_word()?;
            if NON_OPTION_WORDS.contains(&word)
                || !word.chars().all(|ch| ch.is_ascii_alphabetic())
            {
                return None;
            }
            options.push(word.to_string());
            expect_option = false;
        } else if token.is_comma() {
            expect_option = true;
        } else if token.is_word("or") {
            if saw_or {
                return None;
            }
            saw_or = true;
            expect_option = true;
        } else {
            return None;
        }
    }
    // A trailing ", or" is absorbed by the comma/or alternation above; the
    // list must end on an option and contain the final "or".
    (!expect_option && saw_or && options.len() >= 2).then_some(options)
}

fn participant_noun<'a>(
    input: &mut crate::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<PlayerFilter> {
    alt((
        primitives::kw("player").value(PlayerFilter::Any),
        primitives::kw("opponent").value(PlayerFilter::Opponent),
    ))
    .parse_next(input)
}

/// The opening statement: who participates, who chooses, and the options.
fn opening(sentence: &SentenceInput) -> Option<ChoosePlayerOptionEffect> {
    let tokens = trim(sentence.lowered());
    // "For each player, choose <options>": the controller chooses for each.
    if let Some(((_, participants, _, _), rest)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["for", "each"]),
            participant_noun,
            opt(primitives::comma()),
            primitives::kw("choose"),
        ),
    ) {
        let options = named_options(rest)?;
        return Some(ChoosePlayerOptionEffect::new(
            participants,
            options,
            PlayerOptionChooser::Controller,
        ));
    }
    // "Each opponent chooses <options>": each participant chooses.
    let ((_, participants, _), rest) = primitives::parse_prefix(
        tokens,
        (primitives::kw("each"), participant_noun, primitives::kw("chooses")),
    )?;
    let options = named_options(rest)?;
    Some(ChoosePlayerOptionEffect::new(
        participants,
        options,
        PlayerOptionChooser::Participant,
    ))
}

fn synthetic(word: &str) -> OwnedLexToken {
    OwnedLexToken::word(word, TextSpan::synthetic())
}

fn option_index(options: &[String], token: &OwnedLexToken) -> Option<usize> {
    let word = token.as_word()?;
    options
        .iter()
        .position(|option| option.eq_ignore_ascii_case(word))
}

/// The statement with its option quantifier rewritten to the ordinary
/// participant quantifier, and the option it read: "each foe …" reads as
/// "each player …", "for each player who chose fame, …" as "for each
/// player, …", "… to each foe …" as "… to each player …".
fn rewrite_option_quantifier(
    tokens: &[OwnedLexToken],
    options: &[String],
) -> Option<(Vec<OwnedLexToken>, usize)> {
    // "for each player|opponent who chose <option>"
    for start in 0..tokens.len() {
        let Some(((_, _), rest)) = primitives::parse_prefix(
            &tokens[start..],
            (
                primitives::phrase(&["for", "each"]),
                alt((primitives::kw("player"), primitives::kw("opponent"))),
            ),
        ) else {
            continue;
        };
        let Some((_, rest)) = primitives::parse_prefix(rest, primitives::phrase(&["who", "chose"]))
        else {
            continue;
        };
        let (option_token, after) = rest.split_first()?;
        let index = option_index(options, option_token)?;
        let mut rewritten = tokens[..start].to_vec();
        rewritten.extend([synthetic("for"), synthetic("each"), synthetic("player")]);
        rewritten.extend_from_slice(after);
        return Some((rewritten, index));
    }
    // "each <option>" (singular option nouns such as friend/foe)
    for start in 0..tokens.len().saturating_sub(1) {
        if !tokens[start].is_word("each") {
            continue;
        }
        let Some(index) = option_index(options, &tokens[start + 1]) else {
            continue;
        };
        let mut rewritten = tokens[..start].to_vec();
        rewritten.extend([synthetic("each"), synthetic("player")]);
        rewritten.extend_from_slice(&tokens[start + 2..]);
        return Some((rewritten, index));
    }
    None
}

/// Restrict the single participant iteration the rewritten statement
/// produced to the option's player set. Exactly one iteration over every
/// player must exist; otherwise the statement is not the quantified reading.
fn restrict_to_option(effects: &mut [EffectAst], option: &str) -> usize {
    let mut restricted = 0;
    for effect in effects.iter_mut() {
        match effect {
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects: inner }) => {
                let inner = std::mem::take(inner);
                *effect = EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                    sequential: false,
                    filter: PlayerFilter::TaggedPlayer(player_option_choice_tag(option)),
                    effects: inner,
                });
                restricted += 1;
            }
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { filter, .. })
                if *filter == PlayerFilter::Any =>
            {
                *filter = PlayerFilter::TaggedPlayer(player_option_choice_tag(option));
                restricted += 1;
            }
            EffectAst::Sequence { effects: inner }
            | EffectAst::CommaThen { effects: inner }
            | EffectAst::SourceSentence { effects: inner, .. } => {
                restricted += restrict_to_option(inner, option);
            }
            _ => {}
        }
    }
    restricted
}

/// One statement reading an option's players, parsed as the restricted
/// participant statement.
fn option_statement(
    sentence: &SentenceInput,
    options: &[String],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = sentence.lowered();
    let Some((rewritten, index)) = rewrite_option_quantifier(tokens, options) else {
        return Ok(None);
    };
    let mut effects = super::parse_effect_sentence_lexed(&rewritten)?;
    if restrict_to_option(&mut effects, &options[index]) != 1 {
        return Err(CardTextError::ParseError(format!(
            "per-player option statement did not read as one participant iteration (clause: '{}')",
            crate::lexer::render_token_slice(tokens)
        )));
    }
    Ok(Some(effects))
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<PlayerOptionChoiceGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(choice) = opening(sentence) else {
        return Ok(None);
    };
    // Only a choice that a following statement reads forms the procedure.
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    if rewrite_option_quantifier(next.lowered(), &choice.options).is_none() {
        return Ok(None);
    }
    Ok(Some(PlayerOptionChoiceGroup {
        options: choice.options.clone(),
        effects: vec![EffectAst::ChoosePlayerOption(choice)],
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

pub(super) fn continue_with(
    group: &mut PlayerOptionChoiceGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    let Some(effects) = option_statement(sentence, &group.options)? else {
        return Ok(false);
    };
    group.effects.extend(effects);
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: PlayerOptionChoiceGroup) -> Vec<EffectAst> {
    group.effects
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sentences(text: &str) -> Vec<SentenceInput> {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        crate::lexer::split_lexed_sentences(&tokens)
            .into_iter()
            .map(SentenceInput::from_lexed)
            .collect()
    }

    #[test]
    fn friend_or_foe_opens_with_controller_choice() {
        let inputs = sentences(
            "For each player, choose friend or foe. Each friend sacrifices a creature of their choice.",
        );
        let group = open(&inputs, 0).unwrap().expect("procedure opens");
        assert_eq!(group.options, vec!["friend".to_string(), "foe".to_string()]);
        assert!(matches!(
            group.effects.as_slice(),
            [EffectAst::ChoosePlayerOption(choice)]
                if choice.chooser == PlayerOptionChooser::Controller
                    && choice.participants == PlayerFilter::Any
        ));
    }

    #[test]
    fn an_unread_choice_does_not_open() {
        let inputs = sentences("Each opponent chooses fame or fortune. Draw a card.");
        assert!(open(&inputs, 0).unwrap().is_none());
    }

    #[test]
    fn object_choices_are_not_named_options() {
        let tokens = crate::lexer::lex_line("a creature or planeswalker", 0).unwrap();
        assert!(named_options(&tokens).is_none());
        let tokens = crate::lexer::lex_line("money, friends, or secrets", 0).unwrap();
        assert_eq!(named_options(&tokens).unwrap().len(), 3);
    }
}
