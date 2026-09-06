//! The exchanges actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ExchangeActionAst {
    ExchangeLifeTotals {
        player2: PlayerAst,
    },
    ExchangeTextBoxes {
        target: TargetAst,
    },
    ExchangeZones {
        zone1: Zone,
        zone2: Zone,
    },
    ExchangeValues {
        left: ExchangeValueAst,
        right: ExchangeValueAst,
        duration: Until,
    },
    ExchangeControl {
        filter: ObjectFilter,
        count: u32,
        shared_type: Option<SharedTypeConstraintAst>,
    },
    ExchangeControlHeterogeneous {
        permanent1: TargetAst,
        permanent2: TargetAst,
        shared_type: Option<SharedTypeConstraintAst>,
    },
}
