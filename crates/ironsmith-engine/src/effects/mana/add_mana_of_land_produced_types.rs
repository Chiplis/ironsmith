//! Add mana of any color/type that permanents matching a filter could produce.

use crate::ability::{Ability, AbilityKind, ActivatedAbilityRuntimeExt as _};
use crate::effect::Value;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use super::production_resolution::ResolvedManaOutput;
use crate::mana_payment::replacement_program::{CompiledManaReplacements, ReplacementResources};
use crate::target::{ObjectFilter, PlayerFilter};
pub use ironsmith_core::ManaTypeSource;

/// Effect that adds mana constrained by what matching permanents could produce.
///
/// The historical land-specific type and field names are retained for artifact
/// compatibility; the producer filter supplies any card-type restriction.
///
/// This models text like:
/// - "Add one mana of any color that a land an opponent controls could produce."
/// - "Add one mana of any type that a Gate you control could produce."
/// - "That player adds one mana of any type that land produced."
#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfLandProducedTypesEffect {
    /// Number of mana to add.
    pub amount: Value,
    /// Which player receives the mana.
    pub player: PlayerFilter,
    /// Permanents to inspect for producible mana.
    pub land_filter: ObjectFilter,
    /// Whether colorless mana is allowed ("any type" vs "any color").
    pub allow_colorless: bool,
    /// If true, all mana must be the same type.
    pub same_type: bool,
    /// Whether to inspect potential producer abilities or the actual triggering event.
    pub mana_type_source: ManaTypeSource,
}

impl AddManaOfLandProducedTypesEffect {
    pub fn new(
        amount: impl Into<Value>,
        player: PlayerFilter,
        land_filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
            land_filter,
            allow_colorless,
            same_type,
            mana_type_source: ManaTypeSource::MatchingLandsCouldProduce,
        }
    }

    pub fn from_triggering_event(
        amount: impl Into<Value>,
        player: PlayerFilter,
        land_filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
            land_filter,
            allow_colorless,
            same_type,
            mana_type_source: ManaTypeSource::TriggeringEventProduced,
        }
    }
}

impl EffectExecutor for AddManaOfLandProducedTypesEffect {
    fn mana_production(&self) -> Option<crate::mana_payment::program::ManaProduction<'_>> {
        use crate::mana_payment::program::ManaProduction;
        Some(ManaProduction::LandProducedTypes {
            amount: &self.amount,
            player: &self.player,
            filter: &self.land_filter,
            allow_colorless: self.allow_colorless,
            same_type: self.same_type,
            source: self.mana_type_source,
        })
    }

    fn directly_produces_mana(&self) -> bool {
        true
    }

    /// CR 106.7: the symbols matching permanents could produce right now.
    /// Without this, a source whose activation cannot be projected (Benthic
    /// Explorers' untap-an-opponent's-land cost) had no inferred colours, so
    /// the payment search offered only its open variant and never the colour
    /// the cost needed.
    fn producible_mana_symbols(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        if self.mana_type_source != ManaTypeSource::MatchingLandsCouldProduce {
            return None;
        }
        let ctx = ExecutionContext::new_default(source, controller);
        let symbols: Vec<ManaSymbol> = collect_available_mana_symbols(game, &ctx, &self.land_filter)
            .into_iter()
            .filter(|symbol| is_allowed_symbol(*symbol, self.allow_colorless))
            .collect();
        (!symbols.is_empty()).then_some(symbols)
    }

    #[crate::effects::project_effect_outcome]
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        use super::production_resolution::{execute_production, ManaOutputProjection, ManaChoiceMenu};
        execute_production(self.mana_production().expect("mana production descriptor"),
            game, ctx, ManaOutputProjection::AuthoredCount, ManaChoiceMenu::Types)
    }
}

pub(super) fn collect_triggering_event_mana_symbols(
    game: &GameState,
    ctx: &ExecutionContext,
    source_filter: &ObjectFilter,
) -> Result<Vec<ManaSymbol>, ExecutionError> {
    let Some(event) = ctx
        .triggering_event
        .as_ref()
        .and_then(|event| event.downcast::<crate::events::ManaAddedEvent>())
    else {
        return Err(ExecutionError::IncompleteEvidence(
            "produced mana types require the exact triggering production event".into(),
        ));
    };

    // This is an event-time comparison. A later live object cannot fill a
    // missing production receipt, even if it still has the same identity.
    let snapshot = event.snapshot.as_ref().ok_or_else(|| {
        ExecutionError::IncompleteEvidence(
            "produced mana types require the event-time source snapshot".into(),
        )
    })?;
    if snapshot.object_id != event.source {
        return Err(ExecutionError::IncompleteEvidence(
            "mana production source snapshot belongs to a different object".into(),
        ));
    }
    let filter_ctx = ctx.filter_context(game);
    if !source_filter.matches_snapshot(snapshot, &filter_ctx, game) {
        return Ok(Vec::new());
    }
    if event.mana.iter().any(|symbol| {
        !matches!(
            symbol,
            ManaSymbol::White
                | ManaSymbol::Blue
                | ManaSymbol::Black
                | ManaSymbol::Red
                | ManaSymbol::Green
                | ManaSymbol::Colorless
        )
    }) {
        return Err(ExecutionError::IncompleteEvidence(
            "mana production receipt contains an unresolved mana symbol".into(),
        ));
    }
    let mut symbols = event.mana.clone();
    symbols.sort_by_key(|symbol| canonical_symbol_order(*symbol));
    symbols.dedup();
    Ok(symbols)
}

pub(super) fn collect_available_mana_symbols(
    game: &GameState,
    ctx: &ExecutionContext,
    land_filter: &ObjectFilter,
) -> Vec<ManaSymbol> {
    let mut symbols = Vec::new();
    let filter_ctx = ctx.filter_context(game);
    for &id in &game.battlefield {
        let Some(object) = game.object(id) else {
            continue;
        };
        if land_filter.matches(object, &filter_ctx, game) {
            let abilities = game
                .current_abilities(id)
                .unwrap_or_else(|| object.abilities_vec());
            symbols.extend(permanent_mana_types(
                game,
                id,
                game.controller_of(object),
                &abilities,
                None,
            ));
        }
    }
    // "any type the sacrificed land could produce" (Squandered Resources):
    // a tagged producer that already left the battlefield is read from its
    // last-known abilities (CR 106.7, 608.2h).
    for constraint in &land_filter.tagged_constraints {
        if constraint.relation != crate::target::TaggedOpbjectRelation::IsTaggedObject {
            continue;
        }
        let Some(snapshots) = ctx.get_tagged_all(constraint.tag.as_str()) else {
            continue;
        };
        for snapshot in snapshots {
            if game.battlefield.contains(&snapshot.object_id)
                || !land_filter.matches_snapshot(snapshot, &filter_ctx, game)
            {
                continue;
            }
            symbols.extend(permanent_mana_types(
                game,
                snapshot.object_id,
                snapshot.controller,
                &snapshot.abilities,
                Some(snapshot),
            ));
        }
    }

    symbols.sort_by_key(|symbol| canonical_symbol_order(*symbol));
    symbols.dedup();
    symbols
}

/// CR 106.7 evaluates each ability as though it resolved now, without paying
/// activation costs. An undefined recursive production contributes no types.
pub(crate) fn permanent_mana_types(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    abilities: &[Ability],
    historical: Option<&ObjectSnapshot>,
) -> Vec<ManaSymbol> {
    thread_local! {
        static ACTIVE: std::cell::RefCell<Vec<(ObjectId, usize)>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    struct ActiveAbility;
    impl Drop for ActiveAbility {
        fn drop(&mut self) {
            ACTIVE.with(|active| {
                active.borrow_mut().pop();
            });
        }
    }

    let mut types = Vec::new();
    let mut replacements = None;
    let mut snapshot = historical.cloned();
    for (index, ability) in abilities.iter().enumerate() {
        if !ability.functions_in(&crate::zone::Zone::Battlefield)
            || ACTIVE.with(|active| active.borrow().contains(&(source, index)))
        {
            continue;
        }
        use crate::events::mana::ManaProductionProvenance::{
            ManaAbility, TappedSourceForMana, Unknown,
        };
        let (effects, fixed, provenance) = match &ability.kind {
            AbilityKind::Activated(a) if a.could_add_mana(game, source, controller) => {
                let provenance = if !a.is_runtime_mana_ability(game, source, controller) {
                    Unknown
                } else if a.has_tap_cost() {
                    TappedSourceForMana
                } else {
                    ManaAbility
                };
                (&a.effects, a.mana_symbols(), provenance)
            }
            AbilityKind::Triggered(t)
                if t.effects
                    .all_effects()
                    .iter()
                    .any(|e| e.contains_mana_production()) =>
            {
                (&t.effects, &[][..], Unknown)
            }
            _ => continue,
        };
        ACTIVE.with(|active| active.borrow_mut().push((source, index)));
        let _active = ActiveAbility;
        let mut ctx = ExecutionContext::new_default(source, controller);
        ctx.ability_index = Some(index);
        ctx.source_snapshot = historical.cloned();
        if let AbilityKind::Triggered(t) = &ability.kind
            && let Some(condition) = &t.intervening_if
            && !crate::condition_eval::evaluate_condition_resolution(game, condition, &ctx)
                .unwrap_or(false)
        {
            continue;
        }
        let mut add = |recipient, output: ResolvedManaOutput| {
            let compiled = replacements
                .get_or_insert_with(|| CompiledManaReplacements::for_hypothetical_production(game));
            let compiled = compiled.as_ref().filter(|r| !r.is_empty());
            if compiled.is_none() {
                match output {
                    ResolvedManaOutput::Exact(mana) => types.extend(mana),
                    ResolvedManaOutput::Choice {
                        available,
                        count,
                        distinct,
                        same_type,
                    } if count > 0
                        && (!distinct
                            || count as usize <= available.len() && (!same_type || count == 1)) =>
                    {
                        types.extend(available)
                    }
                    _ => {}
                }
                return;
            }
            if snapshot.is_none() {
                snapshot = game
                    .object(source)
                    .map(|o| ObjectSnapshot::from_object_with_calculated_characteristics(o, game));
            }
            let mut apply = |mana: &[ManaSymbol]| {
                if mana.is_empty() {
                    return;
                }
                let event = crate::events::ManaAddedEvent::new(
                    source,
                    controller,
                    recipient,
                    mana.to_vec(),
                )
                .with_snapshot(snapshot.clone())
                .with_production_provenance(provenance);
                if let Some(branches) = compiled.unwrap().branches(
                    game,
                    event,
                    &ReplacementResources::default(),
                    usize::MAX,
                ) {
                    for branch in branches {
                        types.extend(branch.event.mana);
                    }
                }
            };
            match output {
                ResolvedManaOutput::Exact(mana) => apply(&mana),
                ResolvedManaOutput::Choice {
                    available,
                    count,
                    distinct,
                    same_type,
                } => {
                    // Production predicates inspect types and quantities. Permutations of
                    // the same bundle have the same possible resulting types.
                    if same_type {
                        if !distinct || count <= 1 {
                            for symbol in available {
                                apply(&vec![symbol; count as usize]);
                            }
                        }
                    } else {
                        visit_mana_bundles(
                            &available,
                            count as usize,
                            distinct,
                            &mut Vec::new(),
                            &mut apply,
                        );
                    }
                }
            }
        };
        if !fixed.is_empty() {
            add(controller, ResolvedManaOutput::Exact(fixed.to_vec()));
            continue;
        }
        for effect in crate::ability::selected_resolution_effects_for_current_state(
            effects, game, source, controller,
        ) {
            if let Some(production) = effect.mana_production() {
                if let Ok(resolved) = production.resolve(game, &ctx) {
                    add(resolved.player, resolved.output);
                }
            } else if effect.contains_mana_production() {
                let mut available = Vec::new();
                effect.collect_producible_mana_symbols(game, source, controller, &mut available);
                for symbol in available {
                    add(controller, ResolvedManaOutput::Exact(vec![symbol]));
                }
            }
        }
    }
    types.retain(|s| is_allowed_symbol(*s, true));
    types.sort_by_key(|s| canonical_symbol_order(*s));
    types.dedup();
    types
}

fn visit_mana_bundles(
    available: &[ManaSymbol],
    remaining: usize,
    distinct: bool,
    bundle: &mut Vec<ManaSymbol>,
    visit: &mut impl FnMut(&[ManaSymbol]),
) {
    if remaining == 0 {
        visit(bundle);
        return;
    }
    let Some((&first, rest)) = available.split_first() else {
        return;
    };
    if distinct && remaining > available.len() {
        return;
    }
    let max = if distinct { 1 } else { remaining };
    let min = if rest.is_empty() { remaining } else { 0 };
    for count in min..=max {
        let start = bundle.len();
        bundle.resize(start + count, first);
        visit_mana_bundles(rest, remaining - count, distinct, bundle, visit);
        bundle.truncate(start);
    }
}

pub(super) fn is_allowed_symbol(symbol: ManaSymbol, allow_colorless: bool) -> bool {
    match symbol {
        ManaSymbol::White
        | ManaSymbol::Blue
        | ManaSymbol::Black
        | ManaSymbol::Red
        | ManaSymbol::Green => true,
        ManaSymbol::Colorless => allow_colorless,
        _ => false,
    }
}

fn canonical_symbol_order(symbol: ManaSymbol) -> usize {
    match symbol {
        ManaSymbol::White => 0,
        ManaSymbol::Blue => 1,
        ManaSymbol::Black => 2,
        ManaSymbol::Red => 3,
        ManaSymbol::Green => 4,
        ManaSymbol::Colorless => 5,
        _ => 100,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::CardDefinitionBuilder;
    use crate::events::mana::ManaProductionProvenance;
    use crate::ids::{CardId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn triggering_event_mode_adds_only_a_type_the_land_actually_produced() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let land = CardDefinitionBuilder::new(CardId::new(), "Abilityless Land")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_definition(&land, bob, Zone::Battlefield);
        let snapshot =
            ObjectSnapshot::from_object(game.object(land_id).expect("land should exist"), &game);

        // Resolve from LKI to prove this does not recompute what the current
        // battlefield object could produce.
        game.remove_object(land_id);
        let event = crate::events::ManaAddedEvent::new(land_id, bob, bob, vec![ManaSymbol::Red])
            .with_snapshot(Some(snapshot))
            .with_production_provenance(ManaProductionProvenance::TappedSourceForMana)
            .into_trigger_event();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);
        let effect = AddManaOfLandProducedTypesEffect::from_triggering_event(
            1,
            PlayerFilter::IteratedPlayer,
            ObjectFilter::land(),
            true,
            false,
        );

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("actual produced-type effect should resolve");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.player(bob).expect("Bob should exist").mana_pool.red, 1);
        assert_eq!(
            game.player(bob).expect("Bob should exist").mana_pool.green,
            0
        );
    }

    #[test]
    fn triggering_event_mode_respects_type_and_source_filter_restrictions() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let land = CardDefinitionBuilder::new(CardId::new(), "Ordinary Land")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_definition(&land, alice, Zone::Battlefield);
        let snapshot =
            ObjectSnapshot::from_object(game.object(land_id).expect("land should exist"), &game);
        let event =
            crate::events::ManaAddedEvent::new(land_id, alice, alice, vec![ManaSymbol::Colorless])
                .with_snapshot(Some(snapshot))
                .with_production_provenance(ManaProductionProvenance::TappedSourceForMana)
                .into_trigger_event();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);
        let effect = AddManaOfLandProducedTypesEffect::from_triggering_event(
            1,
            PlayerFilter::You,
            ObjectFilter::land(),
            false,
            false,
        );

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("color-restricted produced-type effect should resolve");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(
            game.player(alice)
                .expect("Alice should exist")
                .mana_pool
                .colorless,
            0
        );
    }
}

#[cfg(test)]
mod retained_production_evidence_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::{CardId, CardType, Zone};
    #[test]
    fn absent_mismatched_and_known_empty_production_receipts_stay_distinct() {
        for case in 0..5 {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let player = crate::PlayerId(0);
            let definition = CardBuilder::new(CardId::new(), "Production source").card_types(vec![CardType::Land]).build();
            let source = game.create_object_from_card(&definition, player, Zone::Battlefield);
            let mut ctx = ExecutionContext::new_default(source, player);
            if case > 0 {
                let mut snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
                if case == 2 { snapshot.object_id = crate::ObjectId::from_raw(999_991); }
                let event = crate::events::ManaAddedEvent::new(source, player, player,
                    if case == 3 { Vec::new() } else { vec![ManaSymbol::Blue] })
                    .with_snapshot((case != 1).then_some(snapshot));
                ctx = ctx.with_triggering_event(event.into_trigger_event());
            }
            let effect = AddManaOfLandProducedTypesEffect::from_triggering_event(1, PlayerFilter::You, ObjectFilter::land(), true, false);
            let projection = effect.mana_production().unwrap().resolve(&game, &ctx);
            if case < 3 {
                assert!(matches!(projection, Err(ExecutionError::IncompleteEvidence(_))));
                assert!(matches!(effect.execute(&mut game, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
                assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
            } else {
                projection.unwrap();
                if case == 4 { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
                effect.execute(&mut game, &mut ctx).unwrap();
                assert_eq!(game.player(player).unwrap().mana_pool.blue, u64::from(case == 4));
            }
        }
    }
}
