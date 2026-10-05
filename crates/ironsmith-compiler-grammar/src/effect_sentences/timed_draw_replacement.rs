use crate::cards::builders::{CardTextError, EffectAst, ZoneReplacementDurationAst};
use crate::lexer::{OwnedLexToken, parser_token_word_refs, render_token_slice};
use crate::target::PlayerFilter;

pub(super) fn parse_timed_draw_replacement_sentence(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = crate::grammar::effects::timed_draw_replacement::parse_timed_draw_replacement(tokens) else { return Ok(None); };
    let words = parser_token_word_refs(shape.player);
    let (player, player_target) = match words.as_slice() {
        ["you"] => (PlayerFilter::You, None),
        ["they"] | ["that", "player"] => (PlayerFilter::IteratedPlayer, None),
        ["target", "player"] | ["target", "opponent"] => (PlayerFilter::Any, Some(crate::util::parse_target_phrase(shape.player)?)),
        _ => return Err(CardTextError::ParseError("unsupported timed draw replacement player scope".into())),
    };
    let effects = super::parse_effect_sentences_lexed(&shape.body)?;
    let duration = if shape.one_shot { ZoneReplacementDurationAst::OneShot } else { ZoneReplacementDurationAst::UntilEndOfTurn };
    Ok(Some(vec![EffectAst::subject_verb_register_timed_draw_replacement(
        player, player_target, effects, duration, render_token_slice(tokens),
    )]))
}
