//! "Whenever this permanent becomes the target of [spell filter]" trigger.

use crate::events::EventKind;
use crate::events::spells::BecomesTargetedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Match the particular spell/ability that selected the target. Several
/// activations can share a physical source, and an ability copy has its own
/// stack proxy but keeps the original source. Neither is interchangeable.
fn targeting_stack_object_matches(
    filter: &ObjectFilter,
    event: &BecomesTargetedEvent,
    ctx: &TriggerContext,
) -> bool {
    let entry = if let Some(id) = event.stack_ability {
        ctx.game.stack.iter().find(|entry| entry.is_ability && entry.target_id() == id)
    } else if !event.by_ability {
        ctx.game.stack.iter().find(|entry| !entry.is_ability && entry.object_id == event.source)
    } else {
        // Legacy manually produced events can prove an identity only when
        // there is exactly one matching activation. Never choose a sibling.
        let mut entries = ctx.game.stack.iter().filter(|entry|
            entry.is_ability && BecomesTargetedEvent::source_for_stack_entry(entry) == event.source);
        let first = entries.next();
        if entries.next().is_some() { return false; }
        first
    };
    let Some(entry) = entry else { return false; };
    let subject = ctx.game.object(entry.object_id).map(crate::filter::ObjectSubject::Live)
        .or_else(|| entry.source_snapshot.as_ref()
            .filter(|snapshot| snapshot.object_id == event.source)
            .map(crate::filter::ObjectSubject::Snapshot));
    let Some(subject) = subject else { return false; };
    let mut filter_ctx = ctx.filter_ctx.clone();
    filter_ctx.stack_entry = Some(entry.target_id());
    fn matches(
        filter: &ObjectFilter, subject: crate::filter::ObjectSubject<'_>,
        context: &crate::filter::FilterContext, game: &crate::game_state::GameState,
        controller: crate::ids::PlayerId,
    ) -> bool {
        if filter.controller.as_ref().is_some_and(|player|
            !crate::filter::player_filter_matches_game(player, controller, game, context))
        { return false; }
        let mut qualities = filter.clone();
        qualities.controller = None;
        qualities.any_of.clear();
        subject.matches(&qualities, context, game)
            && (filter.any_of.is_empty() || filter.any_of.iter().any(|branch|
                matches(branch, subject, context, game, controller)))
    }
    matches(filter, subject, &filter_ctx, ctx.game, entry.controller)
}

#[derive(Debug, Clone, PartialEq)]
pub struct BecomesTargetedBySpellTrigger {
    pub filter: ObjectFilter,
}

impl BecomesTargetedBySpellTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for BecomesTargetedBySpellTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BecomesTargeted {
            return false;
        }
        let Some(e) = event.downcast::<BecomesTargetedEvent>() else {
            return false;
        };
        if e.target_object() != Some(ctx.source_id) || e.by_ability {
            return false;
        }
        targeting_stack_object_matches(&self.filter, e, ctx)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::BecomesTargeted])
    }

    fn display(&self) -> String {
        format!(
            "Whenever this permanent becomes the target of {}",
            self.filter.description()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BecomesTargetedByStackObjectTrigger {
    pub filter: ObjectFilter,
}

impl BecomesTargetedByStackObjectTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for BecomesTargetedByStackObjectTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BecomesTargeted {
            return false;
        }
        let Some(e) = event.downcast::<BecomesTargetedEvent>() else {
            return false;
        };
        if e.target_object() != Some(ctx.source_id) {
            return false;
        }
        targeting_stack_object_matches(&self.filter, e, ctx)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::BecomesTargeted])
    }

    fn display(&self) -> String {
        format!(
            "Whenever this permanent becomes the target of {}",
            self.filter.description()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BecomesTargetedObjectByStackObjectTrigger {
    pub target_filter: ObjectFilter,
    pub source_filter: ObjectFilter,
}

impl BecomesTargetedObjectByStackObjectTrigger {
    pub fn new(target_filter: ObjectFilter, source_filter: ObjectFilter) -> Self {
        Self {
            target_filter,
            source_filter,
        }
    }
}

impl TriggerMatcher for BecomesTargetedObjectByStackObjectTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BecomesTargeted {
            return false;
        }
        let Some(e) = event.downcast::<BecomesTargetedEvent>() else {
            return false;
        };
        let Some(target_id) = e.target_object() else {
            return false;
        };
        let Some(target) = ctx.game.object(target_id) else {
            return false;
        };
        if !self
            .target_filter
            .matches(target, &ctx.filter_ctx, ctx.game)
        {
            return false;
        }
        targeting_stack_object_matches(&self.source_filter, e, ctx)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::BecomesTargeted])
    }

    fn display(&self) -> String {
        format!(
            "Whenever {} becomes the target of {}",
            self.target_filter.description(),
            self.source_filter.description()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::game_state::{GameState, StackEntry};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn create_aura_spell_on_stack(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build();
        let id = game.create_object_from_card(&card, controller, Zone::Stack);
        game.push_to_stack(StackEntry::new(id, controller));
        id
    }

    fn aura_spell_filter() -> ObjectFilter {
        ObjectFilter::default()
            .in_zone(Zone::Stack)
            .with_type(CardType::Enchantment)
            .with_subtype(Subtype::Aura)
    }

    #[test]
    fn matches_when_source_is_matching_spell() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let target = create_creature(&mut game, "Target", alice);
        let aura_spell = create_aura_spell_on_stack(&mut game, "Ethereal Armor", bob);

        let trigger = BecomesTargetedBySpellTrigger::new(aura_spell_filter());
        let ctx = TriggerContext::for_source(target, alice, &game);
        let source = game
            .object(aura_spell)
            .expect("aura source spell should exist on stack");
        assert!(
            aura_spell_filter().matches(source, &ctx.filter_ctx, &game),
            "aura spell filter should match source object: {source:#?}"
        );
        let event = TriggerEvent::new_with_provenance(
            BecomesTargetedEvent::new(target, aura_spell, bob, false),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_targeting_source_is_ability() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let target = create_creature(&mut game, "Target", alice);
        let aura_source = create_aura_spell_on_stack(&mut game, "Aura Source", bob);

        let trigger = BecomesTargetedBySpellTrigger::new(aura_spell_filter());
        let ctx = TriggerContext::for_source(target, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            BecomesTargetedEvent::new(target, aura_source, bob, true),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn stack_object_trigger_matches_ability_targeting_only_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let target = create_creature(&mut game, "Target", alice);
        let ability_source = create_creature(&mut game, "Ability Source", bob);
        game.push_to_stack(
            StackEntry::ability(
                ability_source,
                bob,
                crate::resolution::ResolutionProgram::from_effects(Vec::new()),
            )
            .with_targets(vec![crate::game_state::Target::Object(target)]),
        );

        let mut filter = ObjectFilter::ability();
        filter.target_count = Some(crate::effect::ChoiceCount::exactly(1));
        filter.targets_only_object = Some(Box::new(ObjectFilter::source()));
        let trigger = BecomesTargetedByStackObjectTrigger::new(filter);
        let ctx = TriggerContext::for_source(target, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            BecomesTargetedEvent::new(target, ability_source, bob, true),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
    }
    #[test]
    fn source_shared_activations_and_copy_proxy_keep_their_own_target_sets() {
        let mut game = setup_game();
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        let target = create_creature(&mut game, "Target", a);
        let other = create_creature(&mut game, "Other target", a);
        let source = create_creature(&mut game, "Ability source", a);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game);
        game.push_to_stack(StackEntry::ability(source, b, crate::resolution::ResolutionProgram::from_effects(vec![]))
            .with_targets(vec![crate::game_state::Target::Object(target), crate::game_state::Target::Object(other)]));
        let first = game.stack.last().unwrap().clone();
        game.push_to_stack(StackEntry::ability(source, b, crate::resolution::ResolutionProgram::from_effects(vec![]))
            .with_targets(vec![crate::game_state::Target::Object(target)]));
        let second = game.stack.last().unwrap().clone();
        let proxy = create_aura_spell_on_stack(&mut game, "Copied ability proxy", b);
        game.stack.pop();
        let mut copy = StackEntry::ability(proxy, b, crate::resolution::ResolutionProgram::from_effects(vec![]))
            .with_targets(vec![crate::game_state::Target::Object(target)]);
        copy.source_snapshot = Some(snapshot); game.stack.push(copy);
        let copy = game.stack.last().unwrap().clone();
        let mut filter = ObjectFilter::ability(); filter.controller = Some(crate::target::PlayerFilter::Opponent);
        filter.target_count = Some(crate::effect::ChoiceCount::exactly(1));
        filter.targets_only_object = Some(Box::new(ObjectFilter::source()));
        let trigger = BecomesTargetedByStackObjectTrigger::new(filter);
        for (entry, expected) in [(first, false), (second, true), (copy, true)] {
            let event = TriggerEvent::new_with_provenance(BecomesTargetedEvent::from_stack_entry(crate::game_state::Target::Object(target), &entry), Default::default());
            assert_eq!(trigger.matches(&event, &TriggerContext::for_source(target, a, &game)), expected);
        }
    }

    #[test]
    fn departed_ability_source_still_has_exact_stack_kind_controller_and_target_count() {
        let mut game = setup_game(); let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        let target = create_creature(&mut game, "Target", a);
        let source = create_creature(&mut game, "Departing source", b);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game);
        let mut entry = StackEntry::ability(source, b, crate::resolution::ResolutionProgram::from_effects(vec![]))
            .with_targets(vec![crate::game_state::Target::Object(target)]);
        entry.source_snapshot = Some(snapshot); game.push_to_stack(entry);
        let entry = game.stack.last().unwrap().clone();
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let event = TriggerEvent::new_with_provenance(BecomesTargetedEvent::from_stack_entry(crate::game_state::Target::Object(target), &entry), Default::default());
        let mut filter = ObjectFilter::ability(); filter.controller = Some(crate::target::PlayerFilter::Opponent);
        filter.target_count = Some(crate::effect::ChoiceCount::exactly(1)); filter.targets_only_object = Some(Box::new(ObjectFilter::source()));
        let trigger = BecomesTargetedByStackObjectTrigger::new(filter.clone());
        assert!(trigger.matches(&event, &TriggerContext::for_source(target, a, &game)));
        filter.target_count = Some(crate::effect::ChoiceCount::exactly(2));
        assert!(!BecomesTargetedByStackObjectTrigger::new(filter).matches(&event, &TriggerContext::for_source(target, a, &game)));
        game.stack.clear();
        assert!(!trigger.matches(&event, &TriggerContext::for_source(target, a, &game)));
    }

    #[test]
    fn prospective_spell_zone_exception_survives_without_a_stack_entry() {
        let mut game = setup_game(); let a = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Prospective spell").card_types(vec![CardType::Instant]).build();
        for zone in [Zone::Hand, Zone::Exile] {
            let id = game.create_object_from_card(&card, a, zone);
            let source = game.object(id).unwrap();
            let mut context = game.filter_context_for(a, Some(id));
            let filter = ObjectFilter::spell();
            assert!(!filter.matches(source, &context, &game));
            context.prospective_cast = Some(id); context.caster = Some(a);
            assert!(filter.matches(source, &context, &game));
            let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(source, &game);
            context.stack_entry = Some(crate::ids::ObjectId::from_raw(999));
            assert!(!filter.matches_snapshot(&snapshot, &context, &game), "historical source cannot invent the announced stack entry");
        }
    }

    #[test]
    fn targets_only_source_allows_repeated_slots_but_a_single_target_counts_instances() {
        let mut game = setup_game(); let a = PlayerId::from_index(0);
        let target = create_creature(&mut game, "Repeated target", a);
        let source = create_creature(&mut game, "Ability source", a);
        game.push_to_stack(StackEntry::ability(source, a, crate::resolution::ResolutionProgram::from_effects(vec![]))
            .with_targets(vec![crate::game_state::Target::Object(target); 2]));
        let entry = game.stack.last().unwrap().clone();
        let event = TriggerEvent::new_with_provenance(BecomesTargetedEvent::from_stack_entry(crate::game_state::Target::Object(target), &entry), Default::default());
        let mut only_source = ObjectFilter::ability(); only_source.targets_only_object = Some(Box::new(ObjectFilter::source()));
        assert!(BecomesTargetedByStackObjectTrigger::new(only_source.clone()).matches(&event, &TriggerContext::for_source(target, a, &game)));
        only_source.target_count = Some(crate::effect::ChoiceCount::exactly(1));
        assert!(!BecomesTargetedByStackObjectTrigger::new(only_source).matches(&event, &TriggerContext::for_source(target, a, &game)));
    }

}
