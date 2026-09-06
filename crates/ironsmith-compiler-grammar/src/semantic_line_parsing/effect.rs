use crate::cards::builders::ForEachEffectAst;
use super::*;

pub fn parse_next_spell_cost_reduction_sentence(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let parsed = activated_line_grammar::parse_next_spell_cost_reduction_tokens(tokens)?;
    Some(EffectAst::subject_verb_reduce_next_spell_cost_this_turn(
        PlayerAst::You,
        parsed.spell_filter,
        parsed.reduction,
    ))
}

pub fn parse_each_player_and_their_creatures_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let parsed = effect_grammar::parse_each_player_creatures_damage_tokens(tokens)?;
    let mut filter = ObjectFilter::default();
    filter.card_types = vec![CardType::Creature];
    filter.controller = Some(PlayerFilter::IteratedPlayer);

    Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_damage(
                parsed.amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            ),
            EffectAst::subject_verb_damage_each(parsed.amount, filter),
        ],
    })])
}
