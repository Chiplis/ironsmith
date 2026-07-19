#[cfg(target_arch = "wasm32")]
fn loaded_deck_sample_index(len: usize) -> usize {
    debug_assert!(len > 0);
    ((js_sys::Math::random() * len as f64).floor() as usize).min(len - 1)
}

#[cfg(not(target_arch = "wasm32"))]
fn loaded_deck_sample_index(_len: usize) -> usize {
    0
}

impl WasmGame {
    pub(super) fn is_cancelable(&self) -> bool {
        if let Some(replay) = self.pending_replay_action.as_ref() {
            return self.is_replay_chain_cancelable(replay);
        }

        if self.pending_action_checkpoint.is_none()
            && self
                .pending_decision
                .as_ref()
                .is_some_and(|ctx| !matches!(ctx, DecisionContext::Priority(_)))
        {
            return false;
        }

        if let Some(checkpoint) = self.pending_action_checkpoint.as_ref() {
            return !self.has_irreversible_mana_undo_lock()
                && !self.has_irreversible_library_change_since(checkpoint)
                && !self.has_irreversible_random_change_since(checkpoint);
        }

        let Some(epoch) = self.priority_epoch_checkpoint.as_ref() else {
            return false;
        };

        self.priority_epoch_has_undoable_action
            && !self.has_irreversible_mana_undo_lock()
            && !self.has_land_play_since(epoch)
            && !self.has_irreversible_library_change_since(epoch)
            && !self.has_irreversible_random_change_since(epoch)
    }

    fn response_starts_cancelable_action_chain(response: &PriorityResponse) -> bool {
        match response {
            PriorityResponse::PriorityAction(action) => {
                Self::priority_action_starts_cancelable_action_chain(action)
            }
            _ => false,
        }
    }

    fn priority_action_starts_cancelable_action_chain(action: &LegalAction) -> bool {
        !matches!(
            action,
            LegalAction::PassPriority
                | LegalAction::PlayLand { .. }
                | LegalAction::KeepOpeningHand
                | LegalAction::TakeMulligan
                | LegalAction::ContinuePregame
                | LegalAction::BeginGame
                | LegalAction::UsePregameAction { .. }
        )
    }

    fn replay_answers_start_cancelable_action_chain(
        root: &ReplayRoot,
        nested_answers: &[ReplayDecisionAnswer],
    ) -> bool {
        match root {
            ReplayRoot::Response(response) => {
                Self::response_starts_cancelable_action_chain(response)
            }
            ReplayRoot::Advance => matches!(
                nested_answers.first(),
                Some(ReplayDecisionAnswer::Priority(action))
                    if Self::priority_action_starts_cancelable_action_chain(action)
            ),
            ReplayRoot::AddCardToZone { .. } => false,
        }
    }

    fn is_replay_chain_cancelable(&self, replay: &PendingReplayAction) -> bool {
        let ReplayRoot::Response(response) = &replay.root else {
            return false;
        };

        if matches!(
            response,
            PriorityResponse::PriorityAction(LegalAction::PassPriority)
        ) {
            return false;
        }

        if replay.nested_answers.iter().any(|answer| {
            matches!(
                answer,
                ReplayDecisionAnswer::Priority(LegalAction::PassPriority)
            )
        }) {
            return false;
        }

        if self.has_irreversible_mana_undo_lock() {
            return false;
        }

        if self.has_land_play_since(&replay.checkpoint) {
            return false;
        }

        if self.pending_decision.is_none()
            && self.replay_chain_has_irreversible_mana_activation(replay)
        {
            return false;
        }

        !self.has_irreversible_library_change_since(&replay.checkpoint)
            && !self.has_irreversible_random_change_since(&replay.checkpoint)
    }

    fn priority_action_chain_still_pending(&self) -> bool {
        self.priority_state.pending_cast.is_some()
            || self.priority_state.pending_activation.is_some()
            || self.priority_state.pending_mana_ability.is_some()
            || self.priority_state.pending_method_selection.is_some()
            || self.priority_state.pending_continuation.is_some()
    }

    fn select_objects_uses_live_priority_response(&self) -> bool {
        self.priority_state
            .pending_activation
            .as_ref()
            .is_some_and(|pending| {
                matches!(
                    pending.stage,
                    ActivationStage::ChoosingSacrifice | ActivationStage::ChoosingCardCost
                )
            })
            || self
                .priority_state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| {
                    matches!(
                        pending.stage,
                        CastStage::ChoosingSacrifice | CastStage::ChoosingCardCost
                    )
                })
    }

    pub(super) fn decision_requires_root_reexecution(&self, ctx: &DecisionContext) -> bool {
        match ctx {
            // Number/target prompts only have a direct priority response while a
            // cast or activation is actively staged. Resolution-time prompts are
            // captured by replay and must rebuild their original execution path.
            DecisionContext::Number(_) | DecisionContext::Targets(_) => {
                !self.decision_has_direct_priority_response(ctx)
            }
            DecisionContext::SelectOptions(options) => {
                !self.select_options_uses_live_priority_response(options)
                    && replay_decision_requires_root_reexecution(ctx)
            }
            _ => {
                replay_decision_requires_root_reexecution(ctx)
                    || matches!(ctx, DecisionContext::SelectObjects(_))
                        && !self.select_objects_uses_live_priority_response()
            }
        }
    }

    fn select_options_uses_live_priority_response(
        &self,
        _ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> bool {
        self.game.effect_store.pending_replacement_choice.is_some()
            || self.priority_state.pending_method_selection.is_some()
            || self.priority_state.pending_mana_ability.is_some()
            || self
                .priority_state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| {
                    matches!(
                        pending.stage,
                        CastStage::ChoosingOptionalCosts
                            | CastStage::ChoosingAssistPlayer
                            | CastStage::ActivatingAssistManaAbilities
                            | CastStage::ChoosingAssistContribution
                            | CastStage::ChoosingNextCost
                            | CastStage::PayingAssistMana
                            | CastStage::PayingMana
                    )
                })
            || self
                .priority_state
                .pending_activation
                .as_ref()
                .is_some_and(|pending| {
                    matches!(
                        pending.stage,
                        ActivationStage::ChoosingAlternativeCost
                            | ActivationStage::ChoosingNextCost
                            | ActivationStage::PayingMana
                    )
                })
    }

    pub(super) fn decision_uses_live_priority_response(&self, ctx: &DecisionContext) -> bool {
        if self.priority_state.pending_continuation.is_some() {
            return true;
        }

        match ctx {
            DecisionContext::Priority(_)
            | DecisionContext::Modes(_)
            | DecisionContext::HybridChoice(_) => true,
            DecisionContext::Number(_) | DecisionContext::Targets(_) => {
                self.decision_has_direct_priority_response(ctx)
            }
            DecisionContext::SelectOptions(ctx) => {
                self.select_options_uses_live_priority_response(ctx)
            }
            DecisionContext::SelectObjects(_) => self.select_objects_uses_live_priority_response(),
            _ => false,
        }
    }

    fn decision_has_direct_priority_response(&self, ctx: &DecisionContext) -> bool {
        match ctx {
            DecisionContext::Number(_) | DecisionContext::Targets(_) => {
                self.priority_state.pending_cast.is_some()
                    || self.priority_state.pending_activation.is_some()
            }
            DecisionContext::Priority(_)
            | DecisionContext::Modes(_)
            | DecisionContext::HybridChoice(_) => true,
            _ => false,
        }
    }

    fn has_irreversible_mana_undo_lock(&self) -> bool {
        if self.priority_epoch_undo_locked_by_mana {
            return true;
        }

        // In-flight locks should not hide Undo while the user is still resolving
        // a prompt in the current action chain. The lock is latched at epoch level
        // when that chain commits.
        if self.pending_decision.is_some() {
            return false;
        }

        self.priority_state
            .pending_cast
            .as_ref()
            .is_some_and(|pending| pending.undo_locked_by_mana)
            || self
                .priority_state
                .pending_activation
                .as_ref()
                .is_some_and(|pending| pending.undo_locked_by_mana)
            || self
                .priority_state
                .pending_mana_ability
                .as_ref()
                .is_some_and(|pending| pending.undo_locked_by_mana)
    }

    pub(super) fn visible_undo_land_stable_id(&self, cancelable: bool) -> Option<u64> {
        if !cancelable {
            return None;
        }

        let Some(DecisionContext::Priority(ctx)) = self.pending_decision.as_ref() else {
            return None;
        };
        if ctx.player != self.perspective {
            return None;
        }

        self.priority_epoch_undo_land_stable_id
    }

    fn replay_root_has_irreversible_mana_activation(game: &GameState, root: &ReplayRoot) -> bool {
        if let ReplayRoot::Response(PriorityResponse::PriorityAction(action)) = root {
            return Self::legal_action_has_irreversible_mana_ability(game, action);
        }
        false
    }

    fn replay_root_starts_undoable_action(root: &ReplayRoot) -> bool {
        match root {
            ReplayRoot::Response(PriorityResponse::PriorityAction(LegalAction::PassPriority)) => {
                false
            }
            ReplayRoot::Response(_) => true,
            ReplayRoot::Advance | ReplayRoot::AddCardToZone { .. } => false,
        }
    }

    fn replay_root_is_mana_activation(root: &ReplayRoot) -> bool {
        matches!(
            root,
            ReplayRoot::Response(PriorityResponse::PriorityAction(
                LegalAction::ActivateManaAbility { .. }
            ))
        )
    }

    fn replay_root_land_mana_source_stable_id(game: &GameState, root: &ReplayRoot) -> Option<u64> {
        let ReplayRoot::Response(PriorityResponse::PriorityAction(
            LegalAction::ActivateManaAbility { source, .. },
        )) = root
        else {
            return None;
        };

        let object = game.object(*source)?;
        object
            .has_card_type(CardType::Land)
            .then_some(object.stable_id.0.0)
    }

    fn stack_grew_since(&self, checkpoint: &ReplayCheckpoint) -> bool {
        self.game.stack.len() > checkpoint.game.stack.len()
    }

    fn replay_root_mana_activation_added_to_stack(
        &self,
        checkpoint: &ReplayCheckpoint,
        root: &ReplayRoot,
    ) -> bool {
        Self::replay_root_is_mana_activation(root) && self.stack_grew_since(checkpoint)
    }

    fn committed_undo_land_stable_id(
        &self,
        checkpoint: &ReplayCheckpoint,
        root: &ReplayRoot,
    ) -> Option<u64> {
        if Self::replay_root_has_irreversible_mana_activation(&checkpoint.game, root)
            || self.replay_root_mana_activation_added_to_stack(checkpoint, root)
        {
            return None;
        }

        Self::replay_root_land_mana_source_stable_id(&checkpoint.game, root)
    }

    fn replay_chain_has_irreversible_mana_activation(&self, replay: &PendingReplayAction) -> bool {
        if Self::replay_root_has_irreversible_mana_activation(&replay.checkpoint.game, &replay.root)
        {
            return true;
        }

        replay.nested_answers.iter().any(|answer| {
            if let ReplayDecisionAnswer::Priority(action) = answer {
                return Self::legal_action_has_irreversible_mana_ability(
                    &replay.checkpoint.game,
                    action,
                );
            }
            false
        })
    }

    fn has_land_play_since(&self, checkpoint: &ReplayCheckpoint) -> bool {
        for before_player in &checkpoint.game.players {
            let Some(after_player) = self.game.player(before_player.id) else {
                return true;
            };
            if after_player.lands_played_this_turn > before_player.lands_played_this_turn {
                return true;
            }
        }
        false
    }

    fn legal_action_has_irreversible_mana_ability(game: &GameState, action: &LegalAction) -> bool {
        let LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } = action
        else {
            return false;
        };

        !ironsmith::game_loop::mana_ability_is_undo_safe(game, *source, *ability_index)
    }

    /// Returns true when the current game diverged from `checkpoint` in a way
    /// that should not be silently rewound (hidden-information/library changes).
    ///
    /// Allowed library delta:
    /// - removing cards from library only when those cards are currently on stack
    /// - preserving relative order of remaining library cards
    ///
    /// Everything else is treated as irreversible for cancel purposes.
    fn has_irreversible_library_change_since(&self, checkpoint: &ReplayCheckpoint) -> bool {
        for before_player in &checkpoint.game.players {
            let Some(after_player) = self.game.player(before_player.id) else {
                return true;
            };

            let before_library = &before_player.library;
            let after_library = &after_player.library;

            if before_library == after_library {
                continue;
            }

            // Cards moving into library are not safely reversible (includes
            // "put into library" effects and many reorder/shuffle outcomes).
            if after_library.len() > before_library.len() {
                return true;
            }

            let after_set: HashSet<ObjectId> = after_library.iter().copied().collect();
            let removed: HashSet<ObjectId> = before_library
                .iter()
                .copied()
                .filter(|id| !after_set.contains(id))
                .collect();

            // Pure reorder/shuffle with no net removals.
            if removed.is_empty() {
                return true;
            }

            // Moving a card from library is only reversible when that card is
            // currently on stack.
            if removed
                .iter()
                .any(|id| !self.game.stack.iter().any(|entry| entry.object_id == *id))
            {
                return true;
            }

            let expected_after: Vec<ObjectId> = before_library
                .iter()
                .copied()
                .filter(|id| !removed.contains(id))
                .collect();

            if expected_after != *after_library {
                return true;
            }
        }

        false
    }

    fn has_irreversible_random_change_since(&self, checkpoint: &ReplayCheckpoint) -> bool {
        self.game.irreversible_random_count() != checkpoint.game.irreversible_random_count()
    }

    pub(super) fn semantic_score_for_name(&self, card_name: &str) -> Option<f32> {
        self.external_semantic_score_for_name(card_name)
            .or_else(|| CardRegistry::generated_parser_semantic_score(card_name))
    }

    fn is_known_card_name_query(&mut self, query: &str) -> bool {
        if query.trim().is_empty() {
            return false;
        }
        self.registry.ensure_cards_loaded([query]);
        self.registry.get(query).is_some()
    }

    fn autocomplete_name_corpus() -> &'static [(String, String)] {
        AUTOCOMPLETE_CARD_NAMES.get_or_init(|| {
            let mut names = CardRegistry::generated_parser_card_names();
            names.sort_unstable();
            names.dedup();
            names
                .into_iter()
                .map(|name| {
                    let lower = name.to_lowercase();
                    (name, lower)
                })
                .collect()
        })
    }

    fn has_demo_supported_cost_symbols(cost: &ironsmith::mana::ManaCost) -> bool {
        !cost.pips().iter().flatten().any(|symbol| {
            matches!(
                symbol,
                ManaSymbol::Colorless | ManaSymbol::Snow | ManaSymbol::Life(_) | ManaSymbol::X
            )
        })
    }

    fn is_strict_demo_spell_candidate(def: &CardDefinition) -> bool {
        if def.card.is_token || def.card.is_land() {
            return false;
        }
        // Keep startup-safe random decks to cards the current UI/decision loop
        // can consistently represent without hitting unsupported cast branches.
        if !def.alternative_casts.is_empty()
            || !def.optional_costs.is_empty()
            || def.additional_cost.has_non_mana_costs()
            || def.name().contains("//")
        {
            return false;
        }
        let Some(cost) = &def.card.mana_cost else {
            return false;
        };
        Self::has_demo_supported_cost_symbols(cost)
    }

    fn is_fallback_demo_spell_candidate(def: &CardDefinition) -> bool {
        if def.card.is_token || def.card.is_land() {
            return false;
        }
        match &def.card.mana_cost {
            Some(cost) => Self::has_demo_supported_cost_symbols(cost),
            None => true,
        }
    }

    fn build_random_demo_deck_names(
        &mut self,
        deck_size: usize,
        land_count: usize,
    ) -> Result<Vec<String>, String> {
        if deck_size == 0 || land_count >= deck_size {
            return Err(
                "invalid deck sizing (deck_size must be > 0 and land_count < deck_size)"
                    .to_string(),
            );
        }

        const DEMO_BASIC_LANDS: &[&str] = &["Plains", "Island", "Swamp", "Mountain", "Forest"];
        const DEMO_SPELL_POOL: &[&str] = &[
            "Lightning Bolt",
            "Counterspell",
            "Giant Growth",
            "Opt",
            "Divination",
            "Llanowar Elves",
            "Grizzly Bears",
            "Ornithopter",
            "Serra Angel",
            "Doom Blade",
            "Raise Dead",
            "Unsummon",
        ];

        let spells_needed = deck_size - land_count;
        let mut rng = rand_chacha::ChaCha12Rng::seed_from_u64(self.next_deck_seed());

        self.registry.ensure_cards_loaded(
            DEMO_BASIC_LANDS
                .iter()
                .copied()
                .chain(DEMO_SPELL_POOL.iter().copied()),
        );

        let mut strict_spell_pool: Vec<String> = Vec::new();
        let mut fallback_spell_pool: Vec<String> = Vec::new();
        let mut strict_seen: HashSet<String> = HashSet::new();
        let mut fallback_seen: HashSet<String> = HashSet::new();

        for candidate in DEMO_SPELL_POOL {
            if self.semantic_threshold > 0.0
                && let Some(score) = self.semantic_score_for_name(candidate)
                && score < self.semantic_threshold
            {
                continue;
            }
            let Some(def) = self.registry.get(candidate) else {
                continue;
            };
            let canonical = def.name().to_string();
            let key = canonical.to_lowercase();
            if Self::is_strict_demo_spell_candidate(def) {
                if strict_seen.insert(key) {
                    strict_spell_pool.push(canonical);
                }
            } else if Self::is_fallback_demo_spell_candidate(def) && fallback_seen.insert(key) {
                fallback_spell_pool.push(canonical);
            }
        }

        let mut spell_pool = if strict_spell_pool.is_empty() {
            fallback_spell_pool
        } else {
            strict_spell_pool
        };

        if spell_pool.is_empty() {
            return Err(
                "registry has no nonland cards eligible for random deck generation".to_string(),
            );
        }

        spell_pool.shuffle(&mut rng);

        let mut spells: Vec<String> = Vec::with_capacity(spells_needed);
        while spells.len() < spells_needed {
            spell_pool.shuffle(&mut rng);
            let before = spells.len();
            for card_name in &spell_pool {
                spells.push(card_name.clone());
                if self
                    .validate_normal_constructed_card_names(&spells, &[])
                    .is_err()
                {
                    spells.pop();
                }
                if spells.len() >= spells_needed {
                    break;
                }
            }
            if spells.len() == before {
                return Err(format!(
                    "eligible demo spell pool has insufficient copy-limit capacity for {spells_needed} cards"
                ));
            }
        }

        let mut symbol_counts: HashMap<ManaSymbol, u32> = HashMap::new();
        for card_name in &spells {
            if let Some(def) = self.registry.get(card_name)
                && let Some(cost) = &def.card.mana_cost
            {
                for pip in cost.pips() {
                    for symbol in pip {
                        match symbol {
                            ManaSymbol::White
                            | ManaSymbol::Blue
                            | ManaSymbol::Black
                            | ManaSymbol::Red
                            | ManaSymbol::Green => {
                                *symbol_counts.entry(*symbol).or_insert(0) += 1;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let mut deck = spells;
        let total_colored_symbols: u32 = symbol_counts.values().sum();
        let color_order = [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
        ];

        let mut assigned_lands = 0usize;
        for color in color_order {
            let count = symbol_counts.get(&color).copied().unwrap_or(0);
            if count == 0 || total_colored_symbols == 0 {
                continue;
            }
            let share = (count as f64 / total_colored_symbols as f64) * land_count as f64;
            let land_slots = share.round() as usize;
            let basic_name = Self::basic_land_name_for_symbol(color);
            if self.registry.get(basic_name).is_none() {
                continue;
            }
            for _ in 0..land_slots {
                deck.push(basic_name.to_string());
                assigned_lands += 1;
                if assigned_lands >= land_count {
                    break;
                }
            }
            if assigned_lands >= land_count {
                break;
            }
        }

        let fallback_color = symbol_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(color, _)| *color)
            .unwrap_or(ManaSymbol::Green);
        let mut fallback_land = Self::basic_land_name_for_symbol(fallback_color);
        if self.registry.get(fallback_land).is_none() {
            fallback_land = DEMO_BASIC_LANDS
                .into_iter()
                .find(|name| self.registry.get(name).is_some())
                .ok_or_else(|| "registry has no basic lands for demo manabase".to_string())?;
        }
        while assigned_lands < land_count {
            deck.push(fallback_land.to_string());
            assigned_lands += 1;
        }

        self.validate_normal_constructed_card_names(&deck, &[])?;
        deck.shuffle(&mut rng);
        Ok(deck)
    }

    fn next_deck_seed(&mut self) -> u64 {
        self.game.next_random_u64()
    }

    fn basic_land_name_for_symbol(symbol: ManaSymbol) -> &'static str {
        match symbol {
            ManaSymbol::White => "Plains",
            ManaSymbol::Blue => "Island",
            ManaSymbol::Black => "Swamp",
            ManaSymbol::Red => "Mountain",
            ManaSymbol::Green => "Forest",
            _ => "Forest",
        }
    }

    fn populate_player_library(
        &mut self,
        player_id: PlayerId,
        deck_names: &[String],
    ) -> Result<(), String> {
        self.registry
            .ensure_cards_loaded(deck_names.iter().map(|name| name.as_str()));

        for name in deck_names {
            let Some(definition) = self.find_card_definition(name).cloned() else {
                return Err(format!("unknown card name: {name}"));
            };
            self.game.create_object_from_catalog_definition(
                &definition,
                &self.registry,
                player_id,
                ironsmith::zone::Zone::Library,
            );
        }

        self.game.shuffle_player_library(player_id);
        Ok(())
    }

    fn find_card_definition(&self, query: &str) -> Option<&CardDefinition> {
        self.registry.get(query).or_else(|| {
            self.registry
                .all()
                .find(|def| def.name().eq_ignore_ascii_case(query))
        })
    }

    fn generated_parse_source_for_name(&self, query: &str) -> Option<(String, String)> {
        self.external_parse_source_for_name(query)
            .or_else(|| CardRegistry::generated_parser_card_parse_source(query))
    }

    fn extract_oracle_text_from_parse_block(block: &str) -> Option<String> {
        let oracle_lines: Vec<&str> = block
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("Mana cost:")
                    && !trimmed.starts_with("Type:")
                    && !trimmed.starts_with("Power/Toughness:")
                    && !trimmed.starts_with("Loyalty:")
                    && !trimmed.starts_with("Defense:")
            })
            .collect();
        let compiled_card_text = oracle_lines.join("\n").trim().to_string();
        if compiled_card_text.is_empty() {
            None
        } else {
            Some(compiled_card_text)
        }
    }

    fn compile_definition_from_parse_source(
        source_name: &str,
        parse_block: &str,
    ) -> Result<CardDefinition, String> {
        ironsmith_registry::compile_to_runtime_definition(
            source_name,
            parse_block.to_string(),
            false,
        )
        .map_err(|err| err.to_string())
    }

    fn card_lookup_error_for_query(query: &str, err: String) -> String {
        if err == "generated registry not available" {
            format!("unknown card name: {query}")
        } else {
            err
        }
    }

    fn compiled_ability_lines(definition: &CardDefinition) -> Vec<String> {
        definition
            .abilities
            .iter()
            .enumerate()
            .map(|(index, ability)| {
                let text = match &ability.kind {
                    ironsmith::ability::AbilityKind::Static(static_ability) => {
                        static_ability.display()
                    }
                    ironsmith::ability::AbilityKind::Triggered(triggered) => {
                        let trigger = triggered.trigger.display();
                        let effects = if triggered.effects.is_empty() {
                            String::new()
                        } else {
                            ironsmith::compiled_text::compile_effect_list(&triggered.effects)
                        };
                        if effects.trim().is_empty() {
                            trigger
                        } else {
                            format!("{trigger} -> {effects}")
                        }
                    }
                    ironsmith::ability::AbilityKind::Activated(activated) => {
                        let cost = activated.mana_cost.display();
                        let resolution = if let Some(mana) = &activated.mana_output {
                            if mana.is_empty() {
                                ironsmith::compiled_text::compile_effect_list(&activated.effects)
                            } else {
                                format!(
                                    "Add {}",
                                    ironsmith::mana::ManaCost::from_symbols(mana.clone())
                                        .to_oracle()
                                )
                            }
                        } else {
                            ironsmith::compiled_text::compile_effect_list(&activated.effects)
                        };

                        match (cost.trim().is_empty(), resolution.trim().is_empty()) {
                            (true, true) => "Activated ability".to_string(),
                            (false, true) => cost,
                            (true, false) => resolution,
                            (false, false) => format!("{cost} -> {resolution}"),
                        }
                    }
                };
                format!("Ability {}: {}", index + 1, text)
            })
            .collect()
    }

    fn custom_type_line(
        supertypes: &[String],
        card_types: &[String],
        subtypes: &[String],
    ) -> Option<String> {
        let left = supertypes
            .iter()
            .chain(card_types.iter())
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        let right = subtypes
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();

        if left.is_empty() && right.is_empty() {
            return None;
        }

        let mut line = left.join(" ");
        if !right.is_empty() {
            if !line.is_empty() {
                line.push_str(" — ");
            }
            line.push_str(&right.join(" "));
        }
        Some(line)
    }

    fn parse_custom_color_indicator(tokens: &[String]) -> Result<Option<ColorSet>, JsValue> {
        let mut colors = ColorSet::COLORLESS;
        for token in tokens {
            let normalized = token.trim().to_lowercase();
            if normalized.is_empty() || normalized == "c" || normalized == "colorless" {
                continue;
            }
            let Some(color) = Color::from_mana_code_or_name(&normalized) else {
                return Err(JsValue::from_str(&format!(
                    "unknown color indicator value: {}",
                    token.trim()
                )));
            };
            colors = colors.with(color);
        }

        if colors.is_empty() {
            Ok(None)
        } else {
            Ok(Some(colors))
        }
    }

    fn color_indicator_codes(colors: Option<ColorSet>) -> Vec<String> {
        let Some(colors) = colors else {
            return Vec::new();
        };

        Color::ALL
            .iter()
            .filter(|color| colors.contains(**color))
            .map(|color| match color {
                Color::White => "W",
                Color::Blue => "U",
                Color::Black => "B",
                Color::Red => "R",
                Color::Green => "G",
            })
            .map(str::to_string)
            .collect()
    }

    fn build_custom_face_parse_block(face: &CustomCardFaceInput) -> Result<String, JsValue> {
        let mut lines = Vec::new();

        if let Some(mana_cost) = face
            .mana_cost
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("Mana cost: {mana_cost}"));
        }

        let Some(type_line) =
            Self::custom_type_line(&face.supertypes, &face.card_types, &face.subtypes)
        else {
            return Err(JsValue::from_str("custom cards must include a type line"));
        };
        lines.push(format!("Type: {type_line}"));

        match (
            face.power
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty()),
            face.toughness
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty()),
        ) {
            (Some(power), Some(toughness)) => {
                lines.push(format!("Power/Toughness: {power}/{toughness}"));
            }
            (None, None) => {}
            _ => {
                return Err(JsValue::from_str(
                    "custom card power and toughness must both be provided",
                ));
            }
        }

        if let Some(loyalty) = face.loyalty {
            lines.push(format!("Loyalty: {loyalty}"));
        }
        if let Some(defense) = face.defense {
            lines.push(format!("Defense: {defense}"));
        }

        let oracle_text = face.oracle_text.trim();
        if !oracle_text.is_empty() {
            lines.push(oracle_text.to_string());
        }

        Ok(lines.join("\n"))
    }

    fn compile_custom_card_faces(
        &self,
        draft: &CustomCardInput,
    ) -> Result<Vec<CardDefinition>, JsValue> {
        let expected_faces = draft.layout.face_count();
        if draft.faces.len() != expected_faces {
            return Err(JsValue::from_str(&format!(
                "{} layout requires {} face(s)",
                match draft.layout {
                    CustomCardLayoutInput::Single => "single-face",
                    CustomCardLayoutInput::TransformLike => "double-faced",
                    CustomCardLayoutInput::Split => "split",
                },
                expected_faces
            )));
        }

        let mut definitions = Vec::with_capacity(expected_faces);
        for (index, face) in draft.faces.iter().enumerate() {
            let name = face.name.trim();
            if name.is_empty() {
                return Err(JsValue::from_str(&format!(
                    "face {} must include a name",
                    index + 1
                )));
            }

            let mut builder = ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name);
            if let Some(colors) = Self::parse_custom_color_indicator(&face.color_indicator)? {
                builder = builder.color_indicator(colors);
            }

            let parse_block = Self::build_custom_face_parse_block(face)?;
            let mut definition = ironsmith_registry::compile_builder_to_runtime_definition(
                builder,
                parse_block,
                false,
            )
            .map_err(|err| JsValue::from_str(&format!("face {} parse failed: {err}", index + 1)))?;
            definition.card.linked_face_layout = draft.layout.linked_face_layout();
            definitions.push(definition);
        }

        if definitions.len() == 2 {
            let first_id = definitions[0].card.id;
            let second_id = definitions[1].card.id;
            let first_name = definitions[0].card.name.clone();
            let second_name = definitions[1].card.name.clone();

            definitions[0].card.other_face = Some(second_id);
            definitions[0].card.other_face_name = Some(second_name.clone());
            definitions[1].card.other_face = Some(first_id);
            definitions[1].card.other_face_name = Some(first_name.clone());

            if draft.layout == CustomCardLayoutInput::Split && draft.has_fuse {
                definitions[0].has_fuse = true;
            }
        }

        Ok(definitions)
    }

    fn definition_to_custom_face_input(definition: &CardDefinition) -> CustomCardFaceInput {
        CustomCardFaceInput {
            name: definition.card.name.clone(),
            mana_cost: definition.card.mana_cost.as_ref().map(ManaCost::to_oracle),
            color_indicator: Self::color_indicator_codes(definition.card.color_indicator),
            supertypes: definition
                .card
                .supertypes
                .iter()
                .map(|value| format!("{value:?}"))
                .collect(),
            card_types: definition
                .card
                .card_types
                .iter()
                .map(|value| format!("{value:?}"))
                .collect(),
            subtypes: definition
                .card
                .subtypes
                .iter()
                .map(|value| format!("{value:?}"))
                .collect(),
            oracle_text: Self::definition_display_oracle_text(definition),
            power: definition
                .card
                .power_toughness
                .map(|value| value.power.to_string()),
            toughness: definition
                .card
                .power_toughness
                .map(|value| value.toughness.to_string()),
            loyalty: definition.card.loyalty,
            defense: definition.card.defense,
        }
    }

    fn definition_display_oracle_text(definition: &CardDefinition) -> String {
        ironsmith::compiled_text::compiled_text_lines(definition).join("\n")
    }

    fn definition_type_line(definition: &CardDefinition) -> String {
        let left = definition
            .card
            .supertypes
            .iter()
            .map(|value| format!("{value:?}"))
            .chain(
                definition
                    .card
                    .card_types
                    .iter()
                    .map(|value| format!("{value:?}")),
            )
            .collect::<Vec<_>>();
        let right = definition
            .card
            .subtypes
            .iter()
            .map(|value| format!("{value:?}"))
            .collect::<Vec<_>>();

        let mut line = left.join(" ");
        if !right.is_empty() {
            if !line.is_empty() {
                line.push_str(" — ");
            }
            line.push_str(&right.join(" "));
        }
        line
    }

    fn definition_to_custom_preview_face(
        definition: &CardDefinition,
        source_oracle_text: Option<&str>,
    ) -> CustomCardPreviewFace {
        CustomCardPreviewFace {
            name: definition.card.name.clone(),
            mana_cost: definition.card.mana_cost.as_ref().map(ManaCost::to_oracle),
            color_indicator: Self::color_indicator_codes(definition.card.color_indicator),
            type_line: Self::definition_type_line(definition),
            oracle_text: source_oracle_text
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| Self::definition_display_oracle_text(definition)),
            power: definition
                .card
                .power_toughness
                .map(|value| value.power.to_string()),
            toughness: definition
                .card
                .power_toughness
                .map(|value| value.toughness.to_string()),
            loyalty: definition.card.loyalty,
            defense: definition.card.defense,
            compiled_text: ironsmith::compiled_text::compiled_text_lines(definition),
            compiled_abilities: Self::compiled_ability_lines(definition),
            raw_compilation: format!("{:#?}", definition),
        }
    }

    pub(super) fn build_custom_card_preview(
        &self,
        draft: &CustomCardInput,
    ) -> Result<CustomCardPreviewResult, JsValue> {
        let definitions = self.compile_custom_card_faces(draft)?;
        Ok(CustomCardPreviewResult {
            layout: draft.layout,
            has_fuse: draft.layout == CustomCardLayoutInput::Split && draft.has_fuse,
            faces: definitions
                .iter()
                .enumerate()
                .map(|(index, definition)| {
                    Self::definition_to_custom_preview_face(
                        definition,
                        draft.faces.get(index).map(|face| face.oracle_text.as_str()),
                    )
                })
                .collect(),
            can_create: true,
        })
    }

    fn build_loaded_deck_seed(
        &mut self,
        player_index: u8,
    ) -> Result<CustomCardSeedResult, JsValue> {
        let Some(deck) = self.loaded_decks.get(player_index as usize) else {
            return Err(JsValue::from_str("no loaded deck found for that player"));
        };
        if deck.is_empty() {
            return Err(JsValue::from_str("loaded deck is empty"));
        }

        let eligible = deck
            .iter()
            .filter_map(|name| self.find_card_definition(name).cloned())
            .filter(|definition| !definition.card.is_land())
            .collect::<Vec<_>>();
        if eligible.is_empty() {
            return Err(JsValue::from_str(
                "loaded deck has no nonland cards available for sampling",
            ));
        }

        let sample_index = loaded_deck_sample_index(eligible.len());
        let definition = eligible[sample_index].clone();
        let layout = match definition.card.linked_face_layout {
            ironsmith::card::LinkedFaceLayout::Split => CustomCardLayoutInput::Split,
            ironsmith::card::LinkedFaceLayout::TransformLike => {
                CustomCardLayoutInput::TransformLike
            }
            ironsmith::card::LinkedFaceLayout::None => CustomCardLayoutInput::Single,
        };

        let mut faces = vec![Self::definition_to_custom_face_input(&definition)];
        if layout.face_count() == 2 {
            let Some(other_face) = self
                .registry
                .linked_face_definition_by_name_or_id(
                    definition.card.other_face_name.as_deref(),
                    definition.card.other_face,
                )
                .cloned()
            else {
                return Err(JsValue::from_str(
                    "sampled card references an unsupported linked face",
                ));
            };
            faces.push(Self::definition_to_custom_face_input(&other_face));
        }

        Ok(CustomCardSeedResult {
            layout,
            has_fuse: layout == CustomCardLayoutInput::Split && definition.has_fuse,
            faces,
        })
    }

    fn add_definition_to_zone_with_triggers(
        &mut self,
        definition: &CardDefinition,
        player_id: PlayerId,
        zone: Zone,
    ) -> Result<ObjectId, JsValue> {
        self.game
            .register_linked_face_family_from_catalog(definition, &self.registry);
        // Create in Command zone first, then move to target zone so that
        // zone-change triggers (ETB, etc.) fire naturally.
        let temp_id = self.game.create_object_from_definition(
            definition,
            player_id,
            ironsmith::zone::Zone::Command,
        );
        let object_id = if zone == ironsmith::zone::Zone::Battlefield {
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            let Some(result) = self.game.move_object_with_etb_processing_with_dm(
                temp_id,
                ironsmith::zone::Zone::Battlefield,
                &mut dm,
            ) else {
                self.game.remove_object(temp_id);
                return Err(JsValue::from_str(
                    "battlefield entry was prevented by replacement effect",
                ));
            };

            let entered_id = result.new_id;
            let entered_tapped = result.enters_tapped;
            let entered_battlefield = self
                .game
                .object(entered_id)
                .is_some_and(|obj| obj.zone == ironsmith::zone::Zone::Battlefield);
            if entered_battlefield {
                let etb_event_provenance = self
                    .game
                    .provenance_graph_mut()
                    .alloc_root_event(ironsmith::events::EventKind::EnterBattlefield);
                let event = if entered_tapped {
                    ironsmith::triggers::TriggerEvent::new_with_provenance(
                        ironsmith::events::EnterBattlefieldEvent::tapped(
                            entered_id,
                            ironsmith::zone::Zone::Command,
                        ),
                        etb_event_provenance,
                    )
                } else {
                    ironsmith::triggers::TriggerEvent::new_with_provenance(
                        ironsmith::events::EnterBattlefieldEvent::new(
                            entered_id,
                            ironsmith::zone::Zone::Command,
                        ),
                        etb_event_provenance,
                    )
                };
                self.game.queue_trigger_event(etb_event_provenance, event);

                ironsmith::game_loop::drain_pending_trigger_events(
                    &mut self.game,
                    &mut self.trigger_queue,
                );

                ironsmith::game_loop::handle_saga_enters_battlefield(
                    &mut self.game,
                    entered_id,
                    &mut self.trigger_queue,
                    &mut dm,
                );
            }

            entered_id
        } else {
            self.game
                .move_object_by_effect(temp_id, zone)
                .unwrap_or(temp_id)
        };
        ironsmith::game_loop::drain_pending_trigger_events(&mut self.game, &mut self.trigger_queue);
        self.recompute_ui_decision()?;
        Ok(object_id)
    }

    pub(super) fn build_card_load_diagnostics(
        &mut self,
        card_name: &str,
        explicit_error: Option<&str>,
    ) -> CardLoadDiagnostics {
        let query = card_name.trim();
        let parse_source = if query.is_empty() {
            None
        } else {
            self.generated_parse_source_for_name(query)
        };
        let source_compile_result = parse_source.as_ref().map(|(source_name, parse_block)| {
            Self::compile_definition_from_parse_source(source_name, parse_block)
        });

        if !query.is_empty() {
            self.registry.ensure_cards_loaded([query]);
        }
        let registry_definition = self.find_card_definition(query).cloned();
        let compiled_definition = registry_definition.or_else(|| {
            source_compile_result
                .as_ref()
                .and_then(|result| result.as_ref().ok().cloned())
        });
        let canonical_name = compiled_definition
            .as_ref()
            .map(|definition| definition.name().to_string())
            .or_else(|| {
                parse_source
                    .as_ref()
                    .map(|(source_name, _)| source_name.clone())
            });
        let oracle_text = parse_source
            .as_ref()
            .and_then(|(_, parse_block)| Self::extract_oracle_text_from_parse_block(parse_block))
            .or_else(|| {
                compiled_definition
                    .as_ref()
                    .map(Self::definition_display_oracle_text)
                    .filter(|text| !text.trim().is_empty())
            });
        let compiled_text = compiled_definition
            .as_ref()
            .map(ironsmith::compiled_text::compiled_text_lines)
            .unwrap_or_default();
        let compiled_abilities = compiled_definition
            .as_ref()
            .map(Self::compiled_ability_lines)
            .unwrap_or_default();
        let semantic_score = canonical_name
            .as_deref()
            .and_then(|name| self.semantic_score_for_name(name))
            .or_else(|| self.semantic_score_for_name(query));
        let threshold_percent =
            (self.semantic_threshold > 0.0).then_some(self.semantic_threshold * 100.0);
        let parse_error = if query.is_empty() {
            Some("card name cannot be empty".to_string())
        } else if let Some(result) = source_compile_result.as_ref() {
            result
                .clone()
                .err()
                .or_else(|| self.external_compile_error_for_name(query))
                .or_else(|| CardRegistry::try_compile_card(query).err())
        } else {
            self.external_compile_error_for_name(query)
                .or_else(|| CardRegistry::try_compile_card(query).err())
        };
        let error = explicit_error
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| parse_error.clone());

        CardLoadDiagnostics {
            query: query.to_string(),
            canonical_name,
            error,
            parse_error,
            oracle_text,
            compiled_text,
            compiled_abilities,
            semantic_score,
            threshold_percent,
        }
    }

    pub(super) fn validate_match_setup_input(
        &mut self,
        config: &MatchSetupInput,
    ) -> Result<MatchValidationResult, JsValue> {
        let player_count = config.player_names.len();
        if player_count == 0 {
            return Err(JsValue::from_str("player_names cannot be empty"));
        }
        config
            .validate_multiplayer_profile()
            .map_err(|error| JsValue::from_str(&error))?;

        let decks = config.decks.as_ref();
        let sideboards = config.sideboards.as_ref();
        let commanders = config.commanders.as_ref();
        let hidden_manifests = config.hidden_deck_manifests.as_deref().unwrap_or(&[]);
        let hidden_manifest_for_player = |player_index: usize| {
            hidden_manifests
                .iter()
                .find(|manifest| usize::from(manifest.owner) == player_index)
        };

        if let Some(decks) = decks
            && decks.len() != player_count
        {
            return Err(JsValue::from_str(
                "deck count must match number of players in game",
            ));
        }
        if let Some(commanders) = commanders
            && commanders.len() != player_count
        {
            return Err(JsValue::from_str(
                "commander count must match number of players in game",
            ));
        }
        if let Some(sideboards) = sideboards
            && sideboards.len() != player_count
        {
            return Err(JsValue::from_str(
                "sideboard count must match number of players in game",
            ));
        }

        if config.format.uses_commander_setup() {
            let Some(decks) = decks else {
                return Err(JsValue::from_str(
                    "commander-variant matches require explicit decklists",
                ));
            };
            let Some(commanders) = commanders else {
                return Err(JsValue::from_str(
                    "commander-variant matches require commander lists",
                ));
            };

            for (player_index, (deck, commander_list)) in
                decks.iter().zip(commanders.iter()).enumerate()
            {
                if config.format == MatchFormatInput::CommanderDraft {
                    if !(commander_list.len() == 1 || commander_list.len() == 2) {
                        return Err(JsValue::from_str(
                            "Commander Draft requires exactly 1 or 2 commanders per player",
                        ));
                    }
                    if hidden_manifest_for_player(player_index).is_some() {
                        return Err(JsValue::from_str(
                            "Commander Draft setup requires explicit completed card pools",
                        ));
                    }
                    if deck.len() + commander_list.len() < 60 {
                        return Err(JsValue::from_str(
                            "Commander Draft decks must contain at least 60 cards including commanders",
                        ));
                    }
                    continue;
                }
                let (expected_deck_size, expected_commander_count) = match config.format {
                    MatchFormatInput::Commander | MatchFormatInput::ArchenemyCommander => {
                        if !(commander_list.len() == 1 || commander_list.len() == 2) {
                            return Err(JsValue::from_str(
                                "commander matches require exactly 1 or 2 commanders per player",
                            ));
                        }
                        (if commander_list.len() == 2 { 98 } else { 99 }, None)
                    }
                    MatchFormatInput::Brawl => {
                        if commander_list.len() != 1 {
                            return Err(JsValue::from_str(
                                "Brawl matches require exactly one commander per player",
                            ));
                        }
                        (59, Some(1))
                    }
                    MatchFormatInput::Normal
                    | MatchFormatInput::FreeForAll
                    | MatchFormatInput::GrandMelee
                    | MatchFormatInput::TeamVsTeam
                    | MatchFormatInput::Emperor
                    | MatchFormatInput::TwoHeadedGiant
                    | MatchFormatInput::AlternatingTeams
                    | MatchFormatInput::Ante
                    | MatchFormatInput::Planechase
                    | MatchFormatInput::Vanguard
                    | MatchFormatInput::Archenemy
                    | MatchFormatInput::SupervillainRumble
                    | MatchFormatInput::ConspiracyDraft
                    | MatchFormatInput::CommanderDraft => unreachable!(),
                };
                if let Some(manifest) = hidden_manifest_for_player(player_index)
                    && deck.is_empty()
                {
                    if manifest.deck_count != expected_deck_size {
                        return Err(JsValue::from_str(&format!(
                            "commander-variant committed main decks must contain {expected_deck_size} cards"
                        )));
                    }
                    if expected_commander_count
                        .is_some_and(|expected| manifest.commander_count != expected)
                    {
                        let expected_commander_count = expected_commander_count.unwrap_or_default();
                        return Err(JsValue::from_str(&format!(
                            "commander-variant committed setup must contain {expected_commander_count} commander(s)"
                        )));
                    }
                    continue;
                }
                if deck.len() != expected_deck_size {
                    return Err(JsValue::from_str(&format!(
                        "commander-variant main decks must contain {expected_deck_size} cards"
                    )));
                }
            }
        }

        let mut cache = HashMap::<String, Option<String>>::new();
        let mut issues = Vec::new();

        if let Some(decks) = decks {
            for (player_index, deck) in decks.iter().enumerate() {
                if deck.is_empty() && hidden_manifest_for_player(player_index).is_some() {
                    continue;
                }
                self.collect_match_validation_issues(
                    player_index,
                    &config.player_names[player_index],
                    "deck",
                    deck,
                    &mut cache,
                    &mut issues,
                );
            }
        }

        if let Some(commanders) = commanders {
            for (player_index, commander_list) in commanders.iter().enumerate() {
                self.collect_match_validation_issues(
                    player_index,
                    &config.player_names[player_index],
                    "commander",
                    commander_list,
                    &mut cache,
                    &mut issues,
                );
            }
        }

        if let Some(sideboards) = sideboards {
            for (player_index, sideboard) in sideboards.iter().enumerate() {
                self.collect_match_validation_issues(
                    player_index,
                    &config.player_names[player_index],
                    "sideboard",
                    sideboard,
                    &mut cache,
                    &mut issues,
                );
            }
        }
        if let Some(setup) = config.commander_draft.as_ref() {
            for (player_index, pool) in setup.card_pools.iter().enumerate() {
                self.collect_match_validation_issues(
                    player_index.min(config.player_names.len().saturating_sub(1)),
                    config
                        .player_names
                        .get(player_index)
                        .or_else(|| config.player_names.first())
                        .map(String::as_str)
                        .unwrap_or("Player"),
                    "Commander Draft pool",
                    pool,
                    &mut cache,
                    &mut issues,
                );
            }
        }

        if let Some(planar_decks) = config.planar_decks.as_ref() {
            for (deck_index, planar_deck) in planar_decks.iter().enumerate() {
                self.collect_match_validation_issues(
                    deck_index.min(config.player_names.len().saturating_sub(1)),
                    config
                        .player_names
                        .get(deck_index)
                        .or_else(|| config.player_names.first())
                        .map(String::as_str)
                        .unwrap_or("Player"),
                    "planar deck",
                    &planar_deck
                        .iter()
                        .map(|card| card.name.clone())
                        .collect::<Vec<_>>(),
                    &mut cache,
                    &mut issues,
                );
            }
        }

        if let Some(vanguards) = config.vanguards.as_ref() {
            for (player_index, card) in vanguards.iter().enumerate() {
                self.collect_match_validation_issues(
                    player_index.min(config.player_names.len().saturating_sub(1)),
                    config
                        .player_names
                        .get(player_index)
                        .or_else(|| config.player_names.first())
                        .map(String::as_str)
                        .unwrap_or("Player"),
                    "vanguard",
                    std::slice::from_ref(&card.name),
                    &mut cache,
                    &mut issues,
                );
            }
        }
        if let Some(scheme_decks) = config.scheme_decks.as_ref() {
            for (player_index, deck) in scheme_decks.iter().enumerate() {
                self.collect_match_validation_issues(
                    player_index.min(config.player_names.len().saturating_sub(1)),
                    config
                        .player_names
                        .get(player_index)
                        .or_else(|| config.player_names.first())
                        .map(String::as_str)
                        .unwrap_or("Player"),
                    "scheme deck",
                    deck,
                    &mut cache,
                    &mut issues,
                );
            }
        }

        if issues.is_empty() {
            match config.format {
                MatchFormatInput::Commander | MatchFormatInput::ArchenemyCommander => {
                    self.validate_commander_setup(
                        player_count,
                        decks.expect("Commander decks checked above"),
                        commanders.expect("Commander commanders checked above"),
                        sideboards.map(Vec::as_slice),
                        hidden_manifests,
                    )
                    .map_err(|error| JsValue::from_str(&error))?;
                    if config.format == MatchFormatInput::ArchenemyCommander {
                        self.load_scheme_decks_for_setup(
                            config.scheme_decks.as_deref().ok_or_else(|| {
                                JsValue::from_str(
                                    "Archenemy Commander matches require scheme decks",
                                )
                            })?,
                            player_count,
                            ironsmith::game_state::ArchenemyVariant::Commander,
                        )
                        .map_err(|error| JsValue::from_str(&error))?;
                    }
                }
                MatchFormatInput::Brawl if hidden_manifests.is_empty() => self
                    .validate_brawl_setup(
                        decks.expect("Brawl decks checked above"),
                        commanders.expect("Brawl commanders checked above"),
                    )
                    .map_err(|error| JsValue::from_str(&error))?,
                MatchFormatInput::ConspiracyDraft => {
                    self.validate_conspiracy_limited_setup(
                        player_count,
                        decks.map(Vec::as_slice),
                        sideboards.map(Vec::as_slice),
                        hidden_manifests,
                    )
                    .map_err(|error| JsValue::from_str(&error))?;
                    self.load_conspiracies_for_setup(
                        config.conspiracies.as_deref().ok_or_else(|| {
                            JsValue::from_str(
                                "Conspiracy Draft games require conspiracy selections",
                            )
                        })?,
                        sideboards.map(Vec::as_slice).ok_or_else(|| {
                            JsValue::from_str("Conspiracy Draft games require drafted sideboards")
                        })?,
                        player_count,
                    )
                    .map_err(|error| JsValue::from_str(&error))?;
                }
                MatchFormatInput::CommanderDraft => self
                    .validate_commander_draft_setup(
                        player_count,
                        decks.expect("Commander Draft decks checked above"),
                        commanders.expect("Commander Draft commanders checked above"),
                        sideboards.map(Vec::as_slice),
                        hidden_manifests,
                        config
                            .commander_draft
                            .as_ref()
                            .expect("validated Commander Draft metadata"),
                    )
                    .map_err(|error| JsValue::from_str(&error))?,
                MatchFormatInput::Normal
                | MatchFormatInput::FreeForAll
                | MatchFormatInput::GrandMelee
                | MatchFormatInput::TeamVsTeam
                | MatchFormatInput::Emperor
                | MatchFormatInput::TwoHeadedGiant
                | MatchFormatInput::AlternatingTeams
                | MatchFormatInput::Ante
                | MatchFormatInput::Planechase
                | MatchFormatInput::Vanguard
                | MatchFormatInput::Archenemy
                | MatchFormatInput::SupervillainRumble => {
                    if commanders
                        .is_some_and(|commanders| commanders.iter().any(|list| !list.is_empty()))
                    {
                        return Err(JsValue::from_str(
                            "normal constructed matches cannot designate commanders",
                        ));
                    }
                    self.validate_normal_constructed_setup(
                        player_count,
                        decks.map(Vec::as_slice),
                        sideboards.map(Vec::as_slice),
                        hidden_manifests,
                    )
                    .map_err(|error| JsValue::from_str(&error))?;
                    Self::validate_ante_manifest_visibility(config.format, hidden_manifests)
                        .map_err(|error| JsValue::from_str(&error))?;
                    self.validate_ante_card_legality_for_setup(
                        decks.map(Vec::as_slice),
                        sideboards.map(Vec::as_slice),
                        config.format == MatchFormatInput::Ante,
                    )
                    .map_err(|error| JsValue::from_str(&error))?;
                    if config.format == MatchFormatInput::Planechase {
                        self.load_planar_decks_for_setup(
                            config.planar_decks.as_deref().ok_or_else(|| {
                                JsValue::from_str("Planechase matches require planar decks")
                            })?,
                            player_count,
                        )
                        .map_err(|error| JsValue::from_str(&error))?;
                    } else if config.format == MatchFormatInput::GrandMelee
                        && config
                            .planar_decks
                            .as_ref()
                            .is_some_and(|decks| !decks.is_empty())
                    {
                        self.load_planar_decks_for_setup(
                            config.planar_decks.as_deref().expect("checked nonempty"),
                            player_count,
                        )
                        .map_err(|error| JsValue::from_str(&error))?;
                    } else if config
                        .planar_decks
                        .as_ref()
                        .is_some_and(|decks| !decks.is_empty())
                    {
                        return Err(JsValue::from_str(
                            "planar decks may be supplied only for a Planechase match",
                        ));
                    }
                    if config.format == MatchFormatInput::Vanguard {
                        self.load_vanguards_for_setup(
                            config.vanguards.as_deref().ok_or_else(|| {
                                JsValue::from_str("Vanguard matches require vanguard cards")
                            })?,
                            player_count,
                        )
                        .map_err(|error| JsValue::from_str(&error))?;
                    } else if config
                        .vanguards
                        .as_ref()
                        .is_some_and(|cards| !cards.is_empty())
                    {
                        return Err(JsValue::from_str(
                            "vanguard cards may be supplied only for a Vanguard match",
                        ));
                    }
                    let archenemy_variant = match config.format {
                        MatchFormatInput::Archenemy => {
                            Some(ironsmith::game_state::ArchenemyVariant::Default)
                        }
                        MatchFormatInput::SupervillainRumble => {
                            Some(ironsmith::game_state::ArchenemyVariant::SupervillainRumble)
                        }
                        _ => None,
                    };
                    if let Some(variant) = archenemy_variant {
                        self.load_scheme_decks_for_setup(
                            config.scheme_decks.as_deref().ok_or_else(|| {
                                JsValue::from_str("Archenemy matches require scheme decks")
                            })?,
                            player_count,
                            variant,
                        )
                        .map_err(|error| JsValue::from_str(&error))?;
                    } else if config
                        .scheme_decks
                        .as_ref()
                        .is_some_and(|decks| decks.iter().any(|deck| !deck.is_empty()))
                    {
                        return Err(JsValue::from_str(
                            "scheme decks may be supplied only for an Archenemy match",
                        ));
                    }
                }
                MatchFormatInput::Brawl => {}
            }
        }

        Ok(MatchValidationResult {
            valid: issues.is_empty(),
            issues,
        })
    }

    fn collect_match_validation_issues(
        &mut self,
        player_index: usize,
        player_name: &str,
        section: &str,
        cards: &[String],
        cache: &mut HashMap<String, Option<String>>,
        issues: &mut Vec<MatchValidationIssue>,
    ) {
        let mut seen = HashSet::new();

        for card_name in cards {
            let trimmed = card_name.trim();
            let cache_key = trimmed.to_ascii_lowercase();
            if !seen.insert(cache_key.clone()) {
                continue;
            }

            let error = if let Some(existing) = cache.get(&cache_key) {
                existing.clone()
            } else {
                let computed = self.validate_match_card_name(trimmed);
                cache.insert(cache_key, computed.clone());
                computed
            };

            if let Some(error) = error {
                issues.push(MatchValidationIssue {
                    player_index,
                    player_name: player_name.to_string(),
                    section: section.to_string(),
                    card_name: trimmed.to_string(),
                    error,
                });
            }
        }
    }

    fn validate_match_card_name(&mut self, query: &str) -> Option<String> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Some("card name cannot be empty".to_string());
        }

        self.registry.ensure_cards_loaded([trimmed]);
        if let Some(definition) = self.find_card_definition(trimmed).cloned() {
            return ironsmith::cards::unsupported_generated_definition_error(&definition);
        }

        if let Some(error) = self.external_compile_error_for_name(trimmed) {
            return Some(error);
        }

        match ironsmith::cards::CardRegistry::try_compile_card(trimmed) {
            Ok(_) => Some(format!("unknown card name: {trimmed}")),
            Err(err) => Some(Self::card_lookup_error_for_query(trimmed, err)),
        }
    }

    fn load_compilable_card_definition(
        &mut self,
        query: &str,
    ) -> Result<ironsmith::cards::CardDefinition, JsValue> {
        self.load_compilable_card_definition_result(query)
            .map_err(|error| JsValue::from_str(&error))
    }

    fn load_compilable_card_definition_result(
        &mut self,
        query: &str,
    ) -> Result<ironsmith::cards::CardDefinition, String> {
        if let Some(definition) = self.find_card_definition(query).cloned() {
            if let Some(error) =
                ironsmith::cards::unsupported_generated_definition_error(&definition)
            {
                return Err(error);
            }
            return Ok(definition);
        }

        if let Ok(definition) = ironsmith::cards::CardRegistry::try_compile_card(query)
            && ironsmith::cards::unsupported_generated_definition_error(&definition).is_none()
        {
            return Ok(definition);
        }

        if let Some(error) = self.external_compile_error_for_name(query) {
            return Err(error);
        }

        match ironsmith::cards::CardRegistry::try_compile_card(query) {
            Ok(_) => Err(format!("unknown card name: {query}")),
            Err(err) => Err(Self::card_lookup_error_for_query(query, err)),
        }
    }
}
