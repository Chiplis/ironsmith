//! Shared delayed-trigger queue primitives.

use std::collections::HashMap;

use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::resolution::ResolutionProgram;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::triggers::{DelayedTrigger, Trigger};

pub(crate) fn tagged_collection_has_object_in_zone(
    game: &GameState,
    tagged_objects: &HashMap<TagKey, Vec<ObjectSnapshot>>,
    tag: &TagKey,
    zone: crate::zone::Zone,
) -> bool {
    tagged_objects.get(tag).is_some_and(|snapshots| {
        snapshots.iter().any(|snapshot| {
            game.find_object_by_stable_id(snapshot.stable_id)
                .or_else(|| game.object(snapshot.object_id).map(|_| snapshot.object_id))
                .and_then(|object_id| game.object(object_id))
                .is_some_and(|object| object.zone == zone)
        })
    })
}

/// Config used to enqueue a delayed trigger.
#[derive(Debug, Clone)]
pub struct DelayedTriggerConfig {
    pub trigger: Trigger,
    pub persistent_player_rule: Option<String>,
    pub intervening_if: Option<crate::effect::Condition>,
    pub effects: ResolutionProgram,
    pub linked_exile_owner: Option<crate::linked_exile::LinkedExileOwner>,
    pub source_number_owner: Option<crate::linked_exile::LinkedExileOwner>,
    pub one_shot: bool,
    pub not_before_turn: Option<u32>,
    pub expires_at_turn: Option<u32>,
    /// Registration expires before events on its controller's first turn
    /// whose turn number is greater than this anchor.
    pub expires_before_controller_turn_after: Option<u32>,
    pub expires_after_controller_turn_after: Option<u32>,
    pub expires_at_end_of_combat: bool,
    pub bound_extra_turn_index: Option<usize>,
    pub while_any_tagged_object_in_zone: Option<(TagKey, crate::zone::Zone)>,
    pub source_collection_scope: Option<ironsmith_core::effect::SourceCollectionScope>,
    /// "For as long as this creature remains tapped": the registration ends
    /// once this permanent untaps or leaves the battlefield (CR 611.2b).
    pub while_source_tapped: Option<ObjectId>,
    pub target_objects: Vec<ObjectId>,
    pub ability_source: Option<ObjectId>,
    pub controller: PlayerId,
    pub x_value: Option<u32>,
    pub choices: Vec<crate::target::ChooseSpec>,
    pub tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    pub tagged_players: HashMap<TagKey, Vec<PlayerId>>,
    pub defending_player_reference: Option<crate::combat_state::DefendingPlayerReference>,
    pub prepayment: Option<crate::triggers::PendingDelayedTriggerPayment>,
    pub prevention_shield: Option<crate::prevention::PreventionShieldId>,
}

impl DelayedTriggerConfig {
    pub fn new(
        trigger: Trigger,
        effects: impl Into<ResolutionProgram>,
        one_shot: bool,
        target_objects: Vec<ObjectId>,
        controller: PlayerId,
    ) -> Self {
        Self {
            trigger,
            persistent_player_rule: None,
            intervening_if: None,
            effects: effects.into(),
            linked_exile_owner: None,
            source_number_owner: None,
            one_shot,
            not_before_turn: None,
            expires_at_turn: None,
            expires_before_controller_turn_after: None,
            expires_after_controller_turn_after: None,
            expires_at_end_of_combat: false,
            bound_extra_turn_index: None,
            while_any_tagged_object_in_zone: None,
            source_collection_scope: None,
            while_source_tapped: None,
            target_objects,
            ability_source: None,
            controller,
            x_value: None,
            choices: Vec::new(),
            tagged_objects: HashMap::new(),
            tagged_players: HashMap::new(),
            defending_player_reference: None,
            prepayment: None,
            prevention_shield: None,
        }
    }

    pub fn as_persistent_player_rule(mut self, name: String, condition: Option<crate::effect::Condition>) -> Self {
        self.persistent_player_rule = Some(name);
        self.intervening_if = condition;
        self.one_shot = false;
        self
    }

    pub fn with_not_before_turn(mut self, not_before_turn: Option<u32>) -> Self {
        self.not_before_turn = not_before_turn;
        self
    }

    pub fn with_expires_at_turn(mut self, expires_at_turn: Option<u32>) -> Self {
        self.expires_at_turn = expires_at_turn;
        self
    }

    pub fn with_expires_before_controller_turn_after(mut self, anchor_turn: Option<u32>) -> Self {
        self.expires_before_controller_turn_after = anchor_turn;
        self
    }

    pub fn with_expires_after_controller_turn_after(mut self, anchor_turn: Option<u32>) -> Self {
        self.expires_after_controller_turn_after = anchor_turn;
        self
    }

    pub fn with_expires_at_end_of_combat(mut self, expires: bool) -> Self {
        self.expires_at_end_of_combat = expires;
        self
    }

    pub fn with_bound_extra_turn_index(mut self, index: Option<usize>) -> Self {
        self.bound_extra_turn_index = index;
        self
    }

    pub fn while_any_tagged_object_in_zone_opt(
        mut self,
        duration: Option<(TagKey, crate::zone::Zone)>,
    ) -> Self {
        self.while_any_tagged_object_in_zone = duration;
        self
    }

    pub fn while_source_tapped(mut self, source: Option<ObjectId>) -> Self {
        self.while_source_tapped = source;
        self
    }

    pub fn with_ability_source(mut self, ability_source: Option<ObjectId>) -> Self {
        self.ability_source = ability_source;
        self
    }

    pub fn with_linked_exile_owner(mut self, owner: Option<crate::linked_exile::LinkedExileOwner>) -> Self {
        self.linked_exile_owner = owner;
        self
    }

    pub fn with_source_number_owner(mut self, owner: Option<crate::linked_exile::LinkedExileOwner>) -> Self {
        self.source_number_owner = owner;
        self
    }

    pub fn with_x_value(mut self, x_value: Option<u32>) -> Self {
        self.x_value = x_value;
        self
    }

    pub fn with_choices(mut self, choices: Vec<crate::target::ChooseSpec>) -> Self {
        self.choices = choices;
        self
    }

    pub fn with_tagged_objects(
        mut self,
        tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    ) -> Self {
        self.tagged_objects = tagged_objects;
        self
    }

    pub fn with_tagged_players(mut self, tagged_players: HashMap<TagKey, Vec<PlayerId>>) -> Self {
        self.tagged_players = tagged_players;
        self
    }

    pub fn with_defending_player_reference(mut self, reference: Option<crate::combat_state::DefendingPlayerReference>) -> Self {
        self.defending_player_reference = reference;
        self
    }

    pub fn with_source_collection_scope(mut self, scope: Option<ironsmith_core::effect::SourceCollectionScope>) -> Self {
        self.source_collection_scope = scope; self
    }

    pub fn with_prepayment(
        mut self,
        prepayment: Option<crate::triggers::PendingDelayedTriggerPayment>,
    ) -> Self {
        self.prepayment = prepayment;
        self
    }

    pub fn with_prevention_shield(
        mut self,
        prevention_shield: Option<crate::prevention::PreventionShieldId>,
    ) -> Self {
        self.prevention_shield = prevention_shield;
        self
    }
}

/// How watcher identity should be represented in delayed scheduling.
#[derive(Debug, Clone)]
pub(crate) enum DelayedWatcherIdentity {
    /// One delayed trigger that watches any object in this set.
    Combined(Vec<ObjectId>),
    /// One delayed trigger per watched object.
    PerObject(Vec<ObjectId>),
}

impl DelayedWatcherIdentity {
    pub fn combined(watchers: Vec<ObjectId>) -> Self {
        Self::Combined(watchers)
    }

    pub fn per_object(watchers: Vec<ObjectId>) -> Self {
        Self::PerObject(watchers)
    }
}

/// Trigger/effect policy template for delayed scheduling.
#[derive(Debug, Clone)]
pub(crate) struct DelayedTriggerTemplate {
    pub trigger: Trigger,
    pub effects: ResolutionProgram,
    pub linked_exile_owner: Option<crate::linked_exile::LinkedExileOwner>,
    pub source_number_owner: Option<crate::linked_exile::LinkedExileOwner>,
    pub one_shot: bool,
    pub not_before_turn: Option<u32>,
    pub expires_at_turn: Option<u32>,
    pub expires_before_controller_turn_after: Option<u32>,
    pub expires_after_controller_turn_after: Option<u32>,
    pub expires_at_end_of_combat: bool,
    pub bound_extra_turn_index: Option<usize>,
    pub while_any_tagged_object_in_zone: Option<(TagKey, crate::zone::Zone)>,
    pub source_collection_scope: Option<ironsmith_core::effect::SourceCollectionScope>,
    pub while_source_tapped: Option<ObjectId>,
    pub ability_source: Option<ObjectId>,
    pub controller: PlayerId,
    pub x_value: Option<u32>,
    pub choices: Vec<crate::target::ChooseSpec>,
    pub tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    pub tagged_players: HashMap<TagKey, Vec<PlayerId>>,
    pub defending_player_reference: Option<crate::combat_state::DefendingPlayerReference>,
    pub prepayment: Option<crate::triggers::PendingDelayedTriggerPayment>,
    pub prevention_shield: Option<crate::prevention::PreventionShieldId>,
}

impl DelayedTriggerTemplate {
    pub fn new(
        trigger: Trigger,
        effects: impl Into<ResolutionProgram>,
        one_shot: bool,
        controller: PlayerId,
    ) -> Self {
        Self {
            trigger,
            effects: effects.into(),
            linked_exile_owner: None,
            source_number_owner: None,
            one_shot,
            not_before_turn: None,
            expires_at_turn: None,
            expires_before_controller_turn_after: None,
            expires_after_controller_turn_after: None,
            expires_at_end_of_combat: false,
            bound_extra_turn_index: None,
            while_any_tagged_object_in_zone: None,
            source_collection_scope: None,
            while_source_tapped: None,
            ability_source: None,
            controller,
            x_value: None,
            choices: Vec::new(),
            tagged_objects: HashMap::new(),
            tagged_players: HashMap::new(),
            defending_player_reference: None,
            prepayment: None,
            prevention_shield: None,
        }
    }

    pub fn with_not_before_turn(mut self, not_before_turn: Option<u32>) -> Self {
        self.not_before_turn = not_before_turn;
        self
    }

    pub fn with_expires_at_turn(mut self, expires_at_turn: Option<u32>) -> Self {
        self.expires_at_turn = expires_at_turn;
        self
    }

    pub fn with_expires_before_controller_turn_after(mut self, anchor_turn: Option<u32>) -> Self {
        self.expires_before_controller_turn_after = anchor_turn;
        self
    }

    pub fn with_expires_after_controller_turn_after(mut self, anchor_turn: Option<u32>) -> Self {
        self.expires_after_controller_turn_after = anchor_turn;
        self
    }

    pub fn with_expires_at_end_of_combat(mut self, expires: bool) -> Self {
        self.expires_at_end_of_combat = expires;
        self
    }

    pub fn with_bound_extra_turn_index(mut self, index: Option<usize>) -> Self {
        self.bound_extra_turn_index = index;
        self
    }

    pub fn while_any_tagged_object_in_zone_opt(
        mut self,
        duration: Option<(TagKey, crate::zone::Zone)>,
    ) -> Self {
        self.while_any_tagged_object_in_zone = duration;
        self
    }

    pub fn while_source_tapped(mut self, source: Option<ObjectId>) -> Self {
        self.while_source_tapped = source;
        self
    }

    pub fn with_ability_source(mut self, ability_source: Option<ObjectId>) -> Self {
        self.ability_source = ability_source;
        self
    }

    pub fn with_linked_exile_owner(mut self, owner: Option<crate::linked_exile::LinkedExileOwner>) -> Self {
        self.linked_exile_owner = owner;
        self
    }

    pub fn with_source_number_owner(mut self, owner: Option<crate::linked_exile::LinkedExileOwner>) -> Self {
        self.source_number_owner = owner;
        self
    }

    pub fn with_x_value(mut self, x_value: Option<u32>) -> Self {
        self.x_value = x_value;
        self
    }

    pub fn with_tagged_objects(
        mut self,
        tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    ) -> Self {
        self.tagged_objects = tagged_objects;
        self
    }

    pub fn with_tagged_players(mut self, tagged_players: HashMap<TagKey, Vec<PlayerId>>) -> Self {
        self.tagged_players = tagged_players;
        self
    }

    pub fn with_defending_player_reference(mut self, reference: Option<crate::combat_state::DefendingPlayerReference>) -> Self {
        self.defending_player_reference = reference;
        self
    }

    pub fn with_source_collection_scope(mut self, scope: Option<ironsmith_core::effect::SourceCollectionScope>) -> Self {
        self.source_collection_scope = scope; self
    }

    pub fn with_prepayment(
        mut self,
        prepayment: Option<crate::triggers::PendingDelayedTriggerPayment>,
    ) -> Self {
        self.prepayment = prepayment;
        self
    }

    pub fn with_prevention_shield(
        mut self,
        prevention_shield: Option<crate::prevention::PreventionShieldId>,
    ) -> Self {
        self.prevention_shield = prevention_shield;
        self
    }
}

/// Complete effect identity: validated canonical models compare across
/// independent direct/artifact materializations. Native callbacks require the
/// same immutable executor; display text and Debug output never prove identity.
#[derive(Clone, PartialEq)]
enum PlayerRuleEffectDefinition {
    Compiled(String),
    Native(crate::effect::Effect),
}

fn player_rule_program_definition(program: &ResolutionProgram) -> ironsmith_core::ResolutionProgram<PlayerRuleEffectDefinition> {
    program.clone().try_map_effects(|effect| Ok::<_, std::convert::Infallible>(
        match effect.serialized_model() {
            Some(model) => PlayerRuleEffectDefinition::Compiled(model.to_owned()),
            None => PlayerRuleEffectDefinition::Native(effect),
        }
    )).unwrap_or_else(|never| match never {})
}

fn compatible_player_rule(existing: &DelayedTrigger, config: &DelayedTriggerConfig) -> bool {
    let trigger_equal = match (existing.trigger.compiled_model(), config.trigger.compiled_model()) {
        (Some(left),Some(right)) => left == right,
        (None,None) => existing.trigger.runtime_matcher_identity() == config.trigger.runtime_matcher_identity(),
        _ => false,
    };
    trigger_equal && existing.intervening_if == config.intervening_if
        && player_rule_program_definition(&existing.effects) == player_rule_program_definition(&config.effects)
        && existing.choices == config.choices
}

/// Existing callers register source-bound delayed instructions. The checked
/// gateway also protects direct callers that supply a persistent rule.
pub fn queue_delayed_trigger(game: &mut GameState, config: DelayedTriggerConfig) {
    try_queue_delayed_trigger(game, config)
        .unwrap_or_else(|error| panic!("delayed registration invariant failed: {error}"));
}

/// A persistent rule name admits one compatible definition per player. A
/// source-authority conflict is an invariant failure, never first-wins behavior.
/// Returns false only for a proved identical repeat registration.
pub fn try_queue_delayed_trigger(game: &mut GameState, config: DelayedTriggerConfig) -> Result<bool, crate::effects::ExecutionError> {
    let mut config = config;
    if let Some(name) = config.persistent_player_rule.as_ref() {
        // Rule ownership belongs to a player, not the card or the exiled set.
        if let Some(existing) = game.effect_store.delayed_triggers.iter().find(|entry|
            entry.controller == config.controller && entry.persistent_player_rule.as_ref() == Some(name)) {
            if compatible_player_rule(existing, &config) { return Ok(false); }
            return Err(crate::effects::ExecutionError::InternalError(format!(
                "incompatible persistent player rule {name:?} for {:?}", config.controller)));
        }
        config.ability_source = None;
        config.target_objects.clear();
        config.tagged_objects.clear();
        config.tagged_players.clear();
        config.linked_exile_owner = None;
        config.source_number_owner = None;
        config.x_value = None;
    }

    // Capture only compiler-declared free references. Names produced inside
    // the delayed body are local slots and must not sample a parent's tags.
    // Exact object IDs deliberately fail after departure/reentry into exile.
    for tag in config.effects.object_reference_imports.clone() {
        if config.effects.captured_object_references.iter().any(|(key,_)|key==&tag) {continue;}
        let snapshots=config.tagged_objects.get(&tag).or_else(|| {
            (tag.as_str()=="__prior_exiled_card__").then(||
                config.tagged_objects.get(&TagKey::from(crate::tag::SOURCE_EXILED_SELF_TAG))).flatten()
        });
        let ids=snapshots.into_iter().flatten().map(|snapshot|snapshot.object_id).collect();
        // An absent import is an explicitly empty exact reference, never an
        // invitation to revive a later physical-card incarnation.
        config.effects.captured_object_references.push((tag,ids));
    }
    if let Some(scope) = &config.source_collection_scope {
        if let Some(source)=config.ability_source {
            let members=config.tagged_objects.get(&scope.tag).into_iter().flatten().map(|snapshot|snapshot.object_id).collect::<Vec<_>>();
            game.note_source_collection(source,None,&scope.name,scope.zone,&members);
        }
        let incoming = config.tagged_objects.get(&scope.tag).cloned().unwrap_or_default();
        if let Some(existing) = game.effect_store.delayed_triggers.iter_mut().find(|existing| {
            existing.ability_source == config.ability_source && existing.source_collection_scope.as_ref().is_some_and(|old| old.name == scope.name)
        }) {
            let old = existing.source_collection_scope.as_ref().unwrap();
            let values = existing.tagged_objects.entry(old.tag.clone()).or_default();
            for snapshot in incoming { if !values.iter().any(|previous| previous.object_id == snapshot.object_id) { values.push(snapshot); } }
            // Only the declared collection slot grows. Independent captured
            // parent references retain their original per-watcher binding.
            let ids=values.iter().map(|snapshot|snapshot.object_id).collect::<Vec<_>>();
            for (tag,captured) in &mut existing.effects.captured_object_references {
                if tag==&old.tag {*captured=ids.clone();}
            }
            return Ok(false);
        }
    }
    let (ability_source_stable_id, ability_source_name, ability_source_snapshot) = config
        .ability_source
        .and_then(|source_id| {
            game.object(source_id).map(|object| {
                let mut snapshot =
                    ObjectSnapshot::from_object_with_calculated_characteristics(object, game);
                if game.object_has_static_ability_id(
                    source_id,
                    crate::static_abilities::StaticAbilityId::Lifelink,
                ) && !snapshot
                    .has_static_ability_id(crate::static_abilities::StaticAbilityId::Lifelink)
                {
                    std::sync::Arc::make_mut(&mut snapshot.abilities)
                        .push(crate::ability::lifelink());
                }
                (object.stable_id, object.name.to_string(), snapshot)
            }).or_else(|| game.source_last_known_snapshot(source_id)
                .map(|snapshot| (snapshot.stable_id, snapshot.name.to_string(), snapshot.clone())))
        })
        .map(|(stable_id, name, snapshot)| (Some(stable_id), Some(name), Some(snapshot)))
        .unwrap_or((None, None, None));

    // Tagged objects are pinned to the incarnation they have as the delayed
    // trigger is created by the scheduling effect, which knows which moves
    // its resolution may follow (`pin_tagged_objects_to_current`). "Sacrifice
    // it at the beginning of the next end step" affects that permanent only;
    // if it later leaves and returns it's a new object the delayed trigger
    // can't find (CR 603.7c, 400.7).
    let tagged_objects = config.tagged_objects;

    game.effect_store.delayed_triggers.push(DelayedTrigger {
        linked_exile_owner: config.linked_exile_owner,
        source_number_owner: config.source_number_owner,
        trigger: config.trigger,
        persistent_player_rule: config.persistent_player_rule,
        intervening_if: config.intervening_if,
        effects: config.effects,
        one_shot: config.one_shot,
        x_value: config.x_value,
        not_before_turn: config.not_before_turn,
        expires_at_turn: config.expires_at_turn,
        expires_before_controller_turn_after: config.expires_before_controller_turn_after,
        expires_after_controller_turn_after: config.expires_after_controller_turn_after,
        expires_at_end_of_combat: config.expires_at_end_of_combat,
        bound_extra_turn_index: config.bound_extra_turn_index,
        while_any_tagged_object_in_zone: config.while_any_tagged_object_in_zone,
        source_collection_scope: config.source_collection_scope,
        while_source_tapped: config.while_source_tapped,
        target_objects: config.target_objects,
        ability_source: config.ability_source,
        ability_source_stable_id,
        ability_source_name,
        ability_source_snapshot,
        controller: config.controller,
        choices: config.choices,
        tagged_objects,
        tagged_players: config.tagged_players,
        defending_player_reference: config.defending_player_reference,
        prepayment: config.prepayment,
        prevention_shield: config.prevention_shield,
    });
    Ok(true)
}

/// Queue delayed trigger(s) using a shared template and watcher identity policy.
///
/// Returns how many delayed triggers were enqueued.
pub(crate) fn queue_delayed_from_template(
    game: &mut GameState,
    watchers: DelayedWatcherIdentity,
    template: DelayedTriggerTemplate,
) -> usize {
    match watchers {
        DelayedWatcherIdentity::Combined(target_objects) => {
            queue_delayed_trigger(
                game,
                DelayedTriggerConfig::new(
                    template.trigger,
                    template.effects,
                    template.one_shot,
                    target_objects,
                    template.controller,
                )
                .with_not_before_turn(template.not_before_turn)
                .with_expires_at_turn(template.expires_at_turn)
                .with_expires_after_controller_turn_after(template.expires_after_controller_turn_after)
                .with_expires_before_controller_turn_after(
                    template.expires_before_controller_turn_after,
                )
                .with_expires_at_end_of_combat(template.expires_at_end_of_combat)
                .with_bound_extra_turn_index(template.bound_extra_turn_index)
                .while_any_tagged_object_in_zone_opt(template.while_any_tagged_object_in_zone)
                .with_source_collection_scope(template.source_collection_scope.clone())
                .while_source_tapped(template.while_source_tapped)
                .with_linked_exile_owner(template.linked_exile_owner.clone())
                .with_source_number_owner(template.source_number_owner.clone())
                .with_ability_source(template.ability_source)
                .with_x_value(template.x_value)
                .with_choices(template.choices)
                .with_tagged_objects(template.tagged_objects)
                .with_tagged_players(template.tagged_players)
                .with_defending_player_reference(template.defending_player_reference)
                .with_prepayment(template.prepayment)
                .with_prevention_shield(template.prevention_shield),
            );
            1
        }
        DelayedWatcherIdentity::PerObject(target_objects) => {
            let mut queued = 0usize;
            for watched in target_objects {
                queue_delayed_trigger(
                    game,
                    DelayedTriggerConfig::new(
                        template.trigger.clone(),
                        template.effects.clone(),
                        template.one_shot,
                        vec![watched],
                        template.controller,
                    )
                    .with_not_before_turn(template.not_before_turn)
                    .with_expires_at_turn(template.expires_at_turn)
                .with_expires_after_controller_turn_after(template.expires_after_controller_turn_after)
                    .with_expires_before_controller_turn_after(
                        template.expires_before_controller_turn_after,
                    )
                    .with_expires_at_end_of_combat(template.expires_at_end_of_combat)
                .with_bound_extra_turn_index(template.bound_extra_turn_index)
                    .while_any_tagged_object_in_zone_opt(
                        template.while_any_tagged_object_in_zone.clone(),
                    )
                    .with_source_collection_scope(template.source_collection_scope.clone())
                    .while_source_tapped(template.while_source_tapped)
                    .with_linked_exile_owner(template.linked_exile_owner.clone())
                    .with_source_number_owner(template.source_number_owner.clone())
                .with_ability_source(template.ability_source)
                    .with_x_value(template.x_value)
                    .with_choices(template.choices.clone())
                    .with_tagged_objects(template.tagged_objects.clone())
                    .with_tagged_players(template.tagged_players.clone())
                    .with_defending_player_reference(template.defending_player_reference)
                    .with_prepayment(template.prepayment.clone())
                    .with_prevention_shield(template.prevention_shield),
                );
                queued += 1;
            }
            queued
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;
    use crate::ids::PlayerId;
    use crate::target::ChooseSpec;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_queue_delayed_trigger_defaults() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let watched = game.new_object_id();

        let config = DelayedTriggerConfig::new(
            Trigger::this_leaves_battlefield(),
            vec![Effect::sacrifice_source()],
            true,
            vec![watched],
            alice,
        );
        queue_delayed_trigger(&mut game, config);

        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        let delayed = &game.effect_store.delayed_triggers[0];
        assert!(delayed.one_shot);
        assert_eq!(delayed.target_objects, vec![watched]);
        assert_eq!(delayed.controller, alice);
        assert_eq!(delayed.not_before_turn, None);
        assert_eq!(delayed.expires_at_turn, None);
        assert_eq!(delayed.ability_source, None);
        assert_eq!(delayed.ability_source_stable_id, None);
        assert_eq!(delayed.ability_source_name, None);
        assert!(delayed.ability_source_snapshot.is_none());
    }

    #[test]
    fn test_queue_delayed_trigger_with_optional_fields() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let watched = game.new_object_id();
        let turn = game.turn.turn_number;

        let config = DelayedTriggerConfig::new(
            Trigger::end_of_combat(),
            vec![Effect::exile(ChooseSpec::SpecificObject(watched))],
            false,
            vec![watched],
            alice,
        )
        .with_not_before_turn(Some(turn + 1))
        .with_expires_at_turn(Some(turn))
        .with_ability_source(Some(source));
        queue_delayed_trigger(&mut game, config);

        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        let delayed = &game.effect_store.delayed_triggers[0];
        assert!(!delayed.one_shot);
        assert_eq!(delayed.not_before_turn, Some(turn + 1));
        assert_eq!(delayed.expires_at_turn, Some(turn));
        assert_eq!(delayed.ability_source, Some(source));
    }

    #[test]
    fn test_queue_delayed_from_template_combined_watchers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let watched_a = game.new_object_id();
        let watched_b = game.new_object_id();

        let template = DelayedTriggerTemplate::new(
            Trigger::this_leaves_battlefield(),
            vec![Effect::exile(ChooseSpec::SpecificObject(source))],
            true,
            alice,
        )
        .with_ability_source(Some(source));

        let queued = queue_delayed_from_template(
            &mut game,
            DelayedWatcherIdentity::combined(vec![watched_a, watched_b]),
            template,
        );

        assert_eq!(queued, 1);
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        let delayed = &game.effect_store.delayed_triggers[0];
        assert_eq!(delayed.target_objects, vec![watched_a, watched_b]);
    }

    #[test]
    fn test_queue_delayed_from_template_per_object_watchers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let watched_a = game.new_object_id();
        let watched_b = game.new_object_id();

        let template = DelayedTriggerTemplate::new(
            Trigger::this_leaves_battlefield(),
            vec![Effect::sacrifice_source()],
            true,
            alice,
        );

        let queued = queue_delayed_from_template(
            &mut game,
            DelayedWatcherIdentity::per_object(vec![watched_a, watched_b]),
            template,
        );

        assert_eq!(queued, 2);
        assert_eq!(game.effect_store.delayed_triggers.len(), 2);
        assert_eq!(
            game.effect_store.delayed_triggers[0].target_objects,
            vec![watched_a]
        );
        assert_eq!(
            game.effect_store.delayed_triggers[1].target_objects,
            vec![watched_b]
        );
    }
}
