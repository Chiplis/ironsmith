use crate::cards::builders::LifeResourceActionAst;
use super::*;
use crate::model::ast::SubjectVerbEffectAst;

#[test]
fn creature_card_exiled_this_way_keeps_a_typed_lki_gate() {
    let tokens =
        crate::lexer::lex_line("For each creature card exiled this way, you gain 1 life", 0)
            .expect("typed exiled-result iterator should lex");
    let effects = parse_for_each_exiled_this_way_sentence(&tokens)
        .expect("typed iterator should parse")
        .expect("typed iterator should be recognized");
    let [
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            effects: iterated, ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one tagged-result loop: {effects:#?}");
    };
    let [
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::ItMatchedLastKnown(filter),
            if_true,
            if_false,
        }),
    ] = iterated.as_slice()
    else {
        panic!("expected a typed LKI condition inside the loop: {iterated:#?}");
    };
    assert_eq!(filter.card_types, [CardType::Creature]);
    assert!(filter.union_surface.explicit_card_noun());
    assert!(if_false.is_empty());
    assert!(matches!(
        if_true.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { .. }),
            ..
        })]
    ));
}
