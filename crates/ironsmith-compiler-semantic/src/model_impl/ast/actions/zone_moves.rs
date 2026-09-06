//! The zonemoves actions of `SubjectVerbActionAst`.

use ironsmith_compiler_ast::TagRef;
use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ZoneMoveActionAst {
    ReturnSourceTransformedFromExile,
    ExileWhenSourceLeaves {
        target: TargetAst,
    },
    SacrificeSourceWhenLeaves {
        target: TargetAst,
    },
    ExileInsteadOfGraveyardThisTurn,
    /// Put the chosen/iterated objects onto the battlefield under a resolved
    /// controller. Inside a `ForEachTagged`, `TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.as_str())` lowers
    /// to `ChooseSpec::Iterated`; otherwise the tagged collection is used.
    /// Lowers to `Effect::put_onto_battlefield`.
    PutOntoBattlefield {
        target: TargetAst,
        tapped: bool,
        controller: ReturnControllerAst,
        cloak: bool,
        shuffle_before: bool,
    },
    MayMoveToZone {
        target: TargetAst,
        zone: Zone,
    },
    ReturnToBattlefield {
        target: TargetAst,
        target_reference_surface: Option<ironsmith_core::SearchResultReferenceSurface>,
        from_graveyard_or_exile: bool,
        tapped: bool,
        transformed: bool,
        converted: bool,
        controller: ReturnControllerAst,
        count_value: Option<Value>,
        as_aura: Option<ReturnAsAuraAst>,
        top_only: bool,
    },
    ReturnAllToBattlefield {
        filter: ObjectFilter,
        tapped: bool,
        face_down: bool,
        controller: ReturnControllerAst,
        verb_surface: ironsmith_core::MoveToZoneVerbSurface,
    },
    ExileUntilSourceLeaves {
        target: TargetAst,
        duration: ironsmith_core::ExileUntilDuration,
        /// A separately declared permanent whose departure ends the exile
        /// duration. `None` means the ability source is the watcher.
        leave_watcher: Option<TargetAst>,
        face_down: bool,
        all: bool,
        explicit_return_surface: bool,
    },
    MoveToZone {
        target: TargetAst,
        /// The target is selected from the first matching object in its ordered source zone.
        source_top_only: bool,
        zone: Zone,
        to_top: bool,
        library_order: Option<LibraryBottomOrderAst>,
        library_order_chooser: PlayerAst,
        verb_surface: ironsmith_core::MoveToZoneVerbSurface,
        target_plural_surface: bool,
        target_reference_surface: Option<ironsmith_core::SearchResultReferenceSurface>,
        destination_player_surface: Option<PlayerAst>,
        destination_player_reference_surface:
            Option<ironsmith_core::DestinationPlayerReferenceSurface>,
        exiled_with_source_surface: Option<ironsmith_core::ExiledWithSourceMoveSurface>,
        battlefield_controller: ReturnControllerAst,
        battlefield_tapped: bool,
        battlefield_attacking: bool,
        battlefield_attack_target_player_or_planeswalker_controlled_by: Option<PlayerAst>,
        battlefield_face_down: bool,
        battlefield_transformed: bool,
        attached_to: Option<TargetAst>,
        all: bool,
    },
    SearchLibrary {
        filter: ObjectFilter,
        /// Zones searched by the authored search action. Ordinary library
        /// searches contain only `Library`; multi-zone searches retain every
        /// authored origin so lowering does not collapse them back to one
        /// library when another modifier (such as battlefield entry counters)
        /// selects this AST shape.
        search_zones: Vec<Zone>,
        destination: Zone,
        chooser: PlayerAst,
        player: PlayerAst,
        search_mode: crate::effect::SearchSelectionMode,
        reveal: bool,
        reveal_reference_surface: Option<crate::effect::SearchResultReferenceSurface>,
        shuffle: bool,
        count: ChoiceCount,
        count_value: Option<Value>,
        library_position_from_top: Option<Value>,
        result_reference_surface: crate::effect::SearchResultReferenceSurface,
        search_top_in_any_order_surface: bool,
        tapped: bool,
        enters_with_counters: Vec<ironsmith_core::BattlefieldEntryCounterSpec>,
        /// Whether the put clause hands the found card to the searcher ("… and
        /// put it onto the battlefield under your control"). Without it the card
        /// enters under the SEARCHED player's control, which is only correct
        /// when you searched your own library.
        enters_under_your_control: bool,
    },
    SearchLibrarySlotsToHand {
        slots: Vec<SearchLibrarySlotAst>,
        destination: Zone,
        reveal: bool,
        progress_tag: TagRef,
    },
    Destroy {
        target: TargetAst,
        no_regeneration: bool,
        creature_destroyed_this_way_surface: bool,
    },
    DestroyAll {
        filter: ObjectFilter,
        no_regeneration: bool,
        creature_destroyed_this_way_surface: bool,
    },
    DestroyAllOfChosenColor {
        filter: ObjectFilter,
        no_regeneration: bool,
        creature_destroyed_this_way_surface: bool,
    },
    DestroyAllAttachedTo {
        filter: ObjectFilter,
        target: TargetAst,
    },
    ExileAllAttachedTo {
        filter: ObjectFilter,
        target: TargetAst,
        face_down: bool,
    },
    Exile {
        target: TargetAst,
        face_down: bool,
        /// The target is selected from the first matching object in its ordered source zone.
        source_top_only: bool,
        /// Preserve an authored plural reference even when the linked target
        /// specification itself is represented by a singular tagged handle.
        target_plural_surface: bool,
    },
    ExileAll {
        filter: ObjectFilter,
        face_down: bool,
    },
    ReturnToHand {
        target: TargetAst,
        random: bool,
        destination_player_surface: Option<PlayerAst>,
        exiled_with_source_surface: Option<ironsmith_core::ExiledWithSourceMoveSurface>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
        set_reference_surface: Option<String>,
    },
    ReturnAllToHand {
        filter: ObjectFilter,
        destination_player_surface: Option<PlayerAst>,
        exiled_with_source_surface: Option<ironsmith_core::ExiledWithSourceMoveSurface>,
    },
    ReturnAllToHandOfChosenColor {
        filter: ObjectFilter,
    },
    Discard {
        count: Value,
        random: bool,
        any_number: bool,
        filter: Option<ObjectFilter>,
        tag: Option<TagRef>,
    },
    DiscardHand,
    PlayFromGraveyardUntilEot,
    Sacrifice {
        filter: ObjectFilter,
        count: u32,
        target: Option<TargetAst>,
        /// The object phrase selected one member of a referenced collection
        /// ("one of them") rather than referring to a known singleton ("it").
        one_of_referenced_set: bool,
    },
    SacrificeAll {
        filter: ObjectFilter,
    },
}
