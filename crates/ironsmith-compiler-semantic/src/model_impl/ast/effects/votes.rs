//! The votes actions of `EffectAst`.

use super::*;

#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum VoteEffectAst {
    BidLife {
        /// The auctioned object (Illicit Auction); `None` for an untargeted
        /// auction whose reward is the winner's actions (Pain's Reward).
        target: Option<TargetAst>,
        /// `None` when the controller opens with "a bid of any number".
        starting_bid: Option<u32>,
        winner_effects: Vec<EffectAst>,
        /// "You and target spell's controller bid life" (Mages' Contest):
        /// the only other bidder; `None` means every player bids.
        opponent: Option<PlayerAst>,
        /// "If you win the bidding, ...": the reward requires the
        /// controller to be the high bidder.
        controller_must_win: bool,
    },
    VoteStart {
        options: Vec<String>,
        secret: bool,
        starting_with_controller: bool,
    },
    SecretChoiceStart {
        options: Vec<String>,
        participants: Vec<PlayerFilter>,
        object_choice: Option<crate::effects::SecretObjectChoice>,
    },
    SecretChoiceReveal,
    VoteStartObjects {
        filter: ObjectFilter,
        count: ChoiceCount,
        secret: bool,
        starting_with_controller: bool,
    },
    VoteStartPlayers {
        filter: PlayerFilter,
        exclude_voter: bool,
        secret: bool,
        starting_with_controller: bool,
    },
    VoteOption {
        option: String,
        effects: Vec<EffectAst>,
    },
    VoteExtra {
        count: u32,
        optional: bool,
    },
}
