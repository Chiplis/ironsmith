//! A bounded choice from historical completed-payment participants.
use super::*;
pub(super) fn read(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let view = crate::lexer::TokenWordView::new(tokens);
    let words = view.word_refs();
    if words.first() != Some(&"choose") || !words.ends_with(&["additional", "cost"]) {
        return Ok(None);
    }
    let Some(from) = words
        .windows(7)
        .position(|part| part == ["from", "the", "creatures", "tapped", "to", "pay", "its"])
    else {
        return Ok(None);
    };
    if from <= 1 || from + 7 >= words.len() - 2 {
        return Ok(None);
    }
    let filter_end = view.token_start_indices()[from];
    let filter = crate::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(
        &tokens[1..filter_end],
        false,
    )?;
    let name = words[from + 7..words.len() - 2].join(" ");
    let source = crate::tag::declared_key(ironsmith_core::tag::additional_cost_tap_receipt(&name));
    let pool = crate::tag::declared_key(format!("__matching_additional_tap:{name}"));
    let selected =
        crate::tag::declared_key(ironsmith_core::tag::selected_additional_cost_tap(&name));
    let remember =
        EffectAst::subject_verb_filtered_tagged_objects(filter, pool.clone(), vec![source]);
    let choose = EffectAst::ObjectChoices(
        crate::cards::builders::ObjectChoiceEffectAst::ChooseObjects {
            filter: ObjectFilter::exact_tagged(pool).in_zone(Zone::Battlefield),
            count: crate::effect::ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: selected,
        },
    );
    crate::word_claims::claim_tokens(&tokens[..1]);
    crate::word_claims::claim_tokens(&tokens[filter_end..]);
    Ok(Some(vec![remember, choose]))
}
