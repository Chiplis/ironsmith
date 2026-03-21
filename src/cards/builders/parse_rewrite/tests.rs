use crate::cards::builders::{CardDefinitionBuilder, CardTextError};
use crate::ids::CardId;
use crate::mana::ManaSymbol;
use crate::object::CounterType;
use crate::types::{CardType, Subtype, Supertype};

use super::{
    lex_line, lower_activation_cost_cst, parse_activation_cost_rewrite, parse_count_word_rewrite,
    parse_mana_symbol_group_rewrite, parse_text_with_annotations_rewrite_lowered,
    parse_type_line_rewrite,
};

#[test]
fn rewrite_lexer_tracks_spans_for_activation_lines() {
    let tokens = lex_line("{T}, Sacrifice a creature: Add {B}{B}.", 3)
        .expect("rewrite lexer should classify activation line");
    assert_eq!(tokens[0].slice, "{T}");
    assert_eq!(tokens[0].span.line, 3);
    assert_eq!(tokens[0].span.start, 0);
    assert_eq!(tokens[0].span.end, 3);
    assert!(tokens.iter().any(|token| token.slice == ":"));
}

#[test]
fn rewrite_lexer_accepts_plus_prefixed_counter_words() {
    let tokens = lex_line("Put a +1/+1 counter on target creature.", 0)
        .expect("rewrite lexer should accept +1/+1 words");
    assert!(tokens.iter().any(|token| token.slice == "+1/+1"));
}

#[test]
fn rewrite_count_word_parser_handles_digits_and_words() {
    assert_eq!(parse_count_word_rewrite("2").expect("digit count"), 2);
    assert_eq!(parse_count_word_rewrite("three").expect("word count"), 3);
}

#[test]
fn rewrite_mana_symbol_group_parser_handles_hybrid_symbols() {
    let symbols = parse_mana_symbol_group_rewrite("{W/U}")
        .expect("rewrite parser should parse hybrid mana group");
    assert_eq!(symbols, vec![ManaSymbol::White, ManaSymbol::Blue]);
}

#[test]
fn rewrite_type_line_parser_handles_supertypes_types_and_subtypes() {
    let parsed = parse_type_line_rewrite("Legendary Creature — Elf Druid")
        .expect("rewrite type-line parser should succeed");
    assert_eq!(parsed.supertypes, vec![Supertype::Legendary]);
    assert_eq!(parsed.card_types, vec![CardType::Creature]);
    assert_eq!(parsed.subtypes, vec![Subtype::Elf, Subtype::Druid]);
}

#[test]
fn rewrite_activation_cost_parses_sacrifice_segments() {
    let cst = parse_activation_cost_rewrite("Sacrifice a creature")
        .expect("rewrite activation-cost parser should parse sacrifice segments");
    let lowered = lower_activation_cost_cst(&cst)
        .expect("rewrite sacrifice segment should lower to TotalCost");
    assert!(!lowered.is_free());

    let another = parse_activation_cost_rewrite("Sacrifice another creature")
        .expect("rewrite activation-cost parser should preserve 'another creature'");
    let rendered = another
        .segments
        .iter()
        .map(|segment| format!("{segment:?}"))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        rendered.contains("other: true"),
        "expected rewrite sacrifice CST to preserve 'another', got {rendered}"
    );
}

#[test]
fn rewrite_activation_cost_parses_energy_and_counter_variants() {
    let energy = parse_activation_cost_rewrite("Pay {E}{E}")
        .expect("rewrite activation-cost parser should parse energy payment");
    let counter_add = parse_activation_cost_rewrite("Put a +1/+1 counter on this creature")
        .expect("rewrite parser should parse add-counter cost");
    let counter_remove = parse_activation_cost_rewrite("Remove a +1/+1 counter from this creature")
        .expect("rewrite parser should parse remove-counter cost");
    let exile_hand = parse_activation_cost_rewrite("Exile a blue card from your hand")
        .expect("rewrite parser should parse exile-from-hand cost");

    assert!(matches!(
        energy.segments.as_slice(),
        [super::ActivationCostSegmentCst::Energy(2)]
    ));
    assert!(matches!(
        counter_add.segments.as_slice(),
        [super::ActivationCostSegmentCst::PutCounters {
            counter_type: CounterType::PlusOnePlusOne,
            count: 1
        }]
    ));
    assert!(matches!(
        counter_remove.segments.as_slice(),
        [super::ActivationCostSegmentCst::RemoveCounters {
            counter_type: CounterType::PlusOnePlusOne,
            count: 1
        }]
    ));
    assert!(matches!(
        exile_hand.segments.as_slice(),
        [super::ActivationCostSegmentCst::ExileFromHand {
            count: 1,
            color_filter: Some(colors)
        }] if *colors == crate::color::ColorSet::BLUE
    ));
}

#[test]
fn rewrite_lowered_simple_card_parses() -> Result<(), CardTextError> {
    let text = "Type: Creature — Spirit\n{1}: This creature gets +1/+1 until end of turn.";
    let builder = CardDefinitionBuilder::new(CardId::new(), "Shared Spirit");
    let (definition, _) =
        parse_text_with_annotations_rewrite_lowered(builder, text.to_string(), false)?;
    assert_eq!(definition.abilities.len(), 1);
    Ok(())
}
