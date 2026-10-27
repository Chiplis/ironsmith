//! Native announcement ownership for continuous top-card visibility. The
//! boundary travels with exact GameState clones, including a resolving effect's
//! local cast/payment state, rather than depending on a host's Undo cursor.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LibraryTopAnnouncement {
    Cast(ObjectId),
    Activation(crate::provenance::ProvNodeId),
    Land(ObjectId),
}
#[derive(Debug, Clone)]
pub(crate) struct LibraryTopVisibilityBoundary {
    tops: HashMap<PlayerId, (Option<ObjectId>, u64)>,
    audit_checkpoint: usize,
    drawn_cards: HashSet<ObjectId>,
}
#[derive(Debug, Clone, Default)]
pub(super) struct LibraryTopIdentityState {
    revealed: BTreeMap<PlayerId, ObjectId>,
    pub(super) pending_reorders: std::collections::BTreeSet<ObjectId>,
    concealed: HashSet<ObjectId>,
    pub(super) reconciling: bool,
}

impl GameState {
    /// The public reveal is a continuous rule. Merely letting one player look
    /// at their top card does not create a public reveal epoch (CR 401.6).
    fn library_top_is_publicly_revealed(&self, player: PlayerId) -> bool {
        use crate::static_abilities::StaticAbilityId as Id;
        self.static_library_top_visible_during_announcements(player)
            && self.battlefield.iter().any(|source| {
                !self.is_phased_out(*source) && self.object(*source).is_some_and(|object| {
                    self.object_has_static_ability_id(*source, Id::AllPlayersLookAtTopCardsOfLibraries)
                        || (self.current_controller(*source).unwrap_or(object.owner) == player
                            && self.object_has_static_ability_id(*source, Id::AllPlayersLookAtYourTopLibraryCard))
                })
            })
    }

    pub(super) fn revealed_library_top_before_shuffle(&mut self, player: PlayerId) -> Option<ObjectId> {
        self.reconcile_library_top_identity();
        self.runtime_cache.library_top_identity.revealed.get(&player).copied()
            .filter(|id| self.player(player).is_some_and(|state| state.library.contains(id)))
    }

    /// Track actual concealment and renew a returning revealed incarnation.
    /// Runs at library mutations and continuous refresh, including source
    /// ability/control changes; it never infers epochs from UI inspections.
    pub(crate) fn reconcile_library_top_identity(&mut self) -> bool {
        if self.runtime_cache.library_top_identity.reconciling || self.has_open_simultaneous_action() { return false; }
        if self.runtime_cache.library_top_identity.revealed.is_empty()
            && self.runtime_cache.library_top_identity.concealed.is_empty()
            && self.runtime_cache.library_top_identity.pending_reorders.is_empty()
            && self.players.iter().all(|player| player.library.is_empty())
        { return false; }
        self.runtime_cache.library_top_identity.reconciling = true;
        let reordered = std::mem::take(&mut self.runtime_cache.library_top_identity.pending_reorders);
        for card in reordered { self.reidentify_reordered_library_card(card); }
        let mut revealed = self.players.iter().filter_map(|player| {
            let top = player.library.last().copied()?;
            self.library_top_is_publicly_revealed(player.id).then_some((player.id, top))
        }).collect::<BTreeMap<_, _>>();
        let previous = std::mem::take(&mut self.runtime_cache.library_top_identity.revealed);
        for (player, id) in previous {
            if revealed.get(&player) != Some(&id)
                && self.object(id).is_some_and(|object| object.zone == Zone::Library)
            {
                self.runtime_cache.library_top_identity.concealed.insert(id);
            }
        }
        let mut changed = false;
        for id in revealed.values_mut() {
            if self.runtime_cache.library_top_identity.concealed.contains(id)
                && let Some(new_id) = self.reidentify_reordered_library_card(*id)
            {
                self.runtime_cache.library_top_identity.concealed.remove(id);
                *id = new_id;
                changed = true;
            }
        }
        let concealed = std::mem::take(&mut self.runtime_cache.library_top_identity.concealed);
        self.runtime_cache.library_top_identity.concealed = concealed.into_iter()
            .filter(|id| self.object(*id).is_some_and(|object| object.zone == Zone::Library))
            .collect();
        self.runtime_cache.library_top_identity.revealed = revealed;
        self.runtime_cache.library_top_identity.reconciling = false;
        changed
    }

    pub(crate) fn capture_library_top_visibility_boundary(&self) -> LibraryTopVisibilityBoundary {
        LibraryTopVisibilityBoundary {
            tops: self.players.iter().map(|player| (player.id,
                (player.library.last().copied(), self.library_top_revision(player.id)))).collect(),
            audit_checkpoint: self.crypto_audit_checkpoint(),
            drawn_cards: HashSet::new(),
        }
    }
    pub(crate) fn register_library_top_announcement(&mut self, owner: LibraryTopAnnouncement, boundary: LibraryTopVisibilityBoundary) {
        self.retain_duration_window(owner);
        self.runtime_cache.library_top_announcements.entry(owner).or_insert(boundary);
    }
    pub(crate) fn begin_library_top_announcement(&mut self, owner: LibraryTopAnnouncement) {
        if !self.runtime_cache.library_top_announcements.contains_key(&owner) {
            let boundary = self.capture_library_top_visibility_boundary();
            self.register_library_top_announcement(owner, boundary);
        }
    }
    pub(crate) fn finish_library_top_announcement(&mut self, owner: LibraryTopAnnouncement) {
        let boundary = self.runtime_cache.library_top_announcements.remove(&owner);
        if boundary.is_some_and(|boundary| !boundary.drawn_cards.is_empty()) {
            self.mark_continuous_state_dirty();
        }
        if let LibraryTopAnnouncement::Land(card) = owner { self.runtime_cache.pending_land_permission_grants.remove(&card); }
        self.reconcile_library_top_identity();
    }
    /// CR 121.8 tracks the exact drawn incarnation under every enclosing
    /// casting/activation announcement. Nested mana activations cannot release
    /// a card while their parent announcement still owns it.
    pub(crate) fn retain_announcement_draw(&mut self, card: ObjectId) {
        let mut retained = false;
        for (owner, boundary) in &mut self.runtime_cache.library_top_announcements {
            if matches!(owner, LibraryTopAnnouncement::Cast(_) | LibraryTopAnnouncement::Activation(_)) {
                retained |= boundary.drawn_cards.insert(card);
            }
        }
        if retained { self.mark_continuous_state_dirty(); }
    }

    pub(crate) fn drawn_card_waits_for_announcement(&self, card: ObjectId) -> bool {
        self.runtime_cache.library_top_announcements.values()
            .any(|boundary| boundary.drawn_cards.contains(&card))
            && self.object(card).is_some_and(|object| object.zone == Zone::Hand)
    }

    /// The draw-time reveal entitlement reads the drawn card before CR 121.8
    /// conceals its characteristics. This private view never publishes them.
    pub(crate) fn draw_reveal_characteristics_view(&self, card: ObjectId) -> Self {
        let mut view = self.clone();
        for boundary in view.runtime_cache.library_top_announcements.values_mut() {
            boundary.drawn_cards.remove(&card);
        }
        view.mark_continuous_state_dirty();
        view
    }

    pub(crate) fn stage_land_permission_grants(&mut self, card: ObjectId, abilities: Vec<StaticAbility>) {
        if !abilities.is_empty() { self.runtime_cache.pending_land_permission_grants.insert(card, abilities); }
    }
    pub(crate) fn capture_cast_grant_completion(&mut self, spell: ObjectId, completion: crate::grant_registry::GrantUseCompletion) {
        self.runtime_cache.pending_grant_use_completions.entry(spell).or_default().push(completion);
    }
    pub(crate) fn complete_cast_grant(&mut self, spell: ObjectId) {
        if let Some(completions) = self.runtime_cache.pending_grant_use_completions.remove(&spell) {
            for completion in completions { completion.complete(self); }
        }
    }
    pub fn has_library_top_announcement(&self) -> bool {
        !self.runtime_cache.library_top_announcements.is_empty() || !self.runtime_cache.pending_grant_use_completions.is_empty() || !self.runtime_cache.pending_land_permission_grants.is_empty()
    }
    pub fn static_library_top_visible_during_announcements(&self, player: PlayerId) -> bool {
        let current = self.player(player).and_then(|player| player.library.last().copied());
        self.runtime_cache.library_top_announcements.values().all(|boundary| {
            boundary.tops.get(&player).is_some_and(|(top, revision)|
                *top == current && *revision == self.library_top_revision(player))
                && !self.crypto_audit_operations_since(boundary.audit_checkpoint).iter().any(|operation|
                    matches!(operation, HiddenInfoOperation::LibraryShuffle {player: owner, ..} if *owner == player))
        })
    }
}

#[cfg(test)]
mod deferred_draw_tests {
    use super::*;
    use crate::cards::CardDefinitionBuilder;

    #[test]
    fn nested_announcement_draws_keep_exact_identity_and_clone_ownership() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let player = PlayerId::from_index(0);
        let card = CardDefinitionBuilder::new(CardId::new(), "Drawn creature")
            .card_types(vec![crate::types::CardType::Creature])
            .mana_cost(crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Blue]))
            .power_toughness(crate::card::PowerToughness::fixed(2, 3))
            .build();
        game.create_object_from_definition(&card, player, Zone::Library);
        let cast = LibraryTopAnnouncement::Cast(ObjectId(900));
        let child = LibraryTopAnnouncement::Cast(ObjectId(901));
        game.begin_library_top_announcement(cast);
        game.begin_library_top_announcement(child);
        let drawn = game.draw_cards(player, 1)[0];
        game.finish_library_top_announcement(child);
        assert!(game.is_face_down(drawn));
        assert!(game.current_characteristics(drawn).unwrap().card_types.is_empty());
        assert!(game.try_current_characteristics(drawn).unwrap().unwrap().name.is_empty());
        let mut fork = game.clone();
        game.finish_library_top_announcement(cast);
        assert!(!game.is_face_down(drawn));
        assert_eq!(game.current_name(drawn).as_deref(), Some("Drawn creature"));
        assert!(fork.current_characteristics(drawn).unwrap().card_types.is_empty());
        let grave = fork.move_object_by_effect(drawn, Zone::Graveyard).unwrap();
        assert_ne!(drawn, grave);
        assert_eq!(fork.current_name(grave).as_deref(), Some("Drawn creature"));
        let returned = fork.move_object_by_effect(grave, Zone::Hand).unwrap();
        assert!(!fork.drawn_card_waits_for_announcement(returned));
        assert_eq!(fork.current_name(returned).as_deref(), Some("Drawn creature"));
    }

    #[test]
    fn deferred_native_miracle_and_optional_draw_reveals_wait_for_announcement() {
        use crate::decision::DecisionMaker;
        use crate::decisions::context::{SelectOptionsContext, BooleanContext, ViewCardsContext};
        use crate::effects::{DrawCardsEffect, EffectExecutor, ExecutionContext};
        struct Choices { miracles: usize, views: usize }
        impl DecisionMaker for Choices {
            fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
                if context.description == "Choose a Miracle reveal" {
                    assert!(!game.has_library_top_announcement());
                    self.miracles += 1;
                    vec![1]
                } else { context.options.iter().filter(|option| option.legal).take(context.min).map(|option| option.index).collect() }
            }
            fn decide_boolean(&mut self, game: &GameState, _: &BooleanContext) -> bool {
                assert!(!game.has_library_top_announcement());
                true
            }
            fn view_cards(&mut self, game: &GameState, _: PlayerId, cards: &[ObjectId], context: &ViewCardsContext) {
                if context.public {
                    assert!(!game.has_library_top_announcement());
                    for card in cards { assert!(!game.current_name(*card).unwrap().is_empty()); }
                    self.views += 1;
                }
            }
        }
        for optional in [false, true] {
            for miracle in 0..3 {
              for observe_before_finish in [false, true] {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let player = PlayerId::from_index(0);
                game.set_starting_player(player);
                game.turn.phase = Phase::FirstMain;
                game.turn.step = None;
                let keeper = CardDefinitionBuilder::new(CardId::new(), "Priority keeper")
                    .card_types(vec![crate::types::CardType::Artifact])
                    .with_ability(crate::ability::Ability::activated(crate::cost::TotalCost::free(), vec![crate::effect::Effect::gain_life(0)]))
                    .build();
                for owner in [player, PlayerId::from_index(1)] { game.create_object_from_definition(&keeper, owner, Zone::Battlefield); }
                let revealer = CardDefinitionBuilder::new(CardId::new(), "Draw revealer")
                    .card_types(vec![crate::types::CardType::Enchantment])
                    .with_ability(crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::reveal_first_card_you_draw_each_turn(optional, false)))
                    .build();
                let source = game.create_object_from_definition(&revealer, player, Zone::Battlefield);
                let mut card = CardDefinitionBuilder::new(CardId::new(), "Deferred miracle")
                    .card_types(vec![crate::types::CardType::Instant])
                    .mana_cost(crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Blue]));
                if miracle == 1 { card = card.miracle(crate::mana::ManaCost::new()); }
                let granter = if miracle == 2 {
                    let grant = CardDefinitionBuilder::new(CardId::new(), "Draw-time Miracle grant")
                        .card_types(vec![crate::types::CardType::Artifact])
                        .with_ability(crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::grants(
                            crate::grant::GrantSpec::new(
                                crate::grant::Grantable::AlternativeCast(crate::alternative_cast::AlternativeCastingMethod::Miracle { cost: crate::mana::ManaCost::new() }),
                                crate::target::ObjectFilter { card_types: vec![crate::types::CardType::Instant], ..Default::default() },
                                Zone::Hand,
                            )
                        ))).build();
                    Some(game.create_object_from_definition(&grant, player, Zone::Battlefield))
                } else { None };
                game.create_object_from_definition(&card.build(), player, Zone::Library);
                let owner = LibraryTopAnnouncement::Cast(ObjectId(900));
                game.begin_library_top_announcement(owner);
                let mut choices = Choices { miracles: 0, views: 0 };
                let mut ctx = ExecutionContext::new(source, player, &mut choices);
                DrawCardsEffect::you(1).execute(&mut game, &mut ctx).unwrap();
                assert_eq!((choices.miracles, choices.views), (0, 0));
                // A later casting cost can remove the grant after the draw.
                // The already-created reveal opportunity keeps its own price.
                if let Some(granter) = granter { game.move_object_by_effect(granter, Zone::Graveyard).unwrap(); }
                let mut queue = crate::triggers::TriggerQueue::new();
                if observe_before_finish {
                    crate::game_loop::advance_priority_with_dm(&mut game, &mut queue, &mut choices).unwrap();
                    assert_eq!((choices.miracles, choices.views), (0, 0));
                }
                game.finish_library_top_announcement(owner);
                crate::game_loop::advance_priority_with_dm(&mut game, &mut queue, &mut choices).unwrap();
                assert_eq!(choices.miracles, usize::from(miracle != 0));
                assert_eq!(choices.views, if miracle != 0 { 4 } else { 2 });
                assert_eq!(game.stack.len(), usize::from(miracle != 0));
              }
            }
        }
    }
}

#[cfg(test)]
#[path = "revealed_top_identity_tests.rs"]
mod revealed_top_identity_tests;
