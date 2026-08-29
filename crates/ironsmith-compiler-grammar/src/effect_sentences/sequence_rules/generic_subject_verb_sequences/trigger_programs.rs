use super::*;

/// Preserve a standalone two-sentence Pact-style delayed payment after an
/// earlier sequence rule has already consumed the instruction that precedes
/// it (for example, a prevention effect and its life-gain follow-up).
pub fn parse_delayed_upkeep_unless_pays_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = sequence_grammar::parse_delayed_upkeep_payment_shape(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };

    Ok(Some(vec![EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::You,
        effects: vec![EffectAst::UnlessPays {
            effects: vec![EffectAst::subject_verb_lose_game(PlayerAst::You)],
            player: PlayerAst::You,
            cost: ironsmith_core::TotalCost::mana(shape.mana),
            before_delayed_step: false,
        }],
    }]))
}
