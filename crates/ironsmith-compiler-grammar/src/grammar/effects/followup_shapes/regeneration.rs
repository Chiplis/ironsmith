use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CantBeRegeneratedSubject {
    It,
    They,
    CreatureDestroyedThisWay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CantBeRegeneratedFollowupShape {
    pub subject: CantBeRegeneratedSubject,
    pub this_turn: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageRegenerationExileGate {
    DamagedObjectIsCreature,
    ThisSpellWasKicked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageRegenerationExileFollowupShape {
    pub gate: DamageRegenerationExileGate,
}

fn regeneration_subject<'a>(input: &mut LexStream<'a>) -> WResult<CantBeRegeneratedSubject> {
    alt((
        primitives::kw("it").value(CantBeRegeneratedSubject::It),
        primitives::kw("they").value(CantBeRegeneratedSubject::They),
        primitives::phrase(&["those", "creatures"]).value(CantBeRegeneratedSubject::They),
        alt((
            primitives::phrase(&["creature", "destroyed", "this", "way"]),
            primitives::phrase(&["creatures", "destroyed", "this", "way"]),
            primitives::phrase(&["a", "creature", "destroyed", "this", "way"]),
        ))
        .value(CantBeRegeneratedSubject::CreatureDestroyedThisWay),
    ))
    .parse_next(input)
}

fn cant<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("cant"),
        primitives::kw("can't"),
        primitives::kw("cannot"),
    ))
    .void()
    .parse_next(input)
}

#[cfg(test)]
#[path = "regeneration_inline_tests.rs"]
mod tests;

#[path = "regeneration/combat.rs"]
mod combat_programs;
pub use combat_programs::parse_damage_regeneration_exile_followup;
use combat_programs::{
    damage_regeneration_exile_gate, damage_regeneration_subject,
    parse_damage_regeneration_exile_followup_lexed,
};
#[path = "regeneration/condition.rs"]
mod condition_programs;
pub use condition_programs::parse_cant_be_regenerated_followup;
use condition_programs::parse_cant_be_regenerated_followup_lexed;
