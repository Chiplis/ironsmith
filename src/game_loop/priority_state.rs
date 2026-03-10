// ============================================================================
// Priority Loop
// ============================================================================

/// Stage of the spell casting process.
///
/// Per MTG Comprehensive Rules 601.2, casting follows this order:
/// 1. Proposing (601.2a) - Move spell to stack
/// 2. ChoosingModes (601.2b) - Announce modes for modal spells
/// 3. ChoosingX (601.2b) - Announce X value
/// 4. ChoosingOptionalCosts (601.2b) - Announce additional costs (kicker, buyback)
/// 5. AnnouncingCost (601.2b) - Announce hybrid/Phyrexian mana choices
/// 6. ChoosingTargets (601.2c) - Choose targets
/// 7. ChoosingNextCost - Choose the next remaining cost to pay
/// 8. ProcessingCosts - Pay a selected non-mana cost
/// 9. ChoosingSacrifice / ChoosingCardCost - Resolve object/card cost choices
/// 10. PayingMana (601.2g-h) - Activate mana abilities and pay costs
/// 11. ReadyToFinalize (601.2i) - Spell becomes cast
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastStage {
    /// Spell is being proposed - moved to stack per 601.2a.
    /// This is the first stage when casting begins.
    Proposing,
    /// Need to choose modes for modal spells (per 601.2b).
    /// Modes must be chosen before targets.
    ChoosingModes,
    /// Need to choose X value (for spells with X in cost).
    ChoosingX,
    /// Need to choose optional costs (kicker, buyback, etc.).
    ChoosingOptionalCosts,
    /// Need to announce hybrid/Phyrexian mana payment choices (per 601.2b).
    /// These choices are locked in before targets are chosen.
    AnnouncingCost,
    /// Need to choose targets.
    ChoosingTargets,
    /// Need to choose the next remaining cost to pay.
    ChoosingNextCost,
    /// Need to pay a selected immediate non-mana cost.
    ProcessingCosts,
    /// Need to choose a permanent for a sacrifice cost.
    ChoosingSacrifice,
    /// Need to choose cards/objects for non-mana costs (discard, exile-from-hand, etc.).
    ChoosingCardCost,
    /// Need to pay mana costs (player can activate mana abilities).
    PayingMana,
    /// Ready to finalize (mana has been paid and spell becomes cast).
    ReadyToFinalize,
}

impl CastStage {
    pub fn name(&self) -> &'static str {
        match self {
            CastStage::Proposing => "proposing",
            CastStage::ChoosingModes => "choosing modes",
            CastStage::ChoosingX => "choosing X",
            CastStage::ChoosingOptionalCosts => "choosing optional costs",
            CastStage::AnnouncingCost => "announcing costs",
            CastStage::ChoosingTargets => "choosing targets",
            CastStage::ChoosingNextCost => "choosing next cost",
            CastStage::ProcessingCosts => "processing costs",
            CastStage::ChoosingSacrifice => "choosing sacrifices",
            CastStage::ChoosingCardCost => "choosing card costs",
            CastStage::PayingMana => "paying mana",
            CastStage::ReadyToFinalize => "ready to finalize",
        }
    }
}

impl std::fmt::Display for CastStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Pending casting method selection for a spell with multiple available methods.
#[derive(Debug, Clone)]
pub struct PendingMethodSelection {
    /// The spell being cast.
    pub spell_id: ObjectId,
    /// The zone the spell is being cast from.
    pub from_zone: Zone,
    /// The player casting the spell.
    pub caster: PlayerId,
    /// The available casting method options.
    pub available_methods: Vec<crate::decision::CastingMethodOption>,
}

/// A spell or ability being cast/activated that needs decisions.
#[derive(Debug, Clone)]
pub struct PendingCast {
    /// The spell/ability being cast.
    pub spell_id: ObjectId,
    /// The zone the spell is being cast from.
    pub from_zone: Zone,
    /// The player casting the spell.
    pub caster: PlayerId,
    /// Provenance parent for costs/effects emitted by this cast flow.
    pub provenance: ProvNodeId,
    /// Current stage of the casting process.
    pub stage: CastStage,
    /// The chosen X value (if applicable).
    pub x_value: Option<u32>,
    /// Targets that have been chosen so far.
    pub chosen_targets: Vec<Target>,
    /// Target requirements that still need to be fulfilled.
    pub remaining_requirements: Vec<TargetRequirement>,
    /// The casting method (normal or alternative like flashback).
    pub casting_method: CastingMethod,
    /// Which optional costs will be paid (kicker, buyback, etc.).
    pub optional_costs_paid: OptionalCostsPaid,
    /// Ordered trace of cost payments performed so far.
    pub payment_trace: Vec<CostStep>,
    /// True after activating a mana ability that is not undo-safe
    /// (for example it adds/removes counters, sacrifices, loses life, or has
    /// non-mana side effects).
    pub undo_locked_by_mana: bool,
    /// Mana actually spent to cast the spell (color-by-color).
    pub mana_spent_to_cast: ManaPool,
    /// The computed mana cost to pay (set during PayingMana stage).
    pub mana_cost_to_pay: Option<crate::mana::ManaCost>,
    /// Remaining mana pips to pay (pip-by-pip payment flow).
    /// Each element is a pip with its alternatives (e.g., [Black, Life(2)] for {B/P}).
    pub remaining_mana_pips: Vec<Vec<crate::mana::ManaSymbol>>,
    /// Remaining non-mana spell costs to pay, in player-chosen order.
    pub remaining_cost_steps: Vec<ActivationCostStep>,
    /// Tagged object snapshots captured while paying spell costs.
    pub tagged_objects: std::collections::HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>,
    /// Next `sacrifice_cost_{N}` tag index to assign for choose-and-sacrifice costs.
    pub next_sacrifice_cost_tag_index: usize,
    /// Pre-chosen modes for modal spells (per MTG rule 601.2b).
    /// Set during ChoosingModes stage, used during resolution.
    pub chosen_modes: Option<Vec<usize>>,
    /// Hybrid/Phyrexian mana payment choices made during cost announcement (601.2b).
    /// Maps pip index to the chosen mana symbol for that pip.
    pub hybrid_choices: Vec<(usize, crate::mana::ManaSymbol)>,
    /// Hybrid/Phyrexian pips that still need announcement (601.2b).
    /// Each element is (pip_index, alternatives). Processed one at a time.
    pub pending_hybrid_pips: Vec<(usize, Vec<crate::mana::ManaSymbol>)>,
    /// The spell's ObjectId on the stack (after being moved per 601.2a).
    pub stack_id: ObjectId,
    /// Permanents that contributed keyword-ability alternative payments while casting this spell.
    pub keyword_payment_contributions: Vec<KeywordPaymentContribution>,
}

impl PendingCast {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        spell_id: ObjectId,
        from_zone: Zone,
        caster: PlayerId,
        provenance: ProvNodeId,
        stage: CastStage,
        x_value: Option<u32>,
        remaining_requirements: Vec<TargetRequirement>,
        casting_method: CastingMethod,
        optional_costs_paid: OptionalCostsPaid,
        chosen_modes: Option<Vec<usize>>,
        stack_id: ObjectId,
    ) -> Self {
        Self {
            spell_id,
            from_zone,
            caster,
            provenance,
            stage,
            x_value,
            chosen_targets: Vec::new(),
            remaining_requirements,
            casting_method,
            optional_costs_paid,
            payment_trace: Vec::new(),
            undo_locked_by_mana: false,
            mana_spent_to_cast: ManaPool::default(),
            mana_cost_to_pay: None,
            remaining_mana_pips: Vec::new(),
            remaining_cost_steps: Vec::new(),
            tagged_objects: std::collections::HashMap::new(),
            next_sacrifice_cost_tag_index: 0,
            chosen_modes,
            hybrid_choices: Vec::new(),
            pending_hybrid_pips: Vec::new(),
            stack_id,
            keyword_payment_contributions: Vec::new(),
        }
    }
}

/// Stage of the ability activation process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationStage {
    /// Need to choose X value for abilities with X in cost.
    ChoosingX,
    /// Need to announce hybrid/Phyrexian mana payment choices (per MTG rule 601.2b via 602.2b).
    AnnouncingCost,
    /// Need to choose ability targets.
    ChoosingTargets,
    /// Need to choose the next remaining cost to pay.
    ChoosingNextCost,
    /// Need to pay a selected deferred non-mana cost.
    ProcessingCosts,
    /// Need to choose sacrifice targets.
    ChoosingSacrifice,
    /// Need to choose cards in hand for discard/exile-from-hand costs.
    ChoosingCardCost,
    /// Need to pay mana costs (player can activate mana abilities).
    PayingMana,
    /// Ready to finalize (costs paid, ability goes on stack).
    ReadyToFinalize,
}

impl ActivationStage {
    pub fn name(&self) -> &'static str {
        match self {
            ActivationStage::ChoosingX => "choosing X",
            ActivationStage::AnnouncingCost => "announcing costs",
            ActivationStage::ChoosingTargets => "choosing targets",
            ActivationStage::ChoosingNextCost => "choosing next cost",
            ActivationStage::ProcessingCosts => "processing costs",
            ActivationStage::ChoosingSacrifice => "choosing sacrifices",
            ActivationStage::ChoosingCardCost => "choosing card costs",
            ActivationStage::PayingMana => "paying mana",
            ActivationStage::ReadyToFinalize => "ready to finalize",
        }
    }
}

impl std::fmt::Display for ActivationStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Pending card-in-hand choice required by an activated ability cost.
#[derive(Debug, Clone)]
pub enum ActivationCardCostChoice {
    /// Choose a card to discard from hand.
    Discard {
        card_types: Vec<CardType>,
        description: String,
    },
    /// Choose a card to exile from hand.
    ExileFromHand {
        color_filter: Option<crate::color::ColorSet>,
        description: String,
    },
    /// Choose a card to exile from graveyard.
    ExileFromGraveyard {
        card_type: Option<CardType>,
        description: String,
    },
    /// Choose an object in a specific zone to exile as a cost.
    ExileChosenObject {
        filter: ObjectFilter,
        zone: Zone,
        description: String,
        choice_tag: crate::tag::TagKey,
    },
    /// Choose a card to reveal from hand.
    RevealFromHand {
        card_type: Option<CardType>,
        description: String,
    },
    /// Choose a permanent to return to hand.
    ReturnToHand {
        filter: ObjectFilter,
        description: String,
        choice_tag: Option<crate::tag::TagKey>,
    },
}

/// Ordered activation-cost step that still needs to be paid after targets are chosen.
#[derive(Debug, Clone)]
pub enum ActivationCostStep {
    /// A cost that can be paid directly through the cost payer.
    Cost(crate::costs::Cost),
    /// A sacrifice choice that must be surfaced through SelectObjects.
    Sacrifice {
        filter: ObjectFilter,
        description: String,
        choice_tag: Option<crate::tag::TagKey>,
    },
    /// A card/object choice that must be surfaced through SelectObjects.
    CardChoice(ActivationCardCostChoice),
}

/// Expand a cost processing mode into pending card/object-choice steps.
///
/// Returns `true` when the mode produced one-or-more card/object selections.
pub(crate) fn append_card_choice_costs_from_processing_mode(
    mode: &crate::costs::CostProcessingMode,
    out: &mut Vec<ActivationCardCostChoice>,
) -> bool {
    use crate::costs::CostProcessingMode;

    let description = mode.display();
    let before_len = out.len();
    match mode {
        CostProcessingMode::DiscardCards { count, card_types } => {
            for _ in 0..*count {
                out.push(ActivationCardCostChoice::Discard {
                    card_types: card_types.clone(),
                    description: description.clone(),
                });
            }
        }
        CostProcessingMode::ExileFromHand {
            count,
            color_filter,
        } => {
            for _ in 0..*count {
                out.push(ActivationCardCostChoice::ExileFromHand {
                    color_filter: *color_filter,
                    description: description.clone(),
                });
            }
        }
        CostProcessingMode::ExileFromGraveyard { count, card_type } => {
            for _ in 0..*count {
                out.push(ActivationCardCostChoice::ExileFromGraveyard {
                    card_type: *card_type,
                    description: description.clone(),
                });
            }
        }
        CostProcessingMode::RevealFromHand { count, card_type } => {
            for _ in 0..*count {
                out.push(ActivationCardCostChoice::RevealFromHand {
                    card_type: *card_type,
                    description: description.clone(),
                });
            }
        }
        CostProcessingMode::ReturnToHandTarget { filter } => {
            out.push(ActivationCardCostChoice::ReturnToHand {
                filter: filter.clone(),
                description,
                choice_tag: None,
            });
        }
        CostProcessingMode::Immediate
        | CostProcessingMode::InlineWithTriggers
        | CostProcessingMode::ManaPayment { .. }
        | CostProcessingMode::SacrificeTarget { .. } => {}
    }

    out.len() > before_len
}

fn tagged_filter_matches(filter: &ObjectFilter, tag: &crate::tag::TagKey) -> bool {
    filter.tagged_constraints.len() == 1
        && filter.tagged_constraints[0].tag == *tag
        && filter.tagged_constraints[0].relation
            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
}

fn choose_tagged_cost_step(
    choose: &crate::effects::ChooseObjectsEffect,
    next: &crate::costs::Cost,
) -> Option<ActivationCostStep> {
    if !choose.count.is_single() {
        return None;
    }

    let next_effect = next.effect_ref()?;

    if let Some(sacrifice) = next_effect.downcast_ref::<crate::effects::SacrificeEffect>() {
        if sacrifice.player == crate::target::PlayerFilter::You
            && tagged_filter_matches(&sacrifice.filter, &choose.tag)
        {
            return Some(ActivationCostStep::Sacrifice {
                filter: choose.filter.clone(),
                description: crate::costs::CostProcessingMode::SacrificeTarget {
                    filter: choose.filter.clone(),
                }
                .display(),
                choice_tag: Some(choose.tag.clone()),
            });
        }
    }

    if let Some(exile) = next_effect.downcast_ref::<crate::effects::ExileEffect>() {
        let zone = choose.filter.zone.unwrap_or(choose.zone);
        let description = match zone {
            Zone::Hand => crate::costs::CostProcessingMode::ExileFromHand {
                count: 1,
                color_filter: choose.filter.colors,
            }
            .display(),
            Zone::Graveyard => crate::costs::CostProcessingMode::ExileFromGraveyard {
                count: 1,
                card_type: if choose.filter.card_types.len() == 1 {
                    choose.filter.card_types.first().copied()
                } else {
                    None
                },
            }
            .display(),
            _ => format!("Exile {}", choose.filter.description()),
        };

        match exile.spec.base() {
            ChooseSpec::Tagged(tag) if tag == &choose.tag => {
                return Some(ActivationCostStep::CardChoice(
                    ActivationCardCostChoice::ExileChosenObject {
                        filter: choose.filter.clone(),
                        zone,
                        description,
                        choice_tag: choose.tag.clone(),
                    },
                ));
            }
            ChooseSpec::Object(filter) if tagged_filter_matches(filter, &choose.tag) => {
                return Some(ActivationCostStep::CardChoice(
                    ActivationCardCostChoice::ExileChosenObject {
                        filter: choose.filter.clone(),
                        zone,
                        description,
                        choice_tag: choose.tag.clone(),
                    },
                ));
            }
            _ => {}
        }
    }

    if let Some(return_to_hand) = next_effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        if let ChooseSpec::Object(filter) = return_to_hand.spec.base()
            && tagged_filter_matches(filter, &choose.tag)
        {
            return Some(ActivationCostStep::CardChoice(
                ActivationCardCostChoice::ReturnToHand {
                    filter: choose.filter.clone(),
                    description: crate::costs::CostProcessingMode::ReturnToHandTarget {
                        filter: choose.filter.clone(),
                    }
                    .display(),
                    choice_tag: Some(choose.tag.clone()),
                },
            ));
        }
    }

    None
}

pub(crate) fn append_activation_cost_steps_from_components(
    components: &[crate::costs::Cost],
    out: &mut Vec<ActivationCostStep>,
) {
    let mut idx = 0usize;
    while idx < components.len() {
        if let Some(choose) = components[idx]
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(next) = components.get(idx + 1)
            && let Some(step) = choose_tagged_cost_step(choose, next)
        {
            out.push(step);
            idx += 2;
            continue;
        }

        append_activation_cost_steps_from_cost(&components[idx], out);
        idx += 1;
    }
}

/// Expand an activation cost into ordered post-target cost-payment steps.
pub(crate) fn append_activation_cost_steps_from_cost(
    cost: &crate::costs::Cost,
    out: &mut Vec<ActivationCostStep>,
) {
    use crate::costs::CostProcessingMode;

    let mode = cost.processing_mode();
    let description = mode.display();
    match mode {
        CostProcessingMode::Immediate | CostProcessingMode::InlineWithTriggers => {
            out.push(ActivationCostStep::Cost(cost.clone()));
        }
        CostProcessingMode::SacrificeTarget { filter } => {
            out.push(ActivationCostStep::Sacrifice {
                filter,
                description,
                choice_tag: None,
            });
        }
        CostProcessingMode::DiscardCards { count, card_types } => {
            for _ in 0..count {
                out.push(ActivationCostStep::CardChoice(
                    ActivationCardCostChoice::Discard {
                        card_types: card_types.clone(),
                        description: description.clone(),
                    },
                ));
            }
        }
        CostProcessingMode::ExileFromHand {
            count,
            color_filter,
        } => {
            for _ in 0..count {
                out.push(ActivationCostStep::CardChoice(
                    ActivationCardCostChoice::ExileFromHand {
                        color_filter,
                        description: description.clone(),
                    },
                ));
            }
        }
        CostProcessingMode::ExileFromGraveyard { count, card_type } => {
            for _ in 0..count {
                out.push(ActivationCostStep::CardChoice(
                    ActivationCardCostChoice::ExileFromGraveyard {
                        card_type,
                        description: description.clone(),
                    },
                ));
            }
        }
        CostProcessingMode::RevealFromHand { count, card_type } => {
            for _ in 0..count {
                out.push(ActivationCostStep::CardChoice(
                    ActivationCardCostChoice::RevealFromHand {
                        card_type,
                        description: description.clone(),
                    },
                ));
            }
        }
        CostProcessingMode::ReturnToHandTarget { filter } => {
            out.push(ActivationCostStep::CardChoice(
                ActivationCardCostChoice::ReturnToHand {
                    filter,
                    description,
                    choice_tag: None,
                },
            ));
        }
        CostProcessingMode::ManaPayment { .. } => {}
    }
}

/// An activated ability being activated that needs decisions.
#[derive(Debug, Clone)]
pub struct PendingActivation {
    /// The source permanent of the activated ability.
    pub source: ObjectId,
    /// Index of the ability being activated.
    pub ability_index: usize,
    /// The player activating the ability.
    pub activator: PlayerId,
    /// Provenance parent for costs/effects emitted by this activation flow.
    pub provenance: ProvNodeId,
    /// Current stage of the activation process.
    pub stage: ActivationStage,
    /// The effects of the ability.
    pub effects: Vec<crate::effect::Effect>,
    /// Targets that have been chosen so far.
    pub chosen_targets: Vec<Target>,
    /// Target requirements that still need to be fulfilled.
    pub remaining_requirements: Vec<TargetRequirement>,
    /// The computed mana cost to pay.
    pub mana_cost_to_pay: Option<crate::mana::ManaCost>,
    /// Ordered trace of cost payments performed so far.
    pub payment_trace: Vec<CostStep>,
    /// True after activating a mana ability that is not undo-safe while paying
    /// this activation's mana costs.
    pub undo_locked_by_mana: bool,
    /// Remaining mana pips to pay (pip-by-pip payment flow).
    /// Each element is a pip with its alternatives (e.g., [Black, Life(2)] for {B/P}).
    pub remaining_mana_pips: Vec<Vec<crate::mana::ManaSymbol>>,
    /// Remaining non-mana activation costs awaiting payment.
    pub remaining_cost_steps: Vec<ActivationCostStep>,
    /// Tagged object snapshots captured while paying activation costs.
    ///
    /// This preserves cost-time references such as `sacrifice_cost_0` for
    /// later resolution-time value lookups.
    pub tagged_objects: std::collections::HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>,
    /// Next `sacrifice_cost_{N}` tag index to assign for choose-and-sacrifice costs.
    pub next_sacrifice_cost_tag_index: usize,
    /// Whether this ability is once per turn (needs recording).
    pub is_once_per_turn: bool,
    /// Stable instance ID of the source (persists across zone changes).
    pub source_stable_id: StableId,
    /// Last known information for the source at activation time.
    pub source_snapshot: ObjectSnapshot,
    /// Name of the source for display purposes.
    pub source_name: String,
    /// The chosen X value for abilities with X in cost.
    pub x_value: Option<usize>,
    /// Hybrid/Phyrexian mana choices made during AnnouncingCost stage (per MTG rule 601.2b via 602.2b).
    /// Each element is (pip_index, chosen_symbol).
    pub hybrid_choices: Vec<(usize, crate::mana::ManaSymbol)>,
    /// Pending hybrid/Phyrexian pips that still need announcement.
    /// Each element is (pip_index, alternatives).
    pub pending_hybrid_pips: Vec<(usize, Vec<crate::mana::ManaSymbol>)>,
}

impl PendingActivation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: ObjectId,
        ability_index: usize,
        activator: PlayerId,
        provenance: ProvNodeId,
        stage: ActivationStage,
        effects: Vec<crate::effect::Effect>,
        remaining_requirements: Vec<TargetRequirement>,
        mana_cost_to_pay: Option<crate::mana::ManaCost>,
        payment_trace: Vec<CostStep>,
        remaining_cost_steps: Vec<ActivationCostStep>,
        tagged_objects: std::collections::HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>,
        next_sacrifice_cost_tag_index: usize,
        is_once_per_turn: bool,
        source_stable_id: StableId,
        source_snapshot: ObjectSnapshot,
        source_name: String,
        x_value: Option<usize>,
        pending_hybrid_pips: Vec<(usize, Vec<crate::mana::ManaSymbol>)>,
    ) -> Self {
        Self {
            source,
            ability_index,
            activator,
            provenance,
            stage,
            effects,
            chosen_targets: Vec::new(),
            remaining_requirements,
            mana_cost_to_pay,
            payment_trace,
            undo_locked_by_mana: false,
            remaining_mana_pips: Vec::new(),
            remaining_cost_steps,
            tagged_objects,
            next_sacrifice_cost_tag_index,
            is_once_per_turn,
            source_stable_id,
            source_snapshot,
            source_name,
            x_value,
            hybrid_choices: Vec::new(),
            pending_hybrid_pips,
        }
    }
}

/// A mana ability being activated that needs mana payment first.
///
/// Mana abilities don't use the stack, but if they have a mana cost
/// (like Blood Celebrant's {B}), we need to let the player tap mana sources first.
#[derive(Debug, Clone)]
pub struct PendingManaAbility {
    /// The source permanent of the mana ability.
    pub source: ObjectId,
    /// Index of the ability being activated.
    pub ability_index: usize,
    /// The player activating the ability.
    pub activator: PlayerId,
    /// Provenance parent for costs/effects emitted by this mana-ability flow.
    pub provenance: ProvNodeId,
    /// The mana cost that needs to be paid.
    pub mana_cost: crate::mana::ManaCost,
    /// Other (non-mana) costs that have already been validated.
    pub other_costs: Vec<crate::costs::Cost>,
    /// The mana symbols to add (for simple mana abilities).
    pub mana_to_add: Vec<crate::mana::ManaSymbol>,
    /// The effects to execute (for complex mana abilities like Blood Celebrant).
    pub effects: Vec<crate::effect::Effect>,
    /// True when undo should be blocked for this pending mana ability flow.
    /// This is set when either:
    /// - the root mana ability itself is not undo-safe, or
    /// - a mana ability activated to pay this mana cost is not undo-safe.
    pub undo_locked_by_mana: bool,
}

/// State for tracking the priority loop between decisions.
#[derive(Debug, Clone)]
pub struct PriorityLoopState {
    tracker: PriorityTracker,
    /// A pending spell cast waiting for target selection.
    pub pending_cast: Option<PendingCast>,
    /// A pending ability activation waiting for cost payment.
    pub pending_activation: Option<PendingActivation>,
    /// A pending casting method selection for spells with multiple available methods.
    pub pending_method_selection: Option<PendingMethodSelection>,
    /// A pending mana ability activation waiting for mana payment.
    pub pending_mana_ability: Option<PendingManaAbility>,
    /// Checkpoint of game state saved when starting an action chain.
    /// If an error occurs during the chain, we restore to this state.
    pub checkpoint: Option<GameState>,
    /// Whether pip-by-pip mana payment should auto-pick a single legal option.
    /// CLI/tests can keep this enabled for speed; WASM UI can disable it to require explicit taps.
    pub auto_choose_single_pip_payment: bool,
}

impl PriorityLoopState {
    /// Create a new priority loop state.
    pub fn new(num_players: usize) -> Self {
        Self {
            tracker: PriorityTracker::new(num_players),
            pending_cast: None,
            pending_activation: None,
            pending_method_selection: None,
            pending_mana_ability: None,
            checkpoint: None,
            auto_choose_single_pip_payment: true,
        }
    }

    /// Save a checkpoint of the current game state.
    /// This should be called when starting an action chain (cast spell, activate ability).
    pub fn save_checkpoint(&mut self, game: &GameState) {
        self.checkpoint = Some(game.clone());
    }

    /// Clear the checkpoint (called when action completes successfully or after restore).
    pub fn clear_checkpoint(&mut self) {
        self.checkpoint = None;
    }

    /// Check if there's an active action chain (pending cast or activation).
    pub fn has_pending_action(&self) -> bool {
        self.pending_cast.is_some()
            || self.pending_activation.is_some()
            || self.pending_method_selection.is_some()
    }

    /// Configure whether single-option pip payments should be auto-selected.
    pub fn set_auto_choose_single_pip_payment(&mut self, enabled: bool) {
        self.auto_choose_single_pip_payment = enabled;
    }

    /// Reset pass tracking and assign priority to the active player for a fresh priority window.
    pub fn reset_for_new_priority_window(&mut self, game: &mut GameState) {
        self.tracker.set_players_in_game(game.players_in_game());
        reset_priority(game, &mut self.tracker);
    }
}
