use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::CardTextError;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype};

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::filters::parse_object_filter_with_grammar_entrypoint_lexed;
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactPermissionSubject {
    GenericSpell,
    GenericSpells,
    PermanentSpell,
    PermanentSpells,
    NoncreatureSpells,
    YourCommander,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellSubjectFacts {
    pub contains_spell: bool,
    pub contains_singular_spell: bool,
    pub contains_plural_spells: bool,
    pub starts_with_generic_spell: bool,
    pub exact: Option<ExactPermissionSubject>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListSeparator {
    Comma,
    And,
    Or,
}

pub fn parse_spell_subject_facts(tokens: &[OwnedLexToken]) -> SpellSubjectFacts {
    let contains_singular_spell = primitives::find_prefix(tokens, || {
        primitives::kw("spell").value(ExactPermissionSubject::GenericSpell)
    })
    .is_some();
    let contains_plural_spells = primitives::find_prefix(tokens, || {
        primitives::kw("spells").value(ExactPermissionSubject::GenericSpells)
    })
    .is_some();
    SpellSubjectFacts {
        contains_spell: contains_singular_spell || contains_plural_spells,
        contains_singular_spell,
        contains_plural_spells,
        starts_with_generic_spell: primitives::parse_prefix(tokens, generic_spell_head).is_some(),
        exact: parse_exact_permission_subject(tokens),
    }
}

pub fn parse_exact_permission_subject(tokens: &[OwnedLexToken]) -> Option<ExactPermissionSubject> {
    primitives::parse_all(tokens, exact_permission_subject, "exact permission subject").ok()
}

pub fn parse_permission_subject_filter_tokens(
    filter_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    if parse_aura_enchant_creature_subject(filter_tokens).is_some() {
        return Ok(Some(
            ObjectFilter::default()
                .with_subtype(Subtype::Aura)
                .with_ability_marker("enchant creature"),
        ));
    }
    if matches!(
        parse_exact_permission_subject(filter_tokens),
        Some(ExactPermissionSubject::NoncreatureSpells)
    ) {
        return Ok(Some(ObjectFilter::noncreature_spell()));
    }
    if matches!(
        parse_exact_permission_subject(filter_tokens),
        Some(ExactPermissionSubject::PermanentSpell | ExactPermissionSubject::PermanentSpells)
    ) {
        return Ok(Some(permanent_spell_filter()));
    }
    if let Some(filter) = parse_simple_spell_type_list_filter_tokens(filter_tokens) {
        return Ok(Some(filter));
    }
    if let Some(filter) = parse_binary_permission_subject_filter_tokens(filter_tokens)? {
        return Ok(Some(filter));
    }

    if let Ok(mut filter) = parse_object_filter_with_grammar_entrypoint_lexed(filter_tokens, false)
    {
        if filter.all_card_types.is_empty()
            && filter.card_types.len() > 1
            && parse_subject_separator_fact(filter_tokens).is_none()
        {
            filter.all_card_types = std::mem::take(&mut filter.card_types);
        }
        return Ok(Some(normalize_permission_subject_filter(filter)));
    }

    Ok(None)
}

pub fn parse_cast_permission_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let spell_subject = parse_spell_subject_facts(tokens);
    if matches!(
        spell_subject.exact,
        Some(ExactPermissionSubject::GenericSpell | ExactPermissionSubject::GenericSpells)
    ) {
        return Ok(Some(ObjectFilter::default()));
    }
    if let Some(filter) = parse_simple_spell_type_list_filter_tokens(tokens) {
        return Ok(Some(filter));
    }
    parse_permission_subject_filter_tokens(tokens)
}

pub fn parse_simple_spell_type_list_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let card_types =
        primitives::parse_all(tokens, simple_spell_type_list, "permission spell type list").ok()?;
    Some(ObjectFilter {
        card_types,
        ..ObjectFilter::default()
    })
}

pub fn generic_spell_subject_requires_nonland(tokens: &[OwnedLexToken]) -> bool {
    parse_spell_subject_facts(tokens).starts_with_generic_spell
}

pub fn permanent_spell_filter() -> ObjectFilter {
    ObjectFilter {
        card_types: vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Planeswalker,
            CardType::Battle,
        ],
        ..ObjectFilter::default()
    }
}

fn normalize_permission_subject_filter(mut filter: ObjectFilter) -> ObjectFilter {
    filter.zone = None;
    filter.stack_kind = None;
    filter.has_mana_cost = false;
    filter
}

fn parse_binary_permission_subject_filter_tokens(
    filter_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some((separator_token, _, right_tokens)) = primitives::find_prefix(filter_tokens, || {
        alt((primitives::kw("and"), primitives::kw("or"))).void()
    }) else {
        return Ok(None);
    };
    let left_tokens = trim_lexed_commas(&filter_tokens[..separator_token]);
    let right_tokens = trim_lexed_commas(right_tokens);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return Ok(None);
    }
    let Ok(left) = parse_object_filter_with_grammar_entrypoint_lexed(left_tokens, false) else {
        return Ok(None);
    };
    let Ok(right) = parse_object_filter_with_grammar_entrypoint_lexed(right_tokens, false) else {
        return Ok(None);
    };
    Ok(Some(ObjectFilter {
        any_of: vec![
            normalize_permission_subject_filter(left),
            normalize_permission_subject_filter(right),
        ],
        ..ObjectFilter::default()
    }))
}

fn parse_subject_separator_fact(tokens: &[OwnedLexToken]) -> Option<ListSeparator> {
    primitives::find_prefix(tokens, || {
        alt((
            primitives::comma().value(ListSeparator::Comma),
            primitives::kw("and").value(ListSeparator::And),
            primitives::kw("or").value(ListSeparator::Or),
        ))
    })
    .map(|(_, separator, _)| separator)
}

fn exact_permission_subject(input: &mut LexStream<'_>) -> WResult<ExactPermissionSubject> {
    alt((
        primitives::phrase(&["your", "commander"]).value(ExactPermissionSubject::YourCommander),
        primitives::phrase(&["noncreature", "spells"])
            .value(ExactPermissionSubject::NoncreatureSpells),
        (
            opt(article),
            primitives::kw("permanent"),
            primitives::kw("spells"),
        )
            .value(ExactPermissionSubject::PermanentSpells),
        (
            opt(article),
            primitives::kw("permanent"),
            primitives::kw("spell"),
        )
            .value(ExactPermissionSubject::PermanentSpell),
        (opt(article), primitives::kw("spells")).value(ExactPermissionSubject::GenericSpells),
        (opt(article), primitives::kw("spell")).value(ExactPermissionSubject::GenericSpell),
    ))
    .parse_next(input)
}

fn generic_spell_head(input: &mut LexStream<'_>) -> WResult<()> {
    opt(article).parse_next(input)?;
    alt((primitives::kw("spell"), primitives::kw("spells")))
        .void()
        .parse_next(input)
}

fn article<'a>(input: &mut LexStream<'a>) -> WResult<&'a OwnedLexToken> {
    alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("the"),
    ))
    .parse_next(input)
}

fn parse_aura_enchant_creature_subject(tokens: &[OwnedLexToken]) -> Option<()> {
    primitives::parse_all(
        tokens,
        (
            primitives::kw("aura"),
            alt((primitives::kw("spells"), primitives::kw("cards"))),
            primitives::phrase(&["with", "enchant", "creature"]),
        )
            .void(),
        "aura enchant-creature permission subject",
    )
    .ok()
}

fn simple_spell_type_list(input: &mut LexStream<'_>) -> WResult<Vec<CardType>> {
    opt(article).parse_next(input)?;
    let first = card_type.parse_next(input)?;
    let mut card_types = vec![first];
    let mut saw_separator = false;
    let mut saw_or = false;

    loop {
        let checkpoint = input.checkpoint();
        let separator = match alt((
            primitives::comma().value(ListSeparator::Comma),
            primitives::kw("and").value(ListSeparator::And),
            primitives::kw("or").value(ListSeparator::Or),
        ))
        .parse_next(input)
        {
            Ok(separator) => separator,
            Err(_) => {
                input.reset(&checkpoint);
                break;
            }
        };
        saw_separator = true;
        if separator == ListSeparator::Or {
            saw_or = true;
        }
        if separator == ListSeparator::Comma {
            let conjunction_checkpoint = input.checkpoint();
            match alt((
                primitives::kw("and").value(ListSeparator::And),
                primitives::kw("or").value(ListSeparator::Or),
            ))
            .parse_next(input)
            {
                Ok(ListSeparator::Or) => saw_or = true,
                Ok(_) => {}
                Err(_) => input.reset(&conjunction_checkpoint),
            }
        }
        let parsed_type = card_type.parse_next(input)?;
        if !card_types.contains(&parsed_type) {
            card_types.push(parsed_type);
        }
    }

    opt(alt((primitives::kw("spell"), primitives::kw("spells")))).parse_next(input)?;
    eof.parse_next(input)?;
    if !saw_separator || !saw_or || card_types.is_empty() {
        return Err(primitives::backtrack_err(
            "permission type list",
            "card type disjunction",
        ));
    }
    Ok(card_types)
}

fn card_type(input: &mut LexStream<'_>) -> WResult<CardType> {
    alt((
        primitives::kw("artifact").value(CardType::Artifact),
        primitives::kw("battle").value(CardType::Battle),
        primitives::kw("creature").value(CardType::Creature),
        primitives::kw("enchantment").value(CardType::Enchantment),
        primitives::kw("instant").value(CardType::Instant),
        primitives::kw("land").value(CardType::Land),
        primitives::kw("planeswalker").value(CardType::Planeswalker),
        primitives::kw("sorcery").value(CardType::Sorcery),
    ))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("permission subject fixture should lex")
    }

    #[test]
    fn parses_typed_spell_subject_facts() {
        let singular = parse_spell_subject_facts(&lex("a spell you own"));
        assert!(singular.contains_singular_spell);
        assert!(!singular.contains_plural_spells);
        assert!(singular.starts_with_generic_spell);

        assert_eq!(
            parse_exact_permission_subject(&lex("permanent spells")),
            Some(ExactPermissionSubject::PermanentSpells)
        );
    }

    #[test]
    fn parses_type_lists_and_binary_subjects() {
        let types = parse_simple_spell_type_list_filter_tokens(&lex(
            "artifact, creature, or enchantment spells",
        ))
        .expect("type list");
        assert_eq!(
            types.card_types,
            vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment
            ]
        );

        let binary =
            parse_permission_subject_filter_tokens(&lex("artifact cards or creature cards"))
                .expect("filter parse")
                .expect("binary filter");
        assert_eq!(binary.any_of.len(), 2);
    }

    #[test]
    fn parses_special_permission_subject_filters() {
        let aura =
            parse_permission_subject_filter_tokens(&lex("Aura spells with enchant creature"))
                .expect("filter parse")
                .expect("aura filter");
        assert!(aura.subtypes.contains(&Subtype::Aura));
        assert_eq!(aura.ability_markers, ["enchant creature"]);

        let permanent = parse_permission_subject_filter_tokens(&lex("permanent spells"))
            .expect("filter parse")
            .expect("permanent filter");
        assert!(permanent.card_types.contains(&CardType::Creature));
        assert!(permanent.card_types.contains(&CardType::Planeswalker));

        let noncreature = parse_cast_permission_filter_tokens(&lex("noncreature spells"))
            .expect("filter parse")
            .expect("noncreature-spell filter");
        assert!(
            noncreature
                .excluded_card_types
                .contains(&CardType::Creature)
        );
        assert!(noncreature.excluded_card_types.contains(&CardType::Land));
    }

    #[test]
    fn qualified_generic_spell_subjects_keep_keyword_constraints() {
        let cycling =
            parse_cast_permission_filter_tokens(&lex("spells that have a cycling ability"))
                .expect("qualified spell subject should not hard-error")
                .expect("qualified spell subject should produce a filter");
        assert_eq!(cycling.ability_markers, vec!["cycling".to_string()]);

        let generic = parse_cast_permission_filter_tokens(&lex("spells"))
            .expect("generic spell subject should not hard-error")
            .expect("generic spell subject should produce a filter");
        assert_eq!(generic, ObjectFilter::default());
    }
}
