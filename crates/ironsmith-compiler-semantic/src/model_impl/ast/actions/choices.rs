//! The choices actions of `SubjectVerbActionAst`.

use super::*;
use ironsmith_compiler_ast::TagRef;

/// `ChoosePlayer.exclude_previous_choices` for "choose a different <player>"
/// (Dack Fayden, Helping Hand: "For each of those permanents, choose a
/// different opponent."): distinct from every earlier choice, including the
/// choices the same instruction made in earlier iterations of its loop.
pub const DISTINCT_FROM_EVERY_PRIOR_PLAYER_CHOICE: usize = usize::MAX;

#[derive(Clone, PartialEq, TagKeyWalk)]
pub enum ChoiceActionAst {
    ChooseColor,
    ChooseNumber { min: u32, max: Option<u32>, source_owned: bool, draw_count_choice: bool },
    ChooseCardType {
        options: Vec<CardType>,
        shared_among: Option<(ObjectFilter, u32)>,
    },
    ChooseNamedOption {
        options: Vec<String>,
    },
    ChooseCreatureType {
        allowed_subtypes: Vec<Subtype>,
        secretly: bool,
        excluded_subtypes: Vec<Subtype>,
        family: SubtypeFamily,
        /// "Note a creature type that hasn't been noted for this Saga."
        note_for_source: bool,
    },
    ChooseLandType {
        exclude_basic: bool,
        /// "choose a basic land type".
        basic_only: bool,
    },
    ChooseCardName {
        filter: Option<ObjectFilter>,
        tag: TagRef,
        options: Vec<String>,
        exclude_source_history: bool,
    },
    ChoosePlayer {
        filter: PlayerFilter,
        tag: TagRef,
        random: bool,
        exclude_previous_choices: usize,
    },
    ChooseSpellCastHistory {
        cast_by: PlayerAst,
        filter: ObjectFilter,
        tag: TagRef,
    },
}
