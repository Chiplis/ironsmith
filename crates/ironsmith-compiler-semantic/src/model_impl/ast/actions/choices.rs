//! The choices actions of `SubjectVerbActionAst`.

use ironsmith_compiler_ast::TagRef;
use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ChoiceActionAst {
    ChooseColor,
    ChooseCardType {
        options: Vec<CardType>,
    },
    ChooseNamedOption {
        options: Vec<String>,
    },
    ChooseCreatureType {
        excluded_subtypes: Vec<Subtype>,
        family: SubtypeFamily,
    },
    ChooseLandType {
        exclude_basic: bool,
    },
    ChooseCardName {
        filter: Option<ObjectFilter>,
        tag: TagRef,
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
