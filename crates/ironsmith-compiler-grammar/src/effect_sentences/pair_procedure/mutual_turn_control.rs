//! "Choose target player and another target player. The first player controls
//! the second player during the second player's next turn, and the second
//! player controls the first player during the first player's next turn."
//! (Cruel Entertainment): two player targets, each controlled during their
//! own next turn by the other target (CR 722). Lowered as one iteration over
//! the two target players whose control is exercised by the other target.
use super::*;
use crate::cards::builders::{ControlActionAst, ControlDurationAst, ForEachEffectAst};
use crate::lexer::OwnedLexToken;
use crate::util::trim_edge_punctuation_tokens;

fn normalized(tokens: &[OwnedLexToken]) -> String {
    crate::lexer::parser_token_word_refs(trim_edge_punctuation_tokens(tokens))
        .join(" ")
        .replace(['\'', '’'], "")
        .replace("player s ", "players ")
}

pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(choose), Some(control)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    if normalized(choose.lowered()) != "choose target player and another target player"
        || normalized(control.lowered())
            != "the first player controls the second player during the second players next turn and the second player controls the first player during the first players next turn"
    {
        return Ok(None);
    }
    let mut controlled = EffectAst::subject_verb_control_player(
        PlayerAst::Implicit,
        PlayerFilter::IteratedPlayer,
        ControlDurationAst::DuringNextTurn,
    );
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Control(ControlActionAst::ControlPlayer { controller, .. }),
        ..
    }) = &mut controlled
    else {
        return Ok(None);
    };
    // The other of the two declared target players.
    *controller = Some(PlayerFilter::excluding(
        PlayerFilter::target_player(),
        PlayerFilter::IteratedPlayer,
    ));
    Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachTargetPlayers {
        count: ChoiceCount::exactly(2),
        filter: PlayerFilter::Any,
        effects: vec![controlled],
    })]))
}
