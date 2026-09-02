use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::{CardDefinition, CardDefinitionBuilder};
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::mana::{ManaCost, ManaSymbol};
use ironsmith_compiler::static_abilities::CompanionDeckCardFacts;
use ironsmith_compiler::static_abilities::{CompanionDeckCondition, StaticAbilityPayload};
use ironsmith_compiler::types::{CardType, Subtype};

fn compile_companion(text: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Companion Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .unwrap_or_else(|error| panic!("companion line should compile: {error}"))
}

fn condition(definition: &CardDefinition) -> &CompanionDeckCondition {
    let AbilityKind::Static(ability) = &definition.abilities[0].kind else {
        panic!(
            "expected a static companion ability: {:#?}",
            definition.abilities
        );
    };
    let StaticAbilityPayload::Companion(condition) = &ability.payload else {
        panic!("companion must not lower to fallback text: {ability:#?}");
    };
    condition
}

#[test]
fn every_printed_companion_condition_lowers_to_a_typed_deck_predicate() {
    let cases = [
        (
            "Companion — Your starting deck contains only cards with even mana values.",
            CompanionDeckCondition::OnlyManaValueParity {
                even: true,
                lands_are_exempt: false,
            },
        ),
        (
            "Companion — No card in your starting deck has more than one of the same mana symbol in its mana cost.",
            CompanionDeckCondition::NoRepeatedManaSymbols,
        ),
        (
            "Companion — Each creature card in your starting deck is a Cat, Elemental, Nightmare, Dinosaur, or Beast card.",
            CompanionDeckCondition::CreatureSubtypes(vec![
                Subtype::Cat,
                Subtype::Elemental,
                Subtype::Nightmare,
                Subtype::Dinosaur,
                Subtype::Beast,
            ]),
        ),
        (
            "Companion — Your starting deck contains only cards with mana value 3 or greater and land cards.",
            CompanionDeckCondition::NonlandManaValueAtLeast(3),
        ),
        (
            "Companion — Each permanent card in your starting deck has mana value 2 or less.",
            CompanionDeckCondition::PermanentManaValueAtMost(2),
        ),
        (
            "Companion — Each nonland card in your starting deck has a different name.",
            CompanionDeckCondition::UniqueNonlandNames,
        ),
        (
            "Companion — Your starting deck contains only cards with odd mana values and land cards.",
            CompanionDeckCondition::OnlyManaValueParity {
                even: false,
                lands_are_exempt: true,
            },
        ),
        (
            "Companion — Each nonland card in your starting deck shares a card type.",
            CompanionDeckCondition::SharedNonlandCardType,
        ),
        (
            "Companion — Your starting deck contains at least twenty cards more than the minimum deck size.",
            CompanionDeckCondition::CardsAboveMinimumDeckSize(20),
        ),
        (
            "Companion — Each permanent card in your starting deck has an activated ability.",
            CompanionDeckCondition::PermanentsHaveActivatedAbility,
        ),
    ];

    for (text, expected) in cases {
        let definition = compile_companion(text);
        assert_eq!(condition(&definition), &expected, "{text}");
        let AbilityKind::Static(ability) = &definition.abilities[0].kind else {
            unreachable!("condition() already proved this is static");
        };
        let (_, condition_text) = text.split_once('—').expect("Companion has an em dash");
        assert!(
            ability.label.eq_ignore_ascii_case(condition_text.trim()),
            "{text}: {:?}",
            ability.label
        );
    }
}

#[test]
fn unknown_companion_condition_remains_non_executable_instead_of_being_guessed() {
    let definition = compile_companion("Companion — Your deck is mysterious.");
    let AbilityKind::Static(ability) = &definition.abilities[0].kind else {
        panic!("expected static fallback");
    };
    assert!(
        matches!(ability.payload, StaticAbilityPayload::None),
        "unknown future conditions must not silently acquire semantics: {ability:#?}"
    );
}

fn deck_card(
    name: &str,
    mana_cost: Option<ManaCost>,
    card_types: Vec<CardType>,
    subtypes: Vec<Subtype>,
    has_all_creature_types: bool,
    has_activated_ability: bool,
) -> CompanionDeckCardFacts {
    CompanionDeckCardFacts {
        name: name.to_string(),
        mana_cost,
        card_types,
        subtypes,
        has_all_creature_types,
        has_activated_ability,
    }
}

fn simple_card(name: &str, mana_value: u8, card_type: CardType) -> CompanionDeckCardFacts {
    deck_card(
        name,
        Some(ManaCost::from_symbols(vec![ManaSymbol::Generic(
            mana_value,
        )])),
        vec![card_type],
        Vec::new(),
        false,
        false,
    )
}

#[test]
fn every_companion_deck_predicate_accepts_and_rejects_its_meaningful_boundary() {
    let even = simple_card("Even", 2, CardType::Creature);
    let odd = simple_card("Odd", 3, CardType::Creature);
    assert!(
        CompanionDeckCondition::OnlyManaValueParity {
            even: true,
            lands_are_exempt: false,
        }
        .is_fulfilled_by(std::slice::from_ref(&even), 1)
    );
    assert!(
        !CompanionDeckCondition::OnlyManaValueParity {
            even: true,
            lands_are_exempt: false,
        }
        .is_fulfilled_by(std::slice::from_ref(&odd), 1)
    );

    let distinct_hybrids = deck_card(
        "Distinct hybrid symbols",
        Some(ManaCost::from_pips(vec![
            vec![ManaSymbol::White, ManaSymbol::Blue],
            vec![ManaSymbol::White, ManaSymbol::Black],
        ])),
        vec![CardType::Creature],
        Vec::new(),
        false,
        false,
    );
    let repeated_symbol = deck_card(
        "Repeated symbol",
        Some(ManaCost::from_symbols(vec![
            ManaSymbol::White,
            ManaSymbol::White,
        ])),
        vec![CardType::Creature],
        Vec::new(),
        false,
        false,
    );
    assert!(CompanionDeckCondition::NoRepeatedManaSymbols.is_fulfilled_by(&[distinct_hybrids], 1));
    assert!(!CompanionDeckCondition::NoRepeatedManaSymbols.is_fulfilled_by(&[repeated_symbol], 1));

    let cat = deck_card(
        "Cat",
        None,
        vec![CardType::Creature],
        vec![Subtype::Cat],
        false,
        false,
    );
    let changeling = deck_card(
        "Changeling",
        None,
        vec![CardType::Creature],
        Vec::new(),
        true,
        false,
    );
    let elf = deck_card(
        "Elf",
        None,
        vec![CardType::Creature],
        vec![Subtype::Elf],
        false,
        false,
    );
    let kaheera = CompanionDeckCondition::CreatureSubtypes(vec![Subtype::Cat]);
    assert!(kaheera.is_fulfilled_by(&[cat, changeling], 2));
    assert!(!kaheera.is_fulfilled_by(&[elf], 1));

    let land = simple_card("Land", 0, CardType::Land);
    let high_nonland = simple_card("High", 3, CardType::Instant);
    assert!(
        CompanionDeckCondition::NonlandManaValueAtLeast(3)
            .is_fulfilled_by(&[land.clone(), high_nonland.clone()], 2)
    );
    assert!(
        !CompanionDeckCondition::NonlandManaValueAtLeast(3)
            .is_fulfilled_by(std::slice::from_ref(&even), 1)
    );

    assert!(
        CompanionDeckCondition::PermanentManaValueAtMost(2)
            .is_fulfilled_by(&[even.clone(), high_nonland], 2)
    );
    assert!(
        !CompanionDeckCondition::PermanentManaValueAtMost(2)
            .is_fulfilled_by(std::slice::from_ref(&odd), 1)
    );

    assert!(
        CompanionDeckCondition::UniqueNonlandNames
            .is_fulfilled_by(&[land.clone(), land.clone(), even.clone()], 3)
    );
    assert!(
        !CompanionDeckCondition::UniqueNonlandNames
            .is_fulfilled_by(&[even.clone(), even.clone()], 2)
    );

    assert!(
        CompanionDeckCondition::OnlyManaValueParity {
            even: false,
            lands_are_exempt: true,
        }
        .is_fulfilled_by(&[odd.clone(), land], 2)
    );
    assert!(
        !CompanionDeckCondition::OnlyManaValueParity {
            even: false,
            lands_are_exempt: true,
        }
        .is_fulfilled_by(std::slice::from_ref(&even), 1)
    );

    let artifact_creature = deck_card(
        "Artifact creature",
        None,
        vec![CardType::Artifact, CardType::Creature],
        Vec::new(),
        false,
        false,
    );
    assert!(
        CompanionDeckCondition::SharedNonlandCardType
            .is_fulfilled_by(&[even.clone(), artifact_creature], 2)
    );
    assert!(
        !CompanionDeckCondition::SharedNonlandCardType.is_fulfilled_by(
            &[even.clone(), simple_card("Spell", 1, CardType::Instant)],
            2
        )
    );

    assert!(
        CompanionDeckCondition::CardsAboveMinimumDeckSize(2).is_fulfilled_by(
            &[
                even.clone(),
                odd.clone(),
                simple_card("Third", 1, CardType::Instant)
            ],
            1
        )
    );
    assert!(
        !CompanionDeckCondition::CardsAboveMinimumDeckSize(2)
            .is_fulfilled_by(&[even.clone(), odd], 1)
    );

    let activated_permanent = deck_card(
        "Activated permanent",
        None,
        vec![CardType::Artifact],
        Vec::new(),
        false,
        true,
    );
    assert!(
        CompanionDeckCondition::PermanentsHaveActivatedAbility.is_fulfilled_by(
            &[
                activated_permanent,
                simple_card("Spell", 1, CardType::Instant)
            ],
            2
        )
    );
    assert!(!CompanionDeckCondition::PermanentsHaveActivatedAbility.is_fulfilled_by(&[even], 1));
}
