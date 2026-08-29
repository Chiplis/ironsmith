use super::*;

pub fn parse_tempting_offer_copy_spell_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !effect_grammar::is_tempting_offer_copy_sequence(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
        sentences[sentence_idx + 2].lowered(),
        sentences[sentence_idx + 3].lowered(),
    ) {
        return Ok(None);
    }

    let stack_spell_filter = ObjectFilter {
        zone: Some(Zone::Stack),
        card_types: vec![CardType::Instant, CardType::Sorcery],
        has_mana_cost: true,
        ..Default::default()
    };
    let target_spell = TargetAst::Tagged(TagKey::from(IT_TAG), None);
    let opponent_copy = EffectAst::subject_verb_copy_spell(
        target_spell.clone(),
        Value::Fixed(1),
        PlayerAst::That,
        true,
        false,
        Vec::new(),
    );
    let your_copy_count = Value::PendingEffectMetricOffset {
        source: ironsmith_core::EffectMetricSource::Outcome,
        metric: ironsmith_core::EffectMetric::PlayersWithPositiveCount,
        offset: 1,
    };
    let your_copy = EffectAst::subject_verb_copy_spell(
        target_spell,
        your_copy_count,
        PlayerAst::You,
        true,
        false,
        Vec::new(),
    )
    .with_copy_count_surface(
        ironsmith_core::effect::CopyCountSurface::OncePlusAdditionalPerOpponentWhoCopiedThisWay,
    );

    Ok(Some(vec![
        EffectAst::subject_verb_explicit_target_only(TargetAst::Object(
            stack_spell_filter,
            Some(TextSpan::synthetic()),
            None,
        )),
        EffectAst::ForEachOpponent {
            effects: vec![EffectAst::MayByPlayer {
                player: PlayerAst::That,
                effects: vec![opponent_copy],
            }],
        },
        your_copy,
    ]))
}
