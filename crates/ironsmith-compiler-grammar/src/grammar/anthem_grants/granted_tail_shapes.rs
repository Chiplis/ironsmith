use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrantedAbilityCandidate {
    pub has_token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrantedSubjectFacts {
    pub rejected_action: bool,
    pub has_may: bool,
    pub attached_subject: bool,
    pub unbound_pronoun: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeAdditionSubjectSplit<'a> {
    pub base_subject_tokens: &'a [OwnedLexToken],
    pub addition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantedAbilityConditionKind {
    AsLongAs,
    If,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrantedAbilityConditionSplit<'a> {
    pub ability_tokens: &'a [OwnedLexToken],
    pub condition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialGrantedKeyword {
    Blitz,
    Emerge,
    Scavenge,
}

pub fn parse_granted_ability_candidates(tokens: &[OwnedLexToken]) -> Vec<GrantedAbilityCandidate> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    let mut inside_quotes = false;
    let mut prefix_has_get = false;
    let mut candidates = Vec::new();

    while let Ok(token) = take_token(&mut input) {
        let token_index = initial_len.saturating_sub(input.len() + 1);
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes {
            continue;
        }
        if token_is_get(token) {
            prefix_has_get = true;
            continue;
        }
        if token_is_have(token)
            && token_index > 0
            && token_index + 1 < tokens.len()
            && !prefix_has_get
        {
            candidates.push(GrantedAbilityCandidate {
                has_token: token_index,
            });
        }
    }
    candidates
}

pub fn parse_granted_subject_facts(tokens: &[OwnedLexToken]) -> GrantedSubjectFacts {
    let tokens = trim_lexed_commas(tokens);
    let words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    GrantedSubjectFacts {
        rejected_action: contains_parser(tokens, || parse_rejected_subject_action),
        has_may: contains_parser(tokens, || primitives::kw("may").void()),
        attached_subject: primitives::parse_prefix(tokens, parse_attached_subject_head).is_some(),
        unbound_pronoun: crate::word_primitives::parse_any_sequence_complete(
            &words,
            &[&["it"], &["they"], &["them"]],
        ),
    }
}

pub fn split_type_addition_subject(
    tokens: &[OwnedLexToken],
) -> Option<TypeAdditionSubjectSplit<'_>> {
    let mut tokens = trim_lexed_commas(tokens);
    if let Some((head, ())) =
        primitives::split_lexed_once_before_suffix(tokens, 0, || primitives::kw("and").void())
    {
        tokens = trim_lexed_commas(head);
    }
    let search = tokens.get(1..)?;
    let (relative_index, _, _) = primitives::find_prefix(search, || parse_be_word)?;
    let is_token = relative_index + 1;
    let base_subject_tokens = trim_lexed_commas(&tokens[..is_token]);
    let addition_tokens = trim_lexed_commas(&tokens[is_token..]);
    if base_subject_tokens.is_empty() || addition_tokens.is_empty() {
        return None;
    }
    Some(TypeAdditionSubjectSplit {
        base_subject_tokens,
        addition_tokens,
    })
}

pub fn split_granted_ability_condition(
    tokens: &[OwnedLexToken],
    kind: GrantedAbilityConditionKind,
) -> Option<GrantedAbilityConditionSplit<'_>> {
    let tokens = trim_lexed_commas(tokens);
    if contains_parser(tokens, || primitives::quote().void()) {
        return None;
    }
    let search = tokens.get(1..)?;
    let (relative_index, _, rest) = match kind {
        GrantedAbilityConditionKind::AsLongAs => {
            primitives::find_prefix(search, || primitives::phrase(&["as", "long", "as"]).void())?
        }
        GrantedAbilityConditionKind::If => {
            primitives::find_prefix(search, || primitives::kw("if").void())?
        }
    };
    let condition_token = relative_index + 1;
    let ability_tokens = trim_lexed_commas(&tokens[..condition_token]);
    let condition_tokens = trim_lexed_commas(rest);
    if ability_tokens.is_empty() || condition_tokens.is_empty() {
        return None;
    }
    Some(GrantedAbilityConditionSplit {
        ability_tokens,
        condition_tokens,
    })
}

pub fn parse_special_granted_keyword(tokens: &[OwnedLexToken]) -> Option<SpecialGrantedKeyword> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    if parse_complete_keyword(tokens, "emerge") {
        return Some(SpecialGrantedKeyword::Emerge);
    }

    let sentences = primitives::split_lexed_slices_on_period(tokens);
    if sentences.len() < 2 {
        return None;
    }
    let leading = super::trim_anthem_clause_tokens(sentences[0]);
    let mut trailing = Vec::new();
    for sentence in &sentences[1..] {
        trailing.extend_from_slice(super::trim_anthem_clause_tokens(sentence));
    }
    // One declared alternation: the alternatives are exclusive shapes, and the
    // first that reads the input names it.
    let alternation = None::<SpecialGrantedKeyword>
        .or_else(|| {
            if parse_complete_keyword(leading, "blitz")
                && super::parse_granted_blitz_cost_equals_mana(&trailing)
            {
                return Some(SpecialGrantedKeyword::Blitz);
            }
            None
        })
        .or_else(|| {
            if parse_complete_keyword(leading, "emerge")
                && super::parse_granted_emerge_cost_equals_mana(&trailing)
            {
                return Some(SpecialGrantedKeyword::Emerge);
            }
            None
        })
        .or_else(|| {
            if parse_complete_keyword(leading, "scavenge")
                && super::parse_granted_scavenge_cost_equals_mana(&trailing)
            {
                return Some(SpecialGrantedKeyword::Scavenge);
            }
            None
        });
    if let Some(shape) = alternation {
        return Some(shape);
    }
    None
}

fn parse_complete_keyword(tokens: &[OwnedLexToken], keyword: &'static str) -> bool {
    primitives::parse_all(tokens, primitives::kw(keyword), "granted-keyword").is_ok()
}

fn parse_rejected_subject_action(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        alt((
            primitives::kw("deal"),
            primitives::kw("deals"),
            primitives::kw("create"),
            primitives::kw("creates"),
            primitives::kw("draw"),
            primitives::kw("draws"),
            primitives::kw("destroy"),
        )),
        alt((
            primitives::kw("destroys"),
            primitives::kw("exile"),
            primitives::kw("exiles"),
            primitives::kw("return"),
            primitives::kw("returns"),
            primitives::kw("sacrifice"),
            primitives::kw("sacrifices"),
        )),
        alt((
            primitives::kw("put"),
            primitives::kw("puts"),
            primitives::kw("gain"),
            primitives::kw("gains"),
            primitives::kw("lose"),
            primitives::kw("loses"),
            primitives::kw("discard"),
        )),
        alt((
            primitives::kw("discards"),
            primitives::kw("counter"),
            primitives::kw("counters"),
            primitives::kw("search"),
            primitives::kw("reveals"),
            primitives::kw("investigate"),
            primitives::kw("investigates"),
        )),
    ))
    .void()
    .parse_next(input)
}

fn parse_attached_subject_head(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("enchanted"), primitives::kw("equipped")))
        .void()
        .parse_next(input)
}

fn parse_be_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("is"), primitives::kw("are")))
        .void()
        .parse_next(input)
}

fn token_is_get(token: &OwnedLexToken) -> bool {
    let mut input = LexStream::new(std::slice::from_ref(token));
    alt((primitives::kw("get"), primitives::kw("gets")))
        .parse_next(&mut input)
        .is_ok()
}

fn token_is_have(token: &OwnedLexToken) -> bool {
    let mut input = LexStream::new(std::slice::from_ref(token));
    alt((primitives::kw("have"), primitives::kw("has")))
        .parse_next(&mut input)
        .is_ok()
}

fn contains_parser<'a, P, F>(tokens: &'a [OwnedLexToken], make_parser: F) -> bool
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
{
    let mut input = LexStream::new(tokens);
    loop {
        let mut candidate = input.clone();
        if make_parser().parse_next(&mut candidate).is_ok() {
            return true;
        }
        if take_token(&mut input).is_err() {
            return false;
        }
    }
}

fn take_token<'a>(input: &mut LexStream<'a>) -> WResult<&'a OwnedLexToken> {
    any.parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn candidates_ignore_quotes_and_get_prefixes() {
        let tokens = lex_line("Creatures you control have flying", 0).unwrap();
        assert_eq!(parse_granted_ability_candidates(&tokens).len(), 1);

        let tokens = lex_line("Creatures you control get +1/+1 and have flying", 0).unwrap();
        assert!(parse_granted_ability_candidates(&tokens).is_empty());
    }

    #[test]
    fn splits_type_addition_and_trailing_conditions() {
        let subject = lex_line("Other permanents you control are artifacts and", 0).unwrap();
        let split = split_type_addition_subject(&subject).unwrap();
        assert!(!split.base_subject_tokens.is_empty());
        assert!(!split.addition_tokens.is_empty());

        let subject = lex_line(
            "Clues you control are Equipment in addition to their other types and",
            0,
        )
        .unwrap();
        let split = split_type_addition_subject(&subject).unwrap();
        assert_eq!(
            super::super::super::super::lexer::render_token_slice(split.base_subject_tokens),
            "Clues you control"
        );
        assert_eq!(
            super::super::super::super::lexer::render_token_slice(split.addition_tokens),
            "are Equipment in addition to their other types"
        );

        let ability = lex_line("flying as long as you control an artifact", 0).unwrap();
        let split =
            split_granted_ability_condition(&ability, GrantedAbilityConditionKind::AsLongAs)
                .unwrap();
        assert_eq!(split.ability_tokens.len(), 1);
        assert!(!split.condition_tokens.is_empty());
    }

    #[test]
    fn recognizes_special_granted_keyword_sentences() {
        let blitz = lex_line("Blitz. Its blitz cost is equal to its mana cost.", 0).unwrap();
        assert_eq!(
            parse_special_granted_keyword(&blitz),
            Some(SpecialGrantedKeyword::Blitz)
        );

        let emerge = lex_line("Emerge", 0).unwrap();
        assert_eq!(
            parse_special_granted_keyword(&emerge),
            Some(SpecialGrantedKeyword::Emerge)
        );
    }
}
