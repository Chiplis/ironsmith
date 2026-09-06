//! The mana actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ManaActionAst {
    AddMana {
        mana: Vec<ManaSymbol>,
    },
    AddManaScaled {
        mana: Vec<ManaSymbol>,
        amount: Value,
    },
    AddManaAnyColor {
        amount: Value,
        available_colors: Option<Vec<crate::color::Color>>,
        distinct_colors: bool,
    },
    AddManaAnyOneColor {
        amount: Value,
    },
    AddManaChosenColor {
        amount: Value,
        fixed_option: Option<crate::color::Color>,
    },
    AddManaFromLandCouldProduce {
        amount: Value,
        land_filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
        mana_type_source: crate::effects::ManaTypeSource,
    },
    AddManaColorsAmong {
        filter: ObjectFilter,
    },
    AddOneManaAnyColorAmong {
        filter: ObjectFilter,
        choose_color_of_object_surface: bool,
    },
    AddManaCommanderIdentity {
        amount: Value,
    },
    DontLoseThisManaAsStepsAndPhasesEndThisTurn,
    AddManaImprintedColors,
    PayMana {
        cost: ManaCost,
        /// Typed value for a printed `{X}` payment whose X is defined by the
        /// surrounding Oracle sentence rather than chosen by the player.
        x_value: Option<Value>,
        /// Inclusive typed maximum for a printed `{X}` payment whose X is
        /// chosen by the paying player.
        x_maximum: Option<Value>,
    },
    DoubleManaPool,
    EmptyManaPool,
}
