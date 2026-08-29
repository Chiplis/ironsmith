use super::*;
use crate::lexer::lex_line;

fn tokens(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).unwrap()
}

#[test]
fn parses_dynamic_counts_as_typed_facts() {
    assert_eq!(
        parse_dynamic_counter_count_shape(&tokens("two life lost this way")),
        Some(DynamicCounterCountShape::LifeLostThisWay { group_size: 2 })
    );
    assert_eq!(
        parse_dynamic_counter_count_shape(&tokens("spells you cast this turn")),
        Some(DynamicCounterCountShape::SpellsCastThisTurn {
            player: PlayerFilter::You,
            other_than_first: false,
        })
    );
}

#[test]
fn parses_target_counts_and_half_life() {
    let target_tokens = tokens("each of up to X target creatures");
    assert_eq!(
        parse_counter_target_count_shape(&target_tokens),
        Some((ChoiceCount::up_to_dynamic_x(), 5))
    );
    assert_eq!(
        parse_half_starting_life_shape(&tokens(
            "half target player's starting life total, rounded down"
        )),
        Some(HalfStartingLifeShape {
            player: PlayerFilter::target_player(),
            rounding: HalfStartingLifeRounding::Down,
        })
    );
}

#[test]
fn parses_counter_surfaces() {
    assert!(is_named_source_power_shape(&tokens("Krenko's power")));
    assert!(is_him_or_her_counter_target(&tokens("him or her")));
    assert_eq!(
        parse_counter_count_prefix_shape(&tokens("this's counters on that creature")),
        CounterCountPrefixShape::Referential(ReferentialCounterCountShape {
            source: CounterReferenceSource::Source,
            counter_type: None,
            consumed: 2,
        })
    );
    let text = "Counter Bear's counters on that creature";
    let context = crate::parse_context::ParseContext::for_fragment(
        "Counter Bear",
        Vec::new(),
        Vec::new(),
        text,
    );
    assert_eq!(
        parse_counter_count_prefix_shape_with_context(context.view(), &tokens(text)),
        CounterCountPrefixShape::Referential(ReferentialCounterCountShape {
            source: CounterReferenceSource::Source,
            counter_type: None,
            consumed: 3,
        })
    );
    let counter_tokens = tokens("+1/+1 counters on target creature equal to the difference");
    let shape = parse_put_counter_target_shape(&counter_tokens).unwrap();
    assert!(shape.equal_to_difference);

    let counter_tokens = tokens(
        "X +1/+1 counters on target creature you control, where X is the mana value of that card",
    );
    let shape = parse_put_counter_target_shape(&counter_tokens).unwrap();
    assert_eq!(
        primitives::TokenWordView::new(shape.target_tokens).to_word_refs(),
        ["target", "creature", "you", "control"]
    );
}

#[test]
fn preserves_distinct_target_in_until_leaves_suffix() {
    let tokens = tokens(
        "target creature or enchantment you don't control until target enchantment you control leaves the battlefield",
    );
    let (exiled, watcher) =
        split_until_target_leaves_shape(&tokens).expect("target watcher suffix");

    assert_eq!(
        primitives::TokenWordView::new(exiled).to_word_refs(),
        [
            "target",
            "creature",
            "or",
            "enchantment",
            "you",
            "dont",
            "control"
        ]
    );
    assert_eq!(
        primitives::TokenWordView::new(watcher).to_word_refs(),
        ["target", "enchantment", "you", "control"]
    );
}
