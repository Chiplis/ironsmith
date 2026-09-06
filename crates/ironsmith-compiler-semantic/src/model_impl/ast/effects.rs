use ironsmith_compiler_ast::TagRef;
use ironsmith_core::tag::TagKeyWalk;

#[path = "effects/delayed.rs"]
mod delayed;
pub use delayed::*;
#[path = "effects/for_each.rs"]
mod for_each;
pub use for_each::*;
#[path = "effects/choices.rs"]
mod choices;
pub use choices::*;
#[path = "effects/votes.rs"]
mod votes;
pub use votes::*;
#[path = "effects/conditionals.rs"]
mod conditionals;
pub use conditionals::*;
#[path = "effects/permissions.rs"]
mod permissions;
pub use permissions::*;

use super::*;
use crate::model::document_program::CompilerDocumentProgramAst;

/// One mode of an `EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf)` modal choice: a label shown to the
/// player and the effects that resolve when that mode is chosen.
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ChooseOneModeAst {
    pub description: String,
    pub effects: Vec<EffectAst>,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum EffectAst {
    /// Permissions: see [`PermissionEffectAst`].
    Permissions(PermissionEffectAst),
    /// Conditionals: see [`ConditionalEffectAst`].
    Conditionals(ConditionalEffectAst),
    /// Votes: see [`VoteEffectAst`].
    Votes(VoteEffectAst),
    /// ObjectChoices: see [`ObjectChoiceEffectAst`].
    ObjectChoices(ObjectChoiceEffectAst),
    /// ForEach: see [`ForEachEffectAst`].
    ForEach(ForEachEffectAst),
    /// Delayed: see [`DelayedEffectAst`].
    Delayed(DelayedEffectAst),
    /// A grammar-resolved effect program with explicit conjunction,
    /// disjunction, ordering, dependency, and carry semantics.
    Coordination(CoordinationAst),
    /// Compiler-owned conditions, replacements, prevention, permissions,
    /// durations, delayed execution, and nested programs.
    ControlFlow(Box<CompilerControlFlowAst>),
    /// One typed repeated program with a scope-owned iterator symbol.
    Iteration(Box<CompilerIterationAst>),
    /// One typed vote whose individual choices and aggregate tally are bound.
    Vote(Box<CompilerVoteAst>),
    /// A document sequence of typed statements with explicit reference edges.
    DocumentProgram(Box<CompilerDocumentProgramAst>),
    SubjectVerb(SubjectVerbEffectAst),
    SolveCase,
    RestartGame {
        cards_left_in_exile: Option<ChooseSpec>,
        source_surface: Option<SourceReferenceSurface>,
    },
    PlaySubgame {
        /// Effects performed in the resumed parent game for each participant
        /// who did not win the child game.
        nonwinner_effects: Vec<EffectAst>,
    },
    Sequence {
        effects: Vec<EffectAst>,
    },
    /// Effects separated by an authored same-sentence `, then` connective.
    /// This is typed punctuation provenance; every child still executes in
    /// ordinary sequential scope.
    CommaThen {
        effects: Vec<EffectAst>,
    },
    /// Effects authored as one Oracle sentence inside a multi-sentence
    /// resolution. This is compiler-only grouping metadata: preparation
    /// lowers each top-level source sentence into its own resolution segment
    /// while preserving ordinary reference flow between the segments.
    SourceSentence {
        effects: Vec<EffectAst>,
        /// Whether the sentence begins with the explicit ordering connective
        /// "Then". This is typed punctuation provenance, not retained source
        /// text.
        leading_then: bool,
        /// Whether the sentence begins with the explicit participant-ordering
        /// connective "starting with you".
        starting_with_controller: bool,
    },
    /// Effects printed as one coordinated Oracle clause (for example,
    /// "destroy target artifact and target enchantment"). This remains a
    /// real typed boundary through lowering so rendering never has to infer
    /// coordination from unrelated adjacent effects.
    Coordinated {
        effects: Vec<EffectAst>,
        leading_duration: bool,
        /// This exact wrapper was introduced from an explicit leading
        /// If/When-result clause, rather than by an ordinary coordinated
        /// specialist parser.
        result_conjunction: bool,
    },
    /// Presentation wrapper for a named numeric-result row such as
    /// `1 | Trapped! — ...`. The label does not affect branch execution, but
    /// remains tied to the exact typed result predicate.
    ResultBranchLabel {
        label: String,
        effects: Vec<EffectAst>,
    },
    ManaRestricted {
        effects: Vec<EffectAst>,
        restrictions: Vec<crate::model::compiler_semantic::CompilerManaUsageRestriction>,
    },
    SelfReplacement {
        predicate: PredicateAst,
        if_true: Vec<EffectAst>,
        if_false: Vec<EffectAst>,
        attach_to_previous_ability: bool,
    },
    /// Lower `effect` (which must lower to a single runtime effect) and apply
    /// `tag_all(tag)` to it, tagging every object the effect affects. Lowers to
    /// `Effect::tag_all`.
    TagAffected {
        effect: Box<EffectAst>,
        tag: TagRef,
    },
    DirectionalAdjacentPlayerControl {
        filter: ObjectFilter,
        left_option: String,
        right_option: String,
    },
    /// Moves every object tagged `tag` to `zone`, preserving each object's
    /// controller. Lowers to `for_each_tagged(tag, [move(Iterated, zone)])`.
    /// Unlike a hand-written `ForEachTagged` whose body references `it`, this
    /// keeps the iterated reference internal to lowering, so the iteration does
    /// not surface a bare `it` that would be mistaken for an outer (triggering)
    /// object reference.
    MoveTaggedGroupToZone {
        tag: TagRef,
        zone: Zone,
    },
    /// Binds the most recently looked-at / referenced object collection
    /// (whatever is currently in `last_object_tag`) to the explicit parse-time
    /// tag `into`. This is a lowering-time alias only: it emits no runtime
    /// effect, but lets later composed effects reference the earlier pool via
    /// `into` even after an intervening `ChooseObjects` clobbers
    /// `last_object_tag`. Used to compose the "put some into hand, rest
    /// elsewhere" looked-cards shapes from reusable primitives.
    SnapshotLastObjectTag {
        into: TagRef,
    },
}

impl EffectAst {
    pub fn subject_verb(
        role: SubjectVerbRoleAst,
        player: PlayerAst,
        action: SubjectVerbActionAst,
    ) -> Self {
        Self::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { role, player },
            action,
        })
    }

    pub fn subject_verb_draw_for_each_tagged_matching(
        player: PlayerAst,
        tag: TagRef,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::DrawForEachTaggedMatching { tag, filter }),
        )
    }

    pub fn subject_verb_grant_next_spell_ability_this_turn(
        player: PlayerAst,
        filter: ObjectFilter,
        ability: GrantedAbilityAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantNextSpellAbilityThisTurn {
                filter,
                ability: Box::new(ability),
            }),
        )
    }

    pub fn subject_verb_may_move_to_zone(player: PlayerAst, target: TargetAst, zone: Zone) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MayMoveToZone { target, zone }),
        )
    }

    /// Composes "choose up to N of the looked-at cards into hand, put the rest
    /// on the bottom of the library" from reusable primitives, mirroring the
    /// runtime effects the retired `PutSomeIntoHandRestOnBottomOfLibrary` recipe
    /// lowered to. `looked_tag` names the prior looked-at pool; callers that
    /// emit the look themselves should pass a fresh tag, while standalone
    /// follow-ups should snapshot the prior `last_object_tag` via
    /// `SnapshotLastObjectTag` (handled here) and pass `crate::tag::CompilerReferenceTag::It.as_str()`.
    pub fn compose_put_some_into_hand_rest_on_bottom_of_library(
        player: PlayerAst,
        count: ChoiceCount,
        looked_tag: TagRef,
        chosen_tag: TagRef,
        order: LibraryBottomOrderAst,
    ) -> Vec<Self> {
        let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
        choose_filter.zone = Some(Zone::Library);
        vec![
            Self::SnapshotLastObjectTag {
                into: looked_tag.clone(),
            },
            Self::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                filter: choose_filter,
                count,
                player,
                tag: chosen_tag.clone(),
                zone: Zone::Library,
            }),
            Self::MoveTaggedGroupToZone {
                tag: chosen_tag.clone(),
                zone: Zone::Hand,
            },
            Self::subject_verb_put_tagged_remainder_on_bottom_of_library(
                looked_tag,
                Some(chosen_tag),
                order,
                player,
            ),
        ]
    }

    /// Composes "choose up to N of the looked-at cards for the top of the
    /// library, put the rest on the bottom" while preserving the stated
    /// random-versus-chosen ordering of the remainder.
    pub fn compose_put_some_on_top_rest_on_bottom_of_library(
        player: PlayerAst,
        count: ChoiceCount,
        looked_tag: TagRef,
        chosen_tag: TagRef,
        order: LibraryBottomOrderAst,
    ) -> Vec<Self> {
        let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
        choose_filter.zone = Some(Zone::Library);
        vec![
            Self::SnapshotLastObjectTag {
                into: looked_tag.clone(),
            },
            Self::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                filter: choose_filter,
                count,
                player,
                tag: chosen_tag.clone(),
                zone: Zone::Library,
            }),
            Self::ForEach(ForEachEffectAst::ForEachTagged {
                tag: chosen_tag.clone(),
                effects: vec![Self::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    Zone::Library,
                    true,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }),
            Self::subject_verb_put_tagged_remainder_on_bottom_of_library(
                looked_tag,
                Some(chosen_tag),
                order,
                player,
            ),
        ]
    }

    /// Composes "choose N of the looked-at cards into hand, put the rest into
    /// the graveyard" from reusable primitives, mirroring the runtime effects
    /// the retired `PutSomeIntoHandRestIntoGraveyard` recipe lowered to: a
    /// per-looked-card `ForEachTagged` that keeps cards in the chosen group and
    /// moves the remainder to the graveyard. See
    /// `compose_put_some_into_hand_rest_on_bottom_of_library` for the
    /// `looked_tag` contract.
    pub fn compose_put_some_into_hand_rest_into_graveyard(
        player: PlayerAst,
        count: ChoiceCount,
        looked_tag: TagRef,
        chosen_tag: TagRef,
    ) -> Vec<Self> {
        Self::compose_put_some_to_zone_rest_to_zone(
            player,
            count,
            looked_tag,
            chosen_tag,
            Zone::Hand,
            Zone::Graveyard,
        )
    }

    pub fn compose_put_some_to_zone_rest_to_zone(
        player: PlayerAst,
        count: ChoiceCount,
        looked_tag: TagRef,
        chosen_tag: TagRef,
        chosen_zone: Zone,
        rest_zone: Zone,
    ) -> Vec<Self> {
        let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
        choose_filter.zone = Some(Zone::Library);

        vec![
            Self::SnapshotLastObjectTag {
                into: looked_tag.clone(),
            },
            Self::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                filter: choose_filter,
                count,
                player,
                tag: chosen_tag.clone(),
                zone: Zone::Library,
            }),
            Self::MoveTaggedGroupToZone {
                tag: chosen_tag.clone(),
                zone: chosen_zone,
            },
            Self::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::Implicit,
                SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone {
                    tag: looked_tag,
                    keep_tagged: chosen_tag,
                    zone: rest_zone,
                    surface: ironsmith_core::LibraryRemainderSurface::Rest,
                }),
            ),
        ]
    }

    pub fn subject_verb_grant_protection_choice(
        target: TargetAst,
        chooser: PlayerAst,
        allow_colorless: bool,
        allow_artifacts: bool,
        choose_card_type: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantProtectionChoice {
                target,
                chooser,
                allow_colorless,
                allow_artifacts,
                choose_card_type,
            }),
        )
    }

    pub fn subject_verb_prevent_all_combat_damage(duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamage { duration }),
        )
    }

    pub fn subject_verb_assign_no_combat_damage(source: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::AssignNoCombatDamage { source, duration }),
        )
    }

    pub fn subject_verb_prevent_all_combat_damage_from_source(
        source: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb_prevent_all_combat_damage_from_source_with_surface(
            source, duration, false,
        )
    }

    pub fn subject_verb_prevent_all_combat_damage_source_would_deal(
        source: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb_prevent_all_combat_damage_from_source_with_surface(
            source, duration, true,
        )
    }

    fn subject_verb_prevent_all_combat_damage_from_source_with_surface(
        source: TargetAst,
        duration: Until,
        source_would_deal_surface: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSource {
                duration,
                source,
                source_would_deal_surface,
            }),
        )
    }

    pub fn subject_verb_prevent_all_combat_damage_from_source_filter(
        source_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSourceFilter {
                duration,
                source_filter,
                excluded_source_target: None,
            }),
        )
    }

    pub fn subject_verb_prevent_all_combat_damage_from_source_filter_excluding_target(
        source_filter: ObjectFilter,
        excluded_source_target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSourceFilter {
                duration,
                source_filter,
                excluded_source_target: Some(excluded_source_target),
            }),
        )
    }

    pub fn subject_verb_prevent_all_combat_damage_to_players(duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageToPlayers { duration }),
        )
    }

    pub fn subject_verb_prevent_all_combat_damage_to_you(duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageToYou { duration }),
        )
    }

    pub fn subject_verb_prevent_next_time_damage(
        source: PreventNextTimeDamageSourceAst,
        target: PreventNextTimeDamageTargetAst,
    ) -> Self {
        Self::subject_verb_prevent_next_time_damage_with_reflection(source, target, false)
    }

    pub fn subject_verb_prevent_next_time_damage_with_reflection(
        source: PreventNextTimeDamageSourceAst,
        target: PreventNextTimeDamageTargetAst,
        reflect_damage_to_source_controller: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventNextTimeDamage {
                source,
                target,
                reflect_damage_to_source_controller,
                follow_up_effects: Vec::new(),
            }),
        )
    }

    pub fn subject_verb_replace_next_damage_to_target(
        target: TargetAst,
        damage_target_tag: TagRef,
        replacement_effects: Vec<EffectAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::ReplaceNextDamageToTarget {
                target,
                damage_target_tag,
                replacement_effects,
            }),
        )
    }

    pub fn subject_verb_prevent_damage(amount: Value, target: TargetAst, duration: Until) -> Self {
        Self::subject_verb_prevent_damage_with_source_choice(amount, target, duration, false)
    }

    pub fn subject_verb_prevent_damage_with_source_choice(
        amount: Value,
        target: TargetAst,
        duration: Until,
        source_of_your_choice: bool,
    ) -> Self {
        Self::subject_verb_prevent_damage_with_options(
            amount,
            target,
            duration,
            source_of_your_choice,
            false,
            Vec::new(),
        )
    }

    pub fn subject_verb_prevent_damage_with_options(
        amount: Value,
        target: TargetAst,
        duration: Until,
        source_of_your_choice: bool,
        protect_you_and_permanents_you_control: bool,
        follow_up_effects: Vec<EffectAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamage {
                amount,
                target,
                duration,
                source_of_your_choice,
                protect_you_and_permanents_you_control,
                follow_up_effects,
            }),
        )
    }

    pub fn subject_verb_prevent_all_damage_to_target(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb_prevent_all_damage_to_target_with_source_choice(target, duration, false)
    }

    pub fn subject_verb_prevent_all_damage_to_target_with_source_choice(
        target: TargetAst,
        duration: Until,
        source_of_your_choice: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget {
                target,
                duration,
                source_of_your_choice,
                source_choice_shares_activation_mana_color: false,
                source_target: None,
            }),
        )
    }

    pub fn subject_verb_prevent_all_damage_to_target_with_mana_color_source_choice(
        target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget {
                target,
                duration,
                source_of_your_choice: true,
                source_choice_shares_activation_mana_color: true,
                source_target: None,
            }),
        )
    }

    pub fn subject_verb_prevent_all_damage_to_target_from_target_source(
        target: TargetAst,
        source_target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget {
                target,
                duration,
                source_of_your_choice: false,
                source_choice_shares_activation_mana_color: false,
                source_target: Some(source_target),
            }),
        )
    }

    pub fn subject_verb_prevent_all_damage_to_target_from_source_filter(
        target: TargetAst,
        source_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter {
                target,
                duration,
                source_filter,
            }),
        )
    }

    pub fn subject_verb_prevent_all_damage_from_source_filter(
        source_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageFromSourceFilter {
                duration,
                source_filter,
            }),
        )
    }

    pub fn subject_verb_prevent_damage_to_target_put_counters(
        amount: Option<Value>,
        target: TargetAst,
        duration: Until,
        counter_type: CounterType,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageToTargetPutCounters {
                amount,
                target,
                duration,
                counter_type,
            }),
        )
    }

    pub fn subject_verb_prevent_damage_each(
        amount: Value,
        filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageEach {
                amount,
                filter,
                duration,
            }),
        )
    }

    pub fn subject_verb_copy_spell(
        target: TargetAst,
        count: Value,
        player: PlayerAst,
        may_choose_new_targets: bool,
        choose_new_target_singular: bool,
        removed_supertypes: Vec<crate::types::Supertype>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Stack(StackActionAst::CopySpell {
                target,
                target_reference_kind: None,
                target_reference_pronoun: false,
                all_matches: false,
                count,
                count_surface: None,
                player,
                may_choose_new_targets,
                choose_new_target_singular,
                removed_supertypes,
                set_colors: None,
                added_card_types: Vec::new(),
                added_subtypes: Vec::new(),
                set_base_power_toughness: None,
            }),
        )
    }

    pub fn with_copy_count_surface(
        mut self,
        surface: ironsmith_core::effect::CopyCountSurface,
    ) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Stack(StackActionAst::CopySpell { count_surface, .. }),
            ..
        }) = &mut self
        {
            *count_surface = Some(surface);
        }
        self
    }

    /// Preserve the authored kind of a stack-object back-reference.
    pub fn with_copy_target_reference_kind(mut self, kind: crate::filter::StackObjectKind) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Stack(StackActionAst::CopySpell {
                    target_reference_kind,
                    ..
                }),
            ..
        }) = &mut self
        {
            *target_reference_kind = Some(kind);
        }
        self
    }

    /// Preserve an authored pronoun independently of the resolved target tag.
    pub fn with_copy_target_reference_pronoun(mut self, pronoun: bool) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Stack(StackActionAst::CopySpell {
                    target_reference_pronoun,
                    ..
                }),
            ..
        }) = &mut self
        {
            *target_reference_pronoun = pronoun;
        }
        self
    }

    /// Preserve the set quantifier on a spell/ability-copy action.
    pub fn with_copy_all_matches(mut self, all_matches: bool) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Stack(StackActionAst::CopySpell {
                    all_matches: action_all_matches,
                    ..
                }),
            ..
        }) = &mut self
        {
            *action_all_matches = all_matches;
        }
        self
    }

    /// Preserve card types introduced by a copy exception.
    pub fn with_copy_added_card_types(mut self, added_card_types: Vec<CardType>) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Stack(StackActionAst::CopySpell {
                    added_card_types: action_added_card_types,
                    ..
                }),
            ..
        }) = &mut self
        {
            *action_added_card_types = added_card_types;
        }
        self
    }

    /// Preserve subtypes introduced by a copy exception.
    pub fn with_copy_added_subtypes(mut self, added_subtypes: Vec<Subtype>) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Stack(StackActionAst::CopySpell {
                    added_subtypes: action_added_subtypes,
                    ..
                }),
            ..
        }) = &mut self
        {
            *action_added_subtypes = added_subtypes;
        }
        self
    }

    /// Preserve fixed base power/toughness introduced by a copy exception.
    pub fn with_copy_set_base_power_toughness(
        mut self,
        set_base_power_toughness: Option<(i32, i32)>,
    ) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Stack(StackActionAst::CopySpell {
                    set_base_power_toughness: action_set_base_power_toughness,
                    ..
                }),
            ..
        }) = &mut self
        {
            *action_set_base_power_toughness = set_base_power_toughness;
        }
        self
    }

    /// Preserve colors set by a copy exception.
    pub fn with_copy_set_colors(mut self, set_colors: Option<crate::color::ColorSet>) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Stack(StackActionAst::CopySpell {
                    set_colors: action_set_colors,
                    ..
                }),
            ..
        }) = &mut self
        {
            *action_set_colors = set_colors;
        }
        self
    }

    pub fn subject_verb_copy_spell_for_each_target(
        target: TargetAst,
        object_filter: Option<ObjectFilter>,
        player_filter: Option<PlayerFilter>,
        player: PlayerAst,
        exclude_current_targets: bool,
        removed_supertypes: Vec<crate::types::Supertype>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Stack(StackActionAst::CopySpellForEachTarget {
                target,
                object_filter,
                player_filter,
                player,
                exclude_current_targets,
                removed_supertypes,
            }),
        )
    }

    pub fn subject_verb_scale_x_value(target: TargetAst, multiplier: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Stack(StackActionAst::ScaleXValue { target, multiplier }),
        )
    }

    pub fn subject_verb_put_tagged_remainder_on_bottom_of_library(
        tag: TagRef,
        keep_tagged: Option<TagRef>,
        order: LibraryBottomOrderAst,
        player: PlayerAst,
    ) -> Self {
        Self::subject_verb_put_tagged_remainder_on_bottom_of_library_with_surface(
            tag,
            keep_tagged,
            order,
            player,
            ironsmith_core::LibraryRemainderSurface::Rest,
        )
    }

    pub fn subject_verb_put_tagged_remainder_on_bottom_of_library_with_surface(
        tag: TagRef,
        keep_tagged: Option<TagRef>,
        order: LibraryBottomOrderAst,
        player: PlayerAst,
        surface: ironsmith_core::LibraryRemainderSurface,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary {
                tag,
                keep_tagged,
                order,
                player,
                surface,
            }),
        )
    }

    pub fn subject_verb_cast_tagged(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        as_copy: bool,
        without_paying_mana_cost: bool,
        cost_reduction: Option<ManaCost>,
    ) -> Self {
        Self::subject_verb_cast_tagged_with_additional_cost(
            tag,
            player,
            allow_land,
            as_copy,
            without_paying_mana_cost,
            None,
            cost_reduction,
        )
    }

    pub fn subject_verb_cast_tagged_with_additional_cost(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        as_copy: bool,
        without_paying_mana_cost: bool,
        additional_mana_cost: Option<ManaCost>,
        cost_reduction: Option<ManaCost>,
    ) -> Self {
        Self::subject_verb_cast_tagged_with_additional_cost_and_mana_spend_mode(
            tag,
            player,
            allow_land,
            as_copy,
            without_paying_mana_cost,
            additional_mana_cost,
            cost_reduction,
            ironsmith_core::value_model::ManaSpendMode::Normal,
        )
    }

    pub fn subject_verb_cast_tagged_with_additional_cost_and_mana_spend_mode(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        as_copy: bool,
        without_paying_mana_cost: bool,
        additional_mana_cost: Option<ManaCost>,
        cost_reduction: Option<ManaCost>,
        mana_spend_mode: ironsmith_core::value_model::ManaSpendMode,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Stack(StackActionAst::CastTagged {
                tag,
                player,
                allow_land,
                as_copy,
                copy_cast_reminder_surface: false,
                copy_instruction_surface: None,
                without_paying_mana_cost,
                additional_mana_cost,
                cost_reduction,
                mana_spend_mode,
            }),
        )
    }

    pub fn with_copy_instruction_surface(
        mut self,
        surface: ironsmith_core::effect::CopyInstructionSurface,
    ) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Stack(StackActionAst::CastTagged {
                    copy_instruction_surface,
                    ..
                }),
            ..
        }) = &mut self
        {
            *copy_instruction_surface = Some(surface);
        }
        self
    }

    pub fn may_cast_matching_spell_without_paying_mana_cost(
        player: PlayerAst,
        filter: ObjectFilter,
        zone: Zone,
    ) -> Self {
        Self::Permissions(PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost {
            player,
            zone_owner: player,
            filter,
            zone,
            payment: ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost,
        })
    }

    pub fn may_cast_matching_spell_without_paying_mana_cost_from_zone_owner(
        player: PlayerAst,
        zone_owner: PlayerAst,
        filter: ObjectFilter,
        zone: Zone,
    ) -> Self {
        Self::Permissions(PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost {
            player,
            zone_owner,
            filter,
            zone,
            payment: ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost,
        })
    }

    pub fn may_cast_matching_spell_with_alternative_cost(
        player: PlayerAst,
        filter: ObjectFilter,
        zone: Zone,
        kind: crate::filter::AlternativeCastKind,
    ) -> Self {
        Self::Permissions(PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost {
            player,
            zone_owner: player,
            filter,
            zone,
            payment: ironsmith_core::MayCastMatchingSpellPayment::AlternativeCost(kind),
        })
    }

    pub fn subject_verb_grant_play_tagged_until_end_of_turn(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        allow_any_color_for_cast: impl Into<ironsmith_core::value_model::ManaSpendMode>,
    ) -> Self {
        Self::subject_verb_grant_play_tagged_until_end_of_turn_with_optional_surface(
            tag,
            player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            None,
        )
    }

    pub fn subject_verb_grant_play_tagged_until_end_of_turn_with_optional_surface(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        allow_any_color_for_cast: impl Into<ironsmith_core::value_model::ManaSpendMode>,
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
    ) -> Self {
        let allow_any_color_for_cast = allow_any_color_for_cast.into();
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                while_on_top_of_library: false,
                free_cast_from_current_zone: false,
                until_source_exiles_another: false,
                max_plays: None,
                surface,
            }),
        )
    }

    pub fn subject_verb_grant_play_tagged_until_end_of_turn_from_current_zone_with_optional_surface(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        allow_any_color_for_cast: impl Into<ironsmith_core::value_model::ManaSpendMode>,
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
    ) -> Self {
        let allow_any_color_for_cast = allow_any_color_for_cast.into();
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                while_on_top_of_library: false,
                free_cast_from_current_zone: true,
                until_source_exiles_another: false,
                max_plays: None,
                surface,
            }),
        )
    }

    pub fn subject_verb_grant_play_tagged_until_end_of_turn_while_on_top_of_library(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        allow_any_color_for_cast: impl Into<ironsmith_core::value_model::ManaSpendMode>,
    ) -> Self {
        let allow_any_color_for_cast = allow_any_color_for_cast.into();
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                while_on_top_of_library: true,
                free_cast_from_current_zone: true,
                until_source_exiles_another: false,
                max_plays: None,
                surface: None,
            }),
        )
    }

    pub fn subject_verb_grant_play_tagged_until_source_exiles_another(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        source_surface: SourceReferenceSurface,
        object_surface: ironsmith_core::GrantPlayTaggedObjectSurface,
    ) -> Self {
        let surface = ironsmith_core::GrantPlayTaggedSurface::default()
            .with_object(object_surface)
            .with_until_source_exiles_another(source_surface);
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                tag,
                player,
                allow_land,
                without_paying_mana_cost: false,
                allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode::Normal,
                while_on_top_of_library: false,
                free_cast_from_current_zone: false,
                until_source_exiles_another: true,
                max_plays: None,
                surface: Some(surface),
            }),
        )
    }

    pub fn subject_verb_grant_tagged_spell_alternative_cost_pay_life_by_mana_value_until_end_of_turn(
        tag: TagRef,
        player: PlayerAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                tag,
                player,
            }),
        )
    }

    pub fn subject_verb_grant_play_tagged_until_your_next_turn(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: impl Into<ironsmith_core::value_model::ManaSpendMode>,
    ) -> Self {
        let allow_any_color_for_cast = allow_any_color_for_cast.into();
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
                until_next_end_step: false,
                max_plays: None,
            }),
        )
    }

    pub fn subject_verb_grant_play_tagged_until_your_next_end_step(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: impl Into<ironsmith_core::value_model::ManaSpendMode>,
    ) -> Self {
        let allow_any_color_for_cast = allow_any_color_for_cast.into();
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
                until_next_end_step: true,
                max_plays: None,
            }),
        )
    }

    /// Apply a shared deferred-use limit to a tagged play permission.
    ///
    /// The tagged collection remains intact so the player chooses which card
    /// to play at play/cast time rather than during effect resolution.
    pub fn with_tagged_play_max_plays(mut self, limit: Option<u32>) -> Self {
        if let Self::SubjectVerb(subject_verb) = &mut self {
            match &mut subject_verb.action {
                SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn { max_plays, .. })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn { max_plays, .. }) => {
                    *max_plays = limit;
                }
                _ => {}
            }
        }
        self
    }

    pub fn subject_verb_grant_play_tagged_for_as_long_as_exiled(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        allow_any_color_for_cast: impl Into<ironsmith_core::value_model::ManaSpendMode>,
        filter: Option<ObjectFilter>,
    ) -> Self {
        let allow_any_color_for_cast = allow_any_color_for_cast.into();
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                filter,
                during_turns_counter_put_on_source: None,
                spell_cost_increase: None,
                lands_enter_tapped: false,
            }),
        )
    }

    pub fn subject_verb_grant_play_tagged_during_turns_counter_put_on_source(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        counter_type: crate::object::CounterType,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player,
                allow_land,
                without_paying_mana_cost: false,
                allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode::Normal,
                filter: None,
                during_turns_counter_put_on_source: Some(counter_type),
                spell_cost_increase: None,
                lands_enter_tapped: false,
            }),
        )
    }

    pub fn subject_verb_grant_play_tagged_with_play_constraints(
        tag: TagRef,
        player: PlayerAst,
        spell_cost_increase: Option<ManaCost>,
        lands_enter_tapped: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player,
                allow_land: true,
                without_paying_mana_cost: false,
                allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode::Normal,
                filter: None,
                during_turns_counter_put_on_source: None,
                spell_cost_increase,
                lands_enter_tapped,
            }),
        )
    }

    pub fn subject_verb_grant_play_tagged_for_as_long_as_you_control_source(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: impl Into<ironsmith_core::value_model::ManaSpendMode>,
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
    ) -> Self {
        let allow_any_color_for_cast = allow_any_color_for_cast.into();
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsYouControlSource {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
                surface,
            }),
        )
    }

    pub fn subject_verb_return_to_battlefield(
        target: TargetAst,
        tapped: bool,
        transformed: bool,
        converted: bool,
        controller: ReturnControllerAst,
        count_value: Option<Value>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                target,
                target_reference_surface: None,
                from_graveyard_or_exile: false,
                tapped,
                transformed,
                converted,
                controller,
                count_value,
                as_aura: None,
                top_only: false,
            }),
        )
    }

    pub fn with_graveyard_or_exile_return_origin(mut self) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                    from_graveyard_or_exile,
                    ..
                }),
            ..
        }) = &mut self
        {
            *from_graveyard_or_exile = true;
        }
        self
    }

    pub fn with_top_only_return_choice(mut self, top_only: bool) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                    top_only: return_top_only,
                    ..
                }),
            ..
        }) = &mut self
        {
            *return_top_only = top_only;
        }
        self
    }

    pub fn subject_verb_return_all_to_battlefield(
        filter: ObjectFilter,
        tapped: bool,
        face_down: bool,
        controller: ReturnControllerAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToBattlefield {
                filter,
                tapped,
                face_down,
                controller,
                verb_surface: ironsmith_core::MoveToZoneVerbSurface::Return,
            }),
        )
    }

    pub fn subject_verb_put_all_onto_battlefield(
        filter: ObjectFilter,
        tapped: bool,
        face_down: bool,
        controller: ReturnControllerAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToBattlefield {
                filter,
                tapped,
                face_down,
                controller,
                verb_surface: ironsmith_core::MoveToZoneVerbSurface::Put,
            }),
        )
    }

    pub fn subject_verb_exile_until_source_leaves(target: TargetAst, face_down: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves {
                target,
                duration: ironsmith_core::ExileUntilDuration::SourceLeavesBattlefield,
                leave_watcher: None,
                face_down,
                all: false,
                explicit_return_surface: false,
            }),
        )
    }

    pub fn subject_verb_exile_until_target_leaves(
        target: TargetAst,
        leave_watcher: TargetAst,
        face_down: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves {
                target,
                duration: ironsmith_core::ExileUntilDuration::SourceLeavesBattlefield,
                leave_watcher: Some(leave_watcher),
                face_down,
                all: false,
                explicit_return_surface: false,
            }),
        )
    }

    pub fn subject_verb_exile_all_until_source_leaves(target: TargetAst, face_down: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves {
                target,
                duration: ironsmith_core::ExileUntilDuration::SourceLeavesBattlefield,
                leave_watcher: None,
                face_down,
                all: true,
                explicit_return_surface: false,
            }),
        )
    }

    pub fn subject_verb_exile_until_opponent_becomes_monarch(
        target: TargetAst,
        face_down: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves {
                target,
                duration: ironsmith_core::ExileUntilDuration::OpponentBecomesMonarch,
                leave_watcher: None,
                face_down,
                all: false,
                explicit_return_surface: false,
            }),
        )
    }

    pub fn with_explicit_exile_return_surface(mut self) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves {
                    explicit_return_surface,
                    ..
                }),
            ..
        }) = &mut self
        {
            *explicit_return_surface = true;
        }
        self
    }

    pub fn subject_verb_move_to_zone(
        target: TargetAst,
        zone: Zone,
        to_top: bool,
        battlefield_controller: ReturnControllerAst,
        battlefield_tapped: bool,
        attached_to: Option<TargetAst>,
    ) -> Self {
        Self::subject_verb_move_to_zone_with_attack_target(
            target,
            zone,
            to_top,
            battlefield_controller,
            battlefield_tapped,
            false,
            None,
            false,
            attached_to,
        )
    }

    pub fn subject_verb_move_to_zone_with_attacking(
        target: TargetAst,
        zone: Zone,
        to_top: bool,
        battlefield_controller: ReturnControllerAst,
        battlefield_tapped: bool,
        battlefield_attacking: bool,
        battlefield_face_down: bool,
        attached_to: Option<TargetAst>,
    ) -> Self {
        Self::subject_verb_move_to_zone_with_attack_target(
            target,
            zone,
            to_top,
            battlefield_controller,
            battlefield_tapped,
            battlefield_attacking,
            None,
            battlefield_face_down,
            attached_to,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn subject_verb_move_to_zone_with_attack_target(
        target: TargetAst,
        zone: Zone,
        to_top: bool,
        battlefield_controller: ReturnControllerAst,
        battlefield_tapped: bool,
        battlefield_attacking: bool,
        battlefield_attack_target_player_or_planeswalker_controlled_by: Option<PlayerAst>,
        battlefield_face_down: bool,
        attached_to: Option<TargetAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                target,
                source_top_only: false,
                zone,
                to_top,
                library_order: None,
                library_order_chooser: PlayerAst::Implicit,
                verb_surface: ironsmith_core::MoveToZoneVerbSurface::Put,
                target_plural_surface: false,
                target_reference_surface: None,
                destination_player_surface: None,
                destination_player_reference_surface: None,
                exiled_with_source_surface: None,
                battlefield_controller,
                battlefield_tapped,
                battlefield_attacking,
                battlefield_attack_target_player_or_planeswalker_controlled_by,
                battlefield_face_down,
                battlefield_transformed: false,
                attached_to,
                all: false,
            }),
        )
    }

    pub fn subject_verb_move_all_to_zone(
        target: TargetAst,
        zone: Zone,
        to_top: bool,
        battlefield_controller: ReturnControllerAst,
        battlefield_tapped: bool,
        attached_to: Option<TargetAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                target,
                source_top_only: false,
                zone,
                to_top,
                library_order: None,
                library_order_chooser: PlayerAst::Implicit,
                verb_surface: ironsmith_core::MoveToZoneVerbSurface::Put,
                target_plural_surface: true,
                target_reference_surface: None,
                destination_player_surface: None,
                destination_player_reference_surface: None,
                exiled_with_source_surface: None,
                battlefield_controller,
                battlefield_tapped,
                battlefield_attacking: false,
                battlefield_attack_target_player_or_planeswalker_controlled_by: None,
                battlefield_face_down: false,
                battlefield_transformed: false,
                attached_to,
                all: true,
            }),
        )
    }

    pub fn with_destination_player_surface(mut self, player: Option<PlayerAst>) -> Self {
        if let Some(player) = player
            && let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                destination_player_surface,
                ..
            }) = &mut subject_verb.action
        {
            *destination_player_surface = Some(player);
        }
        self
    }

    pub fn with_library_order(
        mut self,
        order: Option<LibraryBottomOrderAst>,
        chooser: PlayerAst,
    ) -> Self {
        if let Some(order) = order
            && let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                library_order,
                library_order_chooser,
                ..
            }) = &mut subject_verb.action
        {
            *library_order = Some(order);
            *library_order_chooser = chooser;
        }
        self
    }

    pub fn with_move_to_zone_verb_surface(
        mut self,
        surface: ironsmith_core::MoveToZoneVerbSurface,
    ) -> Self {
        if let Self::SubjectVerb(subject_verb) = &mut self {
            match &mut subject_verb.action {
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { verb_surface, .. })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToBattlefield { verb_surface, .. }) => {
                    *verb_surface = surface;
                }
                _ => {}
            }
        }
        self
    }

    pub fn with_source_top_only(mut self, source_top_only: bool) -> Self {
        if !source_top_only {
            return self;
        }
        if let Self::SubjectVerb(subject_verb) = &mut self {
            match &mut subject_verb.action {
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile {
                    source_top_only, ..
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    source_top_only, ..
                }) => *source_top_only = true,
                _ => {}
            }
        }
        self
    }

    pub fn with_move_to_zone_actor_surface(mut self, actor: PlayerAst) -> Self {
        if matches!(actor, PlayerAst::Implicit) {
            return self;
        }
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { .. }),
        }) = &mut self
        {
            subject.player = actor;
        }
        self
    }

    pub fn with_move_to_zone_plural_surface(mut self) -> Self {
        if let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                target_plural_surface,
                ..
            })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile {
                target_plural_surface,
                ..
            }) = &mut subject_verb.action
        {
            *target_plural_surface = true;
        }
        self
    }

    pub fn with_move_to_zone_plural_surface_if(self, plural: bool) -> Self {
        if plural {
            self.with_move_to_zone_plural_surface()
        } else {
            self
        }
    }

    pub fn with_move_to_zone_target_reference_surface(
        mut self,
        surface: ironsmith_core::SearchResultReferenceSurface,
    ) -> Self {
        if let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                target_reference_surface,
                ..
            }) = &mut subject_verb.action
        {
            *target_reference_surface = Some(surface);
        }
        self
    }

    pub fn with_move_to_zone_transformed(mut self) -> Self {
        if let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                battlefield_transformed,
                ..
            }) = &mut subject_verb.action
        {
            *battlefield_transformed = true;
        }
        self
    }

    pub fn with_destination_player_reference_surface(
        mut self,
        surface: Option<ironsmith_core::DestinationPlayerReferenceSurface>,
    ) -> Self {
        if let Some(surface) = surface
            && let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                destination_player_reference_surface,
                ..
            }) = &mut subject_verb.action
        {
            *destination_player_reference_surface = Some(surface);
        }
        self
    }

    pub fn with_exiled_with_source_surface(
        mut self,
        surface: Option<ironsmith_core::ExiledWithSourceMoveSurface>,
    ) -> Self {
        let Some(surface) = surface else {
            return self;
        };
        if let Self::SubjectVerb(subject_verb) = &mut self {
            match &mut subject_verb.action {
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    exiled_with_source_surface,
                    ..
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand {
                    exiled_with_source_surface,
                    ..
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand {
                    exiled_with_source_surface,
                    ..
                }) => *exiled_with_source_surface = Some(surface),
                _ => {}
            }
        }
        self
    }

    pub fn subject_verb_move_to_library_top_or_bottom_choice(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryTopOrBottomChoice { target }),
        )
    }

    pub fn subject_verb_target_only(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TargetOnly {
                target,
                explicit_declaration: false,
            },
        )
    }

    pub fn subject_verb_explicit_target_only(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TargetOnly {
                target,
                explicit_declaration: true,
            },
        )
    }

    pub fn subject_verb_explicit_target_only_for_chooser(
        target: TargetAst,
        chooser: PlayerAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            chooser,
            SubjectVerbActionAst::TargetOnly {
                target,
                explicit_declaration: true,
            },
        )
    }

    pub fn subject_verb_tag_matching_objects(
        filter: ObjectFilter,
        zones: Vec<Zone>,
        tag: TagRef,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TagMatchingObjects {
                filter,
                zones,
                tag,
                source_tags: Vec::new(),
            },
        )
    }

    pub fn subject_verb_tagged_object_union(
        filter: ObjectFilter,
        zones: Vec<Zone>,
        tag: TagRef,
        source_tags: Vec<TagRef>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TagMatchingObjects {
                filter,
                zones,
                tag,
                source_tags,
            },
        )
    }

    pub fn subject_verb_pump(
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
        condition: Option<PredicateAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
                power,
                toughness,
                target,
                duration,
                condition,
                set_quantifier_surface: None,
            }),
        )
    }

    pub fn subject_verb_set_base_power_toughness(
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                power,
                toughness,
                target,
                duration,
                set_quantifier_surface: None,
            }),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn subject_verb_become_base_pt_creature(
        power: Value,
        toughness: Value,
        target: TargetAst,
        card_types: Vec<CardType>,
        subtypes: Vec<Subtype>,
        subtype_families: Vec<SubtypeFamily>,
        colors: Option<ColorSet>,
        abilities: Vec<crate::model::CompilerStaticAbilityCore>,
        granted_abilities: Vec<GrantedAbilityAst>,
        preserve_other_types: bool,
        type_retention_surface: Option<ironsmith_core::TypeRetentionSurface>,
        animation_pt_surface: Option<ironsmith_core::AnimationPtSurface>,
        animation_duration_surface: Option<ironsmith_core::AnimationDurationSurface>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                name_override: None,
                add_supertypes: Vec::new(),
                remove_all_abilities: false,
                power,
                toughness,
                target,
                card_types,
                subtypes,
                subtype_families,
                colors,
                abilities,
                granted_abilities,
                preserve_other_types,
                type_retention_surface,
                animation_pt_surface,
                animation_duration_surface,
                set_quantifier_surface: None,
                duration,
            }),
        )
    }

    /// Preserve an authored plural/set subject on a resolving continuous
    /// effect without changing the target identity established by its AST.
    pub fn with_set_quantifier_surface(
        mut self,
        surface: Option<ironsmith_core::SetQuantifierSurface>,
    ) -> Self {
        let Some(surface) = surface else {
            return self;
        };
        let Self::SubjectVerb(subject_verb) = &mut self else {
            return self;
        };
        match &mut subject_verb.action {
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
                set_quantifier_surface,
                ..
            })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll {
                set_quantifier_surface,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                set_quantifier_surface,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                set_quantifier_surface,
                ..
            })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                set_quantifier_surface,
                ..
            })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll {
                set_quantifier_surface,
                ..
            })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                set_quantifier_surface,
                ..
            }) => *set_quantifier_surface = Some(surface),
            _ => {}
        }
        self
    }

    pub fn subject_verb_set_base_power(power: Value, target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower {
                power,
                target,
                duration,
            }),
        )
    }

    pub fn subject_verb_pump_for_each(
        power_per: i32,
        toughness_per: i32,
        target: TargetAst,
        count: Value,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach {
                power_per,
                toughness_per,
                target,
                count,
                duration,
            }),
        )
    }

    pub fn subject_verb_pump_all(
        filter: ObjectFilter,
        power: Value,
        toughness: Value,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll {
                filter,
                power,
                toughness,
                duration,
                set_quantifier_surface: None,
            }),
        )
    }

    pub fn subject_verb_pump_by_last_effect(
        power: i32,
        toughness: i32,
        target: TargetAst,
        duration: Until,
        includes_this_way: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpByLastEffect {
                power,
                toughness,
                target,
                duration,
                includes_this_way,
            }),
        )
    }

    pub fn subject_verb_add_card_types(
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddCardTypes {
                target,
                card_types,
                duration,
            }),
        )
    }

    pub fn subject_verb_remove_card_types(
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveCardTypes {
                target,
                card_types,
                duration,
            }),
        )
    }

    pub fn subject_verb_set_card_types(
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCardTypes {
                target,
                card_types,
                duration,
            }),
        )
    }

    pub fn subject_verb_add_subtypes(
        target: TargetAst,
        subtypes: Vec<Subtype>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddSubtypes {
                target,
                subtypes,
                duration,
            }),
        )
    }

    pub fn subject_verb_remove_subtypes(
        target: TargetAst,
        subtypes: Vec<Subtype>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSubtypes {
                target,
                subtypes,
                duration,
            }),
        )
    }

    pub fn subject_verb_set_creature_subtypes(
        target: TargetAst,
        subtypes: Vec<Subtype>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCreatureSubtypes {
                target,
                subtypes,
                duration,
            }),
        )
    }

    pub fn subject_verb_become_saddled_until_end_of_turn(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeSaddledUntilEndOfTurn { target }),
        )
    }

    pub fn subject_verb_add_colors(target: TargetAst, colors: ColorSet, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddColors {
                target,
                colors,
                duration,
            }),
        )
    }

    pub fn subject_verb_add_all_subtypes_of_family(
        target: TargetAst,
        family: SubtypeFamily,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddAllSubtypesOfFamily {
                target,
                family,
                duration,
            }),
        )
    }

    pub fn subject_verb_remove_all_subtypes_of_family(
        target: TargetAst,
        family: SubtypeFamily,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAllSubtypesOfFamily {
                target,
                family,
                duration,
            }),
        )
    }

    pub fn subject_verb_become_aura_enchantment(
        target: TargetAst,
        attachment_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb_become_aura_enchantment_with_grants(
            target,
            attachment_filter,
            Vec::new(),
            duration,
        )
    }

    pub fn subject_verb_become_aura_enchantment_with_grants(
        target: TargetAst,
        attachment_filter: ObjectFilter,
        granted_abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment {
                target,
                attachment_filter,
                granted_abilities,
                duration,
            }),
        )
    }

    pub fn subject_verb_become_basic_land_type(
        target: TargetAst,
        subtype: Subtype,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandType {
                target,
                subtype,
                duration,
            }),
        )
    }

    pub fn subject_verb_set_colors(target: TargetAst, colors: ColorSet, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetColors {
                target,
                colors,
                duration,
            }),
        )
    }

    pub fn subject_verb_make_colorless(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::MakeColorless { target, duration }),
        )
    }

    pub fn subject_verb_become_basic_land_type_choice(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandTypeChoice { target, duration }),
        )
    }

    pub fn subject_verb_become_creature_type_choice(
        target: TargetAst,
        duration: Until,
        excluded_subtypes: Vec<Subtype>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCreatureTypeChoice {
                target,
                duration,
                excluded_subtypes,
            }),
        )
    }

    pub fn subject_verb_become_color_choice(
        target: TargetAst,
        duration: Until,
        allow_multiple: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeColorChoice {
                target,
                duration,
                allow_multiple,
            }),
        )
    }

    pub fn subject_verb_become_copy(
        target: TargetAst,
        source: TargetAst,
        duration: Until,
        preserve_source_abilities: bool,
        name_override: Option<String>,
        name_override_surface: Option<SourceReferenceSurface>,
        add_supertypes: Vec<Supertype>,
        remove_supertypes: Vec<Supertype>,
        add_colors: ColorSet,
        add_card_types: Vec<CardType>,
        set_card_types: Vec<CardType>,
        add_subtypes: Vec<Subtype>,
        set_subtypes: Vec<Subtype>,
        granted_abilities: Vec<GrantedAbilityAst>,
        set_base_power_toughness: Option<(Value, Value)>,
        copy_exception_surface: Option<String>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy {
                target,
                source,
                duration,
                preserve_source_abilities,
                name_override,
                name_override_surface,
                add_supertypes,
                remove_supertypes,
                add_colors,
                add_card_types,
                set_card_types,
                add_subtypes,
                set_subtypes,
                granted_abilities,
                set_base_power_toughness,
                copy_exception_surface,
            }),
        )
    }

    pub fn subject_verb_grant_abilities_all(
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
                condition: None,
                set_quantifier_surface: None,
                lock_filter_at_resolution: true,
            }),
        )
    }

    pub fn subject_verb_grant_abilities_all_with_condition(
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: PredicateAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
                condition: Some(condition),
                set_quantifier_surface: None,
                lock_filter_at_resolution: true,
            }),
        )
    }

    pub fn subject_verb_grant_abilities_all_dynamically(
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
                condition: None,
                set_quantifier_surface: None,
                lock_filter_at_resolution: false,
            }),
        )
    }

    pub fn subject_verb_grant_abilities_all_dynamically_with_condition(
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: PredicateAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
                condition: Some(condition),
                set_quantifier_surface: None,
                lock_filter_at_resolution: false,
            }),
        )
    }

    pub fn subject_verb_remove_abilities_all(
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll {
                filter,
                abilities,
                duration,
                condition: None,
                set_quantifier_surface: None,
            }),
        )
    }

    pub fn subject_verb_remove_abilities_all_with_condition(
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: PredicateAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll {
                filter,
                abilities,
                duration,
                condition: Some(condition),
                set_quantifier_surface: None,
            }),
        )
    }

    pub fn subject_verb_grant_abilities_choice_all(
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceAll {
                filter,
                abilities,
                duration,
            }),
        )
    }

    pub fn subject_verb_grant_abilities_to_target(
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
                condition: None,
                set_quantifier_surface: None,
            }),
        )
    }

    pub fn subject_verb_grant_abilities_to_target_with_condition(
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: PredicateAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
                condition: Some(condition),
                set_quantifier_surface: None,
            }),
        )
    }

    pub fn subject_verb_grant_to_target(
        target: TargetAst,
        grantable: crate::model::CompilerGrantableCore,
        duration: crate::grant::GrantDuration,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantToTarget {
                target,
                grantable: Box::new(grantable),
                duration,
            }),
        )
    }

    pub fn subject_verb_grant_by_spec(
        spec: crate::model::CompilerGrantSpecCore,
        player: PlayerAst,
        duration: crate::grant::GrantDuration,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantBySpec {
                spec: Box::new(spec),
                player,
                duration,
            }),
        )
    }

    pub fn subject_verb_remove_abilities_from_target(
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            }),
        )
    }

    pub fn subject_verb_grant_abilities_choice_to_target(
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget {
                target,
                abilities,
                duration,
            }),
        )
    }

    pub fn subject_verb_consult_top_of_library(
        player: PlayerAst,
        mode: LibraryConsultModeAst,
        filter: ObjectFilter,
        stop_rule: LibraryConsultStopRuleAst,
        all_tag: TagRef,
        match_tag: TagRef,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                player,
                mode,
                filter,
                stop_rule,
                max_exposed: None,
                all_tag,
                match_tag,
            }),
        )
    }

    pub fn subject_verb_consult_top_of_library_with_max_exposed(
        player: PlayerAst,
        mode: LibraryConsultModeAst,
        filter: ObjectFilter,
        stop_rule: LibraryConsultStopRuleAst,
        max_exposed: Value,
        all_tag: TagRef,
        match_tag: TagRef,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                player,
                mode,
                filter,
                stop_rule,
                max_exposed: Some(max_exposed),
                all_tag,
                match_tag,
            }),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn subject_verb_search_library(
        filter: ObjectFilter,
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
        enters_under_your_control: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            chooser,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary {
                filter,
                search_zones: vec![Zone::Library],
                destination,
                chooser,
                player,
                search_mode,
                reveal,
                reveal_reference_surface,
                shuffle,
                count,
                count_value,
                library_position_from_top,
                result_reference_surface,
                search_top_in_any_order_surface,
                tapped,
                enters_with_counters: Vec::new(),
                enters_under_your_control,
            }),
        )
    }

    pub fn with_search_zones(mut self, zones: Vec<Zone>) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary { search_zones, .. }),
            ..
        }) = &mut self
        {
            *search_zones = zones;
        }
        self
    }

    pub fn with_search_battlefield_entry_counters(
        mut self,
        counters: Vec<ironsmith_core::BattlefieldEntryCounterSpec>,
    ) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary {
                    enters_with_counters,
                    ..
                }),
            ..
        }) = &mut self
        {
            *enters_with_counters = counters;
        }
        self
    }

    pub fn subject_verb_cant(
        restriction: crate::effect::Restriction,
        duration: crate::effect::Until,
        condition: Option<PredicateAst>,
    ) -> Self {
        Self::subject_verb_cant_starting(
            restriction,
            duration,
            crate::effect::RestrictionStart::Immediate,
            condition,
        )
    }

    pub fn subject_verb_cant_starting(
        restriction: crate::effect::Restriction,
        duration: crate::effect::Until,
        start: crate::effect::RestrictionStart,
        condition: Option<PredicateAst>,
    ) -> Self {
        Self::subject_verb_cant_starting_with_duration_surface(
            restriction,
            duration,
            start,
            crate::effect::RestrictionDurationSurface::Default,
            condition,
        )
    }

    pub fn subject_verb_cant_starting_with_duration_surface(
        restriction: crate::effect::Restriction,
        duration: crate::effect::Until,
        start: crate::effect::RestrictionStart,
        duration_surface: crate::effect::RestrictionDurationSurface,
        condition: Option<PredicateAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Cant {
                restriction,
                duration,
                start,
                duration_surface,
                condition,
            },
        )
    }

    pub fn subject_verb_redirect_next_damage_from_source_to_target(
        amount: Value,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextDamageFromSourceToTarget {
                amount,
                protected_target: None,
                destination: RedirectNextTimeDamageDestinationAst::TargetObject,
                destination_target: Some(target),
            }),
        )
    }

    pub fn subject_verb_redirect_next_damage_to_controller(
        amount: Value,
        protected_target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextDamageFromSourceToTarget {
                amount,
                protected_target: Some(protected_target),
                destination: RedirectNextTimeDamageDestinationAst::Controller,
                destination_target: None,
            }),
        )
    }

    pub fn subject_verb_redirect_next_time_damage_to_source(
        source: PreventNextTimeDamageSourceAst,
        target: TargetAst,
        destination: RedirectNextTimeDamageDestinationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextTimeDamageToSource {
                source,
                target,
                destination,
                destination_target: None,
                all_this_turn: false,
            }),
        )
    }

    pub fn subject_verb_redirect_next_time_damage_to_target(
        source: PreventNextTimeDamageSourceAst,
        target: TargetAst,
        destination_target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextTimeDamageToSource {
                source,
                target,
                destination: RedirectNextTimeDamageDestinationAst::TargetObject,
                destination_target: Some(destination_target),
                all_this_turn: false,
            }),
        )
    }

    pub fn subject_verb_redirect_all_damage_this_turn_to_source(
        source: PreventNextTimeDamageSourceAst,
        target: TargetAst,
        destination: RedirectNextTimeDamageDestinationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextTimeDamageToSource {
                source,
                target,
                destination,
                destination_target: None,
                all_this_turn: true,
            }),
        )
    }

    pub fn subject_verb_redirect_all_damage_this_turn_by_source_to_source_controller(
        source: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnBySourceToSourceController { source }),
        )
    }

    pub fn subject_verb_redirect_all_damage_this_turn_to_target(
        player_filter: PlayerFilter,
        object_filter: ObjectFilter,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnToTarget {
                player_filter,
                object_filter,
                target,
            }),
        )
    }

    pub fn subject_verb_meld(
        result_name: impl Into<String>,
        enters_tapped: bool,
        enters_attacking: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Meld {
                result_name: result_name.into(),
                enters_tapped,
                enters_attacking,
            }),
        )
    }

    pub fn subject_verb_search_library_slots_to_hand(
        player: PlayerAst,
        slots: Vec<SearchLibrarySlotAst>,
        reveal: bool,
        progress_tag: TagRef,
    ) -> Self {
        Self::subject_verb_search_library_slots(player, slots, Zone::Hand, reveal, progress_tag)
    }

    pub fn subject_verb_search_library_slots(
        player: PlayerAst,
        slots: Vec<SearchLibrarySlotAst>,
        destination: Zone,
        reveal: bool,
        progress_tag: TagRef,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrarySlotsToHand {
                slots,
                destination,
                reveal,
                progress_tag,
            }),
        )
    }

    pub fn subject_verb_retarget_stack_object(
        chooser: PlayerAst,
        target: TargetAst,
        mode: RetargetModeAst,
        require_change: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            chooser,
            SubjectVerbActionAst::Stack(StackActionAst::RetargetStackObject {
                target,
                mode,
                require_change,
                copy_reference_plural: false,
            }),
        )
    }

    /// Preserve an authored plural copy back-reference ("the copies").
    pub fn with_retarget_plural_copy_reference(mut self, plural: bool) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Stack(StackActionAst::RetargetStackObject {
                    copy_reference_plural,
                    ..
                }),
            ..
        }) = &mut self
        {
            *copy_reference_plural = plural;
        }
        self
    }

    pub fn subject_verb_grant_ability_to_source(ability: ParsedAbility, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilityToSource {
                ability: Box::new(ability),
                duration,
            }),
        )
    }

    pub fn subject_verb_exchange_control(
        filter: ObjectFilter,
        count: u32,
        shared_type: Option<SharedTypeConstraintAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeControl {
                filter,
                count,
                shared_type,
            }),
        )
    }

    pub fn subject_verb_exchange_control_heterogeneous(
        permanent1: TargetAst,
        permanent2: TargetAst,
        shared_type: Option<SharedTypeConstraintAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeControlHeterogeneous {
                permanent1,
                permanent2,
                shared_type,
            }),
        )
    }

    pub fn subject_verb_destroy_all_attached_to(filter: ObjectFilter, target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllAttachedTo { filter, target }),
        )
    }

    pub fn subject_verb_exile_all_attached_to(
        filter: ObjectFilter,
        target: TargetAst,
        face_down: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAllAttachedTo {
                filter,
                target,
                face_down,
            }),
        )
    }

    pub fn subject_verb_attach(object: TargetAst, target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Control(ControlActionAst::Attach { object, target }),
        )
    }

    pub fn subject_verb_unattach(object: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Control(ControlActionAst::Unattach { object }),
        )
    }

    pub fn subject_verb_enchant(filter: AuraAttachmentFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Control(ControlActionAst::Enchant { filter }),
        )
    }

    pub fn subject_verb_exile_when_source_leaves(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileWhenSourceLeaves { target }),
        )
    }

    pub fn subject_verb_sacrifice_source_when_leaves(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeSourceWhenLeaves { target }),
        )
    }

    pub fn subject_verb_register_zone_replacement(
        target: TargetAst,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterZoneReplacement {
                target,
                from_zone,
                to_zone,
                replacement_zone,
                library_placement: None,
                duration,
                optional: false,
                choice_description: None,
                counters: Vec::new(),
                linked_exile_follow_up: None,
            }),
        )
    }

    pub fn subject_verb_register_zone_replacement_with_counters(
        target: TargetAst,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
        counters: Vec<(CounterType, u32)>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterZoneReplacement {
                target,
                from_zone,
                to_zone,
                replacement_zone,
                library_placement: None,
                duration,
                optional: false,
                choice_description: None,
                counters,
                linked_exile_follow_up: None,
            }),
        )
    }

    pub fn subject_verb_register_zone_replacement_with_library_placement(
        target: TargetAst,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        placement: ironsmith_core::ZoneReplacementLibraryPlacement,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterZoneReplacement {
                target,
                from_zone,
                to_zone,
                replacement_zone,
                library_placement: Some(placement),
                duration,
                optional: false,
                choice_description: None,
                counters: Vec::new(),
                linked_exile_follow_up: None,
            }),
        )
    }

    pub fn subject_verb_register_zone_replacement_with_linked_exile_follow_up(
        target: TargetAst,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
        follow_up: ironsmith_core::LinkedExileFollowUp,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterZoneReplacement {
                target,
                from_zone,
                to_zone,
                replacement_zone,
                library_placement: None,
                duration,
                optional: false,
                choice_description: None,
                counters: Vec::new(),
                linked_exile_follow_up: Some(follow_up),
            }),
        )
    }

    pub fn subject_verb_register_future_zone_replacement(
        filter: ObjectFilter,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
        cause_policy: FutureZoneReplacementCausePolicyAst,
        link_exiled_to_source: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterFutureZoneReplacement {
                filter,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
                cause_policy,
                link_exiled_to_source,
            }),
        )
    }

    pub fn subject_verb_register_draw_replacement(
        player: PlayerFilter,
        replacement_effects: Vec<EffectAst>,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDrawReplacement {
                player,
                replacement_effects,
                duration,
            }),
        )
    }

    pub fn subject_verb_register_mana_replacement(
        source_filter: ObjectFilter,
        replacement_mana: Vec<ManaSymbol>,
        mode: crate::effects::ReplacementApplyMode,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterManaReplacement {
                source_filter,
                replacement_mana,
                mode,
            }),
        )
    }

    pub fn subject_verb_register_damaged_by_source_zone_replacement(
        filter: ObjectFilter,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDamagedBySourceZoneReplacement {
                filter,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
            }),
        )
    }

    pub fn subject_verb_register_enter_under_control_replacement(
        filter: ObjectFilter,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterEnterUnderControlReplacement { filter, duration }),
        )
    }

    pub fn subject_verb_register_enter_tapped_replacement(
        filter: ObjectFilter,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterEnterTappedReplacement { filter, duration }),
        )
    }

    pub fn subject_verb_register_next_batch_enter_with_counters(
        filter: ObjectFilter,
        counter_type: CounterType,
        count: Value,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterNextBatchEnterWithCounters {
                filter,
                counter_type,
                count,
            }),
        )
    }

    pub fn subject_verb_choose_spell_cast_history(
        chooser: PlayerAst,
        cast_by: PlayerAst,
        filter: ObjectFilter,
        tag: TagRef,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            chooser,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseSpellCastHistory {
                cast_by,
                filter,
                tag,
            }),
        )
    }

    pub fn subject_verb_damage(amount: Value, target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                amount,
                target,
                unpreventable: false,
            }),
        )
    }

    pub fn subject_verb_damage_each(amount: Value, filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { amount, filter }),
        )
    }

    pub fn subject_verb_damage_equal_to_power(source: TargetAst, target: TargetAst) -> Self {
        Self::subject_verb_damage_with_source(
            source.clone(),
            Value::PowerOf(Box::new(crate::target::ChooseSpec::Source)),
            target,
        )
    }

    pub fn subject_verb_damage_with_source(
        source: TargetAst,
        amount: Value,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                source,
                amount,
                target,
                unpreventable: false,
            }),
        )
    }

    pub fn subject_verb_distributed_damage(amount: Value, target: TargetAst) -> Self {
        Self::subject_verb_distributed_damage_with_source_and_mode(
            amount,
            target,
            TargetAst::Source(None),
            PlayerFilter::You,
            ironsmith_core::DamageDistributionMode::Chosen,
        )
    }

    pub fn subject_verb_evenly_distributed_damage(amount: Value, target: TargetAst) -> Self {
        Self::subject_verb_distributed_damage_with_source_and_mode(
            amount,
            target,
            TargetAst::Source(None),
            PlayerFilter::You,
            ironsmith_core::DamageDistributionMode::EvenRoundedDown,
        )
    }

    pub fn subject_verb_distributed_damage_with_source(
        amount: Value,
        target: TargetAst,
        source: TargetAst,
        chooser: PlayerFilter,
    ) -> Self {
        Self::subject_verb_distributed_damage_with_source_and_mode(
            amount,
            target,
            source,
            chooser,
            ironsmith_core::DamageDistributionMode::Chosen,
        )
    }

    pub fn subject_verb_distributed_damage_with_source_and_mode(
        amount: Value,
        target: TargetAst,
        source: TargetAst,
        chooser: PlayerFilter,
        distribution: ironsmith_core::DamageDistributionMode,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage {
                amount,
                target,
                source,
                chooser,
                distribution,
            }),
        )
    }

    pub fn subject_verb_proliferate(count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Proliferate { count }),
        )
    }

    pub fn subject_verb_investigate(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Investigate { count }),
        )
    }

    pub fn subject_verb_incubate(player: PlayerAst, amount: Value, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Incubate { amount, count }),
        )
    }

    pub fn subject_verb_learn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Learn),
        )
    }

    pub fn subject_verb_emit_keyword_action(
        action: crate::events::KeywordActionKind,
        amount: u32,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::EmitKeywordAction { action, amount }),
        )
    }

    pub fn subject_verb_amass(subtype: Option<Subtype>, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Amass { subtype, amount }),
        )
    }

    pub fn subject_verb_bolster(amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Bolster { amount }),
        )
    }

    pub fn subject_verb_support(amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Support { amount }),
        )
    }

    pub fn subject_verb_adapt(amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Adapt { amount }),
        )
    }

    pub fn subject_verb_monstrosity(amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Monstrosity { amount }),
        )
    }

    pub fn subject_verb_discover(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Discover { count }),
        )
    }

    pub fn subject_verb_fateseal(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fateseal { count }),
        )
    }

    pub fn subject_verb_populate(count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Populate {
                count,
                enters_tapped: false,
                enters_attacking: false,
                has_haste: false,
                sacrifice_at_next_end_step: false,
                exile_at_next_end_step: false,
                next_end_step_player: PlayerFilter::Any,
                exile_at_end_of_combat: false,
                sacrifice_at_end_of_combat: false,
            }),
        )
    }

    pub fn subject_verb_explore(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Explore { target }),
        )
    }

    pub fn subject_verb_endure(target: TargetAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Endure { target, amount }),
        )
    }

    pub fn subject_verb_exploit() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Exploit),
        )
    }

    pub fn subject_verb_connive(target: TargetAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Connive { target, count }),
        )
    }

    pub fn subject_verb_connive_iterated() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::ConniveIterated),
        )
    }

    pub fn subject_verb_put_rest_on_bottom_of_library() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Library(LibraryActionAst::PutRestOnBottomOfLibrary),
        )
    }

    pub fn subject_verb_dont_lose_this_mana_as_steps_and_phases_end_this_turn() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Mana(ManaActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn),
        )
    }

    pub fn subject_verb_open_attraction(player: PlayerAst, reminder: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::OpenAttraction { reminder }),
        )
    }

    pub fn subject_verb_manifest_top_card(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::ManifestTopCardOfLibrary),
        )
    }

    pub fn subject_verb_cloak_top_card(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::CloakTopCardOfLibrary),
        )
    }

    pub fn subject_verb_manifest_from_hand(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::ManifestCardFromHand),
        )
    }

    pub fn subject_verb_manifest_dread(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::ManifestDread),
        )
    }

    pub fn subject_verb_earthbend(counters: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Earthbend { counters }),
        )
    }

    pub fn subject_verb_behold(subtype: Subtype, count: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Behold { subtype, count }),
        )
    }

    pub fn subject_verb_fight(creature1: TargetAst, creature2: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fight {
                creature1,
                creature2,
                mutual_surface: false,
            }),
        )
    }

    pub fn with_mutual_fight_surface(mut self) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fight { mutual_surface, .. }),
            ..
        }) = &mut self
        {
            *mutual_surface = true;
        }
        self
    }

    pub fn subject_verb_fight_iterated(creature2: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::FightIterated { creature2 }),
        )
    }

    pub fn subject_verb_clash(opponent: ClashOpponentAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Clash { opponent }),
        )
    }

    pub fn subject_verb_add_mana(player: PlayerAst, mana: Vec<ManaSymbol>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddMana { mana }),
        )
    }

    pub fn subject_verb_add_mana_scaled(
        player: PlayerAst,
        mana: Vec<ManaSymbol>,
        amount: Value,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaScaled { mana, amount }),
        )
    }

    pub fn subject_verb_add_mana_any_color(
        player: PlayerAst,
        amount: Value,
        available_colors: Option<Vec<crate::color::Color>>,
    ) -> Self {
        Self::subject_verb_add_mana_any_color_with_distinct(player, amount, available_colors, false)
    }

    pub fn subject_verb_add_mana_any_color_with_distinct(
        player: PlayerAst,
        amount: Value,
        available_colors: Option<Vec<crate::color::Color>>,
        distinct_colors: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaAnyColor {
                amount,
                available_colors,
                distinct_colors,
            }),
        )
    }

    pub fn subject_verb_add_mana_any_one_color(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaAnyOneColor { amount }),
        )
    }

    pub fn subject_verb_add_mana_chosen_color(
        player: PlayerAst,
        amount: Value,
        fixed_option: Option<crate::color::Color>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaChosenColor {
                amount,
                fixed_option,
            }),
        )
    }

    pub fn subject_verb_add_mana_from_land_could_produce(
        player: PlayerAst,
        amount: Value,
        land_filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
        mana_type_source: crate::effects::ManaTypeSource,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaFromLandCouldProduce {
                amount,
                land_filter,
                allow_colorless,
                same_type,
                mana_type_source,
            }),
        )
    }

    pub fn subject_verb_add_mana_colors_among(player: PlayerAst, filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaColorsAmong { filter }),
        )
    }

    pub fn subject_verb_add_one_mana_any_color_among(
        player: PlayerAst,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddOneManaAnyColorAmong {
                filter,
                choose_color_of_object_surface: false,
            }),
        )
    }

    pub fn subject_verb_choose_color_of_object_add_mana(
        player: PlayerAst,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddOneManaAnyColorAmong {
                filter,
                choose_color_of_object_surface: true,
            }),
        )
    }

    pub fn subject_verb_add_mana_commander_identity(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaCommanderIdentity { amount }),
        )
    }

    pub fn subject_verb_exchange_life_totals(player1: PlayerAst, player2: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player1,
            SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeLifeTotals { player2 }),
        )
    }

    pub fn subject_verb_exchange_text_boxes(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeTextBoxes { target }),
        )
    }

    pub fn subject_verb_exchange_zones(player: PlayerAst, zone1: Zone, zone2: Zone) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeZones { zone1, zone2 }),
        )
    }

    pub fn subject_verb_exchange_values(
        left: ExchangeValueAst,
        right: ExchangeValueAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeValues {
                left,
                right,
                duration,
            }),
        )
    }

    pub fn subject_verb_exile_instead_of_graveyard_this_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileInsteadOfGraveyardThisTurn),
        )
    }

    pub fn subject_verb_control_combat_choices_this_turn(attackers: bool, blockers: bool) -> Self {
        Self::subject_verb_control_combat_choices(attackers, blockers, false)
    }

    pub fn subject_verb_control_combat_choices(
        attackers: bool,
        blockers: bool,
        this_combat: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Control(ControlActionAst::ControlCombatChoicesThisTurn {
                attackers,
                blockers,
                this_combat,
            }),
        )
    }

    pub fn subject_verb_control_player(
        player: PlayerAst,
        target: PlayerFilter,
        duration: ControlDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Control(ControlActionAst::ControlPlayer {
                player: target,
                duration,
            }),
        )
    }

    pub fn subject_verb_reduce_next_spell_cost_this_turn(
        player: PlayerAst,
        filter: ObjectFilter,
        reduction: ManaCost,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Stack(StackActionAst::ReduceNextSpellCostThisTurn { filter, reduction }),
        )
    }

    pub fn subject_verb_reduce_matching_spell_cost_this_turn(
        player: PlayerAst,
        filter: ObjectFilter,
        reduction: Value,
    ) -> Self {
        Self::subject_verb_reduce_matching_spell_cost(player, filter, reduction, Until::EndOfTurn)
    }

    pub fn subject_verb_reduce_matching_spell_cost(
        player: PlayerAst,
        filter: ObjectFilter,
        reduction: Value,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Stack(StackActionAst::ReduceMatchingSpellCostThisTurn {
                filter,
                reduction,
                duration,
                next_only: false,
            }),
        )
    }

    pub fn subject_verb_reduce_next_spell_generic_cost_this_turn(
        player: PlayerAst,
        filter: ObjectFilter,
        reduction: Value,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Stack(StackActionAst::ReduceMatchingSpellCostThisTurn {
                filter,
                reduction,
                duration: Until::EndOfTurn,
                next_only: true,
            }),
        )
    }

    pub fn subject_verb_gain_control(
        player: PlayerAst,
        target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb_gain_control_with_condition(player, target, duration, None)
    }

    pub fn subject_verb_gain_control_with_condition(
        player: PlayerAst,
        target: TargetAst,
        duration: Until,
        condition: Option<PredicateAst>,
    ) -> Self {
        Self::subject_verb_gain_control_with_condition_and_source_surface(
            player, target, duration, condition, None,
        )
    }

    pub fn subject_verb_gain_control_with_condition_and_source_surface(
        player: PlayerAst,
        target: TargetAst,
        duration: Until,
        condition: Option<PredicateAst>,
        source_reference_surface: Option<SourceReferenceSurface>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Control(ControlActionAst::GainControl {
                target,
                duration,
                condition,
                controller_reference: None,
                source_reference_surface,
            }),
        )
    }

    pub fn subject_verb_reveal_top(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTop),
        )
    }

    pub fn subject_verb_exile_top_of_library(
        player: PlayerAst,
        count: Value,
        tags: Vec<TagRef>,
        accumulated_tags: Vec<TagRef>,
    ) -> Self {
        Self::subject_verb_exile_top_of_library_with_optional_surface(
            player,
            count,
            tags,
            accumulated_tags,
            None,
        )
    }

    pub fn subject_verb_exile_top_of_library_with_optional_surface(
        player: PlayerAst,
        count: Value,
        tags: Vec<TagRef>,
        accumulated_tags: Vec<TagRef>,
        surface: Option<ironsmith_core::ExileTopLibrarySurface>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                count,
                surface,
                tags,
                accumulated_tags,
                face_down: false,
            }),
        )
    }

    pub fn subject_verb_exile_top_of_library_face_down(
        player: PlayerAst,
        count: Value,
        accumulated_tag: TagRef,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                count,
                surface: None,
                tags: Vec::new(),
                accumulated_tags: vec![accumulated_tag],
                face_down: true,
            }),
        )
    }

    pub fn subject_verb_reveal_tagged(tag: TagRef) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTagged { tag }),
        )
    }

    pub fn subject_verb_put_onto_battlefield(
        player: PlayerAst,
        target: TargetAst,
        tapped: bool,
        controller: ReturnControllerAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PutOntoBattlefield {
                target,
                tapped,
                controller,
                cloak: false,
                shuffle_before: false,
            }),
        )
    }

    pub fn subject_verb_cloak_onto_battlefield(
        player: PlayerAst,
        target: TargetAst,
        tapped: bool,
        controller: ReturnControllerAst,
        shuffle_before: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PutOntoBattlefield {
                target,
                tapped,
                controller,
                cloak: true,
                shuffle_before,
            }),
        )
    }

    pub fn subject_verb_reveal_cards_from_hand(
        player: PlayerAst,
        count: ChoiceCount,
        count_value: Option<Value>,
        tag: TagRef,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealCardsFromHand {
                count,
                count_value,
                tag,
            }),
        )
    }

    pub fn subject_verb_look_at_top_cards(player: PlayerAst, count: Value, tag: TagRef) -> Self {
        Self::subject_verb_top_library_cards(player, count, tag, false)
    }

    pub fn subject_verb_reveal_top_cards(player: PlayerAst, count: Value, tag: TagRef) -> Self {
        Self::subject_verb_top_library_cards(player, count, tag, true)
    }

    pub fn subject_verb_look_at_objects(player: PlayerAst, filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtObjects { filter }),
        )
    }

    pub fn subject_verb_look_at_target(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTarget { target }),
        )
    }

    fn subject_verb_top_library_cards(
        player: PlayerAst,
        count: Value,
        tag: TagRef,
        reveal: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards { count, tag, reveal }),
        )
    }

    pub fn subject_verb_put_into_hand(player: PlayerAst, object: ObjectRefAst) -> Self {
        let ObjectRefAst::Tagged(tag) = object;
        Self::subject_verb_move_to_zone(
            TargetAst::Tagged(tag, None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )
        .with_destination_player_surface(Some(player))
    }

    pub fn subject_verb_additional_land_plays(
        player: PlayerAst,
        count: Value,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::AdditionalLandPlays { count, duration }),
        )
    }

    pub fn subject_verb_extra_turn_after_turn(
        player: PlayerAst,
        anchor: ExtraTurnAnchorAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Game(GameActionAst::ExtraTurnAfterTurn { anchor }),
        )
    }

    pub fn subject_verb_reorder_top_of_library(tag: TagRef) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Library(LibraryActionAst::ReorderTopOfLibrary { tag }),
        )
    }

    pub fn subject_verb_shuffle_objects_into_library(player: PlayerAst, target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleObjectsIntoLibrary {
                target,
                all: false,
                owner_library_destination: false,
                possessive_owner_subject: false,
            }),
        )
    }

    pub fn subject_verb_shuffle_objects_into_library_possessive_owner(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::ItsOwner,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleObjectsIntoLibrary {
                target,
                all: false,
                owner_library_destination: false,
                possessive_owner_subject: true,
            }),
        )
    }

    pub fn subject_verb_shuffle_objects_into_owner_library(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::ItsOwner,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleObjectsIntoLibrary {
                target,
                all: false,
                owner_library_destination: true,
                possessive_owner_subject: false,
            }),
        )
    }

    pub fn subject_verb_shuffle_all_objects_into_library(
        player: PlayerAst,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleObjectsIntoLibrary {
                target,
                all: true,
                owner_library_destination: false,
                possessive_owner_subject: false,
            }),
        )
    }

    pub fn subject_verb_shuffle_all_objects_into_owner_library(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::ItsOwner,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleObjectsIntoLibrary {
                target,
                all: true,
                owner_library_destination: true,
                possessive_owner_subject: false,
            }),
        )
    }

    pub fn subject_verb_add_mana_imprinted_colors() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaImprintedColors),
        )
    }

    pub fn subject_verb_flip_coin(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Random(RandomActionAst::FlipCoin),
        )
    }

    pub fn subject_verb_flip_coin_face_only(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Random(RandomActionAst::FlipCoinFaceOnly),
        )
    }

    pub fn subject_verb_roll_die(player: PlayerAst, sides: u32) -> Self {
        Self::subject_verb_roll_die_with_surface(player, sides, None)
    }

    pub fn subject_verb_roll_die_with_surface(
        player: PlayerAst,
        sides: u32,
        surface: Option<DieSurface>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Random(RandomActionAst::RollDie { sides, surface }),
        )
    }

    pub fn subject_verb_roll_dice_choose_result_with_surface(
        player: PlayerAst,
        count: u32,
        sides: u32,
        surface: Option<DieSurface>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Random(RandomActionAst::RollDiceChooseResult {
                count,
                sides,
                surface,
            }),
        )
    }

    pub fn subject_verb_shuffle_hand_and_graveyard_into_library(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleHandAndGraveyardIntoLibrary),
        )
    }

    pub fn subject_verb_shuffle_hand_graveyard_and_owned_permanents_into_library(
        player: PlayerAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary),
        )
    }

    pub fn subject_verb_shuffle_graveyard_into_library(player: PlayerAst) -> Self {
        Self::subject_verb_shuffle_graveyard_into_library_with_surface(player, false)
    }

    pub fn subject_verb_shuffle_graveyard_into_library_with_surface(
        player: PlayerAst,
        explicit_all_cards_from: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleGraveyardIntoLibrary {
                explicit_all_cards_from,
            }),
        )
    }

    pub fn subject_verb_reorder_graveyard(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::ReorderGraveyard),
        )
    }

    pub fn subject_verb_choose_color(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseColor),
        )
    }

    pub fn subject_verb_choose_card_type(player: PlayerAst, options: Vec<CardType>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCardType { options }),
        )
    }

    pub fn subject_verb_choose_named_option(player: PlayerAst, options: Vec<String>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNamedOption { options }),
        )
    }

    pub fn subject_verb_choose_creature_type(
        player: PlayerAst,
        excluded_subtypes: Vec<Subtype>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCreatureType {
                excluded_subtypes,
                family: SubtypeFamily::Creature,
            }),
        )
    }

    pub fn subject_verb_choose_subtype_type(player: PlayerAst, family: SubtypeFamily) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCreatureType {
                excluded_subtypes: Vec::new(),
                family,
            }),
        )
    }

    pub fn subject_verb_choose_land_type(player: PlayerAst, exclude_basic: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseLandType { exclude_basic }),
        )
    }

    pub fn subject_verb_choose_card_name(
        player: PlayerAst,
        filter: Option<ObjectFilter>,
        tag: TagRef,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCardName { filter, tag }),
        )
    }

    pub fn subject_verb_choose_player(
        chooser: PlayerAst,
        filter: PlayerFilter,
        tag: TagRef,
        random: bool,
        exclude_previous_choices: usize,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            chooser,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChoosePlayer {
                filter,
                tag,
                random,
                exclude_previous_choices,
            }),
        )
    }

    pub fn subject_verb_tap(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap { target }),
        )
    }

    pub fn subject_verb_untap(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Untap { target }),
        )
    }

    pub fn subject_verb_tap_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { filter }),
        )
    }

    pub fn subject_verb_untap_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { filter }),
        )
    }

    pub fn subject_verb_tap_or_untap(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntap { target }),
        )
    }

    pub fn subject_verb_tap_or_untap_all(
        tap_filter: ObjectFilter,
        untap_filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntapAll {
                tap_filter,
                untap_filter,
            }),
        )
    }

    pub fn subject_verb_phase_out(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOut {
                target,
                duration: crate::effects::PhaseOutDuration::UntilNextUntap,
                source_surface: None,
            }),
        )
    }

    pub fn subject_verb_phase_out_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOutAll {
                filter,
                duration: crate::effects::PhaseOutDuration::UntilNextUntap,
                source_surface: None,
            }),
        )
    }

    pub fn subject_verb_phase_out_all_until_source_leaves(
        filter: ObjectFilter,
        source_surface: SourceReferenceSurface,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOutAll {
                filter,
                duration: crate::effects::PhaseOutDuration::UntilSourceLeaves,
                source_surface: Some(source_surface),
            }),
        )
    }

    pub fn subject_verb_phase_in(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseIn { target }),
        )
    }

    pub fn subject_verb_phase_in_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseInAll { filter }),
        )
    }

    pub fn subject_verb_transform(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Transform { target }),
        )
    }

    pub fn subject_verb_convert(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Convert { target }),
        )
    }

    pub fn subject_verb_destroy(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy {
                target,
                no_regeneration: false,
                creature_destroyed_this_way_surface: false,
            }),
        )
    }

    pub fn subject_verb_destroy_no_regeneration(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy {
                target,
                no_regeneration: true,
                creature_destroyed_this_way_surface: false,
            }),
        )
    }

    pub fn subject_verb_destroy_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll {
                filter,
                no_regeneration: false,
                creature_destroyed_this_way_surface: false,
            }),
        )
    }

    pub fn subject_verb_destroy_all_of_chosen_color(
        filter: ObjectFilter,
        no_regeneration: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor {
                filter,
                no_regeneration,
                creature_destroyed_this_way_surface: false,
            }),
        )
    }

    pub fn subject_verb_exile(target: TargetAst, face_down: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile {
                target,
                face_down,
                source_top_only: false,
                target_plural_surface: false,
            }),
        )
    }

    pub fn subject_verb_exile_all(filter: ObjectFilter, face_down: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, face_down }),
        )
    }

    pub fn subject_verb_look_at_hand(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtHand { target }),
        )
    }

    pub fn subject_verb_counter(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Stack(StackActionAst::Counter { target }),
        )
    }

    pub fn subject_verb_counter_unless_pays(
        target: TargetAst,
        cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Stack(StackActionAst::CounterUnlessPays { target, cost }),
        )
    }

    pub fn subject_verb_put_counters(
        counter_type: CounterType,
        count: Value,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
        distributed: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {
                counter_type,
                count,
                target,
                target_count,
                distributed,
            }),
        )
    }

    pub fn subject_verb_put_counter_choice(
        counter_types: Vec<CounterType>,
        count: Value,
        mode_texts: Vec<String>,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::PutCounterChoice {
                counter_types,
                count,
                mode_texts,
                target,
                target_count,
            }),
        )
    }

    pub fn subject_verb_put_or_remove_counters(
        put_counter_type: CounterType,
        put_count: Value,
        remove_counter_type: CounterType,
        remove_count: Value,
        put_mode_text: impl Into<String>,
        remove_mode_text: impl Into<String>,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::PutOrRemoveCounters {
                put_counter_type,
                put_count,
                remove_counter_type,
                remove_count,
                put_mode_text: put_mode_text.into(),
                remove_mode_text: remove_mode_text.into(),
                target,
                target_count,
            }),
        )
    }

    pub fn subject_verb_put_counters_all(
        counter_type: CounterType,
        count: Value,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::PutCountersAll {
                counter_type,
                count,
                filter,
            }),
        )
    }

    pub fn subject_verb_remove_up_to_any_counters(
        amount: Value,
        target: TargetAst,
        counter_type: Option<CounterType>,
        up_to: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::RemoveUpToAnyCounters {
                amount,
                target,
                counter_type,
                up_to,
                distributed_across_all: false,
                all_of_them: false,
            }),
        )
    }

    pub fn subject_verb_remove_up_to_counters_among(
        amount: Value,
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        up_to: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::RemoveUpToAnyCounters {
                amount,
                target: TargetAst::Object(filter, None, None),
                counter_type,
                up_to,
                distributed_across_all: true,
                all_of_them: false,
            }),
        )
    }

    pub fn subject_verb_remove_all_of_them_counters_from_source() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::RemoveUpToAnyCounters {
                amount: Value::CountersOn(Box::new(ChooseSpec::Source), None),
                target: TargetAst::Source(None),
                counter_type: None,
                up_to: false,
                distributed_across_all: false,
                all_of_them: true,
            }),
        )
    }

    pub fn subject_verb_move_all_counters(from: TargetAst, to: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::MoveAllCounters { from, to }),
        )
    }

    pub fn subject_verb_move_one_counter(from: TargetAst, to: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::MoveOneCounter { from, to }),
        )
    }

    pub fn subject_verb_for_each_counter_kind_put_or_remove(target: TargetAst) -> Self {
        Self::subject_verb_counter_kind_put_or_remove(target, true)
    }

    pub fn subject_verb_one_counter_kind_put_or_remove(target: TargetAst) -> Self {
        Self::subject_verb_counter_kind_put_or_remove(target, false)
    }

    pub fn subject_verb_fixed_counter_kind_put_or_remove(
        target: TargetAst,
        counter_type: CounterType,
        optional_action: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::ForEachCounterKindPutOrRemove {
                target,
                counter_source: None,
                all_kinds: false,
                fixed_counter_type: Some(counter_type),
                optional_action,
                put_only: false,
                choose_target_per_kind: false,
            }),
        )
    }

    pub fn subject_verb_put_each_counter_kind_from_on_one_of(
        counter_source: TargetAst,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::ForEachCounterKindPutOrRemove {
                target,
                counter_source: Some(counter_source),
                all_kinds: true,
                fixed_counter_type: None,
                optional_action: false,
                put_only: true,
                choose_target_per_kind: true,
            }),
        )
    }

    fn subject_verb_counter_kind_put_or_remove(target: TargetAst, all_kinds: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::ForEachCounterKindPutOrRemove {
                target,
                counter_source: None,
                all_kinds,
                fixed_counter_type: None,
                optional_action: false,
                put_only: false,
                choose_target_per_kind: false,
            }),
        )
    }

    pub fn subject_verb_put_counter_of_chosen_kind(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::PutCounterOfChosenKind { target }),
        )
    }

    pub fn subject_verb_return_to_hand(target: TargetAst, random: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand {
                target,
                random,
                destination_player_surface: None,
                exiled_with_source_surface: None,
                set_quantifier_surface: None,
                set_reference_surface: None,
            }),
        )
    }

    pub fn subject_verb_return_all_to_hand(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand {
                filter,
                destination_player_surface: None,
                exiled_with_source_surface: None,
            }),
        )
    }

    pub fn with_return_destination_player_surface(mut self, player: Option<PlayerAst>) -> Self {
        if let Some(player) = player
            && let Self::SubjectVerb(subject_verb) = &mut self
        {
            match &mut subject_verb.action {
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand {
                    destination_player_surface,
                    ..
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand {
                    destination_player_surface,
                    ..
                }) => *destination_player_surface = Some(player),
                _ => {}
            }
        }
        self
    }

    pub fn with_return_set_quantifier_surface(
        mut self,
        surface: Option<ironsmith_core::SetQuantifierSurface>,
    ) -> Self {
        if let Some(surface) = surface
            && let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand {
                set_quantifier_surface,
                ..
            }) = &mut subject_verb.action
        {
            *set_quantifier_surface = Some(surface);
        }
        self
    }

    pub fn with_return_set_reference_surface(mut self, surface: Option<String>) -> Self {
        if let Some(surface) = surface
            && let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand {
                set_reference_surface,
                ..
            }) = &mut subject_verb.action
        {
            *set_reference_surface = Some(surface);
        }
        self
    }

    pub fn subject_verb_return_all_to_hand_of_chosen_color(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHandOfChosenColor { filter }),
        )
    }

    pub fn subject_verb_move_to_library_nth_from_top(target: TargetAst, position: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryNthFromTop { target, position }),
        )
    }

    pub fn subject_verb_double_counters_on_each(
        counter_type: Option<CounterType>,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnEach {
                counter_type,
                filter,
            }),
        )
    }

    pub fn subject_verb_double_counters_on_target(
        counter_type: Option<CounterType>,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnTarget {
                counter_type,
                target,
            }),
        )
    }

    pub fn subject_verb_remove_counters_all(
        amount: Value,
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        up_to: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::RemoveCountersAll {
                amount,
                filter,
                counter_type,
                up_to,
            }),
        )
    }

    pub fn subject_verb_put_sticker(
        target: TargetAst,
        action: crate::events::KeywordActionKind,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutSticker { target, action },
        )
    }

    pub fn subject_verb_unlock_room_door(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::UnlockRoomDoor),
        )
    }

    pub fn subject_verb_switch_power_toughness(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::SwitchPowerToughness { target, duration }),
        )
    }

    pub fn subject_verb_scale_power_toughness_all(
        filter: ObjectFilter,
        power: bool,
        toughness: bool,
        multiplier: i32,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::ScalePowerToughnessAll {
                filter,
                power,
                toughness,
                multiplier,
                duration,
            }),
        )
    }

    pub fn subject_verb_reveal_hand(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealHand),
        )
    }

    pub fn subject_verb_discard(
        player: PlayerAst,
        count: Value,
        random: bool,
        any_number: bool,
        filter: Option<ObjectFilter>,
        tag: Option<TagRef>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Discard {
                count,
                random,
                any_number,
                filter,
                tag,
            }),
        )
    }

    pub fn subject_verb_discard_hand(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DiscardHand),
        )
    }

    pub fn subject_verb_poison_counters(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Counters(CounterActionAst::PoisonCounters { count }),
        )
    }

    pub fn subject_verb_energy_counters(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Counters(CounterActionAst::EnergyCounters { count }),
        )
    }

    pub fn subject_verb_experience_counters(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Counters(CounterActionAst::ExperienceCounters { count }),
        )
    }

    pub fn subject_verb_ticket_counters(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Counters(CounterActionAst::TicketCounters { count }),
        )
    }

    pub fn subject_verb_pay_energy(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayEnergy { amount }),
        )
    }

    pub fn subject_verb_pay_life(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayLife { amount }),
        )
    }

    pub fn subject_verb_pay_any_energy(player: PlayerAst, min_amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyEnergy { min_amount }),
        )
    }

    pub fn subject_verb_pay_any_life(player: PlayerAst, min_amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyLife { min_amount }),
        )
    }

    pub fn subject_verb_pay_mana(player: PlayerAst, cost: ManaCost) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::PayMana {
                cost,
                x_value: None,
                x_maximum: None,
            }),
        )
    }

    pub fn subject_verb_pay_mana_up_to(
        player: PlayerAst,
        cost: ManaCost,
        x_maximum: Value,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::PayMana {
                cost,
                x_value: None,
                x_maximum: Some(x_maximum),
            }),
        )
    }

    pub fn subject_verb_double_mana_pool(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::DoubleManaPool),
        )
    }

    pub fn subject_verb_empty_mana_pool(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::EmptyManaPool),
        )
    }

    pub fn subject_verb_set_life_total(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetLifeTotal { amount }),
        )
    }

    pub fn subject_verb_skip_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipTurn),
        )
    }

    pub fn subject_verb_end_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Game(GameActionAst::EndTurn),
        )
    }

    pub fn subject_verb_reverse_turn_order() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Game(GameActionAst::ReverseTurnOrder),
        )
    }

    pub fn subject_verb_end_combat_phase(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Game(GameActionAst::EndCombatPhase),
        )
    }

    pub fn subject_verb_skip_combat_phases(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipCombatPhases),
        )
    }

    pub fn subject_verb_skip_next_combat_phase_this_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipNextCombatPhaseThisTurn),
        )
    }

    pub fn subject_verb_skip_main_phases_this_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipMainPhasesThisTurn),
        )
    }

    pub fn subject_verb_skip_combat_phases_this_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipCombatPhasesThisTurn),
        )
    }

    pub fn subject_verb_skip_draw_step(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipDrawStep),
        )
    }

    pub fn subject_verb_additional_phases(phases: Vec<crate::effects::AdditionalPhase>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::AdditionalPhases { phases }),
        )
    }

    pub fn subject_verb_play_from_graveyard_until_eot(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PlayFromGraveyardUntilEot),
        )
    }

    pub fn subject_verb_ring_tempts_you(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::RingTemptsYou),
        )
    }

    pub fn subject_verb_venture_into_dungeon(
        player: PlayerAst,
        undercity_if_no_active: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::VentureIntoDungeon {
                undercity_if_no_active,
            }),
        )
    }

    pub fn subject_verb_become_monarch(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeMonarch),
        )
    }

    pub fn subject_verb_take_initiative(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::TakeInitiative),
        )
    }

    pub fn subject_verb_create_emblem(player: PlayerAst, emblem: EmblemDescriptionAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Tokens(TokenActionAst::CreateEmblem { emblem }),
        )
    }

    pub fn subject_verb_lose_game(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Game(GameActionAst::LoseGame),
        )
    }

    pub fn subject_verb_win_game(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Game(GameActionAst::WinGame),
        )
    }

    pub fn subject_verb_detain(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Detain { target }),
        )
    }

    pub fn subject_verb_goad(target: TargetAst) -> Self {
        Self::subject_verb_goad_for(target, Until::YourNextTurn)
    }

    pub fn subject_verb_goad_for(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Goad { target, duration }),
        )
    }

    pub fn subject_verb_suspect(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Suspect { target }),
        )
    }

    pub fn subject_verb_clear_suspected(target: Option<TargetAst>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::ClearSuspected { target }),
        )
    }

    pub fn subject_verb_heal_damage(target: TargetAst, amount: Option<Value>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Damage(DamageActionAst::HealDamage { target, amount }),
        )
    }

    pub fn subject_verb_remove_from_combat(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::RemoveFromCombat { target }),
        )
    }

    pub fn subject_verb_flip(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Flip { target }),
        )
    }

    pub fn subject_verb_regenerate(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Regenerate {
                target,
                follow_up_effects: Vec::new(),
            }),
        )
    }

    pub fn subject_verb_regenerate_with_follow_up_effects(
        target: TargetAst,
        follow_up_effects: Vec<EffectAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Regenerate {
                target,
                follow_up_effects,
            }),
        )
    }

    pub fn subject_verb_regenerate_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::RegenerateAll { filter }),
        )
    }

    pub fn subject_verb_sacrifice(
        player: PlayerAst,
        filter: ObjectFilter,
        count: u32,
        target: Option<TargetAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice {
                filter,
                count,
                target,
                one_of_referenced_set: false,
            }),
        )
    }

    pub fn with_sacrifice_one_of_referenced_set(mut self) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice {
                    one_of_referenced_set,
                    ..
                }),
            ..
        }) = &mut self
        {
            *one_of_referenced_set = true;
        }
        self
    }

    pub fn subject_verb_sacrifice_all(player: PlayerAst, filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { filter }),
        )
    }
}
