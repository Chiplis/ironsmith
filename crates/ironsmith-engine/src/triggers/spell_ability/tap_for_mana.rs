//! "Whenever [player] taps [object] for mana" trigger.

use crate::events::mana::ManaProductionProvenance;
use crate::events::{EventKind, ManaAddedEvent};
use crate::filter::ObjectFilterExt as _;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct TapForManaTrigger {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
}

impl TapForManaTrigger {
    pub fn new(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self { player, filter }
    }
}

impl TriggerMatcher for TapForManaTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::ManaAdded {
            return false;
        }
        let Some(e) = event.downcast::<ManaAddedEvent>() else {
            return false;
        };
        if e.mana.is_empty() || e.provenance != ManaProductionProvenance::TappedSourceForMana {
            return false;
        }

        let player_matches = match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => e.player == *id,
            PlayerFilter::Active => ctx.game.is_active_player(e.player),
            PlayerFilter::IteratedPlayer => event.trigger_player() == Some(e.player),
            _ => true,
        };
        if !player_matches {
            return false;
        }

        if let Some(snapshot) = e.snapshot.as_ref() {
            self.filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
        } else if let Some(obj) = ctx.game.object(e.source) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        let player = match &self.player {
            PlayerFilter::You => "you".to_string(),
            PlayerFilter::Opponent => "an opponent".to_string(),
            PlayerFilter::Any => "a player".to_string(),
            PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => "that player".to_string(),
            other => describe_player_filter_fallback(other),
        };
        let verb = if matches!(self.player, PlayerFilter::You) {
            "tap"
        } else {
            "taps"
        };
        let object = describe_tap_for_mana_filter(&self.filter);
        let object_phrase = if starts_with_determiner(&object) {
            object
        } else {
            format!("a {object}")
        };
        format!("Whenever {player} {verb} {object_phrase} for mana")
    }
}

fn describe_tap_for_mana_filter(filter: &ObjectFilter) -> String {
    let description = filter.description();
    if filter.subtypes.len() < 3 {
        return description;
    }
    let parts = description.split(" or ").collect::<Vec<_>>();
    if parts.len() != filter.subtypes.len() {
        return description;
    }
    format!(
        "{}, or {}",
        parts[..parts.len() - 1].join(", "),
        parts[parts.len() - 1]
    )
}

fn describe_player_filter_fallback(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Defending => "the defending player".to_string(),
        PlayerFilter::Attacking => "the attacking player".to_string(),
        PlayerFilter::Active => "the active player".to_string(),
        PlayerFilter::Teammate => "a teammate".to_string(),
        _ => "a player".to_string(),
    }
}

fn starts_with_determiner(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("another ")
        || lower.starts_with("other ")
        || lower.starts_with("target ")
        || lower.starts_with("that ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::CardDefinitionBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaSymbol;
    use crate::snapshot::ObjectSnapshot;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    #[test]
    fn matches_actual_mana_from_a_tapped_filtered_source_using_lki() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let land = CardDefinitionBuilder::new(CardId::new(), "Snow Forest")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Forest])
            .build();
        let land_id = game.create_object_from_definition(&land, alice, Zone::Battlefield);
        let snapshot =
            ObjectSnapshot::from_object(game.object(land_id).expect("land should exist"), &game);
        game.remove_object(land_id);

        let trigger = TapForManaTrigger::new(
            PlayerFilter::You,
            ObjectFilter::land().with_subtype(Subtype::Forest),
        );
        let ctx = TriggerContext::for_source(game.new_object_id(), alice, &game);
        let event = ManaAddedEvent::new(land_id, alice, alice, vec![ManaSymbol::Green])
            .with_snapshot(Some(snapshot))
            .with_production_provenance(ManaProductionProvenance::TappedSourceForMana)
            .into_trigger_event();

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn rejects_mana_not_produced_by_tapping_the_source() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let land = CardDefinitionBuilder::new(CardId::new(), "Forest")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_definition(&land, alice, Zone::Battlefield);
        let trigger = TapForManaTrigger::new(PlayerFilter::Any, ObjectFilter::land());
        let ctx = TriggerContext::for_source(game.new_object_id(), alice, &game);
        let event = ManaAddedEvent::new(land_id, alice, alice, vec![ManaSymbol::Green])
            .into_trigger_event();

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn display_uses_oracle_disjunction_punctuation_for_three_subtypes() {
        let mut filter = ObjectFilter::default();
        filter.subtypes = vec![Subtype::Mountain, Subtype::Forest, Subtype::Plains];
        let trigger = TapForManaTrigger::new(PlayerFilter::Any, filter);

        assert_eq!(
            trigger.display(),
            "Whenever a player taps a Mountain, Forest, or Plains for mana"
        );
    }

    /// Badgermole Cub: earthbend animates a land, and tapping that land for
    /// mana is tapping a creature for mana, so the Cub's additional {G} applies.
    ///
    /// Regression: the priority-level mana-ability path snapshotted the source
    /// with its printed characteristics, so the animated land still looked like
    /// a plain land to this trigger's filter.
    #[test]
    fn matches_an_animated_land_tapped_for_mana_through_the_priority_path() {
        use crate::ability::Ability;
        use crate::card::{CardBuilder, PowerToughness};
        use crate::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
        use crate::effect::Effect;
        use crate::effects::{EarthbendEffect, EffectExecutor, ExecutionContext, ResolvedTarget};
        use crate::filter::ObjectFilter;
        use crate::game_loop::{
            PriorityLoopState, PriorityResponse, apply_priority_response_with_dm,
        };
        use crate::game_state::Phase;
        use crate::target::ChooseSpec;
        use crate::triggers::{Trigger, TriggerQueue};

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let swamp = game.create_object_from_definition(
            &crate::cards::definitions::basic_swamp(),
            alice,
            Zone::Battlefield,
        );

        let cub_card = CardBuilder::new(CardId::new(), "Badgermole Cub")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let cub = game.create_object_from_card(&cub_card, alice, Zone::Battlefield);
        if let Some(object) = game.object_mut(cub) {
            object.abilities_mut().push(Ability::triggered(
                Trigger::player_taps_for_mana(PlayerFilter::You, ObjectFilter::creature()),
                vec![Effect::add_mana(vec![ManaSymbol::Green])],
            ));
        }

        // Badgermole Cub's earthbend 1, targeting the Swamp.
        let mut ctx = ExecutionContext::new_default(cub, alice)
            .with_targets(vec![ResolvedTarget::Object(swamp)]);
        EarthbendEffect::new(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control())),
            1,
        )
        .execute(&mut game, &mut ctx)
        .expect("earthbend should resolve");
        game.refresh_continuous_state();

        let action = compute_legal_actions(&game, alice)
            .into_iter()
            .find(|action| {
                matches!(action, LegalAction::ActivateManaAbility { source, .. } if *source == swamp)
            })
            .expect("the animated Swamp should still offer its mana ability");

        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut decision_maker = SelectFirstDecisionMaker;
        apply_priority_response_with_dm(
            &mut game,
            &mut trigger_queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut decision_maker,
        )
        .expect("tapping the animated Swamp for mana should succeed");

        let pool = &game.player(alice).expect("alice").mana_pool;
        assert_eq!(pool.black, 1, "the Swamp should still add {{B}}");
        assert_eq!(
            pool.green, 1,
            "tapping the animated Swamp taps a creature for mana, so the additional {{G}} applies"
        );
    }
}
