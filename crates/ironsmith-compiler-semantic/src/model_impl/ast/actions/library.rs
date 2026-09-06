//! The library actions of `SubjectVerbActionAst`.

use ironsmith_compiler_ast::TagRef;
use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum LibraryActionAst {
    Mill {
        count: Value,
    },
    ManifestTopCardOfLibrary,
    CloakTopCardOfLibrary,
    ShuffleHandAndGraveyardIntoLibrary,
    ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary,
    ShuffleGraveyardIntoLibrary {
        explicit_all_cards_from: bool,
    },
    ReorderGraveyard,
    PutRestOnBottomOfLibrary,
    ExileTopOfLibrary {
        count: Value,
        surface: Option<ironsmith_core::ExileTopLibrarySurface>,
        tags: Vec<TagRef>,
        accumulated_tags: Vec<TagRef>,
        face_down: bool,
    },
    ReorderTopOfLibrary {
        tag: TagRef,
    },
    ShuffleLibrary,
    ShuffleObjectsIntoLibrary {
        target: TargetAst,
        all: bool,
        owner_library_destination: bool,
        possessive_owner_subject: bool,
    },
    PutTaggedRemainderOnBottomOfLibrary {
        tag: TagRef,
        keep_tagged: Option<TagRef>,
        order: LibraryBottomOrderAst,
        player: PlayerAst,
        surface: ironsmith_core::LibraryRemainderSurface,
    },
    /// Moves every object tagged `tag` that is NOT also in the `keep_tagged`
    /// group to `zone`, preserving each object's controller. Lowers to
    /// `for_each_tagged(tag, [conditional(in keep_tagged, [], [move iterated to
    /// zone])])`, keeping the iterated reference internal to lowering (no bare
    /// `it` surfaces). The graveyard/exile analog of
    /// `PutTaggedRemainderOnBottomOfLibrary`.
    PutTaggedRemainderInZone {
        tag: TagRef,
        keep_tagged: TagRef,
        zone: Zone,
        surface: ironsmith_core::LibraryRemainderSurface,
    },
    MoveToLibraryTopOrBottomChoice {
        target: TargetAst,
    },
    ConsultTopOfLibrary {
        player: PlayerAst,
        mode: LibraryConsultModeAst,
        filter: ObjectFilter,
        stop_rule: LibraryConsultStopRuleAst,
        max_exposed: Option<Value>,
        all_tag: TagRef,
        match_tag: TagRef,
    },
    MoveToLibraryNthFromTop {
        target: TargetAst,
        position: Value,
    },
}
