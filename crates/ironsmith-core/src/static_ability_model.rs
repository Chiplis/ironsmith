use crate::CounterType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalSpellKeywordKind {
    Flash,
    Cascade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraveyardCountMetric {
    CardTypes,
    ManaValues,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalSpellKeywordSpec {
    pub keyword: ConditionalSpellKeywordKind,
    pub metric: GraveyardCountMetric,
    pub threshold: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PregameActionKind {
    BeginOnBattlefield(PregameBeginOnBattlefieldSpec),
    ChooseColor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PregameBeginOnBattlefieldSpec {
    pub require_not_starting_player: bool,
    pub counters: Vec<(CounterType, u32)>,
    pub exile_cards_from_hand: usize,
}
