use super::super::super::lexer::{OwnedLexToken, trim_lexed_commas};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionalCompanionJoin {
    Each,
    Both,
    Plain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedSubjectOptionalCompanionShape<'a> {
    pub first_subject_tokens: &'a [OwnedLexToken],
    pub companion_tokens: &'a [OwnedLexToken],
    pub action_tokens: &'a [OwnedLexToken],
    pub join: OptionalCompanionJoin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeadingOptionalCompanionVerb {
    Destroy,
    Tap,
    Untap,
    /// Any other verb whose object is a target phrase ("exile", "return",
    /// "regenerate", a custom card's verb, ...).
    Other,
}

/// "<verb> <slot>, <slot>, and <slot>": one action applied to independently
/// chosen target slots, each with its own count ("up to one target creature
/// and up to two target lands", "target artifact and one target land").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeadingOptionalCompanionShape<'a> {
    pub verb: LeadingOptionalCompanionVerb,
    pub verb_token: &'a OwnedLexToken,
    /// Each slot's own tokens; the last slot also carries any trailing words
    /// the action shares (a destination such as "to their owners' hands").
    pub slots: Vec<&'a [OwnedLexToken]>,
    /// The separators between slots (",", "and", ", and").
    pub separators: Vec<&'a [OwnedLexToken]>,
    /// Trailing words of the last slot that every slot's action shares, when
    /// the last slot ends in a prepositional tail after its target noun.
    pub shared_tail: &'a [OwnedLexToken],
}

impl<'a> LeadingOptionalCompanionShape<'a> {
    pub fn first_target_tokens(&self) -> &'a [OwnedLexToken] {
        self.slots[0]
    }

    pub fn companion_tokens(&self) -> &'a [OwnedLexToken] {
        self.slots[self.slots.len() - 1]
    }
}

const TARGET_COUNT_WORDS: &[&str] = &["up", "to", "another", "other", "any", "number", "of", "x"];

/// Whether a target slot starts at `start`: optional count words ("up to
/// one", "two", "another", "any number of") followed by "target".
fn target_slot_starts_at(tokens: &[OwnedLexToken], start: usize) -> bool {
    for token in tokens.iter().skip(start).take(6) {
        let Some(text) = word(token) else {
            return false;
        };
        if text == "target" {
            return true;
        }
        if !(TARGET_COUNT_WORDS.contains(&text)
            || crate::util::parse_number_word_u32(text).is_some()
            || text.parse::<u32>().is_ok())
        {
            return false;
        }
    }
    false
}

/// Split a verb's object list into independently counted target slots.
/// Only a separator that is directly followed by a target slot splits; any
/// other comma or "and" belongs to the current slot's own filter.
pub fn split_target_slot_list(
    body: &[OwnedLexToken],
) -> Option<(Vec<&[OwnedLexToken]>, Vec<&[OwnedLexToken]>)> {
    let mut slots = Vec::new();
    let mut separators = Vec::new();
    let mut slot_start = 0usize;
    let mut index = 1usize;
    while index < body.len() {
        let token = &body[index];
        let next_start = if word(token) == Some("and") {
            Some(index + 1)
        } else if token.is_comma() {
            Some(if body.get(index + 1).and_then(word) == Some("and") {
                index + 2
            } else {
                index + 1
            })
        } else {
            None
        };
        if let Some(next_start) = next_start
            && index > slot_start
            && target_slot_starts_at(body, next_start)
        {
            slots.push(&body[slot_start..index]);
            separators.push(&body[index..next_start]);
            slot_start = next_start;
            index = next_start + 1;
            continue;
        }
        index += 1;
    }
    if slots.is_empty() || slot_start >= body.len() {
        return None;
    }
    slots.push(&body[slot_start..]);
    Some((slots, separators))
}

/// The prepositional tail of the last slot after its target noun phrase
/// ("... target land card from your graveyard to the battlefield"), which a
/// shared action such as "return" applies to every slot.
fn shared_slot_tail(last_slot: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let Some(target) = last_slot.iter().position(|token| word(token) == Some("target")) else {
        return &[];
    };
    last_slot[target + 1..]
        .iter()
        .position(|token| {
            matches!(word(token), Some("to" | "into" | "onto" | "from" | "until" | "under"))
        })
        .map_or(&[], |offset| {
            let tail = &last_slot[target + 1 + offset..];
            // The tail is the shared destination or duration only; a comma
            // starts a following clause.
            let end = tail.iter().position(OwnedLexToken::is_comma).unwrap_or(tail.len());
            &tail[..end]
        })
}

fn word(token: &OwnedLexToken) -> Option<&str> {
    token.as_word().map(|_| token.parser_text())
}

fn is_cant(token: &OwnedLexToken) -> bool {
    matches!(word(token), Some("cant" | "can't"))
}

fn optional_companion_separator(tokens: &[OwnedLexToken]) -> Option<usize> {
    crate::slice_primitives::find_window_by(tokens, 3, |window| {
        word(&window[0]) == Some("and")
            && word(&window[1]) == Some("up")
            && word(&window[2]) == Some("to")
    })
}

fn explicit_target_phrase_count(tokens: &[OwnedLexToken]) -> usize {
    tokens
        .iter()
        .filter(|token| token.is_word("target"))
        .count()
}

/// Captures a shared predicate whose subjects are one required/source object
/// plus an independently optional target, for example:
///
/// `this creature and up to one other target creature each get ...`
///
/// The shape deliberately owns only coordination. Target counts, `other`,
/// controller restrictions, the action, and the duration stay delegated to
/// their existing semantic parsers.
pub fn parse_shared_subject_optional_companion_shape(
    tokens: &[OwnedLexToken],
) -> Option<SharedSubjectOptionalCompanionShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    // "This creature and another target creature each get +1/+0" (Deadly
    // Dancer): a required companion target coordinates the same way.
    let separator = optional_companion_separator(tokens).or_else(|| {
        crate::slice_primitives::find_window_by(tokens, 3, |window| {
            word(&window[0]) == Some("and")
                && word(&window[1]) == Some("another")
                && word(&window[2]) == Some("target")
        })
    })?;
    let first_subject_tokens = trim_lexed_commas(&tokens[..separator]);
    if first_subject_tokens.is_empty() {
        return None;
    }

    let companion_and_action = &tokens[separator + 1..];
    let coordinated_action =
        crate::slice_primitives::find_window_by(companion_and_action, 2, |window| {
            matches!(
                (word(&window[0]), word(&window[1])),
                (Some("each"), Some("get" | "gets")) | (Some("both"), Some("gain" | "gains"))
            )
        })
        .map(|idx| {
            let join = if word(&companion_and_action[idx]) == Some("each") {
                OptionalCompanionJoin::Each
            } else {
                OptionalCompanionJoin::Both
            };
            (idx, idx + 1, join)
        });
    let (action_idx, action_word_idx, join) = coordinated_action.or_else(|| {
        crate::slice_primitives::select_position(companion_and_action, is_cant)
            .map(|idx| (idx, idx, OptionalCompanionJoin::Plain))
    })?;

    let companion_tokens = trim_lexed_commas(&companion_and_action[..action_idx]);
    let action_tokens = trim_lexed_commas(&companion_and_action[action_word_idx..]);
    if companion_tokens.is_empty()
        || action_tokens.is_empty()
        || !companion_tokens.iter().any(|token| token.is_word("target"))
    {
        return None;
    }
    // "each" / "both": the distributive join read into `join`.
    if join != OptionalCompanionJoin::Plain {
        crate::word_claims::claim_tokens(&companion_and_action[action_idx..action_idx + 1]);
    }

    Some(SharedSubjectOptionalCompanionShape {
        first_subject_tokens,
        companion_tokens,
        action_tokens,
        join,
    })
}

/// Captures a leading action applied to independently counted target slots:
/// `tap it and up to one target ...`, `destroy target artifact and up to one
/// other target artifact`, `untap up to one target creature and up to two
/// target lands`, `exile target creature, up to one target artifact, and up
/// to one target enchantment`.
pub fn parse_leading_optional_companion_shape(
    tokens: &[OwnedLexToken],
) -> Option<LeadingOptionalCompanionShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let verb_token = tokens.first()?;
    let verb = match word(verb_token)? {
        "destroy" => LeadingOptionalCompanionVerb::Destroy,
        "tap" => LeadingOptionalCompanionVerb::Tap,
        "untap" => LeadingOptionalCompanionVerb::Untap,
        // Verbs whose object is not a list of target slots: a counter, a
        // token, a card choice, or a revealed/looked-at set.
        "put" | "choose" | "create" | "search" | "look" | "reveal" | "distribute" | "deal"
        | "deals" | "if" | "when" | "whenever" | "at" | "as" | "until" | "you" | "each"
        | "target" | "up" => return None,
        _ => LeadingOptionalCompanionVerb::Other,
    };
    let body = &tokens[1..];
    // A following action (", then discard a card") or clause break is not
    // part of any slot; the chain grammar owns the sequence.
    if body.iter().any(|token| {
        matches!(word(token), Some("then")) || token.kind == crate::lexer::TokenKind::Semicolon
    }) {
        return None;
    }
    let (slots, separators) = split_target_slot_list(body)?;
    // "an opponent chooses target creature they control" / "target creature
    // of an opponent's choice": a slot with its own chooser is not a plain
    // target slot of this action.
    if slots.iter().any(|slot| {
        slot.iter()
            .any(|token| matches!(word(token), Some("choice" | "chooses" | "choose")))
    }) {
        return None;
    }
    // "Shuffle this creature and target creature ..." (Floodpits Drowner):
    // for a general verb every slot, the first included, is a target slot;
    // a leading source reference keeps its own reading.
    if verb == LeadingOptionalCompanionVerb::Other && !target_slot_starts_at(slots[0], 0) {
        return None;
    }
    // A non-final slot with its own zone or destination phrase ("target
    // card from your graveyard and target card from an opponent's
    // graveyard") is an independent clause, not a slot sharing the last
    // slot's tail.
    if slots[..slots.len() - 1]
        .iter()
        .any(|slot| !shared_slot_tail(slot).is_empty())
    {
        return None;
    }
    // "Destroy target X, target Y, and target Z" is the destroy family's
    // own multi-target list (`parse_destroy_multi_target_shape`), which
    // already gives every slot its own choice; this shape keeps only the
    // destroy pair it has always owned.
    if verb == LeadingOptionalCompanionVerb::Destroy
        && (slots.len() > 2
            || explicit_target_phrase_count(body) > 2
            || slots[1].first().and_then(word) != Some("up"))
    {
        return None;
    }
    if slots.iter().any(|slot| trim_lexed_commas(slot).is_empty()) {
        return None;
    }
    let shared_tail = shared_slot_tail(slots[slots.len() - 1]);
    Some(LeadingOptionalCompanionShape {
        verb,
        verb_token,
        slots: slots.into_iter().map(trim_lexed_commas).collect(),
        separators,
        shared_tail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{lex_line, render_token_slice};

    #[test]
    fn captures_source_optional_target_and_shared_predicate_separately() {
        let tokens = lex_line(
            "This creature and up to one other target creature you control each get +3/+3 until end of turn.",
            0,
        )
        .unwrap();
        let shape = parse_shared_subject_optional_companion_shape(&tokens).unwrap();
        assert_eq!(
            render_token_slice(shape.first_subject_tokens),
            "This creature"
        );
        assert_eq!(
            render_token_slice(shape.companion_tokens),
            "up to one other target creature you control"
        );
        assert_eq!(
            render_token_slice(shape.action_tokens),
            "get +3/+3 until end of turn."
        );
        assert_eq!(shape.join, OptionalCompanionJoin::Each);
    }

    #[test]
    fn captures_named_source_with_optional_target() {
        let tokens = lex_line(
            "Lockjaw and up to one other target creature you control can't be blocked this turn.",
            0,
        )
        .unwrap();
        let shape = parse_shared_subject_optional_companion_shape(&tokens).unwrap();
        assert_eq!(render_token_slice(shape.first_subject_tokens), "Lockjaw");
        assert_eq!(
            render_token_slice(shape.companion_tokens),
            "up to one other target creature you control"
        );
        assert_eq!(
            render_token_slice(shape.action_tokens),
            "can't be blocked this turn."
        );
        assert_eq!(shape.join, OptionalCompanionJoin::Plain);
    }

    #[test]
    fn captures_leading_action_without_absorbing_the_optional_count() {
        let tokens = lex_line(
            "Destroy target artifact or enchantment and up to one other target artifact or enchantment.",
            0,
        )
        .unwrap();
        let shape = parse_leading_optional_companion_shape(&tokens).unwrap();
        assert_eq!(shape.verb, LeadingOptionalCompanionVerb::Destroy);
        assert_eq!(
            render_token_slice(shape.first_target_tokens()),
            "target artifact or enchantment"
        );
        assert_eq!(
            render_token_slice(shape.companion_tokens()),
            "up to one other target artifact or enchantment."
        );
    }

    #[test]
    fn leading_pair_does_not_claim_a_multi_slot_shared_action() {
        let tokens = lex_line(
            "Destroy up to one target artifact, up to one target creature, and up to one target enchantment.",
            0,
        )
        .unwrap();
        assert!(parse_leading_optional_companion_shape(&tokens).is_none());
    }
}
