use super::*;

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
    /// Target requirement assignments bound to `chosen_targets`.
    pub chosen_target_assignments: Vec<crate::game_state::TargetAssignment>,
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
    /// Stable display pips for the mana payment overlay.
    ///
    /// Unlike `remaining_mana_pips`, this is not mutated as payment proceeds so
    /// the UI can render the full cost with paid and upcoming pips.
    pub display_mana_pips: Vec<Vec<crate::mana::ManaSymbol>>,
    /// Remaining mana pips to pay (pip-by-pip payment flow).
    /// Each element is a pip with its alternatives (e.g., [Black, Life(2)] for {B/P}).
    pub remaining_mana_pips: Vec<Vec<crate::mana::ManaSymbol>>,
    /// Payment options already computed for the current pip prompt.
    pub current_pip_payment_options: Vec<ManaPipPaymentOption>,
    /// Remaining non-mana spell costs to pay, in player-chosen order.
    pub remaining_cost_steps: Vec<ActivationCostStep>,
    /// Tagged object snapshots captured while paying spell costs.
    pub tagged_objects: std::collections::HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>,
    /// Outcomes of spell cost effects labeled with `WithIdEffect`.
    pub effect_outcomes:
        std::collections::HashMap<crate::effect::EffectId, crate::effect::EffectOutcome>,
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
    /// Live state for staged "remove counters from among ..." cost payment.
    pub pending_remove_counters_among: Option<PendingRemoveCountersAmongChoice>,
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
            chosen_target_assignments: Vec::new(),
            remaining_requirements,
            casting_method,
            optional_costs_paid,
            payment_trace: Vec::new(),
            undo_locked_by_mana: false,
            mana_spent_to_cast: ManaPool::default(),
            mana_cost_to_pay: None,
            display_mana_pips: Vec::new(),
            remaining_mana_pips: Vec::new(),
            current_pip_payment_options: Vec::new(),
            remaining_cost_steps: Vec::new(),
            tagged_objects: std::collections::HashMap::new(),
            effect_outcomes: std::collections::HashMap::new(),
            next_sacrifice_cost_tag_index: 0,
            chosen_modes,
            hybrid_choices: Vec::new(),
            pending_hybrid_pips: Vec::new(),
            stack_id,
            keyword_payment_contributions: Vec::new(),
            pending_remove_counters_among: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PendingRemoveCountersAmongChoice {
    pub cost: crate::effects::RemoveAnyCountersAmongEffect,
    pub distribution_ready: bool,
    pub allocations: std::collections::VecDeque<(ObjectId, u32)>,
    pub removed_total: u32,
}

/// Stage of the ability activation process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationStage {
    /// Need to choose modes for modal activated abilities.
    ChoosingModes,
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
            ActivationStage::ChoosingModes => "choosing modes",
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
        cost: crate::costs::Cost,
        card_types: Vec<CardType>,
        description: String,
    },
    /// Choose a card to exile from hand.
    ExileFromHand {
        cost: crate::costs::Cost,
        color_filter: Option<crate::color::ColorSet>,
        description: String,
    },
    /// Choose a card to exile from graveyard.
    ExileFromGraveyard {
        cost: crate::costs::Cost,
        card_type: Option<CardType>,
        description: String,
    },
    /// Choose an object in a specific zone to exile as a cost.
    ExileChosenObject {
        cost: crate::costs::Cost,
        filter: ObjectFilter,
        zone: Zone,
        description: String,
        choice_tag: crate::tag::TagKey,
    },
    /// Choose a card to reveal from hand.
    RevealFromHand {
        cost: crate::costs::Cost,
        card_type: Option<CardType>,
        color_filter: Option<crate::color::ColorSet>,
        description: String,
    },
    /// Choose a permanent to return to hand.
    ReturnToHand {
        cost: crate::costs::Cost,
        filter: ObjectFilter,
        description: String,
        choice_tag: Option<crate::tag::TagKey>,
    },
    /// Choose an object in a specific zone to move to another zone as a cost.
    MoveChosenObjectToZone {
        cost: crate::costs::Cost,
        filter: ObjectFilter,
        source_zone: Zone,
        destination_zone: Zone,
        description: String,
        choice_tag: crate::tag::TagKey,
    },
}

/// Ordered activation-cost step that still needs to be paid after targets are chosen.
#[derive(Debug, Clone)]
pub enum ActivationCostStep {
    /// A cost that can be paid directly through the cost payer.
    Cost(crate::costs::Cost),
    /// A sacrifice choice that must be surfaced through SelectObjects.
    Sacrifice {
        cost: crate::costs::Cost,
        filter: ObjectFilter,
        description: String,
        choice_tag: Option<crate::tag::TagKey>,
    },
    /// A card/object choice that must be surfaced through SelectObjects.
    CardChoice(ActivationCardCostChoice),
}

pub(crate) fn tagged_filter_matches(filter: &ObjectFilter, tag: &crate::tag::TagKey) -> bool {
    filter.tagged_constraints.len() == 1
        && filter.tagged_constraints[0].tag == *tag
        && filter.tagged_constraints[0].relation
            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
}

fn single_exile_object_cost(filter: &ObjectFilter, zone: Zone) -> crate::costs::Cost {
    let mut single_filter = filter.clone();
    if single_filter.zone.is_none() {
        single_filter.zone = Some(zone);
    }
    crate::costs::Cost::validated_effect(crate::effect::Effect::new(
        crate::effects::ExileEffect::with_spec(
            ChooseSpec::Object(single_filter).with_count(crate::effect::ChoiceCount::exactly(1)),
        ),
    ))
}

pub(crate) fn choose_tagged_cost_step(
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
                cost: crate::costs::Cost::sacrifice(choose.filter.clone()),
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
        let zone = choose.filter.zone.or(choose.zone)?;
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
        let cost = single_exile_object_cost(&choose.filter, zone);

        match exile.spec.base() {
            ChooseSpec::Tagged(tag) if tag == &choose.tag => {
                return Some(ActivationCostStep::CardChoice(
                    ActivationCardCostChoice::ExileChosenObject {
                        cost: cost.clone(),
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
                        cost: cost.clone(),
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
                    cost: crate::costs::Cost::return_to_hand(choose.filter.clone()),
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

    if let Some(move_to_zone) = next_effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        let source_zone = choose.filter.zone.or(choose.zone)?;
        let matches_choice = match move_to_zone.target.base() {
            ChooseSpec::Tagged(tag) => tag == &choose.tag,
            ChooseSpec::Object(filter) => tagged_filter_matches(filter, &choose.tag),
            _ => false,
        };
        if matches_choice {
            let destination_zone = move_to_zone.zone;
            let action = match destination_zone {
                Zone::Graveyard => "put into a graveyard",
                Zone::Exile => "exile",
                Zone::Hand => "return to hand",
                Zone::Library => "put into a library",
                Zone::Battlefield => "put onto the battlefield",
                _ => "move",
            };
            return Some(ActivationCostStep::CardChoice(
                ActivationCardCostChoice::MoveChosenObjectToZone {
                    cost: crate::costs::Cost::validated_effect(crate::effect::Effect::new(
                        crate::effects::MoveToZoneEffect::new(
                            ChooseSpec::Tagged(choose.tag.clone()),
                            destination_zone,
                            move_to_zone.to_top,
                        ),
                    )),
                    filter: choose.filter.clone(),
                    source_zone,
                    destination_zone,
                    description: format!("{} {}", action, choose.filter.description()),
                    choice_tag: choose.tag.clone(),
                },
            ));
        }
    }

    None
}

fn single_choice_cost(cost: &crate::costs::Cost) -> crate::costs::Cost {
    use crate::costs::CostProcessingMode;

    match cost.processing_mode() {
        CostProcessingMode::DiscardCards { card_types, .. } => {
            crate::costs::Cost::discard_types(1, card_types)
        }
        CostProcessingMode::ExileFromHand { color_filter, .. } => {
            crate::costs::Cost::exile_from_hand(1, color_filter)
        }
        CostProcessingMode::ExileFromGraveyard { card_type, .. } => {
            crate::costs::Cost::exile_from_graveyard(1, card_type)
        }
        CostProcessingMode::ExileObjects { filter, zone, .. } => {
            single_exile_object_cost(&filter, zone)
        }
        CostProcessingMode::RevealFromHand {
            card_type,
            color_filter,
            ..
        } => crate::costs::Cost::reveal_from_hand_with_color_filter(1, card_type, color_filter),
        _ => cost.clone(),
    }
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

    if cost.dynamic_mana_cost_ref().is_some() {
        return;
    }

    let mode = cost.processing_mode();
    let description = mode.display();
    match mode {
        CostProcessingMode::Immediate | CostProcessingMode::InlineWithTriggers => {
            out.push(ActivationCostStep::Cost(cost.clone()));
        }
        CostProcessingMode::SacrificeTarget { filter } => {
            out.push(ActivationCostStep::Sacrifice {
                cost: single_choice_cost(cost),
                filter,
                description,
                choice_tag: None,
            });
        }
        CostProcessingMode::DiscardCards { count, card_types } => {
            for _ in 0..count {
                out.push(ActivationCostStep::CardChoice(
                    ActivationCardCostChoice::Discard {
                        cost: single_choice_cost(cost),
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
                        cost: single_choice_cost(cost),
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
                        cost: single_choice_cost(cost),
                        card_type,
                        description: description.clone(),
                    },
                ));
            }
        }
        CostProcessingMode::ExileObjects {
            count,
            filter,
            zone,
        } => {
            for _ in 0..count {
                out.push(ActivationCostStep::CardChoice(
                    ActivationCardCostChoice::ExileChosenObject {
                        cost: single_choice_cost(cost),
                        filter: filter.clone(),
                        zone,
                        description: description.clone(),
                        choice_tag: crate::tag::TagKey::from("exile_cost"),
                    },
                ));
            }
        }
        CostProcessingMode::RevealFromHand {
            count,
            card_type,
            color_filter,
        } => {
            let crate::effect::Value::Fixed(count) = count else {
                out.push(ActivationCostStep::Cost(cost.clone()));
                return;
            };
            for _ in 0..count.max(0) as u32 {
                out.push(ActivationCostStep::CardChoice(
                    ActivationCardCostChoice::RevealFromHand {
                        cost: single_choice_cost(cost),
                        card_type,
                        color_filter,
                        description: description.clone(),
                    },
                ));
            }
        }
        CostProcessingMode::ReturnToHandTarget { filter } => {
            out.push(ActivationCostStep::CardChoice(
                ActivationCardCostChoice::ReturnToHand {
                    cost: single_choice_cost(cost),
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
    pub effects: crate::resolution::ResolutionProgram,
    /// Targets that have been chosen so far.
    pub chosen_targets: Vec<Target>,
    /// Target requirement assignments bound to `chosen_targets`.
    pub chosen_target_assignments: Vec<crate::game_state::TargetAssignment>,
    /// Target requirements that still need to be fulfilled.
    pub remaining_requirements: Vec<TargetRequirement>,
    /// The computed mana cost to pay.
    pub mana_cost_to_pay: Option<crate::mana::ManaCost>,
    /// Stable display pips for the mana payment overlay.
    ///
    /// Unlike `remaining_mana_pips`, this is not mutated as payment proceeds so
    /// the UI can render the full cost with paid and upcoming pips.
    pub display_mana_pips: Vec<Vec<crate::mana::ManaSymbol>>,
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
    /// Pre-chosen modes for modal activated abilities.
    /// Set during activation and used during resolution.
    pub chosen_modes: Option<Vec<usize>>,
    /// Spending restrictions for mana produced as this activated ability resolves.
    pub mana_usage_restrictions: Vec<crate::ability::ManaUsageRestriction>,
    /// Chosen creature type snapshot for restricted mana produced by the source.
    pub mana_source_chosen_creature_type: Option<crate::types::Subtype>,
    /// Hybrid/Phyrexian mana choices made during AnnouncingCost stage (per MTG rule 601.2b via 602.2b).
    /// Each element is (pip_index, chosen_symbol).
    pub hybrid_choices: Vec<(usize, crate::mana::ManaSymbol)>,
    /// Pending hybrid/Phyrexian pips that still need announcement.
    /// Each element is (pip_index, alternatives).
    pub pending_hybrid_pips: Vec<(usize, Vec<crate::mana::ManaSymbol>)>,
    /// Live state for staged "remove counters from among ..." cost payment.
    pub pending_remove_counters_among: Option<PendingRemoveCountersAmongChoice>,
}

impl PendingActivation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: ObjectId,
        ability_index: usize,
        activator: PlayerId,
        provenance: ProvNodeId,
        stage: ActivationStage,
        effects: crate::resolution::ResolutionProgram,
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
        mana_usage_restrictions: Vec<crate::ability::ManaUsageRestriction>,
        mana_source_chosen_creature_type: Option<crate::types::Subtype>,
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
            chosen_target_assignments: Vec::new(),
            remaining_requirements,
            mana_cost_to_pay,
            display_mana_pips: Vec::new(),
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
            chosen_modes: None,
            mana_usage_restrictions,
            mana_source_chosen_creature_type,
            hybrid_choices: Vec::new(),
            pending_hybrid_pips,
            pending_remove_counters_among: None,
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
    pub effects: crate::resolution::ResolutionProgram,
    /// Spending restrictions for mana produced by this ability.
    pub mana_usage_restrictions: Vec<crate::ability::ManaUsageRestriction>,
    /// Chosen creature type snapshot for restricted mana produced by the source.
    pub mana_source_chosen_creature_type: Option<crate::types::Subtype>,
    /// How mana produced by this activation was generated.
    pub mana_production_provenance: crate::events::mana::ManaProductionProvenance,
    /// True when undo should be blocked for this pending mana ability flow.
    /// This is set when either:
    /// - the root mana ability itself is not undo-safe, or
    /// - a mana ability activated to pay this mana cost is not undo-safe.
    pub undo_locked_by_mana: bool,
}

/// A suspended priority-loop response that should be rerun once a nested
/// decision answer is available.
#[derive(Debug, Clone)]
pub enum PendingPriorityContinuation {
    ApplyResponse(PriorityResponse),
    ApplyDecisionContext(crate::decisions::context::DecisionContext),
}

/// State for tracking the priority loop between decisions.
#[derive(Debug, Clone)]
pub struct PriorityLoopState {
    pub(super) tracker: PriorityTracker,
    /// A pending spell cast waiting for target selection.
    pub pending_cast: Option<PendingCast>,
    /// A pending ability activation waiting for cost payment.
    pub pending_activation: Option<PendingActivation>,
    /// A pending casting method selection for spells with multiple available methods.
    pub pending_method_selection: Option<PendingMethodSelection>,
    /// A pending mana ability activation waiting for mana payment.
    pub pending_mana_ability: Option<PendingManaAbility>,
    /// A suspended live priority response waiting for a nested decision answer.
    pub pending_continuation: Option<PendingPriorityContinuation>,
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
            pending_continuation: None,
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
            || self.pending_mana_ability.is_some()
            || self.pending_continuation.is_some()
    }

    /// Configure whether single-option pip payments should be auto-selected.
    pub fn set_auto_choose_single_pip_payment(&mut self, enabled: bool) {
        self.auto_choose_single_pip_payment = enabled;
    }

    /// Return the pass-tracker state needed to restore an in-progress priority window.
    pub fn priority_tracker_snapshot(&self) -> (usize, usize) {
        (
            self.tracker.consecutive_passes,
            self.tracker.players_in_game,
        )
    }

    /// Restore pass tracking after importing a sync checkpoint.
    pub fn restore_priority_tracker_for_sync(
        &mut self,
        consecutive_passes: usize,
        players_in_game: usize,
    ) {
        let players_in_game = if players_in_game == 0 {
            self.tracker.players_in_game
        } else {
            players_in_game
        }
        .max(1);
        self.tracker.players_in_game = players_in_game;
        self.tracker.consecutive_passes = consecutive_passes.min(players_in_game.saturating_sub(1));
    }

    /// Reset pass tracking and assign priority to the active player for a fresh priority window.
    pub fn reset_for_new_priority_window(&mut self, game: &mut GameState) {
        self.tracker.set_players_in_game(game.players_in_game());
        reset_priority(game, &mut self.tracker);
    }
}
