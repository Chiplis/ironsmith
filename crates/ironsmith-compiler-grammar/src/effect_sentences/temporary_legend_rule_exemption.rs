//! "The \"legend rule\" doesn't apply to permanents you control this turn."
//! (Hall of Echoes): a resolving rule-modifying effect (CR 704.5j exemption)
//! for the rest of the turn. It is a rule change, not a characteristic one,
//! so permanents you begin controlling later in the turn are covered too; it
//! is modeled as the controller-scoped exemption carried by every permanent
//! you control until end of turn (the affected set stays live).
use crate::cards::builders::{EffectAst, GrantedAbilityAst, StaticAbilityAst};
use crate::effect::Until;
use crate::lexer::OwnedLexToken;
use crate::model::CompilerStaticAbilityCore as StaticAbility;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

pub(crate) fn parse(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::token_word_refs(tokens);
    let (scope, rest) = match words.as_slice() {
        [
            "the",
            "legend",
            "rule",
            "doesnt" | "doesn't",
            "apply",
            "to",
            scope @ ("permanents" | "creatures"),
            "you",
            "control",
            rest @ ..,
        ] => (*scope, rest),
        _ => return None,
    };
    let duration = match rest {
        ["this", "turn"] | ["until", "end", "of", "turn"] => Until::EndOfTurn,
        _ => return None,
    };
    let exempt = if scope == "creatures" {
        ObjectFilter::creature()
    } else {
        ObjectFilter::permanent()
    };
    let holders = ObjectFilter::permanent()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You);
    Some(EffectAst::subject_verb_grant_abilities_all_dynamically(
        holders,
        vec![GrantedAbilityAst::StaticAbility(Box::new(StaticAbilityAst::Static(
            StaticAbility::legend_rule_doesnt_apply_to_controller_matching(exempt),
        )))],
        duration,
    ))
}
