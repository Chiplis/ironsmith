pub use ironsmith_core::TagKey;
pub use ironsmith_core::tag::{TagKeyWalk, tag_keys_of};
use ironsmith_compiler_ast::symbols::{Cardinality, ObjectDomain, ReferenceRole};

const SENTENCE_HELPER_ROOT: &str = "__sentence_helper_";

pub fn sentence_helper_tag(purpose: &str, line: usize, start: usize, end: usize) -> TagKey {
    declared({
    TagKey::new(format!(
        "{SENTENCE_HELPER_ROOT}{purpose}_l{line}_s{start}_e{end}"
    ))
    })
}

pub fn is_sentence_helper_tag(tag: &TagKey, purpose: &str) -> bool {
    let Some(rest) = tag.as_str().strip_prefix(SENTENCE_HELPER_ROOT) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(purpose) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix("_l") else {
        return false;
    };
    let Some((line, rest)) = rest.split_once("_s") else {
        return false;
    };
    let Some((start, end)) = rest.split_once("_e") else {
        return false;
    };
    line.parse::<usize>().is_ok() && start.parse::<usize>().is_ok() && end.parse::<usize>().is_ok()
}

pub fn generated_result_tag(purpose: &str, ordinal: u32) -> TagKey {
    declared({
    if matches!(purpose, "exiled" | "looked" | "chosen" | "revealed") {
        sentence_helper_tag(purpose, 0, 0, ordinal as usize)
    } else {
        TagKey::new(format!("{purpose}_{ordinal}"))
    }
    })
}

/// Compiler-owned roles for objects selected while paying a cost.
///
/// The runtime key spelling is an interchange detail centralized here; grammar
/// and resolution carry the role and ordinal instead of assembling prefixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerCostObjectTag {
    Tap,
    Discard,
    Sacrifice,
    Unattach,
    Exile,
    ReturnToHand,
    Blight,
}

/// Mints a key outside the `CompilerReferenceTag` vocabulary and declares it in
/// the active reference scope. Prefer a vocabulary tag's `bind()`.
pub fn declared_key(key: impl Into<TagKey>) -> TagKey {
    declared(key.into())
}

/// A key the grammar mints for an object it will refer back to: declared in
/// the active reference scope (see `ironsmith_compiler_ast::reference_ledger`).
fn declared(key: TagKey) -> TagKey {
    ironsmith_compiler_ast::reference_ledger::note_minted(
        key.clone(),
        ReferenceRole::Affected,
        ObjectDomain::Object,
        Cardinality::Any,
    );
    key
}

impl CompilerCostObjectTag {
    const fn stem(self) -> &'static str {
        match self {
            Self::Tap => "tap_cost",
            Self::Discard => "discard_cost",
            Self::Sacrifice => "sacrifice_cost",
            Self::Unattach => "unattach_cost",
            Self::Exile => "exile_cost",
            Self::ReturnToHand => "return_cost",
            Self::Blight => "blight_cost",
        }
    }

    pub fn key(self, ordinal: usize) -> TagKey {
        declared(TagKey::new(format!("{}_{ordinal}", self.stem())))
    }

    pub fn matches(self, tag: &TagKey) -> bool {
        let stem = self.stem();
        tag.as_str()
            .strip_prefix(stem)
            .and_then(|suffix| suffix.strip_prefix('_'))
            .is_some_and(|ordinal| ordinal.parse::<usize>().is_ok())
    }
}

/// Typed relationships used to derive one compiler binding from another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerDerivedTag {
    DelegatedSubset,
    ExiledCollection,
    CorrelatedSource,
    CorrelatedResult,
    OpposingTarget,
}

impl CompilerDerivedTag {
    const fn serialized_suffix(self) -> &'static str {
        match self {
            Self::DelegatedSubset => "__delegated_subset",
            Self::ExiledCollection => "__exiled_collection",
            Self::CorrelatedSource => "_correlated_source",
            Self::CorrelatedResult => "_correlated_result",
            Self::OpposingTarget => "_opposing_target",
        }
    }

    pub fn key(self, source: &TagKey) -> TagKey {
        declared(TagKey::new(format!("{}{}", source.as_str(), self.serialized_suffix())))
    }
}

/// Generated bindings whose identity is an authored member or replacement
/// ordinal rather than a free-form name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerIndexedTag {
    LinkedFanoutPrimary,
    LinkedFanoutGroup,
    DrawReplacementAll,
    DrawReplacementMatch,
    WaterbendCost,
    ExiledAggregate,
}

impl CompilerIndexedTag {
    const fn stem(self) -> &'static str {
        match self {
            Self::LinkedFanoutPrimary => "linked_fanout_primary",
            Self::LinkedFanoutGroup => "linked_fanout_group",
            Self::DrawReplacementAll => "draw_replacement_all",
            Self::DrawReplacementMatch => "draw_replacement_match",
            Self::WaterbendCost => "waterbend_cost",
            Self::ExiledAggregate => "__sentence_helper_exiled_aggregate",
        }
    }

    pub fn key(self, ordinal: impl std::fmt::Display) -> TagKey {
        declared(TagKey::new(format!("{}_{ordinal}", self.stem())))
    }

    pub fn key_in_scope(self, scope: impl std::fmt::Display) -> TagKey {
        declared(match self {
            Self::DrawReplacementAll => TagKey::new(format!("draw_replacement_{scope}_all")),
            Self::DrawReplacementMatch => TagKey::new(format!("draw_replacement_{scope}_match")),
            _ => self.key(scope),
        })
    }
}

/// Compiler-level semantic classes for generated collection bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerTagClass {
    RevealedCollection,
    SearchedCollection,
    ExiledCollection,
    SentenceHelperExiledCollection,
    SentenceHelperConsultMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerProvenanceTag {
    ParticipantChoice { line: usize, start: usize },
}

impl CompilerProvenanceTag {
    pub fn key(self) -> TagKey {
        declared({
        match self {
            Self::ParticipantChoice { line, start } => {
                TagKey::new(format!("participant_choice_l{line}_s{start}"))
            }
        }
        })
    }
}

impl CompilerTagClass {
    pub fn contains(self, tag: &TagKey) -> bool {
        let value = tag.as_str();
        match self {
            Self::RevealedCollection => {
                value.starts_with("revealed")
                    || value.starts_with("__sentence_helper_revealed")
                    || value == CompilerReferenceTag::RevealedThisWay.as_str()
            }
            Self::SearchedCollection => value.starts_with("searched"),
            Self::ExiledCollection => {
                value.starts_with("exiled_")
                    || Self::SentenceHelperExiledCollection.contains(tag)
                    || value == CompilerReferenceTag::SourceExiled.as_str()
            }
            Self::SentenceHelperExiledCollection => value.starts_with("__sentence_helper_exiled"),
            Self::SentenceHelperConsultMatch => {
                value.starts_with("__sentence_helper_consult_match")
            }
        }
    }
}

/// Stable compiler-owned identities for recurring semantic bindings.
///
/// Grammar and reference resolution use these variants instead of inventing
/// string keys at individual recognition sites. Conversion to the runtime's
/// dynamic [`TagKey`] is confined to this explicit materialization boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerReferenceTag {
    AdditionalCostObject,
    ThisWaySacrificed,
    PriorExiledCard,
    RevealedThisWay,
    SourceObject,
    Exploited,
    Exploiter,
    ManifestDreadGraveyard,
    ManaPaidObject,
    AttackingGroup,
    ZoneChangeGroup,
    InitiativeHolder,
    PreviousIteratedObjects,
    CastModifiedCreatures,
    CastControlledObjects,
    Triggering,
    Enchanted,
    Equipped,
    DivvySource,
    DivvyChosen,
    DivvyPile,
    LastRevealed,
    Sacrificed0,
    Targeted0,
    Targeted1,
    Keep,
    Damaged0,
    Damaged,
    TriggeringSource,
    TriggeringPermanentSpell,
    SearchLibrarySlotsProgress,
    PublicRevealed,
    ChosenObjects,
    SacrificeCost0,
    TapCost0,
    ModularTriggeringObject,
    ExaltedAttacker,
    DiscardedThisWay,
    DiscardedCost,
    ConvokedThisSpell,
    ChosenForEachPlayer,
    RevealedLibrary,
    It,
    DelegatedLibraryChooser,
    ChosenName,
    Chosen,
    VotedWithYou,
    VotedAgainstYou,
    SearchedOutsideGame,
    SearchedMultiZone,
    Searched,
    SameNameReference,
    SaddledItThisTurn,
    CrewedItThisTurn,
    RevealUntilLandRevealed,
    RevealUntilLandMatched,
    ReturnedControlLoss,
    Rest,
    PhaseOutSelection,
    OutsideGameOrExileSelected,
    OathRevealed,
    OathCreature,
    MultiZoneSearchChosen,
    LivingWeaponCreated,
    JunkExiledCard,
    JointDiscardOrSacrifice,
    IterativeLibraryExiled,
    IterativeLibraryCurrent,
    HideawayLooked,
    HideawayExiled,
    GraftEnteredCreature,
    ForMirrodinCreated,
    ExchangePlayerOne,
    ExchangePlayerTwo,
    ExchangeCreaturesOne,
    ExchangeCreaturesTwo,
    EachPlayerShuffled,
    EachPlayerQualifyingShuffled,
    EachPlayerConsultRevealed,
    EachPlayerConsultMatched,
    ControllerConsultRevealed,
    ControllerConsultMatched,
    DivvyOpponent,
    DemonstrateOpponent,
    Blocking,
    Blocked,
    ChosenDiscardingOpponent,
    ChosenCounteredExileSpell,
    BeheldCost0,
    BeheldChosenType,
    GiftedPlayer,
    WhereXCommanderManaValue,
    SourceExiled,
    MillProbe,
    EachPlayerRevealedThisWay,
    EachGraveyardChosen,
    DrawnRevealedCard,
    DelayedOwnedExiledChoice,
    CostExiledTop,
    CopiedStackObject,
    ChosenHandSpellToCast,
    ChosenCastFromGraveyard,
    ChosenCastFromAmong,
    AbilityControllerTargetChoice,
    OpponentTargetChoice,
    VoteWinners,
    VotedObjects,
    TokenDynamicThatCard,
    ConditionCollectionChoice,
    DamagedThisWay,
    ReturnedSourceExiled,
    TwistYourCreatures,
    TwistOpponentCreatures,
    PluralAntecedentCards,
    TappedThisWayGroup,
    OtherAttacker,
}

impl CompilerReferenceTag {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdditionalCostObject => "__additional_cost_object__",
            Self::ThisWaySacrificed => "__this_way_sacrificed__",
            Self::PriorExiledCard => "__prior_exiled_card__",
            Self::RevealedThisWay => "__revealed_this_way__",
            Self::SourceObject => "__source_object__",
            Self::Exploited => "exploited",
            Self::Exploiter => "exploiter",
            Self::ManifestDreadGraveyard => "__manifest_dread_graveyard__",
            Self::ManaPaidObject => "__mana_paid_object__",
            Self::AttackingGroup => "__attacking_group__",
            Self::ZoneChangeGroup => "__zone_change_group__",
            Self::InitiativeHolder => "__initiative_holder__",
            Self::PreviousIteratedObjects => "__previous_iterated_objects__",
            Self::CastModifiedCreatures => "__cast_modified_creatures__",
            Self::CastControlledObjects => "__cast_controlled_objects__",
            Self::Triggering => "triggering",
            Self::Enchanted => "enchanted",
            Self::Equipped => "equipped",
            Self::DivvySource => "divvy_source",
            Self::DivvyChosen => "divvy_chosen",
            Self::DivvyPile => "divvy_pile",
            Self::LastRevealed => "__last_revealed__",
            Self::Sacrificed0 => "sacrificed_0",
            Self::Targeted0 => "targeted_0",
            Self::Targeted1 => "targeted_1",
            Self::Keep => "keep",
            Self::Damaged0 => "damaged_0",
            Self::Damaged => "damaged",
            Self::TriggeringSource => "triggering_source",
            Self::TriggeringPermanentSpell => "triggering_permanent_spell",
            Self::SearchLibrarySlotsProgress => "search_library_slots_progress",
            Self::PublicRevealed => "__public_revealed",
            Self::ChosenObjects => "__chosen_objects__",
            Self::SacrificeCost0 => "sacrifice_cost_0",
            Self::TapCost0 => "tap_cost_0",
            Self::ModularTriggeringObject => "modular_triggering_object",
            Self::ExaltedAttacker => "exalted_attacker",
            Self::DiscardedThisWay => "discarded_this_way",
            Self::DiscardedCost => "discarded_cost",
            Self::ConvokedThisSpell => "convoked_this_spell",
            Self::ChosenForEachPlayer => "chosen_for_each_player",
            Self::RevealedLibrary => "__revealed_library__",
            Self::It => "__it__",
            Self::DelegatedLibraryChooser => "__delegated_library_chooser__",
            Self::ChosenName => "__chosen_name__",
            Self::Chosen => "chosen",
            Self::VotedWithYou => "voted_with_you",
            Self::VotedAgainstYou => "voted_against_you",
            Self::SearchedOutsideGame => "searched_outside_game",
            Self::SearchedMultiZone => "searched_multi_zone",
            Self::Searched => "searched",
            Self::SameNameReference => "same_name_reference",
            Self::SaddledItThisTurn => "saddled_it_this_turn",
            Self::CrewedItThisTurn => "crewed_it_this_turn",
            Self::RevealUntilLandRevealed => "reveal_until_land_revealed",
            Self::RevealUntilLandMatched => "reveal_until_land_matched",
            Self::ReturnedControlLoss => "returned_control_loss",
            Self::Rest => "rest",
            Self::PhaseOutSelection => "phase_out_selection",
            Self::OutsideGameOrExileSelected => "outside_game_or_exile_selected",
            Self::OathRevealed => "oath_revealed",
            Self::OathCreature => "oath_creature",
            Self::MultiZoneSearchChosen => "multi_zone_search_chosen",
            Self::LivingWeaponCreated => "living_weapon_created",
            Self::JunkExiledCard => "junk_exiled_card",
            Self::JointDiscardOrSacrifice => "joint_discard_or_sacrifice",
            Self::IterativeLibraryExiled => "iterative_library_exiled",
            Self::IterativeLibraryCurrent => "iterative_library_current",
            Self::HideawayLooked => "hideaway_looked",
            Self::HideawayExiled => "hideaway_exiled",
            Self::GraftEnteredCreature => "graft_entered_creature",
            Self::ForMirrodinCreated => "for_mirrodin_created",
            Self::ExchangePlayerOne => "exchange_player_one",
            Self::ExchangePlayerTwo => "exchange_player_two",
            Self::ExchangeCreaturesOne => "exchange_creatures_one",
            Self::ExchangeCreaturesTwo => "exchange_creatures_two",
            Self::EachPlayerShuffled => "each_player_shuffled",
            Self::EachPlayerQualifyingShuffled => "each_player_qualifying_shuffled",
            Self::EachPlayerConsultRevealed => "each_player_consult_revealed",
            Self::EachPlayerConsultMatched => "each_player_consult_matched",
            Self::ControllerConsultRevealed => "controller_consult_revealed",
            Self::ControllerConsultMatched => "controller_consult_matched",
            Self::DivvyOpponent => "divvy_opponent",
            Self::DemonstrateOpponent => "demonstrate_opponent",
            Self::Blocking => "blocking",
            Self::Blocked => "blocked",
            Self::ChosenDiscardingOpponent => "chosen_discarding_opponent",
            Self::ChosenCounteredExileSpell => "chosen_countered_exile_spell",
            Self::BeheldCost0 => "beheld_cost_0",
            Self::BeheldChosenType => "beheld_chosen_type",
            Self::GiftedPlayer => "gifted_player",
            Self::WhereXCommanderManaValue => "__where_x_commander_mana_value",
            Self::SourceExiled => "__source_exiled__",
            Self::MillProbe => "__mill_probe__",
            Self::EachPlayerRevealedThisWay => "__each_player_revealed_this_way",
            Self::EachGraveyardChosen => "__each_graveyard_chosen",
            Self::DrawnRevealedCard => "__drawn_revealed_card__",
            Self::DelayedOwnedExiledChoice => "__delayed_owned_exiled_choice",
            Self::CostExiledTop => "__cost_exiled_top__",
            Self::CopiedStackObject => "__copied_stack_object__",
            Self::ChosenHandSpellToCast => "__chosen_hand_spell_to_cast",
            Self::ChosenCastFromGraveyard => "__chosen_cast_from_graveyard",
            Self::ChosenCastFromAmong => "__chosen_cast_from_among",
            Self::AbilityControllerTargetChoice => "__ability_controller_target_choice_0",
            Self::OpponentTargetChoice => "__opponent_target_choice_1",
            Self::VoteWinners => "__vote_winners__",
            Self::VotedObjects => "__voted_objects__",
            Self::TokenDynamicThatCard => "__token_dynamic_that_card",
            Self::ConditionCollectionChoice => "__condition_collection_choice",
            Self::DamagedThisWay => "damaged_0",
            Self::ReturnedSourceExiled => "source_exiled_returned",
            Self::TwistYourCreatures => "__twist_your_creatures__",
            Self::TwistOpponentCreatures => "__twist_opponent_creatures__",
            Self::PluralAntecedentCards => "plural_antecedent_cards",
            Self::TappedThisWayGroup => "tapped_this_way_group",
            Self::OtherAttacker => "other_attacker",
        }
    }

    pub fn key(self) -> TagKey {
        TagKey::new(self.as_str())
    }

    pub fn matches(self, tag: &TagKey) -> bool {
        tag.as_str() == self.as_str()
    }

    /// The symbol role and domain this reference tag stands for.
    pub fn symbol_role(self) -> (ReferenceRole, ObjectDomain) {
        use ObjectDomain as D;
        use ReferenceRole as R;
        match self {
            Self::Triggering
            | Self::TriggeringSource
            | Self::TriggeringPermanentSpell
            | Self::ModularTriggeringObject => (R::Triggering, D::Object),
            Self::Targeted0 | Self::Targeted1 | Self::AbilityControllerTargetChoice => {
                (R::Target, D::Object)
            }
            Self::Chosen
            | Self::ChosenObjects
            | Self::DivvyChosen
            | Self::DivvyPile
            | Self::MultiZoneSearchChosen
            | Self::ChosenHandSpellToCast
            | Self::ChosenCastFromGraveyard
            | Self::ChosenCastFromAmong
            | Self::ChosenCounteredExileSpell
            | Self::EachGraveyardChosen
            | Self::DelayedOwnedExiledChoice
            | Self::PhaseOutSelection
            | Self::OutsideGameOrExileSelected
            | Self::BeheldChosenType => (R::Chosen, D::Object),
            Self::ChosenName | Self::WhereXCommanderManaValue => (R::Chosen, D::Value),
            Self::ChosenForEachPlayer
            | Self::ChosenDiscardingOpponent
            | Self::DivvyOpponent
            | Self::DemonstrateOpponent
            | Self::GiftedPlayer
            | Self::InitiativeHolder
            | Self::DelegatedLibraryChooser
            | Self::VotedWithYou
            | Self::VotedAgainstYou
            | Self::ExchangePlayerOne
            | Self::ExchangePlayerTwo => (R::Chosen, D::Player),
            Self::Sacrificed0 | Self::ThisWaySacrificed | Self::SacrificeCost0 | Self::JointDiscardOrSacrifice => {
                (R::Sacrificed, D::Object)
            }
            Self::DiscardedThisWay | Self::DiscardedCost => (R::Discarded, D::Card),
            Self::RevealedThisWay
            | Self::LastRevealed
            | Self::PublicRevealed
            | Self::RevealedLibrary
            | Self::OathRevealed
            | Self::RevealUntilLandRevealed
            | Self::RevealUntilLandMatched
            | Self::EachPlayerRevealedThisWay
            | Self::EachPlayerConsultRevealed
            | Self::EachPlayerConsultMatched
            | Self::ControllerConsultRevealed
            | Self::ControllerConsultMatched
            | Self::DrawnRevealedCard
            | Self::HideawayLooked => (R::Revealed, D::Card),
            Self::Searched | Self::SearchedOutsideGame | Self::SearchedMultiZone | Self::SearchLibrarySlotsProgress => {
                (R::Searched, D::Card)
            }
            Self::PriorExiledCard
            | Self::SourceExiled
            | Self::HideawayExiled
            | Self::JunkExiledCard
            | Self::IterativeLibraryExiled
            | Self::CostExiledTop
            | Self::ManifestDreadGraveyard => (R::Exiled, D::Card),
            Self::LivingWeaponCreated | Self::ForMirrodinCreated => (R::Created, D::Object),
            Self::CopiedStackObject => (R::Copied, D::Spell),
            Self::PreviousIteratedObjects | Self::IterativeLibraryCurrent => (R::Iteration, D::Object),
            Self::ManaPaidObject | Self::TapCost0 | Self::ConvokedThisSpell | Self::AdditionalCostObject | Self::BeheldCost0 => {
                (R::CostPaid, D::Object)
            }
            Self::SourceObject => (R::Source, D::Object),
            _ => (R::Affected, D::Object),
        }
    }

    /// Mint this tag's key and declare the symbol it stands for in the
    /// enclosing reference scope (see `ironsmith_compiler_ast::reference_ledger`).
    pub fn bind(self) -> TagKey {
        let key = self.key();
        let (role, domain) = self.symbol_role();
        ironsmith_compiler_ast::reference_ledger::note_minted(key.clone(), role, domain, Cardinality::Any);
        key
    }
}
