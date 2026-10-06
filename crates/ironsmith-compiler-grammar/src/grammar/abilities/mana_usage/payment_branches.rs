//! Disjoint payment actions retain independent transaction predicates.
use super::*;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Option<ManaUsageRestriction> {
    let mut words = Vec::new();
    for token in tokens {
        if token.is_comma() {
            words.push(",");
        } else {
            words.extend(crate::lexer::parser_token_word_refs(std::slice::from_ref(
                token,
            )));
        }
    }
    if !matches!(
        words.get(..5)?,
        ["spend", "this" | "that", "mana", "only", "to"]
    ) {
        return None;
    }
    let mut starts = vec![5];
    let mut ends = Vec::new();
    for i in 5..words.len() {
        if words[i] != "or" {
            continue;
        }
        let next = i + 1 + usize::from(words.get(i + 1) == Some(&"to"));
        if words
            .get(next)
            .is_some_and(|w| matches!(*w, "cast" | "activate" | "pay" | "turn" | "foretell"))
        {
            ends.push(i);
            starts.push(next);
        }
    }
    // A comma before a new action also separates a transaction arm.
    for i in 5..words.len() {
        if words[i] != "," {
            continue;
        }
        let next = i + 1;
        if words
            .get(next)
            .is_some_and(|w| matches!(*w, "cast" | "activate" | "pay" | "turn" | "foretell"))
        {
            ends.push(i);
            starts.push(next);
        }
    }
    starts.sort_unstable();
    ends.sort_unstable();
    ends.push(words.len());
    if starts.len() != ends.len() {
        return None;
    }
    let mut predicates = Vec::new();
    let mut extended_action = false;
    for (start, end) in starts.into_iter().zip(ends) {
        let mut arm = &words[start..end];
        while arm.last() == Some(&",") {
            arm = &arm[..arm.len() - 1];
        }
        let (predicate, extended) = parse_arm(arm)?;
        predicates.push(predicate);
        extended_action |= extended;
    }
    // Preserve existing legacy shapes. Only these additional action domains
    // belong to this reader; a trailing unknown arm rejects the whole shape.
    if !extended_action {
        return None;
    }
    Some(ManaUsageRestriction::PaymentTransaction {
        restriction: Some(if predicates.len() == 1 {
            predicates.remove(0)
        } else {
            ManaPaymentPredicate::AnyOf(predicates)
        }),
        on_spend: Vec::new(),
    })
}

fn parse_arm(words: &[&str]) -> Option<(ManaPaymentPredicate, bool)> {
    use crate::mana::ManaSymbol;
    let purpose = |p| ManaPaymentPredicate::Purpose(p);
    match words {
        ["activate", "an", "equip", "ability"] | ["activate", "equip", "abilities"] => Some((
            ManaPaymentPredicate::ActivatedAbilityKeyword(ironsmith_core::ActivatedAbilityKeyword::Equip), true,
        )),
        ["activate", "power-up", "abilities"] | ["activate", "power", "up", "abilities"] => Some((
            ManaPaymentPredicate::ActivatedAbilityKeyword(ironsmith_core::ActivatedAbilityKeyword::PowerUp), true,
        )),
        ["cast", "an", "equipment", "spell"] | ["cast", "equipment", "spells"] => Some((
            cast_spell_payment_predicate(ObjectFilter::default().with_subtype(crate::types::Subtype::Equipment)), false,
        )),
        ["pay", "a", "disturb", "cost"] => Some((ManaPaymentPredicate::DisturbCost, true)),
        ["foretell", "a", "card", "from", "your", "hand"] | ["foretell", "cards"] =>
            Some((purpose(ManaPaymentPurpose::Foretell), true)),
        ["cast", "spells", "that", "have", "foretell"] => Some((
            cast_spell_payment_predicate(ObjectFilter::default()
                .with_alternative_cast(crate::filter::AlternativeCastKind::Foretell)), true,
        )),
        ["cast", "an", "instant", "or", "sorcery", "spell"] => {
            let mut filter = ObjectFilter::default();
            filter.card_types = vec![CardType::Instant, CardType::Sorcery];
            Some((cast_spell_payment_predicate(filter), false))
        }
        ["cast", "a", "face", "down", "creature", "spell"]
        | ["cast", "a", "face-down", "creature", "spell"] => Some((
            cast_spell_payment_predicate(ObjectFilter::default().face_down().with_type(CardType::Creature)), false,
        )),
        ["pay", "a", "mana", "cost", "to", "turn", "a", "manifested", "creature", "face", "up"] => Some((
            ManaPaymentPredicate::All(vec![
                ManaPaymentPredicate::TurnFaceUpMethod(ironsmith_core::ManaTurnFaceUpMethod::PrintedManaCost),
                ManaPaymentPredicate::SourceManifested,
            ]), true,
        )),
        ["pay", "a", "morph", "cost"] => Some((
            ManaPaymentPredicate::AnyOf(vec![
                ManaPaymentPredicate::TurnFaceUpMethod(ironsmith_core::ManaTurnFaceUpMethod::Morph),
                ManaPaymentPredicate::TurnFaceUpMethod(ironsmith_core::ManaTurnFaceUpMethod::Megamorph),
            ]), true,
        )),
        ["cast", "a", "colorless", "spell"] => Some((
            cast_spell_payment_predicate(ObjectFilter::default().colorless()),
            false,
        )),
        ["cast", "face", "down", "spells"] | ["cast", "face-down", "spells"] => Some((
            cast_spell_payment_predicate(ObjectFilter::default().face_down()),
            false,
        )),
        [
            "activate",
            "an",
            "ability",
            "of",
            "a",
            "colorless",
            "permanent",
        ] => Some((
            ManaPaymentPredicate::All(vec![
                ManaPaymentPredicate::AnyOf(any_ability_payment_predicates().to_vec()),
                ManaPaymentPredicate::SourceMatches(
                    ObjectFilter::default()
                        .colorless()
                        .in_zone(Zone::Battlefield),
                ),
            ]),
            false,
        )),
        ["pay", "a", "cost", "that", "contains", "c"] => Some((
            ManaPaymentPredicate::CostContains(ManaSymbol::Colorless),
            true,
        )),
        ["turn", "permanents", "face", "up"] | ["turn", "a", "permanent", "face", "up"] => Some((
            ManaPaymentPredicate::All(vec![
                purpose(ManaPaymentPurpose::TurnFaceUp),
                ManaPaymentPredicate::SourceMatches(
                    ObjectFilter::default()
                        .in_zone(Zone::Battlefield)
                        .face_down(),
                ),
            ]),
            true,
        )),
        ["turn", "creatures", "face", "up"] | ["turn", "a", "creature", "face", "up"] => Some((
            ManaPaymentPredicate::All(vec![
                purpose(ManaPaymentPurpose::TurnFaceUp),
                ManaPaymentPredicate::SourceMatches(
                    ObjectFilter::default()
                        .in_zone(Zone::Battlefield)
                        .face_down()
                        .with_type(CardType::Creature),
                ),
            ]),
            true,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_payment_alternatives_keep_disjoint_purposes() {
        for (text, branches) in [
            (
                "Spend this mana only to cast a colorless spell, activate an ability of a colorless permanent, or pay a cost that contains {C}.",
                3,
            ),
            (
                "Spend this mana only to cast face-down spells or to turn creatures face up.",
                2,
            ),
            ("Spend this mana only to turn permanents face up.", 1),
            ("Spend this mana only to cast an Equipment spell or activate an equip ability.", 2),
            ("Spend this mana only to activate power-up abilities.", 1),
            ("Spend this mana only to foretell a card from your hand or cast an instant or sorcery spell.", 2),
            ("Spend this mana only to cast a face-down creature spell, pay a mana cost to turn a manifested creature face up, or pay a morph cost.", 3),
            ("Spend this mana only to pay a disturb cost or cast an instant or sorcery spell.", 2),
            ("Spend this mana only to foretell cards or cast spells that have foretell.", 2),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let Some(ManaUsageRestriction::PaymentTransaction {
                restriction: Some(predicate),
                on_spend,
            }) = parse(&tokens)
            else {
                panic!("{text}");
            };
            assert!(on_spend.is_empty());
            let count = match predicate {
                ManaPaymentPredicate::AnyOf(xs) => xs.len(),
                _ => 1,
            };
            assert_eq!(count, branches, "{text}");
        }
        for text in [
            "Spend this mana only to turn permanents face up or draw a card.",
            "Spend this mana only to foretell cards or dance.",
            "Spend this mana only to pay a morph cost and draw a card.",
            "Spend this mana only to foretell cards or cast spells that were foretold.",
            "Spend this mana only to turn permanents face up and cast spells.",
            "Spend this mana only to cast a colorless spell or pay a cost that contains {C} with no other requirements.",
        ] {
            assert!(
                parse(&crate::lexer::lex_line(text, 0).unwrap()).is_none(),
                "{text}"
            );
        }
    }
}
