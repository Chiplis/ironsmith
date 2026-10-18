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
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct ChooseOneModeAst {
    pub description: String,
    pub effects: Vec<EffectAst>,
}

#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
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
    /// "Each player exiles the top card of their library. ... If two or more
    /// players' cards are tied for greatest, the tied players repeat this
    /// process until the tie is broken." (Timesifter): the contenders each
    /// exile the top card of their library until at most one has the
    /// greatest mana value; `contenders_tag` then names that player.
    GreatestManaValueTieBreakExile {
        contenders_tag: TagRef,
        exiled_tag: TagRef,
    },
    /// "Each player rolls a d20. If one or more opponents had the highest
    /// result, ..." (Chaos Dragon): every player rolls; `tag` then names the
    /// highest rollers matching `keep`.
    RollDieKeepHighestPlayers {
        sides: u32,
        keep: PlayerFilter,
        tag: TagRef,
    },
    /// "Until that player's next end step, they may play that card [without
    /// paying its mana cost]" (Casey & Raph): a tagged play permission that
    /// lasts until the permission holder's own next end step.
    GrantPlayTaggedUntilPlayerNextEndStep {
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
    },
    /// "It becomes day." / "It becomes night." (CR 731.2-731.3).
    SetDayNight(ironsmith_core::DayNightDesignation),
    /// One named choice per participating player ("For each player, choose
    /// friend or foe", "Each opponent chooses fame or fortune"); the players
    /// per option are recorded for the following instructions.
    ChoosePlayerOption(ironsmith_core::ChoosePlayerOptionEffect),
    /// "a card spliced onto that spell" (CR 702.47): tags the cards spliced
    /// onto `spell` as `tag` for a following instruction's filter.
    TagSplicedCards { spell: TargetAst, tag: TagRef },
    /// "If each of those creatures could block all creatures that the other
    /// is blocking, remove both of them from combat. Each one then blocks all
    /// creatures the other was blocking." (Sorrow's Path)
    ///
    /// With `attackers`, the pair are blocked attacking creatures that trade
    /// the creatures blocking exactly one of them (General Jarkeld).
    ExchangeBlocks { blockers: TargetAst, attackers: bool },
    /// "Flip a coin. If you win the flip, you gain N life and target opponent
    /// loses N life, and you decide whether to flip again. If you lose the
    /// flip, you lose N life and that opponent gains N life, and that player
    /// decides whether to flip again. Double the life stakes with each flip."
    /// (Game of Chaos)
    CoinFlipLifeStakes { opponent: TargetAst, initial_stakes: u32 },
    /// "Creatures it was blocking that had become blocked by only this
    /// creature this combat become unblocked." (Ydwen Efreet)
    UnblockSoleBlockedAttackers(ironsmith_core::UnblockSoleBlockedAttackersEffect),
    /// "Until end of turn, any time you could activate a mana ability, you may
    /// pay 1 life. If you do, add {C}." (Channel): a player-owned mana ability
    /// for the rest of the turn (CR 605.1a).
    GrantPayLifeManaAbilityUntilEndOfTurn(ironsmith_core::GrantPayLifeManaAbilityUntilEndOfTurnEffect),
    /// "roll five six-sided dice and store those results on it" (Centaur of
    /// Attention).
    RollAndStoreDieResults(ironsmith_core::RollAndStoreDieResultsEffect),
    /// "you may reroll any number of this creature's stored results".
    RerollStoredDieResults(ironsmith_core::RerollStoredDieResultsEffect),
    /// "This turn, instead of declaring blockers, each defending player ...
    /// divides them into ... piles ... Each creature in a pile that can block
    /// the creature that pile is assigned to does so." (Camouflage)
    DeclareBlockersByRandomPilesThisTurn,
    /// "Creatures it was blocking that had become blocked by only that
    /// creature this combat become unblocked." (False Orders): the same
    /// unblocking, for a referenced creature instead of the source.
    UnblockSoleBlockedAttackersOf { blocker: TargetAst },
    /// "You may have it block an attacking creature of your choice." (False
    /// Orders): `blocker` starts blocking a chosen attacker matching
    /// `attackers` (see `BlockAttackerOfChoiceEffect`).
    BlockAttackerOfChoice {
        blocker: TargetAst,
        attackers: ObjectFilter,
        optional: bool,
    },
    /// "<player> plays with their hand revealed for as long as this creature
    /// remains on the battlefield" (Stromgald Spy).
    PlayWithHandRevealed(ironsmith_core::PlayWithHandRevealedEffect),
    /// "Until that player's next turn, that player plays with that card
    /// revealed in their hand and can't play it." (Firestorm Phoenix)
    RevealSourceInHandUntilOwnersNextTurn(ironsmith_core::RevealSourceInHandUntilOwnersNextTurnEffect),
    /// "the last red instant or sorcery spell that dealt damage to you this
    /// turn": tags that damage source's event-time snapshot.
    TagLastDamageSource(ironsmith_core::TagLastDamageSourceEffect),
    /// "You choose how each player votes this turn." (Illusion of Choice).
    ControlVotesThisTurn,
    /// "This ability still resolves if its target becomes illegal."
    ResolvesDespiteIllegalTargets,
    /// "Note the type of mana spent to pay this activation cost."
    NoteActivationManaType,
    NoteActivationManaPayment,
    CastFaceDownUsingActivationMana { player: PlayerFilter, filter: ObjectFilter, turn_face_up_before_action: bool },
    RegisterExtraDieReplacement { player: TargetAst, chooser: PlayerFilter, additional: u32 },
    /// "Note one of its creature types that hasn't been noted for this
    /// artifact": records one of the triggering object's creature types on
    /// the ability's source.
    NoteTriggeringCreatureType,
    /// "Note the number and kind of counters that were on that creature"
    /// (Tawnos's Coffin): records `object`'s last-known counters under `tag`
    /// for a later "with the noted number and kind of counters on it" entry.
    NoteCounters { object: TargetAst, tag: TagRef },
    /// "put the same number and kind of counters on <target>" (Captain
    /// Marvel, Bold Plagiarist): copies the triggering counter placement's
    /// kind and amount onto `target`.
    PutTriggeringCounterKind { target: TargetAst },
    /// "Target spell's controller exiles it ..." (Ertai's Meddling): keep the
    /// target spell's announced choices (targets, X, modes, optional costs)
    /// with its card, by stable identity, before it leaves the stack.
    RetainSpellChoices { target: TargetAst, controller_tag: TagRef },
    /// "... the player puts it onto the stack as a copy of the original
    /// spell" (Ertai's Meddling): put the card back onto the stack (not cast)
    /// with the retained choices of the spell it was, under that spell's
    /// controller (CR 707.10).
    PutRetainedSpellCopyOntoStack { target: TargetAst },
    /// "for each 1 damage that would be dealt to you until your next upkeep,
    /// you remove an echo counter from this enchantment instead" (Soul
    /// Echo): a resolution-registered replacement of damage to `player`.
    RegisterDamageCounterReplacement {
        player: PlayerFilter,
        counter_type: ironsmith_core::CounterType,
        object: TargetAst,
        expiry: ironsmith_core::RegisteredReplacementExpiry,
    },
    /// "For each player, choose friend or foe." Tags the two player groups.
    ChooseFriendsOrFoes {
        friends: TagRef,
        foes: TagRef,
    },
    /// "Until end of turn, you may tap lands you don't control for mana."
    /// (Piracy): a this-turn permission for the resolving player.
    GrantTapUncontrolledLandsForMana,
    /// "Each player reveals the top card of their library. The player who
    /// reveals the card with the greatest mana value may change the target or
    /// targets. If two or more cards are tied for greatest, the target or
    /// targets remain unchanged." (Psychic Battle): the unique winner of the
    /// reveal contest may choose new targets for the triggering spell or
    /// ability; `doesnt_trigger_same_named_sources` is "Changing targets
    /// this way doesn't trigger abilities of permanents named <this card>."
    RevealContestWinnerMayChangeTriggeringTargets {
        doesnt_trigger_same_named_sources: bool,
    },
    /// Relax the loyalty-ability activation rule this turn (CR 606.3).
    GrantLoyaltyActivationAllowance {
        scope: ironsmith_core::LoyaltyActivationScope,
        allowance: ironsmith_core::LoyaltyActivationAllowance,
    },
    /// "You may pay [cost] to end this effect." (Licids): offers the
    /// resolving ability's controller a special action (CR 116.2c) that ends
    /// the continuous effects the ability's earlier instructions created.
    PayToEndThisEffect {
        cost: ManaCost,
    },
    /// A player other than the library's owner privately looks at its top
    /// cards ("They look at the top four cards of your library").
    LookAtTopCardsAsViewer {
        library_owner: PlayerFilter,
        viewer: PlayerFilter,
        count: Value,
        tag: TagRef,
    },
    /// The authored-subject form of `LookAtTopCardsAsViewer`, before player
    /// references are resolved: "That player looks at the top three cards of
    /// your library" (the subject is the private viewer, CR 701.16a-style
    /// look; the named library belongs to `library_owner`).
    PlayerLooksAtTopCardsOfLibrary {
        viewer: PlayerAst,
        library_owner: PlayerAst,
        count: Value,
        tag: TagRef,
    },
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
    /// Preserve an object reference even when the nested action does not affect it.
    TagReferenced {
        effect: Box<EffectAst>,
        tag: TagRef,
    },
    /// Tag only the objects affected by the nested action, including an empty set.
    TagAffected {
        effect: Box<EffectAst>,
        tag: TagRef,
    },
    DirectionalAdjacentPlayerControl {
        filter: ObjectFilter,
        left_option: String,
        right_option: String,
        /// Every matching object moves, not one chosen object per player.
        all_matching: bool,
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
    /// Collect optional mana payments in controller-first turn order; bind
    /// their checked total as a fresh local X for the complete nested body.
    CollectManaPayments {
        effects: Vec<EffectAst>,
    },
    /// A die-result row that fixes X for the program it governs ("1—9 | X is
    /// one."): run `effects` with X equal to `value`.
    BindX {
        value: Value,
        effects: Vec<EffectAst>,
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
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::DrawForEachTaggedMatching {
                tag,
                filter,
            }),
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
                mode: ironsmith_core::NextSpellGrantMode::Ability,
            }),
        )
    }

    pub fn subject_verb_next_play_timing_this_turn(
        player: PlayerAst,
        filter: ObjectFilter,
        includes_land_plays: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantNextSpellAbilityThisTurn {
                filter,
                ability: Box::new(GrantedAbilityAst::KeywordAction(Box::new(
                    crate::payload::KeywordAction::Flash,
                ))),
                mode: if includes_land_plays {
                    ironsmith_core::NextSpellGrantMode::PlayTiming
                } else {
                    ironsmith_core::NextSpellGrantMode::CastTiming
                },
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
                also_each: None,
            }),
        )
    }

    /// Protection of one shared choice granted to a recipient and to every
    /// member of a quantified object set.
    pub fn subject_verb_grant_protection_choice_with_each(
        target: TargetAst,
        also_each: ObjectFilter,
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
                also_each: Some(also_each),
            }),
        )
    }

    pub fn subject_verb_prevent_all_combat_damage(duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllCombatDamage { duration },
            ),
        )
    }

    pub fn subject_verb_assign_no_combat_damage(source: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::AssignNoCombatDamage { source, duration },
            ),
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
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllCombatDamageFromSource {
                    duration,
                    source,
                    source_would_deal_surface,
                    dealt_to_and_by: false,
                },
            ),
        )
    }

    /// "Prevent all combat damage that would be dealt to and dealt by <X>".
    pub fn subject_verb_prevent_all_combat_damage_to_and_from_source(
        source: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllCombatDamageFromSource {
                    duration,
                    source,
                    source_would_deal_surface: false,
                    dealt_to_and_by: true,
                },
            ),
        )
    }

    pub fn subject_verb_prevent_all_combat_damage_from_source_filter(
        source_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllCombatDamageFromSourceFilter {
                    duration,
                    source_filter,
                    excluded_source_target: None,
                    source_of_your_choice: false,
                },
            ),
        )
    }

    /// "Prevent all combat damage a creature of your choice would deal this
    /// turn": a single source matching `source_filter`, chosen on resolution.
    pub fn subject_verb_prevent_all_combat_damage_from_chosen_source_filter(
        source_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllCombatDamageFromSourceFilter {
                    duration,
                    source_filter,
                    excluded_source_target: None,
                    source_of_your_choice: true,
                },
            ),
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
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllCombatDamageFromSourceFilter {
                    duration,
                    source_filter,
                    excluded_source_target: Some(excluded_source_target),
                    source_of_your_choice: false,
                },
            ),
        )
    }

    pub fn subject_verb_prevent_all_combat_damage_to_players(duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllCombatDamageToPlayers { duration },
            ),
        )
    }

    pub fn subject_verb_prevent_all_combat_damage_to_you(duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllCombatDamageToYou {
                    duration,
                    follow_up_effects: Vec::new(),
                },
            ),
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
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventNextTimeDamage {
                    source,
                    target,
                    reflect_damage_to_source_controller,
                    reflect_source_filter: None,
                    follow_up_effects: Vec::new(),
                    portion: ironsmith_core::NextTimeDamagePreventionPortion::All,
                    combat_only: false,
                },
            ),
        )
    }

    /// "The next time <source> would deal [combat] damage to <recipient> this
    /// turn, prevent half that damage, rounded down / all but N of that
    /// damage."
    pub fn subject_verb_prevent_next_time_damage_portion(
        source: PreventNextTimeDamageSourceAst,
        target: PreventNextTimeDamageTargetAst,
        portion: ironsmith_core::NextTimeDamagePreventionPortion,
        combat_only: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventNextTimeDamage {
                    source,
                    target,
                    reflect_damage_to_source_controller: false,
                    reflect_source_filter: None,
                    follow_up_effects: Vec::new(),
                    portion,
                    combat_only,
                },
            ),
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
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::ReplaceNextDamageToTarget {
                    target,
                    damage_target_tag,
                    replacement_effects,
                },
            ),
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
                combat_only: false,
                source_of_your_choice,
                protect_you_and_permanents_you_control,
                protect_all: false,
                follow_up_effects,
                divided: false,
                source: None,
            }),
        )
    }

    pub fn subject_verb_prevent_all_damage_to_target(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb_prevent_all_damage_to_target_with_source_choice(target, duration, false)
    }

    pub fn subject_verb_prevent_all_combat_damage_to_target(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllDamageToTarget {
                    target,
                    duration,
                    combat_only: true,
                    source_of_your_choice: false,
                    source_choice_shares_activation_mana_color: false,
                    source_target: None,
                    protect_source_target: false,
                    follow_up_effects: Vec::new(),
                    source_would_deal_surface: false,
                },
            ),
        )
    }

    pub fn subject_verb_prevent_all_damage_to_target_with_source_choice(
        target: TargetAst,
        duration: Until,
        source_of_your_choice: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllDamageToTarget {
                    target,
                    duration,
                    combat_only: false,
                    source_of_your_choice,
                    source_choice_shares_activation_mana_color: false,
                    source_target: None,
                    protect_source_target: false,
                    follow_up_effects: Vec::new(),
                    source_would_deal_surface: false,
                },
            ),
        )
    }

    pub fn subject_verb_prevent_all_damage_to_target_with_mana_color_source_choice(
        target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllDamageToTarget {
                    target,
                    duration,
                    combat_only: false,
                    source_of_your_choice: true,
                    source_choice_shares_activation_mana_color: true,
                    source_target: None,
                    protect_source_target: false,
                    follow_up_effects: Vec::new(),
                    source_would_deal_surface: false,
                },
            ),
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
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllDamageToTarget {
                    target,
                    duration,
                    combat_only: false,
                    source_of_your_choice: false,
                    source_choice_shares_activation_mana_color: false,
                    source_target: Some(source_target),
                    protect_source_target: false,
                    follow_up_effects: Vec::new(),
                    source_would_deal_surface: false,
                },
            ),
        )
    }

    /// One target declaration binds both directions of the prevention shield.
    pub fn subject_verb_prevent_all_damage_to_and_by_target(
        target: TargetAst,
        duration: Until,
    ) -> Self {
        let mut effect = Self::subject_verb_prevent_all_damage_to_target_from_target_source(
            TargetAst::ObjectOrPlayer(
                ObjectFilter::default(),
                crate::target::PlayerFilter::Any,
                None,
            ),
            target,
            duration,
        );
        if let Self::SubjectVerb(subject) = &mut effect
            && let SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllDamageToTarget {
                    protect_source_target,
                    ..
                },
            ) = &mut subject.action
        {
            *protect_source_target = true;
        }
        effect
    }

    pub fn subject_verb_prevent_all_damage_to_target_from_source_filter(
        target: TargetAst,
        source_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter {
                    target,
                    duration,
                    source_filter,
                    source_would_deal_surface: false,
                    of_chosen_color: false,
                    source_of_your_choice: false,
                    follow_up_effects: Vec::new(),
                    cause_targets_protected: false,
                },
            ),
        )
    }

    /// Limit a recipient prevention to damage caused by a spell or ability
    /// that targets the protected object (Silhouette, CR 615 / 115).
    pub fn with_prevention_cause_targets_protected(mut self) -> Self {
        if let Self::SubjectVerb(subject) = &mut self
            && let SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter {
                    cause_targets_protected,
                    ..
                },
            ) = &mut subject.action
        {
            *cause_targets_protected = true;
        }
        self
    }

    /// Mark an all-damage prevention as authored "<source> would deal".
    pub fn with_prevention_source_would_deal_surface(mut self) -> Self {
        if let Self::SubjectVerb(subject) = &mut self
            && let SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllDamageToTarget {
                    source_would_deal_surface,
                    ..
                }
                | DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter {
                    source_would_deal_surface,
                    ..
                }
                | DamagePreventionActionAst::PreventAllDamageFromSourceFilter {
                    source_would_deal_surface,
                    ..
                },
            ) = &mut subject.action
        {
            *source_would_deal_surface = true;
        }
        self
    }

    pub fn subject_verb_prevent_all_damage_from_source_filter(
        source_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllDamageFromSourceFilter {
                    duration,
                    source_filter,
                    of_chosen_color: false,
                    source_would_deal_surface: false,
                    follow_up_effects: Vec::new(),
                },
            ),
        )
    }

    pub fn subject_verb_prevent_all_damage_from_source_filter_of_chosen_color(
        source_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventAllDamageFromSourceFilter {
                    duration,
                    source_filter,
                    of_chosen_color: true,
                    source_would_deal_surface: false,
                    follow_up_effects: Vec::new(),
                },
            ),
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
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::PreventDamageToTargetPutCounters {
                    amount,
                    target,
                    duration,
                    counter_type,
                },
            ),
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

    /// Substitute a non-mana payment for the cast spell's mana cost on a
    /// cast-tagged action; other effects are returned unchanged.
    pub fn with_cast_tagged_alternative_payment(
        mut self,
        payment: ironsmith_core::CastTaggedAlternativePayment,
    ) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Stack(StackActionAst::CastTagged {
                    alternative_payment,
                    ..
                }),
            ..
        }) = &mut self
        {
            *alternative_payment = Some(payment);
        }
        self
    }

    /// Attach one required price to this exact cast/permission action.
    pub fn with_casting_alternative_cost(mut self, cost: ironsmith_core::TotalCost<crate::model::CompilerCost>) -> Self {
        if let Self::SubjectVerb(subject) = &mut self {
            match &mut subject.action {
                SubjectVerbActionAst::Stack(StackActionAst::CastTagged { alternative_cost, .. })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn { alternative_cost, .. }) => *alternative_cost = Some(cost),
                _ => {},
            }
        }
        self
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
                alternative_cost: None,
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
                alternative_payment: None,
                caster_land_mana_abilities_only: false,
                control_caster_while_spell_resolves: false,
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
        Self::Permissions(
            PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                player,
                zone_owner: player,
                filter,
                zone,
                payment: ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost,
            },
        )
    }

    pub fn may_cast_matching_spell_without_paying_mana_cost_from_zone_owner(
        player: PlayerAst,
        zone_owner: PlayerAst,
        filter: ObjectFilter,
        zone: Zone,
    ) -> Self {
        Self::Permissions(
            PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                player,
                zone_owner,
                filter,
                zone,
                payment: ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost,
            },
        )
    }

    pub fn may_cast_matching_spell_with_alternative_cost(
        player: PlayerAst,
        filter: ObjectFilter,
        zone: Zone,
        kind: crate::filter::AlternativeCastKind,
    ) -> Self {
        Self::Permissions(
            PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                player,
                zone_owner: player,
                filter,
                zone,
                payment: ironsmith_core::MayCastMatchingSpellPayment::AlternativeCost(kind),
            },
        )
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
                spell_filter: None,
                alternative_cost: None,
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                while_on_top_of_library: false,
                free_cast_from_current_zone: false,
                until_source_exiles_another: false,
                spell_cost_reduction: None,
                spell_cost_increase_unless_targets: None,
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
                spell_filter: None,
                alternative_cost: None,
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                while_on_top_of_library: false,
                free_cast_from_current_zone: true,
                until_source_exiles_another: false,
                spell_cost_reduction: None,
                spell_cost_increase_unless_targets: None,
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
                spell_filter: None,
                alternative_cost: None,
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                while_on_top_of_library: true,
                free_cast_from_current_zone: true,
                until_source_exiles_another: false,
                spell_cost_reduction: None,
                spell_cost_increase_unless_targets: None,
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
                spell_filter: None,
                alternative_cost: None,
                tag,
                player,
                allow_land,
                without_paying_mana_cost: false,
                allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode::Normal,
                while_on_top_of_library: false,
                free_cast_from_current_zone: false,
                until_source_exiles_another: true,
                spell_cost_reduction: None,
                spell_cost_increase_unless_targets: None,
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
            SubjectVerbActionAst::Grants(
                GrantActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                    tag,
                    player,
                },
            ),
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
                until_next_turn_start: false,
                during_next_turn: false,
                until_end_of_combat_next_turn: false,
                spell_filter: None,
                surface: None,
                max_plays: None,
                without_paying_mana_cost: false,
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
                until_next_turn_start: false,
                during_next_turn: false,
                until_end_of_combat_next_turn: false,
                spell_filter: None,
                surface: None,
                max_plays: None,
                without_paying_mana_cost: false,
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
                SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                    max_plays,
                    ..
                })
                | SubjectVerbActionAst::Grants(
                    GrantActionAst::GrantPlayTaggedUntilYourNextTurn { max_plays, .. },
                ) => {
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
                permission_bound_mana: false,
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                filter,
                during_turns_counter_put_on_source: None,
                spell_cost_increase: None,
                lands_enter_tapped: false,
                surface: None,
                during_turns_attacked_with: None,
                during_turns_cast_another_spell: false,
                spell_cost_reduction_per_opponent: 0,
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
                permission_bound_mana: false,
                tag,
                player,
                allow_land,
                without_paying_mana_cost: false,
                allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode::Normal,
                filter: None,
                during_turns_counter_put_on_source: Some(counter_type),
                spell_cost_increase: None,
                lands_enter_tapped: false,
                surface: None,
                during_turns_attacked_with: None,
                during_turns_cast_another_spell: false,
                spell_cost_reduction_per_opponent: 0,
            }),
        )
    }

    /// "During any turn you attacked with <filter>, you may play that card."
    pub fn subject_verb_grant_play_tagged_during_turns_attacked_with(
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: impl Into<ironsmith_core::value_model::ManaSpendMode>,
        condition: ironsmith_core::effect::AttackedWithTurnCondition,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled {
                permission_bound_mana: false,
                tag,
                player,
                allow_land,
                without_paying_mana_cost: false,
                allow_any_color_for_cast: allow_any_color_for_cast.into(),
                filter: None,
                during_turns_counter_put_on_source: None,
                spell_cost_increase: None,
                lands_enter_tapped: false,
                surface: None,
                during_turns_attacked_with: Some(condition),
                during_turns_cast_another_spell: false,
                spell_cost_reduction_per_opponent: 0,
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
                permission_bound_mana: false,
                tag,
                player,
                allow_land: true,
                without_paying_mana_cost: false,
                allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode::Normal,
                filter: None,
                during_turns_counter_put_on_source: None,
                spell_cost_increase,
                lands_enter_tapped,
                surface: None,
                during_turns_attacked_with: None,
                during_turns_cast_another_spell: false,
                spell_cost_reduction_per_opponent: 0,
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
            SubjectVerbActionAst::Grants(
                GrantActionAst::GrantPlayTaggedForAsLongAsYouControlSource {
                    tag,
                    player,
                    allow_land,
                    allow_any_color_for_cast,
                    surface,
                    max_plays: None,
                    during_your_turns: false,
                    alternative_cost: None,
                    alternative_cost_x_is_mana_value: false,
                },
            ),
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
                result_tag: None,
                target_reference_surface: None,
                from_graveyard_or_exile: false,
                tapped,
                transformed,
                converted,
                controller,
                count_value,
                as_aura: None,
                top_only: false,
                as_non_aura: false,
                tapped_if: None,
                noted_counters: None,
            }),
        )
    }

    /// "return ... with the noted number and kind of counters on it": the
    /// returned object enters with the counters noted under `tag`. Returns
    /// whether this effect is a return-to-battlefield that now carries them.
    pub fn set_return_noted_counters(&mut self, tag: TagRef) -> bool {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                    noted_counters,
                    ..
                }),
            ..
        }) = self
        {
            *noted_counters = Some(tag);
            return true;
        }
        false
    }

    /// "return ... as a non-Aura enchantment": the returned object enters
    /// without its Aura subtype.
    pub fn with_return_as_non_aura(mut self) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                    as_non_aura,
                    ..
                }),
            ..
        }) = &mut self
        {
            *as_non_aura = true;
        }
        self
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
                aura_attachment_filter: None,
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
                aura_attachment_filter: None,
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
            retained_reference_tags: Vec::new(),
                target,
                source_top_only: false,
                tagged_destinations: Vec::new(),
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
                battlefield_attack_player_only: false,
                battlefield_blocking: None,
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
            retained_reference_tags: Vec::new(),
                target,
                source_top_only: false,
                tagged_destinations: Vec::new(),
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
                battlefield_attack_player_only: false,
                battlefield_blocking: None,
                battlefield_face_down: false,
                battlefield_transformed: false,
                attached_to,
                all: true,
            }),
        )
    }

    /// "... onto the battlefield [tapped and] attacking that opponent / that
    /// player or a planeswalker they control": the entering attacker's
    /// attack-target player (CR 508.4). No-op for other effects or `None`.
    pub fn with_battlefield_attack_target(mut self, target: Option<(PlayerAst, bool)>) -> Self {
        if let Some((player, player_only)) = target
            && let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                battlefield_attacking,
                battlefield_attack_target_player_or_planeswalker_controlled_by,
                battlefield_attack_player_only,
                ..
            }) = &mut subject_verb.action
        {
            *battlefield_attacking = true;
            *battlefield_attack_target_player_or_planeswalker_controlled_by = Some(player);
            *battlefield_attack_player_only = player_only;
        }
        self
    }

    /// "onto the battlefield blocking that creature": the blocked attacker.
    /// No-op for other effects or `None`.
    pub fn with_battlefield_blocking(mut self, blocked: Option<TargetAst>) -> Self {
        if let Some(blocked) = blocked
            && let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                battlefield_blocking,
                ..
            }) = &mut subject_verb.action
        {
            *battlefield_blocking = Some(blocked);
        }
        self
    }

    /// "onto the battlefield transformed" (CR 712.14). No-op for other
    /// effects or `false`.
    pub fn with_battlefield_transformed(mut self, transformed: bool) -> Self {
        if transformed
            && let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                battlefield_transformed,
                ..
            }) = &mut subject_verb.action
        {
            *battlefield_transformed = true;
        }
        self
    }

    /// Narrows an entering-attacking move's attack target to the named player
    /// itself ("attacking that opponent"). No-op for other effects.
    pub fn with_battlefield_attack_player_only(mut self) -> Self {
        if let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                battlefield_attack_player_only,
                ..
            }) = &mut subject_verb.action
        {
            *battlefield_attack_player_only = true;
        }
        self
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

    pub fn with_tagged_destinations(mut self, destinations: Vec<(TagRef, Zone)>) -> Self {
        if let Self::SubjectVerb(subject) = &mut self
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                tagged_destinations, ..
            }) = &mut subject.action
        {
            *tagged_destinations = destinations;
        }
        self
    }

    pub fn with_move_to_zone_verb_surface(
        mut self,
        surface: ironsmith_core::MoveToZoneVerbSurface,
    ) -> Self {
        if let Self::SubjectVerb(subject_verb) = &mut self {
            match &mut subject_verb.action {
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    verb_surface,
                    ..
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToBattlefield {
                    verb_surface,
                    ..
                }) => {
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
                    source_top_only,
                    ..
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    source_top_only,
                    ..
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
            SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryTopOrBottomChoice {
                target,
                top_position: 0,
            }),
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
                filter_source_snapshots: false,
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
                filter_source_snapshots: false,
            },
        )
    }

    pub fn subject_verb_filtered_tagged_objects(
        filter: ObjectFilter,
        tag: TagRef,
        source_tags: Vec<TagRef>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TagMatchingObjects {
                filter,
                zones: Vec::new(),
                tag,
                source_tags,
                filter_source_snapshots: true,
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
        Self::subject_verb_become_object_template(Some((power, toughness)), target, card_types, subtypes, subtype_families,
            colors, abilities, granted_abilities, preserve_other_types, type_retention_surface,
            animation_pt_surface, animation_duration_surface, duration)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn subject_verb_become_object_template(
        base_power_toughness: Option<(Value, Value)>,
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
                remove_other_abilities: false,
                base_power_toughness,
                target,
                card_types,
                subtypes,
                subtype_families,
                colors,
                abilities,
                granted_abilities,
                preserve_other_types,
                preserve_other_colors: false,
                type_retention_surface,
                animation_pt_surface,
                animation_duration_surface,
                set_quantifier_surface: None,
                duration,
            }),
        )
    }

    /// Colors and card types have independent retention semantics.
    pub fn with_animation_color_retention(mut self, preserve: bool) -> Self {
        if let Self::SubjectVerb(subject) = &mut self
            && let SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature { preserve_other_colors, .. }) = &mut subject.action
        { *preserve_other_colors = preserve; }
        self
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
            | SubjectVerbActionAst::Characteristics(
                CharacteristicActionAst::SetBasePowerToughness {
                    set_quantifier_surface,
                    ..
                },
            )
            | SubjectVerbActionAst::Characteristics(
                CharacteristicActionAst::BecomeBasePtCreature {
                    set_quantifier_surface,
                    ..
                },
            )
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

    pub fn subject_verb_set_base_toughness(
        toughness: Value,
        target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBaseToughness {
                toughness,
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

    pub fn subject_verb_remove_supertypes(target: TargetAst, supertypes: Vec<Supertype>, duration: Until) -> Self {
        Self::subject_verb(SubjectVerbRoleAst::Actor, PlayerAst::Implicit,
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSupertypes { target, supertypes, duration }))
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
            SubjectVerbActionAst::Characteristics(
                CharacteristicActionAst::BecomeSaddledUntilEndOfTurn { target },
            ),
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
            SubjectVerbActionAst::Characteristics(
                CharacteristicActionAst::AddAllSubtypesOfFamily {
                    target,
                    family,
                    duration,
                },
            ),
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

    pub fn subject_verb_change_text(target: TargetAst, selection: ironsmith_core::TextChangeSelection, duration: Until) -> Self {
        Self::subject_verb(SubjectVerbRoleAst::Actor, PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::ChangeText { target, selection, duration }))
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
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::MakeColorless {
                target,
                duration,
            }),
        )
    }

    pub fn subject_verb_become_basic_land_type_choice(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(
                CharacteristicActionAst::BecomeBasicLandTypeChoice { target, duration, allowed_subtypes: Vec::new(), preserve_other_types: false },
            ),
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
            SubjectVerbActionAst::Characteristics(
                CharacteristicActionAst::BecomeCreatureTypeChoice {
                    target,
                    duration,
                    excluded_subtypes,
                    in_addition_to_other_types: false,
                },
            ),
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
                color_choice: None,
            }),
        )
    }

    /// "<target> becomes that color": the color of an exact earlier local
    /// color decision, bound during reference resolution.
    pub fn subject_verb_become_that_color(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeColorChoice {
                target,
                duration,
                allow_multiple: false,
                color_choice: Some(ironsmith_core::ColorChoiceReference::Pending),
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
        retain_source_colors: bool,
        copy_ability_absent_marker: Option<String>,
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
                retain_source_colors,
                copy_ability_absent_marker,
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
                mode: ironsmith_core::AbilityLossMode::Lose,
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
                mode: ironsmith_core::AbilityLossMode::Lose,
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
                only_if_absent: false,
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
                only_if_absent: false,
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
                mode: ironsmith_core::AbilityLossMode::Lose,
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
                accumulated_match_tag: None,
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
                accumulated_match_tag: None,
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
                enters_under_player: None,
            }),
        )
    }

    pub fn with_search_battlefield_controller(mut self, controller: PlayerAst) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary {
                    enters_under_player,
                    ..
                }),
            ..
        }) = &mut self
        {
            *enters_under_player = Some(controller);
        }
        self
    }

    pub fn with_search_zones(mut self, zones: Vec<Zone>) -> Self {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary {
                    search_zones, ..
                }),
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
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::RedirectNextDamageFromSourceToTarget {
                    amount,
                    protected_target: None,
                    destination: RedirectNextTimeDamageDestinationAst::TargetObject,
                    destination_target: Some(target),
                    source_of_your_choice: false,
                },
            ),
        )
    }

    pub fn subject_verb_redirect_next_damage_to_controller(
        amount: Value,
        protected_target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::RedirectNextDamageFromSourceToTarget {
                    amount,
                    protected_target: Some(protected_target),
                    destination: RedirectNextTimeDamageDestinationAst::Controller,
                    destination_target: None,
                    source_of_your_choice: false,
                },
            ),
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
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::RedirectNextTimeDamageToSource {
                    source,
                    combat_only: false,
                    target: Some(target),
                    destination,
                    destination_target: None,
                    all_this_turn: false,
                },
            ),
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
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::RedirectNextTimeDamageToSource {
                    source,
                    combat_only: false,
                    target: Some(target),
                    destination: RedirectNextTimeDamageDestinationAst::TargetObject,
                    destination_target: Some(destination_target),
                    all_this_turn: false,
                },
            ),
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
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::RedirectNextTimeDamageToSource {
                    source,
                    combat_only: false,
                    target: Some(target),
                    destination,
                    destination_target: None,
                    all_this_turn: true,
                },
            ),
        )
    }

    pub fn subject_verb_redirect_all_damage_this_turn_by_source_to_source_controller(
        source: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                    source,
                },
            ),
        )
    }

    pub fn subject_verb_scoped_damage_redirection(target: TargetAst, scope: TimedDamageRedirectionAst) -> Self {
        Self::subject_verb(SubjectVerbRoleAst::Actor, PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnToTarget {
                player_filter: PlayerFilter::You, object_filter: ObjectFilter::default(), target, scope: Some(scope),
            }))
    }

    pub fn subject_verb_redirect_all_damage_this_turn_to_target(
        player_filter: PlayerFilter,
        object_filter: ObjectFilter,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DamagePrevention(
                DamagePreventionActionAst::RedirectAllDamageThisTurnToTarget {
                    player_filter,
                    object_filter,
                    target,
                    scope: None,
                },
            ),
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
                new_target_restriction: None,
            }),
        )
    }

    /// Attach "The new target must be ..." to a stack retarget instruction.
    /// Returns false when this effect is not a retarget.
    pub fn set_retarget_new_target_restriction(
        &mut self,
        restriction: ironsmith_core::NewTargetRestriction,
    ) -> bool {
        if let Self::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Stack(StackActionAst::RetargetStackObject {
                    new_target_restriction,
                    ..
                }),
            ..
        }) = self
        {
            *new_target_restriction = Some(restriction);
            return true;
        }
        false
    }

    /// Restrict retargeting to matching objects through the shared restriction.
    pub fn with_retarget_new_target_restriction(mut self, filter: ObjectFilter) -> Self {
        self.set_retarget_new_target_restriction(ironsmith_core::NewTargetRestriction::Object(filter));
        self
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
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllAttachedTo {
                filter,
                target,
            }),
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
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeSourceWhenLeaves {
                target,
            }),
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

    pub fn subject_verb_register_zone_replacement_with_counters_and_linked_exile_follow_up(
        target: TargetAst,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
        counters: Vec<(CounterType, u32)>,
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
                counters,
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
            SubjectVerbActionAst::Replacements(
                ReplacementActionAst::RegisterFutureZoneReplacement {
                    filter,
                    from_zone,
                    to_zone,
                    replacement_zone,
                    duration,
                    cause_policy,
                    link_exiled_to_source,
                    cast_this_way_surface: false,
                },
            ),
        )
    }

    /// Mark a future zone replacement as authored with "a spell cast this way".
    pub fn with_cast_this_way_replacement_surface(mut self) -> Self {
        if let EffectAst::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::Replacements(
                ReplacementActionAst::RegisterFutureZoneReplacement {
                    cast_this_way_surface,
                    ..
                },
            ) = &mut subject_verb.action
        {
            *cast_this_way_surface = true;
        }
        self
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
                player_target: None,
                display: None,
                lose_game: false,
            }),
        )
    }

    /// Turns a timed draw-replacement registration into one for the player's
    /// next game loss (CR 104.3, 614.1). No-op for other effects or `false`.
    pub fn with_lose_game_replacement(mut self, lose: bool) -> Self {
        if lose
            && let Self::SubjectVerb(subject_verb) = &mut self
            && let SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDrawReplacement {
                lose_game,
                ..
            }) = &mut subject_verb.action
        {
            *lose_game = true;
        }
        self
    }

    pub fn subject_verb_register_timed_draw_replacement(
        player: PlayerFilter, player_target: Option<TargetAst>, replacement_effects: Vec<Self>,
        duration: ZoneReplacementDurationAst, display: String,
    ) -> Self {
        Self::subject_verb(SubjectVerbRoleAst::Actor, PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDrawReplacement {
                player, replacement_effects, duration, player_target, display: Some(display),
                lose_game: false,
            }))
    }

    pub fn subject_verb_register_counter_placement_replacement(
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        additional: u32,
        mode: crate::effects::ReplacementApplyMode,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(
                ReplacementActionAst::RegisterCounterPlacementReplacement {
                    filter,
                    counter_type,
                    additional,
                    mode,
                },
            ),
        )
    }

    pub fn subject_verb_register_mana_rewrite(rule: ironsmith_core::ManaOutputRewrite, target: Option<TargetAst>,
        mode: crate::effects::ReplacementApplyMode, display: String) -> Self {
        Self::subject_verb(SubjectVerbRoleAst::Actor, PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterManaRewrite { rule, target, mode, display }))
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
            SubjectVerbActionAst::Replacements(
                ReplacementActionAst::RegisterDamagedBySourceZoneReplacement {
                    filter,
                    from_zone,
                    to_zone,
                    replacement_zone,
                    duration,
                },
            ),
        )
    }

    pub fn subject_verb_register_enter_under_control_replacement(
        filter: ObjectFilter,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(
                ReplacementActionAst::RegisterEnterUnderControlReplacement { filter, duration },
            ),
        )
    }

    pub fn subject_verb_register_enter_tapped_replacement(
        filter: ObjectFilter,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(
                ReplacementActionAst::RegisterEnterTappedReplacement { filter, duration },
            ),
        )
    }

    pub fn subject_verb_register_enter_with_counters_replacement(
        filter: ObjectFilter,
        counter_type: CounterType,
        count: Value,
        mode: crate::effects::ReplacementApplyMode,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(
                ReplacementActionAst::RegisterEnterWithCountersReplacement {
                    filter,
                    counter_type,
                    count,
                    mode,
                },
            ),
        )
    }

    pub fn subject_verb_register_next_batch_enter_as_copy(
        filter: ObjectFilter,
        source: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(
                ReplacementActionAst::RegisterNextBatchEnterAsCopy { filter, source },
            ),
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
            SubjectVerbActionAst::Replacements(
                ReplacementActionAst::RegisterNextBatchEnterWithCounters {
                    filter,
                    counter_type,
                    count,
                },
            ),
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
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::EmitKeywordAction {
                action,
                amount,
            }),
        )
    }

    pub fn subject_verb_collect_evidence(amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor, PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::CollectEvidence { amount }),
        )
    }

    pub fn subject_verb_empower_jace(amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::EmpowerJace { amount }),
        )
    }

    pub fn subject_verb_amass(subtype: Option<Subtype>, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Amass { subtype, amount }),
        )
    }

    pub fn subject_verb_bolster(amount: impl Into<Value>) -> Self {
        let amount = amount.into();
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Bolster { amount }),
        )
    }

    pub fn subject_verb_support(amount: impl Into<Value>) -> Self {
        let amount = amount.into();
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

    pub fn subject_verb_airbend(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Airbend { target }),
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
        Self::subject_verb_dont_lose_this_mana_as_steps_and_phases_end_this_turn_by(
            PlayerAst::Implicit,
            None,
        )
    }

    pub fn subject_verb_dont_lose_this_mana_as_steps_and_phases_end_this_turn_by(
        player: PlayerAst,
        color: Option<crate::color::Color>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn {
                color,
                until_end_of_combat: false,
            }),
        )
    }

    pub fn subject_verb_roll_to_visit_attractions(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::RollToVisitAttractions),
        )
    }

    pub fn subject_verb_roll_planar_die(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::RollPlanarDie),
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

    pub fn subject_verb_earthbend(counters: impl Into<Value>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Earthbend {
                counters: counters.into(),
            }),
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
            action:
                SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fight { mutual_surface, .. }),
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

    pub fn subject_verb_add_mana_lost_this_way(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaLostThisWay { amount }),
        )
    }

    pub fn subject_verb_add_mana_noted_type(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaNotedType { amount, use_noted_amount: false }),
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

    pub fn subject_verb_add_mana_equal_to_mana_cost(
        player: PlayerAst,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaEqualToManaCost {
                filter,
                colored_symbols_only: false,
            }),
        )
    }

    /// "For each colored mana symbol in <card>'s mana cost, add one mana of
    /// that color" (Charmed Pendant).
    pub fn subject_verb_add_mana_for_each_colored_symbol_in_mana_cost(
        player: PlayerAst,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaEqualToManaCost {
                filter,
                colored_symbols_only: true,
            }),
        )
    }

    pub fn subject_verb_add_mana_any_color_among(
        player: PlayerAst,
        amount: Value,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaAnyColorAmong { amount, filter }),
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

    pub fn subject_verb_add_mana_draft_chosen_colors(
        player: PlayerAst,
        amount: Value,
        card_name: String,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaDraftChosenColors {
                amount,
                card_name,
                chosen_at_deck_construction: false,
            }),
        )
    }

    /// "Add one mana of either of the circled colors" (Cryptic Spires): the
    /// colors circled for this card as its owner built their deck.
    pub fn subject_verb_add_mana_deck_construction_chosen_colors(
        player: PlayerAst,
        amount: Value,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::AddManaDraftChosenColors {
                amount,
                card_name: String::new(),
                chosen_at_deck_construction: true,
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

    pub fn subject_verb_exchange_source_text_box(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeTextBoxes {
                target,
                include_source: true,
            }),
        )
    }

    pub fn subject_verb_exchange_text_boxes(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeTextBoxes {
                target,
                include_source: false,
            }),
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
                controller: None,
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
            SubjectVerbActionAst::Stack(StackActionAst::ReduceNextSpellCostThisTurn {
                filter,
                reduction,
            }),
        )
    }

    pub fn subject_verb_free_cast_next_spell_this_turn(
        player: PlayerAst,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Stack(StackActionAst::FreeCastNextSpellThisTurn {
                filter,
                pay_life_equal_to_mana_value: false,
            }),
        )
    }

    /// "Rather than pay the mana cost of the next spell you cast this turn,
    /// you may pay life equal to that spell's mana value."
    pub fn subject_verb_pay_life_for_next_spell_this_turn(
        player: PlayerAst,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Stack(StackActionAst::FreeCastNextSpellThisTurn {
                filter,
                pay_life_equal_to_mana_value: true,
            }),
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
                increase: false,
            }),
        )
    }

    /// "Spells <player> cast(s) this turn cost {N} more to cast" (or, with
    /// `next_only`, the next such spell): a resolving, turn-scoped increase.
    pub fn subject_verb_increase_matching_spell_cost(
        player: PlayerAst,
        filter: ObjectFilter,
        increase: Value,
        duration: Until,
        next_only: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Stack(StackActionAst::ReduceMatchingSpellCostThisTurn {
                filter,
                reduction: increase,
                duration,
                next_only,
                increase: true,
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
                increase: false,
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
                manifest: false,
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
                manifest: false,
                shuffle_before,
            }),
        )
    }

    /// Manifest the chosen/tagged cards (CR 701.40a): each is put onto the
    /// battlefield face down as a 2/2 creature.
    pub fn subject_verb_manifest_onto_battlefield(
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
                cloak: false,
                manifest: true,
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

    /// "Target player looks at the top three cards of their library": the
    /// subject player both owns the library and is the one who looks, so the
    /// subject is recorded in the actor role (the viewer of the private look).
    pub fn subject_verb_player_looks_at_own_top_cards(
        player: PlayerAst,
        count: Value,
        tag: TagRef,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {
                count,
                tag,
                reveal: false,
            }),
        )
    }

    pub fn subject_verb_reveal_top_cards(player: PlayerAst, count: Value, tag: TagRef) -> Self {
        Self::subject_verb_top_library_cards(player, count, tag, true)
    }

    pub fn subject_verb_look_at_objects(player: PlayerAst, filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtObjects { filter, permit_while_exiled: false }),
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
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {
                count,
                tag,
                reveal,
            }),
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
            SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::AdditionalLandPlays {
                count,
                duration,
            }),
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
                shuffle_subject_library: false,
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
                shuffle_subject_library: false,
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
                shuffle_subject_library: false,
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
                shuffle_subject_library: false,
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
                shuffle_subject_library: false,
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
            SubjectVerbActionAst::Random(RandomActionAst::RollDie { sides, surface, result_modifier: None }),
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
                ignore_lower: false,
                in_order: false,
            }),
        )
    }

    /// "Roll X d6" whose results are read back as their total.
    pub fn subject_verb_roll_dice_total_with_surface(
        player: PlayerAst,
        count: Value,
        sides: u32,
        surface: Option<DieSurface>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Random(RandomActionAst::RollDiceTotal {
                count,
                sides,
                surface,
                highest_only: false,
            }),
        )
    }

    /// "Roll a d20 for each player being attacked and ignore all but the
    /// highest roll" (Iron Mastiff): a dynamic number of dice whose result is
    /// the highest roll.
    pub fn subject_verb_roll_dice_highest_with_surface(
        player: PlayerAst,
        count: Value,
        sides: u32,
        surface: Option<DieSurface>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Random(RandomActionAst::RollDiceTotal {
                count,
                sides,
                surface,
                highest_only: true,
            }),
        )
    }

    /// "Roll a six-sided die twice": two rolls kept in order ("the first
    /// result", "the second result").
    pub fn subject_verb_roll_die_repeatedly_in_order_with_surface(
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
                ignore_lower: false,
                in_order: true,
            }),
        )
    }

    /// "Roll two d20 and ignore the lower roll."
    pub fn subject_verb_roll_dice_ignore_lower_with_surface(
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
                ignore_lower: true,
                in_order: false,
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
            SubjectVerbActionAst::Library(
                LibraryActionAst::ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary,
            ),
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

    pub fn subject_verb_reorder_graveyard_at_random(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::ReorderGraveyardAtRandom),
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
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCardType { options, shared_among: None }),
        )
    }

    pub fn subject_verb_choose_shared_card_type(player: PlayerAst, filter: ObjectFilter, count: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCardType {
                options: Vec::new(), shared_among: Some((filter, count)),
            }),
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
                allowed_subtypes: Vec::new(),
                secretly: false,
                excluded_subtypes,
                family: SubtypeFamily::Creature,
                note_for_source: false,
            }),
        )
    }

    pub fn subject_verb_choose_subtype_options(
        player: PlayerAst,
        options: Vec<Subtype>,
        secretly: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCreatureType {
                allowed_subtypes: options,
                secretly,
                excluded_subtypes: Vec::new(),
                family: SubtypeFamily::Creature,
                note_for_source: false,
            }),
        )
    }

    /// "Note a creature type that hasn't been noted for this Saga."
    pub fn subject_verb_note_unnoted_creature_type(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCreatureType {
                allowed_subtypes: Vec::new(),
                secretly: false,
                excluded_subtypes: Vec::new(),
                family: SubtypeFamily::Creature,
                note_for_source: true,
            }),
        )
    }

    pub fn subject_verb_choose_subtype_type(player: PlayerAst, family: SubtypeFamily) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCreatureType {
                allowed_subtypes: Vec::new(),
                secretly: false,
                excluded_subtypes: Vec::new(),
                family,
                note_for_source: false,
            }),
        )
    }

    pub fn subject_verb_choose_land_type(player: PlayerAst, exclude_basic: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseLandType {
                exclude_basic,
                basic_only: false,
            }),
        )
    }

    pub fn subject_verb_choose_basic_land_type(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseLandType {
                exclude_basic: false,
                basic_only: true,
            }),
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
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCardName { filter, tag, options: Vec::new(), exclude_source_history: false }),
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

    pub fn subject_verb_phase_out_until_source_leaves(
        target: TargetAst,
        source_surface: SourceReferenceSurface,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOut {
                target,
                duration: crate::effects::PhaseOutDuration::UntilSourceLeaves,
                source_surface: Some(source_surface),
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
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseInAll { filter, simultaneous_phase_out: None }),
        )
    }

    pub fn subject_verb_phase_exchange(phase_in: ObjectFilter, phase_out: ObjectFilter) -> Self {
        Self::subject_verb(SubjectVerbRoleAst::Actor, PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseInAll {
                filter: phase_in, simultaneous_phase_out: Some(phase_out),
            }))
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
                source_controller_may_look: false,
                source_top_only: false,
                target_plural_surface: false,
            }),
        )
    }

    pub fn subject_verb_exile_with_source_controller_look(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile {
                target,
                face_down: true,
                source_controller_may_look: true,
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
            SubjectVerbActionAst::Stack(StackActionAst::Counter {
                target,
                exile_permission: None,
            }),
        )
    }

    /// The explicit "you" in the permission is the controller of this
    /// resolving spell or ability. The counter remains unconditional; the
    /// characteristic gate belongs only to its exile destination rider.
    pub fn subject_verb_counter_with_exile_permission(
        target: TargetAst,
        permission: ironsmith_core::CounterExilePermission,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::Stack(StackActionAst::Counter {
                target,
                exile_permission: Some(permission),
            }),
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
                maximum_total: None,
                counter_type,
                count,
                target,
                target_count,
                distributed,
                required_recipient: None,
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
            SubjectVerbActionAst::Counters(CounterActionAst::MoveAllCounters { from, to, remove_from_source: true }),
        )
    }

    pub fn subject_verb_put_referenced_counters(from: TargetAst, to: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::MoveAllCounters {
                from, to, remove_from_source: false,
            }),
        )
    }

    pub fn subject_verb_move_counters(
        counter_type: CounterType,
        count: Value,
        from: TargetAst,
        to: TargetAst,
    ) -> Self {
        Self::subject_verb_move_counters_amount(counter_type, ironsmith_core::effect::CounterMoveAmount::Exact(count), from, to)
    }

    pub fn subject_verb_move_counters_amount(
        counter_type: CounterType,
        count: ironsmith_core::effect::CounterMoveAmount,
        from: TargetAst,
        to: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::MoveCounters {
                counter_type,
                count,
                from,
                to,
                from_all: false,
            }),
        )
    }

    pub fn subject_verb_move_counters_from_all(
        counter_type: CounterType,
        count: ironsmith_core::effect::CounterMoveAmount,
        from: ObjectFilter,
        to: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::MoveCounters {
                counter_type, count, from: TargetAst::Object(from, None, None), to,
                from_all: true,
            }),
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

    /// "For each kind of counter on target permanent or player, give that
    /// permanent or player another counter of that kind" (CR 122.1: one more
    /// counter of every kind already there, players included).
    pub fn subject_verb_for_each_counter_kind_put_another(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counters(CounterActionAst::ForEachCounterKindPutOrRemove {
                target,
                counter_source: None,
                all_kinds: true,
                fixed_counter_type: None,
                optional_action: false,
                put_only: true,
                alternative_put_counter: None,
                choose_target_per_kind: false,
            }),
        )
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
                alternative_put_counter: None,
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
                alternative_put_counter: None,
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
                alternative_put_counter: None,
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
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHandOfChosenColor {
                filter,
            }),
        )
    }

    pub fn subject_verb_move_to_library_nth_from_top(target: TargetAst, position: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryNthFromTop {
                target,
                position,
            }),
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
            SubjectVerbActionAst::PutSticker {
                target,
                action,
                max_ticket_cost: None,
                without_paying_ticket_cost: false,
            },
        )
    }

    /// "put an ability sticker with ticket cost X or less on it without paying
    /// that sticker's ticket cost" (Pin Collection).
    pub fn subject_verb_put_sticker_with_ticket_terms(
        target: TargetAst,
        action: crate::events::KeywordActionKind,
        max_ticket_cost: Option<Value>,
        without_paying_ticket_cost: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutSticker {
                target,
                action,
                max_ticket_cost,
                without_paying_ticket_cost,
            },
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
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::SwitchPowerToughness {
                target,
                duration,
            }),
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

    pub fn subject_verb_rad_counters(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Counters(CounterActionAst::RadCounters { count }),
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
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyLife { min_amount, maximum: None, source_owned: false }),
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
                independent_x_choice: false,
                mana_cost_of: None,
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
                independent_x_choice: false,
                mana_cost_of: None,
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

    pub fn subject_verb_activate_mana_ability_of_each_land(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Mana(ManaActionAst::ActivateManaAbilityOfEachLand),
        )
    }

    pub fn subject_verb_reduce_speed(player: PlayerAst, amount: Value, minimum: u8) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::ReduceSpeed {
                amount,
                minimum,
            }),
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
            SubjectVerbActionAst::TurnStructure(
                TurnStructureActionAst::SkipNextCombatPhaseThisTurn,
            ),
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

    pub fn subject_verb_additional_phases_with_main_surface(
        phases: Vec<crate::effects::AdditionalPhase>,
        after_main_phase: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::AdditionalPhases {
                phases,
                after_main_phase,
            }),
        )
    }

    pub fn subject_verb_additional_phases(phases: Vec<crate::effects::AdditionalPhase>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::AdditionalPhases {
                phases,
                after_main_phase: false,
            }),
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

    pub fn subject_verb_draw_game() -> Self {
        Self::subject_verb(SubjectVerbRoleAst::Actor, PlayerAst::Implicit,
            SubjectVerbActionAst::Game(GameActionAst::DrawGame))
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
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Goad {
                target,
                duration,
                spelled_out_requirement: false,
            }),
        )
    }

    /// "<creatures> attack each combat if able and attack a player other than
    /// you if able" — the goad requirement written out (Kardur, Doomscourge).
    pub fn subject_verb_goad_requirement(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Goad {
                target,
                duration,
                spelled_out_requirement: true,
            }),
        )
    }

    pub fn subject_verb_become_plotted(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::BecomePlotted { target }),
        )
    }

    pub fn subject_verb_become_foretold(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::BecomeForetold {
                target,
                foretell_cost_reduction: None,
            }),
        )
    }

    pub fn subject_verb_prepare(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Prepare {
                target,
                unprepare: false,
            }),
        )
    }

    pub fn subject_verb_unprepare(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Prepare {
                target,
                unprepare: true,
            }),
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

    pub fn subject_verb_clear_goad(target: Option<TargetAst>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::ClearGoad { target }),
        )
    }

    pub fn subject_verb_heal_damage(target: TargetAst, amount: Option<Value>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Damage(DamageActionAst::HealDamage { target, amount }),
        )
    }

    pub fn subject_verb_become_blocked(target: TargetAst) -> Self {
        Self::subject_verb(SubjectVerbRoleAst::Actor, PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::BecomeBlocked{target}))
    }

    pub fn subject_verb_reselect_attack_target(target: TargetAst, players_only: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::ReselectAttackTarget {
                target,
                players_only,
                attacked_player: None,
                enters_combat: false,
                among_attacked_by: None,
            }),
        )
    }

    /// "<attacking creatures> are now attacking <player>" (CR 506.4).
    pub fn subject_verb_now_attacking_player(target: TargetAst, player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::ReselectAttackTarget {
                target,
                players_only: true,
                attacked_player: Some(player),
                enters_combat: false,
                among_attacked_by: None,
            }),
        )
    }

    /// "it's attacking that player": a creature not in combat becomes an
    /// attacking creature attacking that player (CR 506.3d).
    pub fn subject_verb_attacking_player(target: TargetAst, player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::ReselectAttackTarget {
                target,
                players_only: true,
                attacked_player: Some(player),
                enters_combat: true,
                among_attacked_by: None,
            }),
        )
    }

    /// "<it> is attacking a player or planeswalker <player> is attacking"
    /// (Tahngarth, First Mate): a creature not in combat becomes attacking
    /// one of the things that player's attackers attack.
    pub fn subject_verb_attacking_one_attacked_by(target: TargetAst, player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::ReselectAttackTarget {
                target,
                players_only: false,
                attacked_player: None,
                enters_combat: true,
                among_attacked_by: Some(player),
            }),
        )
    }

    pub fn subject_verb_remove_from_combat(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::RemoveFromCombat {
                target,
            }),
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
                remove_damage_only: false,
            }),
        )
    }

    /// "The next time <target> would be destroyed this turn, remove all
    /// damage marked on it instead." (Pyramids)
    pub fn subject_verb_destruction_damage_removal_shield(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Regenerate {
                target,
                follow_up_effects: Vec::new(),
                remove_damage_only: true,
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
                remove_damage_only: false,
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
