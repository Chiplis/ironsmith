//! "Whenever [filter] attacks" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedEvent;
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::ids::ObjectId;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when a matching creature attacks.
///
/// Used by cards that care about other creatures attacking.
#[derive(Debug, Clone, PartialEq)]
pub struct AttacksTrigger {
    /// Filter for creatures that trigger this ability.
    pub filter: ObjectFilter,
    /// If true, this trigger fires only once when one or more matching creatures attack.
    pub one_or_more: bool,
    /// Minimum number of total attackers required for this trigger to fire.
    pub min_total_attackers: usize,
}

/// Trigger that fires once when one or more matching players are attacked.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayersAttackedTrigger {
    pub player_filter: PlayerFilter,
}

impl PlayersAttackedTrigger {
    pub fn one_or_more(player_filter: PlayerFilter) -> Self {
        Self { player_filter }
    }

    fn target_matches(
        &self,
        target: &crate::combat_state::AttackTarget,
        ctx: &TriggerContext,
    ) -> bool {
        let crate::combat_state::AttackTarget::Player(player) = target else {
            return false;
        };
        self.player_filter.matches_player(*player, &ctx.filter_ctx)
    }

    fn is_first_matching_attacker_this_combat(
        &self,
        attacker: ObjectId,
        attack_target: &crate::combat_state::AttackTarget,
        ctx: &TriggerContext,
    ) -> bool {
        let Some(combat) = ctx.game.combat.as_ref() else {
            return true;
        };
        if !self.target_matches(attack_target, ctx) {
            return false;
        }
        for info in &combat.attackers {
            if self.target_matches(&info.target, ctx) {
                return info.creature == attacker;
            }
        }
        true
    }
}

impl AttacksTrigger {
    /// Create a new attacks trigger with the given filter.
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: false,
            min_total_attackers: 1,
        }
    }

    /// Create an attacks trigger that fires once for one-or-more attackers.
    pub fn one_or_more(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: true,
            min_total_attackers: 1,
        }
    }

    /// Create an attacks trigger that fires once for one-or-more attackers and
    /// only if at least `min_total_attackers` attackers were declared.
    pub fn one_or_more_with_min_total_attackers(
        filter: ObjectFilter,
        min_total_attackers: usize,
    ) -> Self {
        Self {
            filter,
            one_or_more: true,
            min_total_attackers: min_total_attackers.max(1),
        }
    }

    /// Create an attacks trigger for any creature.
    pub fn any_creature() -> Self {
        Self::new(ObjectFilter::creature())
    }

    fn is_first_matching_attacker_this_combat(
        &self,
        attacker: ObjectId,
        attack_target: &crate::combat_state::AttackTarget,
        ctx: &TriggerContext,
    ) -> bool {
        let Some(combat) = ctx.game.combat.as_ref() else {
            return true;
        };
        let match_per_defending_player = self
            .filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some();
        let current_defending_player = match_per_defending_player
            .then(|| defending_player_for_attack_target(attack_target, ctx.game))
            .flatten();
        for info in &combat.attackers {
            if match_per_defending_player
                && defending_player_for_attack_target(&info.target, ctx.game)
                    != current_defending_player
            {
                continue;
            }
            if self.matches_attacker_info(info, ctx) {
                return info.creature == attacker;
            }
        }
        true
    }

    fn matching_attacker_count_this_combat(&self, ctx: &TriggerContext) -> Option<i32> {
        let combat = ctx.game.combat.as_ref()?;
        let count = combat
            .attackers
            .iter()
            .filter(|info| self.matches_attacker_info(info, ctx))
            .count();
        (count > 0).then_some(count as i32)
    }

    fn matches_attacker_info(
        &self,
        info: &crate::combat_state::AttackerInfo,
        ctx: &TriggerContext,
    ) -> bool {
        let Some(obj) = ctx.game.object(info.creature) else {
            return false;
        };
        self.matches_attacker_object_and_target(obj, &info.target, ctx)
    }

    fn matches_attacker_object_and_target(
        &self,
        obj: &crate::object::Object,
        target: &crate::combat_state::AttackTarget,
        ctx: &TriggerContext,
    ) -> bool {
        let mut object_filter = self.filter.clone();
        let attacked_player_filter = object_filter
            .attacking_player_or_planeswalker_controlled_by
            .take();
        let attacked_target_must_be_player = object_filter.targets_only_player.take().is_some();
        if !object_filter.matches(obj, &ctx.filter_ctx, ctx.game) {
            return false;
        }
        let Some(attacked_player_filter) = attacked_player_filter else {
            return true;
        };
        let attacked_player = match target {
            crate::combat_state::AttackTarget::Player(player) => Some(*player),
            crate::combat_state::AttackTarget::Planeswalker(planeswalker) => {
                if attacked_target_must_be_player {
                    None
                } else {
                    ctx.game
                        .object(*planeswalker)
                        .map(|planeswalker| ctx.game.controller_of(planeswalker))
                }
            }
        };
        attacked_player
            .is_some_and(|player| attacked_player_filter.matches_player(player, &ctx.filter_ctx))
    }
}

fn pluralize_attack_subject(subject: &str) -> String {
    if subject == "creature" {
        return "creatures".to_string();
    }
    if let Some(rest) = subject.strip_prefix("creature ") {
        return format!("creatures {rest}");
    }
    subject.to_string()
}

impl TriggerMatcher for AttacksTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        let Some(obj) = ctx.game.object(e.attacker) else {
            return false;
        };
        let attack_target = match e.target {
            crate::events::combat::AttackEventTarget::Player(player) => {
                crate::combat_state::AttackTarget::Player(player)
            }
            crate::events::combat::AttackEventTarget::Planeswalker(planeswalker) => {
                crate::combat_state::AttackTarget::Planeswalker(planeswalker)
            }
        };
        if !self.matches_attacker_object_and_target(obj, &attack_target, ctx) {
            return false;
        }
        if e.total_attackers < self.min_total_attackers {
            return false;
        }
        if self.one_or_more {
            return self.is_first_matching_attacker_this_combat(e.attacker, &attack_target, ctx);
        }
        true
    }

    fn display(&self) -> String {
        let mut display_filter = self.filter.clone();
        // Attacking already implies a creature; oracle says "another Cat you
        // control attacks", not "another Cat creature you control attacks".
        if display_filter.card_types == [crate::types::CardType::Creature]
            && !display_filter.subtypes.is_empty()
            && display_filter.all_card_types.is_empty()
        {
            display_filter.card_types.clear();
        }
        let attacked_player = display_filter
            .attacking_player_or_planeswalker_controlled_by
            .take();
        let attacked_target_must_be_player = display_filter.targets_only_player.take().is_some();
        let mut subject = display_filter.description();
        if let Some(stripped) = subject.strip_prefix("a ") {
            subject = stripped.to_string();
        } else if let Some(stripped) = subject.strip_prefix("an ") {
            subject = stripped.to_string();
        }
        let base_subject = subject.clone();
        let subject = if self.one_or_more {
            pluralize_one_or_more_attack_subject(&subject)
        } else {
            subject
        };
        let target_tail = match (attacked_player.as_ref(), attacked_target_must_be_player) {
            (Some(PlayerFilter::Opponent), true) => " an opponent",
            (Some(PlayerFilter::Any), true) => " a player",
            (Some(PlayerFilter::Opponent), false) => {
                " an opponent or a planeswalker controlled by an opponent"
            }
            (Some(PlayerFilter::You), true) => " you",
            _ => "",
        };

        if self.one_or_more {
            if self.min_total_attackers > 1 {
                if display_filter.source {
                    let other_count = self.min_total_attackers.saturating_sub(1) as u32;
                    let other_text = ironsmith_core::cardinal_word(other_count)
                        .unwrap_or_else(|| other_count.to_string());
                    return format!(
                        "Whenever this creature and at least {other_text} other creatures attack{target_tail}"
                    );
                }
                let min_total = ironsmith_core::cardinal_word(self.min_total_attackers as u32)
                    .unwrap_or_else(|| self.min_total_attackers.to_string());
                if let Some(controlled_subject) = subject.strip_suffix(" you control") {
                    return format!(
                        "Whenever you attack with {min_total} or more {}",
                        pluralize_attack_subject(controlled_subject)
                    );
                }
                return format!("Whenever {min_total} or more {subject} attack{target_tail}");
            }
            if subject == "creature you control" && target_tail.is_empty() {
                return "Whenever you attack".to_string();
            }
            if base_subject == "creature you control"
                && matches!(attacked_player.as_ref(), Some(PlayerFilter::Any))
                && attacked_target_must_be_player
            {
                return "Whenever you attack a player".to_string();
            }
            if base_subject == "creature an opponent controls"
                && matches!(attacked_player.as_ref(), Some(PlayerFilter::Opponent))
                && attacked_target_must_be_player
            {
                return "Whenever an opponent attacks another one of your opponents".to_string();
            }
            return format!("Whenever one or more {subject} attack{target_tail}");
        }
        if self.min_total_attackers > 1 {
            return format!(
                "Whenever {} or more {subject} attack{target_tail}",
                self.min_total_attackers,
            );
        }
        if display_filter.source {
            return format!("Whenever this creature attacks{target_tail}");
        }
        format!(
            "Whenever {} attacks{target_tail}",
            display_filter.description()
        )
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        if !self.one_or_more || !self.matches(event, ctx) {
            return None;
        }
        self.matching_attacker_count_this_combat(ctx)
    }
}

impl TriggerMatcher for PlayersAttackedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        let attack_target = match e.target {
            crate::events::combat::AttackEventTarget::Player(player) => {
                crate::combat_state::AttackTarget::Player(player)
            }
            crate::events::combat::AttackEventTarget::Planeswalker(planeswalker) => {
                crate::combat_state::AttackTarget::Planeswalker(planeswalker)
            }
        };
        self.is_first_matching_attacker_this_combat(e.attacker, &attack_target, ctx)
    }

    fn display(&self) -> String {
        match &self.player_filter {
            PlayerFilter::Opponent => {
                "Whenever one or more of your opponents are attacked".to_string()
            }
            PlayerFilter::You => "Whenever you are attacked".to_string(),
            PlayerFilter::Any => "Whenever one or more players are attacked".to_string(),
            _ => "Whenever one or more matching players are attacked".to_string(),
        }
    }
}

fn defending_player_for_attack_target(
    target: &crate::combat_state::AttackTarget,
    game: &crate::game_state::GameState,
) -> Option<crate::ids::PlayerId> {
    match target {
        crate::combat_state::AttackTarget::Player(player) => Some(*player),
        crate::combat_state::AttackTarget::Planeswalker(planeswalker) => game
            .object(*planeswalker)
            .map(|planeswalker| game.controller_of(planeswalker)),
    }
}

fn pluralize_one_or_more_attack_subject(subject: &str) -> String {
    // Bare subtype subjects ("Dragon you control") pluralize the subtype.
    if !subject.contains(" creature")
        && let Some((head, tail)) = subject.split_once(' ')
        && !head.is_empty()
        && head
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        && !head.ends_with('s')
        && (tail.starts_with("you ")
            || tail.starts_with("an opponent ")
            || tail.starts_with("your "))
    {
        return format!("{head}s {tail}");
    }
    if let Some((head, tail)) = subject.split_once(" creature ") {
        if !head.contains(' ')
            && head
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            return format!("{head}s {tail}");
        }
        return format!("{head} creatures {tail}");
    }
    if let Some(stripped) = subject.strip_suffix(" creature") {
        if !stripped.contains(' ')
            && stripped
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            return format!("{stripped}s");
        }
        return format!("{stripped} creatures");
    }
    if let Some((head, tail)) = subject.split_once(" permanent ") {
        return format!("{head} permanents {tail}");
    }
    if let Some(stripped) = subject.strip_suffix(" permanent") {
        return format!("{stripped} permanents");
    }
    subject.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::events::combat::AttackEventTarget;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::AttachmentTarget;
    use crate::target::PlayerFilter;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn test_matches_creature_attack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);

        let trigger = AttacksTrigger::any_creature();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(creature_id, AttackEventTarget::Player(bob)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = AttacksTrigger::any_creature();
        assert!(trigger.display().contains("attacks"));
    }

    #[test]
    fn one_or_more_you_control_attack_player_displays_you_attack_player() {
        let mut filter = ObjectFilter::creature().you_control();
        filter.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::Any);
        filter.targets_only_player = Some(PlayerFilter::Any);
        let trigger = AttacksTrigger::one_or_more(filter);

        assert_eq!(trigger.display(), "Whenever you attack a player");
    }

    #[test]
    fn one_or_more_opponent_controls_attack_opponent_displays_opponent_attacks_another() {
        let mut filter = ObjectFilter::creature();
        filter.controller = Some(PlayerFilter::Opponent);
        filter.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::Opponent);
        filter.targets_only_player = Some(PlayerFilter::Opponent);
        let trigger = AttacksTrigger::one_or_more(filter);

        assert_eq!(
            trigger.display(),
            "Whenever an opponent attacks another one of your opponents"
        );
    }

    #[test]
    fn test_one_or_more_only_matches_first_attacker_in_declaration() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let attacker_one = create_creature(&mut game, "A", alice);
        let attacker_two = create_creature(&mut game, "B", alice);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_one,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: attacker_two,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let trigger = AttacksTrigger::one_or_more(ObjectFilter::creature());
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let first_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&first_event, &ctx));

        let second_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_two,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&second_event, &ctx));
    }

    #[test]
    fn one_or_more_attack_an_opponent_matches_first_attacker_for_each_opponent() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let source_id = ObjectId::from_raw(100);
        let bob_attacker_one = create_creature(&mut game, "A", alice);
        let bob_attacker_two = create_creature(&mut game, "B", alice);
        let charlie_attacker = create_creature(&mut game, "C", alice);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: bob_attacker_one,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: bob_attacker_two,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: charlie_attacker,
            target: AttackTarget::Player(charlie),
        });
        game.combat = Some(combat);

        let mut filter = ObjectFilter::creature().you_control();
        filter.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::Opponent);
        filter.targets_only_player = Some(PlayerFilter::Any);
        let trigger = AttacksTrigger::one_or_more(filter);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let first_bob_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                bob_attacker_one,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&first_bob_event, &ctx));

        let second_bob_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                bob_attacker_two,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&second_bob_event, &ctx));

        let charlie_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                charlie_attacker,
                AttackEventTarget::Player(charlie),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&charlie_event, &ctx));
    }

    #[test]
    fn players_attacked_one_or_more_matches_first_attacker_across_all_opponents() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let source_id = ObjectId::from_raw(100);
        let bob_attacker = create_creature(&mut game, "A", bob);
        let charlie_attacker = create_creature(&mut game, "B", charlie);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: bob_attacker,
            target: AttackTarget::Player(charlie),
        });
        combat.attackers.push(AttackerInfo {
            creature: charlie_attacker,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let trigger = PlayersAttackedTrigger::one_or_more(PlayerFilter::Opponent);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert_eq!(
            trigger.display(),
            "Whenever one or more of your opponents are attacked"
        );

        let first_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                bob_attacker,
                AttackEventTarget::Player(charlie),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&first_event, &ctx));

        let second_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                charlie_attacker,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&second_event, &ctx));

        let controller_attacked_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                bob_attacker,
                AttackEventTarget::Player(alice),
                1,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&controller_attacked_event, &ctx));
    }

    #[test]
    fn one_or_more_can_match_attackers_attacking_enchanted_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_card = CardBuilder::new(CardId::from_raw(900), "Curse Probe")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("source should exist")
            .attached_to = Some(AttachmentTarget::Player(bob));
        let attacker = create_creature(&mut game, "A", alice);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let mut filter = ObjectFilter::creature();
        filter.controller = Some(PlayerFilter::Any);
        filter.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::TaggedPlayer(
            crate::tag::TagKey::from("enchanted"),
        ));
        let trigger = AttacksTrigger::one_or_more(filter);
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker,
                AttackEventTarget::Player(bob),
                1,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&event, &ctx));
        drop(ctx);

        let walker_card = CardBuilder::new(CardId::from_raw(901), "Walker")
            .card_types(vec![CardType::Planeswalker])
            .build();
        let walker = game.create_object_from_card(&walker_card, bob, Zone::Battlefield);
        let walker_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker,
                AttackEventTarget::Planeswalker(walker),
                1,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&walker_event, &ctx));

        let mut player_only_filter = trigger.filter.clone();
        player_only_filter.targets_only_player = Some(PlayerFilter::Any);
        let player_only_trigger = AttacksTrigger::one_or_more(player_only_filter);
        assert!(!player_only_trigger.matches(&walker_event, &ctx));
    }

    #[test]
    fn test_one_or_more_with_min_total_attackers_requires_threshold() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let attacker_one = create_creature(&mut game, "A", alice);
        let attacker_two = create_creature(&mut game, "B", alice);
        let attacker_three = create_creature(&mut game, "C", alice);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_one,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: attacker_two,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: attacker_three,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let trigger =
            AttacksTrigger::one_or_more_with_min_total_attackers(ObjectFilter::creature(), 3);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let below_threshold = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&below_threshold, &ctx));

        let first_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&first_event, &ctx));

        let second_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_two,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&second_event, &ctx));
    }

    #[test]
    fn test_one_or_more_event_value_counts_matching_attackers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let attacker_one = create_creature(&mut game, "A", alice);
        let attacker_two = create_creature(&mut game, "B", alice);
        let other_attacker = create_creature(&mut game, "C", bob);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_one,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: attacker_two,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: other_attacker,
            target: AttackTarget::Player(alice),
        });
        game.combat = Some(combat);

        let trigger = AttacksTrigger::one_or_more(ObjectFilter::creature().you_control());
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );

        assert_eq!(trigger.event_value_amount(&event, &ctx), Some(2));
    }
}
