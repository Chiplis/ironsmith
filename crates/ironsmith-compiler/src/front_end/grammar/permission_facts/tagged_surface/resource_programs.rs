use super::*;

pub(super) fn lifetime_from_turn_duration(
    duration: leaf::LeafTurnDurationPhrase,
) -> PermissionLifetimeFact {
    match duration {
        leaf::LeafTurnDurationPhrase::ThisTurn => PermissionLifetimeFact::ThisTurn,
        leaf::LeafTurnDurationPhrase::UntilEndOfTurn => PermissionLifetimeFact::UntilEndOfTurn,
        leaf::LeafTurnDurationPhrase::UntilYourNextTurn
        | leaf::LeafTurnDurationPhrase::UntilYourNextTurnEnd => {
            PermissionLifetimeFact::UntilYourNextTurn
        }
    }
}
