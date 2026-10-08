//! "If <event> would happen, <program> instead." (CR 614.1a): a damage or
//! life-gain event replaced by an arbitrary effect program, which runs with
//! the replaced event as its context ("that much", "that many", "that
//! player"). Event modifications (amount changes, redirection, prevention)
//! keep their own readers; this reader declines any body that names them.

use super::*;
use ironsmith_core::ReplacedEventSpec;

/// Body words that belong to a modification or prevention of the event
/// rather than to a replacement program, owned by the specialized readers.
const MODIFICATION_WORDS: &[&str] = &[
    "prevent", "damage", "double", "twice", "plus", "gain", "gains", "may",
];

fn words_of(tokens: &[OwnedLexToken]) -> Vec<&str> {
    crate::lexer::token_word_refs(tokens)
}

/// The players an event subject names.
fn player_subject(words: &[&str]) -> Option<PlayerFilter> {
    match words {
        ["you"] => Some(PlayerFilter::You),
        ["a", "player"] => Some(PlayerFilter::Any),
        ["an", "opponent"] => Some(PlayerFilter::Opponent),
        _ => None,
    }
}

/// The source of the ability ("this creature", a normalized self-name).
fn source_subject(words: &[&str]) -> Option<ObjectFilter> {
    if !crate::util::is_source_reference_words(words) {
        return None;
    }
    let mut filter = ObjectFilter::source();
    filter.source_surface = crate::util::source_reference_surface_for_words(words);
    Some(filter)
}

/// The watched event, read from the clause between "if" and the comma.
fn replaced_event(header: &[OwnedLexToken]) -> Option<ReplacedEventSpec> {
    let words = words_of(header);
    let ["if", rest @ ..] = words.as_slice() else {
        return None;
    };
    let would = rest.iter().position(|word| *word == "would")?;
    let (subject, predicate) = (&rest[..would], &rest[would + 1..]);
    match predicate {
        // "If damage would be dealt to you", "If combat damage would be dealt
        // to this creature".
        ["be", "dealt", "to", recipient @ ..] => {
            let combat_only = match subject {
                ["damage"] => false,
                ["combat", "damage"] => true,
                _ => return None,
            };
            if let Some(player) = player_subject(recipient) {
                return Some(ReplacedEventSpec::DamageToPlayer {
                    player,
                    source_filter: None,
                    combat_only,
                });
            }
            Some(ReplacedEventSpec::DamageToObject {
                target: source_subject(recipient)?,
                source_filter: None,
                combat_only,
            })
        }
        // "If this creature would be dealt damage".
        ["be", "dealt", "damage"] | ["be", "dealt", "combat", "damage"] => {
            Some(ReplacedEventSpec::DamageToObject {
                target: source_subject(subject)?,
                source_filter: None,
                combat_only: predicate.len() == 4,
            })
        }
        // "If Szadek would deal combat damage to a player", "If a Zombie you
        // control would deal combat damage to a player".
        ["deal", "damage", "to", recipient @ ..]
        | ["deal", "combat", "damage", "to", recipient @ ..] => {
            let combat_only = predicate.get(1) == Some(&"combat");
            let player = player_subject(recipient)?;
            let source_filter = match source_subject(subject) {
                Some(filter) => filter,
                None => {
                    // The subject words are the header tokens after "if";
                    // header words and tokens coincide (no punctuation).
                    let subject_tokens = header.get(1..1 + subject.len())?;
                    if words_of(subject_tokens) != subject {
                        return None;
                    }
                    let filter =
                        crate::object_filters::parse_object_filter_lexed(subject_tokens, false)
                            .ok()?;
                    if filter == ObjectFilter::default() {
                        return None;
                    }
                    filter
                }
            };
            Some(ReplacedEventSpec::DamageToPlayer {
                player,
                source_filter: Some(source_filter),
                combat_only,
            })
        }
        // "If an opponent would gain life".
        ["gain", "life"] => Some(ReplacedEventSpec::LifeGain {
            player: player_subject(subject)?,
        }),
        _ => None,
    }
}

pub fn parse_if_event_would_happen_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(comma) = tokens.iter().position(|token| token.is_comma()) else {
        return Ok(None);
    };
    let header = &tokens[..comma];
    if header.iter().any(|token| token.as_word().is_none()) {
        return Ok(None);
    }
    let Some(event) = replaced_event(header) else {
        return Ok(None);
    };
    let mut body: Vec<OwnedLexToken> = tokens[comma + 1..].to_vec();
    // Exactly one "instead": leading the program, or closing its first
    // sentence ("..., exile that many cards from your graveyard instead. If
    // you can't, you lose the game.").
    let leading = body.first().is_some_and(|token| token.is_word("instead"));
    if leading {
        body.remove(0);
    }
    let sentence_end = body
        .iter()
        .position(|token| token.is_period())
        .unwrap_or(body.len());
    let trailing = sentence_end > 0 && body[sentence_end - 1].is_word("instead");
    if leading == trailing {
        return Ok(None);
    }
    if trailing {
        body.remove(sentence_end - 1);
    }
    if body.iter().any(|token| token.is_word("instead") || token.is_quote())
        || words_of(&body)
            .iter()
            .any(|word| MODIFICATION_WORDS.contains(word))
    {
        return Ok(None);
    }
    // "+1/+1 counters on it instead" for the source is the specialized
    // put-counter replacement's reading; keep a single owner.
    if matches!(event, ReplacedEventSpec::DamageToObject { .. })
        && words_of(&body).contains(&"+1/+1")
    {
        return Ok(None);
    }
    // "put that many -1/-1 counters on it instead" (Lichenthrope): when the
    // damaged permanent is the source itself, "it" is that source.
    if let ReplacedEventSpec::DamageToObject { target, .. } = &event
        && target.source
        && let Some(last) = body
            .iter()
            .rposition(|token| !token.is_period())
        && body[last].is_word("it")
    {
        let replacement = crate::lexer::synthetic_word_tokens(["this", "permanent"]);
        body.splice(last..=last, replacement);
    }
    let body = crate::util::trim_edge_punctuation_tokens(&body);
    if body.is_empty() {
        return Ok(None);
    }
    // A program this reader cannot read stays unclaimed rather than adding a
    // diagnostic to lines other readers own.
    let Ok(effects) = crate::clause_support::parse_effect_sentences_lexed(body) else {
        return Ok(None);
    };
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(StaticAbility::event_replacement_with_effects(
        event,
        effects,
        crate::lexer::render_token_slice(tokens),
    )))
}
