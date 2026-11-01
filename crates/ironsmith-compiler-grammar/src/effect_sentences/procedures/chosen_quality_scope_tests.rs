use super::*;
use crate::cards::builders::{EffectAst, PermissionEffectAst, SubjectVerbActionAst, RevealLookActionAst, ObjectChoiceEffectAst, LibraryActionAst};
use crate::model::{ControlFlowNodeAst, NestedProgramAst};
fn viewed(quality: &str) -> String {
    format!("Look at the top card of your library. You may reveal a {quality} card from among them and put it into your hand. You may put the rest into your graveyard.")
}
fn sentences(text: &str) -> Vec<SentenceInput> {
    let tokens = crate::lexer::lex_line(text, 0).unwrap();
    crate::lexer::split_lexed_sentences(&tokens).iter().map(|tokens| SentenceInput::from_lexed(tokens)).collect()
}
fn control(effects: &[EffectAst]) -> &crate::model::CompilerControlFlowAst {
    let [EffectAst::ControlFlow(control)] = effects else {panic!("missing scoped control: {effects:?}")};
    control
}
fn branches(control: &crate::model::CompilerControlFlowAst) -> [&NestedProgramAst; 2] {
    let ControlFlowNodeAst::Condition {consequence_program, alternative_program: Some(alternative_program), ..} = &control.node else {panic!("missing full alternative")};
    [&control.programs[*consequence_program], &control.programs[*alternative_program]]
}
fn look_tag(program: &NestedProgramAst) -> crate::tag::TagKey {
    program.effects.iter().find_map(|effect| match effect {
        EffectAst::SubjectVerb(subject) => match &subject.action {
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {tag, ..}) => Some(tag.key.clone()),
            _ => None,
        }, _ => None,
    }).expect("private view must be inside this branch")
}
fn check_pool(program: &NestedProgramAst) {
    let tag = look_tag(program);
    let filter = program.effects.iter().find_map(|effect| match effect {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {filter, ..}) => Some(filter),
        _ => None,
    }).expect("this branch owns its matching selection");
    assert!(filter.tagged_constraints.iter().any(|constraint| constraint.tag == tag));
    let rest = program.effects.iter().find_map(|effect| match effect {
        EffectAst::Permissions(PermissionEffectAst::May {effects}) => effects.iter().find_map(|effect| match effect {
            EffectAst::SubjectVerb(subject) => match &subject.action {
                SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone {tag, ..}) => Some(tag.key.clone()),
                _ => None,
            }, _ => None,
        }), _ => None,
    }).expect("exact optional remainder must be inside this branch");
    assert_eq!(rest, tag);
}
#[test]
fn state_condition_encloses_entire_viewed_program_and_otherwise_pool() {
    let text = format!("If land was chosen, {} Otherwise, {}", viewed("land"), viewed("nonland"));
    let matched = recognize(&sentences(&text), 0).unwrap().unwrap();
    assert_eq!(matched.consumed_sentences, 6);
    let [yes, no] = branches(control(&matched.effects));
    check_pool(yes); check_pool(no);
    assert_ne!(look_tag(yes), look_tag(no), "false arm cannot reuse prior true-arm pool");
}
#[test]
fn unrelated_trailing_draw_is_outside_both_complete_branches() {
    let text = format!("If land was chosen, {} Otherwise, {} Draw a card.", viewed("land"), viewed("nonland"));
    let input = sentences(&text);
    let matched = recognize(&input, 0).unwrap().unwrap();
    assert_eq!(matched.consumed_sentences, 6);
    assert_eq!(input.len(), 7);
    for branch in branches(control(&matched.effects)) {
        check_pool(branch);
        assert!(!format!("{:?}", branch.effects).contains("Draw"));
    }
    // Also exercise the public document route, not just the procedure helper:
    // a flat independent-statement shortcut must not erase these nested scopes.
    let tokens = crate::lexer::lex_line(&text, 0).unwrap();
    let effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens).unwrap();
    assert_eq!(effects.len(), 2, "one conditional program, then the unrelated draw");
    assert!(matches!(&effects[0], EffectAst::ControlFlow(_)));
    assert!(format!("{:?}", effects[1]).contains("Draw"));
}
#[test]
fn ordinary_state_predicate_can_gate_a_complete_viewed_procedure() {
    let text = format!("If you control an artifact, {} Otherwise, {}", viewed("creature"), viewed("enchantment"));
    let matched = recognize(&sentences(&text), 0).unwrap().unwrap();
    assert_eq!(matched.consumed_sentences, 6);
    for branch in branches(control(&matched.effects)) {check_pool(branch);}
    assert!(!format!("{:?}", control(&matched.effects).node).contains("SourceChosenOption"));
}
#[test]
fn invalid_suffix_in_either_branch_rejects_without_speculative_claim_leaks() {
    for invalid_true in [false, true] {
        let bad = viewed("land").replace("into your graveyard.", "into your graveyard {2}.");
        let yes = if invalid_true {bad.clone()} else {viewed("land")};
        let no = if invalid_true {viewed("nonland")} else {bad};
        let input = sentences(&format!("If land was chosen, {yes} Otherwise, {no}"));
        let (result, claims) = crate::word_claims::with_enforcement_options(true, true, || crate::word_claims::scoped(|| recognize(&input, 0)));
        assert!(result.is_err(), "malformed scoped branch must not fall back to a shorter accepted program");
        assert!(claims.claimed_spans().is_empty(), "failed speculative scope cannot claim any branch tokens");
    }
}
