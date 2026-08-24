use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SubjectVerbRoleAst {
    Actor,
    AffectedPlayer,
    Chooser,
    LibraryOwner,
}

#[derive(Clone, PartialEq)]
pub struct SubjectVerbSubjectAst {
    pub role: SubjectVerbRoleAst,
    pub player: PlayerAst,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnAsAuraAst {
    pub attachment_filter: ObjectFilter,
    pub remove_all_abilities: bool,
    pub granted_abilities: Vec<GrantedAbilityAst>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmblemDescriptionAst {
    pub text: String,
    pub abilities: Vec<EmblemAbilityAst>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EmblemAbilityAst {
    Static(Vec<StaticAbilityAst>),
    Activated(ParsedAbility),
    Triggered {
        trigger: TriggerSpec,
        effects: Vec<EffectAst>,
        trigger_limit_condition: Option<ConditionExpr>,
    },
}

#[derive(Clone, PartialEq)]
pub enum SubjectVerbActionAst {
    Draw {
        count: Value,
    },
    DrawForEachTaggedMatching {
        tag: TagKey,
        filter: ObjectFilter,
    },
    LoseLife {
        amount: Value,
    },
    PayLife {
        amount: Value,
    },
    GainLife {
        amount: Value,
    },
    RevealHand,
    Mill {
        count: Value,
    },
    Scry {
        count: Value,
    },
    Surveil {
        count: Value,
    },
    Proliferate {
        count: Value,
    },
    Investigate {
        count: Value,
    },
    Incubate {
        amount: Value,
        count: Value,
    },
    Learn,
    EmitKeywordAction {
        action: crate::events::KeywordActionKind,
        amount: u32,
    },
    ReorderTopPlanarDeck {
        count: u32,
    },
    ReturnSourceTransformedFromExile,
    Reconfigure {
        target: TargetAst,
    },
    CumulativeUpkeep {
        cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
    },
    Casualty {
        power: u32,
    },
    Amass {
        subtype: Option<Subtype>,
        amount: Value,
    },
    Bolster {
        amount: u32,
    },
    Support {
        amount: u32,
    },
    Adapt {
        amount: u32,
    },
    Monstrosity {
        amount: Value,
    },
    Discover {
        count: Value,
    },
    Fateseal {
        count: Value,
    },
    Populate {
        count: Value,
        enters_tapped: bool,
        enters_attacking: bool,
        has_haste: bool,
        sacrifice_at_next_end_step: bool,
        exile_at_next_end_step: bool,
        next_end_step_player: PlayerFilter,
        exile_at_end_of_combat: bool,
        sacrifice_at_end_of_combat: bool,
    },
    Explore {
        target: TargetAst,
    },
    Endure {
        target: TargetAst,
        amount: Value,
    },
    Exploit,
    Connive {
        target: TargetAst,
        count: Value,
    },
    ConniveIterated,
    OpenAttraction {
        reminder: bool,
    },
    ManifestTopCardOfLibrary,
    CloakTopCardOfLibrary,
    ManifestCardFromHand,
    ManifestDread,
    Earthbend {
        counters: u32,
    },
    Behold {
        subtype: Subtype,
        count: u32,
    },
    Fight {
        creature1: TargetAst,
        creature2: TargetAst,
    },
    FightIterated {
        creature2: TargetAst,
    },
    Clash {
        opponent: ClashOpponentAst,
    },
    FlipCoin,
    /// Flip without a call when only the physical heads/tails face matters.
    FlipCoinFaceOnly,
    RollDie {
        sides: u32,
        die_text: Option<String>,
    },
    RollDiceChooseResult {
        count: u32,
        sides: u32,
        die_text: Option<String>,
    },
    ShuffleHandAndGraveyardIntoLibrary,
    ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary,
    ShuffleGraveyardIntoLibrary {
        explicit_all_cards_from: bool,
    },
    ReorderGraveyard,
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
        tag: TagKey,
    },
    ChoosePlayer {
        filter: PlayerFilter,
        tag: TagKey,
        random: bool,
        exclude_previous_choices: usize,
    },
    NoteLifeTotal,
    ChooseSpellCastHistory {
        cast_by: PlayerAst,
        filter: ObjectFilter,
        tag: TagKey,
    },
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
    ExchangeLifeTotals {
        player2: PlayerAst,
    },
    ExchangeTextBoxes {
        target: TargetAst,
    },
    ExchangeZones {
        zone1: Zone,
        zone2: Zone,
    },
    PutRestOnBottomOfLibrary,
    DontLoseThisManaAsStepsAndPhasesEndThisTurn,
    ExchangeValues {
        left: ExchangeValueAst,
        right: ExchangeValueAst,
        duration: Until,
    },
    ExchangeControl {
        filter: ObjectFilter,
        count: u32,
        shared_type: Option<SharedTypeConstraintAst>,
    },
    ExchangeControlHeterogeneous {
        permanent1: TargetAst,
        permanent2: TargetAst,
        shared_type: Option<SharedTypeConstraintAst>,
    },
    Attach {
        object: TargetAst,
        target: TargetAst,
    },
    Unattach {
        object: TargetAst,
    },
    Enchant {
        filter: AuraAttachmentFilter,
    },
    ExileWhenSourceLeaves {
        target: TargetAst,
    },
    SacrificeSourceWhenLeaves {
        target: TargetAst,
    },
    RegisterZoneReplacement {
        target: TargetAst,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        library_placement: Option<ironsmith_core::ZoneReplacementLibraryPlacement>,
        duration: ZoneReplacementDurationAst,
        optional: bool,
        choice_description: Option<String>,
        counters: Vec<(CounterType, u32)>,
        linked_exile_follow_up: Option<ironsmith_core::LinkedExileFollowUp>,
    },
    RegisterFutureZoneReplacement {
        filter: ObjectFilter,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
        cause_policy: FutureZoneReplacementCausePolicyAst,
        link_exiled_to_source: bool,
    },
    RegisterDrawReplacement {
        player: PlayerFilter,
        replacement_effects: Vec<EffectAst>,
        duration: ZoneReplacementDurationAst,
    },
    RegisterManaReplacement {
        source_filter: ObjectFilter,
        replacement_mana: Vec<ManaSymbol>,
        mode: crate::effects::ReplacementApplyMode,
    },
    RegisterDamagedBySourceZoneReplacement {
        filter: ObjectFilter,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
    },
    RegisterEnterUnderControlReplacement {
        filter: ObjectFilter,
        duration: ZoneReplacementDurationAst,
    },
    RegisterEnterTappedReplacement {
        filter: ObjectFilter,
        duration: ZoneReplacementDurationAst,
    },
    RegisterNextBatchEnterWithCounters {
        filter: ObjectFilter,
        counter_type: CounterType,
        count: Value,
    },
    ExileInsteadOfGraveyardThisTurn,
    ControlCombatChoicesThisTurn {
        attackers: bool,
        blockers: bool,
        this_combat: bool,
    },
    GainControl {
        target: TargetAst,
        duration: Until,
        condition: Option<ConditionExpr>,
        /// Explicit object whose controller performs the control change.
        ///
        /// This preserves authored relational subjects such as "that
        /// source's controller" without resolving them through the generic
        /// last-object antecedent.
        controller_reference: Option<ObjectRef>,
        source_reference_surface: Option<SourceReferenceSurface>,
    },
    RevealTop,
    ExileTopOfLibrary {
        count: Value,
        surface: Option<ironsmith_core::ExileTopLibrarySurface>,
        tags: Vec<TagKey>,
        accumulated_tags: Vec<TagKey>,
        face_down: bool,
    },
    RevealTagged {
        tag: TagKey,
    },
    /// Put the chosen/iterated objects onto the battlefield under a resolved
    /// controller. Inside a `ForEachTagged`, `TargetAst::Tagged(IT_TAG)` lowers
    /// to `ChooseSpec::Iterated`; otherwise the tagged collection is used.
    /// Lowers to `Effect::put_onto_battlefield`.
    PutOntoBattlefield {
        target: TargetAst,
        tapped: bool,
        controller: ReturnControllerAst,
        cloak: bool,
        shuffle_before: bool,
    },
    RevealCardsFromHand {
        count: ChoiceCount,
        count_value: Option<Value>,
        tag: TagKey,
    },
    LookAtTopCards {
        count: Value,
        tag: TagKey,
        reveal: bool,
    },
    LookAtObjects {
        filter: ObjectFilter,
    },
    LookAtTarget {
        target: TargetAst,
    },
    MayMoveToZone {
        target: TargetAst,
        zone: Zone,
    },
    AdditionalLandPlays {
        count: Value,
        duration: Until,
    },
    ExtraTurnAfterTurn {
        anchor: ExtraTurnAnchorAst,
    },
    ReorderTopOfLibrary {
        tag: TagKey,
    },
    AddManaImprintedColors,
    ShuffleLibrary,
    ShuffleObjectsIntoLibrary {
        target: TargetAst,
        all: bool,
        owner_library_destination: bool,
        possessive_owner_subject: bool,
    },
    GrantProtectionChoice {
        target: TargetAst,
        chooser: PlayerAst,
        allow_colorless: bool,
        allow_artifacts: bool,
        choose_card_type: bool,
    },
    PreventAllCombatDamage {
        duration: Until,
    },
    AssignNoCombatDamage {
        source: TargetAst,
        duration: Until,
    },
    PreventAllCombatDamageFromSource {
        duration: Until,
        source: TargetAst,
        source_would_deal_surface: bool,
    },
    PreventAllCombatDamageFromSourceFilter {
        duration: Until,
        source_filter: ObjectFilter,
        excluded_source_target: Option<TargetAst>,
    },
    PreventAllCombatDamageToPlayers {
        duration: Until,
    },
    PreventAllCombatDamageToYou {
        duration: Until,
    },
    PreventNextTimeDamage {
        source: PreventNextTimeDamageSourceAst,
        target: PreventNextTimeDamageTargetAst,
        reflect_damage_to_source_controller: bool,
        follow_up_effects: Vec<EffectAst>,
    },
    ReplaceNextDamageToTarget {
        target: TargetAst,
        damage_target_tag: TagKey,
        replacement_effects: Vec<EffectAst>,
    },
    PreventDamage {
        amount: Value,
        target: TargetAst,
        duration: Until,
        source_of_your_choice: bool,
        protect_you_and_permanents_you_control: bool,
        follow_up_effects: Vec<EffectAst>,
    },
    PreventAllDamageToTarget {
        target: TargetAst,
        duration: Until,
        source_of_your_choice: bool,
        source_choice_shares_activation_mana_color: bool,
        source_target: Option<TargetAst>,
    },
    PreventAllDamageToTargetFromSourceFilter {
        target: TargetAst,
        duration: Until,
        source_filter: ObjectFilter,
    },
    PreventAllDamageFromSourceFilter {
        duration: Until,
        source_filter: ObjectFilter,
    },
    PreventDamageToTargetPutCounters {
        amount: Option<Value>,
        target: TargetAst,
        duration: Until,
        counter_type: CounterType,
    },
    PreventDamageEach {
        amount: Value,
        filter: ObjectFilter,
        duration: Until,
    },
    CopySpell {
        target: TargetAst,
        /// Authored kind of a stack-object back-reference. Tagged targets
        /// retain identity but not whether the source text named a spell,
        /// ability, or their union.
        target_reference_kind: Option<crate::filter::StackObjectKind>,
        /// The authored target back-reference was the pronoun `it`.
        ///
        /// This survives reference resolution independently of the semantic
        /// target tag so compiled text can reproduce the original pronoun.
        target_reference_pronoun: bool,
        /// Copy every matching stack object instead of choosing one match.
        ///
        /// This is intentionally part of the typed action rather than inferred
        /// from the target filter: `copy target spell` and `copy all spells`
        /// may otherwise lower to the same `ObjectFilter` and lose the printed
        /// set quantifier before runtime execution.
        all_matches: bool,
        count: Value,
        count_surface: Option<ironsmith_core::effect::CopyCountSurface>,
        player: PlayerAst,
        may_choose_new_targets: bool,
        choose_new_target_singular: bool,
        removed_supertypes: Vec<crate::types::Supertype>,
        /// Colors set by an explicit copy exception, such as
        /// "except that the copy is red."
        set_colors: Option<crate::color::ColorSet>,
        /// Card types added by an explicit copy exception, such as
        /// "except the copy is an artifact in addition to its other types."
        added_card_types: Vec<CardType>,
        /// Subtypes added by an explicit copy exception while retaining the
        /// copied spell's other types.
        added_subtypes: Vec<Subtype>,
        /// Base power and toughness set by an explicit copy exception.
        set_base_power_toughness: Option<(i32, i32)>,
    },
    CopySpellForEachTarget {
        target: TargetAst,
        object_filter: Option<ObjectFilter>,
        player_filter: Option<PlayerFilter>,
        player: PlayerAst,
        exclude_current_targets: bool,
        removed_supertypes: Vec<crate::types::Supertype>,
    },
    ScaleXValue {
        target: TargetAst,
        multiplier: u32,
    },
    PutTaggedRemainderOnBottomOfLibrary {
        tag: TagKey,
        keep_tagged: Option<TagKey>,
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
        tag: TagKey,
        keep_tagged: TagKey,
        zone: Zone,
        surface: ironsmith_core::LibraryRemainderSurface,
    },
    CastTagged {
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        as_copy: bool,
        copy_cast_reminder_surface: bool,
        without_paying_mana_cost: bool,
        additional_mana_cost: Option<ManaCost>,
        cost_reduction: Option<ManaCost>,
        mana_spend_mode: ironsmith_core::value_model::ManaSpendMode,
    },
    GrantPlayTaggedUntilEndOfTurn {
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode,
        while_on_top_of_library: bool,
        free_cast_from_current_zone: bool,
        /// Use the source-exile event boundary instead of end of turn.
        until_source_exiles_another: bool,
        /// Total plays shared by the tagged collection across the duration.
        max_plays: Option<u32>,
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
    },
    GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
        tag: TagKey,
        player: PlayerAst,
    },
    GrantPlayTaggedUntilYourNextTurn {
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode,
        until_next_end_step: bool,
        /// Total plays shared by the tagged collection across the duration.
        max_plays: Option<u32>,
    },
    GrantPlayTaggedForAsLongAsExiled {
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode,
        filter: Option<ObjectFilter>,
        /// Restrict the persistent permission to turns in which this counter
        /// type was put on the ability source.
        during_turns_counter_put_on_source: Option<crate::object::CounterType>,
        /// Additional mana cost for nonland cards cast through this exact
        /// permission.
        spell_cost_increase: Option<ManaCost>,
        /// Whether lands played through this exact permission enter tapped.
        lands_enter_tapped: bool,
    },
    GrantPlayTaggedForAsLongAsYouControlSource {
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode,
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
    },
    ReturnToBattlefield {
        target: TargetAst,
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
    MoveToLibraryTopOrBottomChoice {
        target: TargetAst,
    },
    TargetOnly {
        target: TargetAst,
        explicit_declaration: bool,
    },
    TagMatchingObjects {
        filter: ObjectFilter,
        zones: Vec<Zone>,
        tag: TagKey,
        source_tags: Vec<TagKey>,
    },
    Pump {
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
        condition: Option<crate::ConditionExpr>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    SetBasePowerToughness {
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    BecomeBasePtCreature {
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
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
        duration: Until,
    },
    SetBasePower {
        power: Value,
        target: TargetAst,
        duration: Until,
    },
    PumpForEach {
        power_per: i32,
        toughness_per: i32,
        target: TargetAst,
        count: Value,
        duration: Until,
    },
    PumpAll {
        filter: ObjectFilter,
        power: Value,
        toughness: Value,
        duration: Until,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    PumpByLastEffect {
        power: i32,
        toughness: i32,
        target: TargetAst,
        duration: Until,
        includes_this_way: bool,
    },
    AddCardTypes {
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    },
    SetCardTypes {
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    },
    RemoveCardTypes {
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    },
    AddSubtypes {
        target: TargetAst,
        subtypes: Vec<Subtype>,
        duration: Until,
    },
    RemoveSubtypes {
        target: TargetAst,
        subtypes: Vec<Subtype>,
        duration: Until,
    },
    /// "becomes a Bird Giant" without "in addition": replaces the object's
    /// creature subtypes (CR 205.1b) instead of adding to them.
    SetCreatureSubtypes {
        target: TargetAst,
        subtypes: Vec<Subtype>,
        duration: Until,
    },
    BecomeSaddledUntilEndOfTurn {
        target: TargetAst,
    },
    AddColors {
        target: TargetAst,
        colors: ColorSet,
        duration: Until,
    },
    AddAllSubtypesOfFamily {
        target: TargetAst,
        family: SubtypeFamily,
        duration: Until,
    },
    RemoveAllSubtypesOfFamily {
        target: TargetAst,
        family: SubtypeFamily,
        duration: Until,
    },
    BecomeAuraEnchantment {
        target: TargetAst,
        attachment_filter: ObjectFilter,
        granted_abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    },
    BecomeBasicLandType {
        target: TargetAst,
        subtype: Subtype,
        duration: Until,
    },
    SetColors {
        target: TargetAst,
        colors: ColorSet,
        duration: Until,
    },
    MakeColorless {
        target: TargetAst,
        duration: Until,
    },
    BecomeBasicLandTypeChoice {
        target: TargetAst,
        duration: Until,
    },
    BecomeCreatureTypeChoice {
        target: TargetAst,
        duration: Until,
        excluded_subtypes: Vec<Subtype>,
    },
    BecomeColorChoice {
        target: TargetAst,
        duration: Until,
        allow_multiple: bool,
    },
    BecomeCopy {
        target: TargetAst,
        source: TargetAst,
        duration: Until,
        preserve_source_abilities: bool,
        name_override: Option<String>,
        name_override_surface: Option<SourceReferenceSurface>,
        add_supertypes: Vec<Supertype>,
        remove_supertypes: Vec<Supertype>,
        add_card_types: Vec<CardType>,
        set_card_types: Vec<CardType>,
        add_subtypes: Vec<Subtype>,
        set_subtypes: Vec<Subtype>,
        granted_abilities: Vec<GrantedAbilityAst>,
        set_base_power_toughness: Option<(Value, Value)>,
        copy_exception_surface: Option<String>,
    },
    GrantAbilitiesAll {
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: Option<crate::ConditionExpr>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
        /// CR 611.2c normally fixes the affected set when a resolving effect
        /// starts. Some rules effects instead create a continuous rule for a
        /// filter for the stated duration and must also affect later entrants.
        lock_filter_at_resolution: bool,
    },
    RemoveAbilitiesAll {
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: Option<crate::ConditionExpr>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    GrantAbilitiesChoiceAll {
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    },
    GrantAbilitiesToTarget {
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: Option<crate::ConditionExpr>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    GrantToTarget {
        target: TargetAst,
        grantable: Box<crate::model::CompilerGrantableCore>,
        duration: crate::grant::GrantDuration,
    },
    GrantBySpec {
        spec: Box<crate::model::CompilerGrantSpecCore>,
        player: PlayerAst,
        duration: crate::grant::GrantDuration,
    },
    RemoveAbilitiesFromTarget {
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    },
    GrantAbilitiesChoiceToTarget {
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    },
    ConsultTopOfLibrary {
        player: PlayerAst,
        mode: LibraryConsultModeAst,
        filter: ObjectFilter,
        stop_rule: LibraryConsultStopRuleAst,
        max_exposed: Option<Value>,
        all_tag: TagKey,
        match_tag: TagKey,
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
    Cant {
        restriction: crate::effect::Restriction,
        duration: crate::effect::Until,
        start: crate::effect::RestrictionStart,
        duration_surface: crate::effect::RestrictionDurationSurface,
        condition: Option<crate::ConditionExpr>,
    },
    CreateTokenCopy {
        object: ObjectRefAst,
        count: Value,
        player: PlayerAst,
        enters_tapped: bool,
        enters_attacking: bool,
        attack_target_player_or_planeswalker_controlled_by: Option<PlayerAst>,
        attack_target_player_only: bool,
        half_power_toughness_round_up: bool,
        has_haste: bool,
        haste_followup_reference_surface: Option<crate::effect::TokenCopyReferenceSurface>,
        exile_at_end_of_combat: bool,
        exile_at_end_of_combat_reference_surface: Option<crate::effect::TokenCopyReferenceSurface>,
        loses_soulbond: bool,
        sacrifice_at_next_end_step: bool,
        sacrifice_at_next_end_step_reference_surface:
            Option<crate::effect::TokenCopyReferenceSurface>,
        sacrifice_at_next_end_step_ability_text: Option<String>,
        exile_at_next_end_step: bool,
        exile_at_next_end_step_reference_surface: Option<crate::effect::TokenCopyReferenceSurface>,
        next_end_step_player: PlayerFilter,
        set_colors: Option<ColorSet>,
        set_card_types: Option<Vec<CardType>>,
        set_subtypes: Option<Vec<Subtype>>,
        added_card_types: Vec<CardType>,
        added_subtypes: Vec<Subtype>,
        removed_supertypes: Vec<Supertype>,
        set_base_power_toughness: Option<(i32, i32)>,
        set_base_power_toughness_to_source_totals: bool,
        starting_loyalty: Option<u32>,
        granted_abilities: Vec<GrantedAbilityAst>,
    },
    CreateTokenCopyFromSource {
        source: TargetAst,
        count: Value,
        player: PlayerAst,
        enters_tapped: bool,
        enters_attacking: bool,
        attack_target_player_or_planeswalker_controlled_by: Option<PlayerAst>,
        attack_target_player_only: bool,
        half_power_toughness_round_up: bool,
        has_haste: bool,
        haste_followup_reference_surface: Option<crate::effect::TokenCopyReferenceSurface>,
        exile_at_end_of_combat: bool,
        exile_at_end_of_combat_reference_surface: Option<crate::effect::TokenCopyReferenceSurface>,
        loses_soulbond: bool,
        sacrifice_at_next_end_step: bool,
        sacrifice_at_next_end_step_reference_surface:
            Option<crate::effect::TokenCopyReferenceSurface>,
        sacrifice_at_next_end_step_ability_text: Option<String>,
        exile_at_next_end_step: bool,
        exile_at_next_end_step_reference_surface: Option<crate::effect::TokenCopyReferenceSurface>,
        next_end_step_player: PlayerFilter,
        set_colors: Option<ColorSet>,
        set_card_types: Option<Vec<CardType>>,
        set_subtypes: Option<Vec<Subtype>>,
        added_card_types: Vec<CardType>,
        added_subtypes: Vec<Subtype>,
        removed_supertypes: Vec<Supertype>,
        set_base_power_toughness: Option<(i32, i32)>,
        set_base_power_toughness_to_source_totals: bool,
        starting_loyalty: Option<u32>,
        granted_abilities: Vec<GrantedAbilityAst>,
    },
    CreateTokenWithMods {
        name: String,
        definition: crate::model::token_definition::TokenDefinitionSpec,
        count: Value,
        dynamic_power_toughness: Option<(Value, Value)>,
        player: PlayerAst,
        /// The source text explicitly used `you` as the create-action actor.
        /// This does not participate in controller resolution.
        actor_surface_explicit: bool,
        attached_to: Option<TargetAst>,
        tapped: bool,
        attacking: bool,
        /// Authored player attacked by the created token (for example,
        /// `attacking that player` inside a per-opponent loop).
        attack_target_player: Option<PlayerAst>,
        exile_at_end_of_combat: bool,
        sacrifice_at_end_of_combat: bool,
        sacrifice_at_next_end_step: bool,
        exile_at_next_end_step: bool,
        next_end_step_player: PlayerFilter,
        granted_abilities: Vec<GrantedAbilityAst>,
        ability_presentation: Option<ironsmith_core::TokenAbilityPresentation>,
    },
    /// "Create your choice of A, B, or C" — one mode per option, each mode a
    /// complete create effect.
    CreateTokenChoice {
        options: Vec<(String, Box<EffectAst>)>,
    },
    RedirectNextDamageFromSourceToTarget {
        amount: Value,
        protected_target: Option<TargetAst>,
        destination: RedirectNextTimeDamageDestinationAst,
        destination_target: Option<TargetAst>,
    },
    RedirectNextTimeDamageToSource {
        source: PreventNextTimeDamageSourceAst,
        target: TargetAst,
        destination: RedirectNextTimeDamageDestinationAst,
        destination_target: Option<TargetAst>,
        all_this_turn: bool,
    },
    RedirectAllDamageThisTurnBySourceToSourceController {
        source: TargetAst,
    },
    RedirectAllDamageThisTurnToTarget {
        player_filter: PlayerFilter,
        object_filter: ObjectFilter,
        target: TargetAst,
    },
    Meld {
        result_name: String,
        enters_tapped: bool,
        enters_attacking: bool,
    },
    SearchLibrarySlotsToHand {
        slots: Vec<SearchLibrarySlotAst>,
        destination: Zone,
        reveal: bool,
        progress_tag: TagKey,
    },
    RetargetStackObject {
        target: TargetAst,
        mode: RetargetModeAst,
        require_change: bool,
        /// Preserve authored "the copies" independently of the copied
        /// stack-object tag and the per-event copy count.
        copy_reference_plural: bool,
    },
    GrantAbilityToSource {
        ability: Box<ParsedAbility>,
        duration: Until,
    },
    DealDamage {
        amount: Value,
        target: TargetAst,
        unpreventable: bool,
    },
    TurnFaceUp {
        target: TargetAst,
    },
    DealDamageEach {
        amount: Value,
        filter: ObjectFilter,
    },
    DealDamageEqualToPower {
        source: TargetAst,
        amount: Value,
        target: TargetAst,
        unpreventable: bool,
    },
    DealDistributedDamage {
        amount: Value,
        target: TargetAst,
        source: TargetAst,
        chooser: PlayerFilter,
        distribution: ironsmith_core::DamageDistributionMode,
    },
    Tap {
        target: TargetAst,
    },
    Untap {
        target: TargetAst,
    },
    TapAll {
        filter: ObjectFilter,
    },
    UntapAll {
        filter: ObjectFilter,
    },
    TapOrUntap {
        target: TargetAst,
    },
    TapOrUntapAll {
        tap_filter: ObjectFilter,
        untap_filter: ObjectFilter,
    },
    PhaseOut {
        target: TargetAst,
        duration: crate::effects::PhaseOutDuration,
        source_surface: Option<SourceReferenceSurface>,
    },
    PhaseOutAll {
        filter: ObjectFilter,
        duration: crate::effects::PhaseOutDuration,
        source_surface: Option<SourceReferenceSurface>,
    },
    PhaseIn {
        target: TargetAst,
    },
    PhaseInAll {
        filter: ObjectFilter,
    },
    Transform {
        target: TargetAst,
    },
    Convert {
        target: TargetAst,
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
    LookAtHand {
        target: TargetAst,
    },
    Counter {
        target: TargetAst,
    },
    CounterUnlessPays {
        target: TargetAst,
        cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
    },
    PutCounters {
        counter_type: CounterType,
        count: Value,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
        distributed: bool,
    },
    PutCounterChoice {
        counter_types: Vec<CounterType>,
        count: Value,
        mode_texts: Vec<String>,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
    },
    PutOrRemoveCounters {
        put_counter_type: CounterType,
        put_count: Value,
        remove_counter_type: CounterType,
        remove_count: Value,
        put_mode_text: String,
        remove_mode_text: String,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
    },
    PutCountersAll {
        counter_type: CounterType,
        count: Value,
        filter: ObjectFilter,
    },
    RemoveUpToAnyCounters {
        amount: Value,
        target: TargetAst,
        counter_type: Option<CounterType>,
        up_to: bool,
        distributed_across_all: bool,
        all_of_them: bool,
    },
    MoveAllCounters {
        from: TargetAst,
        to: TargetAst,
    },
    MoveOneCounter {
        from: TargetAst,
        to: TargetAst,
    },
    ForEachCounterKindPutOrRemove {
        target: TargetAst,
        all_kinds: bool,
        fixed_counter_type: Option<CounterType>,
        optional_action: bool,
    },
    PutCounterOfChosenKind {
        target: TargetAst,
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
    MoveToLibraryNthFromTop {
        target: TargetAst,
        position: Value,
    },
    DoubleCountersOnEach {
        counter_type: Option<CounterType>,
        filter: ObjectFilter,
    },
    DoubleCountersOnTarget {
        counter_type: Option<CounterType>,
        target: TargetAst,
    },
    RemoveCountersAll {
        amount: Value,
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        up_to: bool,
    },
    PutSticker {
        target: TargetAst,
        action: crate::events::KeywordActionKind,
    },
    UnlockRoomDoor,
    SwitchPowerToughness {
        target: TargetAst,
        duration: Until,
    },
    ScalePowerToughnessAll {
        filter: ObjectFilter,
        power: bool,
        toughness: bool,
        multiplier: i32,
        duration: Until,
    },
    Discard {
        count: Value,
        random: bool,
        any_number: bool,
        filter: Option<ObjectFilter>,
        tag: Option<TagKey>,
    },
    DiscardHand,
    PoisonCounters {
        count: Value,
    },
    EnergyCounters {
        count: Value,
    },
    ExperienceCounters {
        count: Value,
    },
    TicketCounters {
        count: Value,
    },
    PayEnergy {
        amount: Value,
    },
    PayAnyEnergy {
        min_amount: u32,
    },
    PayAnyLife {
        min_amount: u32,
    },
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
    SetLifeTotal {
        amount: Value,
    },
    ReverseTurnOrder,
    EndTurn,
    EndCombatPhase,
    SkipTurn,
    SkipCombatPhases,
    SkipNextCombatPhaseThisTurn,
    SkipMainPhasesThisTurn,
    SkipCombatPhasesThisTurn,
    SkipDrawStep,
    AdditionalPhases {
        phases: Vec<crate::effects::AdditionalPhase>,
    },
    PlayFromGraveyardUntilEot,
    ControlPlayer {
        player: PlayerFilter,
        duration: ControlDurationAst,
    },
    ReduceNextSpellCostThisTurn {
        filter: ObjectFilter,
        reduction: ManaCost,
    },
    ReduceMatchingSpellCostThisTurn {
        filter: ObjectFilter,
        reduction: Value,
        duration: Until,
        next_only: bool,
    },
    GrantNextSpellAbilityThisTurn {
        filter: ObjectFilter,
        ability: Box<GrantedAbilityAst>,
    },
    RingTemptsYou,
    VentureIntoDungeon {
        undercity_if_no_active: bool,
    },
    BecomeMonarch,
    TakeInitiative,
    CreateEmblem {
        emblem: EmblemDescriptionAst,
    },
    LoseGame,
    WinGame,
    Detain {
        target: TargetAst,
    },
    Goad {
        target: TargetAst,
        duration: Until,
    },
    Suspect {
        target: TargetAst,
    },
    ClearSuspected {
        target: Option<TargetAst>,
    },
    HealDamage {
        target: TargetAst,
        amount: Option<Value>,
    },
    RemoveFromCombat {
        target: TargetAst,
    },
    Flip {
        target: TargetAst,
    },
    Regenerate {
        target: TargetAst,
        follow_up_effects: Vec<EffectAst>,
    },
    RegenerateAll {
        filter: ObjectFilter,
    },
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

#[derive(Clone, PartialEq)]
pub struct SubjectVerbEffectAst {
    pub subject: SubjectVerbSubjectAst,
    pub action: SubjectVerbActionAst,
}

impl std::fmt::Debug for SubjectVerbRoleAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Actor => "Actor",
            Self::AffectedPlayer => "AffectedPlayer",
            Self::Chooser => "Chooser",
            Self::LibraryOwner => "LibraryOwner",
        };
        f.write_str(label)
    }
}

impl std::fmt::Debug for SubjectVerbSubjectAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubjectVerbSubject")
            .field("role", &self.role)
            .field("player", &self.player)
            .finish()
    }
}

impl std::fmt::Debug for SubjectVerbActionAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draw { count } => f.debug_tuple("Draw").field(count).finish(),
            Self::DrawForEachTaggedMatching { tag, filter } => f
                .debug_struct("DrawForEachTaggedMatching")
                .field("tag", tag)
                .field("filter", filter)
                .finish(),
            Self::LoseLife { amount } => f.debug_tuple("LoseLife").field(amount).finish(),
            Self::PayLife { amount } => f.debug_tuple("PayLife").field(amount).finish(),
            Self::GainLife { amount } => f.debug_tuple("GainLife").field(amount).finish(),
            Self::RevealHand => f.write_str("RevealHand"),
            Self::Mill { count } => f.debug_tuple("Mill").field(count).finish(),
            Self::Scry { count } => f.debug_tuple("Scry").field(count).finish(),
            Self::Surveil { count } => f.debug_tuple("Surveil").field(count).finish(),
            Self::Proliferate { count } => f.debug_tuple("Proliferate").field(count).finish(),
            Self::Investigate { count } => f.debug_tuple("Investigate").field(count).finish(),
            Self::Incubate { amount, count } => f
                .debug_struct("Incubate")
                .field("amount", amount)
                .field("count", count)
                .finish(),
            Self::Learn => f.write_str("Learn"),
            Self::EmitKeywordAction { action, amount } => f
                .debug_struct("EmitKeywordAction")
                .field("action", action)
                .field("amount", amount)
                .finish(),
            Self::ReorderTopPlanarDeck { count } => {
                f.debug_tuple("ReorderTopPlanarDeck").field(count).finish()
            }
            Self::ReturnSourceTransformedFromExile => {
                f.write_str("ReturnSourceTransformedFromExile")
            }
            Self::Reconfigure { target } => f.debug_tuple("Reconfigure").field(target).finish(),
            Self::CumulativeUpkeep { cost } => {
                f.debug_tuple("CumulativeUpkeep").field(cost).finish()
            }
            Self::Casualty { power } => f.debug_tuple("Casualty").field(power).finish(),
            Self::Amass { subtype, amount } => f
                .debug_struct("Amass")
                .field("subtype", subtype)
                .field("amount", amount)
                .finish(),
            Self::Bolster { amount } => f.debug_tuple("Bolster").field(amount).finish(),
            Self::Support { amount } => f.debug_tuple("Support").field(amount).finish(),
            Self::Adapt { amount } => f.debug_tuple("Adapt").field(amount).finish(),
            Self::Monstrosity { amount } => f.debug_tuple("Monstrosity").field(amount).finish(),
            Self::Discover { count } => f.debug_tuple("Discover").field(count).finish(),
            Self::Fateseal { count } => f.debug_tuple("Fateseal").field(count).finish(),
            Self::Populate { count, .. } => f.debug_tuple("Populate").field(count).finish(),
            Self::Explore { target } => f.debug_tuple("Explore").field(target).finish(),
            Self::Endure { target, amount } => f
                .debug_struct("Endure")
                .field("target", target)
                .field("amount", amount)
                .finish(),
            Self::Exploit => f.write_str("Exploit"),
            Self::Connive { target, count } => f
                .debug_struct("Connive")
                .field("target", target)
                .field("count", count)
                .finish(),
            Self::ConniveIterated => f.write_str("ConniveIterated"),
            Self::OpenAttraction { reminder } => f
                .debug_struct("OpenAttraction")
                .field("reminder", reminder)
                .finish(),
            Self::ManifestTopCardOfLibrary => f.write_str("ManifestTopCardOfLibrary"),
            Self::CloakTopCardOfLibrary => f.write_str("CloakTopCardOfLibrary"),
            Self::ManifestCardFromHand => f.write_str("ManifestCardFromHand"),
            Self::ManifestDread => f.write_str("ManifestDread"),
            Self::Earthbend { counters } => f.debug_tuple("Earthbend").field(counters).finish(),
            Self::Behold { subtype, count } => f
                .debug_struct("Behold")
                .field("subtype", subtype)
                .field("count", count)
                .finish(),
            Self::Fight {
                creature1,
                creature2,
            } => f
                .debug_struct("Fight")
                .field("creature1", creature1)
                .field("creature2", creature2)
                .finish(),
            Self::FightIterated { creature2 } => {
                f.debug_tuple("FightIterated").field(creature2).finish()
            }
            Self::Clash { opponent } => f.debug_tuple("Clash").field(opponent).finish(),
            Self::FlipCoin => f.write_str("FlipCoin"),
            Self::FlipCoinFaceOnly => f.write_str("FlipCoinFaceOnly"),
            Self::RollDie { sides, die_text } => {
                if let Some(die_text) = die_text {
                    f.debug_struct("RollDie")
                        .field("sides", sides)
                        .field("die_text", die_text)
                        .finish()
                } else {
                    f.debug_tuple("RollDie").field(sides).finish()
                }
            }
            Self::RollDiceChooseResult {
                count,
                sides,
                die_text,
            } => f
                .debug_struct("RollDiceChooseResult")
                .field("count", count)
                .field("sides", sides)
                .field("die_text", die_text)
                .finish(),
            Self::ShuffleHandAndGraveyardIntoLibrary => {
                f.write_str("ShuffleHandAndGraveyardIntoLibrary")
            }
            Self::ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary => {
                f.write_str("ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary")
            }
            Self::ShuffleGraveyardIntoLibrary {
                explicit_all_cards_from,
            } => f
                .debug_struct("ShuffleGraveyardIntoLibrary")
                .field("explicit_all_cards_from", explicit_all_cards_from)
                .finish(),
            Self::ReorderGraveyard => f.write_str("ReorderGraveyard"),
            Self::ChooseColor => f.write_str("ChooseColor"),
            Self::ChooseCardType { options } => {
                f.debug_tuple("ChooseCardType").field(options).finish()
            }
            Self::ChooseNamedOption { options } => {
                f.debug_tuple("ChooseNamedOption").field(options).finish()
            }
            Self::ChooseCreatureType {
                excluded_subtypes,
                family,
            } => f
                .debug_struct("ChooseCreatureType")
                .field("excluded_subtypes", excluded_subtypes)
                .field("family", family)
                .finish(),
            Self::ChooseLandType { exclude_basic } => f
                .debug_struct("ChooseLandType")
                .field("exclude_basic", exclude_basic)
                .finish(),
            Self::ChooseCardName { filter, tag } => f
                .debug_struct("ChooseCardName")
                .field("filter", filter)
                .field("tag", tag)
                .finish(),
            Self::ChoosePlayer {
                filter,
                tag,
                random,
                exclude_previous_choices,
            } => f
                .debug_struct("ChoosePlayer")
                .field("filter", filter)
                .field("tag", tag)
                .field("random", random)
                .field("exclude_previous_choices", exclude_previous_choices)
                .finish(),
            Self::NoteLifeTotal => f.write_str("NoteLifeTotal"),
            Self::ChooseSpellCastHistory {
                cast_by,
                filter,
                tag,
            } => f
                .debug_struct("ChooseSpellCastHistory")
                .field("cast_by", cast_by)
                .field("filter", filter)
                .field("tag", tag)
                .finish(),
            Self::AddMana { mana } => f.debug_tuple("AddMana").field(mana).finish(),
            Self::AddManaScaled { mana, amount } => f
                .debug_struct("AddManaScaled")
                .field("mana", mana)
                .field("amount", amount)
                .finish(),
            Self::AddManaAnyColor {
                amount,
                available_colors,
                distinct_colors,
            } => f
                .debug_struct("AddManaAnyColor")
                .field("amount", amount)
                .field("available_colors", available_colors)
                .field("distinct_colors", distinct_colors)
                .finish(),
            Self::AddManaAnyOneColor { amount } => {
                f.debug_tuple("AddManaAnyOneColor").field(amount).finish()
            }
            Self::AddManaChosenColor {
                amount,
                fixed_option,
            } => f
                .debug_struct("AddManaChosenColor")
                .field("amount", amount)
                .field("fixed_option", fixed_option)
                .finish(),
            Self::AddManaFromLandCouldProduce {
                amount,
                land_filter,
                allow_colorless,
                same_type,
                mana_type_source,
            } => f
                .debug_struct("AddManaFromLandCouldProduce")
                .field("amount", amount)
                .field("land_filter", land_filter)
                .field("allow_colorless", allow_colorless)
                .field("same_type", same_type)
                .field("mana_type_source", mana_type_source)
                .finish(),
            Self::AddManaColorsAmong { filter } => f
                .debug_struct("AddManaColorsAmong")
                .field("filter", filter)
                .finish(),
            Self::AddOneManaAnyColorAmong {
                filter,
                choose_color_of_object_surface,
            } => f
                .debug_struct("AddOneManaAnyColorAmong")
                .field("filter", filter)
                .field(
                    "choose_color_of_object_surface",
                    choose_color_of_object_surface,
                )
                .finish(),
            Self::AddManaCommanderIdentity { amount } => f
                .debug_tuple("AddManaCommanderIdentity")
                .field(amount)
                .finish(),
            Self::ExchangeLifeTotals { player2 } => {
                f.debug_tuple("ExchangeLifeTotals").field(player2).finish()
            }
            Self::ExchangeTextBoxes { target } => {
                f.debug_tuple("ExchangeTextBoxes").field(target).finish()
            }
            Self::ExchangeZones { zone1, zone2 } => f
                .debug_struct("ExchangeZones")
                .field("zone1", zone1)
                .field("zone2", zone2)
                .finish(),
            Self::PutRestOnBottomOfLibrary => f.write_str("PutRestOnBottomOfLibrary"),
            Self::DontLoseThisManaAsStepsAndPhasesEndThisTurn => {
                f.write_str("DontLoseThisManaAsStepsAndPhasesEndThisTurn")
            }
            Self::ExchangeValues {
                left,
                right,
                duration,
            } => f
                .debug_struct("ExchangeValues")
                .field("left", left)
                .field("right", right)
                .field("duration", duration)
                .finish(),
            Self::ExchangeControl {
                filter,
                count,
                shared_type,
            } => f
                .debug_struct("ExchangeControl")
                .field("filter", filter)
                .field("count", count)
                .field("shared_type", shared_type)
                .finish(),
            Self::ExchangeControlHeterogeneous {
                permanent1,
                permanent2,
                shared_type,
            } => f
                .debug_struct("ExchangeControlHeterogeneous")
                .field("permanent1", permanent1)
                .field("permanent2", permanent2)
                .field("shared_type", shared_type)
                .finish(),
            Self::Attach { object, target } => f
                .debug_struct("Attach")
                .field("object", object)
                .field("target", target)
                .finish(),
            Self::Unattach { object } => {
                f.debug_struct("Unattach").field("object", object).finish()
            }
            Self::Enchant { filter } => f.debug_tuple("Enchant").field(filter).finish(),
            Self::ExileWhenSourceLeaves { target } => f
                .debug_tuple("ExileWhenSourceLeaves")
                .field(target)
                .finish(),
            Self::SacrificeSourceWhenLeaves { target } => f
                .debug_tuple("SacrificeSourceWhenLeaves")
                .field(target)
                .finish(),
            Self::RegisterZoneReplacement {
                target,
                from_zone,
                to_zone,
                replacement_zone,
                library_placement,
                duration,
                optional,
                choice_description,
                counters,
                linked_exile_follow_up,
            } => f
                .debug_struct("RegisterZoneReplacement")
                .field("target", target)
                .field("from_zone", from_zone)
                .field("to_zone", to_zone)
                .field("replacement_zone", replacement_zone)
                .field("library_placement", library_placement)
                .field("duration", duration)
                .field("optional", optional)
                .field("choice_description", choice_description)
                .field("counters", counters)
                .field("linked_exile_follow_up", linked_exile_follow_up)
                .finish(),
            Self::RegisterFutureZoneReplacement {
                filter,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
                cause_policy,
                link_exiled_to_source,
            } => f
                .debug_struct("RegisterFutureZoneReplacement")
                .field("filter", filter)
                .field("from_zone", from_zone)
                .field("to_zone", to_zone)
                .field("replacement_zone", replacement_zone)
                .field("duration", duration)
                .field("cause_policy", cause_policy)
                .field("link_exiled_to_source", link_exiled_to_source)
                .finish(),
            Self::RegisterDrawReplacement {
                player,
                replacement_effects,
                duration,
            } => f
                .debug_struct("RegisterDrawReplacement")
                .field("player", player)
                .field("replacement_effects", replacement_effects)
                .field("duration", duration)
                .finish(),
            Self::RegisterManaReplacement {
                source_filter,
                replacement_mana,
                mode,
            } => f
                .debug_struct("RegisterManaReplacement")
                .field("source_filter", source_filter)
                .field("replacement_mana", replacement_mana)
                .field("mode", mode)
                .finish(),
            Self::RegisterDamagedBySourceZoneReplacement {
                filter,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
            } => f
                .debug_struct("RegisterDamagedBySourceZoneReplacement")
                .field("filter", filter)
                .field("from_zone", from_zone)
                .field("to_zone", to_zone)
                .field("replacement_zone", replacement_zone)
                .field("duration", duration)
                .finish(),
            Self::RegisterEnterUnderControlReplacement { filter, duration } => f
                .debug_struct("RegisterEnterUnderControlReplacement")
                .field("filter", filter)
                .field("duration", duration)
                .finish(),
            Self::RegisterEnterTappedReplacement { filter, duration } => f
                .debug_struct("RegisterEnterTappedReplacement")
                .field("filter", filter)
                .field("duration", duration)
                .finish(),
            Self::RegisterNextBatchEnterWithCounters {
                filter,
                counter_type,
                count,
            } => f
                .debug_struct("RegisterNextBatchEnterWithCounters")
                .field("filter", filter)
                .field("counter_type", counter_type)
                .field("count", count)
                .finish(),
            Self::ExileInsteadOfGraveyardThisTurn => f.write_str("ExileInsteadOfGraveyardThisTurn"),
            Self::ControlCombatChoicesThisTurn {
                attackers,
                blockers,
                this_combat,
            } => f
                .debug_struct("ControlCombatChoicesThisTurn")
                .field("attackers", attackers)
                .field("blockers", blockers)
                .field("this_combat", this_combat)
                .finish(),
            Self::GainControl {
                target,
                duration,
                condition,
                controller_reference,
                source_reference_surface,
            } => f
                .debug_struct("GainControl")
                .field("target", target)
                .field("duration", duration)
                .field("condition", condition)
                .field("controller_reference", controller_reference)
                .field("source_reference_surface", source_reference_surface)
                .finish(),
            Self::RevealTop => f.write_str("RevealTop"),
            Self::ExileTopOfLibrary {
                count,
                surface,
                tags,
                accumulated_tags,
                face_down,
            } => f
                .debug_struct("ExileTopOfLibrary")
                .field("count", count)
                .field("surface", surface)
                .field("tags", tags)
                .field("accumulated_tags", accumulated_tags)
                .field("face_down", face_down)
                .finish(),
            Self::RevealTagged { tag } => f.debug_tuple("RevealTagged").field(tag).finish(),
            Self::PutOntoBattlefield {
                target,
                tapped,
                controller,
                cloak,
                shuffle_before,
            } => f
                .debug_struct("PutOntoBattlefield")
                .field("target", target)
                .field("tapped", tapped)
                .field("controller", controller)
                .field("cloak", cloak)
                .field("shuffle_before", shuffle_before)
                .finish(),
            Self::RevealCardsFromHand {
                count,
                count_value,
                tag,
            } => f
                .debug_struct("RevealCardsFromHand")
                .field("count", count)
                .field("count_value", count_value)
                .field("tag", tag)
                .finish(),
            Self::LookAtTopCards { count, tag, reveal } => f
                .debug_struct("LookAtTopCards")
                .field("count", count)
                .field("tag", tag)
                .field("reveal", reveal)
                .finish(),
            Self::LookAtObjects { filter } => f
                .debug_struct("LookAtObjects")
                .field("filter", filter)
                .finish(),
            Self::LookAtTarget { target } => f.debug_tuple("LookAtTarget").field(target).finish(),
            Self::MayMoveToZone { target, zone } => f
                .debug_struct("MayMoveToZone")
                .field("target", target)
                .field("zone", zone)
                .finish(),
            Self::AdditionalLandPlays { count, duration } => f
                .debug_struct("AdditionalLandPlays")
                .field("count", count)
                .field("duration", duration)
                .finish(),
            Self::ExtraTurnAfterTurn { anchor } => {
                f.debug_tuple("ExtraTurnAfterTurn").field(anchor).finish()
            }
            Self::ReorderTopOfLibrary { tag } => {
                f.debug_tuple("ReorderTopOfLibrary").field(tag).finish()
            }
            Self::AddManaImprintedColors => f.write_str("AddManaImprintedColors"),
            Self::ShuffleLibrary => f.write_str("ShuffleLibrary"),
            Self::ShuffleObjectsIntoLibrary {
                target,
                all,
                owner_library_destination,
                possessive_owner_subject,
            } => f
                .debug_struct("ShuffleObjectsIntoLibrary")
                .field("target", target)
                .field("all", all)
                .field("owner_library_destination", owner_library_destination)
                .field("possessive_owner_subject", possessive_owner_subject)
                .finish(),
            Self::GrantProtectionChoice {
                target,
                chooser,
                allow_colorless,
                allow_artifacts,
                choose_card_type,
            } => f
                .debug_struct("GrantProtectionChoice")
                .field("target", target)
                .field("chooser", chooser)
                .field("allow_colorless", allow_colorless)
                .field("allow_artifacts", allow_artifacts)
                .field("choose_card_type", choose_card_type)
                .finish(),
            Self::PreventAllCombatDamage { duration } => f
                .debug_struct("PreventAllCombatDamage")
                .field("duration", duration)
                .finish(),
            Self::AssignNoCombatDamage { source, duration } => f
                .debug_struct("AssignNoCombatDamage")
                .field("source", source)
                .field("duration", duration)
                .finish(),
            Self::PreventAllCombatDamageFromSource {
                duration,
                source,
                source_would_deal_surface,
            } => f
                .debug_struct("PreventAllCombatDamageFromSource")
                .field("duration", duration)
                .field("source", source)
                .field("source_would_deal_surface", source_would_deal_surface)
                .finish(),
            Self::PreventAllCombatDamageFromSourceFilter {
                duration,
                source_filter,
                excluded_source_target,
            } => f
                .debug_struct("PreventAllCombatDamageFromSourceFilter")
                .field("duration", duration)
                .field("source_filter", source_filter)
                .field("excluded_source_target", excluded_source_target)
                .finish(),
            Self::PreventAllCombatDamageToPlayers { duration } => f
                .debug_struct("PreventAllCombatDamageToPlayers")
                .field("duration", duration)
                .finish(),
            Self::PreventAllCombatDamageToYou { duration } => f
                .debug_struct("PreventAllCombatDamageToYou")
                .field("duration", duration)
                .finish(),
            Self::PreventNextTimeDamage {
                source,
                target,
                reflect_damage_to_source_controller,
                follow_up_effects,
            } => f
                .debug_struct("PreventNextTimeDamage")
                .field("source", source)
                .field("target", target)
                .field(
                    "reflect_damage_to_source_controller",
                    reflect_damage_to_source_controller,
                )
                .field("follow_up_effects", follow_up_effects)
                .finish(),
            Self::ReplaceNextDamageToTarget {
                target,
                damage_target_tag,
                replacement_effects,
            } => f
                .debug_struct("ReplaceNextDamageToTarget")
                .field("target", target)
                .field("damage_target_tag", damage_target_tag)
                .field("replacement_effects", replacement_effects)
                .finish(),
            Self::PreventDamage {
                amount,
                target,
                duration,
                follow_up_effects,
                ..
            } => f
                .debug_struct("PreventDamage")
                .field("amount", amount)
                .field("target", target)
                .field("duration", duration)
                .field("follow_up_effects", follow_up_effects)
                .finish(),
            Self::PreventAllDamageToTarget {
                target,
                duration,
                source_of_your_choice,
                source_choice_shares_activation_mana_color,
                source_target,
            } => f
                .debug_struct("PreventAllDamageToTarget")
                .field("target", target)
                .field("duration", duration)
                .field("source_of_your_choice", source_of_your_choice)
                .field(
                    "source_choice_shares_activation_mana_color",
                    source_choice_shares_activation_mana_color,
                )
                .field("source_target", source_target)
                .finish(),
            Self::PreventAllDamageToTargetFromSourceFilter {
                target,
                duration,
                source_filter,
            } => f
                .debug_struct("PreventAllDamageToTargetFromSourceFilter")
                .field("target", target)
                .field("duration", duration)
                .field("source_filter", source_filter)
                .finish(),
            Self::PreventAllDamageFromSourceFilter {
                duration,
                source_filter,
            } => f
                .debug_struct("PreventAllDamageFromSourceFilter")
                .field("duration", duration)
                .field("source_filter", source_filter)
                .finish(),
            Self::PreventDamageToTargetPutCounters {
                amount,
                target,
                duration,
                counter_type,
            } => f
                .debug_struct("PreventDamageToTargetPutCounters")
                .field("amount", amount)
                .field("target", target)
                .field("duration", duration)
                .field("counter_type", counter_type)
                .finish(),
            Self::PreventDamageEach {
                amount,
                filter,
                duration,
            } => f
                .debug_struct("PreventDamageEach")
                .field("amount", amount)
                .field("filter", filter)
                .field("duration", duration)
                .finish(),
            Self::CopySpell {
                target,
                target_reference_kind,
                target_reference_pronoun,
                all_matches,
                count,
                count_surface,
                player,
                may_choose_new_targets,
                choose_new_target_singular,
                removed_supertypes,
                set_colors,
                added_card_types,
                added_subtypes,
                set_base_power_toughness,
            } => f
                .debug_struct("CopySpell")
                .field("target", target)
                .field("target_reference_kind", target_reference_kind)
                .field("target_reference_pronoun", target_reference_pronoun)
                .field("all_matches", all_matches)
                .field("count", count)
                .field("count_surface", count_surface)
                .field("player", player)
                .field("may_choose_new_targets", may_choose_new_targets)
                .field("choose_new_target_singular", choose_new_target_singular)
                .field("removed_supertypes", removed_supertypes)
                .field("set_colors", set_colors)
                .field("added_card_types", added_card_types)
                .field("added_subtypes", added_subtypes)
                .field("set_base_power_toughness", set_base_power_toughness)
                .finish(),
            Self::CopySpellForEachTarget {
                target,
                object_filter,
                player_filter,
                player,
                exclude_current_targets,
                removed_supertypes,
            } => f
                .debug_struct("CopySpellForEachTarget")
                .field("target", target)
                .field("object_filter", object_filter)
                .field("player_filter", player_filter)
                .field("player", player)
                .field("exclude_current_targets", exclude_current_targets)
                .field("removed_supertypes", removed_supertypes)
                .finish(),
            Self::ScaleXValue { target, multiplier } => f
                .debug_struct("ScaleXValue")
                .field("target", target)
                .field("multiplier", multiplier)
                .finish(),
            Self::PutTaggedRemainderOnBottomOfLibrary {
                tag,
                keep_tagged,
                order,
                player,
                surface,
            } => f
                .debug_struct("PutTaggedRemainderOnBottomOfLibrary")
                .field("tag", tag)
                .field("keep_tagged", keep_tagged)
                .field("order", order)
                .field("player", player)
                .field("surface", surface)
                .finish(),
            Self::PutTaggedRemainderInZone {
                tag,
                keep_tagged,
                zone,
                surface,
            } => f
                .debug_struct("PutTaggedRemainderInZone")
                .field("tag", tag)
                .field("keep_tagged", keep_tagged)
                .field("zone", zone)
                .field("surface", surface)
                .finish(),
            Self::CastTagged {
                tag,
                player,
                allow_land,
                as_copy,
                copy_cast_reminder_surface,
                without_paying_mana_cost,
                additional_mana_cost,
                cost_reduction,
                mana_spend_mode,
            } => f
                .debug_struct("CastTagged")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("as_copy", as_copy)
                .field("copy_cast_reminder_surface", copy_cast_reminder_surface)
                .field("without_paying_mana_cost", without_paying_mana_cost)
                .field("additional_mana_cost", additional_mana_cost)
                .field("cost_reduction", cost_reduction)
                .field("mana_spend_mode", mana_spend_mode)
                .finish(),
            Self::GrantPlayTaggedUntilEndOfTurn {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                while_on_top_of_library,
                free_cast_from_current_zone,
                until_source_exiles_another,
                max_plays,
                surface,
            } => f
                .debug_struct("GrantPlayTaggedUntilEndOfTurn")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("without_paying_mana_cost", without_paying_mana_cost)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
                .field("while_on_top_of_library", while_on_top_of_library)
                .field("free_cast_from_current_zone", free_cast_from_current_zone)
                .field("until_source_exiles_another", until_source_exiles_another)
                .field("max_plays", max_plays)
                .field("surface", surface)
                .finish(),
            Self::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                tag,
                player,
            } => f
                .debug_struct("GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn")
                .field("tag", tag)
                .field("player", player)
                .finish(),
            Self::GrantPlayTaggedUntilYourNextTurn {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
                until_next_end_step,
                max_plays,
            } => f
                .debug_struct("GrantPlayTaggedUntilYourNextTurn")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
                .field("until_next_end_step", until_next_end_step)
                .field("max_plays", max_plays)
                .finish(),
            Self::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                filter,
                during_turns_counter_put_on_source,
                spell_cost_increase,
                lands_enter_tapped,
            } => f
                .debug_struct("GrantPlayTaggedForAsLongAsExiled")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("without_paying_mana_cost", without_paying_mana_cost)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
                .field("filter", filter)
                .field(
                    "during_turns_counter_put_on_source",
                    during_turns_counter_put_on_source,
                )
                .field("spell_cost_increase", spell_cost_increase)
                .field("lands_enter_tapped", lands_enter_tapped)
                .finish(),
            Self::GrantPlayTaggedForAsLongAsYouControlSource {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
                surface,
            } => f
                .debug_struct("GrantPlayTaggedForAsLongAsYouControlSource")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
                .field("surface", surface)
                .finish(),
            Self::ReturnToBattlefield {
                target,
                from_graveyard_or_exile,
                tapped,
                transformed,
                converted,
                controller,
                count_value,
                as_aura,
                top_only,
            } => f
                .debug_struct("ReturnToBattlefield")
                .field("target", target)
                .field("from_graveyard_or_exile", from_graveyard_or_exile)
                .field("tapped", tapped)
                .field("transformed", transformed)
                .field("converted", converted)
                .field("controller", controller)
                .field("count_value", count_value)
                .field("as_aura", as_aura)
                .field("top_only", top_only)
                .finish(),
            Self::ReturnAllToBattlefield {
                filter,
                tapped,
                face_down,
                controller,
                verb_surface,
            } => f
                .debug_struct("ReturnAllToBattlefield")
                .field("filter", filter)
                .field("tapped", tapped)
                .field("face_down", face_down)
                .field("controller", controller)
                .field("verb_surface", verb_surface)
                .finish(),
            Self::ExileUntilSourceLeaves {
                target,
                duration,
                leave_watcher,
                face_down,
                all,
                explicit_return_surface,
            } => f
                .debug_struct("ExileUntilSourceLeaves")
                .field("target", target)
                .field("duration", duration)
                .field("leave_watcher", leave_watcher)
                .field("face_down", face_down)
                .field("all", all)
                .field("explicit_return_surface", explicit_return_surface)
                .finish(),
            Self::MoveToZone {
                target,
                source_top_only,
                zone,
                to_top,
                library_order,
                library_order_chooser,
                verb_surface,
                target_plural_surface,
                target_reference_surface,
                destination_player_surface,
                destination_player_reference_surface,
                exiled_with_source_surface,
                battlefield_controller,
                battlefield_tapped,
                battlefield_attacking,
                battlefield_attack_target_player_or_planeswalker_controlled_by,
                battlefield_face_down,
                battlefield_transformed,
                attached_to,
                all,
            } => f
                .debug_struct("MoveToZone")
                .field("target", target)
                .field("source_top_only", source_top_only)
                .field("zone", zone)
                .field("to_top", to_top)
                .field("library_order", library_order)
                .field("library_order_chooser", library_order_chooser)
                .field("verb_surface", verb_surface)
                .field("target_plural_surface", target_plural_surface)
                .field("target_reference_surface", target_reference_surface)
                .field("destination_player_surface", destination_player_surface)
                .field(
                    "destination_player_reference_surface",
                    destination_player_reference_surface,
                )
                .field("exiled_with_source_surface", exiled_with_source_surface)
                .field("battlefield_controller", battlefield_controller)
                .field("battlefield_tapped", battlefield_tapped)
                .field("battlefield_attacking", battlefield_attacking)
                .field(
                    "battlefield_attack_target_player_or_planeswalker_controlled_by",
                    battlefield_attack_target_player_or_planeswalker_controlled_by,
                )
                .field("battlefield_face_down", battlefield_face_down)
                .field("battlefield_transformed", battlefield_transformed)
                .field("attached_to", attached_to)
                .field("all", all)
                .finish(),
            Self::MoveToLibraryTopOrBottomChoice { target } => f
                .debug_struct("MoveToLibraryTopOrBottomChoice")
                .field("target", target)
                .finish(),
            Self::TargetOnly {
                target,
                explicit_declaration,
            } => f
                .debug_struct("TargetOnly")
                .field("target", target)
                .field("explicit_declaration", explicit_declaration)
                .finish(),
            Self::TagMatchingObjects {
                filter,
                zones,
                tag,
                source_tags,
            } => {
                let mut debug = f.debug_struct("TagMatchingObjects");
                debug
                    .field("filter", filter)
                    .field("zones", zones)
                    .field("tag", tag);
                if !source_tags.is_empty() {
                    debug.field("source_tags", source_tags);
                }
                debug.finish()
            }
            Self::Pump {
                power,
                toughness,
                target,
                duration,
                condition,
                set_quantifier_surface,
            } => f
                .debug_struct("Pump")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("duration", duration)
                .field("condition", condition)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::SetBasePowerToughness {
                power,
                toughness,
                target,
                duration,
                set_quantifier_surface,
            } => f
                .debug_struct("SetBasePowerToughness")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("duration", duration)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::BecomeBasePtCreature {
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
                set_quantifier_surface,
                duration,
            } => f
                .debug_struct("BecomeBasePtCreature")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("card_types", card_types)
                .field("subtypes", subtypes)
                .field("subtype_families", subtype_families)
                .field("colors", colors)
                .field("abilities", abilities)
                .field("granted_abilities", granted_abilities)
                .field("preserve_other_types", preserve_other_types)
                .field("type_retention_surface", type_retention_surface)
                .field("animation_pt_surface", animation_pt_surface)
                .field("animation_duration_surface", animation_duration_surface)
                .field("set_quantifier_surface", set_quantifier_surface)
                .field("duration", duration)
                .finish(),
            Self::SetBasePower {
                power,
                target,
                duration,
            } => f
                .debug_struct("SetBasePower")
                .field("power", power)
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::PumpForEach {
                power_per,
                toughness_per,
                target,
                count,
                duration,
            } => f
                .debug_struct("PumpForEach")
                .field("power_per", power_per)
                .field("toughness_per", toughness_per)
                .field("target", target)
                .field("count", count)
                .field("duration", duration)
                .finish(),
            Self::PumpAll {
                filter,
                power,
                toughness,
                duration,
                set_quantifier_surface,
            } => f
                .debug_struct("PumpAll")
                .field("filter", filter)
                .field("power", power)
                .field("toughness", toughness)
                .field("duration", duration)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::PumpByLastEffect {
                power,
                toughness,
                target,
                duration,
                includes_this_way,
            } => f
                .debug_struct("PumpByLastEffect")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("duration", duration)
                .field("includes_this_way", includes_this_way)
                .finish(),
            Self::AddCardTypes {
                target,
                card_types,
                duration,
            } => f
                .debug_struct("AddCardTypes")
                .field("target", target)
                .field("card_types", card_types)
                .field("duration", duration)
                .finish(),
            Self::SetCardTypes {
                target,
                card_types,
                duration,
            } => f
                .debug_struct("SetCardTypes")
                .field("target", target)
                .field("card_types", card_types)
                .field("duration", duration)
                .finish(),
            Self::RemoveCardTypes {
                target,
                card_types,
                duration,
            } => f
                .debug_struct("RemoveCardTypes")
                .field("target", target)
                .field("card_types", card_types)
                .field("duration", duration)
                .finish(),
            Self::AddSubtypes {
                target,
                subtypes,
                duration,
            } => f
                .debug_struct("AddSubtypes")
                .field("target", target)
                .field("subtypes", subtypes)
                .field("duration", duration)
                .finish(),
            Self::RemoveSubtypes {
                target,
                subtypes,
                duration,
            } => f
                .debug_struct("RemoveSubtypes")
                .field("target", target)
                .field("subtypes", subtypes)
                .field("duration", duration)
                .finish(),
            Self::SetCreatureSubtypes {
                target,
                subtypes,
                duration,
            } => f
                .debug_struct("SetCreatureSubtypes")
                .field("target", target)
                .field("subtypes", subtypes)
                .field("duration", duration)
                .finish(),
            Self::BecomeSaddledUntilEndOfTurn { target } => f
                .debug_struct("BecomeSaddledUntilEndOfTurn")
                .field("target", target)
                .finish(),
            Self::AddColors {
                target,
                colors,
                duration,
            } => f
                .debug_struct("AddColors")
                .field("target", target)
                .field("colors", colors)
                .field("duration", duration)
                .finish(),
            Self::AddAllSubtypesOfFamily {
                target,
                family,
                duration,
            } => f
                .debug_struct("AddAllSubtypesOfFamily")
                .field("target", target)
                .field("family", family)
                .field("duration", duration)
                .finish(),
            Self::RemoveAllSubtypesOfFamily {
                target,
                family,
                duration,
            } => f
                .debug_struct("RemoveAllSubtypesOfFamily")
                .field("target", target)
                .field("family", family)
                .field("duration", duration)
                .finish(),
            Self::BecomeAuraEnchantment {
                target,
                attachment_filter,
                granted_abilities,
                duration,
            } => f
                .debug_struct("BecomeAuraEnchantment")
                .field("target", target)
                .field("attachment_filter", attachment_filter)
                .field("granted_abilities", granted_abilities)
                .field("duration", duration)
                .finish(),
            Self::BecomeBasicLandType {
                target,
                subtype,
                duration,
            } => f
                .debug_struct("BecomeBasicLandType")
                .field("target", target)
                .field("subtype", subtype)
                .field("duration", duration)
                .finish(),
            Self::SetColors {
                target,
                colors,
                duration,
            } => f
                .debug_struct("SetColors")
                .field("target", target)
                .field("colors", colors)
                .field("duration", duration)
                .finish(),
            Self::MakeColorless { target, duration } => f
                .debug_struct("MakeColorless")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::BecomeBasicLandTypeChoice { target, duration } => f
                .debug_struct("BecomeBasicLandTypeChoice")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::BecomeCreatureTypeChoice {
                target,
                duration,
                excluded_subtypes,
            } => f
                .debug_struct("BecomeCreatureTypeChoice")
                .field("target", target)
                .field("duration", duration)
                .field("excluded_subtypes", excluded_subtypes)
                .finish(),
            Self::BecomeColorChoice {
                target,
                duration,
                allow_multiple,
            } => f
                .debug_struct("BecomeColorChoice")
                .field("target", target)
                .field("duration", duration)
                .field("allow_multiple", allow_multiple)
                .finish(),
            Self::BecomeCopy {
                target,
                source,
                duration,
                preserve_source_abilities,
                name_override,
                name_override_surface,
                add_supertypes,
                remove_supertypes,
                add_card_types,
                set_card_types,
                add_subtypes,
                set_subtypes,
                granted_abilities,
                set_base_power_toughness,
                copy_exception_surface,
            } => f
                .debug_struct("BecomeCopy")
                .field("target", target)
                .field("source", source)
                .field("duration", duration)
                .field("preserve_source_abilities", preserve_source_abilities)
                .field("name_override", name_override)
                .field("name_override_surface", name_override_surface)
                .field("add_supertypes", add_supertypes)
                .field("remove_supertypes", remove_supertypes)
                .field("add_card_types", add_card_types)
                .field("set_card_types", set_card_types)
                .field("add_subtypes", add_subtypes)
                .field("set_subtypes", set_subtypes)
                .field("granted_abilities", granted_abilities)
                .field("set_base_power_toughness", set_base_power_toughness)
                .field("copy_exception_surface", copy_exception_surface)
                .finish(),
            Self::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
                condition,
                set_quantifier_surface,
                lock_filter_at_resolution,
            } => f
                .debug_struct("GrantAbilitiesAll")
                .field("filter", filter)
                .field("abilities", abilities)
                .field("duration", duration)
                .field("condition", condition)
                .field("set_quantifier_surface", set_quantifier_surface)
                .field("lock_filter_at_resolution", lock_filter_at_resolution)
                .finish(),
            Self::RemoveAbilitiesAll {
                filter,
                abilities,
                duration,
                condition,
                set_quantifier_surface,
            } => f
                .debug_struct("RemoveAbilitiesAll")
                .field("filter", filter)
                .field("abilities", abilities)
                .field("duration", duration)
                .field("condition", condition)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::GrantAbilitiesChoiceAll {
                filter,
                abilities,
                duration,
            } => f
                .debug_struct("GrantAbilitiesChoiceAll")
                .field("filter", filter)
                .field("abilities", abilities)
                .field("duration", duration)
                .finish(),
            Self::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
                condition,
                set_quantifier_surface,
            } => f
                .debug_struct("GrantAbilitiesToTarget")
                .field("target", target)
                .field("abilities", abilities)
                .field("duration", duration)
                .field("condition", condition)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::GrantToTarget {
                target,
                grantable,
                duration,
            } => f
                .debug_struct("GrantToTarget")
                .field("target", target)
                .field("grantable", grantable)
                .field("duration", duration)
                .finish(),
            Self::GrantBySpec {
                spec,
                player,
                duration,
            } => f
                .debug_struct("GrantBySpec")
                .field("spec", spec)
                .field("player", player)
                .field("duration", duration)
                .finish(),
            Self::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            } => f
                .debug_struct("RemoveAbilitiesFromTarget")
                .field("target", target)
                .field("abilities", abilities)
                .field("duration", duration)
                .finish(),
            Self::GrantAbilitiesChoiceToTarget {
                target,
                abilities,
                duration,
            } => f
                .debug_struct("GrantAbilitiesChoiceToTarget")
                .field("target", target)
                .field("abilities", abilities)
                .field("duration", duration)
                .finish(),
            Self::ConsultTopOfLibrary {
                player,
                mode,
                filter,
                stop_rule,
                max_exposed,
                all_tag,
                match_tag,
            } => f
                .debug_struct("ConsultTopOfLibrary")
                .field("player", player)
                .field("mode", mode)
                .field("filter", filter)
                .field("stop_rule", stop_rule)
                .field("max_exposed", max_exposed)
                .field("all_tag", all_tag)
                .field("match_tag", match_tag)
                .finish(),
            Self::SearchLibrary {
                filter,
                search_zones,
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
                enters_with_counters,
                enters_under_your_control,
            } => f
                .debug_struct("SearchLibrary")
                .field("enters_under_your_control", enters_under_your_control)
                .field("filter", filter)
                .field("search_zones", search_zones)
                .field("destination", destination)
                .field("chooser", chooser)
                .field("player", player)
                .field("search_mode", search_mode)
                .field("reveal", reveal)
                .field("reveal_reference_surface", reveal_reference_surface)
                .field("shuffle", shuffle)
                .field("count", count)
                .field("count_value", count_value)
                .field("library_position_from_top", library_position_from_top)
                .field("result_reference_surface", result_reference_surface)
                .field(
                    "search_top_in_any_order_surface",
                    search_top_in_any_order_surface,
                )
                .field("tapped", tapped)
                .field("enters_with_counters", enters_with_counters)
                .finish(),
            Self::Cant {
                restriction,
                duration,
                start,
                duration_surface,
                condition,
            } => f
                .debug_struct("Cant")
                .field("restriction", restriction)
                .field("duration", duration)
                .field("start", start)
                .field("duration_surface", duration_surface)
                .field("condition", condition)
                .finish(),
            Self::CreateTokenCopy { .. } => f.write_str("CreateTokenCopy"),
            Self::CreateTokenCopyFromSource { .. } => f.write_str("CreateTokenCopyFromSource"),
            Self::CreateTokenWithMods {
                name,
                count,
                player,
                ..
            } => f
                .debug_struct("CreateTokenWithMods")
                .field("name", name)
                .field("count", count)
                .field("player", player)
                .finish(),
            Self::CreateTokenChoice { options } => {
                let mut builder = f.debug_struct("CreateTokenChoice");
                for (display, _) in options {
                    builder.field("option", display);
                }
                builder.finish()
            }
            Self::RedirectNextDamageFromSourceToTarget {
                amount,
                protected_target,
                destination,
                destination_target,
            } => f
                .debug_struct("RedirectNextDamageFromSourceToTarget")
                .field("amount", amount)
                .field("protected_target", protected_target)
                .field("destination", destination)
                .field("destination_target", destination_target)
                .finish(),
            Self::RedirectNextTimeDamageToSource {
                source,
                target,
                destination,
                destination_target,
                all_this_turn,
            } => f
                .debug_struct("RedirectNextTimeDamageToSource")
                .field("source", source)
                .field("target", target)
                .field("destination", destination)
                .field("destination_target", destination_target)
                .field("all_this_turn", all_this_turn)
                .finish(),
            Self::RedirectAllDamageThisTurnBySourceToSourceController { source } => f
                .debug_struct("RedirectAllDamageThisTurnBySourceToSourceController")
                .field("source", source)
                .finish(),
            Self::RedirectAllDamageThisTurnToTarget {
                player_filter,
                object_filter,
                target,
            } => f
                .debug_struct("RedirectAllDamageThisTurnToTarget")
                .field("player_filter", player_filter)
                .field("object_filter", object_filter)
                .field("target", target)
                .finish(),
            Self::Meld {
                result_name,
                enters_tapped,
                enters_attacking,
            } => f
                .debug_struct("Meld")
                .field("result_name", result_name)
                .field("enters_tapped", enters_tapped)
                .field("enters_attacking", enters_attacking)
                .finish(),
            Self::SearchLibrarySlotsToHand {
                slots,
                destination,
                reveal,
                progress_tag,
            } => f
                .debug_struct("SearchLibrarySlotsToHand")
                .field("slots", slots)
                .field("destination", destination)
                .field("reveal", reveal)
                .field("progress_tag", progress_tag)
                .finish(),
            Self::RetargetStackObject {
                target,
                mode,
                require_change,
                copy_reference_plural,
            } => f
                .debug_struct("RetargetStackObject")
                .field("target", target)
                .field("mode", mode)
                .field("require_change", require_change)
                .field("copy_reference_plural", copy_reference_plural)
                .finish(),
            Self::GrantAbilityToSource { ability, duration } => f
                .debug_struct("GrantAbilityToSource")
                .field("ability", ability)
                .field("duration", duration)
                .finish(),
            Self::TurnFaceUp { target } => f
                .debug_struct("TurnFaceUp")
                .field("target", target)
                .finish(),
            Self::DealDamage { amount, target, .. } => f
                .debug_struct("DealDamage")
                .field("amount", amount)
                .field("target", target)
                .finish(),
            Self::DealDamageEach { amount, filter } => f
                .debug_struct("DealDamageEach")
                .field("amount", amount)
                .field("filter", filter)
                .finish(),
            Self::DealDamageEqualToPower {
                source,
                amount,
                target,
                unpreventable,
            } => f
                .debug_struct("DealDamageEqualToPower")
                .field("source", source)
                .field("amount", amount)
                .field("target", target)
                .field("unpreventable", unpreventable)
                .finish(),
            Self::DealDistributedDamage {
                amount,
                target,
                source,
                chooser,
                distribution,
            } => f
                .debug_struct("DealDistributedDamage")
                .field("amount", amount)
                .field("target", target)
                .field("source", source)
                .field("chooser", chooser)
                .field("distribution", distribution)
                .finish(),
            Self::Tap { target } => f.debug_tuple("Tap").field(target).finish(),
            Self::Untap { target } => f.debug_tuple("Untap").field(target).finish(),
            Self::TapAll { filter } => f.debug_tuple("TapAll").field(filter).finish(),
            Self::UntapAll { filter } => f.debug_tuple("UntapAll").field(filter).finish(),
            Self::TapOrUntap { target } => f.debug_tuple("TapOrUntap").field(target).finish(),
            Self::TapOrUntapAll {
                tap_filter,
                untap_filter,
            } => f
                .debug_struct("TapOrUntapAll")
                .field("tap_filter", tap_filter)
                .field("untap_filter", untap_filter)
                .finish(),
            Self::PhaseOut {
                target,
                duration,
                source_surface,
            } => f
                .debug_struct("PhaseOut")
                .field("target", target)
                .field("duration", duration)
                .field("source_surface", source_surface)
                .finish(),
            Self::PhaseOutAll {
                filter,
                duration,
                source_surface,
            } => f
                .debug_struct("PhaseOutAll")
                .field("filter", filter)
                .field("duration", duration)
                .field("source_surface", source_surface)
                .finish(),
            Self::PhaseIn { target } => f.debug_tuple("PhaseIn").field(target).finish(),
            Self::PhaseInAll { filter } => f.debug_tuple("PhaseInAll").field(filter).finish(),
            Self::Transform { target } => f.debug_tuple("Transform").field(target).finish(),
            Self::Convert { target } => f.debug_tuple("Convert").field(target).finish(),
            Self::Destroy {
                target,
                no_regeneration,
                creature_destroyed_this_way_surface,
            } => f
                .debug_struct("Destroy")
                .field("target", target)
                .field("no_regeneration", no_regeneration)
                .field(
                    "creature_destroyed_this_way_surface",
                    creature_destroyed_this_way_surface,
                )
                .finish(),
            Self::DestroyAll {
                filter,
                no_regeneration,
                creature_destroyed_this_way_surface,
            } => f
                .debug_struct("DestroyAll")
                .field("filter", filter)
                .field("no_regeneration", no_regeneration)
                .field(
                    "creature_destroyed_this_way_surface",
                    creature_destroyed_this_way_surface,
                )
                .finish(),
            Self::DestroyAllOfChosenColor {
                filter,
                no_regeneration,
                creature_destroyed_this_way_surface,
            } => f
                .debug_struct("DestroyAllOfChosenColor")
                .field("filter", filter)
                .field("no_regeneration", no_regeneration)
                .field(
                    "creature_destroyed_this_way_surface",
                    creature_destroyed_this_way_surface,
                )
                .finish(),
            Self::DestroyAllAttachedTo { filter, target } => f
                .debug_struct("DestroyAllAttachedTo")
                .field("filter", filter)
                .field("target", target)
                .finish(),
            Self::ExileAllAttachedTo {
                filter,
                target,
                face_down,
            } => f
                .debug_struct("ExileAllAttachedTo")
                .field("filter", filter)
                .field("target", target)
                .field("face_down", face_down)
                .finish(),
            Self::Exile {
                target,
                face_down,
                source_top_only,
                target_plural_surface,
            } => f
                .debug_struct("Exile")
                .field("target", target)
                .field("face_down", face_down)
                .field("source_top_only", source_top_only)
                .field("target_plural_surface", target_plural_surface)
                .finish(),
            Self::ExileAll { filter, face_down } => f
                .debug_struct("ExileAll")
                .field("filter", filter)
                .field("face_down", face_down)
                .finish(),
            Self::LookAtHand { target } => f.debug_tuple("LookAtHand").field(target).finish(),
            Self::Counter { target } => f.debug_tuple("Counter").field(target).finish(),
            Self::CounterUnlessPays { target, cost } => f
                .debug_struct("CounterUnlessPays")
                .field("target", target)
                .field("cost", cost)
                .finish(),
            Self::PutCounters {
                counter_type,
                count,
                target,
                target_count,
                distributed,
            } => f
                .debug_struct("PutCounters")
                .field("counter_type", counter_type)
                .field("count", count)
                .field("target", target)
                .field("target_count", target_count)
                .field("distributed", distributed)
                .finish(),
            Self::PutCounterChoice {
                counter_types,
                count,
                mode_texts,
                target,
                target_count,
            } => f
                .debug_struct("PutCounterChoice")
                .field("counter_types", counter_types)
                .field("count", count)
                .field("mode_texts", mode_texts)
                .field("target", target)
                .field("target_count", target_count)
                .finish(),
            Self::PutOrRemoveCounters {
                put_counter_type,
                put_count,
                remove_counter_type,
                remove_count,
                put_mode_text,
                remove_mode_text,
                target,
                target_count,
            } => f
                .debug_struct("PutOrRemoveCounters")
                .field("put_counter_type", put_counter_type)
                .field("put_count", put_count)
                .field("remove_counter_type", remove_counter_type)
                .field("remove_count", remove_count)
                .field("put_mode_text", put_mode_text)
                .field("remove_mode_text", remove_mode_text)
                .field("target", target)
                .field("target_count", target_count)
                .finish(),
            Self::PutCountersAll {
                counter_type,
                count,
                filter,
            } => f
                .debug_struct("PutCountersAll")
                .field("counter_type", counter_type)
                .field("count", count)
                .field("filter", filter)
                .finish(),
            Self::RemoveUpToAnyCounters {
                amount,
                target,
                counter_type,
                up_to,
                distributed_across_all,
                all_of_them,
            } => f
                .debug_struct("RemoveUpToAnyCounters")
                .field("amount", amount)
                .field("target", target)
                .field("counter_type", counter_type)
                .field("up_to", up_to)
                .field("distributed_across_all", distributed_across_all)
                .field("all_of_them", all_of_them)
                .finish(),
            Self::MoveAllCounters { from, to } => f
                .debug_struct("MoveAllCounters")
                .field("from", from)
                .field("to", to)
                .finish(),
            Self::MoveOneCounter { from, to } => f
                .debug_struct("MoveOneCounter")
                .field("from", from)
                .field("to", to)
                .finish(),
            Self::ForEachCounterKindPutOrRemove {
                target,
                all_kinds,
                fixed_counter_type,
                optional_action,
            } => f
                .debug_struct("ForEachCounterKindPutOrRemove")
                .field("target", target)
                .field("all_kinds", all_kinds)
                .field("fixed_counter_type", fixed_counter_type)
                .field("optional_action", optional_action)
                .finish(),
            Self::PutCounterOfChosenKind { target } => f
                .debug_struct("PutCounterOfChosenKind")
                .field("target", target)
                .finish(),
            Self::ReturnToHand {
                target,
                random,
                destination_player_surface,
                exiled_with_source_surface,
                set_quantifier_surface,
                set_reference_surface,
            } => f
                .debug_struct("ReturnToHand")
                .field("target", target)
                .field("random", random)
                .field("destination_player_surface", destination_player_surface)
                .field("exiled_with_source_surface", exiled_with_source_surface)
                .field("set_quantifier_surface", set_quantifier_surface)
                .field("set_reference_surface", set_reference_surface)
                .finish(),
            Self::ReturnAllToHand {
                filter,
                destination_player_surface,
                exiled_with_source_surface,
            } => f
                .debug_struct("ReturnAllToHand")
                .field("filter", filter)
                .field("destination_player_surface", destination_player_surface)
                .field("exiled_with_source_surface", exiled_with_source_surface)
                .finish(),
            Self::ReturnAllToHandOfChosenColor { filter } => f
                .debug_struct("ReturnAllToHandOfChosenColor")
                .field("filter", filter)
                .finish(),
            Self::MoveToLibraryNthFromTop { target, position } => f
                .debug_struct("MoveToLibraryNthFromTop")
                .field("target", target)
                .field("position", position)
                .finish(),
            Self::DoubleCountersOnEach {
                counter_type,
                filter,
            } => f
                .debug_struct("DoubleCountersOnEach")
                .field("counter_type", counter_type)
                .field("filter", filter)
                .finish(),
            Self::DoubleCountersOnTarget {
                counter_type,
                target,
            } => f
                .debug_struct("DoubleCountersOnTarget")
                .field("counter_type", counter_type)
                .field("target", target)
                .finish(),
            Self::RemoveCountersAll {
                amount,
                filter,
                counter_type,
                up_to,
            } => f
                .debug_struct("RemoveCountersAll")
                .field("amount", amount)
                .field("filter", filter)
                .field("counter_type", counter_type)
                .field("up_to", up_to)
                .finish(),
            Self::PutSticker { target, action } => f
                .debug_struct("PutSticker")
                .field("target", target)
                .field("action", action)
                .finish(),
            Self::UnlockRoomDoor => f.write_str("UnlockRoomDoor"),
            Self::SwitchPowerToughness { target, duration } => f
                .debug_struct("SwitchPowerToughness")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::ScalePowerToughnessAll {
                filter,
                power,
                toughness,
                multiplier,
                duration,
            } => f
                .debug_struct("ScalePowerToughnessAll")
                .field("filter", filter)
                .field("power", power)
                .field("toughness", toughness)
                .field("multiplier", multiplier)
                .field("duration", duration)
                .finish(),
            Self::Discard {
                count,
                random,
                any_number,
                filter,
                tag,
            } => f
                .debug_struct("Discard")
                .field("count", count)
                .field("random", random)
                .field("any_number", any_number)
                .field("filter", filter)
                .field("tag", tag)
                .finish(),
            Self::DiscardHand => f.write_str("DiscardHand"),
            Self::PoisonCounters { count } => f.debug_tuple("PoisonCounters").field(count).finish(),
            Self::EnergyCounters { count } => f.debug_tuple("EnergyCounters").field(count).finish(),
            Self::ExperienceCounters { count } => {
                f.debug_tuple("ExperienceCounters").field(count).finish()
            }
            Self::TicketCounters { count } => f.debug_tuple("TicketCounters").field(count).finish(),
            Self::PayEnergy { amount } => f.debug_tuple("PayEnergy").field(amount).finish(),
            Self::PayAnyEnergy { min_amount } => f
                .debug_struct("PayAnyEnergy")
                .field("min_amount", min_amount)
                .finish(),
            Self::PayAnyLife { min_amount } => f
                .debug_struct("PayAnyLife")
                .field("min_amount", min_amount)
                .finish(),
            Self::PayMana {
                cost,
                x_value,
                x_maximum,
            } => f
                .debug_struct("PayMana")
                .field("cost", cost)
                .field("x_value", x_value)
                .field("x_maximum", x_maximum)
                .finish(),
            Self::DoubleManaPool => f.write_str("DoubleManaPool"),
            Self::EmptyManaPool => f.write_str("EmptyManaPool"),
            Self::SetLifeTotal { amount } => f.debug_tuple("SetLifeTotal").field(amount).finish(),
            Self::ReverseTurnOrder => f.write_str("ReverseTurnOrder"),
            Self::EndTurn => f.write_str("EndTurn"),
            Self::EndCombatPhase => f.write_str("EndCombatPhase"),
            Self::SkipTurn => f.write_str("SkipTurn"),
            Self::SkipCombatPhases => f.write_str("SkipCombatPhases"),
            Self::SkipNextCombatPhaseThisTurn => f.write_str("SkipNextCombatPhaseThisTurn"),
            Self::SkipMainPhasesThisTurn => f.write_str("SkipMainPhasesThisTurn"),
            Self::SkipCombatPhasesThisTurn => f.write_str("SkipCombatPhasesThisTurn"),
            Self::SkipDrawStep => f.write_str("SkipDrawStep"),
            Self::AdditionalPhases { phases } => {
                f.debug_tuple("AdditionalPhases").field(phases).finish()
            }
            Self::PlayFromGraveyardUntilEot => f.write_str("PlayFromGraveyardUntilEot"),
            Self::ControlPlayer { player, duration } => f
                .debug_struct("ControlPlayer")
                .field("player", player)
                .field("duration", duration)
                .finish(),
            Self::ReduceNextSpellCostThisTurn { filter, reduction } => f
                .debug_struct("ReduceNextSpellCostThisTurn")
                .field("filter", filter)
                .field("reduction", reduction)
                .finish(),
            Self::ReduceMatchingSpellCostThisTurn {
                filter,
                reduction,
                duration,
                next_only,
            } => f
                .debug_struct("ReduceMatchingSpellCostThisTurn")
                .field("filter", filter)
                .field("reduction", reduction)
                .field("duration", duration)
                .field("next_only", next_only)
                .finish(),
            Self::GrantNextSpellAbilityThisTurn { filter, ability } => f
                .debug_struct("GrantNextSpellAbilityThisTurn")
                .field("filter", filter)
                .field("ability", ability)
                .finish(),
            Self::RingTemptsYou => f.write_str("RingTemptsYou"),
            Self::VentureIntoDungeon {
                undercity_if_no_active,
            } => f
                .debug_struct("VentureIntoDungeon")
                .field("undercity_if_no_active", undercity_if_no_active)
                .finish(),
            Self::BecomeMonarch => f.write_str("BecomeMonarch"),
            Self::TakeInitiative => f.write_str("TakeInitiative"),
            Self::CreateEmblem { emblem } => f.debug_tuple("CreateEmblem").field(emblem).finish(),
            Self::LoseGame => f.write_str("LoseGame"),
            Self::WinGame => f.write_str("WinGame"),
            Self::Detain { target } => f.debug_tuple("Detain").field(target).finish(),
            Self::Goad { target, duration } => f
                .debug_struct("Goad")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::Suspect { target } => f.debug_tuple("Suspect").field(target).finish(),
            Self::ClearSuspected { target } => {
                f.debug_tuple("ClearSuspected").field(target).finish()
            }
            Self::HealDamage { target, amount } => f
                .debug_struct("HealDamage")
                .field("target", target)
                .field("amount", amount)
                .finish(),
            Self::RemoveFromCombat { target } => {
                f.debug_tuple("RemoveFromCombat").field(target).finish()
            }
            Self::Flip { target } => f.debug_tuple("Flip").field(target).finish(),
            Self::Regenerate {
                target,
                follow_up_effects,
            } => f
                .debug_struct("Regenerate")
                .field("target", target)
                .field("follow_up_effects", follow_up_effects)
                .finish(),
            Self::RegenerateAll { filter } => f.debug_tuple("RegenerateAll").field(filter).finish(),
            Self::Sacrifice {
                filter,
                count,
                target,
                one_of_referenced_set,
            } => f
                .debug_struct("Sacrifice")
                .field("filter", filter)
                .field("count", count)
                .field("target", target)
                .field("one_of_referenced_set", one_of_referenced_set)
                .finish(),
            Self::SacrificeAll { filter } => f
                .debug_struct("SacrificeAll")
                .field("filter", filter)
                .finish(),
        }
    }
}

impl std::fmt::Debug for SubjectVerbEffectAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubjectVerb")
            .field("subject", &self.subject)
            .field("action", &self.action)
            .finish()
    }
}
