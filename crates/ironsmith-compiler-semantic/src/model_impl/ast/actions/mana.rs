//! The mana actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq, TagKeyWalk)]
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
    /// "Add one mana of this artifact's last noted type."
    AddManaNotedType {
        amount: Value,
    },
    /// "you add the mana lost this way" (Drain Power): `amount` is the
    /// preceding mana-loss metric; the lost mana's types are added.
    AddManaLostThisWay {
        amount: Value,
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
    /// "Add mana equal to enchanted permanent's mana cost" (Elemental
    /// Resonance): one mana per symbol of the matching object's mana cost.
    AddManaEqualToManaCost {
        filter: ObjectFilter,
        /// Only colored symbols add mana (Charmed Pendant).
        colored_symbols_only: bool,
    },
    AddOneManaAnyColorAmong {
        filter: ObjectFilter,
        choose_color_of_object_surface: bool,
    },
    /// "Add three mana in any combination of its colors" (Omnath, Locus of
    /// All): `amount` mana, each of a color among the objects `filter`
    /// matches as the effect resolves.
    AddManaAnyColorAmong {
        amount: Value,
        filter: ObjectFilter,
    },
    AddManaCommanderIdentity {
        amount: Value,
    },
    /// "Add one mana of any color chosen as you drafted cards named <name>"
    /// (Paliano, the High City): `amount` mana, each of a color the
    /// controller chose while drafting cards with that name.
    ///
    /// With `chosen_at_deck_construction` set, the colors are instead the
    /// ones circled for this card as its owner built their deck ("one mana
    /// of either of the circled colors", Cryptic Spires); `card_name` is then
    /// empty and unused.
    AddManaDraftChosenColors {
        amount: Value,
        card_name: String,
        chosen_at_deck_construction: bool,
    },
    /// "you don't lose [unspent red] mana as steps and phases end": `color`
    /// restricts the retained mana to one color.
    DontLoseThisManaAsStepsAndPhasesEndThisTurn {
        color: Option<crate::color::Color>,
        /// "Until end of combat, you don't lose this mana as steps end."
        until_end_of_combat: bool,
    },
    AddManaImprintedColors,
    PayMana {
        cost: ManaCost,
        /// Typed value for a printed `{X}` payment whose X is defined by the
        /// surrounding Oracle sentence rather than chosen by the player.
        x_value: Option<Value>,
        /// Inclusive typed maximum for a printed `{X}` payment whose X is
        /// chosen by the paying player.
        x_maximum: Option<Value>,
        independent_x_choice: bool,
        /// "pay that spell's mana cost" (Ice Cave): the object whose mana
        /// cost is paid in addition to `cost`.
        mana_cost_of: Option<crate::target::ChooseSpec>,
    },
    DoubleManaPool,
    EmptyManaPool,
    /// "<player> activates a mana ability of each land they control".
    ActivateManaAbilityOfEachLand,
}
