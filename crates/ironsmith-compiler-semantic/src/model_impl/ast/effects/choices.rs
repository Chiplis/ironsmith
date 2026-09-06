//! The objectchoices actions of `EffectAst`.

use ironsmith_compiler_ast::TagRef;
use super::*;

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ObjectChoiceEffectAst {
    ChooseObjects {
        filter: ObjectFilter,
        count: ChoiceCount,
        count_value: Option<Value>,
        player: PlayerAst,
        tag: TagRef,
    },
    /// Choose objects subject to a constraint on the selection as a whole.
    ChooseObjectsWithAggregateConstraint {
        filter: ObjectFilter,
        count: ChoiceCount,
        player: PlayerAst,
        tag: TagRef,
        constraint: crate::effect::ChoiceAggregateConstraint,
    },
    ChooseObjectsBottomOfLibrary {
        filter: ObjectFilter,
        count: ChoiceCount,
        count_value: Option<Value>,
        player: PlayerAst,
        tag: TagRef,
    },
    /// Choose from the top boundary of the filter's ordered zone while retaining an explicit
    /// chooser. This composes the existing runtime `ChooseObjectsEffect`
    /// `top_only` capability with later tagged zone moves, which is required
    /// for face-down exile procedures where `ExileTopOfLibraryEffect` (always
    /// public) is not the correct primitive.
    ChooseObjectsTopOfZone {
        filter: ObjectFilter,
        count: ChoiceCount,
        count_value: Option<Value>,
        player: PlayerAst,
        tag: TagRef,
    },
    /// Choose objects strictly within a single explicit `zone`, without the
    /// cross-zone scoping heuristic `ChooseObjects` applies to tagged pools.
    /// Lowers to a plain `ChooseObjectsEffect::new(filter, count, chooser,
    /// tag).in_zone(zone)`, mirroring how the retired looked-cards recipes built
    /// their inner choose. Used to compose "choose N of the looked-at cards"
    /// where the pool is known to live in one zone (e.g. the library).
    ChooseTaggedObjectsInZone {
        filter: ObjectFilter,
        count: ChoiceCount,
        player: PlayerAst,
        tag: TagRef,
        zone: Zone,
    },
    ChooseObjectsAcrossZones {
        filter: ObjectFilter,
        count: ChoiceCount,
        count_value: Option<Value>,
        player: PlayerAst,
        tag: TagRef,
        zones: Vec<Zone>,
        search_mode: Option<crate::effect::SearchSelectionMode>,
    },
    /// A player-facing modal choice: the player picks one mode, and only that
    /// mode's effects resolve. Lowers to `Effect::choose_one`.
    ChooseOneOf {
        modes: Vec<ChooseOneModeAst>,
    },
    /// A resolution-time villainous choice made by the specified player.
    VillainousChoice {
        player: PlayerFilter,
        player_surface: Option<String>,
        modes: Vec<ChooseOneModeAst>,
    },
}
