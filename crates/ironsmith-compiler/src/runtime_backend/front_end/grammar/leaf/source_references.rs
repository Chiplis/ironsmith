use winnow::ascii::space1;
use winnow::combinator::{alt, eof};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::{literal, rest, take_until, take_while};

use crate::target::SourceReferenceSurface;

use super::super::super::lexer::{lex_line, parser_token_word_refs};
use super::super::primitives;
use super::filter_atoms::{
    parse_leaf_card_type, parse_leaf_card_type_complete, parse_leaf_color_complete,
    parse_leaf_subtype_flexible, parse_leaf_subtype_flexible_complete,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeafSourceReferenceAlias {
    pub(crate) words: Vec<String>,
    pub(crate) surface: SourceReferenceSurface,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LeafSourceAnaphor {
    It,
    Its,
    This(SourceReferenceSurface),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeafThisSourceNoun {
    Generic,
    CardType,
    Subtype,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LeafRomanNumeral;

pub(crate) fn parse_leaf_source_reference_aliases_for_name(
    name: &str,
) -> Vec<LeafSourceReferenceAlias> {
    let mut aliases = Vec::new();
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return aliases;
    }

    let mut full_names = Vec::new();
    push_unique_name(&mut full_names, trimmed);
    if let Some(front_face) = parse_name_prefix(trimmed, parse_front_face_name) {
        push_unique_name(&mut full_names, front_face);
    }
    let base_full_names = full_names.clone();
    for full_name in base_full_names {
        if let Some(stripped) = parse_name_prefix(&full_name, parse_digital_variant_name) {
            push_unique_name(&mut full_names, stripped);
        }
        if let Some(stripped) = strip_trailing_roman_numeral(&full_name) {
            push_unique_name(&mut full_names, stripped);
        }
    }

    for full_name in &full_names {
        push_leaf_source_reference_alias(
            &mut aliases,
            full_name,
            SourceReferenceSurface::FullName(full_name.clone()),
        );
        if let Some(without_article) = parse_name_prefix(full_name, parse_leading_name_article) {
            push_leaf_source_reference_alias(
                &mut aliases,
                without_article,
                SourceReferenceSurface::FullName(full_name.clone()),
            );
        }
    }

    for full_name in &full_names {
        if let Some(short_name) = parse_name_prefix(full_name, parse_comma_short_name) {
            let short_name = short_name.trim();
            push_leaf_source_reference_alias(
                &mut aliases,
                short_name,
                SourceReferenceSurface::ShortName(short_name.to_string()),
            );
            if let Some(unmarked) = parse_name_prefix(short_name, parse_digital_variant_name) {
                push_leaf_source_reference_alias(
                    &mut aliases,
                    unmarked,
                    SourceReferenceSurface::ShortName(unmarked.to_string()),
                );
            }
        } else if let Some(unmarked) = parse_name_prefix(full_name, parse_digital_variant_name) {
            let unmarked = unmarked.trim();
            push_leaf_source_reference_alias(
                &mut aliases,
                unmarked,
                SourceReferenceSurface::ShortName(unmarked.to_string()),
            );
        } else if let Some(short_name) = parse_name_prefix(full_name, parse_first_name_word) {
            let short_name = short_name.trim();
            if short_name_is_distinct_name(short_name) {
                push_leaf_source_reference_alias(
                    &mut aliases,
                    short_name,
                    SourceReferenceSurface::ShortName(short_name.to_string()),
                );
            }
        }
    }

    sort_leaf_source_reference_aliases(&mut aliases);
    aliases
}

pub(crate) fn push_leaf_source_reference_alias(
    aliases: &mut Vec<LeafSourceReferenceAlias>,
    raw: &str,
    surface: SourceReferenceSurface,
) {
    for words in parse_source_reference_word_variants(raw) {
        push_leaf_source_reference_alias_words(aliases, words, surface.clone());
    }
}

pub(crate) fn push_leaf_source_reference_alias_words(
    aliases: &mut Vec<LeafSourceReferenceAlias>,
    words: Vec<String>,
    surface: SourceReferenceSurface,
) {
    if !words.is_empty() && !aliases.iter().any(|alias| alias.words == words) {
        aliases.push(LeafSourceReferenceAlias { words, surface });
    }
}

pub(crate) fn sort_leaf_source_reference_aliases(aliases: &mut [LeafSourceReferenceAlias]) {
    aliases.sort_by_key(|alias| std::cmp::Reverse(alias.words.len()));
}

pub(crate) fn parse_leaf_source_reference_alias_words(
    aliases: &[LeafSourceReferenceAlias],
    words: &[&str],
) -> Option<SourceReferenceSurface> {
    parse_leaf_source_reference_alias_words_with_mode(aliases, words, false)
}

pub(crate) fn parse_leaf_source_reference_possessive_alias_words(
    aliases: &[LeafSourceReferenceAlias],
    words: &[&str],
) -> Option<SourceReferenceSurface> {
    parse_leaf_source_reference_alias_words_with_mode(aliases, words, true)
}

fn parse_leaf_source_reference_alias_words_with_mode(
    aliases: &[LeafSourceReferenceAlias],
    words: &[&str],
    allow_possessive: bool,
) -> Option<SourceReferenceSurface> {
    let normalized = words
        .iter()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    for alias in aliases {
        if alias.words.len() != words.len() {
            continue;
        }
        let mut input = normalized.as_str();
        let parsed = parse_dynamic_alias(&mut input, alias, allow_possessive);
        if let Ok(surface) = parsed {
            return Some(surface);
        }
    }
    None
}

fn parse_dynamic_alias(
    input: &mut &str,
    alias: &LeafSourceReferenceAlias,
    allow_possessive: bool,
) -> WResult<SourceReferenceSurface> {
    for (index, expected) in alias.words.iter().enumerate() {
        if index > 0 {
            space1.parse_next(input)?;
        }
        if allow_possessive && index + 1 == alias.words.len() {
            let possessive = format!("{expected}s");
            alt((literal(possessive.as_str()), literal(expected.as_str())))
                .void()
                .parse_next(input)?;
        } else {
            literal(expected.as_str()).parse_next(input)?;
        }
    }
    eof.parse_next(input)?;
    Ok(alias.surface.clone())
}

pub(crate) fn parse_leaf_this_source_reference_surface(
    permanent_type: &str,
) -> Option<SourceReferenceSurface> {
    let permanent_type = permanent_type.trim();
    if permanent_type.is_empty() {
        return None;
    }
    let lower = permanent_type.to_ascii_lowercase();
    let noun = if parse_leaf_card_type_complete(&lower).is_ok() {
        lower
    } else {
        permanent_type.to_string()
    };
    Some(SourceReferenceSurface::ThisPermanentType(format!(
        "this {noun}"
    )))
}

pub(crate) fn parse_leaf_this_source_reference_words(
    words: &[&str],
) -> Option<SourceReferenceSurface> {
    let normalized = words.join(" ");
    let mut input = normalized.as_str();
    match parse_this_source_reference(&mut input, words) {
        Ok(surface) => Some(surface),
        Err(_) => None,
    }
}

pub(crate) fn parse_leaf_source_anaphor_words(words: &[&str]) -> Option<LeafSourceAnaphor> {
    let normalized = words.join(" ");
    let mut input = normalized.as_str();
    match alt((
        (literal("its"), eof).value(LeafSourceAnaphor::Its),
        (literal("it"), eof).value(LeafSourceAnaphor::It),
        |input: &mut &str| parse_this_source_reference(input, words).map(LeafSourceAnaphor::This),
    ))
    .parse_next(&mut input)
    {
        Ok(anaphor) => Some(anaphor),
        Err(_) => None,
    }
}

fn parse_this_source_reference(
    input: &mut &str,
    surface_words: &[&str],
) -> WResult<SourceReferenceSurface> {
    alt((literal("thiss"), literal("this")))
        .void()
        .parse_next(input)?;
    if input.is_empty() {
        return Ok(canonical_this_source_surface(surface_words));
    }

    space1.parse_next(input)?;
    let mut of_input = *input;
    if literal::<_, _, winnow::error::ContextError>("of")
        .parse_next(&mut of_input)
        .is_ok()
    {
        space1.parse_next(&mut of_input)?;
        rest.verify(|tail: &str| !tail.is_empty())
            .parse_next(&mut of_input)?;
        *input = of_input;
        return Ok(canonical_this_source_surface(surface_words));
    }

    parse_this_source_noun.parse_next(input)?;
    eof.parse_next(input)?;
    Ok(canonical_this_source_surface(surface_words))
}

fn parse_this_source_noun(input: &mut &str) -> WResult<LeafThisSourceNoun> {
    alt((
        parse_leaf_card_type.value(LeafThisSourceNoun::CardType),
        parse_leaf_subtype_flexible.value(LeafThisSourceNoun::Subtype),
        alt((
            literal("source"),
            literal("spell"),
            literal("permanent"),
            literal("card"),
            literal("creature"),
            literal("case"),
        ))
        .value(LeafThisSourceNoun::Generic),
    ))
    .parse_next(input)
}

fn canonical_this_source_surface(words: &[&str]) -> SourceReferenceSurface {
    let text = words
        .iter()
        .enumerate()
        .map(|(index, word)| canonical_this_source_word(index, word))
        .collect::<Vec<_>>()
        .join(" ");
    SourceReferenceSurface::ThisPermanentType(text)
}

fn canonical_this_source_word(index: usize, word: &str) -> String {
    if index == 0 && word == "thiss" {
        return "this".to_string();
    }

    let stripped = strip_leaf_source_possessive_suffix(word);
    if index == 1 {
        let fixed_singular = match stripped {
            "cards" => Some("card"),
            "creatures" => Some("creature"),
            "permanents" => Some("permanent"),
            "sources" => Some("source"),
            "spells" => Some("spell"),
            _ => None,
        };
        if let Some(singular) = fixed_singular {
            return singular.to_string();
        }
        if let Some(singular) = stripped.strip_suffix('s')
            && (parse_leaf_card_type_complete(singular).is_ok()
                || parse_leaf_subtype_flexible_complete(singular).is_ok())
        {
            return singular.to_string();
        }
    }
    stripped.to_string()
}

pub(crate) fn strip_leaf_source_possessive_suffix(word: &str) -> &str {
    word.strip_suffix("'s")
        .or_else(|| word.strip_suffix("’s"))
        .or_else(|| word.strip_suffix("s'"))
        .or_else(|| word.strip_suffix("s’"))
        .unwrap_or(word)
}

fn parse_source_reference_word_variants(text: &str) -> Vec<Vec<String>> {
    let parser_words = parse_reference_words(text);
    let lexed_words = lexed_reference_words(text);
    let token_words = parse_reference_token_words(text);
    let mut variants = vec![parser_words.clone()];
    if !lexed_words.is_empty() && lexed_words != parser_words {
        variants.push(lexed_words);
    }
    if token_words != parser_words {
        variants.push(token_words);
    }

    let without_articles = parser_words
        .iter()
        .filter(|word| !is_name_article(word))
        .cloned()
        .collect::<Vec<_>>();
    if !without_articles.is_empty() && !variants.iter().any(|variant| variant == &without_articles)
    {
        variants.push(without_articles);
    }
    variants
}

fn parse_reference_words(text: &str) -> Vec<String> {
    let mut input = text;
    parse_normalized_reference_words
        .parse_next(&mut input)
        .unwrap_or_default()
}

fn parse_normalized_reference_words(input: &mut &str) -> WResult<Vec<String>> {
    let mut words = Vec::new();
    while !input.is_empty() {
        take_while(0.., is_reference_word_separator).parse_next(input)?;
        if input.is_empty() {
            break;
        }
        let raw = take_while(1.., is_reference_word_character).parse_next(input)?;
        let normalized = raw
            .chars()
            .filter_map(|ch| match ch {
                '\'' | '’' | '‘' => None,
                _ if ch.is_ascii_alphanumeric() => Some(ch.to_ascii_lowercase()),
                _ => None,
            })
            .collect::<String>();
        if !normalized.is_empty() {
            words.push(normalized);
        }
    }
    Ok(words)
}

fn parse_reference_token_words(text: &str) -> Vec<String> {
    let mut input = text;
    parse_surface_token_words
        .parse_next(&mut input)
        .unwrap_or_default()
}

fn parse_surface_token_words(input: &mut &str) -> WResult<Vec<String>> {
    let mut words = Vec::new();
    while !input.is_empty() {
        take_while(0.., is_surface_token_separator).parse_next(input)?;
        if input.is_empty() {
            break;
        }
        let raw = take_while(1.., is_surface_token_character).parse_next(input)?;
        words.push(
            raw.chars()
                .map(|ch| match ch {
                    '’' | '‘' => '\'',
                    '−' => '-',
                    _ => ch.to_ascii_lowercase(),
                })
                .collect(),
        );
    }
    Ok(words)
}

fn lexed_reference_words(text: &str) -> Vec<String> {
    match lex_line(text, 0) {
        Ok(tokens) => parser_token_word_refs(&tokens)
            .into_iter()
            .map(str::to_string)
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn is_reference_word_character(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '\'' | '’' | '‘')
}

fn is_reference_word_separator(ch: char) -> bool {
    !is_reference_word_character(ch)
}

fn is_surface_token_character(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '\'' | '’' | '-')
}

fn is_surface_token_separator(ch: char) -> bool {
    !is_surface_token_character(ch)
}

fn is_name_article(word: &str) -> bool {
    matches!(word, "a" | "an" | "the")
}

fn short_name_is_distinct_name(short_name: &str) -> bool {
    let lower = short_name.to_ascii_lowercase();
    !is_name_article(&lower)
        && parse_leaf_color_complete(&lower).is_err()
        && parse_leaf_card_type_complete(&lower).is_err()
        && match parse_leaf_subtype_flexible_complete(&lower) {
            Ok(subtype) => subtype.is_planeswalker_subtype(),
            Err(_) => true,
        }
}

fn push_unique_name(names: &mut Vec<String>, raw: &str) {
    let raw = raw.trim();
    if !raw.is_empty() && !names.iter().any(|name| name == raw) {
        names.push(raw.to_string());
    }
}

fn parse_name_prefix<'a>(
    raw: &'a str,
    mut parser: impl Parser<&'a str, &'a str, ErrMode<ContextError>>,
) -> Option<&'a str> {
    let mut input = raw;
    match parser.parse_next(&mut input) {
        Ok(name) => Some(name),
        Err(_) => None,
    }
}

fn parse_front_face_name<'a>(input: &mut &'a str) -> WResult<&'a str> {
    let name = take_until(0.., " // ").parse_next(input)?;
    literal(" // ").parse_next(input)?;
    Ok(name)
}

fn parse_digital_variant_name<'a>(input: &mut &'a str) -> WResult<&'a str> {
    take_while(1..=1, |ch: char| ch.is_ascii_alphabetic()).parse_next(input)?;
    literal('-').parse_next(input)?;
    let name = rest.parse_next(input)?;
    let name = name.trim();
    if name.is_empty() {
        Err(primitives::backtrack_err(
            "digital source name",
            "letter-hyphen name prefix",
        ))
    } else {
        Ok(name)
    }
}

fn parse_leading_name_article<'a>(input: &mut &'a str) -> WResult<&'a str> {
    alt((literal("The "), literal("A "), literal("An "))).parse_next(input)?;
    let name = rest.parse_next(input)?.trim();
    if name.is_empty() {
        Err(primitives::backtrack_err(
            "source-name article",
            "name following article",
        ))
    } else {
        Ok(name)
    }
}

fn parse_comma_short_name<'a>(input: &mut &'a str) -> WResult<&'a str> {
    let name = take_until(0.., ',').parse_next(input)?;
    literal(',').parse_next(input)?;
    Ok(name)
}

fn parse_first_name_word<'a>(input: &mut &'a str) -> WResult<&'a str> {
    let name = take_until(0.., ' ').parse_next(input)?;
    literal(' ').parse_next(input)?;
    Ok(name)
}

fn strip_trailing_roman_numeral(name: &str) -> Option<&str> {
    let trimmed = name.trim();
    let (boundary, _) = trimmed
        .char_indices()
        .rev()
        .find(|(_, ch)| ch.is_whitespace())?;
    let prefix = trimmed.get(..boundary)?.trim();
    let suffix = trimmed.get(boundary..)?.trim();
    let suffix = suffix.trim_matches(|ch: char| !ch.is_ascii_alphabetic());
    if prefix.is_empty() {
        return None;
    }
    let mut input = suffix;
    if parse_roman_numeral.parse_next(&mut input).is_err() {
        return None;
    }
    Some(prefix)
}

fn parse_roman_numeral(input: &mut &str) -> WResult<LeafRomanNumeral> {
    take_while(2.., |ch: char| {
        matches!(
            ch.to_ascii_uppercase(),
            'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M'
        )
    })
    .parse_next(input)?;
    eof.parse_next(input)?;
    Ok(LeafRomanNumeral)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact(
        aliases: &[LeafSourceReferenceAlias],
        words: &[&str],
    ) -> Option<SourceReferenceSurface> {
        parse_leaf_source_reference_alias_words(aliases, words)
    }

    #[test]
    fn name_aliases_preserve_full_short_face_article_and_internal_article_surfaces() {
        let aliases = parse_leaf_source_reference_aliases_for_name("Kraven the Hunter");
        assert_eq!(
            exact(&aliases, &["kraven", "the", "hunter"]),
            Some(SourceReferenceSurface::FullName(
                "Kraven the Hunter".to_string()
            ))
        );
        assert_eq!(
            exact(&aliases, &["kraven", "hunter"]),
            Some(SourceReferenceSurface::FullName(
                "Kraven the Hunter".to_string()
            ))
        );
        assert_eq!(
            exact(&aliases, &["kraven"]),
            Some(SourceReferenceSurface::ShortName("Kraven".to_string()))
        );

        let aliases = parse_leaf_source_reference_aliases_for_name(
            "Delver of Secrets // Insectile Aberration",
        );
        assert_eq!(
            exact(&aliases, &["delver", "of", "secrets"]),
            Some(SourceReferenceSurface::FullName(
                "Delver of Secrets".to_string()
            ))
        );

        let aliases = parse_leaf_source_reference_aliases_for_name("The Gitrog Monster");
        assert_eq!(
            exact(&aliases, &["gitrog", "monster"]),
            Some(SourceReferenceSurface::FullName(
                "The Gitrog Monster".to_string()
            ))
        );
    }

    #[test]
    fn planeswalker_first_names_remain_valid_source_aliases() {
        for (name, first_name) in [
            ("Sorin of House Markov // Sorin, Ravenous Neonate", "sorin"),
            ("Jace, Vryn's Prodigy // Jace, Telepath Unbound", "jace"),
        ] {
            let aliases = parse_leaf_source_reference_aliases_for_name(name);
            assert_eq!(
                exact(&aliases, &[first_name]),
                Some(SourceReferenceSurface::ShortName(
                    first_name[..1].to_ascii_uppercase() + &first_name[1..]
                ))
            );
        }
    }

    #[test]
    fn source_alias_matching_is_case_insensitive_after_lexical_name_restoration() {
        let aliases = parse_leaf_source_reference_aliases_for_name("Ghyrson Starn, Kelermorph");
        assert_eq!(
            exact(&aliases, &["ghyrson", "Starn"]),
            Some(SourceReferenceSurface::ShortName(
                "Ghyrson Starn".to_string()
            ))
        );
    }

    #[test]
    fn color_adjectives_do_not_become_short_source_aliases() {
        for (name, color) in [
            ("Black Scarab", "black"),
            ("Blue Scarab", "blue"),
            ("Green Scarab", "green"),
            ("Red Scarab", "red"),
            ("White Scarab", "white"),
        ] {
            let aliases = parse_leaf_source_reference_aliases_for_name(name);
            assert_eq!(
                exact(&aliases, &[color]),
                None,
                "{color} must remain available to object-filter parsing: {aliases:#?}"
            );
            assert_eq!(
                exact(&aliases, &[color, "scarab"]),
                Some(SourceReferenceSurface::FullName(name.to_string()))
            );
        }
    }

    #[test]
    fn name_aliases_preserve_comma_digital_and_roman_variants() {
        let aliases = parse_leaf_source_reference_aliases_for_name("Sarulf, Realm Eater");
        assert_eq!(
            exact(&aliases, &["sarulf"]),
            Some(SourceReferenceSurface::ShortName("Sarulf".to_string()))
        );

        let aliases = parse_leaf_source_reference_aliases_for_name("A-Satoru Umezawa");
        assert_eq!(
            exact(&aliases, &["satoru", "umezawa"]),
            Some(SourceReferenceSurface::FullName(
                "A-Satoru Umezawa".to_string()
            ))
        );
        assert_eq!(
            exact(&aliases, &["satoru"]),
            Some(SourceReferenceSurface::ShortName("Satoru".to_string()))
        );

        let aliases = parse_leaf_source_reference_aliases_for_name("Ajani Vengeant II");
        assert_eq!(
            exact(&aliases, &["ajani", "vengeant"]),
            Some(SourceReferenceSurface::FullName(
                "Ajani Vengeant".to_string()
            ))
        );
    }

    #[test]
    fn alias_word_variants_preserve_parser_lexer_and_surface_tokenizations() {
        let variants = parse_source_reference_word_variants("Kraven’s the-Hunter");
        assert!(variants.contains(&vec![
            "kravens".to_string(),
            "the".to_string(),
            "hunter".to_string()
        ]));
        assert!(variants.contains(&vec!["kraven's".to_string(), "the-hunter".to_string()]));
        assert!(variants.contains(&vec!["kravens".to_string(), "hunter".to_string()]));
    }

    #[test]
    fn exact_and_possessive_alias_parsers_return_the_original_surface() {
        let aliases = parse_leaf_source_reference_aliases_for_name("Sarulf, Realm Eater");
        let short = SourceReferenceSurface::ShortName("Sarulf".to_string());
        assert_eq!(exact(&aliases, &["sarulf"]), Some(short.clone()));
        assert_eq!(exact(&aliases, &["sarulfs"]), None);
        assert_eq!(
            parse_leaf_source_reference_possessive_alias_words(&aliases, &["sarulfs"]),
            Some(short)
        );
        assert_eq!(
            parse_leaf_source_reference_possessive_alias_words(
                &aliases,
                &["sarulf", "realm", "eaters"]
            ),
            Some(SourceReferenceSurface::FullName(
                "Sarulf, Realm Eater".to_string()
            ))
        );
    }

    #[test]
    fn this_source_parser_preserves_canonical_surface_rules() {
        for (words, expected) in [
            (&["this"][..], "this"),
            (&["thiss"][..], "this"),
            (&["this", "creatures"][..], "this creature"),
            (&["this", "goblins"][..], "this goblin"),
            (&["this", "of", "those", "cards"][..], "this of those cards"),
        ] {
            assert_eq!(
                parse_leaf_this_source_reference_words(words),
                Some(SourceReferenceSurface::ThisPermanentType(
                    expected.to_string()
                ))
            );
        }
        assert_eq!(parse_leaf_this_source_reference_words(&[]), None);
        assert_eq!(
            parse_leaf_this_source_reference_surface("Creature"),
            Some(SourceReferenceSurface::ThisPermanentType(
                "this creature".to_string()
            ))
        );
        assert_eq!(
            parse_leaf_this_source_reference_surface("Goblin"),
            Some(SourceReferenceSurface::ThisPermanentType(
                "this Goblin".to_string()
            ))
        );
    }
}
