use super::*;

pub fn parse_simple_ability_duration_shape(words: &[&str]) -> Option<GainAbilityDurationShape> {
    let mut start = 0usize;
    while start < words.len() {
        let mut input = &words[start..];
        if let Ok(duration) = continuous_duration.parse_next(&mut input) {
            return Some(GainAbilityDurationShape {
                start,
                len: words[start..].len().saturating_sub(input.len()),
                duration,
                condition: None,
            });
        }
        let mut input = &words[start..];
        if you_control.parse_next(&mut input).is_ok()
            && let Some(duration) = you_control_object_duration(input)
        {
            // "for as long as you control <object>": the whole tail is the
            // controlled object, read word for word below.
            crate::word_claims::claim_words(&words[start..]);
            return Some(GainAbilityDurationShape {
                start,
                len: words.len().saturating_sub(start),
                duration,
                condition: None,
            });
        }
        start += 1;
    }
    None
}

/// The object a "for as long as you control ..." duration names. Never
/// consumes an unread tail word: an object this cannot model declines.
fn you_control_object_duration(object: &[&str]) -> Option<Until> {
    use ironsmith_core::{
        ContinuousDurationObject as Object, ContinuousDurationPlayer as Player,
        ContinuousDurationPredicate as Predicate,
    };
    // "this creature" / "this Saga" / the normalized source name.
    if crate::util::is_source_reference_words(object) {
        return Some(Until::YouStopControllingThis);
    }
    // "this creature and this creature remains tapped" (Rubinia
    // Soulsinger-style): both conditions hold the effect (CR 611.2b).
    if let Some(and) = object.iter().position(|word| *word == "and")
        && let [subject @ .., "remains", "tapped"] = &object[and + 1..]
        && crate::util::is_source_reference_words(&object[..and])
        && crate::util::is_source_reference_words(subject)
    {
        return Some(Until::ForAsLongAs(Predicate::All(vec![
            Predicate::ObjectControlledBy {
                object: Object::Source,
                player: Player::EffectController,
            },
            Predicate::ObjectTapped(Object::Source),
        ])));
    }
    // "that creature": the single object the effect acts on.
    if let ["that", "creature" | "permanent" | "artifact" | "enchantment" | "land" | "planeswalker"] =
        object
    {
        return Some(Until::ForAsLongAs(Predicate::ObjectControlledBy {
            object: Object::AffectedObject,
            player: Player::EffectController,
        }));
    }
    // "both of them" / "those creatures" / "them" / "it": the objects the
    // preceding instruction named. The engine holds the effect while every
    // tagged object stays under your control.
    if matches!(
        object,
        ["both", "of", "them"]
            | ["them"]
            | ["it"]
            | ["those", "creatures" | "permanents" | "artifacts" | "lands" | "enchantments"]
    ) {
        return Some(Until::ForAsLongAs(Predicate::ObjectControlledBy {
            object: Object::Tagged(crate::tag::CompilerReferenceTag::It.bind().into()),
            player: Player::EffectController,
        }));
    }
    None
}

pub fn parse_gain_ability_duration_shape(words: &[&str]) -> Option<GainAbilityDurationShape> {
    parse_simple_ability_duration_shape(words)
}

pub fn parse_leading_gain_duration_shape(words: &[&str]) -> Option<LeadingGainDurationShape> {
    let mut input: WordSliceInput<'_> = words;
    let duration = crate::grammar::primitives::take_leaf(&mut input, continuous_duration)?;
    Some(LeadingGainDurationShape {
        consumed_words: words.len().saturating_sub(input.len()),
        duration,
    })
}

pub fn parse_quoted_gain_duration_shape(
    tokens: &[OwnedLexToken],
    gain_token_idx: usize,
) -> Option<QuotedGainDurationShape> {
    let open_quote_idx = gain_token_idx.checked_add(1)?;
    primitives::parse_prefix(tokens.get(open_quote_idx..)?, primitives::quote())?;
    let after_open = tokens.get(open_quote_idx + 1..)?;
    let (relative_close, _, _) = primitives::find_prefix(after_open, primitives::quote)?;
    let close_quote_token = open_quote_idx + 1 + relative_close;
    let tail_tokens = trim_lexed_commas(tokens.get(close_quote_token + 1..)?);
    if tail_tokens.is_empty() {
        return None;
    }
    if tail_tokens.iter().any(|token| !matches!(token.kind, crate::lexer::TokenKind::Word | crate::lexer::TokenKind::Comma | crate::lexer::TokenKind::Period)) { return None; }
    let tail_words = TokenWordView::new(tail_tokens).to_word_refs();
    // Contextual object-zone atoms are accepted only in this explicitly
    // quoted grant owner. Other duration consumers do not receive unresolved
    // antecedent placeholders merely because their word sequence matches.
    let mut contextual_input=tail_words.as_slice();
    let contextual=super::core_programs::object_zone_duration.parse_next(&mut contextual_input).ok()
        .filter(|_|contextual_input.is_empty())
        .map(|duration|GainAbilityDurationShape{start:0,len:tail_words.len(),duration,condition:None});
    let parsed = contextual.or_else(||parse_simple_ability_duration_shape(&tail_words))?;
    if parsed.start != 0 || parsed.len != tail_words.len() {
        return None;
    }
    Some(QuotedGainDurationShape {
        close_quote_token,
        duration: parsed.duration,
    })
}
