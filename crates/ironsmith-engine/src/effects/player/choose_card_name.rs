use crate::cards::CardRegistry;
use crate::decisions::context::TextInputContext;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, StableId};
use crate::object::ObjectKind;
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;
pub use ironsmith_core::ChooseCardNameEffect;

pub(crate) fn synthetic_chosen_name_snapshot(
    source: ObjectId,
    chooser: crate::ids::PlayerId,
    name: String,
) -> ObjectSnapshot {
    ObjectSnapshot {
        discard_characteristics_undefined: false,
        custom_statuses: Vec::new(), named_action_history:None,
        text_addition_sources: Vec::new(),
        entry_action_objects: Default::default(),
        cast_controlled_objects: None,
        stack_kind: None,
        has_casting_ability: false,
        ability_origins: None,
        source_definition_metadata: None,
        chosen_subtype: None,
        chosen_card_type: None,
                numeric_choice_memory: None,
        secret_chosen_subtype: None,
        noted_life_total: None,
                saddled: None,
        chosen_object: None,
        object_id: source,
        stable_id: StableId::from(source),
        kind: ObjectKind::Card,
        card: None,
        controller: chooser,
        owner: chooser,
        name,
        first_printed_set_name: None,
        mana_cost: None,
        colors: crate::color::ColorSet::default(),
        supertypes: Vec::new(),
        card_types: Vec::new(),
        subtypes: Vec::new(),
        compiled_card_text: String::new(),
        ability_labels: Vec::new(),
        other_face: None,
        other_face_name: None,
        linked_face_layout: crate::card::LinkedFaceLayout::None,
        linked_face_mana_value: None,
        power: None,
        toughness: None,
        base_power: None,
        base_toughness: None,
        loyalty: None,
        defense: None,
        abilities: std::sync::Arc::new(Vec::new()),
        aura_attach_filter: None,
        copiable_values: crate::snapshot::CopiableValues::default(),
                revealed_cast_definition: None,
        x_value: None,
        cast_order_this_turn: None,
        mana_spent_to_cast: crate::player::ManaPool::default(),
        caster_mana_spent_to_cast: None,
        mana_spent_on_x: None,
        snow_mana_spent_to_cast: crate::player::ManaPool::default(),
        mana_sources_spent_to_cast: Vec::new(),
        paid_optional_cost_objects: Default::default(),
        optional_costs_paid: crate::cost::OptionalCostsPaid::default(),
        named_payment_outcomes:Vec::new(),
        counters: std::collections::BTreeMap::new(),
        is_token: false,
        tapped: false,
        attacking: false,
        blocking: false,
        attacking_alone: false,
        blocking_alone: false,
        goaded: None,
            suspected: None,
            ring_bearer: None,
        flipped: false,
        face_down: false,
        transform_count: 0,
        attached_to: None,
        attachments: Vec::new(),
        all_nonlegendary_creature_names: false,
        attachment_snapshots: Vec::new(),
                    related_exiled_cards: Default::default(),
        was_enchanted: false,
        is_monstrous: false,
        case_solved: false,
        is_prepared: false,
        is_commander: false,
        zone: Zone::Command,
    }
}

pub(crate) fn split_chosen_card_names(names: &str) -> Vec<String> {
    names
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

/// CR 201.4a-f: a name chosen "with certain characteristics" must be the name
/// of a card (or of the selected half, face, or alternative characteristics)
/// whose Oracle characteristics match. Names the Oracle table doesn't know
/// fall back to the physical cards in this game; a name neither knows can't
/// be checked and is accepted.
fn oracle_name_matches_filter(
    game: &GameState,
    source: ObjectId,
    chooser: crate::ids::PlayerId,
    filter: &crate::target::ObjectFilter,
    filter_ctx: &crate::filter::FilterContext,
    name: &str,
) -> bool {
    use crate::filter::ObjectFilterExt as _;
    let mut candidates = crate::cards::oracle_face_types(crate::filter::oracle_name_for_lookup(name)).to_vec();
    if candidates.is_empty() {
        candidates = game
            .objects_map()
            .values()
            .filter(|object| {
                matches!(object.kind, ObjectKind::Card)
                    && crate::filter::names_match(&object.name, name)
            })
            .map(|object| (object.supertypes.to_vec(), object.card_types.to_vec()))
            .collect();
    }
    if candidates.is_empty() {
        return true;
    }
    // The choice tests the named card's characteristics, not any object's
    // location or tagged relations.
    let mut functional = filter.clone();
    functional.zone = None;
    functional.tagged_constraints.clear();
    candidates.into_iter().any(|(supertypes, card_types)| {
        let mut snapshot = synthetic_chosen_name_snapshot(source, chooser, name.to_string());
        snapshot.supertypes = supertypes;
        snapshot.card_types = card_types;
        functional.matches_snapshot(&snapshot, filter_ctx, game)
    })
}

impl TextInputContext {
    pub fn first_legal_card_name(&self, game: &GameState) -> Option<String> {
        if let Some(names) = &self.allowed_names {
            return names.iter().find(|name| self.allows_card_name(game, name)).cloned();
        }
        crate::cards::oracle_face_names().find(|name| self.allows_card_name(game, name)).map(str::to_owned)
    }

    /// Check the pending decision before accepting an answer into the replay.
    /// Registration can recognize a combined route such as "Fire // Ice", but
    /// naming a card requires a single Oracle name (CR 201.4b).
    pub fn allows_card_name(&self, game: &GameState, name: &str) -> bool {
        let name = name.trim();
        let matches = |other: &String| crate::filter::names_match(other, name);
        !name.is_empty() && !name.contains("//")
            && !self.excluded_names.iter().any(matches)
            && self.allowed_names.as_ref().is_none_or(|names| names.iter().any(matches))
            && self.name_filter.as_deref().is_none_or(|(filter, ctx)| self.source.is_some_and(|source|
                oracle_name_matches_filter(game, source, self.player, filter, ctx, name)))
    }
}

impl EffectExecutor for ChooseCardNameEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        let mut choice_ctx = TextInputContext::new(chooser, Some(ctx.source), "Choose a card name")
            .with_placeholder("Enter a card name").require_known_value(true);
        if self.exclude_source_history {
            choice_ctx.excluded_names = game.chosen_named_option(ctx.source)
                .map(split_chosen_card_names).unwrap_or_default();
        }
        if let Some(filter) = &self.filter {
            if filter.prior_effect_action_surface() == Some(ironsmith_core::PriorEffectAction::Revealed) {
                use crate::filter::ObjectFilterExt as _;
                let mut functional = filter.clone();
                functional.zone = None;
                functional.tagged_constraints.clear();
                let filter_ctx = ctx.filter_context(game);
                let names: Vec<_> = ctx.get_tagged_all(crate::effects::REVEALED_THIS_WAY_TAG)
                    .into_iter().flatten()
                    .filter(|snapshot| functional.matches_snapshot(snapshot, &filter_ctx, game))
                    .flat_map(|snapshot| {
                        std::iter::once(snapshot.name.clone()).chain(
                            snapshot.other_face_name.clone().filter(|_| snapshot.linked_face_layout == crate::card::LinkedFaceLayout::Split))
                    }).collect();
                if names.is_empty() { return Ok(EffectOutcome::count(0)); }
                choice_ctx.allowed_names = Some(names);
            }
            choice_ctx.name_filter = Some(Box::new((filter.clone(), ctx.filter_context(game))));
        }
        let chosen_name = if self.options.is_empty() {
            ctx.decision_maker.decide_text(game, &choice_ctx)
        } else {
            let menu = self.options.iter()
                .filter(|name| choice_ctx.allows_card_name(game, name))
                .map(|name| (name.clone(), name.clone())).collect::<Vec<_>>();
            if menu.is_empty() { return Ok(EffectOutcome::count(0)); }
            use crate::effects::composition::scoped_choice::{select_typed_option, SingleChoicePolicy};
            let Some(chosen) = select_typed_option(game, ctx, chooser, "Choose a card name", &menu, SingleChoicePolicy::First)? else {
                return Ok(EffectOutcome::count(0));
            };
            chosen
        };
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let chosen_name = chosen_name.trim();
        if !choice_ctx.allows_card_name(game, chosen_name) {
            return Err(ExecutionError::Impossible(format!("invalid card name for this choice: {chosen_name}")));
        }

        let mut registry = CardRegistry::new();
        registry.ensure_cards_loaded([chosen_name]);
        let canonical_name = registry
            .get(chosen_name)
            .map(|definition| definition.name().to_string())
            .unwrap_or_else(|| chosen_name.to_string());

        let mut chosen_names = game
            .chosen_named_option(ctx.source)
            .map(split_chosen_card_names)
            .unwrap_or_default();
        if !chosen_names
            .iter()
            .any(|name| crate::filter::names_match(name, &canonical_name))
        {
            chosen_names.push(canonical_name.clone());
        }
        game.set_chosen_named_option(ctx.source, chosen_names.join("\n"));
        // The resolution tag names this choice only. Persistent source names
        // may include other choices made by its abilities, but must not leak
        // into a later resolution's singular chosen-name comparison.
        let mut snapshot = synthetic_chosen_name_snapshot(ctx.source, chooser, canonical_name);
        if ctx.iteration.iterated_player.is_some() {
            // "Each player chooses a card name": every chooser's name must
            // survive the per-player tag merge, which keys snapshots by stable
            // identity. A fresh synthetic identity keeps each choice distinct
            // (its controller remains the chooser).
            snapshot.stable_id = StableId::from(game.new_object_id());
        }
        ctx.set_tagged_objects(self.tag.clone(), vec![snapshot]);
        Ok(EffectOutcome::count(1))
    }
}
