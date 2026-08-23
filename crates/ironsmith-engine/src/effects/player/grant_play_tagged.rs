//! Grant temporary "you may cast/play this tagged card" permissions.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::grant::Grantable;
use crate::grant_registry::{GrantSource, PlayFromConstraints};
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
pub use ironsmith_core::GrantPlayTaggedDuration;

/// Grant temporary permission to cast or play cards tagged in the current context.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantPlayTaggedEffect {
    pub tag: TagKey,
    pub player: PlayerFilter,
    pub duration: GrantPlayTaggedDuration,
    /// Authored duration placement and tagged-card reference wording.
    /// Gameplay semantics remain in the ordinary typed grant fields.
    pub surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
    pub allow_land: bool,
    pub mana_spend_mode: ironsmith_core::value_model::ManaSpendMode,
    /// Compatibility predicate for older compiled-text pattern matchers.
    /// True for both `AnyColor` and `AnyType`.
    pub allow_any_color_for_cast: bool,
    pub while_on_top_of_library: bool,
    pub filter: Option<ObjectFilter>,
    /// When present, the persistent grant is active only on turns in which
    /// this counter type was put on the resolving ability's source.
    pub during_turns_counter_put_on_source: Option<crate::object::CounterType>,
    /// Additional mana cost imposed on nonland cards cast through this exact
    /// tagged play permission.
    pub spell_cost_increase: Option<crate::mana::ManaCost>,
    /// Whether a land played through this exact tagged permission enters
    /// tapped.
    pub lands_enter_tapped: bool,
    /// True when the granted pool holds more than one card, selecting plural
    /// "cast spells from among those exiled cards" wording over the singular
    /// "cast that card this turn". Purely cosmetic; resolution is unaffected.
    pub cast_pool_is_plural: bool,
    /// Total number of plays shared by the tagged collection. The choice of
    /// card is deferred until a card is actually played.
    pub max_plays: Option<u32>,
}

impl GrantPlayTaggedEffect {
    pub fn new(
        tag: impl Into<TagKey>,
        player: PlayerFilter,
        duration: GrantPlayTaggedDuration,
        allow_land: bool,
        mana_spend_mode: impl Into<ironsmith_core::value_model::ManaSpendMode>,
    ) -> Self {
        let mana_spend_mode = mana_spend_mode.into();
        Self {
            tag: tag.into(),
            player,
            duration,
            surface: None,
            allow_land,
            mana_spend_mode,
            allow_any_color_for_cast: mana_spend_mode.allows_any_color(),
            while_on_top_of_library: false,
            filter: None,
            during_turns_counter_put_on_source: None,
            spell_cost_increase: None,
            lands_enter_tapped: false,
            cast_pool_is_plural: false,
            max_plays: None,
        }
    }

    pub fn cast_pool_is_plural(mut self, plural: bool) -> Self {
        self.cast_pool_is_plural = plural;
        self
    }

    pub fn with_max_plays(mut self, max_plays: Option<u32>) -> Self {
        self.max_plays = max_plays;
        self
    }

    pub fn with_surface(mut self, surface: ironsmith_core::GrantPlayTaggedSurface) -> Self {
        self.surface = Some(surface);
        self
    }

    pub fn with_mana_spend_mode(
        mut self,
        mode: ironsmith_core::value_model::ManaSpendMode,
    ) -> Self {
        self.mana_spend_mode = mode;
        self.allow_any_color_for_cast = mode.allows_any_color();
        self
    }

    /// Oracle clause appended to a temporary cast permission.
    pub fn mana_spend_cast_clause(&self, spell_reference: &str) -> Option<String> {
        match self.mana_spend_mode {
            ironsmith_core::value_model::ManaSpendMode::Normal => None,
            ironsmith_core::value_model::ManaSpendMode::AnyColor => Some(format!(
                "you may spend mana as though it were mana of any color to cast {spell_reference}"
            )),
            ironsmith_core::value_model::ManaSpendMode::AnyType => Some(format!(
                "mana of any type can be spent to cast {spell_reference}"
            )),
        }
    }

    pub fn while_on_top_of_library(mut self) -> Self {
        self.while_on_top_of_library = true;
        self
    }

    pub fn while_on_top_of_library_if(mut self, enabled: bool) -> Self {
        self.while_on_top_of_library = enabled;
        self
    }

    pub fn with_filter(mut self, filter: ObjectFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn during_turns_counter_put_on_source(
        mut self,
        counter_type: crate::object::CounterType,
    ) -> Self {
        self.during_turns_counter_put_on_source = Some(counter_type);
        self
    }

    pub fn with_spell_cost_increase(mut self, cost: crate::mana::ManaCost) -> Self {
        self.spell_cost_increase = Some(cost);
        self
    }

    pub fn with_lands_enter_tapped(mut self, enabled: bool) -> Self {
        self.lands_enter_tapped = enabled;
        self
    }

    pub fn until_your_next_turn(tag: impl Into<TagKey>, player: PlayerFilter) -> Self {
        Self::new(
            tag,
            player,
            GrantPlayTaggedDuration::UntilYourNextTurnEnd,
            true,
            false,
        )
    }

    /// Compute the turn number corresponding to the end of `player`'s next turn.
    ///
    /// This simulates `GameState::next_turn` turn selection logic (including
    /// multiplayer turn order, queued extra turns, and skipped turns) without
    /// mutating game state.
    fn next_turn_number_for_player(game: &GameState, player: crate::ids::PlayerId) -> u32 {
        game.next_turn_number_if_player_stayed(player)
    }

    fn expires_end_of_turn(&self, game: &GameState, player: crate::ids::PlayerId) -> u32 {
        match self.duration {
            GrantPlayTaggedDuration::UntilEndOfTurn => game.turn.turn_number,
            GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
                Self::next_turn_number_for_player(game, player)
            }
            GrantPlayTaggedDuration::UntilYourNextEndStep => {
                // The grant registry stores an end-of-turn boundary rather
                // than a phase-step marker. Keep the same one-turn boundary
                // used by the tagged free-cast implementation so a grant
                // created during the active player's turn remains visible
                // until the next end-step window is crossed.
                game.turn.turn_number.saturating_add(1)
            }
            GrantPlayTaggedDuration::UntilSourceExilesAnother => u32::MAX,
            GrantPlayTaggedDuration::ForAsLongAsExiled => u32::MAX,
            GrantPlayTaggedDuration::ForAsLongAsYouControlSource => u32::MAX,
        }
    }
}

impl EffectExecutor for GrantPlayTaggedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_is_each_tagged_owner = matches!(
            &self.player,
            PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(tag))
                | PlayerFilter::AliasedOwnerOf(crate::target::ObjectRef::Tagged(tag))
                if tag == &self.tag
        );
        let fixed_player_id = if player_is_each_tagged_owner {
            None
        } else {
            Some(resolve_player_filter(game, &self.player, ctx)?)
        };
        let snapshots = ctx.get_tagged_all(self.tag.as_str()).cloned().or_else(|| {
            (self.tag.as_str() == "__source_exiled__").then(|| {
                ctx.tagged_objects
                    .iter()
                    .filter(|(tag, _)| tag.as_str().starts_with("__sentence_helper_exiled"))
                    .flat_map(|(_, snapshots)| snapshots.iter().cloned())
                    .collect::<Vec<_>>()
            })
        });
        let Some(snapshots) = snapshots.filter(|snapshots| !snapshots.is_empty()) else {
            return Ok(EffectOutcome::count(0));
        };

        let mut granted = 0usize;
        let mut seen = std::collections::HashSet::new();
        let mut shared_usage_by_player = std::collections::HashMap::new();
        let mut mana_permission_stable_ids =
            std::collections::HashMap::<crate::ids::PlayerId, Vec<crate::ids::StableId>>::new();
        for snapshot in snapshots {
            let mut object_id = snapshot.object_id;
            if game.object(object_id).is_none() {
                if let Some(found) = game.find_object_by_stable_id(snapshot.stable_id) {
                    object_id = found;
                } else {
                    continue;
                }
            }

            let Some(object) = game.object(object_id) else {
                continue;
            };
            let filter_ctx = ctx.filter_context(game);
            if self
                .filter
                .as_ref()
                .is_some_and(|filter| !filter.matches(object, &filter_ctx, game))
            {
                continue;
            }
            let object_is_land = object.is_land();
            if (!self.allow_land && object_is_land) || !seen.insert(object_id) {
                continue;
            }
            let object_stable_id = object.stable_id;
            let object_zone = object.zone;
            let object_owner = object.owner;
            let player_id = fixed_player_id.unwrap_or(object_owner);
            let expires_end_of_turn = self.expires_end_of_turn(game, player_id);

            if self.mana_spend_mode.allows_any_color() && !object_is_land {
                mana_permission_stable_ids
                    .entry(player_id)
                    .or_default()
                    .push(object_stable_id);
            }

            let source = if let Some(counter_type) = self.during_turns_counter_put_on_source {
                GrantSource::EffectDuringTurnsCounterPutOnSource {
                    source_id: ctx.source,
                    counter_type,
                }
            } else if self.while_on_top_of_library {
                GrantSource::EffectWhileStableCardOnTopOfLibrary {
                    source_id: ctx.source,
                    expires_end_of_turn,
                    stable_id: object_stable_id,
                    player: object_owner,
                    library_top_revision: game.library_top_revision(object_owner),
                }
            } else if self.duration == GrantPlayTaggedDuration::ForAsLongAsYouControlSource {
                GrantSource::EffectWhileControlled {
                    source_id: ctx.source,
                    controller: player_id,
                }
            } else if self.duration == GrantPlayTaggedDuration::UntilSourceExilesAnother {
                GrantSource::until_source_exiles_another(
                    ctx.source,
                    game.exiled_with_source_revision(ctx.source),
                )
            } else if self.duration == GrantPlayTaggedDuration::UntilYourNextTurnEnd {
                GrantSource::until_player_next_turn_end(ctx.source, player_id, expires_end_of_turn)
            } else {
                GrantSource::Effect {
                    source_id: ctx.source,
                    expires_end_of_turn,
                }
            };
            let constraints = PlayFromConstraints {
                spell_cost_increase: self.spell_cost_increase.clone(),
                lands_enter_tapped: self.lands_enter_tapped,
            };
            let shared_usage_id = self.max_plays.map(|max_plays| {
                *shared_usage_by_player.entry(player_id).or_insert_with(|| {
                    game.effect_store
                        .grant_registry
                        .create_shared_usage_budget(max_plays)
                })
            });
            if let Some(shared_usage_id) = shared_usage_id {
                let target_stable_id = ((constraints != PlayFromConstraints::default()
                    && self.duration != GrantPlayTaggedDuration::ForAsLongAsExiled)
                    || self.during_turns_counter_put_on_source.is_some())
                .then_some(object_stable_id);
                game.effect_store
                    .grant_registry
                    .grant_play_from_to_card_in_shared_budget(
                        object_id,
                        target_stable_id,
                        object_zone,
                        player_id,
                        constraints,
                        source,
                        shared_usage_id,
                    );
            } else if constraints != PlayFromConstraints::default() {
                if self.duration == GrantPlayTaggedDuration::ForAsLongAsExiled {
                    game.effect_store.grant_registry.grant_play_from_to_card(
                        object_id,
                        object_zone,
                        player_id,
                        constraints,
                        source,
                    );
                } else {
                    game.effect_store
                        .grant_registry
                        .grant_play_from_to_stable_card(
                            object_id,
                            object_stable_id,
                            object_zone,
                            player_id,
                            constraints,
                            source,
                        );
                }
            } else if self.duration == GrantPlayTaggedDuration::ForAsLongAsExiled {
                game.effect_store.grant_registry.grant_to_card(
                    object_id,
                    object_zone,
                    player_id,
                    Grantable::PlayFrom,
                    source,
                );
            } else if self.during_turns_counter_put_on_source.is_some() {
                game.effect_store.grant_registry.grant_to_stable_card(
                    object_id,
                    object_stable_id,
                    object_zone,
                    player_id,
                    Grantable::PlayFrom,
                    source,
                );
            } else {
                game.effect_store.grant_registry.grant_to_card(
                    object_id,
                    object_zone,
                    player_id,
                    Grantable::PlayFrom,
                    source,
                );
            }
            granted += 1;
        }

        for (player_id, mana_permission_stable_ids) in mana_permission_stable_ids {
            let permission = match self.mana_spend_mode {
                ironsmith_core::value_model::ManaSpendMode::Normal => {
                    unreachable!("normal mana spending does not collect permission stable ids")
                }
                ironsmith_core::value_model::ManaSpendMode::AnyColor => {
                    crate::effect::ManaSpendPermission::any_color_for_casting_stable_ids(
                        crate::target::PlayerFilter::You,
                        mana_permission_stable_ids,
                    )
                }
                ironsmith_core::value_model::ManaSpendMode::AnyType => {
                    crate::effect::ManaSpendPermission::any_type_for_casting_stable_ids(
                        crate::target::PlayerFilter::You,
                        mana_permission_stable_ids,
                    )
                }
            };
            game.effect_store.mana_spend_effects.permissions.push(
                crate::game_state::ActiveManaSpendPermission {
                    permission,
                    controller: player_id,
                    source: crate::game_state::ManaSpendPermissionSource::Effect {
                        source_id: ctx.source,
                        expires_end_of_turn: self.expires_end_of_turn(game, player_id),
                    },
                },
            );
        }

        Ok(EffectOutcome::count(granted as i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Zone;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use std::collections::HashSet;

    #[test]
    fn grant_play_tagged_until_your_next_turn_applies_to_tagged_exile_cards() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::from_raw(1), "Exiled Card").build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("exiled card"), &game);

        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let mut dm = SelectFirstDecisionMaker;
        let source = ObjectId::from_raw(100);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);

        let effect = GrantPlayTaggedEffect::until_your_next_turn("it", PlayerFilter::You);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                alice
            ),
            "tagged card should be playable from exile"
        );

        let grant = game
            .effect_store
            .grant_registry
            .grants
            .first()
            .expect("grant should exist");
        match grant.source {
            GrantSource::EffectUntilPlayerNextTurnEnd {
                expires_end_of_turn,
                duration_player,
                ..
            } => {
                assert_eq!(duration_player, alice);
                assert_eq!(
                    expires_end_of_turn,
                    game.turn.turn_number + 2,
                    "when cast on your own turn, permission should last through your next turn"
                );
            }
            _ => panic!("expected effect grant source"),
        }
    }

    #[test]
    fn shared_tagged_play_budget_is_consumed_by_first_cast() {
        use crate::alternative_cast::CastingMethod;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let first = CardBuilder::new(CardId::from_raw(11), "First Exiled Card").build();
        let second = CardBuilder::new(CardId::from_raw(12), "Second Exiled Card").build();
        let first_id = game.create_object_from_card(&first, alice, Zone::Exile);
        let second_id = game.create_object_from_card(&second, alice, Zone::Exile);
        let snapshots = [first_id, second_id]
            .into_iter()
            .map(|id| ObjectSnapshot::from_object(game.object(id).unwrap(), &game))
            .collect();
        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("witness_cards"), snapshots);

        let source = ObjectId::from_raw(101);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);
        GrantPlayTaggedEffect::until_your_next_turn("witness_cards", PlayerFilter::You)
            .with_max_plays(Some(1))
            .execute(&mut game, &mut ctx)
            .expect("shared tagged permission should resolve");

        assert!(game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            first_id,
            Zone::Exile,
            alice,
        ));
        assert!(game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            second_id,
            Zone::Exile,
            alice,
        ));

        crate::game_loop::propose_spell_cast(
            &mut game,
            first_id,
            Zone::Exile,
            alice,
            &CastingMethod::PlayFrom {
                source,
                zone: Zone::Exile,
                use_alternative: None,
            },
        )
        .expect("first card should use the shared permission");

        assert!(
            !game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                second_id,
                Zone::Exile,
                alice,
            ),
            "playing either card must exhaust the collection's shared budget"
        );
    }

    #[test]
    fn grant_play_tagged_until_your_next_turn_uses_multiplayer_turn_order() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);

        // Alice is active now. In a 3-player game, Alice's next turn ends at +3.
        game.turn.active_player = alice;
        game.turn.turn_number = 10;

        let expires = GrantPlayTaggedEffect::until_your_next_turn("it", PlayerFilter::You)
            .expires_end_of_turn(&game, alice);
        assert_eq!(
            expires, 13,
            "duration should last through Alice's next turn in multiplayer"
        );
    }

    #[test]
    fn grant_play_tagged_until_your_next_turn_respects_extra_and_skipped_turns() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Grant on Bob's turn with queued extra turn for Alice.
        game.turn.active_player = bob;
        game.turn.turn_number = 20;
        game.turn_store.extra_turns = vec![alice];
        let expires_with_extra =
            GrantPlayTaggedEffect::until_your_next_turn("it", PlayerFilter::You)
                .expires_end_of_turn(&game, alice);
        assert_eq!(
            expires_with_extra, 21,
            "queued extra turn for Alice should make her next turn immediate"
        );

        // If Alice's next turn is skipped, duration should extend to the following turn she takes.
        game.turn_store.extra_turns.clear();
        game.turn.active_player = bob;
        game.turn.turn_number = 30;
        game.turn_store.skip_next_turn = HashSet::from([alice]);
        let expires_with_skip =
            GrantPlayTaggedEffect::until_your_next_turn("it", PlayerFilter::You)
                .expires_end_of_turn(&game, alice);
        assert_eq!(
            expires_with_skip, 34,
            "skipped next turn should defer expiration to Alice's subsequent turn"
        );
    }

    #[test]
    fn grant_play_tagged_any_color_permission_survives_refresh_until_turn_ends() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::from_raw(2), "Exiled Spell")
            .card_types(vec![crate::types::CardType::Instant])
            .build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("exiled spell"), &game);

        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let mut dm = SelectFirstDecisionMaker;
        let source = ObjectId::from_raw(101);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);

        let effect = GrantPlayTaggedEffect::new(
            "it",
            PlayerFilter::You,
            GrantPlayTaggedDuration::UntilEndOfTurn,
            false,
            true,
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert!(game.can_spend_mana_as_any_color(alice, Some(exiled_id)));

        game.update_cant_effects();
        assert!(
            game.can_spend_mana_as_any_color(alice, Some(exiled_id)),
            "temporary effect-sourced mana permission should survive tracker refreshes"
        );
    }

    #[test]
    fn grant_play_tagged_for_as_long_as_exiled_uses_open_ended_exile_permission() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::from_raw(3), "Exiled Spell")
            .card_types(vec![crate::types::CardType::Instant])
            .build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("exiled spell"), &game);

        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let mut dm = SelectFirstDecisionMaker;
        let source = ObjectId::from_raw(102);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);

        let effect = GrantPlayTaggedEffect::new(
            "it",
            PlayerFilter::You,
            GrantPlayTaggedDuration::ForAsLongAsExiled,
            true,
            true,
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                alice
            ),
            "tagged card should be playable from exile"
        );
        assert!(game.can_spend_mana_as_any_color(alice, Some(exiled_id)));

        game.turn.turn_number = game.turn.turn_number.saturating_add(20);
        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                alice
            ),
            "while-exiled grant should not expire at end of turn"
        );
        assert!(
            !game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Graveyard,
                alice
            ),
            "grant remains tied to the exile zone"
        );

        let graveyard_id = game
            .move_object(
                exiled_id,
                Zone::Graveyard,
                crate::events::cause::EventCause::effect(),
            )
            .expect("test card should move to graveyard");
        assert!(
            !game.can_spend_mana_as_any_color(alice, Some(graveyard_id)),
            "any-mana permission for spells cast this way should not apply after the card leaves exile"
        );
        let reexiled_id = game
            .move_object(
                graveyard_id,
                Zone::Exile,
                crate::events::cause::EventCause::effect(),
            )
            .expect("test card should return to exile as a new object");
        assert!(
            !game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                reexiled_id,
                Zone::Exile,
                alice,
            ),
            "a later exile must not reactivate an old for-as-long-as-exiled permission"
        );
    }

    #[test]
    fn tagged_owner_permission_is_correlated_per_card_and_keeps_linked_constraints() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);

        let bob_card = CardBuilder::new(CardId::from_raw(30), "Bob's Exiled Spell")
            .card_types(vec![crate::types::CardType::Instant])
            .build();
        let cara_card = CardBuilder::new(CardId::from_raw(31), "Cara's Exiled Spell")
            .card_types(vec![crate::types::CardType::Instant])
            .build();
        let bob_exiled = game.create_object_from_card(&bob_card, bob, Zone::Exile);
        let cara_exiled = game.create_object_from_card(&cara_card, cara, Zone::Exile);
        let snapshots = vec![
            ObjectSnapshot::from_object(game.object(bob_exiled).expect("Bob's exiled card"), &game),
            ObjectSnapshot::from_object(
                game.object(cara_exiled).expect("Cara's exiled card"),
                &game,
            ),
        ];

        let tag = TagKey::from("each_player_exiled");
        let mut tags = std::collections::HashMap::new();
        tags.insert(tag.clone(), snapshots);
        let source = ObjectId::from_raw(103);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);
        let tax = crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(1)]);
        GrantPlayTaggedEffect::new(
            tag.clone(),
            PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(tag)),
            GrantPlayTaggedDuration::ForAsLongAsExiled,
            true,
            false,
        )
        .with_spell_cost_increase(tax.clone())
        .with_lands_enter_tapped(true)
        .execute(&mut game, &mut ctx)
        .expect("correlated play permissions should resolve");

        let can_play = |card, player| {
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                card,
                Zone::Exile,
                player,
            )
        };
        assert!(can_play(bob_exiled, bob));
        assert!(can_play(cara_exiled, cara));
        assert!(!can_play(bob_exiled, cara));
        assert!(!can_play(cara_exiled, bob));

        for (card, owner) in [(bob_exiled, bob), (cara_exiled, cara)] {
            let constraints = game
                .effect_store
                .grant_registry
                .play_from_constraints_for_card(&game, card, Zone::Exile, owner, source);
            assert_eq!(constraints.spell_cost_increase, Some(tax.clone()));
            assert!(constraints.lands_enter_tapped);
            assert!(
                game.effect_store
                    .grant_registry
                    .land_play_from_permissions_enters_tapped(&game, card, Zone::Exile, owner,)
            );
        }
    }

    #[test]
    fn grant_play_tagged_expires_on_sources_next_exile_event() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::from_raw(20), "Permission Source").build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let other_source_card = CardBuilder::new(CardId::from_raw(21), "Other Source").build();
        let other_source_id =
            game.create_object_from_card(&other_source_card, alice, Zone::Battlefield);

        let card = CardBuilder::new(CardId::from_raw(22), "Initially Exiled Card").build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        game.add_exiled_with_source_link(source_id, exiled_id);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("exiled card"), &game);

        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm).with_tagged_objects(tags);
        GrantPlayTaggedEffect::new(
            "it",
            PlayerFilter::You,
            GrantPlayTaggedDuration::UntilSourceExilesAnother,
            true,
            false,
        )
        .execute(&mut game, &mut ctx)
        .expect("grant should resolve");

        let can_play = |game: &GameState| {
            game.effect_store.grant_registry.card_can_play_from_zone(
                game,
                exiled_id,
                Zone::Exile,
                alice,
            )
        };
        assert!(can_play(&game));

        // Re-recording the same link is not a new exile event.
        game.add_exiled_with_source_link(source_id, exiled_id);
        assert!(can_play(&game));

        let unrelated = CardBuilder::new(CardId::from_raw(23), "Unrelated Exiled Card").build();
        let unrelated_id = game.create_object_from_card(&unrelated, alice, Zone::Exile);
        game.add_exiled_with_source_link(other_source_id, unrelated_id);
        assert!(
            can_play(&game),
            "another source must not end the permission"
        );

        let next = CardBuilder::new(CardId::from_raw(24), "Next Exiled Card").build();
        let next_id = game.create_object_from_card(&next, alice, Zone::Exile);
        game.add_exiled_with_source_link(source_id, next_id);
        assert!(
            !can_play(&game),
            "the same source's next successful exile must end the permission"
        );
    }

    #[test]
    fn gwen_stacy_grant_play_permission_ends_when_you_lose_control_of_source() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_card = CardBuilder::new(CardId::from_raw(10), "Gwen Stacy source").build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

        let card = CardBuilder::new(CardId::from_raw(11), "Exiled Card").build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("exiled card"), &game);

        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm).with_tagged_objects(tags);

        let effect = GrantPlayTaggedEffect::new(
            "it",
            PlayerFilter::You,
            GrantPlayTaggedDuration::ForAsLongAsYouControlSource,
            true,
            false,
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                alice
            ),
            "Gwen Stacy permission should apply while you control the source"
        );

        game.set_current_controller(source_id, bob);
        assert!(
            !game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                alice
            ),
            "Gwen Stacy permission should end once you lose control of the source"
        );
    }

    #[test]
    fn gwen_stacy_grant_play_permission_survives_turn_changes_while_controlled() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::from_raw(12), "Gwen Stacy source").build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

        let card = CardBuilder::new(CardId::from_raw(13), "Exiled Card").build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("exiled card"), &game);

        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm).with_tagged_objects(tags);

        let effect = GrantPlayTaggedEffect::new(
            "it",
            PlayerFilter::You,
            GrantPlayTaggedDuration::ForAsLongAsYouControlSource,
            true,
            false,
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        game.turn.turn_number = game.turn.turn_number.saturating_add(5);
        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                alice
            ),
            "Gwen Stacy permission should not expire by turn count while source stays controlled"
        );
    }
}
