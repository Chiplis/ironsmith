use super::super::super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, ChooseOneModeAst, EffectAst, IfResultPredicate, ObjectFilter, PlayerAst,
    TargetAst,
};
use crate::effect::Value;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Preserve an optional per-opponent choice between sacrificing and
/// discarding together with the following consequence for opponents who did
/// neither. The two effects remain inside one opponent frame, so the result
/// test is executable independently for each participant.
pub fn parse_each_opponent_may_sacrifice_or_discard_then_damage_nonparticipants(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(second) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    if !matches!(
        crate::lexer::token_word_refs(first.lowered()).as_slice(),
        [
            "each",
            "opponent",
            "may",
            "sacrifice",
            "a",
            "nonland",
            "permanent",
            "of",
            "their",
            "choice",
            "or",
            "discard",
            "a",
            "card"
        ]
    ) || !matches!(
        crate::lexer::token_word_refs(second.lowered()).as_slice(),
        [
            "then",
            "this",
            "creature",
            "deals",
            "damage",
            "equal",
            "to",
            "its",
            "power",
            "to",
            "each",
            "opponent",
            "who",
            "didnt" | "didn't",
            "sacrifice",
            "a",
            "permanent",
            "or",
            "discard",
            "a",
            "card",
            "this",
            "way"
        ]
    ) {
        return Ok(None);
    }

    let sacrifice_filter = ObjectFilter::nonland()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::IteratedPlayer);
    let sacrifice = EffectAst::subject_verb_sacrifice(PlayerAst::That, sacrifice_filter, 1, None);
    let discard =
        EffectAst::subject_verb_discard(PlayerAst::That, Value::Fixed(1), false, false, None, None);
    let choice = EffectAst::VillainousChoice {
        player: PlayerFilter::IteratedPlayer,
        player_surface: None,
        modes: vec![
            ChooseOneModeAst {
                description: "Sacrifice a nonland permanent".to_string(),
                effects: vec![sacrifice],
            },
            ChooseOneModeAst {
                description: "Discard a card".to_string(),
                effects: vec![discard],
            },
        ],
    };
    let offer = EffectAst::ForEachOpponent {
        effects: vec![EffectAst::MayByPlayer {
            player: PlayerAst::That,
            effects: vec![choice],
        }],
    };
    let damage = EffectAst::subject_verb_damage_equal_to_power(
        TargetAst::Source(None),
        TargetAst::Player(PlayerFilter::IteratedPlayer, None),
    );
    let consequence = EffectAst::ForEachOpponentDid {
        effects: vec![damage],
        predicate: None,
        result_predicate: IfResultPredicate::DidNot,
    };

    Ok(Some(vec![offer, consequence]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn parse_pair(first: &str, second: &str) -> Option<Vec<EffectAst>> {
        let first = lex_line(first, 0).expect("first sentence should lex");
        let second = lex_line(second, 0).expect("second sentence should lex");
        let sentences = [
            SentenceInput::from_lexed(&first),
            SentenceInput::from_lexed(&second),
        ];
        parse_each_opponent_may_sacrifice_or_discard_then_damage_nonparticipants(&sentences, 0)
            .expect("pair parser should not error")
    }

    #[test]
    fn keeps_optional_modes_and_correlated_opponent_failure() {
        let effects = parse_pair(
            "Each opponent may sacrifice a nonland permanent of their choice or discard a card.",
            "Then this creature deals damage equal to its power to each opponent who didn't sacrifice a permanent or discard a card this way.",
        )
        .expect("exact pair should parse");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("VillainousChoice"), "{debug}");
        assert!(debug.contains("ForEachOpponentDid"), "{debug}");
        assert!(debug.contains("DidNot"), "{debug}");
        assert!(debug.contains("DealDamageEqualToPower"), "{debug}");

        let lowered = crate::compile_support::compile_statement_effects(&effects)
            .expect("correlated opponent choice should lower");
        let lowered_debug = format!("{lowered:#?}");
        assert!(
            lowered_debug.contains("ForPlayersEffect"),
            "{lowered_debug}"
        );
        assert!(lowered_debug.contains("WithIdEffect"), "{lowered_debug}");
        assert!(lowered_debug.contains("MayEffect"), "{lowered_debug}");
        assert!(
            lowered_debug.contains("VillainousChoiceEffect"),
            "{lowered_debug}"
        );
        assert!(
            lowered_debug.contains("predicate: DidNotHappen"),
            "{lowered_debug}"
        );
        assert!(
            lowered_debug.matches("IteratedPlayer").count() >= 4,
            "{lowered_debug}"
        );
    }

    #[test]
    fn does_not_claim_a_required_or_nonland_specific_near_miss() {
        assert!(
            parse_pair(
                "Each opponent sacrifices a nonland permanent of their choice or discards a card.",
                "Then this creature deals damage equal to its power to each opponent who didn't sacrifice a permanent or discard a card this way.",
            )
            .is_none()
        );
        assert!(
            parse_pair(
                "Each opponent may sacrifice a permanent of their choice or discard a card.",
                "Then this creature deals damage equal to its power to each opponent who didn't sacrifice a permanent or discard a card this way.",
            )
            .is_none()
        );
    }
}
