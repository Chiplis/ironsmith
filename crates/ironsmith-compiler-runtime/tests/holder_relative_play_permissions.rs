//! cf8 p10: play permissions whose lifetime is "until the end of their next
//! turn" (the permission holder's next turn) and "up to N of those cards".
//! Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effects::{GrantPlayTaggedDuration, GrantPlayTaggedEffect};
use ironsmith::target::PlayerFilter;

const SUSPEND_AGGRESSION: &str = "Mana cost: {1}{R}{W}\nType: Instant\nExile target nonland permanent and the top card of your library. For each of those cards, its owner may play it until the end of their next turn.";
const EXPEDITED_INHERITANCE: &str = "Mana cost: {R}{R}\nType: Enchantment\nWhenever a creature is dealt damage, its controller may exile that many cards from the top of their library. They may play those cards until the end of their next turn.";
const MARCH_OF_RECKLESS_JOY: &str = "Mana cost: {X}{R}\nType: Instant\nAs an additional cost to cast this spell, you may exile any number of red cards from your hand. This spell costs {2} less to cast for each card exiled this way.\nExile the top X cards of your library. You may play up to two of those cards until the end of your next turn.";

fn grants(name: &str, text: &str) -> Vec<GrantPlayTaggedEffect> {
    support::definitions(name, text)
        .iter()
        .flat_map(support::find_all::<GrantPlayTaggedEffect>)
        .collect()
}

#[test]
fn suspend_aggression_owner_keeps_permission_through_their_next_turn() {
    let grants = grants("Suspend Aggression", SUSPEND_AGGRESSION);
    assert_eq!(grants.len(), 2, "one grant per route");
    for grant in grants {
        assert_eq!(grant.duration, GrantPlayTaggedDuration::UntilYourNextTurnEnd);
        assert!(grant.allow_land, "play, not just cast");
        assert_ne!(grant.player, PlayerFilter::You, "each card's owner holds the permission");
        assert_eq!(grant.max_plays, None);
    }
}

#[test]
fn expedited_inheritance_damaged_controller_holds_their_next_turn_permission() {
    let grants = grants("Expedited Inheritance", EXPEDITED_INHERITANCE);
    assert_eq!(grants.len(), 2);
    for grant in grants {
        assert_eq!(grant.duration, GrantPlayTaggedDuration::UntilYourNextTurnEnd);
        assert_ne!(grant.player, PlayerFilter::You, "the exiling player, not the enchantment's controller");
    }
}

#[test]
fn march_of_reckless_joy_shares_a_two_play_budget() {
    let grants = grants("March of Reckless Joy", MARCH_OF_RECKLESS_JOY);
    assert_eq!(grants.len(), 2);
    for grant in grants {
        assert_eq!(grant.duration, GrantPlayTaggedDuration::UntilYourNextTurnEnd);
        assert_eq!(grant.player, PlayerFilter::You);
        assert_eq!(grant.max_plays, Some(2));
    }
}
