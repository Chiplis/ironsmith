use super::*;

use crate::recognition::ParseOutcome;
#[path = "composition_core/bundle_readings.rs"]
mod bundle_readings;

pub fn parse_typed_effect_bundle_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let input = bundle_readings::Bundle {
        tokens,
        sentences: split_lexed_sentences(tokens),
        read_by_cache: Default::default(),
    };
    match bundle_readings::read(&input) {
        ParseOutcome::Match(matched) => Some(matched.value.value),
        // The ladder swallowed every bundle parser's error; a committed
        // diagnostic here is no bundle either.
        ParseOutcome::NoMatch | ParseOutcome::Error(_) => None,
    }
}
