use super::*;
use crate::lexer::{TokenWordView, lex_line};

fn words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    TokenWordView::new(tokens).to_word_refs()
}

#[test]
fn parses_remove_counter_and_combat_shapes() {
    let tokens = lex_line("two charge counters from target artifact", 0).unwrap();
    let shape = parse_remove_clause_shape(&tokens).unwrap();
    let RemoveClauseShape::Counters {
        amount,
        counter_descriptor,
        destination,
        ..
    } = shape
    else {
        panic!("expected counter removal");
    };
    assert_eq!(amount, Value::Fixed(2));
    assert_eq!(words(counter_descriptor), vec!["charge"]);
    let RemoveCounterDestination::Single { target_tokens } = destination else {
        panic!("expected single target");
    };
    assert_eq!(words(target_tokens), vec!["target", "artifact"]);

    let all = lex_line("all counters from that creature", 0).unwrap();
    assert!(matches!(
        parse_remove_clause_shape(&all),
        Ok(RemoveClauseShape::AllCounters {
            counter_descriptor,
            ..
        }) if counter_descriptor.is_empty()
    ));

    let tokens = lex_line("target creature from combat", 0).unwrap();
    assert!(matches!(
        parse_remove_clause_shape(&tokens),
        Ok(RemoveClauseShape::FromCombat { .. })
    ));
}

#[test]
fn parses_counter_distribution_from_among_all_permanents() {
    let tokens = lex_line("up to three stun counters from among all permanents", 0).unwrap();
    let RemoveClauseShape::Counters {
        amount,
        up_to,
        counter_descriptor,
        destination,
    } = parse_remove_clause_shape(&tokens).unwrap()
    else {
        panic!("expected counter removal");
    };
    assert_eq!(amount, Value::Fixed(3));
    assert!(up_to);
    assert_eq!(words(counter_descriptor), vec!["stun"]);
    let RemoveCounterDestination::Among { filter_tokens } = destination else {
        panic!("expected distributed among destination");
    };
    assert_eq!(words(filter_tokens), vec!["permanents"]);
}

#[path = "tests/reference.rs"]
mod reference_programs;
use reference_programs::{
    does_not_treat_unrelated_coordinated_destroy_subjects_as_attached_to_one_target,
    parses_destroy_unless_target_color_sets_differ,
    parses_inline_same_object_no_regeneration_rider,
    parses_target_and_demonstrative_attached_object_set_as_one_destroy_shape,
};
#[path = "tests/choice.rs"]
mod choice_programs;
use choice_programs::{
    chosen_this_way_type_qualifier_remains_an_object_filter,
    parses_not_chosen_by_any_player_as_the_complement_set,
    parses_not_chosen_this_way_as_the_complement_set,
};
#[path = "tests/combat.rs"]
mod combat_programs;
use combat_programs::{
    couldnt_attack_exception_stays_inside_the_destroy_filter_domain,
    parses_combat_history_and_blocked_targets,
};
#[path = "tests/trigger.rs"]
mod trigger_programs;
use trigger_programs::parses_destroy_all_and_delayed_shapes;
#[path = "tests/core.rs"]
mod core_programs;
use core_programs::parses_each_of_any_number_as_an_optional_unbounded_subset;
#[path = "tests/library.rs"]
mod library_programs;
use library_programs::parses_number_of_counters_equal_to_referenced_card_mana_value;
