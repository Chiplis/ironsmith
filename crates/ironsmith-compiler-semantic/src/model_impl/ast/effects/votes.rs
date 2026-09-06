//! The votes actions of `EffectAst`.

use super::*;

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum VoteEffectAst {
    BidLife {
        target: TargetAst,
        starting_bid: u32,
        winner_effects: Vec<EffectAst>,
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
