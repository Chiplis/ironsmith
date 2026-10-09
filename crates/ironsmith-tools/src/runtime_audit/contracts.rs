//! Conservative context-contract checks over the compiler's serialized model.
//!
//! This is not a proof of gameplay correctness. A clean result means that the
//! context references inspected here have a known provider. Unknown providers,
//! delayed captures, and execution paths are explicit coverage gaps. The input
//! is the typed compiler definition, never rendered text or a Debug dump.

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize)]
pub struct ContractFinding {
    pub path: String,
    pub severity: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Binding {
    Present,
    Absent,
    Unknown,
}

impl Binding {
    fn intersect(self, other: Self) -> Self {
        if self == other { self } else { Self::Unknown }
    }
}

#[derive(Clone)]
struct Scope {
    player: Binding,
    object: Binding,
    event: Binding,
    event_object: Binding,
    ability: Binding,
    cast_event: Binding,
    amount: Binding,
    life_gain: Binding,
    life_loss: Binding,
    life_controller: Binding,
    die_result: Binding,
    die_batch: Binding,
    blockers: Binding,
    damaged_player: Binding,
    declared_outcomes: BTreeSet<u64>,
    available_outcomes: BTreeSet<u64>,
    external_outcomes: bool,
}

impl Scope {
    fn empty() -> Self {
        Self {
            player: Binding::Absent,
            object: Binding::Absent,
            event: Binding::Absent,
            event_object: Binding::Absent,
            ability: Binding::Absent,
            cast_event: Binding::Absent,
            amount: Binding::Absent,
            life_gain: Binding::Absent,
            life_loss: Binding::Absent,
            life_controller: Binding::Absent,
            die_result: Binding::Absent,
            die_batch: Binding::Absent,
            blockers: Binding::Absent,
            damaged_player: Binding::Absent,
            declared_outcomes: BTreeSet::new(),
            available_outcomes: BTreeSet::new(),
            external_outcomes: false,
        }
    }

    fn uncertain(&self) -> Self {
        let mut scope = self.clone();
        scope.player = Binding::Unknown;
        scope.object = Binding::Unknown;
        scope.event = Binding::Unknown;
        scope.event_object = Binding::Unknown;
        scope.ability = Binding::Unknown;
        scope.cast_event = Binding::Unknown;
        scope.amount = Binding::Unknown;
        scope.life_gain = Binding::Unknown;
        scope.life_loss = Binding::Unknown;
        scope.life_controller = Binding::Unknown;
        scope.die_result = Binding::Unknown;
        scope.die_batch = Binding::Unknown;
        scope.blockers = Binding::Unknown;
        scope.damaged_player = Binding::Unknown;
        scope.external_outcomes = true;
        scope
    }

    fn intersect(&self, other: &Self) -> Self {
        Self {
            player: self.player.intersect(other.player),
            object: self.object.intersect(other.object),
            event: self.event.intersect(other.event),
            event_object: self.event_object.intersect(other.event_object),
            ability: self.ability.intersect(other.ability),
            cast_event: self.cast_event.intersect(other.cast_event),
            amount: self.amount.intersect(other.amount),
            life_gain: self.life_gain.intersect(other.life_gain),
            life_loss: self.life_loss.intersect(other.life_loss),
            life_controller: self.life_controller.intersect(other.life_controller),
            die_result: self.die_result.intersect(other.die_result),
            die_batch: self.die_batch.intersect(other.die_batch),
            blockers: self.blockers.intersect(other.blockers),
            damaged_player: self.damaged_player.intersect(other.damaged_player),
            declared_outcomes: self
                .declared_outcomes
                .union(&other.declared_outcomes)
                .copied()
                .collect(),
            available_outcomes: self
                .available_outcomes
                .intersection(&other.available_outcomes)
                .copied()
                .collect(),
            external_outcomes: self.external_outcomes || other.external_outcomes,
        }
    }
}

/// Inspect every ability, spell program, cost, and embedded definition.
///
/// Paths use JSON Pointer syntax. `error` identifies a violated necessary
/// context contract; `coverage_gap` means that this checker cannot establish
/// the contract. Neither category says whether a particular play reaches it.
pub fn audit(definition: &Value) -> Vec<ContractFinding> {
    let mut auditor = Auditor {
        findings: Vec::new(),
    };
    if !definition.is_object() || definition.get("abilities").is_none() {
        auditor.gap(
            "",
            "definition_schema",
            "Expected a serialized compiler CardDefinition with an abilities field",
        );
    }
    auditor.walk(definition, "", &mut Scope::empty());
    auditor
        .findings
        .sort_by(|a, b| (&a.path, &a.code, &a.message).cmp(&(&b.path, &b.code, &b.message)));
    auditor
        .findings
        .dedup_by(|a, b| a.path == b.path && a.code == b.code && a.message == b.message);
    auditor.findings
}

struct Auditor {
    findings: Vec<ContractFinding>,
}

impl Auditor {
    fn finding(&mut self, path: &str, severity: &str, code: &str, message: impl Into<String>) {
        self.findings.push(ContractFinding {
            path: path.into(),
            severity: severity.into(),
            code: code.into(),
            message: message.into(),
        });
    }

    fn gap(&mut self, path: &str, code: &str, message: impl Into<String>) {
        self.finding(path, "coverage_gap", code, message);
    }

    fn require(&mut self, path: &str, name: &str, binding: Binding) {
        match binding {
            Binding::Present => {}
            Binding::Absent => self.finding(
                path,
                "error",
                "unbound_context",
                format!("{name} has no provider in this execution scope"),
            ),
            Binding::Unknown => self.gap(
                path,
                "context_provider_unknown",
                format!("Cannot establish a provider for {name} on every execution path"),
            ),
        }
    }

    fn walk(&mut self, value: &Value, path: &str, scope: &mut Scope) {
        match value {
            Value::String(name) => self.atom(name, path, scope),
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    self.walk(item, &child(path, &index.to_string()), scope);
                }
            }
            Value::Object(fields) => {
                if fields.contains_key("card") && fields.contains_key("abilities") {
                    let mut fresh = Scope::empty();
                    collect_outcomes(value, &mut fresh.declared_outcomes, true);
                    // Costs run before the spell's resolution program. Definitions
                    // embedded in tokens/emblems never inherit their creator's loop.
                    for key in [
                        "additional_cost",
                        "optional_costs",
                        "alternative_casts",
                        "spell_effect",
                        "abilities",
                        "aura_attach_filter",
                        "card",
                    ] {
                        self.field(fields, key, path, &mut fresh);
                    }
                    return;
                }
                if let Some(kind) = fields.get("kind") {
                    if let Some((name, payload)) = enum_variant(kind)
                        && matches!(name, "Triggered" | "Activated" | "Static")
                    {
                        self.ability(name, payload, &child(&child(path, "kind"), name));
                        return;
                    }
                    if let (Some(name), Some(payload)) = (kind.as_str(), fields.get("payload")) {
                        self.effect(name, payload, &child(path, "payload"), scope);
                        return;
                    }
                }
                // Grantable and nested ability kinds are sometimes serialized
                // directly, without the enclosing Ability.kind field.
                if let Some((name, payload)) = enum_variant(value)
                    && matches!(name, "Triggered" | "Activated" | "Static")
                {
                    self.ability(name, payload, &child(path, name));
                    return;
                }
                if fields.contains_key("segments") {
                    self.program(fields, path, scope);
                    return;
                }
                if fields.len() == 1 {
                    let (name, payload) = fields.iter().next().unwrap();
                    let at = child(path, name);
                    self.condition_context(name, &at, scope);
                    if is_pending_value(name) {
                        self.finding(
                            &at,
                            "error",
                            "unresolved_compiler_value",
                            format!("Compiler-only {name} reached the executable definition"),
                        );
                    } else if name == "EventValue" || name == "EventValueOffset" {
                        let spec = if name == "EventValueOffset" {
                            payload.get(0).unwrap_or(&Value::Null)
                        } else {
                            payload
                        };
                        self.event_value(spec, &at, scope);
                    } else if matches!(
                        name.as_str(),
                        "EffectValue"
                            | "EffectValueOffset"
                            | "EffectMetric"
                            | "EffectMetricOffset"
                            | "PriorEffectMetric"
                    ) {
                        let id = payload
                            .as_u64()
                            .or_else(|| payload.get(0).and_then(Value::as_u64))
                            .or_else(|| payload.get("effect_id").and_then(Value::as_u64));
                        self.outcome(id, &at, scope);
                    } else if matches!(
                        name.as_str(),
                        "TaggedPlayer"
                            | "Tagged"
                            | "TaggedObject"
                            | "TaggedObjectMatches"
                            | "TaggedObjectMatchedLastKnown"
                    ) {
                        self.gap(&at, "tag_dataflow", "Tagged reference requires producer and lifetime analysis across choices, events, zones, and branches");
                    }
                }
                for (key, value) in fields {
                    if !metadata(key) {
                        self.walk(value, &child(path, key), scope);
                    }
                }
            }
            _ => {}
        }
    }

    fn field(&mut self, fields: &Map<String, Value>, key: &str, path: &str, scope: &mut Scope) {
        if let Some(value) = fields.get(key) {
            self.walk(value, &child(path, key), scope);
        }
    }

    fn atom(&mut self, name: &str, path: &str, scope: &Scope) {
        self.condition_context(name, path, scope);
        match name {
            "IteratedPlayer" => self.require(path, name, scope.player),
            "IteratedObject" => self.require(path, name, scope.object),
            "Iterated" => {
                if scope.player == Binding::Absent && scope.object == Binding::Absent {
                    self.require(path, "ChooseSpec::Iterated", Binding::Absent);
                } else if path.ends_with("/player") {
                    self.require(path, "ChooseSpec::Iterated player", scope.player);
                } else if scope.object != Binding::Present {
                    self.gap(path, "iterated_choice_domain", "ChooseSpec::Iterated may require an object or player depending on the consuming operation");
                }
            }
            // Damage can also be supplied by a preceding effect, so a missing
            // trigger alone does not prove a DamagedPlayer reference invalid.
            "DamagedPlayer" if scope.damaged_player != Binding::Present => self.gap(path, "damage_recipient_dataflow", "DamagedPlayer may come from the triggering event, tagged players, or a prior damage outcome"),
            "ChosenPlayer" | "ChosenNumber" | "TaggedCount" | "LastNotedLifeTotal" | "Defending" | "Attacking" | "TargetPlayerOrControllerOfTarget" => self.gap(path, "persistent_or_choice_context", format!("{name} requires game-state, target, choice, or combat-context validation")),
            "ManaSpentToCastTriggeringObject" | "CasterManaSpentToCastTriggeringObject" => self.require(path, name, scope.cast_event),
            "ThisAbilityResolvedThisTurnCount" => self.require(path, name, scope.ability),
            name if is_pending_value(name) => self.finding(path, "error", "unresolved_compiler_value", format!("Compiler-only {name} reached the executable definition")),
            _ => {}
        }
    }

    fn condition_context(&mut self, name: &str, path: &str, scope: &Scope) {
        // Source-checked against condition_eval.rs: these predicates return
        // false when the triggering event is absent; none falls back to the
        // activated ability's source or its current game-state characteristics.
        let binding = match name {
            "TriggeringSpellManaSpentToCastAtLeast"
            | "TriggeringSpellColoredManaSpentToCastAtLeast"
            | "TriggeringSpellSnowManaOfAnySpellColorSpentToCast"
            | "TriggeringSpellWasKicked"
            | "AnotherOpponentControlsPotentialTarget" => scope.cast_event,
            "CombatParticipant"
            | "TriggeringEventCausedBy"
            | "TriggeringObjectWasEnchanted"
            | "TriggeringObjectHadCounters"
            | "EvolveEnteringCreatureIsLarger"
            | "TriggeringObjectBecameTappedFirstTimeThisTurn"
            | "TriggeringObjectHadCountersPutFirstTimeThisTurn"
            | "TriggeringObjectHadToAttackThisCombat"
            | "TriggeringObjectEnlistedThisCombat"
            | "TriggeringObjectWasCast"
            | "TriggeringObjectWasCastFromZone"
            | "TriggeringObjectDied"
            | "TriggeringPlayerAttackedControllerLastTurn"
            | "TriggeringPlayersTurn"
            | "TriggeringObjectsNoneWereCastOrNoManaSpent"
            | "TriggeringAttackerBlockers"
            | "TriggeringAbilityIsManaAbility"
            | "YouWonTriggeringClash"
            | "YouChoseAnotherRingBearer"
            | "TriggeringAbilityManaSpentToActivateAtLeast"
            | "TriggeringObjectEnteredTransformed"
            | "ManaFromSourceSpentOnTriggeringAction" => scope.event,
            _ => return,
        };
        match binding {
            Binding::Present => {}
            Binding::Absent => self.finding(path, "error", "condition_without_event", format!("{name} cannot be true without its triggering-event context; this scope supplies none")),
            Binding::Unknown => self.gap(path, "condition_event_contract", format!("Cannot establish the triggering-event context required by condition {name}")),
        }
    }

    fn event_value(&mut self, spec: &Value, path: &str, scope: &Scope) {
        match enum_variant(spec).map(|(name, _)| name) {
            Some("Amount" | "LifeAmount") => self.require(path, "EventValue(Amount)", scope.amount),
            Some("LifeChange") => {
                let payload = enum_variant(spec).map(|(_, payload)| payload);
                match payload.and_then(|value| value.get("gained")).and_then(Value::as_bool) {
                    Some(true) => self.require(path, "EventValue(LifeGained)", scope.life_gain),
                    Some(false) => self.require(path, "EventValue(LifeLost)", scope.life_loss),
                    None => self.gap(path, "life_quantity_direction", "Life quantity has no typed direction"),
                }
                if payload.and_then(|value| value.get("for_controller")).and_then(Value::as_bool) == Some(true) {
                    self.require(path, "EventValue(ControllerLifeChange)", scope.life_controller);
                }
            }
            Some("DieBatchTotal" | "DieResultsAtLeast") => self.require(path, "die batch results", scope.die_batch),
            Some("DieResult") => self.require(path, "EventValue(DieResult)", scope.die_result),
            Some("BlockersBeyondFirst") => {
                self.require(path, "EventValue(BlockersBeyondFirst)", scope.blockers)
            }
            _ => self.gap(
                path,
                "event_value_schema",
                "Unknown EventValueSpec contract",
            ),
        }
    }

    fn outcome(&mut self, id: Option<u64>, path: &str, scope: &Scope) {
        let Some(id) = id else {
            self.gap(
                path,
                "effect_id_schema",
                "Cannot decode the referenced EffectId",
            );
            return;
        };
        if scope.available_outcomes.contains(&id) {
            return;
        }
        if scope.declared_outcomes.contains(&id) || scope.external_outcomes {
            self.gap(path, "outcome_dominance", format!("EffectId({id}) is not proven available on this path; it may be conditional, later, iterated, or captured"));
        } else {
            self.finding(
                path,
                "error",
                "missing_effect_outcome",
                format!(
                    "EffectId({id}) has no WithIdEffect producer in this ability or spell scope"
                ),
            );
        }
    }

    fn ability(&mut self, kind: &str, payload: &Value, path: &str) {
        let Some(fields) = payload.as_object() else {
            // Static keywords may be unit variants. They contain no context
            // references, but their gameplay semantics are outside this pass.
            if kind != "Static" {
                self.gap(path, "ability_schema", "Expected an ability payload object");
            }
            return;
        };
        let mut scope = if kind == "Triggered" {
            self.trigger(
                fields.get("trigger").unwrap_or(&Value::Null),
                &child(path, "trigger"),
            )
        } else {
            Scope::empty()
        };
        if kind != "Static" {
            scope.ability = Binding::Present;
        }
        if kind == "Static" {
            self.gap(path, "static_context_contract", "Static, continuous, and replacement payloads may create separate evaluation contexts; nested abilities are checked independently");
            scope = scope.uncertain();
        }
        collect_outcomes(payload, &mut scope.declared_outcomes, true);
        // Delegated targeting can supply a participant to both target filters
        // and later resolution, but the legal-choice contract is not modeled.
        if contains_delegated_target(payload) && scope.player != Binding::Present {
            scope.player = Binding::Unknown;
            self.gap(path, "delegated_target_context", "TargetOnlyEffect delegates a choice; participant binding depends on legal target selection");
        }
        if kind == "Static" {
            for (key, value) in fields {
                if !metadata(key) {
                    self.walk(value, &child(path, key), &mut scope);
                }
            }
        } else {
            for key in [
                "activation_condition",
                "activation_restrictions",
                "intervening_if",
                "choices",
                "mana_cost",
                "effects",
                "mana_usage_restrictions",
            ] {
                if key == "intervening_if" {
                    if let Some(condition) = fields.get(key).filter(|value| !value.is_null()) {
                        self.intervening_condition(condition, &child(path, key), &scope);
                    }
                } else {
                    self.field(fields, key, path, &mut scope);
                }
            }
            // These are declared fields of TriggeredAbility/ActivatedAbility.
            // Unknown executable fields cannot silently bypass the walker.
            for (key, value) in fields {
                if !matches!(
                    key.as_str(),
                    "trigger"
                        | "activation_condition"
                        | "activation_restrictions"
                        | "intervening_if"
                        | "choices"
                        | "mana_cost"
                        | "effects"
                        | "mana_usage_restrictions"
                ) && !metadata(key)
                {
                    self.walk(value, &child(path, key), &mut scope);
                }
            }
        }
    }

    fn intervening_condition(&mut self, value: &Value, path: &str, scope: &Scope) {
        // Trigger-time gating does not run in the later ability's context.
        // ValueComparison/ValueIsPrime explicitly reconstruct an execution
        // context from the event (condition_eval.rs), while other predicates
        // can resolve players through an ExternalEvaluationContext whose
        // iterated_player is None, or bind candidates locally inside filters.
        if let Some((name, payload)) = enum_variant(value) {
            if matches!(name, "And" | "Or" | "Not") {
                let at = child(path, name);
                if let Some(items) = payload.as_array() {
                    for (index, item) in items.iter().enumerate() {
                        self.intervening_condition(item, &child(&at, &index.to_string()), scope);
                    }
                } else {
                    self.intervening_condition(payload, &at, scope);
                }
                return;
            }
        }
        let mut external = scope.clone();
        external.ability = Binding::Absent;
        external.object = Binding::Absent;
        external.available_outcomes.clear();
        external.declared_outcomes.clear();
        external.external_outcomes = false;
        if !matches!(
            enum_variant(value).map(|(name, _)| name),
            Some("ValueComparison" | "ValueIsPrime")
        ) {
            external.player = Binding::Unknown;
            external.ability = Binding::Unknown;
        }
        self.walk(value, path, &mut external);
    }

    fn program(&mut self, fields: &Map<String, Value>, path: &str, scope: &mut Scope) {
        let Some(segments) = fields.get("segments").and_then(Value::as_array) else {
            self.gap(
                path,
                "resolution_program_schema",
                "ResolutionProgram.segments is not an array",
            );
            return;
        };
        for (index, segment) in segments.iter().enumerate() {
            let at = child(&child(path, "segments"), &index.to_string());
            let Some(fields) = segment.as_object() else {
                self.gap(
                    &at,
                    "resolution_segment_schema",
                    "Expected a resolution segment object",
                );
                continue;
            };
            let before = scope.clone();
            self.field(fields, "default_effects", &at, scope);
            if let Some(branches) = fields.get("self_replacements").and_then(Value::as_array) {
                for (index, branch) in branches.iter().enumerate() {
                    let mut replacement = before.clone();
                    self.walk(
                        branch,
                        &child(&child(&at, "self_replacements"), &index.to_string()),
                        &mut replacement,
                    );
                    *scope = scope.intersect(&replacement);
                }
            }
        }
    }

    fn effect(&mut self, kind: &str, payload: &Value, path: &str, scope: &mut Scope) {
        let Some(fields) = payload.as_object() else {
            // Known unit effects have no input references. An unrecognized
            // envelope still needs review even when its payload is null.
            if !known_effect(kind) {
                self.gap(
                    path,
                    "unknown_effect_contract",
                    format!("No context contract for effect {kind}"),
                );
            }
            return;
        };
        match kind {
            "TagTriggeringObjectEffect" => {
                self.require(
                    path,
                    "TagTriggeringObjectEffect triggering event",
                    scope.event,
                );
                if scope.event == Binding::Present {
                    self.require(
                        path,
                        "TagTriggeringObjectEffect event object",
                        scope.event_object,
                    );
                }
                for (key, value) in fields {
                    if !metadata(key) {
                        self.walk(value, &child(path, key), scope);
                    }
                }
            }
            "WithIdEffect" => {
                self.field(fields, "effect", path, scope);
                if let Some(id) = fields.get("id").and_then(Value::as_u64) {
                    scope.available_outcomes.insert(id);
                } else {
                    self.gap(path, "effect_id_schema", "WithIdEffect has no numeric id");
                }
            }
            "TaggedEffect"
            | "CollectManaPaymentsEffect"
            | "BindXValueEffect"
            | "SequenceEffect"
            | "ManaRetainedEffect"
            | "ExecuteWithSourceEffect" => {
                for (key, value) in fields {
                    if !metadata(key) {
                        self.walk(value, &child(path, key), scope);
                    }
                }
            }
            "ForPlayersEffect"
            | "ForEachObject"
            | "ForEachTaggedEffect"
            | "ForEachControllerOfTaggedEffect"
            | "ForEachTaggedPlayerEffect" => {
                for (key, value) in fields {
                    if key != "effects" && !metadata(key) {
                        self.walk(value, &child(path, key), scope);
                    }
                }
                let mut inner = scope.clone();
                inner.player = Binding::Present;
                if matches!(kind, "ForEachObject" | "ForEachTaggedEffect") {
                    inner.object = Binding::Present;
                }
                self.field(fields, "effects", path, &mut inner);
                // A loop may execute zero times; its local bindings/outcomes
                // do not become guaranteed providers after the loop.
            }
            "ConditionalEffect" => {
                self.field(fields, "condition", path, scope);
                let mut yes = scope.clone();
                let mut no = scope.clone();
                self.field(fields, "if_true", path, &mut yes);
                self.field(fields, "if_false", path, &mut no);
                *scope = yes.intersect(&no);
            }
            "IfEffect" => {
                self.outcome(
                    fields.get("condition").and_then(Value::as_u64),
                    &child(path, "condition"),
                    scope,
                );
                self.field(fields, "predicate", path, scope);
                let mut inner = scope.clone();
                // IfEffect only establishes a player when its antecedent
                // actually emits PlayerCounts, not merely because it is an if.
                if inner.player != Binding::Present {
                    inner.player = Binding::Unknown;
                }
                let mut yes = inner.clone();
                let mut no = inner;
                self.field(fields, "then", path, &mut yes);
                self.field(fields, "else_", path, &mut no);
                scope.available_outcomes = yes
                    .available_outcomes
                    .intersection(&no.available_outcomes)
                    .copied()
                    .collect();
            }
            "MayEffect"
            | "UnlessPaysEffect"
            | "UnlessActionEffect"
            | "RepeatEffectsEffect"
            | "RepeatProcessEffect"
            | "ChooseModeEffect"
            | "CumulativeUpkeepEffect"
            | "VillainousChoiceEffect" => {
                // Branches/repetitions inherit existing references, but an
                // outcome produced only in an optional branch cannot dominate
                // a following instruction.
                for (key, value) in fields {
                    if !metadata(key) {
                        self.walk(value, &child(path, key), &mut scope.clone());
                    }
                }
            }
            "ReflexiveTriggerEffect" => {
                self.outcome(
                    fields.get("condition").and_then(Value::as_u64),
                    &child(path, "condition"),
                    scope,
                );
                self.field(fields, "predicate", path, scope);
                let mut inner = scope.clone();
                inner.external_outcomes = true;
                self.field(fields, "choices", path, &mut inner);
                self.field(fields, "effects", path, &mut inner);
            }
            "ScheduleDelayedTriggerEffect" => {
                for (key, value) in fields {
                    if !matches!(key.as_str(), "effects" | "trigger" | "target_choices")
                        && !metadata(key)
                    {
                        self.walk(value, &child(path, key), scope);
                    }
                }
                let mut delayed = self.trigger(
                    fields.get("trigger").unwrap_or(&Value::Null),
                    &child(path, "trigger"),
                );
                delayed.ability = Binding::Present;
                delayed.external_outcomes = true;
                collect_outcomes(payload, &mut delayed.declared_outcomes, true);
                if fields
                    .get("event_value_from_prior_prevention")
                    .and_then(Value::as_bool)
                    == Some(true)
                {
                    delayed.amount = Binding::Present;
                }
                self.field(fields, "target_choices", path, &mut delayed);
                self.field(fields, "effects", path, &mut delayed);
                self.gap(path, "delayed_capture_contract", "Delayed body checked in its own event scope; captured targets, tags, outcomes, and registration lifetime require execution");
            }
            "ScheduleEffectsWhenTaggedLeavesEffect" | "HauntExileEffect" => {
                for (key, value) in fields {
                    if matches!(key.as_str(), "effects" | "haunt_effects" | "haunt_choices") {
                        self.walk(value, &child(path, key), &mut Scope::empty().uncertain());
                    } else if !metadata(key) {
                        self.walk(value, &child(path, key), scope);
                    }
                }
                self.gap(path, "delayed_capture_contract", format!("{kind} creates a later execution scope whose captured state is not fully modeled"));
            }
            "TargetOnlyEffect" => {
                self.field(fields, "chooser", path, scope);
                if fields.get("chooser").is_some_and(|value| !value.is_null()) {
                    let mut inner = scope.clone();
                    if inner.player != Binding::Present {
                        inner.player = Binding::Unknown;
                    }
                    self.field(fields, "target", path, &mut inner);
                    scope.player = inner.player;
                    self.gap(
                        path,
                        "delegated_target_context",
                        "Delegated target binding depends on the legal-choice path",
                    );
                } else {
                    self.field(fields, "target", path, scope);
                }
            }
            "ChooseObjectsEffect" => {
                for (key, value) in fields {
                    if metadata(key) {
                        continue;
                    }
                    let mut inner = scope.clone();
                    if matches!(key.as_str(), "count_value" | "filter")
                        && fields
                            .get("chooser")
                            .is_some_and(|chooser| chooser.get("Target").is_some())
                    {
                        inner.player = Binding::Present;
                    }
                    self.walk(value, &child(path, key), &mut inner);
                }
            }
            "PreventDamageEffect" | "PreventAllDamageToTargetEffect" | "PreventAllDamageEffect" => {
                for (key, value) in fields {
                    if metadata(key) {
                        continue;
                    }
                    if key == "follow_up_effects" {
                        // Prevention follow-ups execute when the shield is
                        // applied, paired with the actual prevented-damage
                        // event, not in the spell that registered the shield.
                        let mut follow_up = Scope::empty();
                        follow_up.event = Binding::Present;
                        follow_up.event_object = Binding::Present;
                        follow_up.amount = Binding::Present;
                        follow_up.player = Binding::Unknown;
                        follow_up.damaged_player = Binding::Unknown;
                        collect_outcomes(value, &mut follow_up.declared_outcomes, true);
                        self.walk(value, &child(path, key), &mut follow_up);
                    } else {
                        self.walk(value, &child(path, key), scope);
                    }
                }
            }
            "SecretChoiceEffect" => {
                for (key, value) in fields {
                    if metadata(key) {
                        continue;
                    }
                    let mut inner = scope.clone();
                    if key == "object_choice" {
                        inner.player = Binding::Present;
                    }
                    self.walk(value, &child(path, key), &mut inner);
                }
            }
            "CreateTokenEffect"
            | "CreateTokenCopyEffect"
            | "CreateEmblemEffect"
            | "GrantAbilitiesTargetEffect"
            | "GrantNextSpellAbilityEffect"
            | "BackupEffect" => {
                // Nested ability/definition recognizers in walk reset scope.
                for (key, value) in fields {
                    if !metadata(key) {
                        self.walk(value, &child(path, key), scope);
                    }
                }
            }
            "SkipTurnEffect" => {
                // The runtime intentionally falls back to its explicit target
                // for this effect's otherwise unbound IteratedPlayer filter.
                let mut inner = scope.clone();
                inner.player = Binding::Unknown;
                self.gap(
                    path,
                    "effect_local_fallback",
                    "SkipTurnEffect has an effect-specific target fallback requiring execution",
                );
                self.walk(payload, path, &mut inner);
            }
            _ if same_scope_effect(kind) => {
                for (key, value) in fields {
                    if !metadata(key) {
                        self.walk(value, &child(path, key), scope);
                    }
                }
            }
            _ => {
                self.gap(
                    path,
                    "unknown_effect_contract",
                    format!("No reviewed context/child-scope contract for effect {kind}"),
                );
                self.walk(payload, path, &mut scope.uncertain());
            }
        }
    }

    fn trigger(&mut self, value: &Value, path: &str) -> Scope {
        let kind = value.get("kind").unwrap_or(value);
        let Some((name, payload)) = enum_variant(kind) else {
            self.gap(
                path,
                "unknown_trigger_contract",
                "Cannot decode the trigger kind",
            );
            return Scope::empty().uncertain();
        };
        if name == "ConditionQualified" {
            let scope = self.trigger(
                payload.get("trigger").unwrap_or(&Value::Null),
                &child(path, "trigger"),
            );
            if let Some(condition) = payload.get("condition") {
                self.intervening_condition(condition, &child(path, "condition"), &scope);
            }
            return scope;
        }
        // A zone gate only restricts where one union arm functions.
        if name == "ZoneGated" {
            return self.trigger(
                payload.get("trigger").unwrap_or(&Value::Null),
                &child(path, "trigger"),
            );
        }
        if name == "AnyOf" || name == "Either" {
            let branches: Vec<&Value> = if name == "AnyOf" || payload.is_array() {
                payload
                    .as_array()
                    .map(|items| items.iter().collect())
                    .unwrap_or_default()
            } else {
                [payload.get("left"), payload.get("right")]
                    .into_iter()
                    .flatten()
                    .collect()
            };
            if branches.is_empty() {
                self.gap(
                    path,
                    "unknown_trigger_contract",
                    "Trigger union has no decodable branches",
                );
                return Scope::empty().uncertain();
            }
            let mut scopes = branches
                .into_iter()
                .enumerate()
                .map(|(index, branch)| self.trigger(branch, &child(path, &index.to_string())))
                .collect::<Vec<_>>();
            let first = scopes.remove(0);
            return scopes
                .iter()
                .fold(first, |left, right| left.intersect(right));
        }
        let mut scope = Scope::empty();
        scope.event = Binding::Present;
        scope.event_object = Binding::Unknown;
        let gain = matches!(name, "PlayerGainsLife" | "YouGainLife" | "YouGainLifeCausedBy" | "YouGainLifeDuringTurn")
            || (name == "LifeChanged" && payload.get("gained").and_then(Value::as_bool) == Some(true));
        let loss = matches!(name, "PlayerLosesLife" | "PlayerLosesLifeDuringTurn")
            || (name == "LifeChanged" && payload.get("gained").and_then(Value::as_bool) == Some(false));
        if gain || loss {
            scope.life_gain = if gain { Binding::Present } else { Binding::Absent };
            scope.life_loss = if loss { Binding::Present } else { Binding::Absent };
            scope.life_controller = if name.starts_with("YouGainLife") || payload.get("player").is_some_and(|player| player == "You") {
                Binding::Present
            } else { Binding::Unknown };
        }
        let mut known = true;
        match name {
            "PlayerGainsLife"
            | "LifeChanged"
            | "PlayerLosesLife"
            | "PlayerPaysLife"
            | "PlayersLoseLifeOneOrMore"
            | "OpponentsEachLoseExactLife"
            | "PlayerLosesLifeDuringTurn"
            | "YouGainLife"
            | "YouGainLifeCausedBy"
            | "YouGainLifeDuringTurn" => {
                scope.player = Binding::Present;
                scope.amount = Binding::Present;
                scope.event_object = Binding::Absent;
            }
            "ThisDealsDamageToPlayer"
            | "ThisDealsCombatDamageToPlayer"
            | "DealsDamageToPlayer"
            | "DealsNoncombatDamageToPlayer"
            | "DealsCombatDamageToPlayer"
            | "DealsCombatDamageToPlayerOneOrMore" => {
                scope.player = Binding::Present;
                scope.amount = Binding::Present;
                scope.damaged_player = Binding::Present;
                scope.event_object = Binding::Present;
            }
            "ThisDealsDamage"
            | "ThisDealsDamageTo"
            | "ThisDealsCombatDamage"
            | "ThisDealsCombatDamageTo"
            | "DealsDamage"
            | "DealsDamageTo"
            | "DealsCombatDamage"
            | "DealsCombatDamageTo"
            | "DealsExactDamageToObjectOrPlayer" => {
                scope.player = Binding::Unknown;
                scope.amount = Binding::Present;
                scope.damaged_player = Binding::Unknown;
                scope.event_object = Binding::Present;
            }
            "IsDealtDamage" => {
                scope.amount = Binding::Present;
                scope.event_object = Binding::Present; // DamageEvent names its source.
                scope.player = if payload.get("target").is_some_and(|target|
                    target.get("Player").is_some() || target.get("SpecificPlayer").is_some()
                        || target == "SourceController" || target == "SourceOwner") {
                    Binding::Present
                } else { Binding::Unknown };
            }
            "PlayerRollsResultMatching" | "PlayerRollsResult"
            | "PlayerRollsHighestNaturalResult"
            | "PlayerRollsToVisitAttractions" => {
                scope.player = Binding::Present;
                scope.die_result = Binding::Present;
                scope.event_object = Binding::Present;
            }
            "PlayerRollsNthDie" => { scope.player = Binding::Present; }
            "PlayerRollsDie" => {
                scope.player = Binding::Present;
                scope.die_batch = if payload.get("one_or_more").and_then(Value::as_bool) == Some(true) { Binding::Present } else { Binding::Absent };
                // This trigger also sees planar dice; DieResult rejects those.
                scope.die_result = Binding::Unknown;
            }
            "SpellCast" | "SpellCastQualified" | "SpellCastSameNameCardInZone" => {
                scope.player = Binding::Present;
                scope.cast_event = Binding::Present;
                scope.event_object = Binding::Present;
                // Some spell-matchers compute target cardinality as an amount.
                scope.amount = Binding::Unknown;
            }
            "KeywordAction"
            | "KeywordActionDuringYourTurn"
            | "KeywordActionFromSource"
            | "KeywordActionMatchingObject"
            | "KeywordActionMatchingObjectOneOrMore"
            | "KeywordActionMatchingObjectDuringYourTurn"
            | "KeywordActionMatchingTaggedObject"
            | "WinsClash"
            | "Expend"
            | "ClassBecomesLevel" => {
                scope.player = Binding::Present;
                scope.amount = Binding::Present;
            }
            "PermanentTransforms" | "PermanentTransformsInto" => {
                scope.event_object = Binding::Present;
            }
            "PermanentMutates" | "PlayerTurnsFaceUp" => {
                // A completed mutation names the permanent and its controller;
                // active face-up names the exact permanent and acting player.
                scope.event_object = Binding::Present;
                scope.player = Binding::Present;
            }
            "PlayerAttackDeclaration" | "RingBearerChosen" => { scope.player = Binding::Present; }
            "BecomesTargetedByAbilitySource" => {
                scope.player = Binding::Present;
                scope.event_object = Binding::Present;
            }
            "PlayerBecomesMonarch" => {scope.player=Binding::Present;scope.event_object=Binding::Absent;}
            "PlayerBecomesTargeted" => {
                // BecomesTargetedEvent::player is the captured source controller;
                // the player target does not invent an event object.
                scope.player = Binding::Present;
                scope.event_object = Binding::Absent;
            }
            "CardsMilled" => {
                scope.player = Binding::Present;
                scope.amount = Binding::Present;
                // Hidden replacement destinations cannot expose a card.
                scope.event_object = Binding::Unknown;
            }
            "PhasingChanged" => {
                scope.event_object = Binding::Present;
                scope.amount = Binding::Present;
            }
            "ControlChanged" => {
                // The typed matcher requires exact transition/departure
                // snapshots. Loss clauses do not bind the event's new player.
                scope.event_object = Binding::Present;
            }
            "AttachmentChanged" => {
                // Both attachment and recipient snapshots are mandatory in
                // the typed matcher; the recipient is the body event object.
                scope.event_object = Binding::Present;
            }
            "PlayerChangesTapState" => {
                // The matcher requires an explicit event actor and an origin
                // snapshot. It supplies 1 per transition; simultaneous queues
                // sum that amount for a one-or-more event.
                scope.player = Binding::Present;
                scope.amount = Binding::Present;
                scope.event_object = Binding::Present;
            }
            "PermanentBecomesUntapped" => {
                scope.player = Binding::Unknown;
                scope.amount = Binding::Present;
                scope.event_object = Binding::Present;
            }
            "BeginningOfUpkeep"
            | "BeginningOfDrawStep"
            | "BeginningOfCombat"
            | "BeginningOfEndStep"
            | "BeginningOfMainPhase"
            | "BeginningOfPrecombatMainPhase"
            | "BeginningOfPostcombatMainPhase"
            | "BeginningOfCleanupStep"
            | "BeginningOfNextCleanupStep"
            | "AsPermanentsUntap"
            | "EndOfCombat"
            | "PlayerLosesGame"
            | "PlayerPlaysLand"
            | "PlayerSearchesLibrary"
            | "PlayerShufflesLibrary"
            | "PlayerTapsForMana"
            | "PlayerGivesGift"
            | "PlayerCoinFlipResult"
            | "SpellCountered"
            | "SpellCopied"
            | "YouCastThisSpell"
            | "NthSpellOfTurnCast"
            | "PlayerRevealsCard"
            | "AbilityActivated"
            | "AbilityActivatedQualified"
            | "AbilityTriggered"
            | "PlayerSacrifices"
            | "PermanentSacrificed"
            | "BecomesTapped"
            | "PermanentBecomesTapped"
            | "BecomesUntapped"
            | "ThisIsTurnedFaceUp"
            | "TurnedFaceUp"
            | "ThisMutates"
            | "ThisBecomesMonstrous" => {
                scope.player = Binding::Present;
                if matches!(name, "YouCastThisSpell" | "NthSpellOfTurnCast") {
                    scope.cast_event = Binding::Present;
                }
            }
            "PermanentDestroyed" => {
                // Successful native matching requires a retained destroyed
                // permanent snapshot and a completed nonbattlefield result.
                scope.event_object = Binding::Present;
            }
            "PlayerDiscardsCard"
            | "PlayerDiscardsCardCausedByController"
            | "YouDrawCard"
            | "Miracle"
            | "PlayerDrawsCardDuringTurn"
            | "PlayerDrawsFirstCardInOwnDrawStep"
            | "PlayerDrawsCard"
            | "PlayerDrawsCardNotDuringTurn"
            | "PlayerDrawsCardExceptFirstInDrawStep"
            | "PlayerDrawsNthCardEachTurn"
            | "PlayerDrawsNumberedCardsEachTurn"
            | "TokensCreated" => {
                scope.player = Binding::Present;
                scope.amount = Binding::Unknown;
            }
            "ThisBecomesBlocked"
            | "BecomesBlocked"
            | "BecomesBlockedOneOrMore"
            | "ThisBecomesBlockedByObject"
            | "BecomesBlockedByObjectWithLesserPower" => {
                scope.player = Binding::Unknown;
                scope.blockers = Binding::Present;
            }
            "ThisAttacks"
            | "ThisAttacksWhileYouControl"
            | "ThisAndAnotherAttackDifferentPlayers"
            | "ThisAttacksPlayerWhoControlsAtLeast"
            | "ThisAttacksPlayerWithMostLife"
            | "ThisAttacksWithGreaterPower"
            | "ThisAttacksWithNOthers"
            | "ThisAttacksWithExactNOthers"
            | "ThisAttacksAndIsntBlocked"
            | "ThisAttacksWhileSaddled"
            | "Attacks"
            | "AttacksAndIsntBlocked"
            | "AttacksAndIsntBlockedOneOrMore"
            | "AttacksWhileSaddled"
            | "AttacksOneOrMore"
            | "PlayersAttackedOneOrMore"
            | "PlayerAttacksOneOrMore"
            | "PlayerAttacksTargetWithOneOrMore"
            | "AttacksOneOrMoreWithMinTotal"
            | "AttacksOneOrMoreWithExactTotal"
            | "AttacksOneOrMoreWithAggregate"
            | "AttacksAlone"
            | "AttacksPlayerAlone"
            | "AttacksYou"
            | "AttacksYouOneOrMore"
            | "ThisBlocks"
            | "ThisBlocksObject"
            | "Blocks"
            | "BlocksOneOrMore"
            | "BlocksOrBecomesBlockedByObject"
            | "BlocksObjectWithLesserPower"
            | "BlocksObject" => {
                scope.player = Binding::Unknown;
                scope.amount = Binding::Unknown;
            }
            "ThisDies"
            | "Dies"
            | "PutIntoGraveyard"
            | "PutIntoGraveyardFromZone"
            | "DiesCreatureDealtDamageByThisTurn"
            | "DiesCreatureDealtDamageByFilteredSourceThisTurn" => {
                // Graveyard events bind the owner when they retain a snapshot.
                scope.player = Binding::Unknown;
                scope.amount = Binding::Present;
            }
            "EntersBattlefield" | "ThisEntersBattlefield" | "ThisEntersBattlefieldWithSurface" => {
                // Both EnterBattlefieldEvent and a ZoneChangeEvent to the
                // battlefield expose an object, but neither binds a player.
                // Only the zone-change representation exposes numeric amount.
                scope.amount = Binding::Unknown;
                scope.event_object = Binding::Present;
            }
            "ThisLeavesBattlefield"
            | "LeavesBattlefield"
            | "CardsLeaveYourGraveyard"
            | "ThisDiesOrIsExiled"
            | "ThisDiesOrIsExiledWithSurface" => {
                scope.player = Binding::Unknown;
                scope.amount = Binding::Present;
            }
            "ZoneChange" => {
                // ZoneChangeEvent::trigger_player only binds an owner for a
                // graveyard destination, and then only with retained LKI.
                scope.player = match payload.get("to").and_then(Value::as_str) {
                    Some("Graveyard") | None => Binding::Unknown,
                    Some(_) => Binding::Absent,
                };
                scope.amount = Binding::Present;
                scope.event_object = Binding::Present;
            }
            "PlayerGetsCounters" | "CounterPutOn" | "NthCounterPutOn" | "CounterRemovedFrom" => {
                scope.player = Binding::Unknown;
                scope.amount = Binding::Present;
            }
            "StateBased"
            | "DayNightChanged"
            | "ThisTransforms"
            | "ThisTransformsWithSurface"
            | "ThisPhasesOut"
            | "FinalChapterAbilityResolved" => {}
            "SagaChapter" => {
                // The matcher accepts CounterPlaced or MarkersChanged only
                // for the source Saga object. Both expose that object and
                // amount; neither runtime event binds IteratedPlayer.
                scope.amount = Binding::Present;
                scope.event_object = Binding::Present;
            }
            // These trigger builders synthesize events or select participants
            // in matcher-specific ways; an enum name alone is insufficient.
            "DungeonRoom"
            | "BecomesTargeted"
            | "BecomesTargetedObject"
            | "BecomesTargetedBySpell"
            | "BecomesTargetedByStackObject"
            | "BecomesTargetedObjectByStackObject"
            | "BecomesTargetedBySourceController"
            | "PlayerOrObjectBecomesTargetedBySourceController"
            | "SourceControllerLosesControl"
            | "Custom" => {
                known = false;
            }
            _ => {
                known = false;
            }
        }
        if matches!(
            name,
            "BeginningOfUpkeep"
                | "BeginningOfDrawStep"
                | "BeginningOfCombat"
                | "BeginningOfEndStep"
                | "BeginningOfMainPhase"
                | "BeginningOfPrecombatMainPhase"
                | "BeginningOfPostcombatMainPhase"
                | "BeginningOfCleanupStep"
                | "BeginningOfNextCleanupStep"
                | "AsPermanentsUntap"
                | "EndOfCombat"
                | "PlayerCoinFlipResult"
        ) {
            scope.event_object = Binding::Absent;
        }
        if !known {
            self.gap(
                path,
                "unknown_trigger_contract",
                format!("No complete event-context contract for trigger {name}"),
            );
            scope = scope.uncertain();
        }
        // Trigger filters run in matcher/filter context, before a triggered
        // ability receives its resolution context. Inspect them too, without
        // treating the eventual event binding as proof they are available
        // during matching.
        self.walk(payload, &child(path, name), &mut Scope::empty().uncertain());
        scope
    }
}

fn child(path: &str, key: &str) -> String {
    format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"))
}

fn enum_variant(value: &Value) -> Option<(&str, &Value)> {
    if let Some(name) = value.as_str() {
        return Some((name, &Value::Null));
    }
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    fields
        .iter()
        .next()
        .map(|(name, payload)| (name.as_str(), payload))
}

fn metadata(key: &str) -> bool {
    matches!(
        key,
        "name"
            | "label"
            | "display"
            | "text"
            | "oracle_text"
            | "canonical_text"
            | "ability_labels"
            | "presentation_label"
            | "source_text"
            | "description"
            | "surface"
            | "result_label"
            | "intro_surface"
            | "tag"
            | "result_tag"
            | "source_binding_tag"
            | "result_binding_tag"
            | "object_tag"
            | "target_tag"
            | "destination_name"
            | "display_subject"
            | "additional_restrictions"
            | "flattened_default_effects"
    ) || key.ends_with("_surface")
        || key.ends_with("_description")
}

fn is_pending_value(name: &str) -> bool {
    matches!(
        name,
        "PendingEffectMetric"
            | "PendingEffectMetricOffset"
            | "PendingPriorEffectMetric"
            | "PendingComparisonLeft"
            | "PendingComparisonRight"
            | "PendingComparisonDifference"
    )
}

fn collect_outcomes(value: &Value, out: &mut BTreeSet<u64>, at_root: bool) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_outcomes(item, out, false);
            }
        }
        Value::Object(fields) => {
            if !at_root
                && ((fields.contains_key("card") && fields.contains_key("abilities"))
                    || fields.get("kind").is_some_and(|kind| {
                        enum_variant(kind).is_some_and(|(name, _)| {
                            matches!(name, "Triggered" | "Activated" | "Static")
                        })
                    }))
            {
                return;
            }
            if fields.get("kind").and_then(Value::as_str) == Some("WithIdEffect")
                && let Some(id) = fields
                    .get("payload")
                    .and_then(|p| p.get("id"))
                    .and_then(Value::as_u64)
            {
                out.insert(id);
            }
            if !at_root
                && matches!(
                    fields.get("kind").and_then(Value::as_str),
                    Some(
                        "ScheduleDelayedTriggerEffect"
                            | "ScheduleEffectsWhenTaggedLeavesEffect"
                            | "HauntExileEffect"
                            | "ReflexiveTriggerEffect"
                    )
                )
            {
                return;
            }
            for (key, value) in fields {
                if !metadata(key) {
                    collect_outcomes(value, out, false);
                }
            }
        }
        _ => {}
    }
}

fn contains_delegated_target(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(contains_delegated_target),
        Value::Object(fields) => {
            if fields.get("kind").and_then(Value::as_str) == Some("TargetOnlyEffect")
                && fields
                    .get("payload")
                    .and_then(|p| p.get("chooser"))
                    .is_some_and(|v| !v.is_null())
            {
                return true;
            }
            fields
                .iter()
                .filter(|(key, _)| !metadata(key))
                .any(|(_, value)| contains_delegated_target(value))
        }
        _ => false,
    }
}

fn known_effect(kind: &str) -> bool {
    // This compile-time registry distinguishes an unfamiliar effect envelope
    // from an ordinary unit effect without linking a decoder into the audit.
    include_str!("../../../ironsmith-artifact-effect-decoder/effect-registry.tsv")
        .lines()
        .any(|line| line.split('\t').next() == Some(kind))
}

fn same_scope_effect(kind: &str) -> bool {
    matches!(
        kind,
        "AddManaEffect"
            | "AddManaOfAnyColorEffect"
            | "AddManaOfAnyOneColorEffect"
            | "AddScaledManaEffect"
            | "GainLifeEffect"
            | "LoseLifeEffect"
            | "SetLifeTotalEffect"
            | "ExchangeLifeTotalsEffect"
            | "DrawCardsEffect"
            | "DiscardEffect"
            | "DiscardHandEffect"
            | "MillEffect"
            | "ScryEffect"
            | "SurveilEffect"
            | "FatesealEffect"
            | "DealDamageEffect"
            | "DealDamageBySourcesEffect"
            | "DealDamageEachEffect"
            | "DealDistributedDamageEffect"
            | "HealDamageEffect"
            | "PreventDamageEffect"
            | "PutCountersEffect"
            | "RemoveCountersEffect"
            | "RemoveUpToCountersEffect"
            | "DoubleCountersEffect"
            | "PoisonCountersEffect"
            | "TapEffect"
            | "UntapEffect"
            | "DestroyEffect"
            | "DestroyNoRegenerationEffect"
            | "CounterEffect"
            | "ExileEffect"
            | "ExileTopOfLibraryEffect"
            | "MoveToZoneEffect"
            | "ReturnToHandEffect"
            | "ReturnFromGraveyardToHandEffect"
            | "MoveToLibraryNthFromTopEffect"
            | "MoveToLibraryTopOrBottomChoiceEffect"
            | "PutOntoBattlefieldEffect"
            | "SacrificeEffect"
            | "SacrificeTargetEffect"
            | "SearchLibraryEffect"
            | "SearchLibrarySlotsEffect"
            | "ShuffleLibraryEffect"
            | "ShuffleGraveyardIntoLibraryEffect"
            | "RevealTaggedEffect"
            | "RevealSourceFromHandEffect"
            | "RevealFromHandEffect"
            | "LookAtHandEffect"
            | "LookAtObjectsEffect"
            | "TagMatchingObjectsEffect"
            | "ModifyPowerToughnessEffect"
            | "FightEffect"
            | "GoadEffect"
            | "MustAttackPlayerThisTurnEffect"
            | "DetainEffect"
            | "AttachObjectsEffect"
            | "AttachToEffect"
            | "ChoosePlayerEffect"
            | "ChooseNumberEffect"
            | "ChooseNumberAtRandomEffect"
            | "ChooseCardNameEffect"
            | "ChangeTextEffect"
            | "ChooseColorEffect"
            | "DrawForEachTaggedMatchingEffect"
            | "FlipCoinEffect"
            | "RollDiceEffect"
            | "RollDiceChooseResultEffect"
            | "EmitKeywordActionEffect"
            | "GrantNextSpellCostReductionEffect"
            | "AdditionalLandPlaysEffect"
            | "LoseTheGameEffect"
            | "WinTheGameEffect"
    )
}

#[cfg(test)]
#[path = "contract_tests.rs"]
mod tests;
