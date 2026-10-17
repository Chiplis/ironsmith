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
use crate::cards::builders::{
    CardTextError, ConditionalEffectAst, EffectAst, ForEachEffectAst, PredicateAst, TextSpan,
};
use ironsmith_core::{
    ChoosePlayerOptionEffect, PlayerNumberChoice, PlayerOptionChooser, player_number_extreme_tag,
    player_option_choice_tag, player_option_participants_tag,
};
use crate::grammar::primitives;
use crate::lexer::OwnedLexToken;
use crate::target::PlayerFilter;

pub(super) struct PlayerOptionChoiceGroup {
    options: Vec<String>,
    /// Each participant chose a number rather than a named option.
    number: bool,
    /// The number range, read again by a guess at the number.
    range: Option<PlayerNumberChoice>,
    effects: Vec<EffectAst>,
    /// The player set the last statement read ("If you are one of those
    /// players, ...").
    last_group: Option<PlayerFilter>,
    /// The extreme ("the highest number") the procedure last named, read by
    /// "that number" / "that much".
    last_extreme: Option<bool>,
    /// Consecutive "If each opponent chose <option>, ..." conditions an
    /// "Otherwise, ..." statement negates.
    pending_conditions: Vec<PredicateAst>,
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
            // The final conjunction may follow the serial comma. It is a
            // separator, not a named option in "money, friends, or secrets".
            if token.is_word("or") {
                if options.is_empty() || saw_or {
                    return None;
                }
                saw_or = true;
                continue;
            }
            let word = token.as_word()?;
            if NON_OPTION_WORDS.contains(&word)
                || !word.chars().all(|ch| ch.is_ascii_alphabetic())
            {
                return None;
            }
            options.push(word.to_string());
            expect_option = false;
        } else if saw_or {
            return None;
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
    // The list must end on an option and contain the final "or".
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
    // "[You] secretly choose a number between 1 and 5 [that hasn't been
    // chosen]" (The Toymaker's Trap): you make one secret number choice.
    let own = match tokens.split_first() {
        Some((first, rest)) if first.is_word("you") => rest,
        _ => tokens,
    };
    if own.first().is_some_and(|token| token.is_word("secretly"))
        && own.get(1).is_some_and(|token| token.is_word("choose"))
    {
        let rest = &own[2..];
        let (rest, exclude) = without_unchosen_suffix(rest);
        let range = number_range(rest)?;
        let mut choice = ChoosePlayerOptionEffect::numbers(PlayerFilter::You, range).with_secret(true);
        choice.exclude_source_history = exclude;
        return Some(choice);
    }
    // "Each opponent [secretly] chooses <options>[, then the choices are
    // revealed]": each participant chooses.
    let ((_, participants, secretly, _), rest) = primitives::parse_prefix(
        tokens,
        (
            primitives::kw("each"),
            participant_noun,
            opt(primitives::kw("secretly")),
            primitives::kw("chooses"),
        ),
    )?;
    let secret = secretly.is_some();
    let rest = without_reveal_tail(rest);
    if let Some(range) = number_range(rest) {
        return Some(ChoosePlayerOptionEffect::numbers(participants, range).with_secret(secret));
    }
    // "Each player chooses a color" (Selective Obliteration): the five
    // colors are the named options.
    if crate::lexer::token_word_refs(rest).as_slice() == ["a", "color"] {
        return Some(
            ChoosePlayerOptionEffect::new(
                participants,
                color_options(),
                PlayerOptionChooser::Participant,
            )
            .with_secret(secret),
        );
    }
    let options = named_options(rest)?;
    Some(
        ChoosePlayerOptionEffect::new(participants, options, PlayerOptionChooser::Participant)
            .with_secret(secret),
    )
}

fn color_options() -> Vec<String> {
    crate::color::Color::ALL
        .iter()
        .map(|color| color.name().to_string())
        .collect()
}

/// "[Then] exile|destroy each permanent unless it's colorless or it's only
/// the color its controller chose": every colored permanent with a color
/// other than the one its controller chose.
fn color_choice_sweep(tokens: &[OwnedLexToken], options: &[String]) -> Option<EffectAst> {
    if options != color_options().as_slice() {
        return None;
    }
    let words = crate::lexer::token_word_refs(trim(tokens));
    let words = words.strip_prefix(&["then"]).unwrap_or(&words[..]);
    let (verb, rest) = words.split_first()?;
    let rest = rest
        .strip_prefix(&["each", "permanent"])
        .or_else(|| rest.strip_prefix(&["all", "permanents"]))?;
    let tail = rest.join(" ").replace(['\'', '’'], "");
    if tail != "unless its colorless or its only the color its controller chose"
        && tail != "unless it s colorless or it s only the color its controller chose"
    {
        return None;
    }
    let all_colors = crate::color::Color::ALL
        .into_iter()
        .collect::<crate::color::ColorSet>();
    let mut filter = crate::target::ObjectFilter::permanent();
    filter.any_of = crate::color::Color::ALL
        .into_iter()
        .map(|color| {
            let mut chose = crate::target::ObjectFilter::default();
            chose.controller = Some(PlayerFilter::TaggedPlayer(player_option_choice_tag(
                color.name(),
            )));
            chose.colors = Some(all_colors.without(color));
            chose
        })
        .collect();
    match *verb {
        "exile" => Some(EffectAst::subject_verb_exile_all(filter, false)),
        "destroy" => Some(EffectAst::subject_verb_destroy_all(filter)),
        _ => None,
    }
}

/// Words a reveal statement ("Then those numbers are revealed.", "then all
/// players reveal those numbers simultaneously and determine the highest and
/// lowest numbers revealed this way") is built from.
const REVEAL_WORDS: &[&str] = &[
    "then", "those", "the", "all", "players", "player", "numbers", "number", "choices",
    "choice", "are", "is", "reveal", "reveals", "revealed", "simultaneously", "and",
    "determine", "highest", "lowest", "this", "way", "you", "chose",
];

fn is_reveal_statement(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::lexer::token_word_refs(trim(tokens));
    words
        .iter()
        .any(|word| matches!(*word, "reveal" | "reveals" | "revealed"))
        && words.iter().all(|word| REVEAL_WORDS.contains(word))
        && tokens.iter().all(|token| token.as_word().is_some() || token.is_comma() || token.is_period())
}

/// "<range> that hasn't been chosen": the range and whether earlier
/// choices of this source are excluded.
fn without_unchosen_suffix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    let tokens = trim(tokens);
    let words = crate::lexer::token_word_refs(tokens);
    let suffix_len = match words.as_slice() {
        [.., "that", "hasn't" | "hasnt", "been", "chosen"] => 4,
        [.., "that", "has", "not", "been", "chosen"] => 5,
        _ => return (tokens, false),
    };
    if tokens.len() < suffix_len
        || tokens[tokens.len() - suffix_len..]
            .iter()
            .any(|token| token.as_word().is_none())
    {
        return (tokens, false);
    }
    (&tokens[..tokens.len() - suffix_len], true)
}

/// "an opponent guesses which number you chose" / "each opponent guesses
/// ...": a guess at the number chosen earlier in the procedure.
fn guess_statement(
    tokens: &[OwnedLexToken],
    range: PlayerNumberChoice,
) -> Option<ChoosePlayerOptionEffect> {
    let tokens = without_reveal_tail(tokens);
    let words = crate::lexer::token_word_refs(tokens);
    let (participants, choose_one) = match words.as_slice() {
        ["an", "opponent", "guesses", "which", "number", "you", "chose"] => {
            (PlayerFilter::Opponent, true)
        }
        ["each", "opponent", "guesses", "which", "number", "you", "chose"] => {
            (PlayerFilter::Opponent, false)
        }
        _ => return None,
    };
    let mut guess = ChoosePlayerOptionEffect::numbers(participants, range);
    guess.guess = true;
    guess.choose_one_participant = choose_one;
    Some(guess)
}

/// "If they guessed wrong|right": some guesser guessed wrong (right).
fn guessed_condition(words: &[&str]) -> Option<(PredicateAst, PlayerFilter)> {
    let right = match words {
        ["if", "they", "guessed", "right"] | ["if", "that", "player", "guessed", "right"] => true,
        ["if", "they", "guessed", "wrong"] | ["if", "that", "player", "guessed", "wrong"] => false,
        _ => return None,
    };
    let guessers = PlayerFilter::TaggedPlayer(ironsmith_core::player_guess_tag(right));
    Some((players_exist(guessers.clone()), guessers))
}

/// At least one player matches `players`.
fn players_exist(players: PlayerFilter) -> PredicateAst {
    PredicateAst::Bound(Box::new(crate::ConditionExpr::ValueComparison {
        left: crate::effect::Value::CountPlayers(players),
        operator: crate::effect::ValueComparisonOperator::GreaterThan,
        right: crate::effect::Value::Fixed(0),
    }))
}

/// "they lose life equal to the number they guessed and you draw a card":
/// the guessers' instruction, read as "each player loses x life ..." over
/// the guessers, with X the guess.
fn guessers_consequence(
    tokens: &[OwnedLexToken],
    guessers: &PlayerFilter,
) -> Result<Vec<EffectAst>, CardTextError> {
    let tokens = trim(tokens);
    let mut rewritten = Vec::with_capacity(tokens.len() + 2);
    let mut guess_value = false;
    let mut idx = 0;
    while idx < tokens.len() {
        let words = crate::lexer::token_word_refs(&tokens[idx..]);
        if idx == 0 && tokens[0].is_word("they") && tokens.len() > 1 {
            rewritten.extend([synthetic("each"), synthetic("player")]);
            let verb = tokens[1].as_word().unwrap_or_default();
            let verb = if verb.ends_with('s') {
                verb.to_string()
            } else {
                format!("{verb}s")
            };
            rewritten.push(synthetic(&verb));
            idx = 2;
            continue;
        }
        if let ["equal", "to", "the", "number", "they", "guessed", ..] = words.as_slice()
            && tokens[idx..idx + 6].iter().all(|token| token.as_word().is_some())
            && let Some(noun) = rewritten.pop()
        {
            rewritten.extend([synthetic("x"), noun]);
            guess_value = true;
            idx += 6;
            continue;
        }
        rewritten.push(tokens[idx].clone());
        idx += 1;
    }
    let mut effects = super::parse_effect_sentence_lexed(&rewritten)?;
    if restrict_to_players(&mut effects, guessers) == 0 {
        effects = vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            sequential: false,
            filter: guessers.clone(),
            effects,
        })];
    }
    if guess_value {
        super::dispatch_entry::replace_unbound_x_in_effects_anywhere(
            &mut effects,
            &crate::effect::Value::PlayersChosenNumberExtreme {
                highest: true,
                guess: true,
            },
            &crate::lexer::render_token_slice(tokens),
        )?;
    }
    Ok(effects)
}

/// The choice clause without a trailing ", then <reveal statement>".
fn without_reveal_tail(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let tokens = trim(tokens);
    for (idx, token) in tokens.iter().enumerate() {
        if token.is_comma()
            && tokens.get(idx + 1).is_some_and(|next| next.is_word("then"))
            && is_reveal_statement(&tokens[idx + 1..])
        {
            return &tokens[..idx];
        }
    }
    tokens
}

/// "a number", "a number <n> or greater", "a number between <a> and <b>".
fn number_range(tokens: &[OwnedLexToken]) -> Option<PlayerNumberChoice> {
    let words = crate::lexer::token_word_refs(trim(tokens));
    let number = |word: &str| word.parse::<u32>().ok();
    match words.as_slice() {
        ["a", "number"] => Some(PlayerNumberChoice { min: 0, max: None }),
        ["a", "number", min, "or", "greater"] => Some(PlayerNumberChoice {
            min: number(min)?,
            max: None,
        }),
        ["a", "number", "between", min, "and", max] | ["a", "number", "from", min, "to", max] => {
            let (min, max) = (number(min)?, number(max)?);
            (min <= max).then_some(PlayerNumberChoice { min, max: Some(max) })
        }
        _ => None,
    }
}

/// Which players a number-choice statement quantifies over.
#[derive(Clone, Copy)]
enum NumberGroup {
    /// "each player with the highest number", "each player who chose that
    /// number".
    Extreme(bool),
    /// "each player who didn't choose the lowest number".
    NotExtreme(bool),
}

fn number_group_filter(group: NumberGroup) -> PlayerFilter {
    match group {
        NumberGroup::Extreme(highest) => {
            PlayerFilter::TaggedPlayer(player_number_extreme_tag(highest))
        }
        NumberGroup::NotExtreme(highest) => PlayerFilter::Excluding {
            base: Box::new(PlayerFilter::TaggedPlayer(player_option_participants_tag())),
            excluded: Box::new(PlayerFilter::TaggedPlayer(player_number_extreme_tag(highest))),
        },
    }
}

fn extreme_word(word: &str) -> Option<bool> {
    match word {
        "highest" => Some(true),
        "lowest" => Some(false),
        _ => None,
    }
}

/// The extreme most recently named in `tokens` ("... the highest number ...").
fn last_named_extreme(tokens: &[OwnedLexToken]) -> Option<bool> {
    let words = crate::lexer::token_word_refs(tokens);
    words
        .windows(2)
        .rev()
        .find_map(|pair| (pair[1] == "number").then(|| extreme_word(pair[0])).flatten())
}

/// "each player|opponent <number qualifier>" rewritten to "each player", and
/// the player set the qualifier names.
fn rewrite_number_quantifier(
    tokens: &[OwnedLexToken],
    default_extreme: Option<bool>,
) -> Option<(Vec<OwnedLexToken>, NumberGroup)> {
    for start in 0..tokens.len().saturating_sub(1) {
        if !tokens[start].is_word("each") || !tokens[start + 1].is_any_word(&["player", "opponent"])
        {
            continue;
        }
        let after = &tokens[start + 2..];
        let words = crate::lexer::token_word_refs(after);
        let extreme_at = |idx: usize| words.get(idx).and_then(|word| extreme_word(word));
        let that_number = || {
            last_named_extreme(&tokens[..start])
                .or(default_extreme)
                .unwrap_or(true)
        };
        let (group, consumed) = match words.as_slice() {
            ["with", "the", _, "number", ..] => (NumberGroup::Extreme(extreme_at(2)?), 4),
            ["who", "chose", "the", _, "number", ..] => (NumberGroup::Extreme(extreme_at(3)?), 5),
            ["who", "chose", "that", "number", ..] => (NumberGroup::Extreme(that_number()), 4),
            ["who", "didn't" | "didnt", "choose", "the", _, "number", ..] => {
                (NumberGroup::NotExtreme(extreme_at(4)?), 6)
            }
            ["who", "did", "not", "choose", "the", _, "number", ..] => {
                (NumberGroup::NotExtreme(extreme_at(5)?), 7)
            }
            _ => continue,
        };
        // Only word tokens were counted; the qualifier never spans
        // punctuation.
        if after[..consumed].iter().any(|token| token.as_word().is_none()) {
            continue;
        }
        let mut rewritten = tokens[..start].to_vec();
        rewritten.extend([synthetic("each"), synthetic("player")]);
        rewritten.extend_from_slice(&after[consumed..]);
        return Some((rewritten, group));
    }
    None
}

/// "that much <noun>" and "<noun> equal to the highest number" read the
/// chosen-number extreme; rewrite them to "x <noun>" so the ordinary
/// sentence parses, then bind X. Returns the extreme X stands for.
fn rewrite_number_value(
    tokens: &mut Vec<OwnedLexToken>,
    default_extreme: Option<bool>,
) -> Option<bool> {
    for idx in 0..tokens.len().saturating_sub(1) {
        if tokens[idx].is_word("that") && tokens[idx + 1].is_word("much") {
            let extreme = last_named_extreme(&tokens[..idx])
                .or(default_extreme)
                .unwrap_or(true);
            tokens.splice(idx..idx + 2, [synthetic("x")]).for_each(drop);
            return Some(extreme);
        }
    }
    for idx in 1..tokens.len() {
        let words = crate::lexer::token_word_refs(&tokens[idx..]);
        if let ["equal", "to", "the", extreme, "number", ..] = words.as_slice()
            && let Some(extreme) = extreme_word(extreme)
            && tokens[idx..idx + 5].iter().all(|token| token.as_word().is_some())
            && tokens[idx - 1].as_word().is_some()
        {
            let noun = tokens[idx - 1].clone();
            tokens.splice(idx - 1..idx + 5, [synthetic("x"), noun]).for_each(drop);
            return Some(extreme);
        }
    }
    None
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
    // "each player|opponent who chose <option>" in subject or object
    // position ("... to each opponent who chose silence").
    for start in 0..tokens.len().saturating_sub(4) {
        if !tokens[start].is_word("each")
            || !tokens[start + 1].is_any_word(&["player", "opponent"])
            || !tokens[start + 2].is_word("who")
            || !tokens[start + 3].is_word("chose")
        {
            continue;
        }
        let Some(index) = option_index(options, &tokens[start + 4]) else {
            continue;
        };
        let mut rewritten = tokens[..start].to_vec();
        rewritten.extend([synthetic("each"), synthetic("player")]);
        rewritten.extend_from_slice(&tokens[start + 5..]);
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
fn restrict_to_players(effects: &mut [EffectAst], players: &PlayerFilter) -> usize {
    let mut restricted = 0;
    for effect in effects.iter_mut() {
        match effect {
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects: inner }) => {
                let inner = std::mem::take(inner);
                *effect = EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                    sequential: false,
                    filter: players.clone(),
                    effects: inner,
                });
                restricted += 1;
            }
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { filter, .. })
                if *filter == PlayerFilter::Any =>
            {
                *filter = players.clone();
                restricted += 1;
            }
            EffectAst::Sequence { effects: inner }
            | EffectAst::CommaThen { effects: inner }
            | EffectAst::SourceSentence { effects: inner, .. } => {
                restricted += restrict_to_players(inner, players);
            }
            _ => {}
        }
    }
    restricted
}

/// The body of the last participant iteration a quantified statement made.
fn last_iteration_body(effects: &mut [EffectAst]) -> Option<&mut Vec<EffectAst>> {
    match effects.last_mut()? {
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects: body, .. }) => {
            Some(body)
        }
        EffectAst::Sequence { effects: inner }
        | EffectAst::CommaThen { effects: inner }
        | EffectAst::SourceSentence { effects: inner, .. } => last_iteration_body(inner),
        _ => None,
    }
}

/// The statement's quantified player set and parsed effects, for a statement
/// that reads the choice ("each player with the highest number loses that
/// much life", "... to each opponent who chose silence", "... to each of
/// them"). `None` when the statement reads no player set of the choice.
fn quantified_statement(
    tokens: &[OwnedLexToken],
    group: &PlayerOptionChoiceGroup,
) -> Result<Option<(Vec<EffectAst>, PlayerFilter, Option<bool>)>, CardTextError> {
    let tokens = trim(tokens);
    let (mut rewritten, players, extreme) = if group.number {
        let Some((rewritten, number_group)) =
            rewrite_number_quantifier(tokens, group.last_extreme)
        else {
            return Ok(None);
        };
        let extreme = match number_group {
            NumberGroup::Extreme(highest) | NumberGroup::NotExtreme(highest) => highest,
        };
        (rewritten, number_group_filter(number_group), Some(extreme))
    } else if let Some((rewritten, index)) = rewrite_option_quantifier(tokens, &group.options) {
        (
            rewritten,
            PlayerFilter::TaggedPlayer(player_option_choice_tag(&group.options[index])),
            None,
        )
    } else if let Some(rewritten) = rewrite_each_of_them(tokens) {
        (
            rewritten,
            PlayerFilter::TaggedPlayer(player_option_participants_tag()),
            None,
        )
    } else {
        return Ok(None);
    };
    let value_extreme = if group.number {
        rewrite_number_value(&mut rewritten, extreme.or(group.last_extreme))
    } else {
        None
    };
    let mut effects = super::parse_effect_sentence_lexed(&rewritten)?;
    if restrict_to_players(&mut effects, &players) == 0 {
        return Err(CardTextError::ParseError(format!(
            "per-player choice statement did not read as a participant iteration (clause: '{}')",
            crate::lexer::render_token_slice(tokens)
        )));
    }
    if let Some(highest) = value_extreme {
        super::dispatch_entry::replace_unbound_x_in_effects_anywhere(
            &mut effects,
            &crate::effect::Value::PlayersChosenNumberExtreme { highest, guess: false },
            &crate::lexer::render_token_slice(tokens),
        )?;
    }
    Ok(Some((effects, players, value_extreme.or(extreme))))
}

/// "each of them" (the players who made the choice) rewritten to "each
/// player".
fn rewrite_each_of_them(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    let start = tokens.windows(3).position(|window| {
        window[0].is_word("each") && window[1].is_word("of") && window[2].is_word("them")
    })?;
    let mut rewritten = tokens[..start].to_vec();
    rewritten.extend([synthetic("each"), synthetic("player")]);
    rewritten.extend_from_slice(&tokens[start + 3..]);
    Some(rewritten)
}

/// "If each opponent chose silence": every participant chose the option.
fn everyone_chose_condition(
    words: &[&str],
    options: &[String],
) -> Option<PredicateAst> {
    let ["each", "player" | "opponent", "chose", option] = words else {
        return None;
    };
    let option = options
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(option))?;
    Some(PredicateAst::Bound(Box::new(crate::ConditionExpr::ValueComparison {
        left: crate::effect::Value::CountPlayers(PlayerFilter::TaggedPlayer(
            player_option_choice_tag(option),
        )),
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: crate::effect::Value::CountPlayers(PlayerFilter::TaggedPlayer(
            player_option_participants_tag(),
        )),
    })))
}

/// The consequence of a conditional statement: a statement reading the
/// choice, or an ordinary sentence.
fn consequence(
    tokens: &[OwnedLexToken],
    group: &mut PlayerOptionChoiceGroup,
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some((effects, players, extreme)) = quantified_statement(tokens, group)? {
        group.last_group = Some(players);
        if extreme.is_some() {
            group.last_extreme = extreme;
        }
        return Ok(effects);
    }
    super::parse_effect_sentence_lexed(trim(tokens))
}

/// Split "<lead words>, <consequence>" at its first comma.
fn split_lead(tokens: &[OwnedLexToken]) -> Option<(Vec<&str>, &[OwnedLexToken])> {
    let tokens = trim(tokens);
    let comma = tokens.iter().position(OwnedLexToken::is_comma)?;
    let lead = &tokens[..comma];
    lead.iter().all(|token| token.as_word().is_some()).then(|| {
        (crate::lexer::token_word_refs(lead), &tokens[comma + 1..])
    })
}

/// Whether this sentence is one of the procedure's statements, without
/// parsing its consequence.
fn reads_choice(sentence: &SentenceInput, choice: &ChoosePlayerOptionEffect) -> bool {
    let tokens = trim(sentence.lowered());
    if is_reveal_statement(tokens) {
        return true;
    }
    if let Some(range) = choice.number {
        return rewrite_number_quantifier(tokens, None).is_some()
            || split_lead(tokens).is_some_and(|(lead, rest)| {
                lead.as_slice() == ["if", "you", "do"] && guess_statement(rest, range).is_some()
            });
    }
    if rewrite_option_quantifier(tokens, &choice.options).is_some()
        || color_choice_sweep(tokens, &choice.options).is_some()
    {
        return true;
    }
    split_lead(tokens).is_some_and(|(lead, _)| {
        lead.first() == Some(&"if") && everyone_chose_condition(&lead[1..], &choice.options).is_some()
    })
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
    if !reads_choice(next, &choice) {
        return Ok(None);
    }
    Ok(Some(PlayerOptionChoiceGroup {
        options: choice.options.clone(),
        number: choice.number.is_some(),
        range: choice.number,
        effects: vec![EffectAst::ChoosePlayerOption(choice)],
        last_group: None,
        last_extreme: None,
        pending_conditions: Vec::new(),
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

pub(super) fn continue_with(
    group: &mut PlayerOptionChoiceGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    let tokens = trim(sentence.lowered());
    // "Then those numbers are revealed.": the choices were collected before
    // any was recorded, so the reveal adds no instruction.
    if is_reveal_statement(tokens) {
        group.consumed += 1;
        return Ok(true);
    }
    if let Some((lead, rest)) = split_lead(tokens) {
        // "If you do, an opponent guesses which number you chose, then you
        // reveal the number you chose."
        if lead.as_slice() == ["if", "you", "do"]
            && let Some(range) = group.range
            && let Some(guess) = guess_statement(rest, range)
        {
            group.effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: players_exist(PlayerFilter::TaggedPlayer(player_number_extreme_tag(
                    true,
                ))),
                if_true: vec![EffectAst::ChoosePlayerOption(guess)],
                if_false: Vec::new(),
            }));
            group.pending_conditions.clear();
            group.consumed += 1;
            return Ok(true);
        }
        // "If they guessed wrong, they lose life equal to the number they
        // guessed and you draw a card."
        if let Some((predicate, guessers)) = guessed_condition(&lead) {
            let if_true = if rest.first().is_some_and(|token| token.is_word("they")) {
                guessers_consequence(rest, &guessers)?
            } else {
                super::parse_effect_sentence_lexed(trim(rest))?
            };
            group.effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate,
                if_true,
                if_false: Vec::new(),
            }));
            group.last_group = Some(guessers);
            group.consumed += 1;
            return Ok(true);
        }
        // "If each opponent chose silence, ..."
        if lead.first() == Some(&"if")
            && let Some(predicate) = everyone_chose_condition(&lead[1..], &group.options)
        {
            let if_true = consequence(rest, group)?;
            group.pending_conditions.push(predicate.clone());
            group.effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate,
                if_true,
                if_false: Vec::new(),
            }));
            group.consumed += 1;
            return Ok(true);
        }
        // "Otherwise, ...": none of the preceding conditions held.
        if lead.as_slice() == ["otherwise"] && !group.pending_conditions.is_empty() {
            let conditions = std::mem::take(&mut group.pending_conditions);
            let any = conditions
                .into_iter()
                .reduce(|left, right| PredicateAst::Or(Box::new(left), Box::new(right)))
                .expect("pending conditions are non-empty");
            let if_true = consequence(rest, group)?;
            group.effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::Not(Box::new(any)),
                if_true,
                if_false: Vec::new(),
            }));
            group.consumed += 1;
            return Ok(true);
        }
        // "If you are one of those players, ..."
        if lead.as_slice() == ["if", "you", "are", "one", "of", "those", "players"]
            && let Some(players) = group.last_group.clone()
        {
            let if_true = super::parse_effect_sentence_lexed(rest)?;
            group.effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::Bound(Box::new(crate::ConditionExpr::PlayerIsYou {
                    player: players,
                })),
                if_true,
                if_false: Vec::new(),
            }));
            group.pending_conditions.clear();
            group.consumed += 1;
            return Ok(true);
        }
    }
    if let Some(sweep) = color_choice_sweep(tokens, &group.options) {
        group.effects.push(sweep);
        group.pending_conditions.clear();
        group.consumed += 1;
        return Ok(true);
    }
    let Some((effects, players, extreme)) = quantified_statement(tokens, group)? else {
        // "For each player who chose fame, gain control of a creature that
        // player controls until end of turn. Untap those creatures and they
        // gain haste until end of turn." A statement about "those <objects>"
        // right after a quantified statement acts on the objects each
        // iteration produced, so it continues that iteration's body.
        if tokens.windows(2).any(|pair| {
            pair[0].is_word("those")
                && pair[1].is_any_word(&["creatures", "permanents", "cards", "artifacts", "lands"])
        }) && group.last_group.is_some()
            && let Some(body) = last_iteration_body(&mut group.effects)
        {
            let followup = super::parse_effect_sentence_lexed(tokens)?;
            body.extend(followup);
            group.pending_conditions.clear();
            group.consumed += 1;
            return Ok(true);
        }
        return Ok(false);
    };
    group.effects.extend(effects);
    group.last_group = Some(players);
    if extreme.is_some() {
        group.last_extreme = extreme;
    }
    group.pending_conditions.clear();
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
