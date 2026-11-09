//! Judge (custom CR 701.96): "To judge a permanent, honor it or condemn it.
//! If you honor it, put a +1/+1 counter on it. If you condemn it, sacrifice
//! it, then draw a card." A judging sentence is the same complete named
//! program the explicit "performs the judge action as follows" wording reads;
//! the judging player chooses and performs every instruction.
use crate::lexer::{OwnedLexToken, TokenKind};

fn program(object: &str) -> String {
    format!(
        "You choose honor or condemn; If you chose honor, perform the honor action with {object}, \
         then put a +1/+1 counter on {object}; If you chose condemn, perform the condemn action \
         with {object}, then sacrifice {object}, then draw a card"
    )
}

/// The judged operand as the program's demonstrative, or `None` when the
/// operand is not one this expansion can name exactly.
fn operand(words: &[&str], antecedent_noun: &str) -> Option<String> {
    match words {
        ["it"] => Some(format!("that {antecedent_noun}")),
        ["that", noun @ ("creature" | "permanent" | "token")] => Some(format!("that {noun}")),
        _ => None,
    }
}

fn sentence_text(tokens: &[OwnedLexToken]) -> String {
    crate::lexer::render_token_slice(tokens)
}

/// Collect backing (custom CR 701.106a): "collect backing N" = for each of
/// N different colors, tap an untapped creature you control of that color or
/// exile a card of that color from your graveyard — the defined quoted cost.
fn collect_backing_sentence(tokens: &[OwnedLexToken]) -> Option<String> {
    let words = crate::lexer::token_word_refs(tokens);
    let at = words.windows(2).position(|pair| pair == ["collect", "backing"])?;
    let amount = *words.get(at + 2)?;
    if at + 3 != words.len() || !(amount.chars().all(|ch| ch.is_ascii_digit()) || amount == "x") {
        return None;
    }
    let view = crate::lexer::TokenWordView::new(tokens);
    let start = *view.token_start_indices().get(at)?;
    let head = sentence_text(&tokens[..start]);
    Some(format!(
        "{head} pay the cost \"Choose {amount} untapped creatures you control or cards from your graveyard, \
         each representing a different color it has; Tap the chosen creatures on the battlefield; \
         Exile the chosen cards from your graveyard\"."
    ))
}

/// Expand one judging sentence (without its terminal period), or `None`.
fn expand_sentence(tokens: &[OwnedLexToken]) -> Option<String> {
    if let Some(expanded) = collect_backing_sentence(tokens) {
        return Some(expanded);
    }
    let verb = tokens
        .iter()
        .position(|token| token.is_any_word(&["judge", "judges"]))?;
    let words = crate::lexer::token_word_refs(&tokens[verb + 1..]);
    let prefix = &tokens[..verb];
    let prefix_words = crate::lexer::token_word_refs(prefix);
    // "Target creature's controller judges it."
    if let ["target", possessive, role @ ("controller" | "owner")] = prefix_words.as_slice()
        && let Some(noun) = possessive.strip_suffix("'s").or_else(|| possessive.strip_suffix("’s"))
        && tokens[verb].is_word("judges")
    {
        let object = operand(&words, noun)?;
        return Some(format!(
            "Choose target {noun}. Its {role} performs the judge action as follows: [{}].",
            program(&object)
        ));
    }
    // "<clause>, then judges that token." / "<clause>, then judge each of them."
    let [.., comma, then] = prefix else { return None };
    if comma.kind != TokenKind::Comma || !then.is_word("then") {
        return None;
    }
    let head = &prefix[..prefix.len() - 2];
    if head.is_empty() {
        return None;
    }
    let head_text = sentence_text(head);
    if tokens[verb].is_word("judges") {
        // A third-person verb continues the head's player subject.
        let object = operand(&words, "permanent")?;
        return Some(format!(
            "{head_text}. That player performs the judge action as follows: [{}].",
            program(&object)
        ));
    }
    if words.as_slice() == ["each", "of", "them"] {
        return Some(format!(
            "{head_text}. For each of them, do the following: [Perform the judge action as follows: {}].",
            program("that permanent")
        ));
    }
    let object = operand(&words, "permanent")?;
    Some(format!(
        "{head_text}. Perform the judge action as follows: [{}].",
        program(&object)
    ))
}

/// Rewrite every judging sentence of an instruction program into its
/// complete named program; `None` when no sentence judges. Other sentences
/// keep their authored tokens.
pub(crate) fn expand(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    if !tokens.iter().any(|token| token.is_any_word(&["judge", "judges", "backing"])) {
        return None;
    }
    let mut out = Vec::new();
    let mut changed = false;
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut sentences = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LBracket => depth += 1,
            TokenKind::RBracket => depth -= 1,
            TokenKind::Period if depth == 0 => {
                sentences.push((start, index + 1));
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < tokens.len() {
        sentences.push((start, tokens.len()));
    }
    for (start, end) in sentences {
        let sentence = &tokens[start..end];
        let body_end = if sentence.last().is_some_and(|token| token.kind == TokenKind::Period) {
            sentence.len() - 1
        } else {
            sentence.len()
        };
        if let Some(expanded) = expand_sentence(&sentence[..body_end])
            && let Ok(lexed) = crate::lexer::lex_synthetic(&expanded)
        {
            crate::word_claims::claim_tokens(sentence);
            out.extend(lexed);
            changed = true;
        } else {
            out.extend_from_slice(sentence);
        }
    }
    changed.then_some(out)
}
