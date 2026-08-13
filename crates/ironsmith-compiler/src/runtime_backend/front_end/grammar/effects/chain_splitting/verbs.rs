use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::super::lexer::{OwnedLexToken, parser_token_word_refs};
use super::{ChainVerbKind, ChainVerbMatch};

pub(crate) fn find_chain_verb_tokens(tokens: &[OwnedLexToken]) -> Option<ChainVerbMatch> {
    find_chain_verb_words(&parser_token_word_refs(tokens))
}

pub(crate) fn find_chain_verb_words(words: &[&str]) -> Option<ChainVerbMatch> {
    let mut input = words;
    while !input.is_empty() {
        let word_index = words.len().saturating_sub(input.len());
        let parsed: WResult<&str> = any.parse_next(&mut input);
        let Ok(word) = parsed else {
            break;
        };
        if duration_end_at(words, word_index, word) || counter_noun_at(words, word_index, word) {
            continue;
        }
        if let Some(kind) = parse_chain_verb_word(word) {
            return Some(ChainVerbMatch { kind, word_index });
        }
    }
    None
}

fn duration_end_at(words: &[&str], idx: usize, word: &str) -> bool {
    matches!(word, "end" | "ends")
        && parse_word_at(words, idx.saturating_sub(1), "until")
        && parse_word_at(words, idx + 1, "of")
        && parse_any_word_at(words, idx + 2, &["turn", "combat"])
}

fn counter_noun_at(words: &[&str], idx: usize, word: &str) -> bool {
    matches!(word, "counter" | "counters")
        && parse_any_word_at(words, idx + 1, &["on", "from", "among"])
}

fn parse_word_at(words: &[&str], idx: usize, expected: &'static str) -> bool {
    let Some(word) = words.get(idx) else {
        return false;
    };
    let mut input = std::slice::from_ref(word);
    let parsed: WResult<&str> = any.parse_next(&mut input);
    parsed.is_ok_and(|word| word == expected)
}

fn parse_any_word_at(words: &[&str], idx: usize, expected: &'static [&'static str]) -> bool {
    expected
        .iter()
        .any(|candidate| parse_word_at(words, idx, candidate))
}

fn parse_chain_verb_word(word: &str) -> Option<ChainVerbKind> {
    let mut input = std::slice::from_ref(&word);
    parse_chain_verb_kind.parse_next(&mut input).ok()
}

fn parse_chain_verb_kind(input: &mut &[&str]) -> WResult<ChainVerbKind> {
    let word: &str = any.parse_next(input)?;
    let kind = match word {
        "adds" | "add" => ChainVerbKind::Add,
        "moves" | "move" => ChainVerbKind::Move,
        "deals" | "deal" => ChainVerbKind::Deal,
        "draws" | "draw" => ChainVerbKind::Draw,
        "counters" | "counter" => ChainVerbKind::Counter,
        "destroys" | "destroy" => ChainVerbKind::Destroy,
        "exiles" | "exile" => ChainVerbKind::Exile,
        "reveals" | "reveal" => ChainVerbKind::Reveal,
        "looks" | "look" => ChainVerbKind::Look,
        "loses" | "lose" => ChainVerbKind::Lose,
        "gains" | "gain" => ChainVerbKind::Gain,
        "puts" | "put" => ChainVerbKind::Put,
        "sacrifices" | "sacrifice" => ChainVerbKind::Sacrifice,
        "creates" | "create" => ChainVerbKind::Create,
        "investigates" | "investigate" => ChainVerbKind::Investigate,
        "proliferates" | "proliferate" => ChainVerbKind::Proliferate,
        "taps" | "tap" => ChainVerbKind::Tap,
        "unattaches" | "unattach" => ChainVerbKind::Unattach,
        "attaches" | "attach" => ChainVerbKind::Attach,
        "untaps" | "untap" => ChainVerbKind::Untap,
        "unlocks" | "unlock" => ChainVerbKind::Unlock,
        "scries" | "scry" => ChainVerbKind::Scry,
        "discards" | "discard" => ChainVerbKind::Discard,
        "transforms" | "transform" => ChainVerbKind::Transform,
        "converts" | "convert" => ChainVerbKind::Convert,
        "flips" | "flip" => ChainVerbKind::Flip,
        "rolls" | "roll" => ChainVerbKind::Roll,
        "regenerates" | "regenerate" => ChainVerbKind::Regenerate,
        "heals" | "heal" | "healed" => ChainVerbKind::Heal,
        "mills" | "mill" => ChainVerbKind::Mill,
        "gets" | "get" => ChainVerbKind::Get,
        "removes" | "remove" => ChainVerbKind::Remove,
        "returns" | "return" => ChainVerbKind::Return,
        "exchanges" | "exchange" => ChainVerbKind::Exchange,
        "becomes" | "become" => ChainVerbKind::Become,
        "switches" | "switch" => ChainVerbKind::Switch,
        "skips" | "skip" => ChainVerbKind::Skip,
        "surveils" | "surveil" => ChainVerbKind::Surveil,
        "incubates" | "incubate" => ChainVerbKind::Incubate,
        "shuffles" | "shuffle" => ChainVerbKind::Shuffle,
        "reorders" | "reorder" => ChainVerbKind::Reorder,
        "reverses" | "reverse" => ChainVerbKind::Reverse,
        "pays" | "pay" => ChainVerbKind::Pay,
        "takes" | "take" => ChainVerbKind::Take,
        "detains" | "detain" => ChainVerbKind::Detain,
        "assigns" | "assign" => ChainVerbKind::Assign,
        "goads" | "goad" => ChainVerbKind::Goad,
        "suspects" | "suspect" => ChainVerbKind::Suspect,
        "ends" | "end" => ChainVerbKind::End,
        _ => return Err(backtrack()),
    };
    Ok(kind)
}

fn backtrack() -> ErrMode<ContextError> {
    ErrMode::Backtrack(ContextError::new())
}

#[cfg(test)]
mod tests {
    use crate::runtime_backend::lexer::lex_line;

    use super::*;

    #[test]
    fn token_entrypoint_normalizes_sentence_case() {
        let tokens = lex_line("Discard a card, then draw a card", 0).unwrap();
        assert_eq!(
            find_chain_verb_tokens(&tokens),
            Some(ChainVerbMatch {
                kind: ChainVerbKind::Discard,
                word_index: 0,
            })
        );
    }
}
