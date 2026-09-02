use super::*;
use crate::model::document_program::CompilerDocumentProgramAst;

/// One mode of an `EffectAst::ChooseOneOf` modal choice: a label shown to the
/// player and the effects that resolve when that mode is chosen.
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseOneModeAst {
    pub description: String,
    pub effects: Vec<EffectAst>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectAst {
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
    UnlessPays {
        effects: Vec<EffectAst>,
        player: PlayerAst,
        cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
        /// The Oracle clause says the cost may be paid before the delayed
        /// step, rather than when the delayed consequence resolves.
        before_delayed_step: bool,
    },
    UnlessAction {
        effects: Vec<EffectAst>,
        alternative: Vec<EffectAst>,
        player: PlayerAst,
    },
    DelayedUntilNextEndStep {
        player: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextCleanupStep {
        player: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextUntapStep {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextUpkeep {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextDrawStep {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextMainPhase {
        player: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextFirstMainPhase {
        player: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    DelayedUntilEndStepOfExtraTurn {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    DelayedUntilEndOfCombat {
        effects: Vec<EffectAst>,
    },
    DelayedTriggerThisTurn {
        trigger: TriggerSpec,
        effects: Vec<EffectAst>,
        one_shot: bool,
        until_end_of_combat: bool,
        attach_to_previous_ability: bool,
    },
    /// Register a repeating or one-shot delayed trigger with an explicit
    /// duration. This is distinct from granting an object a temporary
    /// triggered ability: the registration captures referenced objects when
    /// this effect resolves and then watches them independently.
    DelayedTriggerForDuration {
        trigger: TriggerSpec,
        effects: Vec<EffectAst>,
        one_shot: bool,
        duration: Until,
        either_of_watched_objects: bool,
        /// Keep the registration active only while at least one object from
        /// the captured tag remains in this zone.
        while_any_tagged_object_in_zone: Option<(TagKey, Zone)>,
    },
    DelayedWhenLastObjectDiesThisTurn {
        filter: Option<ObjectFilter>,
        effects: Vec<EffectAst>,
    },
    /// A delayed trigger tied to the object selected or created by the
    /// immediately preceding effect. Unlike the dies-this-turn form, this
    /// trigger has no turn-based expiry.
    DelayedWhenLastObjectLeavesBattlefield {
        filter: ObjectFilter,
        effects: Vec<EffectAst>,
    },
    Conditional {
        predicate: PredicateAst,
        if_true: Vec<EffectAst>,
        if_false: Vec<EffectAst>,
    },
    /// A resolution-time gate authored after the effect as
    /// "<effect> if <predicate>". Keeping this distinct from an ordinary
    /// conditional preserves word order and prevents trigger preparation from
    /// treating it as an intervening-if condition.
    TrailingIf {
        predicate: PredicateAst,
        effects: Vec<EffectAst>,
    },
    /// A resolution-time gate printed after the effect as
    /// "<effect> unless <positive predicate>". Keeping this distinct from a
    /// sole ordinary conditional prevents triggered-ability preparation from
    /// promoting it to an intervening-if condition.
    TrailingUnless {
        predicate: PredicateAst,
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
    ChooseObjects {
        filter: ObjectFilter,
        count: ChoiceCount,
        count_value: Option<Value>,
        player: PlayerAst,
        tag: TagKey,
    },
    /// Choose objects subject to a constraint on the selection as a whole.
    ChooseObjectsWithAggregateConstraint {
        filter: ObjectFilter,
        count: ChoiceCount,
        player: PlayerAst,
        tag: TagKey,
        constraint: crate::effect::ChoiceAggregateConstraint,
    },
    ChooseObjectsBottomOfLibrary {
        filter: ObjectFilter,
        count: ChoiceCount,
        count_value: Option<Value>,
        player: PlayerAst,
        tag: TagKey,
    },
    /// Choose from the top boundary of a library while retaining an explicit
    /// chooser. This composes the existing runtime `ChooseObjectsEffect`
    /// `top_only` capability with later tagged zone moves, which is required
    /// for face-down exile procedures where `ExileTopOfLibraryEffect` (always
    /// public) is not the correct primitive.
    ChooseObjectsTopOfLibrary {
        filter: ObjectFilter,
        count: ChoiceCount,
        count_value: Option<Value>,
        player: PlayerAst,
        tag: TagKey,
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
        tag: TagKey,
        zone: Zone,
    },
    ChooseObjectsAcrossZones {
        filter: ObjectFilter,
        count: ChoiceCount,
        count_value: Option<Value>,
        player: PlayerAst,
        tag: TagKey,
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
    /// Lower `effect` (which must lower to a single runtime effect) under a
    /// fresh internal effect id, then emit an `if_then(id, DidNotHappen,
    /// otherwise)`. The effect id stays internal to lowering and is never
    /// exposed in the AST. Lowers to `Effect::with_id` + `Effect::if_then`.
    IfEffectDidNotHappen {
        effect: Box<EffectAst>,
        otherwise: Vec<EffectAst>,
    },
    /// Lower one producer under a fresh internal effect id, then gate
    /// `if_true` on a typed predicate over that exact producer's outcome.
    /// The internal id is compiler bookkeeping and never appears in parsed
    /// card text.
    IfEffectResult {
        effect: Box<EffectAst>,
        predicate: crate::effect::EffectPredicate,
        if_true: Vec<EffectAst>,
    },
    /// Lower `effect` (which must lower to a single runtime effect) and apply
    /// `tag_all(tag)` to it, tagging every object the effect affects. Lowers to
    /// `Effect::tag_all`.
    TagAffected {
        effect: Box<EffectAst>,
        tag: TagKey,
    },
    DirectionalAdjacentPlayerControl {
        filter: ObjectFilter,
        left_option: String,
        right_option: String,
    },
    MayCastMatchingSpellWithoutPayingManaCost {
        player: PlayerAst,
        zone_owner: PlayerAst,
        filter: ObjectFilter,
        zone: Zone,
        payment: ironsmith_core::MayCastMatchingSpellPayment,
    },
    RepeatThisProcess,
    RepeatThisProcessMay,
    RepeatThisProcessOnce,
    RepeatEffects {
        count: Value,
        effects: Vec<EffectAst>,
    },
    May {
        effects: Vec<EffectAst>,
    },
    MayByPlayer {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    /// Offer each matching player, beginning with the effect controller and
    /// proceeding in turn order, the option to perform `effects`. Stop after
    /// one accepts.
    AnyPlayerMay {
        players: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    ResolvedIfResult {
        condition: EffectId,
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
    ResolvedWhenResult {
        condition: EffectId,
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
    IfResult {
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
    WhenResult {
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
    ForEachOpponent {
        effects: Vec<EffectAst>,
    },
    ForEachPlayersFiltered {
        filter: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    ForEachPlayer {
        effects: Vec<EffectAst>,
    },
    ForEachTargetPlayers {
        count: ChoiceCount,
        filter: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    ForEachObject {
        filter: ObjectFilter,
        effects: Vec<EffectAst>,
    },
    ForEachTagged {
        tag: TagKey,
        effects: Vec<EffectAst>,
    },
    /// Iterate a tagged result while binding `IteratedPlayer` to the
    /// controller recorded by the latest block event against `blocker_tag`.
    /// The ordinary `ForEachTagged` continues to use the result snapshot's
    /// controller at the time it was tagged.
    ForEachTaggedWithControllerAtLastBlockedBy {
        tag: TagKey,
        blocker_tag: TagKey,
        effects: Vec<EffectAst>,
    },
    /// Moves every object tagged `tag` to `zone`, preserving each object's
    /// controller. Lowers to `for_each_tagged(tag, [move(Iterated, zone)])`.
    /// Unlike a hand-written `ForEachTagged` whose body references `it`, this
    /// keeps the iterated reference internal to lowering, so the iteration does
    /// not surface a bare `it` that would be mistaken for an outer (triggering)
    /// object reference.
    MoveTaggedGroupToZone {
        tag: TagKey,
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
        into: TagKey,
    },
    ForEachOpponentDoesNot {
        effects: Vec<EffectAst>,
        predicate: Option<PredicateAst>,
    },
    ForEachPlayerDoesNot {
        effects: Vec<EffectAst>,
        predicate: Option<PredicateAst>,
    },
    ForEachOpponentDid {
        effects: Vec<EffectAst>,
        predicate: Option<PredicateAst>,
        result_predicate: IfResultPredicate,
    },
    ForEachPlayerDid {
        effects: Vec<EffectAst>,
        predicate: Option<PredicateAst>,
        result_predicate: IfResultPredicate,
    },
    ForEachTaggedPlayer {
        tag: TagKey,
        effects: Vec<EffectAst>,
    },
    RepeatProcess {
        effects: Vec<EffectAst>,
        continue_effect_index: usize,
        continue_predicate: IfResultPredicate,
    },
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
        tag: TagKey,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::DrawForEachTaggedMatching { tag, filter },
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
            SubjectVerbActionAst::GrantNextSpellAbilityThisTurn {
                filter,
                ability: Box::new(ability),
            },
        )
    }

    pub fn subject_verb_may_move_to_zone(player: PlayerAst, target: TargetAst, zone: Zone) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::MayMoveToZone { target, zone },
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
        looked_tag: TagKey,
        chosen_tag: TagKey,
        order: LibraryBottomOrderAst,
    ) -> Vec<Self> {
        let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
        choose_filter.zone = Some(Zone::Library);
        vec![
            Self::SnapshotLastObjectTag {
                into: looked_tag.clone(),
            },
            Self::ChooseTaggedObjectsInZone {
                filter: choose_filter,
                count,
                player,
                tag: chosen_tag.clone(),
                zone: Zone::Library,
            },
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
        looked_tag: TagKey,
        chosen_tag: TagKey,
        order: LibraryBottomOrderAst,
    ) -> Vec<Self> {
        let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
        choose_filter.zone = Some(Zone::Library);
        vec![
            Self::SnapshotLastObjectTag {
                into: looked_tag.clone(),
            },
            Self::ChooseTaggedObjectsInZone {
                filter: choose_filter,
                count,
                player,
                tag: chosen_tag.clone(),
                zone: Zone::Library,
            },
            Self::ForEachTagged {
                tag: chosen_tag.clone(),
                effects: vec![Self::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None),
                    Zone::Library,
                    true,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            },
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
        looked_tag: TagKey,
        chosen_tag: TagKey,
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
        looked_tag: TagKey,
        chosen_tag: TagKey,
        chosen_zone: Zone,
        rest_zone: Zone,
    ) -> Vec<Self> {
        let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
        choose_filter.zone = Some(Zone::Library);

        vec![
            Self::SnapshotLastObjectTag {
                into: looked_tag.clone(),
            },
            Self::ChooseTaggedObjectsInZone {
                filter: choose_filter,
                count,
                player,
                tag: chosen_tag.clone(),
                zone: Zone::Library,
            },
            Self::MoveTaggedGroupToZone {
                tag: chosen_tag.clone(),
                zone: chosen_zone,
            },
            Self::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::Implicit,
                SubjectVerbActionAst::PutTaggedRemainderInZone {
                    tag: looked_tag,
                    keep_tagged: chosen_tag,
                    zone: rest_zone,
                    surface: ironsmith_core::LibraryRemainderSurface::Rest,
                },
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
            SubjectVerbActionAst::GrantProtectionChoice {
                target,
                chooser,
                allow_colorless,
                allow_artifacts,
                choose_card_type,
            },
        )
    }

    pub fn subject_verb_prevent_all_combat_damage(duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventAllCombatDamage { duration },
        )
    }

    pub fn subject_verb_assign_no_combat_damage(source: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::AssignNoCombatDamage { source, duration },
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
            SubjectVerbActionAst::PreventAllCombatDamageFromSource {
                duration,
                source,
                source_would_deal_surface,
            },
        )
    }

    pub fn subject_verb_prevent_all_combat_damage_from_source_filter(
        source_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventAllCombatDamageFromSourceFilter {
                duration,
                source_filter,
                excluded_source_target: None,
            },
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
            SubjectVerbActionAst::PreventAllCombatDamageFromSourceFilter {
                duration,
                source_filter,
                excluded_source_target: Some(excluded_source_target),
            },
        )
    }

    pub fn subject_verb_prevent_all_combat_damage_to_players(duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventAllCombatDamageToPlayers { duration },
        )
    }

    pub fn subject_verb_prevent_all_combat_damage_to_you(duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventAllCombatDamageToYou { duration },
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
            SubjectVerbActionAst::PreventNextTimeDamage {
                source,
                target,
                reflect_damage_to_source_controller,
                follow_up_effects: Vec::new(),
            },
        )
    }

    pub fn subject_verb_replace_next_damage_to_target(
        target: TargetAst,
        damage_target_tag: TagKey,
        replacement_effects: Vec<EffectAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ReplaceNextDamageToTarget {
                target,
                damage_target_tag,
                replacement_effects,
            },
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
            SubjectVerbActionAst::PreventDamage {
                amount,
                target,
                duration,
                source_of_your_choice,
                protect_you_and_permanents_you_control,
                follow_up_effects,
            },
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
            SubjectVerbActionAst::PreventAllDamageToTarget {
                target,
                duration,
                source_of_your_choice,
                source_choice_shares_activation_mana_color: false,
                source_target: None,
            },
        )
    }

    pub fn subject_verb_prevent_all_damage_to_target_with_mana_color_source_choice(
        target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventAllDamageToTarget {
                target,
                duration,
                source_of_your_choice: true,
                source_choice_shares_activation_mana_color: true,
                source_target: None,
            },
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
            SubjectVerbActionAst::PreventAllDamageToTarget {
                target,
                duration,
                source_of_your_choice: false,
                source_choice_shares_activation_mana_color: false,
                source_target: Some(source_target),
            },
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
            SubjectVerbActionAst::PreventAllDamageToTargetFromSourceFilter {
                target,
                duration,
                source_filter,
            },
        )
    }

    pub fn subject_verb_prevent_all_damage_from_source_filter(
        source_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventAllDamageFromSourceFilter {
                duration,
                source_filter,
            },
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
            SubjectVerbActionAst::PreventDamageToTargetPutCounters {
                amount,
                target,
                duration,
                counter_type,
            },
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
            SubjectVerbActionAst::PreventDamageEach {
                amount,
                filter,
                duration,
            },
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
            SubjectVerbActionAst::CopySpell {
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
            },
        )
    }

    pub fn with_copy_count_surface(
        mut self,
        surface: ironsmith_core::effect::CopyCountSurface,
    ) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpell { count_surface, .. },
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
                SubjectVerbActionAst::CopySpell {
                    target_reference_kind,
                    ..
                },
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
                SubjectVerbActionAst::CopySpell {
                    target_reference_pronoun,
                    ..
                },
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
                SubjectVerbActionAst::CopySpell {
                    all_matches: action_all_matches,
                    ..
                },
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
                SubjectVerbActionAst::CopySpell {
                    added_card_types: action_added_card_types,
                    ..
                },
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
                SubjectVerbActionAst::CopySpell {
                    added_subtypes: action_added_subtypes,
                    ..
                },
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
                SubjectVerbActionAst::CopySpell {
                    set_base_power_toughness: action_set_base_power_toughness,
                    ..
                },
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
                SubjectVerbActionAst::CopySpell {
                    set_colors: action_set_colors,
                    ..
                },
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
            SubjectVerbActionAst::CopySpellForEachTarget {
                target,
                object_filter,
                player_filter,
                player,
                exclude_current_targets,
                removed_supertypes,
            },
        )
    }

    pub fn subject_verb_scale_x_value(target: TargetAst, multiplier: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ScaleXValue { target, multiplier },
        )
    }

    pub fn subject_verb_put_tagged_remainder_on_bottom_of_library(
        tag: TagKey,
        keep_tagged: Option<TagKey>,
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
        tag: TagKey,
        keep_tagged: Option<TagKey>,
        order: LibraryBottomOrderAst,
        player: PlayerAst,
        surface: ironsmith_core::LibraryRemainderSurface,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                tag,
                keep_tagged,
                order,
                player,
                surface,
            },
        )
    }

    pub fn subject_verb_cast_tagged(
        tag: TagKey,
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
        tag: TagKey,
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
        tag: TagKey,
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
            SubjectVerbActionAst::CastTagged {
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
            },
        )
    }

    pub fn with_copy_instruction_surface(
        mut self,
        surface: ironsmith_core::effect::CopyInstructionSurface,
    ) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CastTagged {
                    copy_instruction_surface,
                    ..
                },
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
        Self::MayCastMatchingSpellWithoutPayingManaCost {
            player,
            zone_owner: player,
            filter,
            zone,
            payment: ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost,
        }
    }

    pub fn may_cast_matching_spell_without_paying_mana_cost_from_zone_owner(
        player: PlayerAst,
        zone_owner: PlayerAst,
        filter: ObjectFilter,
        zone: Zone,
    ) -> Self {
        Self::MayCastMatchingSpellWithoutPayingManaCost {
            player,
            zone_owner,
            filter,
            zone,
            payment: ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost,
        }
    }

    pub fn may_cast_matching_spell_with_alternative_cost(
        player: PlayerAst,
        filter: ObjectFilter,
        zone: Zone,
        kind: crate::filter::AlternativeCastKind,
    ) -> Self {
        Self::MayCastMatchingSpellWithoutPayingManaCost {
            player,
            zone_owner: player,
            filter,
            zone,
            payment: ironsmith_core::MayCastMatchingSpellPayment::AlternativeCost(kind),
        }
    }

    pub fn subject_verb_grant_play_tagged_until_end_of_turn(
        tag: TagKey,
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
        tag: TagKey,
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
            SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
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
            },
        )
    }

    pub fn subject_verb_grant_play_tagged_until_end_of_turn_from_current_zone_with_optional_surface(
        tag: TagKey,
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
            SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
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
            },
        )
    }

    pub fn subject_verb_grant_play_tagged_until_end_of_turn_while_on_top_of_library(
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        allow_any_color_for_cast: impl Into<ironsmith_core::value_model::ManaSpendMode>,
    ) -> Self {
        let allow_any_color_for_cast = allow_any_color_for_cast.into();
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
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
            },
        )
    }

    pub fn subject_verb_grant_play_tagged_until_source_exiles_another(
        tag: TagKey,
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
            SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
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
            },
        )
    }

    pub fn subject_verb_grant_tagged_spell_alternative_cost_pay_life_by_mana_value_until_end_of_turn(
        tag: TagKey,
        player: PlayerAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                tag,
                player,
            },
        )
    }

    pub fn subject_verb_grant_play_tagged_until_your_next_turn(
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: impl Into<ironsmith_core::value_model::ManaSpendMode>,
    ) -> Self {
        let allow_any_color_for_cast = allow_any_color_for_cast.into();
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
                until_next_end_step: false,
                max_plays: None,
            },
        )
    }

    pub fn subject_verb_grant_play_tagged_until_your_next_end_step(
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: impl Into<ironsmith_core::value_model::ManaSpendMode>,
    ) -> Self {
        let allow_any_color_for_cast = allow_any_color_for_cast.into();
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
                until_next_end_step: true,
                max_plays: None,
            },
        )
    }

    /// Apply a shared deferred-use limit to a tagged play permission.
    ///
    /// The tagged collection remains intact so the player chooses which card
    /// to play at play/cast time rather than during effect resolution.
    pub fn with_tagged_play_max_plays(mut self, limit: Option<u32>) -> Self {
        if let Self::SubjectVerb(subject_verb) = &mut self {
            match &mut subject_verb.action {
                SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn { max_plays, .. }
                | SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn { max_plays, .. } => {
                    *max_plays = limit;
                }
                _ => {}
            }
        }
        self
    }

    pub fn subject_verb_grant_play_tagged_for_as_long_as_exiled(
        tag: TagKey,
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
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                filter,
                during_turns_counter_put_on_source: None,
                spell_cost_increase: None,
                lands_enter_tapped: false,
            },
        )
    }

    pub fn subject_verb_grant_play_tagged_during_turns_counter_put_on_source(
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        counter_type: crate::object::CounterType,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player,
                allow_land,
                without_paying_mana_cost: false,
                allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode::Normal,
                filter: None,
                during_turns_counter_put_on_source: Some(counter_type),
                spell_cost_increase: None,
                lands_enter_tapped: false,
            },
        )
    }

    pub fn subject_verb_grant_play_tagged_with_play_constraints(
        tag: TagKey,
        player: PlayerAst,
        spell_cost_increase: Option<ManaCost>,
        lands_enter_tapped: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player,
                allow_land: true,
                without_paying_mana_cost: false,
                allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode::Normal,
                filter: None,
                during_turns_counter_put_on_source: None,
                spell_cost_increase,
                lands_enter_tapped,
            },
        )
    }

    pub fn subject_verb_grant_play_tagged_for_as_long_as_you_control_source(
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: impl Into<ironsmith_core::value_model::ManaSpendMode>,
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
    ) -> Self {
        let allow_any_color_for_cast = allow_any_color_for_cast.into();
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsYouControlSource {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
                surface,
            },
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
            SubjectVerbActionAst::ReturnToBattlefield {
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
            },
        )
    }

    pub fn with_graveyard_or_exile_return_origin(mut self) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ReturnToBattlefield {
                    from_graveyard_or_exile,
                    ..
                },
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
                SubjectVerbActionAst::ReturnToBattlefield {
                    top_only: return_top_only,
                    ..
                },
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
            SubjectVerbActionAst::ReturnAllToBattlefield {
                filter,
                tapped,
                face_down,
                controller,
                verb_surface: ironsmith_core::MoveToZoneVerbSurface::Return,
            },
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
            SubjectVerbActionAst::ReturnAllToBattlefield {
                filter,
                tapped,
                face_down,
                controller,
                verb_surface: ironsmith_core::MoveToZoneVerbSurface::Put,
            },
        )
    }

    pub fn subject_verb_exile_until_source_leaves(target: TargetAst, face_down: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ExileUntilSourceLeaves {
                target,
                duration: ironsmith_core::ExileUntilDuration::SourceLeavesBattlefield,
                leave_watcher: None,
                face_down,
                all: false,
                explicit_return_surface: false,
            },
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
            SubjectVerbActionAst::ExileUntilSourceLeaves {
                target,
                duration: ironsmith_core::ExileUntilDuration::SourceLeavesBattlefield,
                leave_watcher: Some(leave_watcher),
                face_down,
                all: false,
                explicit_return_surface: false,
            },
        )
    }

    pub fn subject_verb_exile_all_until_source_leaves(target: TargetAst, face_down: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ExileUntilSourceLeaves {
                target,
                duration: ironsmith_core::ExileUntilDuration::SourceLeavesBattlefield,
                leave_watcher: None,
                face_down,
                all: true,
                explicit_return_surface: false,
            },
        )
    }

    pub fn subject_verb_exile_until_opponent_becomes_monarch(
        target: TargetAst,
        face_down: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ExileUntilSourceLeaves {
                target,
                duration: ironsmith_core::ExileUntilDuration::OpponentBecomesMonarch,
                leave_watcher: None,
                face_down,
                all: false,
                explicit_return_surface: false,
            },
        )
    }

    pub fn with_explicit_exile_return_surface(mut self) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ExileUntilSourceLeaves {
                    explicit_return_surface,
                    ..
                },
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
            SubjectVerbActionAst::MoveToZone {
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
            },
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
            SubjectVerbActionAst::MoveToZone {
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
            },
        )
    }

    pub fn with_destination_player_surface(mut self, player: Option<PlayerAst>) -> Self {
        if let Some(player) = player
            && let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::MoveToZone {
                destination_player_surface,
                ..
            } = &mut subject_verb.action
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
            && let SubjectVerbActionAst::MoveToZone {
                library_order,
                library_order_chooser,
                ..
            } = &mut subject_verb.action
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
                SubjectVerbActionAst::MoveToZone { verb_surface, .. }
                | SubjectVerbActionAst::ReturnAllToBattlefield { verb_surface, .. } => {
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
                SubjectVerbActionAst::Exile {
                    source_top_only, ..
                }
                | SubjectVerbActionAst::MoveToZone {
                    source_top_only, ..
                } => *source_top_only = true,
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
            action: SubjectVerbActionAst::MoveToZone { .. },
        }) = &mut self
        {
            subject.player = actor;
        }
        self
    }

    pub fn with_move_to_zone_plural_surface(mut self) -> Self {
        if let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::MoveToZone {
                target_plural_surface,
                ..
            }
            | SubjectVerbActionAst::Exile {
                target_plural_surface,
                ..
            } = &mut subject_verb.action
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
            && let SubjectVerbActionAst::MoveToZone {
                target_reference_surface,
                ..
            } = &mut subject_verb.action
        {
            *target_reference_surface = Some(surface);
        }
        self
    }

    pub fn with_move_to_zone_transformed(mut self) -> Self {
        if let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::MoveToZone {
                battlefield_transformed,
                ..
            } = &mut subject_verb.action
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
            && let SubjectVerbActionAst::MoveToZone {
                destination_player_reference_surface,
                ..
            } = &mut subject_verb.action
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
                SubjectVerbActionAst::MoveToZone {
                    exiled_with_source_surface,
                    ..
                }
                | SubjectVerbActionAst::ReturnToHand {
                    exiled_with_source_surface,
                    ..
                }
                | SubjectVerbActionAst::ReturnAllToHand {
                    exiled_with_source_surface,
                    ..
                } => *exiled_with_source_surface = Some(surface),
                _ => {}
            }
        }
        self
    }

    pub fn subject_verb_move_to_library_top_or_bottom_choice(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target },
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
        tag: TagKey,
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
        tag: TagKey,
        source_tags: Vec<TagKey>,
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
            SubjectVerbActionAst::Pump {
                power,
                toughness,
                target,
                duration,
                condition,
                set_quantifier_surface: None,
            },
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
            SubjectVerbActionAst::SetBasePowerToughness {
                power,
                toughness,
                target,
                duration,
                set_quantifier_surface: None,
            },
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
            SubjectVerbActionAst::BecomeBasePtCreature {
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
            },
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
            SubjectVerbActionAst::Pump {
                set_quantifier_surface,
                ..
            }
            | SubjectVerbActionAst::PumpAll {
                set_quantifier_surface,
                ..
            }
            | SubjectVerbActionAst::SetBasePowerToughness {
                set_quantifier_surface,
                ..
            }
            | SubjectVerbActionAst::BecomeBasePtCreature {
                set_quantifier_surface,
                ..
            }
            | SubjectVerbActionAst::GrantAbilitiesAll {
                set_quantifier_surface,
                ..
            }
            | SubjectVerbActionAst::RemoveAbilitiesAll {
                set_quantifier_surface,
                ..
            }
            | SubjectVerbActionAst::GrantAbilitiesToTarget {
                set_quantifier_surface,
                ..
            } => *set_quantifier_surface = Some(surface),
            _ => {}
        }
        self
    }

    pub fn subject_verb_set_base_power(power: Value, target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::SetBasePower {
                power,
                target,
                duration,
            },
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
            SubjectVerbActionAst::PumpForEach {
                power_per,
                toughness_per,
                target,
                count,
                duration,
            },
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
            SubjectVerbActionAst::PumpAll {
                filter,
                power,
                toughness,
                duration,
                set_quantifier_surface: None,
            },
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
            SubjectVerbActionAst::PumpByLastEffect {
                power,
                toughness,
                target,
                duration,
                includes_this_way,
            },
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
            SubjectVerbActionAst::AddCardTypes {
                target,
                card_types,
                duration,
            },
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
            SubjectVerbActionAst::RemoveCardTypes {
                target,
                card_types,
                duration,
            },
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
            SubjectVerbActionAst::SetCardTypes {
                target,
                card_types,
                duration,
            },
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
            SubjectVerbActionAst::AddSubtypes {
                target,
                subtypes,
                duration,
            },
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
            SubjectVerbActionAst::RemoveSubtypes {
                target,
                subtypes,
                duration,
            },
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
            SubjectVerbActionAst::SetCreatureSubtypes {
                target,
                subtypes,
                duration,
            },
        )
    }

    pub fn subject_verb_become_saddled_until_end_of_turn(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::BecomeSaddledUntilEndOfTurn { target },
        )
    }

    pub fn subject_verb_add_colors(target: TargetAst, colors: ColorSet, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::AddColors {
                target,
                colors,
                duration,
            },
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
            SubjectVerbActionAst::AddAllSubtypesOfFamily {
                target,
                family,
                duration,
            },
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
            SubjectVerbActionAst::RemoveAllSubtypesOfFamily {
                target,
                family,
                duration,
            },
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
            SubjectVerbActionAst::BecomeAuraEnchantment {
                target,
                attachment_filter,
                granted_abilities,
                duration,
            },
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
            SubjectVerbActionAst::BecomeBasicLandType {
                target,
                subtype,
                duration,
            },
        )
    }

    pub fn subject_verb_set_colors(target: TargetAst, colors: ColorSet, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::SetColors {
                target,
                colors,
                duration,
            },
        )
    }

    pub fn subject_verb_make_colorless(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::MakeColorless { target, duration },
        )
    }

    pub fn subject_verb_become_basic_land_type_choice(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::BecomeBasicLandTypeChoice { target, duration },
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
            SubjectVerbActionAst::BecomeCreatureTypeChoice {
                target,
                duration,
                excluded_subtypes,
            },
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
            SubjectVerbActionAst::BecomeColorChoice {
                target,
                duration,
                allow_multiple,
            },
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
            SubjectVerbActionAst::BecomeCopy {
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
            },
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
            SubjectVerbActionAst::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
                condition: None,
                set_quantifier_surface: None,
                lock_filter_at_resolution: true,
            },
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
            SubjectVerbActionAst::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
                condition: Some(condition),
                set_quantifier_surface: None,
                lock_filter_at_resolution: true,
            },
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
            SubjectVerbActionAst::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
                condition: None,
                set_quantifier_surface: None,
                lock_filter_at_resolution: false,
            },
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
            SubjectVerbActionAst::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
                condition: Some(condition),
                set_quantifier_surface: None,
                lock_filter_at_resolution: false,
            },
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
            SubjectVerbActionAst::RemoveAbilitiesAll {
                filter,
                abilities,
                duration,
                condition: None,
                set_quantifier_surface: None,
            },
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
            SubjectVerbActionAst::RemoveAbilitiesAll {
                filter,
                abilities,
                duration,
                condition: Some(condition),
                set_quantifier_surface: None,
            },
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
            SubjectVerbActionAst::GrantAbilitiesChoiceAll {
                filter,
                abilities,
                duration,
            },
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
            SubjectVerbActionAst::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
                condition: None,
                set_quantifier_surface: None,
            },
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
            SubjectVerbActionAst::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
                condition: Some(condition),
                set_quantifier_surface: None,
            },
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
            SubjectVerbActionAst::GrantToTarget {
                target,
                grantable: Box::new(grantable),
                duration,
            },
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
            SubjectVerbActionAst::GrantBySpec {
                spec: Box::new(spec),
                player,
                duration,
            },
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
            SubjectVerbActionAst::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            },
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
            SubjectVerbActionAst::GrantAbilitiesChoiceToTarget {
                target,
                abilities,
                duration,
            },
        )
    }

    pub fn subject_verb_consult_top_of_library(
        player: PlayerAst,
        mode: LibraryConsultModeAst,
        filter: ObjectFilter,
        stop_rule: LibraryConsultStopRuleAst,
        all_tag: TagKey,
        match_tag: TagKey,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ConsultTopOfLibrary {
                player,
                mode,
                filter,
                stop_rule,
                max_exposed: None,
                all_tag,
                match_tag,
            },
        )
    }

    pub fn subject_verb_consult_top_of_library_with_max_exposed(
        player: PlayerAst,
        mode: LibraryConsultModeAst,
        filter: ObjectFilter,
        stop_rule: LibraryConsultStopRuleAst,
        max_exposed: Value,
        all_tag: TagKey,
        match_tag: TagKey,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ConsultTopOfLibrary {
                player,
                mode,
                filter,
                stop_rule,
                max_exposed: Some(max_exposed),
                all_tag,
                match_tag,
            },
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
            SubjectVerbActionAst::SearchLibrary {
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
            },
        )
    }

    pub fn with_search_zones(mut self, zones: Vec<Zone>) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::SearchLibrary { search_zones, .. },
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
                SubjectVerbActionAst::SearchLibrary {
                    enters_with_counters,
                    ..
                },
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
            SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget {
                amount,
                protected_target: None,
                destination: RedirectNextTimeDamageDestinationAst::TargetObject,
                destination_target: Some(target),
            },
        )
    }

    pub fn subject_verb_redirect_next_damage_to_controller(
        amount: Value,
        protected_target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget {
                amount,
                protected_target: Some(protected_target),
                destination: RedirectNextTimeDamageDestinationAst::Controller,
                destination_target: None,
            },
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
            SubjectVerbActionAst::RedirectNextTimeDamageToSource {
                source,
                target,
                destination,
                destination_target: None,
                all_this_turn: false,
            },
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
            SubjectVerbActionAst::RedirectNextTimeDamageToSource {
                source,
                target,
                destination: RedirectNextTimeDamageDestinationAst::TargetObject,
                destination_target: Some(destination_target),
                all_this_turn: false,
            },
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
            SubjectVerbActionAst::RedirectNextTimeDamageToSource {
                source,
                target,
                destination,
                destination_target: None,
                all_this_turn: true,
            },
        )
    }

    pub fn subject_verb_redirect_all_damage_this_turn_by_source_to_source_controller(
        source: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController { source },
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
            SubjectVerbActionAst::RedirectAllDamageThisTurnToTarget {
                player_filter,
                object_filter,
                target,
            },
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
            SubjectVerbActionAst::Meld {
                result_name: result_name.into(),
                enters_tapped,
                enters_attacking,
            },
        )
    }

    pub fn subject_verb_search_library_slots_to_hand(
        player: PlayerAst,
        slots: Vec<SearchLibrarySlotAst>,
        reveal: bool,
        progress_tag: TagKey,
    ) -> Self {
        Self::subject_verb_search_library_slots(player, slots, Zone::Hand, reveal, progress_tag)
    }

    pub fn subject_verb_search_library_slots(
        player: PlayerAst,
        slots: Vec<SearchLibrarySlotAst>,
        destination: Zone,
        reveal: bool,
        progress_tag: TagKey,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::SearchLibrarySlotsToHand {
                slots,
                destination,
                reveal,
                progress_tag,
            },
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
            SubjectVerbActionAst::RetargetStackObject {
                target,
                mode,
                require_change,
                copy_reference_plural: false,
            },
        )
    }

    /// Preserve an authored plural copy back-reference ("the copies").
    pub fn with_retarget_plural_copy_reference(mut self, plural: bool) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::RetargetStackObject {
                    copy_reference_plural,
                    ..
                },
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
            SubjectVerbActionAst::GrantAbilityToSource {
                ability: Box::new(ability),
                duration,
            },
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
            SubjectVerbActionAst::ExchangeControl {
                filter,
                count,
                shared_type,
            },
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
            SubjectVerbActionAst::ExchangeControlHeterogeneous {
                permanent1,
                permanent2,
                shared_type,
            },
        )
    }

    pub fn subject_verb_destroy_all_attached_to(filter: ObjectFilter, target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DestroyAllAttachedTo { filter, target },
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
            SubjectVerbActionAst::ExileAllAttachedTo {
                filter,
                target,
                face_down,
            },
        )
    }

    pub fn subject_verb_attach(object: TargetAst, target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Attach { object, target },
        )
    }

    pub fn subject_verb_unattach(object: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Unattach { object },
        )
    }

    pub fn subject_verb_enchant(filter: AuraAttachmentFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Enchant { filter },
        )
    }

    pub fn subject_verb_exile_when_source_leaves(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ExileWhenSourceLeaves { target },
        )
    }

    pub fn subject_verb_sacrifice_source_when_leaves(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::SacrificeSourceWhenLeaves { target },
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
            SubjectVerbActionAst::RegisterZoneReplacement {
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
            },
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
            SubjectVerbActionAst::RegisterZoneReplacement {
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
            },
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
            SubjectVerbActionAst::RegisterZoneReplacement {
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
            },
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
            SubjectVerbActionAst::RegisterZoneReplacement {
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
            },
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
            SubjectVerbActionAst::RegisterFutureZoneReplacement {
                filter,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
                cause_policy,
                link_exiled_to_source,
            },
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
            SubjectVerbActionAst::RegisterDrawReplacement {
                player,
                replacement_effects,
                duration,
            },
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
            SubjectVerbActionAst::RegisterManaReplacement {
                source_filter,
                replacement_mana,
                mode,
            },
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
            SubjectVerbActionAst::RegisterDamagedBySourceZoneReplacement {
                filter,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
            },
        )
    }

    pub fn subject_verb_register_enter_under_control_replacement(
        filter: ObjectFilter,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RegisterEnterUnderControlReplacement { filter, duration },
        )
    }

    pub fn subject_verb_register_enter_tapped_replacement(
        filter: ObjectFilter,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RegisterEnterTappedReplacement { filter, duration },
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
            SubjectVerbActionAst::RegisterNextBatchEnterWithCounters {
                filter,
                counter_type,
                count,
            },
        )
    }

    pub fn subject_verb_choose_spell_cast_history(
        chooser: PlayerAst,
        cast_by: PlayerAst,
        filter: ObjectFilter,
        tag: TagKey,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            chooser,
            SubjectVerbActionAst::ChooseSpellCastHistory {
                cast_by,
                filter,
                tag,
            },
        )
    }

    pub fn subject_verb_damage(amount: Value, target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DealDamage {
                amount,
                target,
                unpreventable: false,
            },
        )
    }

    pub fn subject_verb_damage_each(amount: Value, filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DealDamageEach { amount, filter },
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
            SubjectVerbActionAst::DealDamageEqualToPower {
                source,
                amount,
                target,
                unpreventable: false,
            },
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
            SubjectVerbActionAst::DealDistributedDamage {
                amount,
                target,
                source,
                chooser,
                distribution,
            },
        )
    }

    pub fn subject_verb_proliferate(count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Proliferate { count },
        )
    }

    pub fn subject_verb_investigate(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Investigate { count },
        )
    }

    pub fn subject_verb_incubate(player: PlayerAst, amount: Value, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Incubate { amount, count },
        )
    }

    pub fn subject_verb_learn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Learn,
        )
    }

    pub fn subject_verb_emit_keyword_action(
        action: crate::events::KeywordActionKind,
        amount: u32,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::EmitKeywordAction { action, amount },
        )
    }

    pub fn subject_verb_amass(subtype: Option<Subtype>, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Amass { subtype, amount },
        )
    }

    pub fn subject_verb_bolster(amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Bolster { amount },
        )
    }

    pub fn subject_verb_support(amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Support { amount },
        )
    }

    pub fn subject_verb_adapt(amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Adapt { amount },
        )
    }

    pub fn subject_verb_monstrosity(amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Monstrosity { amount },
        )
    }

    pub fn subject_verb_discover(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Discover { count },
        )
    }

    pub fn subject_verb_fateseal(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Fateseal { count },
        )
    }

    pub fn subject_verb_populate(count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Populate {
                count,
                enters_tapped: false,
                enters_attacking: false,
                has_haste: false,
                sacrifice_at_next_end_step: false,
                exile_at_next_end_step: false,
                next_end_step_player: PlayerFilter::Any,
                exile_at_end_of_combat: false,
                sacrifice_at_end_of_combat: false,
            },
        )
    }

    pub fn subject_verb_explore(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Explore { target },
        )
    }

    pub fn subject_verb_endure(target: TargetAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Endure { target, amount },
        )
    }

    pub fn subject_verb_exploit() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Exploit,
        )
    }

    pub fn subject_verb_connive(target: TargetAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Connive { target, count },
        )
    }

    pub fn subject_verb_connive_iterated() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ConniveIterated,
        )
    }

    pub fn subject_verb_put_rest_on_bottom_of_library() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutRestOnBottomOfLibrary,
        )
    }

    pub fn subject_verb_dont_lose_this_mana_as_steps_and_phases_end_this_turn() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn,
        )
    }

    pub fn subject_verb_open_attraction(player: PlayerAst, reminder: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::OpenAttraction { reminder },
        )
    }

    pub fn subject_verb_manifest_top_card(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ManifestTopCardOfLibrary,
        )
    }

    pub fn subject_verb_cloak_top_card(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::CloakTopCardOfLibrary,
        )
    }

    pub fn subject_verb_manifest_from_hand(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ManifestCardFromHand,
        )
    }

    pub fn subject_verb_manifest_dread(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ManifestDread,
        )
    }

    pub fn subject_verb_earthbend(counters: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Earthbend { counters },
        )
    }

    pub fn subject_verb_behold(subtype: Subtype, count: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Behold { subtype, count },
        )
    }

    pub fn subject_verb_fight(creature1: TargetAst, creature2: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Fight {
                creature1,
                creature2,
                mutual_surface: false,
            },
        )
    }

    pub fn with_mutual_fight_surface(mut self) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Fight { mutual_surface, .. },
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
            SubjectVerbActionAst::FightIterated { creature2 },
        )
    }

    pub fn subject_verb_clash(opponent: ClashOpponentAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Clash { opponent },
        )
    }

    pub fn subject_verb_add_mana(player: PlayerAst, mana: Vec<ManaSymbol>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddMana { mana },
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
            SubjectVerbActionAst::AddManaScaled { mana, amount },
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
            SubjectVerbActionAst::AddManaAnyColor {
                amount,
                available_colors,
                distinct_colors,
            },
        )
    }

    pub fn subject_verb_add_mana_any_one_color(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddManaAnyOneColor { amount },
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
            SubjectVerbActionAst::AddManaChosenColor {
                amount,
                fixed_option,
            },
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
            SubjectVerbActionAst::AddManaFromLandCouldProduce {
                amount,
                land_filter,
                allow_colorless,
                same_type,
                mana_type_source,
            },
        )
    }

    pub fn subject_verb_add_mana_colors_among(player: PlayerAst, filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddManaColorsAmong { filter },
        )
    }

    pub fn subject_verb_add_one_mana_any_color_among(
        player: PlayerAst,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddOneManaAnyColorAmong {
                filter,
                choose_color_of_object_surface: false,
            },
        )
    }

    pub fn subject_verb_choose_color_of_object_add_mana(
        player: PlayerAst,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddOneManaAnyColorAmong {
                filter,
                choose_color_of_object_surface: true,
            },
        )
    }

    pub fn subject_verb_add_mana_commander_identity(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddManaCommanderIdentity { amount },
        )
    }

    pub fn subject_verb_exchange_life_totals(player1: PlayerAst, player2: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player1,
            SubjectVerbActionAst::ExchangeLifeTotals { player2 },
        )
    }

    pub fn subject_verb_exchange_text_boxes(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ExchangeTextBoxes { target },
        )
    }

    pub fn subject_verb_exchange_zones(player: PlayerAst, zone1: Zone, zone2: Zone) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ExchangeZones { zone1, zone2 },
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
            SubjectVerbActionAst::ExchangeValues {
                left,
                right,
                duration,
            },
        )
    }

    pub fn subject_verb_exile_instead_of_graveyard_this_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ExileInsteadOfGraveyardThisTurn,
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
            SubjectVerbActionAst::ControlCombatChoicesThisTurn {
                attackers,
                blockers,
                this_combat,
            },
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
            SubjectVerbActionAst::ControlPlayer {
                player: target,
                duration,
            },
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
            SubjectVerbActionAst::ReduceNextSpellCostThisTurn { filter, reduction },
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
            SubjectVerbActionAst::ReduceMatchingSpellCostThisTurn {
                filter,
                reduction,
                duration,
                next_only: false,
            },
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
            SubjectVerbActionAst::ReduceMatchingSpellCostThisTurn {
                filter,
                reduction,
                duration: Until::EndOfTurn,
                next_only: true,
            },
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
            SubjectVerbActionAst::GainControl {
                target,
                duration,
                condition,
                controller_reference: None,
                source_reference_surface,
            },
        )
    }

    pub fn subject_verb_reveal_top(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::RevealTop,
        )
    }

    pub fn subject_verb_exile_top_of_library(
        player: PlayerAst,
        count: Value,
        tags: Vec<TagKey>,
        accumulated_tags: Vec<TagKey>,
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
        tags: Vec<TagKey>,
        accumulated_tags: Vec<TagKey>,
        surface: Option<ironsmith_core::ExileTopLibrarySurface>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ExileTopOfLibrary {
                count,
                surface,
                tags,
                accumulated_tags,
                face_down: false,
            },
        )
    }

    pub fn subject_verb_exile_top_of_library_face_down(
        player: PlayerAst,
        count: Value,
        accumulated_tag: TagKey,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ExileTopOfLibrary {
                count,
                surface: None,
                tags: Vec::new(),
                accumulated_tags: vec![accumulated_tag],
                face_down: true,
            },
        )
    }

    pub fn subject_verb_reveal_tagged(tag: TagKey) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RevealTagged { tag },
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
            SubjectVerbActionAst::PutOntoBattlefield {
                target,
                tapped,
                controller,
                cloak: false,
                shuffle_before: false,
            },
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
            SubjectVerbActionAst::PutOntoBattlefield {
                target,
                tapped,
                controller,
                cloak: true,
                shuffle_before,
            },
        )
    }

    pub fn subject_verb_reveal_cards_from_hand(
        player: PlayerAst,
        count: ChoiceCount,
        count_value: Option<Value>,
        tag: TagKey,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::RevealCardsFromHand {
                count,
                count_value,
                tag,
            },
        )
    }

    pub fn subject_verb_look_at_top_cards(player: PlayerAst, count: Value, tag: TagKey) -> Self {
        Self::subject_verb_top_library_cards(player, count, tag, false)
    }

    pub fn subject_verb_reveal_top_cards(player: PlayerAst, count: Value, tag: TagKey) -> Self {
        Self::subject_verb_top_library_cards(player, count, tag, true)
    }

    pub fn subject_verb_look_at_objects(player: PlayerAst, filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LookAtObjects { filter },
        )
    }

    pub fn subject_verb_look_at_target(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::LookAtTarget { target },
        )
    }

    fn subject_verb_top_library_cards(
        player: PlayerAst,
        count: Value,
        tag: TagKey,
        reveal: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::LookAtTopCards { count, tag, reveal },
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
            SubjectVerbActionAst::AdditionalLandPlays { count, duration },
        )
    }

    pub fn subject_verb_extra_turn_after_turn(
        player: PlayerAst,
        anchor: ExtraTurnAnchorAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ExtraTurnAfterTurn { anchor },
        )
    }

    pub fn subject_verb_reorder_top_of_library(tag: TagKey) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ReorderTopOfLibrary { tag },
        )
    }

    pub fn subject_verb_shuffle_objects_into_library(player: PlayerAst, target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ShuffleObjectsIntoLibrary {
                target,
                all: false,
                owner_library_destination: false,
                possessive_owner_subject: false,
            },
        )
    }

    pub fn subject_verb_shuffle_objects_into_library_possessive_owner(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::ItsOwner,
            SubjectVerbActionAst::ShuffleObjectsIntoLibrary {
                target,
                all: false,
                owner_library_destination: false,
                possessive_owner_subject: true,
            },
        )
    }

    pub fn subject_verb_shuffle_objects_into_owner_library(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::ItsOwner,
            SubjectVerbActionAst::ShuffleObjectsIntoLibrary {
                target,
                all: false,
                owner_library_destination: true,
                possessive_owner_subject: false,
            },
        )
    }

    pub fn subject_verb_shuffle_all_objects_into_library(
        player: PlayerAst,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ShuffleObjectsIntoLibrary {
                target,
                all: true,
                owner_library_destination: false,
                possessive_owner_subject: false,
            },
        )
    }

    pub fn subject_verb_shuffle_all_objects_into_owner_library(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::ItsOwner,
            SubjectVerbActionAst::ShuffleObjectsIntoLibrary {
                target,
                all: true,
                owner_library_destination: true,
                possessive_owner_subject: false,
            },
        )
    }

    pub fn subject_verb_add_mana_imprinted_colors() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::AddManaImprintedColors,
        )
    }

    pub fn subject_verb_flip_coin(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::FlipCoin,
        )
    }

    pub fn subject_verb_flip_coin_face_only(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::FlipCoinFaceOnly,
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
            SubjectVerbActionAst::RollDie { sides, surface },
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
            SubjectVerbActionAst::RollDiceChooseResult {
                count,
                sides,
                surface,
            },
        )
    }

    pub fn subject_verb_shuffle_hand_and_graveyard_into_library(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary,
        )
    }

    pub fn subject_verb_shuffle_hand_graveyard_and_owned_permanents_into_library(
        player: PlayerAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary,
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
            SubjectVerbActionAst::ShuffleGraveyardIntoLibrary {
                explicit_all_cards_from,
            },
        )
    }

    pub fn subject_verb_reorder_graveyard(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ReorderGraveyard,
        )
    }

    pub fn subject_verb_choose_color(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::ChooseColor,
        )
    }

    pub fn subject_verb_choose_card_type(player: PlayerAst, options: Vec<CardType>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::ChooseCardType { options },
        )
    }

    pub fn subject_verb_choose_named_option(player: PlayerAst, options: Vec<String>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::ChooseNamedOption { options },
        )
    }

    pub fn subject_verb_choose_creature_type(
        player: PlayerAst,
        excluded_subtypes: Vec<Subtype>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::ChooseCreatureType {
                excluded_subtypes,
                family: SubtypeFamily::Creature,
            },
        )
    }

    pub fn subject_verb_choose_subtype_type(player: PlayerAst, family: SubtypeFamily) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::ChooseCreatureType {
                excluded_subtypes: Vec::new(),
                family,
            },
        )
    }

    pub fn subject_verb_choose_land_type(player: PlayerAst, exclude_basic: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::ChooseLandType { exclude_basic },
        )
    }

    pub fn subject_verb_choose_card_name(
        player: PlayerAst,
        filter: Option<ObjectFilter>,
        tag: TagKey,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::ChooseCardName { filter, tag },
        )
    }

    pub fn subject_verb_choose_player(
        chooser: PlayerAst,
        filter: PlayerFilter,
        tag: TagKey,
        random: bool,
        exclude_previous_choices: usize,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            chooser,
            SubjectVerbActionAst::ChoosePlayer {
                filter,
                tag,
                random,
                exclude_previous_choices,
            },
        )
    }

    pub fn subject_verb_tap(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Tap { target },
        )
    }

    pub fn subject_verb_untap(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Untap { target },
        )
    }

    pub fn subject_verb_tap_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TapAll { filter },
        )
    }

    pub fn subject_verb_untap_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::UntapAll { filter },
        )
    }

    pub fn subject_verb_tap_or_untap(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TapOrUntap { target },
        )
    }

    pub fn subject_verb_tap_or_untap_all(
        tap_filter: ObjectFilter,
        untap_filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TapOrUntapAll {
                tap_filter,
                untap_filter,
            },
        )
    }

    pub fn subject_verb_phase_out(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PhaseOut {
                target,
                duration: crate::effects::PhaseOutDuration::UntilNextUntap,
                source_surface: None,
            },
        )
    }

    pub fn subject_verb_phase_out_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PhaseOutAll {
                filter,
                duration: crate::effects::PhaseOutDuration::UntilNextUntap,
                source_surface: None,
            },
        )
    }

    pub fn subject_verb_phase_out_all_until_source_leaves(
        filter: ObjectFilter,
        source_surface: SourceReferenceSurface,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PhaseOutAll {
                filter,
                duration: crate::effects::PhaseOutDuration::UntilSourceLeaves,
                source_surface: Some(source_surface),
            },
        )
    }

    pub fn subject_verb_phase_in(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PhaseIn { target },
        )
    }

    pub fn subject_verb_phase_in_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PhaseInAll { filter },
        )
    }

    pub fn subject_verb_transform(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Transform { target },
        )
    }

    pub fn subject_verb_convert(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Convert { target },
        )
    }

    pub fn subject_verb_destroy(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Destroy {
                target,
                no_regeneration: false,
                creature_destroyed_this_way_surface: false,
            },
        )
    }

    pub fn subject_verb_destroy_no_regeneration(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Destroy {
                target,
                no_regeneration: true,
                creature_destroyed_this_way_surface: false,
            },
        )
    }

    pub fn subject_verb_destroy_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DestroyAll {
                filter,
                no_regeneration: false,
                creature_destroyed_this_way_surface: false,
            },
        )
    }

    pub fn subject_verb_destroy_all_of_chosen_color(
        filter: ObjectFilter,
        no_regeneration: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DestroyAllOfChosenColor {
                filter,
                no_regeneration,
                creature_destroyed_this_way_surface: false,
            },
        )
    }

    pub fn subject_verb_exile(target: TargetAst, face_down: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Exile {
                target,
                face_down,
                source_top_only: false,
                target_plural_surface: false,
            },
        )
    }

    pub fn subject_verb_exile_all(filter: ObjectFilter, face_down: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ExileAll { filter, face_down },
        )
    }

    pub fn subject_verb_look_at_hand(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::LookAtHand { target },
        )
    }

    pub fn subject_verb_counter(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counter { target },
        )
    }

    pub fn subject_verb_counter_unless_pays(
        target: TargetAst,
        cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::CounterUnlessPays { target, cost },
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
            SubjectVerbActionAst::PutCounters {
                counter_type,
                count,
                target,
                target_count,
                distributed,
            },
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
            SubjectVerbActionAst::PutCounterChoice {
                counter_types,
                count,
                mode_texts,
                target,
                target_count,
            },
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
            SubjectVerbActionAst::PutOrRemoveCounters {
                put_counter_type,
                put_count,
                remove_counter_type,
                remove_count,
                put_mode_text: put_mode_text.into(),
                remove_mode_text: remove_mode_text.into(),
                target,
                target_count,
            },
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
            SubjectVerbActionAst::PutCountersAll {
                counter_type,
                count,
                filter,
            },
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
            SubjectVerbActionAst::RemoveUpToAnyCounters {
                amount,
                target,
                counter_type,
                up_to,
                distributed_across_all: false,
                all_of_them: false,
            },
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
            SubjectVerbActionAst::RemoveUpToAnyCounters {
                amount,
                target: TargetAst::Object(filter, None, None),
                counter_type,
                up_to,
                distributed_across_all: true,
                all_of_them: false,
            },
        )
    }

    pub fn subject_verb_remove_all_of_them_counters_from_source() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RemoveUpToAnyCounters {
                amount: Value::CountersOn(Box::new(ChooseSpec::Source), None),
                target: TargetAst::Source(None),
                counter_type: None,
                up_to: false,
                distributed_across_all: false,
                all_of_them: true,
            },
        )
    }

    pub fn subject_verb_move_all_counters(from: TargetAst, to: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::MoveAllCounters { from, to },
        )
    }

    pub fn subject_verb_move_one_counter(from: TargetAst, to: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::MoveOneCounter { from, to },
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
            SubjectVerbActionAst::ForEachCounterKindPutOrRemove {
                target,
                counter_source: None,
                all_kinds: false,
                fixed_counter_type: Some(counter_type),
                optional_action,
                put_only: false,
                choose_target_per_kind: false,
            },
        )
    }

    pub fn subject_verb_put_each_counter_kind_from_on_one_of(
        counter_source: TargetAst,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ForEachCounterKindPutOrRemove {
                target,
                counter_source: Some(counter_source),
                all_kinds: true,
                fixed_counter_type: None,
                optional_action: false,
                put_only: true,
                choose_target_per_kind: true,
            },
        )
    }

    fn subject_verb_counter_kind_put_or_remove(target: TargetAst, all_kinds: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ForEachCounterKindPutOrRemove {
                target,
                counter_source: None,
                all_kinds,
                fixed_counter_type: None,
                optional_action: false,
                put_only: false,
                choose_target_per_kind: false,
            },
        )
    }

    pub fn subject_verb_put_counter_of_chosen_kind(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutCounterOfChosenKind { target },
        )
    }

    pub fn subject_verb_return_to_hand(target: TargetAst, random: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ReturnToHand {
                target,
                random,
                destination_player_surface: None,
                exiled_with_source_surface: None,
                set_quantifier_surface: None,
                set_reference_surface: None,
            },
        )
    }

    pub fn subject_verb_return_all_to_hand(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ReturnAllToHand {
                filter,
                destination_player_surface: None,
                exiled_with_source_surface: None,
            },
        )
    }

    pub fn with_return_destination_player_surface(mut self, player: Option<PlayerAst>) -> Self {
        if let Some(player) = player
            && let Self::SubjectVerb(subject_verb) = &mut self
        {
            match &mut subject_verb.action {
                SubjectVerbActionAst::ReturnToHand {
                    destination_player_surface,
                    ..
                }
                | SubjectVerbActionAst::ReturnAllToHand {
                    destination_player_surface,
                    ..
                } => *destination_player_surface = Some(player),
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
            && let SubjectVerbActionAst::ReturnToHand {
                set_quantifier_surface,
                ..
            } = &mut subject_verb.action
        {
            *set_quantifier_surface = Some(surface);
        }
        self
    }

    pub fn with_return_set_reference_surface(mut self, surface: Option<String>) -> Self {
        if let Some(surface) = surface
            && let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::ReturnToHand {
                set_reference_surface,
                ..
            } = &mut subject_verb.action
        {
            *set_reference_surface = Some(surface);
        }
        self
    }

    pub fn subject_verb_return_all_to_hand_of_chosen_color(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ReturnAllToHandOfChosenColor { filter },
        )
    }

    pub fn subject_verb_move_to_library_nth_from_top(target: TargetAst, position: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::MoveToLibraryNthFromTop { target, position },
        )
    }

    pub fn subject_verb_double_counters_on_each(
        counter_type: Option<CounterType>,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DoubleCountersOnEach {
                counter_type,
                filter,
            },
        )
    }

    pub fn subject_verb_double_counters_on_target(
        counter_type: Option<CounterType>,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DoubleCountersOnTarget {
                counter_type,
                target,
            },
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
            SubjectVerbActionAst::RemoveCountersAll {
                amount,
                filter,
                counter_type,
                up_to,
            },
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
            SubjectVerbActionAst::UnlockRoomDoor,
        )
    }

    pub fn subject_verb_switch_power_toughness(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::SwitchPowerToughness { target, duration },
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
            SubjectVerbActionAst::ScalePowerToughnessAll {
                filter,
                power,
                toughness,
                multiplier,
                duration,
            },
        )
    }

    pub fn subject_verb_reveal_hand(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::RevealHand,
        )
    }

    pub fn subject_verb_discard(
        player: PlayerAst,
        count: Value,
        random: bool,
        any_number: bool,
        filter: Option<ObjectFilter>,
        tag: Option<TagKey>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Discard {
                count,
                random,
                any_number,
                filter,
                tag,
            },
        )
    }

    pub fn subject_verb_discard_hand(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::DiscardHand,
        )
    }

    pub fn subject_verb_poison_counters(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::PoisonCounters { count },
        )
    }

    pub fn subject_verb_energy_counters(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::EnergyCounters { count },
        )
    }

    pub fn subject_verb_experience_counters(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ExperienceCounters { count },
        )
    }

    pub fn subject_verb_ticket_counters(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::TicketCounters { count },
        )
    }

    pub fn subject_verb_pay_energy(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::PayEnergy { amount },
        )
    }

    pub fn subject_verb_pay_life(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::PayLife { amount },
        )
    }

    pub fn subject_verb_pay_any_energy(player: PlayerAst, min_amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::PayAnyEnergy { min_amount },
        )
    }

    pub fn subject_verb_pay_any_life(player: PlayerAst, min_amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::PayAnyLife { min_amount },
        )
    }

    pub fn subject_verb_pay_mana(player: PlayerAst, cost: ManaCost) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::PayMana {
                cost,
                x_value: None,
                x_maximum: None,
            },
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
            SubjectVerbActionAst::PayMana {
                cost,
                x_value: None,
                x_maximum: Some(x_maximum),
            },
        )
    }

    pub fn subject_verb_double_mana_pool(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::DoubleManaPool,
        )
    }

    pub fn subject_verb_empty_mana_pool(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::EmptyManaPool,
        )
    }

    pub fn subject_verb_set_life_total(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::SetLifeTotal { amount },
        )
    }

    pub fn subject_verb_skip_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::SkipTurn,
        )
    }

    pub fn subject_verb_end_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::EndTurn,
        )
    }

    pub fn subject_verb_reverse_turn_order() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ReverseTurnOrder,
        )
    }

    pub fn subject_verb_end_combat_phase(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::EndCombatPhase,
        )
    }

    pub fn subject_verb_skip_combat_phases(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::SkipCombatPhases,
        )
    }

    pub fn subject_verb_skip_next_combat_phase_this_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::SkipNextCombatPhaseThisTurn,
        )
    }

    pub fn subject_verb_skip_main_phases_this_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::SkipMainPhasesThisTurn,
        )
    }

    pub fn subject_verb_skip_combat_phases_this_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::SkipCombatPhasesThisTurn,
        )
    }

    pub fn subject_verb_skip_draw_step(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::SkipDrawStep,
        )
    }

    pub fn subject_verb_additional_phases(phases: Vec<crate::effects::AdditionalPhase>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Implicit,
            SubjectVerbActionAst::AdditionalPhases { phases },
        )
    }

    pub fn subject_verb_play_from_graveyard_until_eot(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::PlayFromGraveyardUntilEot,
        )
    }

    pub fn subject_verb_ring_tempts_you(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::RingTemptsYou,
        )
    }

    pub fn subject_verb_venture_into_dungeon(
        player: PlayerAst,
        undercity_if_no_active: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::VentureIntoDungeon {
                undercity_if_no_active,
            },
        )
    }

    pub fn subject_verb_become_monarch(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::BecomeMonarch,
        )
    }

    pub fn subject_verb_take_initiative(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::TakeInitiative,
        )
    }

    pub fn subject_verb_create_emblem(player: PlayerAst, emblem: EmblemDescriptionAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::CreateEmblem { emblem },
        )
    }

    pub fn subject_verb_lose_game(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseGame,
        )
    }

    pub fn subject_verb_win_game(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::WinGame,
        )
    }

    pub fn subject_verb_detain(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Detain { target },
        )
    }

    pub fn subject_verb_goad(target: TargetAst) -> Self {
        Self::subject_verb_goad_for(target, Until::YourNextTurn)
    }

    pub fn subject_verb_goad_for(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Goad { target, duration },
        )
    }

    pub fn subject_verb_suspect(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Suspect { target },
        )
    }

    pub fn subject_verb_clear_suspected(target: Option<TargetAst>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ClearSuspected { target },
        )
    }

    pub fn subject_verb_heal_damage(target: TargetAst, amount: Option<Value>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::HealDamage { target, amount },
        )
    }

    pub fn subject_verb_remove_from_combat(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RemoveFromCombat { target },
        )
    }

    pub fn subject_verb_flip(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Flip { target },
        )
    }

    pub fn subject_verb_regenerate(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Regenerate {
                target,
                follow_up_effects: Vec::new(),
            },
        )
    }

    pub fn subject_verb_regenerate_with_follow_up_effects(
        target: TargetAst,
        follow_up_effects: Vec<EffectAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Regenerate {
                target,
                follow_up_effects,
            },
        )
    }

    pub fn subject_verb_regenerate_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RegenerateAll { filter },
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
            SubjectVerbActionAst::Sacrifice {
                filter,
                count,
                target,
                one_of_referenced_set: false,
            },
        )
    }

    pub fn with_sacrifice_one_of_referenced_set(mut self) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Sacrifice {
                    one_of_referenced_set,
                    ..
                },
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
            SubjectVerbActionAst::SacrificeAll { filter },
        )
    }
}
