use super::*;
use crate::{lex_line, split_lexed_sentences};

fn sentence_inputs(text: &str) -> Vec<SentenceInput> {
    let tokens = lex_line(text, 0).expect("participant loot text should lex");
    split_lexed_sentences(&tokens)
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect()
}

#[test]
fn preserves_participant_fanout_and_greatest_mana_value_ties() {
    let sentences = sentence_inputs(
        "You and defending player each draw a card, then discard a card. Put two +1/+1 counters on this creature if you discarded the card with the greatest mana value among those cards or tied for greatest.",
    );
    let parsed = parse_controller_defending_loot_then_greatest_mana_value_followup(&sentences, 0)
        .expect("typed participant loot parser")
        .expect("exact participant loot shape");
    let [
        EffectAst::IfEffectResult {
            effect,
            predicate:
                EffectPredicate::PlayerAffectedObjectHasGreatestManaValue {
                    player: PlayerFilter::You,
                },
            if_true,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected typed producer/result gate: {parsed:#?}");
    };
    assert!(matches!(
        effect.as_ref(),
        EffectAst::ForEachPlayersFiltered { filter, effects }
            if *filter == PlayerFilter::excluding(
                PlayerFilter::Any,
                PlayerFilter::excluding(PlayerFilter::NotYou, PlayerFilter::Defending),
            ) && effects.len() == 2
    ));
    assert!(matches!(
        if_true.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutCounters { .. },
            ..
        })]
    ));
}

#[test]
fn rejects_a_different_extremum_condition() {
    let sentences = sentence_inputs(
        "You and defending player each draw a card, then discard a card. Put two +1/+1 counters on this creature if you discarded the card with the lowest mana value among those cards or tied for lowest.",
    );
    assert!(
        parse_controller_defending_loot_then_greatest_mana_value_followup(&sentences, 0)
            .expect("near-miss parser")
            .is_none()
    );
}
