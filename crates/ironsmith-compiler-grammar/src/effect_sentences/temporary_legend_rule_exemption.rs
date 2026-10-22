//! "The \"legend rule\" doesn't apply to permanents you control this turn."
//! (Hall of Echoes): a resolving rule-modifying effect (CR 704.5j exemption)
//! for the rest of the turn. It is a rule change, not a characteristic one,
//! so permanents you begin controlling later in the turn are covered too; it
//! is modeled as a live `ObjectRule::LegendRuleDoesntApply` rule effect.
use crate::cards::builders::EffectAst;
use crate::effect::Until;
use crate::lexer::OwnedLexToken;
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
    let mut exempt = if scope == "creatures" {
        ObjectFilter::creature()
    } else {
        ObjectFilter::permanent()
    };
    exempt = exempt.in_zone(Zone::Battlefield).controlled_by(PlayerFilter::You);
    exempt.set_plural_object_noun_surface(true);
    // A rule modification owned by this resolved effect (CR 611.3a-style
    // rule change): it covers permanents you begin controlling later this
    // turn and can't be removed by taking abilities away from them.
    Some(EffectAst::subject_verb_cant(
        crate::effect::Restriction::object_rule(
            exempt,
            ironsmith_core::ObjectRule::LegendRuleDoesntApply,
        ),
        duration,
        None,
    ))
}
